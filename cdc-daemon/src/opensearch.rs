use crate::constants::{
    MAX_RETRIES, OS_WRITE_BACKOFF_INITIAL_SECS, OS_WRITE_BACKOFF_MAX_SECS, TIMEOUT_DURATION,
};
use crate::grpc_server::DaemonState;
use crate::metrics::PipelineMetrics;
use crate::postgres::StreamBatch;
use opensearch::{BulkParts, OpenSearch, http::request::JsonBody};
use opentelemetry::KeyValue;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::time::sleep;

/// Processes a batch of CDC records for OpenSearch ingestion.
/// Handles INSERT/UPDATE/DELETE operations and submits bulk requests.
pub async fn process_opensearch_packet(
    packet: StreamBatch,
    os_client: Arc<OpenSearch>,
    metrics: Arc<PipelineMetrics>,
    daemon_state: Arc<DaemonState>,
) {
    let mut bulk_bodies: Vec<Value> = Vec::new();
    let mut tracking_count: usize = 0;
    let labels = [KeyValue::new("target_index", packet.target_collection.clone())];

    for row in packet.rows {
        let op = row.get("op").and_then(|v| v.as_str()).unwrap_or("INSERT");
        let doc_id = row
            .get(&packet.id_field)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_default();

        if doc_id.is_empty() {
            continue;
        }

        match op {
            "INSERT" | "UPDATE_INSERT" => {
                bulk_bodies.push(json!({ "index": { "_id": doc_id } }));
                bulk_bodies.push(row);
                tracking_count += 1;
            }
            "DELETE" | "UPDATE_DELETE" => {
                bulk_bodies.push(json!({ "delete": { "_id": doc_id } }));
                tracking_count += 1;
            }
            _ => {}
        }
    }

    if bulk_bodies.is_empty() {
        return;
    }

    let _ = submit_bulk(
        os_client,
        bulk_bodies,
        tracking_count,
        &packet.target_collection,
        metrics,
        daemon_state,
        &labels,
    )
    .await;
}

/// Submits a bulk request to OpenSearch with retry logic and exponential backoff.
async fn submit_bulk(
    os_client: Arc<OpenSearch>,
    request_bodies: Vec<Value>,
    tracking_count: usize,
    target_index: &str,
    metrics: Arc<PipelineMetrics>,
    daemon_state: Arc<DaemonState>,
    labels: &[KeyValue],
) -> Result<(), ()> {
    let mut backoff = OS_WRITE_BACKOFF_INITIAL_SECS;

    for _attempt in 1..=MAX_RETRIES {
        let bodies_to_send: Vec<JsonBody<Value>> =
            request_bodies.iter().cloned().map(JsonBody::from).collect();

        if let Ok(Ok(response)) = tokio::time::timeout(
            TIMEOUT_DURATION,
            os_client
                .bulk(BulkParts::Index(target_index))
                .body(bodies_to_send)
                .send(),
        )
        .await
        {
            if response.status_code().is_success() {
                metrics
                    .records_sunk_success
                    .add(tracking_count as u64, labels);
                daemon_state
                    .records_sunk_success
                    .fetch_add(tracking_count as u64, Ordering::Relaxed);
                return Ok(());
            }
        }

        sleep(Duration::from_secs(backoff)).await;
        backoff = std::cmp::min(backoff * 2, OS_WRITE_BACKOFF_MAX_SECS);
    }

    metrics
        .records_sunk_failed
        .add(tracking_count as u64, labels);
    daemon_state
        .records_sunk_failed
        .fetch_add(tracking_count as u64, Ordering::Relaxed);
    Err(())
}