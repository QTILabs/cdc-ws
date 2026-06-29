mod auth;
mod error;

use axum::{
    Json, Router,
    extract::State,
    http::{Method, StatusCode},
    middleware,
    routing::{get, post},
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tonic::transport::Channel;
use tower_http::cors::{Any, CorsLayer};

#[allow(clippy::all, clippy::pedantic)]
pub mod cdc_daemon_proto {
    tonic::include_proto!("cdc_daemon");
}
use cdc_daemon_proto::cdc_management_client::CdcManagementClient;

#[derive(Clone)]
struct AppState {
    grpc_client: CdcManagementClient<Channel>,
    auth_state: Arc<auth::AuthState>,
}

#[derive(Serialize)]
struct HealthResponseBody {
    is_healthy: bool,
    overall_status: String,
    components: HashMap<String, String>,
}

#[allow(clippy::struct_field_names)]
#[derive(Serialize)]
struct MetricsResponseBody {
    records_ingested: u64,
    records_sunk_success: u64,
    records_sunk_failed: u64,
    records_dlq: u64,
}

#[derive(Serialize)]
struct PipelineStatusBody {
    subscription_name: String,
    target_index: String,
    cursor_name: String,
    state: String,
}

#[derive(Serialize)]
struct ControlResponseBody {
    success: bool,
    message: String,
}

#[derive(Serialize)]
struct ListPipelinesResponseBody {
    pipelines: Vec<PipelineStatusBody>,
}

#[tokio::main]
async fn main() -> error::AppResult<()> {
    let grpc_url = env_or_default("CDC_DAEMON_GRPC_URL", "http://localhost:50051");
    let grpc_client = CdcManagementClient::connect(grpc_url).await?;

    let jwt_secret = env_or_default("JWT_SECRET", "super_secret_key_change_me");
    let mut local_users = HashMap::new();
    let admin_hash = bcrypt::hash("admin_password", bcrypt::DEFAULT_COST)?;
    local_users.insert("admin".to_string(), admin_hash);

    let mut oauth_providers = HashMap::new();
    if let (Ok(id), Ok(secret)) = (
        std::env::var("GITHUB_CLIENT_ID"),
        std::env::var("GITHUB_CLIENT_SECRET"),
    ) {
        let redirect_url = env_or_default(
            "GITHUB_REDIRECT_URL",
            "http://localhost:8080/api/auth/oauth2/github/callback",
        );

        oauth_providers.insert(
            "github".to_string(),
            auth::OAuthProvider {
                client_id: id,
                client_secret: secret,
                auth_url: "https://github.com/login/oauth/authorize".into(),
                token_url: "https://github.com/login/oauth/access_token".into(),
                redirect_url,
                userinfo_url: "https://api.github.com/user".into(),
            },
        );
    }

    if let (Ok(id), Ok(secret), Ok(issuer)) = (
        std::env::var("KEYCLOAK_CLIENT_ID"),
        std::env::var("KEYCLOAK_CLIENT_SECRET"),
        std::env::var("KEYCLOAK_ISSUER"),
    ) {
        let issuer = issuer.trim_end_matches('/');
        let redirect_url = env_or_default(
            "KEYCLOAK_REDIRECT_URL",
            "http://localhost:8080/api/auth/oauth2/keycloak/callback",
        );

        oauth_providers.insert(
            "keycloak".to_string(),
            auth::OAuthProvider {
                client_id: id,
                client_secret: secret,
                auth_url: format!("{issuer}/protocol/openid-connect/auth"),
                token_url: format!("{issuer}/protocol/openid-connect/token"),
                redirect_url,
                userinfo_url: format!("{issuer}/protocol/openid-connect/userinfo"),
            },
        );
    }

    let auth_state = Arc::new(auth::AuthState {
        jwt_secret: jwt_secret.into_bytes(),
        local_users,
        oauth_providers,
        pkce_verifiers: tokio::sync::RwLock::new(HashMap::new()),
    });

    let state = Arc::new(AppState {
        grpc_client,
        auth_state,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let auth_routes = Router::new()
        .route("/api/auth/login", post(auth::login_local))
        .route("/api/auth/oauth2/{provider}/login", get(auth::oauth_login))
        .route(
            "/api/auth/oauth2/{provider}/callback",
            get(auth::oauth_callback),
        );

    let cdc_routes = Router::new()
        .route("/api/cdc/health", get(get_health))
        .route("/api/cdc/metrics", get(get_metrics))
        .route("/api/cdc/pipelines", get(list_pipelines))
        .route("/api/cdc/pipelines/reload", post(reload_pipelines))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));

    let app = Router::new()
        .merge(auth_routes)
        .merge(cdc_routes)
        .layer(cors)
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    println!("BFF REST API (with Auth) listening on http://localhost:8080");
    axum::serve(listener, app).await?;
    Ok(())
}

fn env_or_default(name: &'static str, default: &str) -> String {
    match std::env::var(name) {
        Ok(value) => value,
        Err(_) => default.to_string(),
    }
}

#[axum::debug_handler]
async fn get_health(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthResponseBody>, StatusCode> {
    let mut client = state.grpc_client.clone();
    client
        .get_health(cdc_daemon_proto::HealthRequest {})
        .await
        .map(|r| {
            let response = r.into_inner();
            Json(HealthResponseBody {
                is_healthy: response.is_healthy,
                overall_status: response.overall_status,
                components: response.components,
            })
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[axum::debug_handler]
async fn get_metrics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<MetricsResponseBody>, StatusCode> {
    let mut client = state.grpc_client.clone();
    client
        .get_metrics(cdc_daemon_proto::MetricsRequest {})
        .await
        .map(|r| {
            let response = r.into_inner();
            Json(MetricsResponseBody {
                records_ingested: response.records_ingested,
                records_sunk_success: response.records_sunk_success,
                records_sunk_failed: response.records_sunk_failed,
                records_dlq: response.records_dlq,
            })
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[axum::debug_handler]
async fn list_pipelines(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ListPipelinesResponseBody>, StatusCode> {
    let mut client = state.grpc_client.clone();
    client
        .list_pipelines(cdc_daemon_proto::ListPipelinesRequest {})
        .await
        .map(|r| {
            let response = r.into_inner();
            let pipelines = response
                .pipelines
                .into_iter()
                .map(|pipeline| PipelineStatusBody {
                    subscription_name: pipeline.subscription_name,
                    target_index: pipeline.target_index,
                    cursor_name: pipeline.cursor_name,
                    state: pipeline.state,
                })
                .collect();
            Json(ListPipelinesResponseBody { pipelines })
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[axum::debug_handler]
async fn reload_pipelines(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ControlResponseBody>, StatusCode> {
    let mut client = state.grpc_client.clone();
    client
        .reload_pipelines(cdc_daemon_proto::ReloadPipelinesRequest {})
        .await
        .map(|r| {
            let response = r.into_inner();
            Json(ControlResponseBody {
                success: response.success,
                message: response.message,
            })
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
