use crate::api::error::ApiError;
use crate::api::types;
use crate::turboquant_service::{self, CodebookParams, CompressionParams};
use axum::response::IntoResponse;
use axum::Json;

pub async fn select_strategy(
    Json(body): Json<CompressionParams>,
) -> Result<impl IntoResponse, ApiError> {
    let response = turboquant_service::select_strategy(body)
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(types::ok(response))
}

pub async fn generate_codebook(
    Json(body): Json<CodebookParams>,
) -> Result<impl IntoResponse, ApiError> {
    let codebook = turboquant_service::generate_codebook(body)
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(types::ok(codebook))
}
