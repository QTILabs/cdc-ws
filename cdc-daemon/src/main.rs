mod constants;
mod consumer;
mod dlq;
mod error;
mod grpc_server;
mod metrics;
mod postgres;

use crate::constants::{CHANNEL_CAPACITY, GRPC_SERVER_PORT};
use crate::consumer::run_consumer_loop;
use crate::dlq::resolve_dlq_directory;
use crate::error::DaemonResult;
use crate::grpc_server::{DaemonCommand, DaemonState, PipelineRuntime, start_grpc_server};
use crate::metrics::{TelemetryRuntime, initialize_telemetry};
use crate::postgres::{
    PipelineConfig, StreamBatch, load_pipeline_configs, load_runtime_config, run_producer_loop,
};
use opensearch::{
    OpenSearch,
    auth::Credentials,
    http::{
        Url,
        transport::{SingleNodeConnectionPool, TransportBuilder},
    },
};
use std::sync::Arc;
use tokio::sync::{mpsc::UnboundedReceiver, oneshot};
use tokio::task::JoinSet;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

type SharedMetrics = Arc<crate::metrics::PipelineMetrics>;

#[tokio::main]
async fn main() -> DaemonResult<()> {
    dotenvy::dotenv().ok();

    // 0. Install rustls 0.23 crypto provider (MANDATORY)
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| error::AppError::RustlsProvider)?;

    let runtime_config = load_runtime_config()?;
    let telemetry = initialize_telemetry(&runtime_config.otlp_endpoint)?;
    let TelemetryRuntime {
        tracer_provider,
        meter_provider,
        metrics,
    } = telemetry;

    // 3. Shared State & gRPC Server
    let daemon_state = Arc::new(DaemonState::new());
    let grpc_state = Arc::clone(&daemon_state);
    let (control_tx, mut control_rx) = tokio::sync::mpsc::unbounded_channel::<DaemonCommand>();
    tokio::spawn(async move {
        let _ = start_grpc_server(grpc_state, control_tx, GRPC_SERVER_PORT).await;
    });

    let _dlq_directory = resolve_dlq_directory();

    // 4. Connections
    let rw_conn_str = Arc::<str>::from(runtime_config.rw_conn_str);
    let os_password = runtime_config.os_password;
    let url = Url::parse(&runtime_config.os_url)?;
    let os_client = build_opensearch_client(url, runtime_config.os_user, os_password)?;

    // 5. Concurrency Primitives
    let shutdown_token = CancellationToken::new();
    let cl_token = shutdown_token.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        info!("SIGINT received, shutting down...");
        cl_token.cancel();
    });
    let hostname = runtime_config.hostname;
    let consumer_id = Arc::<str>::from(runtime_config.consumer_id);

    run_pipeline_manager(
        PipelineManagerContext {
            pipelines_path: runtime_config.pipelines_path,
            hostname,
            rw_conn_str: Arc::clone(&rw_conn_str),
            consumer_id: Arc::clone(&consumer_id),
            metrics: Arc::clone(&metrics),
            daemon_state: Arc::clone(&daemon_state),
            os_client: Arc::clone(&os_client),
            shutdown_token,
        },
        &mut control_rx,
    )
    .await?;

    let _ = tracer_provider.shutdown();
    let _ = meter_provider.shutdown();
    Ok(())
}

struct PipelineGeneration {
    cancel_token: CancellationToken,
    join_set: JoinSet<()>,
}

struct PipelineManagerContext {
    pipelines_path: String,
    hostname: String,
    rw_conn_str: Arc<str>,
    consumer_id: Arc<str>,
    metrics: SharedMetrics,
    daemon_state: Arc<DaemonState>,
    os_client: Arc<OpenSearch>,
    shutdown_token: CancellationToken,
}

