use crate::constants::CONSUMER_CONCURRENCY;
use crate::grpc_server::DaemonState;
use crate::metrics::PipelineMetrics;
use crate::opensearch::process_opensearch_packet;
use crate::postgres::StreamBatch;
use crate::qdrant::process_qdrant_packet;
use futures_util::StreamExt;
use opensearch::OpenSearch;
use qdrant_client::Qdrant;
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::error;

/// Main consumer loop that routes incoming batches to the appropriate sink
/// based on the `sink_type` field in the StreamBatch.
pub async fn run_consumer_loop(
    rx_stream: ReceiverStream<StreamBatch>,
    os_client: Option<Arc<OpenSearch>>,
    qdrant_client: Option<Arc<Qdrant>>,
    metrics: Arc<PipelineMetrics>,
    daemon_state: Arc<DaemonState>,
) {
    rx_stream
        .for_each_concurrent(CONSUMER_CONCURRENCY, |packet| {
            let os_cl = os_client.clone();
            let q_cl = qdrant_client.clone();
            let m_cl = metrics.clone();
            let ds_cl = daemon_state.clone();

            async move {
                match packet.sink_type.as_str() {
                    "opensearch" => {
                        if let Some(os_client) = os_cl {
                            process_opensearch_packet(packet, os_client, m_cl, ds_cl).await;
                        } else {
                            error!("OpenSearch sink requested but client is not configured. Dropping batch.");
                        }
                    }
                    "qdrant" => {
                        if let Some(qdrant_client) = q_cl {
                            process_qdrant_packet(packet, qdrant_client, m_cl, ds_cl).await;
                        } else {
                            error!("Qdrant sink requested but client is not configured. Dropping batch.");
                        }
                    }
                    unknown => {
                        error!(
                            "Unknown sink_type '{}' configured in pipeline. Dropping batch.",
                            unknown
                        );
                    }
                }
            }
        })
        .await;
}