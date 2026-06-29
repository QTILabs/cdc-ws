use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("missing required environment variable {0}")]
    MissingEnv(&'static str),

    #[error("failed to initialize the rustls crypto provider")]
    RustlsProvider,

    #[error("telemetry configuration failed: {0}")]
    Telemetry(String),

    #[error("invalid OpenSearch URL: {0}")]
    OpenSearchUrl(#[from] url::ParseError),

    #[error("failed to build OpenSearch transport: {0}")]
    OpenSearchTransportBuild(String),

    #[error("failed to initialize Qdrant client: {0}")]
    QdrantInit(String),

    #[error("Qdrant URL must use https scheme, got '{0}'")]
    InsecureQdrantUrl(String),

    #[error("failed to read pipeline configuration: {0}")]
    PipelinesRead(#[from] std::io::Error),

    #[error("failed to parse pipeline configuration: {0}")]
    PipelinesParse(String),
}

#[derive(Debug, Error)]
pub enum StreamFetchError {
    #[error("postgres fetch query failed: {0}")]
    Query(#[from] tokio_postgres::Error),

    #[error("postgres fetch query timed out")]
    Timeout,
}

pub type DaemonResult<T> = Result<T, AppError>;