impl PipelineGeneration {
    async fn drain(&mut self) {
        while let Some(result) = self.join_set.join_next().await {
            if let Err(e) = result {
                tracing::error!("Thread exited during shutdown: {:?}", e);
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
async fn run_pipeline_manager(
    context: PipelineManagerContext,
    control_rx: &mut UnboundedReceiver<DaemonCommand>,
) -> DaemonResult<()> {
    let PipelineManagerContext {
        pipelines_path,
        hostname,
        rw_conn_str,
        consumer_id,
        metrics,
        daemon_state,
        os_client,
        shutdown_token,
    } = context;

    let mut current_pipelines: Vec<PipelineConfig> =
        match load_pipeline_configs(&pipelines_path).await {
        Ok(pipelines) => pipelines,
        Err(err) => {
            warn!(
                error = %err,
                path = %pipelines_path,
                "failed to load pipeline configuration at startup; daemon will stay alive and wait for reload"
            );
            Vec::new()
        }
    };
    let mut pending_reload_ack: Option<oneshot::Sender<Result<(), String>>> = None;

    loop {
        if current_pipelines.is_empty() {
            daemon_state.pipelines.write().await.clear();

            if let Some(reply) = pending_reload_ack.take() {
                let _ = reply.send(Ok(()));
            }

            tokio::select! {
                () = shutdown_token.cancelled() => {
                    return Ok(());
                }
                command = control_rx.recv() => {
                    match command {
                        Some(DaemonCommand::ReloadPipelines { reply }) => {
                            match load_pipeline_configs(&pipelines_path).await {
                                Ok(new_pipelines) => {
                                    current_pipelines = new_pipelines;
                                    pending_reload_ack = Some(reply);
                                }
                                Err(err) => {
                                    warn!(
                                        error = %err,
                                        path = %pipelines_path,
                                        "reload rejected because pipeline configuration is invalid"
                                    );
                                    let _ = reply.send(Err(err.to_string()));
                                }
                            }
                        }
                        Some(DaemonCommand::StopDaemon { reply }) => {
                            let _ = reply.send(Ok(()));
                            return Ok(());
                        }
                        None => {
                            return Ok(());
                        }
                    }
                }
            }
            continue;
        }

        let mut generation = start_pipeline_generation(
            current_pipelines.clone(),
            hostname.clone(),
            Arc::clone(&rw_conn_str),
            Arc::clone(&consumer_id),
            Arc::clone(&metrics),
            Arc::clone(&daemon_state),
            Arc::clone(&os_client),
        )
        .await?;

        if let Some(reply) = pending_reload_ack.take() {
            let _ = reply.send(Ok(()));
        }

        let mut reload_requested = false;
        loop {
            tokio::select! {
                () = shutdown_token.cancelled() => {
                    generation.cancel_token.cancel();
                    generation.drain().await;
                    return Ok(());
                }
                command = control_rx.recv() => {
                    match command {
                        Some(DaemonCommand::ReloadPipelines { reply }) => {
                            match load_pipeline_configs(&pipelines_path).await {
                                Ok(new_pipelines) => {
                                    generation.cancel_token.cancel();
                                    generation.drain().await;
                                    current_pipelines = new_pipelines;
                                    pending_reload_ack = Some(reply);
                                    reload_requested = true;
                                    break;
                                }
                                Err(err) => {
                                    warn!(
                                        error = %err,
                                        path = %pipelines_path,
                                        "reload rejected because pipeline configuration is invalid"
                                    );
                                    let _ = reply.send(Err(err.to_string()));
                                }
                            }
                        }
                        Some(DaemonCommand::StopDaemon { reply }) => {
                            generation.cancel_token.cancel();
                            generation.drain().await;
                            let _ = reply.send(Ok(()));
                            return Ok(());
                        }
                        None => {
                            generation.cancel_token.cancel();
                            generation.drain().await;
                            return Ok(());
                        }
                    }
                }
                result = generation.join_set.join_next() => {
                    match result {
                        Some(Ok(())) => {}
                        Some(Err(e)) => {
                            tracing::error!("Thread exited: {:?}", e);
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
        }

        if !reload_requested {
            generation.cancel_token.cancel();
            generation.drain().await;
            break;
        }
    }

    Ok(())
}

async fn start_pipeline_generation(
    pipelines: Vec<PipelineConfig>,
    hostname: String,
    rw_conn_str: Arc<str>,
    consumer_id: Arc<str>,
    metrics: SharedMetrics,
    daemon_state: Arc<DaemonState>,
    os_client: Arc<OpenSearch>,
) -> DaemonResult<PipelineGeneration> {
    {
        let mut pipeline_map = daemon_state.pipelines.write().await;
        pipeline_map.clear();
    }

    let cancel_token = CancellationToken::new();
    let mut join_set = JoinSet::new();
    let (tx, rx) = tokio::sync::mpsc::channel::<StreamBatch>(CHANNEL_CAPACITY);

    for config in pipelines {
        let PipelineConfig {
            subscription_name,
            target_index,
            id_field,
            batch_size,
        } = config;
        let tx_clone = tx.clone();
        let conn_string = Arc::clone(&rw_conn_str);
        let m_handle = Arc::clone(&metrics);
        let ds_handle = Arc::clone(&daemon_state);
        let p_cancel_token = cancel_token.child_token();
        let consumer_id = Arc::clone(&consumer_id);
        let pipeline_name = subscription_name.clone();
        let cursor_name = format!("cursor_{subscription_name}_{hostname}");

        daemon_state.pipelines.write().await.insert(
            pipeline_name,
            PipelineRuntime {
                config_subscription: subscription_name.clone(),
                target_index: target_index.clone(),
                cursor_name,
                state: String::from("RUNNING"),
                cancel_token: p_cancel_token.clone(),
            },
        );

        join_set.spawn(async move {
            run_producer_loop(
                conn_string,
                consumer_id,
                PipelineConfig {
                    subscription_name,
                    target_index,
                    id_field,
                    batch_size,
                },
                tx_clone,
                m_handle,
                ds_handle,
                p_cancel_token,
            )
            .await;
        });
    }
    drop(tx);

    let rx_stream = ReceiverStream::new(rx);
    join_set.spawn(async move {
        run_consumer_loop(rx_stream, os_client, metrics, daemon_state).await;
    });

    Ok(PipelineGeneration {
        cancel_token,
        join_set,
    })
}

fn build_opensearch_client(
    url: Url,
    os_user: String,
    os_password: String,
) -> DaemonResult<Arc<OpenSearch>> {
    let transport = TransportBuilder::new(SingleNodeConnectionPool::new(url))
        .auth(Credentials::Basic(os_user, os_password))
        .build()
        .map_err(|err| error::AppError::OpenSearchTransportBuild(err.to_string()))?;
    Ok(Arc::new(OpenSearch::new(transport)))
}
