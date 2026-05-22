use std::sync::Arc;

use axum::response::IntoResponse;
use axum::extract::State;

use super::state::ApiState;
use super::types;

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_hint: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    ),
    tag = "system",
)]
pub async fn health_handler(
    State(_state): State<Arc<ApiState>>,
) -> impl IntoResponse {
    types::ok(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_hint: "see /api/v1/mesh/status".to_string(),
    })
}
