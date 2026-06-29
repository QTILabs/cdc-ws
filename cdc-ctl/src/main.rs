use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use std::process::{Command, Stdio};
use tonic::transport::Channel;

#[allow(clippy::all, clippy::pedantic)]
pub mod cdc_daemon_proto {
    tonic::include_proto!("cdc_daemon");
}

use cdc_daemon_proto::cdc_management_client::CdcManagementClient;
use cdc_daemon_proto::{ReloadPipelinesRequest, StopDaemonRequest};

const DEFAULT_DAEMON_URL: &str = "http://localhost:50051";
const DEFAULT_OTLP_ENDPOINT: &str = "http://localhost:4317";
const DEFAULT_RW_HOST: &str = "localhost";
const DEFAULT_RW_PORT: &str = "4566";
const DEFAULT_RW_USER: &str = "root";
const DEFAULT_RW_DBNAME: &str = "dev";
const DEFAULT_OS_URL: &str = "https://localhost:9200";
const DEFAULT_OS_USER: &str = "admin";
const DEFAULT_PIPELINES_FILE: &str = "pipelines.yaml";
const DEFAULT_HOSTNAME: &str = "local";

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("gRPC transport error: {0}")]
    GrpcTransport(#[from] tonic::transport::Error),
    #[error("gRPC status error: {0}")]
    GrpcStatus(#[from] tonic::Status),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse pipeline file: {0}")]
    PipelinesParse(String),
}

type CliResult<T> = Result<T, CliError>;

#[derive(Parser, Debug)]
#[command(name = "cdc-ctl")]
#[command(about = "CDC daemon control tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Start {
        #[arg(long, default_value = "cdc-daemon")]
        daemon_bin: String,
        #[arg(long)]
        foreground: bool,
    },
    #[command(alias = "hot-reload")]
    Reload {
        #[arg(long, default_value = DEFAULT_DAEMON_URL)]
        daemon_url: String,
    },
    Stop {
        #[arg(long, default_value = DEFAULT_DAEMON_URL)]
        daemon_url: String,
    },
    PrintConfig {
        #[arg(long, default_value = ".env")]
        env_file: String,
        #[arg(long)]
        pipelines_file: Option<String>,
    },
}

#[derive(Deserialize, Serialize, Clone)]
struct PipelineConfig {
    subscription_name: String,
    target_index: String,
    id_field: String,
    batch_size: usize,
}

#[derive(Serialize)]
struct RuntimeConfigView {
    otlp_endpoint: String,
    rw_conn_str: String,
    os_url: String,
    os_user: String,
    os_password: String,
    pipelines_path: String,
    hostname: String,
    consumer_id: String,
}

#[tokio::main]
async fn main() -> CliResult<()> {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Start {
            daemon_bin,
            foreground,
        } => start(&daemon_bin, foreground),
        Commands::Reload { daemon_url } => reload(daemon_url).await,
        Commands::Stop { daemon_url } => stop(daemon_url).await,
        Commands::PrintConfig {
            env_file,
            pipelines_file,
        } => print_config(env_file, pipelines_file).await,
    };

    if let Err(err) = result {
        eprintln!("error: {err}");
        return Err(err);
    }
    Ok(())
}

fn start(daemon_bin: &str, foreground: bool) -> CliResult<()> {
    if foreground {
        println!("starting daemon in foreground using {daemon_bin}");
        let status = Command::new(daemon_bin).status().map_err(CliError::Io)?;
        println!("daemon exited with status: {status}");
        return Ok(());
    }

    let child = Command::new(daemon_bin)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(CliError::Io)?;

    println!(
        "start: success=true message=daemon launched pid={} bin={}",
        child.id(),
        daemon_bin
    );
    Ok(())
}

async fn connect_client(daemon_url: &str) -> CliResult<CdcManagementClient<Channel>> {
    Ok(CdcManagementClient::connect(daemon_url.to_string()).await?)
}

