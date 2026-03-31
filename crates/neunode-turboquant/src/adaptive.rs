//! Adaptive bit-width selection for TurboQuant compression.
//!
//! Chooses between Int8 quantization (training gradients) and TQ_mse
//! (KV cache inference) based on use case and bandwidth constraints.
//!
//! This module is a strategy selector / factory — it does NOT implement
//! quantization itself. It returns a [`QuantizationStrategy`] that callers
//! use to dispatch to the appropriate quantizer.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Compression profile — describes the use case and constraints
// ---------------------------------------------------------------------------

/// Describes the compression use case and its constraints.
///
/// Used by [`AdaptiveSelector`] to pick the optimal strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionProfile {
    /// Training gradient compression across distributed workers.
    Gradient {
        /// Number of workers participating in the training job.
        workers: usize,
        /// Available bandwidth between workers, in Mbps.
        bandwidth_mbps: f64,
    },
    /// KV cache compression for inference serving.
    KvCache {
        /// Target bits per element (e.g. 3.5 for the paper's sweet spot).
        target_bits: f32,
        /// Vector dimension of the KV cache entries.
        dimension: usize,
    },
    /// Custom compression for future use cases.
    Custom {
        /// Exact bits per element to use.
        bits: u8,
        /// Vector dimension.
        dimension: usize,
    },
}

// ---------------------------------------------------------------------------
// Quantization strategy — what the selector returns
// ---------------------------------------------------------------------------

/// The quantization strategy selected by [`AdaptiveSelector`].
///
/// Callers match on this variant to instantiate the appropriate quantizer
/// (e.g. `Int8Quantizer` or `MseQuantizer`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QuantizationStrategy {
    /// Use Int8Quantizer — 4× compression, suitable for training gradients.
    ///
    /// References the `int8` module's `Int8Quantizer` (API: `new`, `new_auto`,
    /// `quantize`, `dequantize`).
    Int8,
    /// Use MseQuantizer with the specified bit-width — configurable compression
    /// for KV cache and general-purpose vector compression.
    Mse {
        /// Bits per element for the MSE codebook quantizer.
        bits: f32,
    },
}

// ---------------------------------------------------------------------------
// Adaptive selector
// ---------------------------------------------------------------------------

/// Selects the optimal quantization strategy for a given compression profile.
///
/// # Selection rules
///
/// | Profile | Condition | Strategy |
/// |---------|-----------|----------|
/// | Gradient | any (TQ_prod deferred to Phase 3) | `Int8` |
/// | KvCache | target_bits ≤ 2.0 | `Mse { bits: 2.0 }` |
/// | KvCache | 2.0 < target_bits ≤ 4.0 | `Mse { bits: 3.5 }` |
/// | KvCache | target_bits > 4.0 | `Mse { bits: 4.0 }` |
/// | Custom | any | `Mse { bits: <profile.bits> }` |
pub struct AdaptiveSelector;

