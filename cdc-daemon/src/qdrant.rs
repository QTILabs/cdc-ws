use crate::constants::TIMEOUT_DURATION;
use crate::dlq;
use crate::grpc_server::DaemonState;
use crate::metrics::PipelineMetrics;
use crate::postgres::StreamBatch;
use opentelemetry::KeyValue;
use qdrant_client::Payload;
use qdrant_client::qdrant::{
    DeletePointsBuilder, PointId, PointStruct, PointsIdsList, Struct,
    UpsertPointsBuilder, Value as QdrantValue, Vector, point_id::PointIdOptions,
    points_selector::PointsSelectorOneOf, value::Kind,
};
use qdrant_client::Qdrant;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;
use serde_json::{json, Map as JsonMap, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tracing::{error, info, warn};

/// Recursively converts serde_json::Value to Qdrant's Protobuf Value
fn json_to_qdrant_value(val: &Value) -> QdrantValue {
    let kind = match val {
        Value::Null => Kind::NullValue(0),
        Value::Bool(b) => Kind::BoolValue(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Kind::IntegerValue(i)
            } else if let Some(f) = n.as_f64() {
                Kind::DoubleValue(f)
            } else {
                Kind::StringValue(n.to_string())
            }
        }
        Value::String(s) => Kind::StringValue(s.clone()),
        Value::Array(arr) => Kind::ListValue(qdrant_client::qdrant::ListValue {
            values: arr.iter().map(json_to_qdrant_value).collect(),
        }),
        Value::Object(map) => Kind::StructValue(Struct {
            fields: map
                .iter()
                .map(|(k, v)| (k.clone(), json_to_qdrant_value(v)))
                .collect(),
        }),
    };
    QdrantValue { kind: Some(kind) }
}

/// Resolves a Point ID from a JSON value (Qdrant requires UUID or u64)
fn resolve_point_id(raw_id: Option<&Value>) -> Option<PointId> {
    match raw_id {
        Some(Value::String(s)) => {
            if let Ok(uuid) = uuid::Uuid::parse_str(s) {
                Some(PointId {
                    point_id_options: Some(PointIdOptions::Uuid(uuid.to_string())),
                })
            } else {
                // Fallback: hash string to u64
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                s.hash(&mut hasher);
                Some(PointId {
                    point_id_options: Some(PointIdOptions::Num(hasher.finish())),
                })
            }
        }
        Some(Value::Number(n)) => n.as_u64().map(|value| PointId {
            point_id_options: Some(PointIdOptions::Num(value)),
        }),
        _ => None,
    }
}

pub async fn process_qdrant_packet(
    packet: StreamBatch,
    qdrant_client: Arc<Qdrant>,
    metrics: Arc<PipelineMetrics>,
    daemon_state: Arc<DaemonState>,
) {
    let collection = &packet.target_collection;
    let vector_field = packet.vector_field.as_deref().unwrap_or("embedding");
    let labels = [KeyValue::new("target_collection", collection.clone())];

    let mut points_to_upsert: Vec<PointStruct> = Vec::new();
    let mut point_ids_to_delete: Vec<PointId> = Vec::new();
    let mut tracking_count = 0;

    for row in packet.rows {
        let op = row.get("op").and_then(|v| v.as_str()).unwrap_or("INSERT");
        let raw_id = row.get(&packet.id_field);

        let point_id = match resolve_point_id(raw_id) {
            Some(id) => id,
            None => {
                warn!("Qdrant Sink: Record missing valid ID field. Sending to DLQ.");
                dlq::write_record(collection, row, "Missing or invalid Qdrant Point ID").await;
                continue;
            }
        };

        match op {
            "INSERT" | "UPDATE_INSERT" => {
                // Extract vector
                let vector_data: Vec<f32> = match row.get(vector_field) {
                    Some(Value::Array(arr)) => arr
                        .iter()
                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                        .collect(),
                    _ => {
                        warn!(
                            "Qdrant Sink: Missing or invalid vector field '{}'. Sending to DLQ.",
                            vector_field
                        );
                        dlq::write_record(
                            collection,
                            row,
                            &format!("Missing vector field '{}' for Qdrant", vector_field),
                        )
                        .await;
                        continue;
                    }
                };

                // Build payload (all fields except vector and op)
                let mut payload = HashMap::new();
                if let Value::Object(map) = &row {
                    for (k, v) in map {
                        if k != vector_field && k != "op" {
                            payload.insert(k.clone(), json_to_qdrant_value(v));
                        }
                    }
                }

                points_to_upsert.push(PointStruct::new(
                    point_id,
                    vector_data,
                    Payload::from(payload),
                ));
                tracking_count += 1;
            }
            "DELETE" | "UPDATE_DELETE" => {
                point_ids_to_delete.push(point_id);
                tracking_count += 1;
            }
            _ => {}
        }
    }

    if !points_to_upsert.is_empty() {
        let _ = submit_qdrant_upsert(
            qdrant_client.clone(),
            collection,
            points_to_upsert,
            tracking_count,
            metrics.clone(),
            daemon_state.clone(),
            &labels,
        )
        .await;
    }

    if !point_ids_to_delete.is_empty() {
        let _ = submit_qdrant_delete(
            qdrant_client.clone(),
            collection,
            point_ids_to_delete,
            tracking_count,
            metrics.clone(),
            daemon_state.clone(),
            &labels,
        )
        .await;
    }
}

