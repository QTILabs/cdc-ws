use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status, transport::Server};

#[allow(clippy::all, clippy::pedantic)]
pub mod cdc_daemon_proto {
    tonic::include_proto!("cdc_daemon");
}

use cdc_daemon_proto::cdc_management_server::{CdcManagement, CdcManagementServer};
use cdc_daemon_proto::{
    HealthRequest, HealthResponse, ListPipelinesRequest, ListPipelinesResponse, MetricsRequest,
    MetricsResponse, PipelineControlRequest, PipelineControlResponse, PipelineStatus,
    ReloadPipelinesRequest, StopDaemonRequest,
};

pub struct DaemonState {
    pub records_ingested: AtomicU64,
    pub records_sunk_success: AtomicU64,
    pub records_sunk_failed: AtomicU64,
    pub records_dlq: AtomicU64,
    pub pipelines: RwLock<HashMap<String, PipelineRuntime>>,
}

pub enum DaemonCommand {
    ReloadPipelines {
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    StopDaemon {
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
}

pub struct PipelineRuntime {
    pub config_subscription: String,
    pub target_collection: String,
    pub cursor_name: String,
    pub state: String,
    pub cancel_token: CancellationToken,
}

impl DaemonState {
    pub fn new() -> Self {
        Self {
            records_ingested: AtomicU64::new(0),
            records_sunk_success: AtomicU64::new(0),
            records_sunk_failed: AtomicU64::new(0),
            records_dlq: AtomicU64::new(0),
            pipelines: RwLock::new(HashMap::new()),
        }
    }
}

pub struct CdcGrpcService {
    pub state: Arc<DaemonState>,
    pub control_tx: UnboundedSender<DaemonCommand>,
}

#[tonic::async_trait]
impl CdcManagement for CdcGrpcService {
    async fn get_health(
        &self,
        _req: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let pipelines = self.state.pipelines.read().await;
        let mut components = HashMap::new();
        let mut is_healthy = true;

        for (_, rt) in pipelines.iter() {
            if rt.state == "ERROR" {
                is_healthy = false;
            }
            components.insert(rt.config_subscription.clone(), rt.state.clone());
        }

        Ok(Response::new(HealthResponse {
            is_healthy,
            overall_status: if is_healthy {
                "RUNNING".into()
            } else {
                "DEGRADED".into()
            },
            components,
        }))
    }

    async fn get_metrics(
        &self,
        _req: Request<MetricsRequest>,
    ) -> Result<Response<MetricsResponse>, Status> {
        Ok(Response::new(MetricsResponse {
            records_ingested: self.state.records_ingested.load(Ordering::Relaxed),
            records_sunk_success: self.state.records_sunk_success.load(Ordering::Relaxed),
            records_sunk_failed: self.state.records_sunk_failed.load(Ordering::Relaxed),
            records_dlq: self.state.records_dlq.load(Ordering::Relaxed),
        }))
    }

    async fn list_pipelines(
        &self,
        _req: Request<ListPipelinesRequest>,
    ) -> Result<Response<ListPipelinesResponse>, Status> {
        let pipelines = self.state.pipelines.read().await;
        let statuses = pipelines
            .values()
            .map(|rt| PipelineStatus {
                subscription_name: rt.config_subscription.clone(),
                target_index: rt.target_collection.clone(),
                cursor_name: rt.cursor_name.clone(),
                state: rt.state.clone(),
            })
            .collect();

        Ok(Response::new(ListPipelinesResponse {
            pipelines: statuses,
        }))
    }

    async fn pause_pipeline(
        &self,
        req: Request<PipelineControlRequest>,
    ) -> Result<Response<PipelineControlResponse>, Status> {
        let sub_name = req.into_inner().subscription_name;
        let pipelines = self.state.pipelines.read().await;

        if let Some(rt) = pipelines.get(&sub_name) {
            rt.cancel_token.cancel();
            Ok(Response::new(PipelineControlResponse {
                success: true,
                message: "Paused".into(),
            }))
        } else {
            Ok(Response::new(PipelineControlResponse {
                success: false,
                message: "Not found".into(),
            }))
        }
    }

    async fn reload_pipelines(
        &self,
        _req: Request<ReloadPipelinesRequest>,
    ) -> Result<Response<PipelineControlResponse>, Status> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.control_tx
            .send(DaemonCommand::ReloadPipelines { reply: reply_tx })
            .map_err(|_| Status::internal("failed to schedule reload"))?;

        match reply_rx.await {
            Ok(Ok(())) => Ok(Response::new(PipelineControlResponse {
                success: true,
                message: "Reload completed".into(),
            })),
            Ok(Err(message)) => Ok(Response::new(PipelineControlResponse {
                success: false,
                message,
            })),
            Err(_) => Err(Status::internal("reload acknowledgement dropped")),
        }
    }

    async fn stop_daemon(
        &self,
        _req: Request<StopDaemonRequest>,
    ) -> Result<Response<PipelineControlResponse>, Status> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.control_tx
            .send(DaemonCommand::StopDaemon { reply: reply_tx })
            .map_err(|_| Status::internal("failed to schedule shutdown"))?;

        match reply_rx.await {
            Ok(Ok(())) => Ok(Response::new(PipelineControlResponse {
                success: true,
                message: "Daemon stopping".into(),
            })),
            Ok(Err(message)) => Ok(Response::new(PipelineControlResponse {
                success: false,
                message,
            })),
            Err(_) => Err(Status::internal("shutdown acknowledgement dropped")),
        }
    }
}

pub async fn start_grpc_server(
    state: Arc<DaemonState>,
    control_tx: UnboundedSender<DaemonCommand>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("0.0.0.0:{port}").parse()?;
    tracing::info!("gRPC Management Server listening on {}", addr);

    Server::builder()
        .add_service(CdcManagementServer::new(CdcGrpcService {
            state,
            control_tx,
        }))
        .serve(addr)
        .await?;

    Ok(())
}