impl AdaptiveSelector {
    /// Pick the optimal quantization strategy for the given profile.
    ///
    /// # Gradient profiles
    ///
    /// Always returns [`QuantizationStrategy::Int8`] because TQ_prod (1–2 bit
    /// gradient compression) is deferred to Phase 3. When TQ_prod lands, the
    /// selector will choose between Int8 and TQ_prod based on bandwidth and
    /// worker count (low bandwidth + many workers → TQ_prod for maximum
    /// compression).
    ///
    /// # KV cache profiles
    ///
    /// Maps the requested `target_bits` to the nearest proven MSE bit-width:
    /// 2.0 (aggressive), 3.5 (sweet spot from the paper), or 4.0 (conservative).
    pub fn select(profile: &CompressionProfile) -> QuantizationStrategy {
        match profile {
            CompressionProfile::Gradient { workers, bandwidth_mbps } => {
                // TQ_prod (1-2 bit) is deferred to Phase 3 — always Int8 for now.
                // Future: if bandwidth < 50 Mbps && workers > 4, use TQ_prod.
                let _ = (workers, bandwidth_mbps);
                QuantizationStrategy::Int8
            }
            CompressionProfile::KvCache { target_bits, dimension: _ } => {
                let bits = if *target_bits <= 2.0 {
                    2.0
                } else if *target_bits <= 4.0 {
                    3.5
                } else {
                    4.0
                };
                QuantizationStrategy::Mse { bits }
            }
            CompressionProfile::Custom { bits, dimension: _ } => {
                QuantizationStrategy::Mse { bits: *bits as f32 }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Gradient profiles → Int8 ---

    #[test]
    fn gradient_low_bandwidth_many_workers() {
        let profile = CompressionProfile::Gradient { workers: 8, bandwidth_mbps: 10.0 };
        assert_eq!(AdaptiveSelector::select(&profile), QuantizationStrategy::Int8);
    }

    #[test]
    fn gradient_high_bandwidth_few_workers() {
        let profile = CompressionProfile::Gradient { workers: 2, bandwidth_mbps: 1000.0 };
        assert_eq!(AdaptiveSelector::select(&profile), QuantizationStrategy::Int8);
    }

    #[test]
    fn gradient_always_int8() {
        // Regardless of parameters, gradient always selects Int8 (TQ_prod deferred).
        let profiles = vec![
            CompressionProfile::Gradient { workers: 1, bandwidth_mbps: 0.1 },
            CompressionProfile::Gradient { workers: 100, bandwidth_mbps: 10000.0 },
            CompressionProfile::Gradient { workers: 4, bandwidth_mbps: 49.9 },
        ];
        for profile in &profiles {
            assert_eq!(
                AdaptiveSelector::select(profile),
                QuantizationStrategy::Int8,
                "expected Int8 for {profile:?}"
            );
        }
    }

    // --- KV cache profiles → Mse with correct bits ---

    #[test]
    fn kv_cache_low_bits() {
        let profile = CompressionProfile::KvCache { target_bits: 1.5, dimension: 4096 };
        let strategy = AdaptiveSelector::select(&profile);
        assert_eq!(strategy, QuantizationStrategy::Mse { bits: 2.0 });
    }

    #[test]
    fn kv_cache_sweet_spot() {
        let profile = CompressionProfile::KvCache { target_bits: 3.5, dimension: 4096 };
        let strategy = AdaptiveSelector::select(&profile);
        assert_eq!(strategy, QuantizationStrategy::Mse { bits: 3.5 });
    }

    #[test]
    fn kv_cache_high_bits() {
        let profile = CompressionProfile::KvCache { target_bits: 6.0, dimension: 4096 };
        let strategy = AdaptiveSelector::select(&profile);
        assert_eq!(strategy, QuantizationStrategy::Mse { bits: 4.0 });
    }

    #[test]
    fn kv_cache_exact_boundary_2() {
        // target_bits == 2.0 → maps to 2.0 (≤ check)
        let profile = CompressionProfile::KvCache { target_bits: 2.0, dimension: 2048 };
        assert_eq!(AdaptiveSelector::select(&profile), QuantizationStrategy::Mse { bits: 2.0 });
    }

    #[test]
    fn kv_cache_exact_boundary_4() {
        // target_bits == 4.0 → maps to 3.5 (≤ check)
        let profile = CompressionProfile::KvCache { target_bits: 4.0, dimension: 2048 };
        assert_eq!(AdaptiveSelector::select(&profile), QuantizationStrategy::Mse { bits: 3.5 });
    }

    // --- Custom profiles ---

    #[test]
    fn custom_profile() {
        let profile = CompressionProfile::Custom { bits: 5, dimension: 512 };
        let strategy = AdaptiveSelector::select(&profile);
        assert_eq!(strategy, QuantizationStrategy::Mse { bits: 5.0 });
    }

    #[test]
    fn custom_profile_1_bit() {
        let profile = CompressionProfile::Custom { bits: 1, dimension: 256 };
        let strategy = AdaptiveSelector::select(&profile);
        assert_eq!(strategy, QuantizationStrategy::Mse { bits: 1.0 });
    }

    // --- Serde roundtrip ---

    #[test]
    fn compression_profile_serde_roundtrip() {
        let profiles = vec![
            CompressionProfile::Gradient { workers: 4, bandwidth_mbps: 100.0 },
            CompressionProfile::KvCache { target_bits: 3.5, dimension: 8192 },
            CompressionProfile::Custom { bits: 7, dimension: 128 },
        ];
        for profile in &profiles {
            let json = serde_json::to_string(profile).unwrap();
            let decoded: CompressionProfile = serde_json::from_str(&json).unwrap();
            // Re-serialize and compare (structural equality is hard with enums)
            let json2 = serde_json::to_string(&decoded).unwrap();
            assert_eq!(json, json2, "serde roundtrip failed for {profile:?}");
        }
    }

    #[test]
    fn quantization_strategy_serde_roundtrip() {
        let strategies = vec![
            QuantizationStrategy::Int8,
            QuantizationStrategy::Mse { bits: 2.0 },
            QuantizationStrategy::Mse { bits: 3.5 },
            QuantizationStrategy::Mse { bits: 4.0 },
        ];
        for strategy in &strategies {
            let json = serde_json::to_string(strategy).unwrap();
            let decoded: QuantizationStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(*strategy, decoded, "serde roundtrip failed for {strategy:?}");
        }
    }

    // --- Debug format ---

    #[test]
    fn debug_format_compression_profile() {
        let profile = CompressionProfile::Gradient { workers: 4, bandwidth_mbps: 50.0 };
        let debug = format!("{profile:?}");
        assert!(debug.contains("Gradient"), "debug should contain variant name: {debug}");
    }

    #[test]
    fn debug_format_quantization_strategy() {
        let strategy = QuantizationStrategy::Mse { bits: 3.5 };
        let debug = format!("{strategy:?}");
        assert!(debug.contains("Mse"), "debug should contain variant name: {debug}");
        assert!(debug.contains("3.5"), "debug should contain bits value: {debug}");
    }

    // --- Strategy equality ---

    #[test]
    fn strategy_equality() {
        assert_eq!(QuantizationStrategy::Int8, QuantizationStrategy::Int8);
        assert_eq!(
            QuantizationStrategy::Mse { bits: 3.5 },
            QuantizationStrategy::Mse { bits: 3.5 }
        );
        assert_ne!(QuantizationStrategy::Int8, QuantizationStrategy::Mse { bits: 8.0 });
        assert_ne!(
            QuantizationStrategy::Mse { bits: 2.0 },
            QuantizationStrategy::Mse { bits: 4.0 }
        );
    }
}
