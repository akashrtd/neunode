use axum::response::IntoResponse;
use axum::Json;
use neunode_turboquant::{
    AdaptiveSelector, Codebook, CodebookConfig, CompressionProfile, QuantizationStrategy,
};
use serde::{Deserialize, Serialize};

use crate::api::error::ApiError;
use crate::api::types;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionRequest {
    profile: String,
    workers: Option<usize>,
    bandwidth_mbps: Option<f64>,
    target_bits: Option<f32>,
    bits: Option<u8>,
    dimension: usize,
}

#[derive(Debug, Serialize)]
pub struct CompressionResponse {
    strategy: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    bits: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodebookRequest {
    bits: u32,
    dimension: usize,
    max_iterations: Option<u32>,
    convergence_threshold: Option<f64>,
    num_samples: Option<usize>,
}

pub async fn select_strategy(
    Json(body): Json<CompressionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.dimension == 0 {
        return Err(ApiError::BadRequest("dimension must be greater than zero".into()));
    }
    let profile = match body.profile.as_str() {
        "gradient" => CompressionProfile::Gradient {
            workers: body.workers.unwrap_or(1),
            bandwidth_mbps: body.bandwidth_mbps.unwrap_or(100.0),
        },
        "kv_cache" => CompressionProfile::KvCache {
            target_bits: body.target_bits.unwrap_or(3.5),
            dimension: body.dimension,
        },
        "custom" => CompressionProfile::Custom {
            bits: body.bits.ok_or_else(|| {
                ApiError::BadRequest("bits is required for custom profile".into())
            })?,
            dimension: body.dimension,
        },
        _ => {
            return Err(ApiError::BadRequest(
                "profile must be gradient, kv_cache, or custom".into(),
            ));
        }
    };
    let response = match AdaptiveSelector::select(&profile) {
        QuantizationStrategy::Int8 => CompressionResponse { strategy: "int8", bits: None },
        QuantizationStrategy::Mse { bits } => {
            CompressionResponse { strategy: "mse", bits: Some(bits) }
        }
    };
    Ok(types::ok(response))
}

pub async fn generate_codebook(
    Json(body): Json<CodebookRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let defaults = CodebookConfig::default();
    let config = CodebookConfig {
        bits: body.bits,
        dimension: body.dimension,
        max_iterations: body.max_iterations.unwrap_or(defaults.max_iterations),
        convergence_threshold: body.convergence_threshold.unwrap_or(defaults.convergence_threshold),
        num_samples: body.num_samples.unwrap_or(defaults.num_samples),
    };
    let codebook =
        Codebook::generate(&config).map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(types::ok(codebook))
}
