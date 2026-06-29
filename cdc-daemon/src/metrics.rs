use crate::error::{AppError, DaemonResult};
use opentelemetry::global;
use opentelemetry::metrics::Counter;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::sync::Arc;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt, util::SubscriberInitExt};

#[allow(clippy::struct_field_names)]
pub struct PipelineMetrics {
    pub records_ingested: Counter<u64>,
    pub records_sunk_success: Counter<u64>,
    pub records_sunk_failed: Counter<u64>,
}

pub struct TelemetryRuntime {
    pub tracer_provider: SdkTracerProvider,
    pub meter_provider: SdkMeterProvider,
    pub metrics: Arc<PipelineMetrics>,
}

pub fn initialize_telemetry(otlp_endpoint: &str) -> DaemonResult<TelemetryRuntime> {
    let span_exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint)
        .build()
        .map_err(|err| AppError::Telemetry(err.to_string()))?;
    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(span_exporter)
        .build();
    let tracer = tracer_provider.tracer("rw_opensearch_sink");

    let env_filter = match EnvFilter::try_from_default_env() {
        Ok(filter) => filter,
        Err(_) => EnvFilter::new("info"),
    };

    Registry::default()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    let metric_exporter = MetricExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint)
        .build()
        .map_err(|err| AppError::Telemetry(err.to_string()))?;
    let meter_provider = SdkMeterProvider::builder()
        .with_periodic_exporter(metric_exporter)
        .build();
    global::set_meter_provider(meter_provider.clone());
    let meter = global::meter("rw_opensearch_sink");

    let metrics = Arc::new(PipelineMetrics {
        records_ingested: meter.u64_counter("pipeline_records_ingested").build(),
        records_sunk_success: meter.u64_counter("pipeline_records_sunk_success").build(),
        records_sunk_failed: meter.u64_counter("pipeline_records_sunk_failed").build(),
    });

    Ok(TelemetryRuntime {
        tracer_provider,
        meter_provider,
        metrics,
    })
}
