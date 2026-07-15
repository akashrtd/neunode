use anyhow::{bail, Result};
use neunode_turboquant::{
    AdaptiveSelector, Codebook, CodebookConfig, CompressionProfile, QuantizationStrategy,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionParams {
    pub profile: String,
    pub workers: Option<usize>,
    pub bandwidth_mbps: Option<f64>,
    pub target_bits: Option<f32>,
    pub bits: Option<u8>,
    pub dimension: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompressionResult {
    pub strategy: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bits: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodebookParams {
    pub bits: u32,
    pub dimension: usize,
    pub max_iterations: Option<u32>,
    pub convergence_threshold: Option<f64>,
    pub num_samples: Option<usize>,
}

pub fn select_strategy(params: CompressionParams) -> Result<CompressionResult> {
    if params.dimension == 0 {
        bail!("dimension must be greater than zero");
    }
    let profile = match params.profile.as_str() {
        "gradient" => CompressionProfile::Gradient {
            workers: params.workers.unwrap_or(1),
            bandwidth_mbps: params.bandwidth_mbps.unwrap_or(100.0),
        },
        "kv_cache" => CompressionProfile::KvCache {
            target_bits: params.target_bits.unwrap_or(3.5),
            dimension: params.dimension,
        },
        "custom" => CompressionProfile::Custom {
            bits: params
                .bits
                .ok_or_else(|| anyhow::anyhow!("bits is required for custom profile"))?,
            dimension: params.dimension,
        },
        _ => bail!("profile must be gradient, kv_cache, or custom"),
    };
    Ok(match AdaptiveSelector::select(&profile) {
        QuantizationStrategy::Int8 => CompressionResult { strategy: "int8", bits: None },
        QuantizationStrategy::Mse { bits } => {
            CompressionResult { strategy: "mse", bits: Some(bits) }
        }
    })
}

pub fn generate_codebook(params: CodebookParams) -> Result<Codebook> {
    let defaults = CodebookConfig::default();
    let config = CodebookConfig {
        bits: params.bits,
        dimension: params.dimension,
        max_iterations: params.max_iterations.unwrap_or(defaults.max_iterations),
        convergence_threshold: params
            .convergence_threshold
            .unwrap_or(defaults.convergence_threshold),
        num_samples: params.num_samples.unwrap_or(defaults.num_samples),
    };
    Codebook::generate(&config).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_is_shared_domain_behavior() {
        let result = select_strategy(CompressionParams {
            profile: "kv_cache".into(),
            workers: None,
            bandwidth_mbps: None,
            target_bits: Some(3.5),
            bits: None,
            dimension: 4096,
        })
        .unwrap();
        assert_eq!(result, CompressionResult { strategy: "mse", bits: Some(3.5) });
    }

    #[test]
    fn rejects_zero_dimension() {
        let error = select_strategy(CompressionParams {
            profile: "gradient".into(),
            workers: None,
            bandwidth_mbps: None,
            target_bits: None,
            bits: None,
            dimension: 0,
        })
        .unwrap_err();
        assert_eq!(error.to_string(), "dimension must be greater than zero");
    }
}
