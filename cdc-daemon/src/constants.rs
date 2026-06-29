use std::time::Duration;

pub const CHANNEL_CAPACITY: usize = 64;
pub const CONNECT_RETRY_SECS: u64 = 5;
pub const OS_WRITE_BACKOFF_INITIAL_SECS: u64 = 2;
pub const OS_WRITE_BACKOFF_MAX_SECS: u64 = 60;
pub const TIMEOUT_DURATION: Duration = Duration::from_secs(10);
pub const CONSUMER_CONCURRENCY: usize = 8;
pub const EMPTY_FETCH_SLEEP_MS: u64 = 100;
pub const MAX_RETRIES: usize = 10;
pub const GRPC_SERVER_PORT: u16 = 50051;

pub const DEFAULT_OTLP_ENDPOINT: &str = "http://localhost:4317";
pub const DEFAULT_RW_HOST: &str = "localhost";
pub const DEFAULT_RW_PORT: &str = "4566";
pub const DEFAULT_RW_USER: &str = "root";
pub const DEFAULT_RW_DBNAME: &str = "dev";
pub const DEFAULT_OS_URL: &str = "https://localhost:9200";
pub const DEFAULT_OS_USER: &str = "admin";
pub const DEFAULT_QDRANT_URL: &str = "https://localhost:6334";
pub const DEFAULT_PIPELINES_FILE: &str = "pipelines.yaml";
pub const DEFAULT_HOSTNAME: &str = "local";
pub const DEFAULT_DLQ_DIR: &str = "/var/log/cdc-dlq";