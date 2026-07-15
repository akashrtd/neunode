use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::error::ApiError;
use super::state::ApiState;
use super::types;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SetConfigRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConfigPathResponse {
    pub path: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/config",
    responses(
        (status = 200, description = "All configuration values", body = Object)
    ),
    tag = "config",
)]
pub async fn get_config(State(state): State<Arc<ApiState>>) -> Result<impl IntoResponse, ApiError> {
    let entries: BTreeMap<String, String> =
        state.config_snapshot()?.list_all().into_iter().collect();
    Ok(types::ok(entries))
}

#[utoipa::path(
    put,
    path = "/api/v1/config",
    request_body = SetConfigRequest,
    responses(
        (status = 200, description = "Configuration value set")
    ),
    tag = "config",
)]
pub async fn set_config(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<SetConfigRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.key.is_empty() {
        return Err(ApiError::BadRequest("key cannot be empty".into()));
    }

    state.set_config_value(&body.key, &body.value)?;

    Ok(types::ack(&format!("Set {} = {}", body.key, body.value)))
}

#[utoipa::path(
    get,
    path = "/api/v1/config/path",
    responses(
        (status = 200, description = "Configuration file path", body = ConfigPathResponse)
    ),
    tag = "config",
)]
pub async fn config_path(
    State(state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(types::ok(ConfigPathResponse {
        path: state.config_snapshot()?.config_path.to_string_lossy().to_string(),
    }))
}