async fn submit_qdrant_upsert(
    client: Arc<Qdrant>,
    collection: &str,
    points: Vec<PointStruct>,
    count: usize,
    metrics: Arc<PipelineMetrics>,
    state: Arc<DaemonState>,
    labels: &[KeyValue],
) -> Result<(), ()> {
    let builder = UpsertPointsBuilder::new(collection, points).wait(true);

    match tokio::time::timeout(TIMEOUT_DURATION, client.upsert_points(builder)).await {
        Ok(Ok(_)) => {
            metrics.records_sunk_success.add(count as u64, labels);
            state
                .records_sunk_success
                .fetch_add(count as u64, Ordering::Relaxed);
            Ok(())
        }
        Ok(Err(e)) => {
            error!("Qdrant Upsert failed: {:?}", e);
            metrics.records_sunk_failed.add(count as u64, labels);
            state
                .records_sunk_failed
                .fetch_add(count as u64, Ordering::Relaxed);
            Err(())
        }
        Err(_) => {
            error!("Qdrant Upsert timed out");
            metrics.records_sunk_failed.add(count as u64, labels);
            state
                .records_sunk_failed
                .fetch_add(count as u64, Ordering::Relaxed);
            Err(())
        }
    }
}

async fn submit_qdrant_delete(
    client: Arc<Qdrant>,
    collection: &str,
    ids: Vec<PointId>,
    count: usize,
    metrics: Arc<PipelineMetrics>,
    state: Arc<DaemonState>,
    labels: &[KeyValue],
) -> Result<(), ()> {
    let builder = DeletePointsBuilder::new(collection)
        .wait(true)
        .points(PointsSelectorOneOf::Points(PointsIdsList { ids }));

    match tokio::time::timeout(TIMEOUT_DURATION, client.delete_points(builder)).await {
        Ok(Ok(_)) => {
            metrics.records_sunk_success.add(count as u64, labels);
            state
                .records_sunk_success
                .fetch_add(count as u64, Ordering::Relaxed);
            Ok(())
        }
        Ok(Err(e)) => {
            error!("Qdrant Delete failed: {:?}", e);
            metrics.records_sunk_failed.add(count as u64, labels);
            state
                .records_sunk_failed
                .fetch_add(count as u64, Ordering::Relaxed);
            Err(())
        }
        Err(_) => {
            error!("Qdrant Delete timed out");
            metrics.records_sunk_failed.add(count as u64, labels);
            state
                .records_sunk_failed
                .fetch_add(count as u64, Ordering::Relaxed);
            Err(())
        }
    }
}

