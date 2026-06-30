use axum::{response::Json, routing::get, Router};
use utoipa::OpenApi;

use crate::ApiDoc;

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

/// Return a Router with the same state type as the main app
pub fn swagger_routes<S>(state: std::sync::Arc<crate::AppState>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/openapi.json", get(openapi_json))
        .with_state(state)
}
