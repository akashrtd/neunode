use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::error::ApiError;
use crate::api::state::ApiState;
use crate::api::types;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PushModelRequest {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelsQuery {
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelResponse {
    pub id: String,
    pub base_model: Option<String>,
    pub context_length: u32,
    pub input_price_per_million: u64,
    pub output_price_per_million: u64,
    pub total_price_per_million: u64,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PushModelResponse {
    pub status: String,
    pub model_id: String,
    pub source: String,
    pub context_length: u32,
    pub input_price_per_million: u64,
    pub output_price_per_million: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RemoveModelResponse {
    pub action: String,
    pub model_id: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/models",
    params(
        ("provider" = Option<String>, Query, description = "Filter by provider substring"),
    ),
    responses(
        (status = 200, description = "Model list", body = Vec<ModelResponse>),
    ),
    tag = "models",
)]
pub async fn list_models(
    State(state): State<Arc<ApiState>>,
    Query(params): Query<ModelsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let models = load_all_models(&state.db);

    let filtered: Vec<ModelResponse> = match params.provider {
        Some(ref p) => models
            .into_iter()
            .filter(|m| m.id.contains(p.as_str()))
            .map(model_info_to_response)
            .collect(),
        None => models.into_iter().map(model_info_to_response).collect(),
    };

    Ok(types::ok(filtered))
}

#[utoipa::path(
    get,
    path = "/api/v1/models/{model_id}",
    params(
        ("model_id" = String, Path, description = "Model ID"),
    ),
    responses(
        (status = 200, description = "Model details", body = ModelResponse),
        (status = 404, description = "Model not found"),
    ),
    tag = "models",
)]
pub async fn show_model(
    State(state): State<Arc<ApiState>>,
    Path(model_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    match load_model(&state.db, &model_id)? {
        Some(m) => Ok(types::ok(model_info_to_response(m))),
        None => Err(ApiError::NotFound(format!("model not found: {model_id}"))),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/models",
    request_body = PushModelRequest,
    responses(
        (status = 201, description = "Model registered", body = PushModelResponse),
        (status = 400, description = "Bad request"),
    ),
    tag = "models",
)]
pub async fn push_model(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<PushModelRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.name.is_empty() {
        return Err(ApiError::BadRequest("model name cannot be empty".into()));
    }
    if body.path.is_empty() {
        return Err(ApiError::BadRequest("model path cannot be empty".into()));
    }

    let model = neunode_inference::provider::ModelInfo {
        id: body.name.clone(),
        base_model: None,
        context_length: 4096,
        input_price_per_million: neunode_core::types::TokenAmount(100),
        output_price_per_million: neunode_core::types::TokenAmount(200),
        capabilities: vec!["chat".to_string()],
    };

    store_model(&state.db, &model)?;

    Ok(types::created(PushModelResponse {
        status: "registered".to_string(),
        model_id: body.name,
        source: body.path,
        context_length: 4096,
        input_price_per_million: 100,
        output_price_per_million: 200,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/v1/models/{model_id}",
    params(
        ("model_id" = String, Path, description = "Model ID"),
    ),
    responses(
        (status = 200, description = "Model removed", body = RemoveModelResponse),
        (status = 404, description = "Model not found"),
    ),
    tag = "models",
)]
pub async fn remove_model(
    State(state): State<Arc<ApiState>>,
    Path(model_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    match load_model(&state.db, &model_id)? {
        Some(_) => {
            delete_model(&state.db, &model_id)?;
            Ok(types::ok(RemoveModelResponse {
                action: "remove".to_string(),
                model_id,
                status: "removed".to_string(),
            }))
        }
        None => Err(ApiError::NotFound(format!("model not found: {model_id}"))),
    }
}

// ---------------------------------------------------------------------------
// DB helpers
// ---------------------------------------------------------------------------

fn store_model(
    db: &neunode_storage::db::NeunodeDb,
    model: &neunode_inference::provider::ModelInfo,
) -> Result<(), ApiError> {
    let key = format!("model:{}", model.id);
    let key_bytes = neunode_storage::codec::serialize(&key)
        .map_err(|e| ApiError::Internal(format!("key: {e}")))?;
    let value = neunode_storage::codec::serialize(model)
        .map_err(|e| ApiError::Internal(format!("serialize model: {e}")))?;
    db.put_raw(neunode_storage::cf::CF_MODELS, &key_bytes, &value)
        .map_err(|e| ApiError::Internal(format!("store model: {e}")))?;
    Ok(())
}

fn load_all_models(
    db: &neunode_storage::db::NeunodeDb,
) -> Vec<neunode_inference::provider::ModelInfo> {
    let entries = match db.prefix_scan(neunode_storage::cf::CF_MODELS, &[]) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .iter()
        .filter(|(k, _)| {
            let key_str = neunode_storage::codec::deserialize::<String>(k).ok().unwrap_or_default();
            key_str.starts_with("model:")
        })
        .filter_map(|(_, v)| {
            neunode_storage::codec::deserialize::<neunode_inference::provider::ModelInfo>(v).ok()
        })
        .collect()
}

fn load_model(
    db: &neunode_storage::db::NeunodeDb,
    model_id: &str,
) -> Result<Option<neunode_inference::provider::ModelInfo>, ApiError> {
    let key = format!("model:{model_id}");
    let key_bytes = neunode_storage::codec::serialize(&key)
        .map_err(|e| ApiError::Internal(format!("key: {e}")))?;
    match db
        .get_raw(neunode_storage::cf::CF_MODELS, &key_bytes)
        .map_err(|e| ApiError::Internal(format!("load model: {e}")))?
    {
        Some(bytes) => {
            let model: neunode_inference::provider::ModelInfo =
                neunode_storage::codec::deserialize(&bytes)
                    .map_err(|e| ApiError::Internal(format!("deserialize: {e}")))?;
            Ok(Some(model))
        }
        None => Ok(None),
    }
}

fn delete_model(db: &neunode_storage::db::NeunodeDb, model_id: &str) -> Result<(), ApiError> {
    let key = format!("model:{model_id}");
    let key_bytes = neunode_storage::codec::serialize(&key)
        .map_err(|e| ApiError::Internal(format!("key: {e}")))?;
    db.delete(neunode_storage::cf::CF_MODELS, &key_bytes)
        .map_err(|e| ApiError::Internal(format!("delete model: {e}")))?;
    Ok(())
}

fn model_info_to_response(m: neunode_inference::provider::ModelInfo) -> ModelResponse {
    let total = m.total_price_per_million();
    ModelResponse {
        id: m.id,
        base_model: m.base_model,
        context_length: m.context_length,
        input_price_per_million: m.input_price_per_million.0 as u64,
        output_price_per_million: m.output_price_per_million.0 as u64,
        total_price_per_million: total.0 as u64,
        capabilities: m.capabilities,
    }
}
