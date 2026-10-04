use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;

use super::state::ApiState;
use super::types;

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_hint: String,
    pub identity_ready: bool,
    pub mesh_running: bool,
    pub ledger_authority: String,
    pub training_executor: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    ),
    tag = "system",
)]
pub async fn health_handler(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    let identity_ready = state.require_did().is_ok();
    let mesh = state.mesh_handle.read().await;
    let mesh_running = match mesh.as_ref() {
        Some(handle) => handle.status().await.is_ok_and(|status| status.running),
        None => false,
    };
    types::ok(HealthResponse {
        status: if identity_ready && mesh_running { "ready" } else { "bootstrap_required" }.into(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_hint: "see /api/v1/mesh/status".to_string(),
        identity_ready,
        mesh_running,
        ledger_authority: "local_rocksdb".into(),
        training_executor: "unavailable".into(),
    })
}