async fn reload(daemon_url: String) -> CliResult<()> {
    let mut client = connect_client(&daemon_url).await?;
    let response = client
        .reload_pipelines(ReloadPipelinesRequest {})
        .await?
        .into_inner();
    println!(
        "reload: success={} message={}",
        response.success, response.message
    );
    Ok(())
}

async fn stop(daemon_url: String) -> CliResult<()> {
    let mut client = connect_client(&daemon_url).await?;
    let response = client.stop_daemon(StopDaemonRequest {}).await?.into_inner();
    println!(
        "stop: success={} message={}",
        response.success, response.message
    );
    Ok(())
}

async fn print_config(env_file: String, pipelines_file: Option<String>) -> CliResult<()> {
    if Path::new(&env_file).exists() {
        let _ = dotenvy::from_path_override(&env_file);
    }

    let hostname = env_or_default("HOSTNAME", DEFAULT_HOSTNAME);
    let consumer_id = std::env::var("CONSUMER_ID").unwrap_or_else(|_| hostname.clone());
    let pipelines_path =
        pipelines_file.unwrap_or_else(|| env_or_default("PIPELINES_FILE", DEFAULT_PIPELINES_FILE));

    let runtime = RuntimeConfigView {
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
        os_password: mask_secret_value(std::env::var("OS_PASSWORD").ok().as_deref()),
        pipelines_path: pipelines_path.clone(),
        hostname,
        consumer_id,
    };

    let pipelines_result = load_pipeline_configs(&pipelines_path).await;

    let output = match pipelines_result {
        Ok(pipelines) => {
            json!({
                "runtime": obfuscate_json(serde_json::to_value(runtime).unwrap_or(json!({}))),
                "pipelines": obfuscate_json(serde_json::to_value(pipelines).unwrap_or(json!([]))),
                "pipelines_error": null,
            })
        }
        Err(err) => {
            json!({
                "runtime": obfuscate_json(serde_json::to_value(runtime).unwrap_or(json!({}))),
                "pipelines": [],
                "pipelines_error": err.to_string(),
            })
        }
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    );
    Ok(())
}

async fn load_pipeline_configs(path: &str) -> CliResult<Vec<PipelineConfig>> {
    let contents = tokio::fs::read_to_string(path).await?;
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "yaml" | "yml" => {
            serde_yaml::from_str(&contents).map_err(|err| CliError::PipelinesParse(err.to_string()))
        }
        "toml" => {
            toml::from_str(&contents).map_err(|err| CliError::PipelinesParse(err.to_string()))
        }
        other => Err(CliError::PipelinesParse(format!(
            "unsupported pipeline config format '{other}'"
        ))),
    }
}

fn env_or_default(name: &'static str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn mask_secret_value(value: Option<&str>) -> String {
    match value {
        Some(v) if !v.is_empty() => mask_literal(v),
        _ => "<missing>".to_string(),
    }
}

fn mask_literal(value: &str) -> String {
    if value.len() <= 4 {
        return "****".to_string();
    }
    let prefix = &value[..2];
    let suffix = &value[value.len() - 2..];
    format!("{prefix}****{suffix}")
}

fn obfuscate_json(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut masked = serde_json::Map::new();
            for (key, val) in map {
                if looks_secret_key(&key) {
                    let replacement = if val.is_null() {
                        json!("<missing>")
                    } else {
                        json!(mask_literal(&value_to_string(&val)))
                    };
                    masked.insert(key, replacement);
                } else {
                    masked.insert(key, obfuscate_json(val));
                }
            }
            serde_json::Value::Object(masked)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(obfuscate_json).collect())
        }
        other => other,
    }
}

fn looks_secret_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    normalized.contains("password")
        || normalized.contains("secret")
        || normalized.contains("token")
        || normalized.ends_with("key")
        || normalized.contains("_key")
}

fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}