// =========================================================================
// Embedding-Capable Qdrant Sink (CdcSink trait abstraction)
// =========================================================================

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkRecord {
    pub id: String,
    pub action: String,
    pub payload: Value,
    pub schema_name: String,
    pub table_name: String,
}

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum QdrantSinkError {
    #[error("Qdrant client error: {0}")]
    ClientError(#[from] anyhow::Error),

    #[error("Embedding engine failure: {0}")]
    EmbeddingError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
}

#[allow(dead_code)]
#[async_trait]
pub trait CdcSink: Send + Sync {
    async fn write_batch(&self, records: &[SinkRecord]) -> Result<(), QdrantSinkError>;
    fn name(&self) -> &str;
}

#[allow(dead_code)]
pub struct QdrantSink {
    qdrant_client: Qdrant,
    collection_name: String,
    embedding_client: reqwest::Client,
    embedding_url: String,
    embedding_dimension: usize,
    vector_field: String,
    payload_fields: Vec<String>,
}

#[allow(dead_code)]
impl QdrantSink {
    pub fn new(
        qdrant_url: String,
        qdrant_api_key: Option<String>,
        collection_name: String,
        embedding_url: String,
        embedding_dimension: usize,
        vector_field: String,
        payload_fields: Vec<String>,
    ) -> Result<Self, QdrantSinkError> {
        let mut builder = Qdrant::from_url(&qdrant_url)
            .timeout(Duration::from_secs(10));
        if let Some(key) = qdrant_api_key {
            builder = builder.api_key(key);
        }
        let qdrant_client = builder.build().map_err(anyhow::Error::from)?;

        let embedding_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .pool_max_idle_per_host(50)
            .build()?;

        Ok(Self {
            qdrant_client,
            collection_name,
            embedding_client,
            embedding_url,
            embedding_dimension,
            vector_field,
            payload_fields,
        })
    }

    fn calculate_point_id(&self, record: &SinkRecord) -> PointId {
        let ns = Uuid::new_v5(
            &Uuid::NAMESPACE_DNS,
            format!("cdc.sovereign.qdrant.{}", self.collection_name).as_bytes(),
        );
        PointId::from(Uuid::new_v5(&ns, record.id.as_bytes()).to_string())
    }

    async fn generate_local_embedding(&self, text_input: &str) -> Result<Vec<f32>, QdrantSinkError> {
        let body = json!({ "model": "local-bge-large-en", "prompt": text_input });
        let mut delay = Duration::from_millis(100);
        let max_retries = 5u32;

        for attempt in 1..=max_retries {
            let res = self.embedding_client
                .post(&self.embedding_url)
                .json(&body)
                .send()
                .await;

            match res {
                Ok(response) if response.status().is_success() => {
                    #[derive(Deserialize)]
                    struct EmbeddingResponse { embedding: Vec<f32> }
                    if let Ok(data) = response.json::<EmbeddingResponse>().await {
                        if data.embedding.len() == self.embedding_dimension {
                            return Ok(data.embedding);
                        }
                        return Err(QdrantSinkError::EmbeddingError(format!(
                            "Dimension mismatch: expected {}, got {}",
                            self.embedding_dimension, data.embedding.len()
                        )));
                    }
                }
                _ => {
                    if attempt == max_retries { break; }
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
            }
        }

        Err(QdrantSinkError::EmbeddingError(format!(
            "Failed to generate embedding after {} attempts.", max_retries
        )))
    }
}

#[async_trait]
impl CdcSink for QdrantSink {
    fn name(&self) -> &str { "QdrantVectorSink" }

    async fn write_batch(&self, records: &[SinkRecord]) -> Result<(), QdrantSinkError> {
        if records.is_empty() { return Ok(()); }

        let mut points_to_upsert = Vec::new();
        let mut points_to_delete = Vec::new();

        for record in records {
            let point_id = self.calculate_point_id(record);

            if record.action.to_uppercase() == "DELETE" {
                points_to_delete.push(point_id);
                continue;
            }

            let text_to_vectorize = record.payload.get(&self.vector_field)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if text_to_vectorize.is_empty() {
                warn!(?point_id, "Vector field '{}' empty or missing. Skipping.", self.vector_field);
                continue;
            }

            let vector = match self.generate_local_embedding(text_to_vectorize).await {
                Ok(v) => v,
                Err(err) => {
                    error!("Embedding failed: {:?}", err);
                    continue;
                }
            };

            let mut payload_map = JsonMap::new();
            payload_map.insert("cdc_schema".to_string(), json!(record.schema_name));
            payload_map.insert("cdc_table".to_string(), json!(record.table_name));
            payload_map.insert("cdc_processed_at".to_string(), json!(chrono::Utc::now().to_rfc3339()));
            for key in &self.payload_fields {
                if let Some(val) = record.payload.get(key) {
                    payload_map.insert(key.clone(), val.clone());
                }
            }

            let payload = Payload::try_from(Value::Object(payload_map))
                .map_err(anyhow::Error::from)?;

            points_to_upsert.push(PointStruct::new(point_id, Vector::from(vector), payload));
        }

        if !points_to_upsert.is_empty() {
            self.qdrant_client
                .upsert_points(
                    UpsertPointsBuilder::new(&self.collection_name, points_to_upsert)
                        .wait(true)
                        .build(),
                )
                .await
                .map_err(|e| QdrantSinkError::ClientError(anyhow::Error::from(e)))?;
            info!("Qdrant upsert batch committed via gRPC.");
        }

        if !points_to_delete.is_empty() {
            self.qdrant_client
                .delete_points(
                    DeletePointsBuilder::new(&self.collection_name)
                        .points(points_to_delete)
                        .wait(true)
                        .build(),
                )
                .await
                .map_err(|e| QdrantSinkError::ClientError(anyhow::Error::from(e)))?;
            info!("Qdrant delete batch committed via gRPC.");
        }

        Ok(())
    }
}