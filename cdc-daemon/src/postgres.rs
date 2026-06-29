use crate::constants::{
    CONNECT_RETRY_SECS, DEFAULT_HOSTNAME, DEFAULT_OS_URL, DEFAULT_OS_USER, DEFAULT_OTLP_ENDPOINT,
    DEFAULT_PIPELINES_FILE, DEFAULT_RW_DBNAME, DEFAULT_RW_HOST, DEFAULT_RW_PORT, DEFAULT_RW_USER,
    EMPTY_FETCH_SLEEP_MS, TIMEOUT_DURATION,
};
use crate::error::{AppError, DaemonResult, StreamFetchError};
use crate::grpc_server::DaemonState;
use crate::metrics::PipelineMetrics;
use futures_util::{StreamExt, stream};
use opentelemetry::KeyValue;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::time::sleep;
use tokio_postgres::Client;
use tokio_postgres::types::Type;
use tokio_util::sync::CancellationToken;
use tracing::warn;

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct PipelineConfig {
    pub subscription_name: String,
    pub target_index: String,
    pub id_field: String,
    pub batch_size: usize,
}

#[derive(Debug)]
pub struct StreamBatch {
    pub target_index: String,
    pub id_field: String,
    pub rows: Vec<Value>,
}

pub struct RuntimeConfig {
    pub otlp_endpoint: String,
    pub rw_conn_str: String,
    pub os_url: String,
    pub os_user: String,
    pub os_password: String,
    pub pipelines_path: String,
    pub hostname: String,
    pub consumer_id: String,
}

pub fn load_runtime_config() -> DaemonResult<RuntimeConfig> {
    let hostname = env_or_default("HOSTNAME", DEFAULT_HOSTNAME);
    let consumer_id = match std::env::var("CONSUMER_ID") {
        Ok(value) => value,
        Err(_) => hostname.clone(),
    };

    Ok(RuntimeConfig {
        otlp_endpoint: env_or_default("OTEL_EXPORTER_OTLP_ENDPOINT", DEFAULT_OTLP_ENDPOINT),
        rw_conn_str: format!(
            "host={} port={} user={} dbname={} sslmode=require",
            env_or_default("RW_HOST", DEFAULT_RW_HOST),
            env_or_default("RW_PORT", DEFAULT_RW_PORT),
            env_or_default("RW_USER", DEFAULT_RW_USER),
            env_or_default("RW_DBNAME", DEFAULT_RW_DBNAME),
        ),
        os_url: env_or_default("OS_URL", DEFAULT_OS_URL),
        os_user: env_or_default("OS_USER", DEFAULT_OS_USER),
        os_password: required_env("OS_PASSWORD")?,
        pipelines_path: env_or_default("PIPELINES_FILE", DEFAULT_PIPELINES_FILE),
        hostname,
        consumer_id,
    })
}

pub async fn load_pipeline_configs(path: &str) -> DaemonResult<Vec<PipelineConfig>> {
    let contents = tokio::fs::read_to_string(path).await?;
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "yaml" | "yml" => {
            serde_yaml::from_str(&contents).map_err(|err| AppError::PipelinesParse(err.to_string()))
        }
        "toml" => {
            toml::from_str(&contents).map_err(|err| AppError::PipelinesParse(err.to_string()))
        }
        other => Err(AppError::PipelinesParse(format!(
            "unsupported pipeline config format '{other}'"
        ))),
    }
}

pub async fn run_producer_loop(
    conn_str: Arc<str>,
    consumer_id: Arc<str>,
    config: PipelineConfig,
    tx: tokio::sync::mpsc::Sender<StreamBatch>,
    metrics: Arc<PipelineMetrics>,
    daemon_state: Arc<DaemonState>,
    cancel_token: CancellationToken,
) {
    let unique_cursor = format!(
        "cursor_{}_{}",
        config.subscription_name,
        consumer_id.as_ref()
    );

    loop {
        if cancel_token.is_cancelled() {
            break;
        }
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let tls_connector = tokio_postgres_rustls::MakeRustlsConnect::new(tls_config);

        if let Ok(Ok((client, connection))) = tokio::time::timeout(
            TIMEOUT_DURATION,
            tokio_postgres::connect(conn_str.as_ref(), tls_connector),
        )
        .await
        {
            tokio::spawn(async move {
                let _ = connection.await;
            });
            let _ = client
                .execute(
                    &format!(
                        "DECLARE {} SUBSCRIPTION CURSOR FOR {};",
                        unique_cursor, config.subscription_name
                    ),
                    &[],
                )
                .await;
            let _ = run_stream_fetch_pipeline(
                &client,
                &config,
                &tx,
                metrics.clone(),
                daemon_state.clone(),
                cancel_token.clone(),
                &unique_cursor,
            )
            .await;
        }
        sleep(Duration::from_secs(CONNECT_RETRY_SECS)).await;
    }
}

async fn run_stream_fetch_pipeline(
    client: &Client,
    config: &PipelineConfig,
    tx: &tokio::sync::mpsc::Sender<StreamBatch>,
    metrics: Arc<PipelineMetrics>,
    daemon_state: Arc<DaemonState>,
    cancel_token: CancellationToken,
    cursor_name: &str,
) -> Result<(), StreamFetchError> {
    let labels = [KeyValue::new(
        "subscription",
        config.subscription_name.clone(),
    )];
    let query = format!(
        "FETCH {} FROM {} WITH (timeout = '5 second');",
        config.batch_size, cursor_name
    );
    let mut query_stream = {
        let cancel_token = cancel_token.clone();
        Box::pin(stream::unfold((client, &query), move |(cl, q)| {
            let cancel_token = cancel_token.clone();
            async move {
                if cancel_token.is_cancelled() {
                    return None;
                }
                let result: Result<Vec<tokio_postgres::Row>, StreamFetchError> =
                    match tokio::time::timeout(TIMEOUT_DURATION, cl.query(q, &[])).await {
                        Ok(Ok(rows)) => Ok(rows),
                        Ok(Err(e)) => Err(StreamFetchError::from(e)),
                        Err(_) => Err(StreamFetchError::Timeout),
                    };
                Some((result, (cl, q)))
            }
        }))
    };

    while let Some(fetch_result) = query_stream.next().await {
        let rows = match fetch_result {
            Ok(rows) => rows,
            Err(err) => {
                warn!(
                    subscription = %config.subscription_name,
                    error = %err,
                    "stream fetch failed; retrying"
                );
                continue;
            }
        };

        if rows.is_empty() {
            sleep(Duration::from_millis(EMPTY_FETCH_SLEEP_MS)).await;
            continue;
        }
        let row_count = rows.len();
        let mut batch_records = Vec::with_capacity(row_count);
        for row in rows {
            let mut json_obj = serde_json::Map::new();
            for (col_idx, column) in row.columns().iter().enumerate() {
                json_obj.insert(
                    column.name().to_string(),
                    postgres_value_to_json(&row, col_idx, column.type_()),
                );
            }
            batch_records.push(Value::Object(json_obj));
        }
        metrics.records_ingested.add(row_count as u64, &labels);
        daemon_state
            .records_ingested
            .fetch_add(row_count as u64, Ordering::Relaxed);
        if tx
            .send(StreamBatch {
                target_index: config.target_index.clone(),
                id_field: config.id_field.clone(),
                rows: batch_records,
            })
            .await
            .is_err()
        {
            break;
        }
    }
    Ok(())
}

pub fn postgres_value_to_json(row: &tokio_postgres::Row, col_idx: usize, col_type: &Type) -> Value {
    match *col_type {
        Type::VARCHAR | Type::TEXT | Type::NAME | Type::BPCHAR | Type::UNKNOWN => row
            .try_get::<_, Option<String>>(col_idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |s| json!(s)),
        Type::INT4 => row
            .try_get::<_, Option<i32>>(col_idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |i| json!(i)),
        Type::INT8 => row
            .try_get::<_, Option<i64>>(col_idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |i| json!(i)),
        Type::BOOL => row
            .try_get::<_, Option<bool>>(col_idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |b| json!(b)),
        Type::FLOAT8 => row
            .try_get::<_, Option<f64>>(col_idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |f| json!(f)),
        Type::JSON | Type::JSONB => row
            .try_get::<_, Option<Value>>(col_idx)
            .ok()
            .flatten()
            .unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn env_or_default(name: &'static str, default: &str) -> String {
    match std::env::var(name) {
        Ok(value) => value,
        Err(_) => default.to_string(),
    }
}

fn required_env(name: &'static str) -> DaemonResult<String> {
    std::env::var(name).map_err(|_| AppError::MissingEnv(name))
}
