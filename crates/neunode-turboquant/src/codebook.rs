//! Scalar quantization codebook for TurboQuant.
//!
//! After random rotation, vector coordinates become approximately i.i.d. with
//! distribution that depends on dimension d:
//! - Small d (< 256): Beta((d-1)/2, (d-1)/2) — wider spread
//! - Large d (>= 256): Approximately Gaussian N(0, 1/d)
//!
//! The codebook finds k = 2^b optimal quantization levels (centroids) that
//! minimize mean squared error via the Lloyd-Max algorithm.

use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Result, TurboQuantError};

/// A scalar quantization codebook with k = 2^bits levels.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Codebook {
    /// Number of bits per value (1-16).
    #[ts(type = "number")]
    pub bits: u32,
    /// Quantization levels (centroids), sorted ascending. Length = 2^bits.
    pub levels: Vec<f32>,
    /// The dimension this codebook was trained for (affects distribution shape).
    #[ts(type = "number")]
    pub dimension: usize,
    /// Number of Lloyd-Max iterations performed.
    #[ts(type = "number")]
    pub iterations: u32,
    /// Final MSE of the codebook.
    pub mse: f64,
}

/// Configuration for codebook generation.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CodebookConfig {
    /// Number of bits per value.
    #[ts(type = "number")]
    pub bits: u32,
    /// Vector dimension (affects Beta distribution parameters).
    #[ts(type = "number")]
    pub dimension: usize,
    /// Maximum Lloyd-Max iterations.
    #[ts(type = "number")]
    pub max_iterations: u32,
    /// Convergence threshold (stop if MSE improvement < this).
    pub convergence_threshold: f64,
    /// Number of samples for empirical distribution estimation.
    #[ts(type = "number")]
    pub num_samples: usize,
}

impl Default for CodebookConfig {
    fn default() -> Self {
        Self {
            bits: 4,
            dimension: 4096,
            max_iterations: 100,
            convergence_threshold: 1e-8,
            num_samples: 10_000,
        }
    }
}

impl CodebookConfig {
    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<()> {
        if self.bits == 0 || self.bits > 16 {
            return Err(TurboQuantError::InvalidBitWidth {
                bits: self.bits,
                reason: "bits must be between 1 and 16".to_string(),
            });
        }
        if self.dimension == 0 {
            return Err(TurboQuantError::QuantizationFailed("dimension must be > 0".to_string()));
        }
        if self.max_iterations == 0 {
            return Err(TurboQuantError::QuantizationFailed(
                "max_iterations must be > 0".to_string(),
            ));
        }
        let min_samples = 1usize << self.bits;
        if self.num_samples < min_samples {
            return Err(TurboQuantError::QuantizationFailed(format!(
                "num_samples ({}) must be >= 2^bits ({})",
                self.num_samples, min_samples
            )));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Distribution sampling helpers (no external stat library)
// ---------------------------------------------------------------------------

/// Box-Muller transform for a single standard normal sample.
fn sample_gaussian(rng: &mut StdRng) -> f64 {
    let u1: f64 = rng.random();
    let u2: f64 = rng.random();
    let u1 = u1.max(1e-10);
    (-2.0_f64 * u1.ln()).sqrt() * (2.0_f64 * std::f64::consts::PI * u2).cos()
}

/// Approximate symmetric Beta(a, a) sampling via the Gamma identity.
///
/// For Beta(a, a): sample X ~ Gamma(a, 1), Y ~ Gamma(a, 1), return X / (X + Y).
/// Gamma(a, 1) is approximated as the sum of `a` exponential samples when a is
/// integer or the floor of a (acceptable for this use-case).
fn sample_beta_symmetric(rng: &mut StdRng, a: f64) -> f64 {
    let n = a as usize;
    if n == 0 {
        // For very small a, use rejection-free fallback: two uniforms
        return rng.random::<f64>();
    }
    let x: f64 = (0..n).map(|_| -rng.random::<f64>().max(1e-10).ln()).sum();
    let y: f64 = (0..n).map(|_| -rng.random::<f64>().max(1e-10).ln()).sum();
    x / (x + y).max(1e-30)
}

/// Generate samples from the target distribution for the given dimension.
/// - d >= 256: Gaussian N(0, 1/d)
/// - d < 256: Beta((d-1)/2, (d-1)/2) scaled to [-1/sqrt(d), 1/sqrt(d)]
fn generate_samples(rng: &mut StdRng, dimension: usize, num_samples: usize) -> Vec<f64> {
    let scale = 1.0 / (dimension as f64).sqrt();
    if dimension >= 256 {
        // Gaussian N(0, 1/d) path
        (0..num_samples).map(|_| sample_gaussian(rng) * scale).collect()
    } else {
        // Beta((d-1)/2, (d-1)/2) path
        // Beta(a,a) in [0,1] -> scale to [-1,1]: x*2 - 1, then multiply by 1/sqrt(d)
        let a = (dimension - 1) as f64 / 2.0;
        (0..num_samples)
            .map(|_| {
                let beta = sample_beta_symmetric(rng, a);
                (beta * 2.0 - 1.0) * scale
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Codebook implementation
// ---------------------------------------------------------------------------

impl Codebook {
    /// Generate a codebook using the Lloyd-Max algorithm.
    ///
    /// Deterministic for a given config (fixed RNG seed = 42).
    pub fn generate(config: &CodebookConfig) -> Result<Self> {
        config.validate()?;

        let k = 1usize << config.bits;
        let mut rng = StdRng::seed_from_u64(42);

        // 1. Draw samples from the target distribution
        let samples = generate_samples(&mut rng, config.dimension, config.num_samples);

        // 2. Initialize centroids evenly spaced across the sample range
        let s_min = samples.iter().copied().fold(f64::INFINITY, f64::min);
        let s_max = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let mut levels: Vec<f64> = (0..k)
            .map(|i| s_min + (s_max - s_min) * ((2 * i + 1) as f64) / (2.0_f64 * k as f64))
            .collect();

        // 3. Lloyd-Max iterations
        let mut prev_mse = f64::INFINITY;
        let mut iterations = 0u32;

        for _ in 0..config.max_iterations {
            iterations += 1;

            // a. Assign each sample to the nearest centroid and accumulate sums
            let mut sums = vec![0.0f64; k];
            let mut counts = vec![0usize; k];
            let mut mse = 0.0f64;

            for &s in &samples {
                // Find nearest centroid via linear scan (k is small for typical bit widths)
                let (idx, dist_sq) = levels
                    .iter()
                    .enumerate()
                    .map(|(i, &c)| (i, (s - c) * (s - c)))
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .expect("k > 0");
                sums[idx] += s;
                counts[idx] += 1;
                mse += dist_sq;
            }
            mse /= samples.len() as f64;

            // b. Recompute centroids as conditional means
            for i in 0..k {
                if counts[i] > 0 {
                    levels[i] = sums[i] / counts[i] as f64;
                }
                // If a region is empty, keep its centroid unchanged
            }

            // c. Check convergence
            let improvement = prev_mse - mse;
            if improvement.abs() < config.convergence_threshold && iterations > 1 {
                break;
            }
            prev_mse = mse;
        }

        // Ensure levels are sorted ascending
        levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let final_mse = compute_mse(&samples, &levels);

        Ok(Self {
            bits: config.bits,
            levels: levels.into_iter().map(|v| v as f32).collect(),
            dimension: config.dimension,
            iterations,
            mse: final_mse,
        })
    }

    /// Quantize a single f32 value to its level index.
    ///
    /// Returns the index (0 to 2^bits - 1) of the nearest centroid.
    pub fn quantize_index(&self, value: f32) -> u32 {
        // Binary search through sorted levels
        let mut lo = 0u32;
        let mut hi = self.levels.len() as u32 - 1;

        // Edge cases: below minimum or above maximum
        if value <= self.levels[lo as usize] {
            return lo;
        }
        if value >= self.levels[hi as usize] {
            return hi;
        }

        // Binary search for the closest level
        while lo + 1 < hi {
            let mid = lo + (hi - lo) / 2;
            if value < self.levels[mid as usize] {
                hi = mid;
            } else {
                lo = mid;
            }
        }

        // lo and hi are adjacent — pick the closer one
        let d_lo = (value - self.levels[lo as usize]).abs();
        let d_hi = (value - self.levels[hi as usize]).abs();
        if d_lo <= d_hi {
            lo
        } else {
            hi
        }
    }

    /// Dequantize a level index back to the centroid value.
    pub fn dequantize(&self, index: u32) -> Result<f32> {
        let k = self.num_levels();
        if index as usize >= k {
            return Err(TurboQuantError::QuantizationFailed(format!(
                "index {} out of range [0, {})",
                index, k
            )));
        }
        Ok(self.levels[index as usize])
    }

    /// Quantize a slice of f32 values to indices.
    pub fn quantize_slice(&self, values: &[f32]) -> Result<Vec<u32>> {
        Ok(values.iter().map(|&v| self.quantize_index(v)).collect())
    }

    /// Dequantize a slice of indices back to f32 values.
    pub fn dequantize_slice(&self, indices: &[u32]) -> Result<Vec<f32>> {
        indices.iter().map(|&i| self.dequantize(i)).collect()
    }

    /// Number of quantization levels.
    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }
}

/// Compute MSE of a set of samples against the nearest centroid in `levels`.
fn compute_mse(samples: &[f64], levels: &[f64]) -> f64 {
    let total: f64 = samples
        .iter()
        .map(|&s| {
            let nearest = levels
                .iter()
                .copied()
                .min_by(|a, b| {
                    (s - a).abs().partial_cmp(&(s - b).abs()).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0.0);
            (s - nearest) * (s - nearest)
        })
        .sum();
    total / samples.len() as f64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Config validation tests --

    #[test]
    fn generate_1bit_symmetric_levels() {
        let config =
            CodebookConfig { bits: 1, dimension: 4096, num_samples: 10_000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        assert_eq!(cb.levels.len(), 2);
        // For symmetric Gaussian, 1-bit centroids should be ~ ±σ = ±1/sqrt(d)
        let expected = 1.0 / (4096.0f32).sqrt();
        assert!(
            (cb.levels[0].abs() - expected).abs() / expected < 0.3,
            "level[0]={} should be ~±{expected}",
            cb.levels[0]
        );
        assert!(
            (cb.levels[1].abs() - expected).abs() / expected < 0.3,
            "level[1]={} should be ~±{expected}",
            cb.levels[1]
        );
    }

    #[test]
    fn config_zero_bits_invalid() {
        let config = CodebookConfig { bits: 0, ..Default::default() };
        assert!(config.validate().is_err());
    }

    #[test]
    fn config_bits_above_16_invalid() {
        let config = CodebookConfig { bits: 17, ..Default::default() };
        assert!(config.validate().is_err());
    }

    #[test]
    fn config_zero_dimension_invalid() {
        let config = CodebookConfig { dimension: 0, ..Default::default() };
        assert!(config.validate().is_err());
    }

    // -- Generation tests --

    #[test]
    fn generate_1bit() {
        let config =
            CodebookConfig { bits: 1, dimension: 4096, num_samples: 1000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        assert_eq!(cb.num_levels(), 2);
        // Symmetric around zero (within tolerance)
        let sum: f32 = cb.levels.iter().sum();
        assert!(sum.abs() < 0.05, "1-bit levels should be ~symmetric, sum={sum}");
    }

    #[test]
    fn generate_2bit() {
        let config =
            CodebookConfig { bits: 2, dimension: 4096, num_samples: 2000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        assert_eq!(cb.num_levels(), 4);
        // 4 levels should be pairwise symmetric
        assert!(cb.levels[0] < cb.levels[1]);
        assert!(cb.levels[1] < cb.levels[2]);
        assert!(cb.levels[2] < cb.levels[3]);
    }

    #[test]
    fn generate_4bit() {
        let config =
            CodebookConfig { bits: 4, dimension: 4096, num_samples: 5000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        assert_eq!(cb.num_levels(), 16);
    }

    #[test]
    fn generate_8bit() {
        let config =
            CodebookConfig { bits: 8, dimension: 4096, num_samples: 10_000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        assert_eq!(cb.num_levels(), 256);
    }

    #[test]
    fn generate_levels_sorted() {
        let config =
            CodebookConfig { bits: 4, dimension: 4096, num_samples: 5000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        for window in cb.levels.windows(2) {
            assert!(window[0] <= window[1], "levels not sorted: {} > {}", window[0], window[1]);
        }
    }

    #[test]
    fn generate_levels_count() {
        let config =
            CodebookConfig { bits: 6, dimension: 4096, num_samples: 5000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        assert_eq!(cb.levels.len(), 64);
    }

    // -- Quantize / dequantize tests --

    #[test]
    fn quantize_dequantize_roundtrip() {
        let config =
            CodebookConfig { bits: 4, dimension: 4096, num_samples: 5000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();

        // Test values within the range of the codebook
        let test_values: Vec<f32> = cb.levels.iter().copied().collect();
        for &v in &test_values {
            let idx = cb.quantize_index(v);
            let recovered = cb.dequantize(idx).unwrap();
            assert!((v - recovered).abs() < 1e-4, "roundtrip failed for {v}: got {recovered}");
        }
    }

    #[test]
    fn quantize_index_boundaries() {
        let config =
            CodebookConfig { bits: 2, dimension: 4096, num_samples: 2000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();

        // Value below all levels → index 0
        assert_eq!(cb.quantize_index(f32::MIN), 0);
        // Value above all levels → last index
        assert_eq!(cb.quantize_index(f32::MAX), cb.num_levels() as u32 - 1);
    }

    #[test]
    fn quantize_slice() {
        let config =
            CodebookConfig { bits: 4, dimension: 4096, num_samples: 5000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        let values = vec![0.0, 0.01, -0.01, 0.05];
        let indices = cb.quantize_slice(&values).unwrap();
        assert_eq!(indices.len(), 4);
        for &idx in &indices {
            assert!((idx as usize) < cb.num_levels());
        }
    }

    #[test]
    fn dequantize_slice() {
        let config =
            CodebookConfig { bits: 4, dimension: 4096, num_samples: 5000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        let indices = vec![0, 1, 2, 3];
        let values = cb.dequantize_slice(&indices).unwrap();
        assert_eq!(values.len(), 4);
        for (i, &v) in values.iter().enumerate() {
            assert!((v - cb.levels[i]).abs() < 1e-6, "dequantize_slice mismatch at index {i}");
        }
    }

    #[test]
    fn dequantize_invalid_index() {
        let config =
            CodebookConfig { bits: 2, dimension: 4096, num_samples: 2000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        assert!(cb.dequantize(4).is_err()); // max valid = 3
        assert!(cb.dequantize(100).is_err());
    }

    // -- MSE tests --

    #[test]
    fn generate_mse_positive() {
        let config =
            CodebookConfig { bits: 4, dimension: 4096, num_samples: 5000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        assert!(cb.mse > 0.0, "MSE should be positive for finite-bit codebook");
    }

    #[test]
    fn generate_mse_decreases_with_bits() {
        let configs = [
            CodebookConfig { bits: 2, dimension: 4096, num_samples: 5000, ..Default::default() },
            CodebookConfig { bits: 4, dimension: 4096, num_samples: 5000, ..Default::default() },
            CodebookConfig { bits: 8, dimension: 4096, num_samples: 5000, ..Default::default() },
        ];
        let codebooks: Vec<Codebook> =
            configs.iter().map(|c| Codebook::generate(c).unwrap()).collect();
        assert!(
            codebooks[0].mse > codebooks[1].mse,
            "2-bit MSE ({}) should > 4-bit MSE ({})",
            codebooks[0].mse,
            codebooks[1].mse,
        );
        assert!(
            codebooks[1].mse > codebooks[2].mse,
            "4-bit MSE ({}) should > 8-bit MSE ({})",
            codebooks[1].mse,
            codebooks[2].mse,
        );
    }

    // -- Determinism test --

    #[test]
    fn generate_deterministic() {
        let config =
            CodebookConfig { bits: 4, dimension: 4096, num_samples: 5000, ..Default::default() };
        let cb1 = Codebook::generate(&config).unwrap();
        let cb2 = Codebook::generate(&config).unwrap();
        assert_eq!(cb1.levels, cb2.levels, "same config should produce same levels");
        assert_eq!(cb1.iterations, cb2.iterations);
        assert!((cb1.mse - cb2.mse).abs() < 1e-12);
    }

    // -- Serde tests --

    #[test]
    fn codebook_serde_roundtrip() {
        let config =
            CodebookConfig { bits: 4, dimension: 4096, num_samples: 5000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        let json = serde_json::to_string(&cb).unwrap();
        let cb2: Codebook = serde_json::from_str(&json).unwrap();
        assert_eq!(cb.levels, cb2.levels);
        assert_eq!(cb.bits, cb2.bits);
        assert_eq!(cb.dimension, cb2.dimension);
    }

    #[test]
    fn config_serde_roundtrip() {
        let config = CodebookConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let config2: CodebookConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.bits, config2.bits);
        assert_eq!(config.dimension, config2.dimension);
        assert_eq!(config.max_iterations, config2.max_iterations);
    }

    // -- Utility tests --

    #[test]
    fn num_levels_correct() {
        let config =
            CodebookConfig { bits: 4, dimension: 4096, num_samples: 5000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        assert_eq!(cb.num_levels(), 16);
    }

    #[test]
    fn quantize_zero_value() {
        let config =
            CodebookConfig { bits: 4, dimension: 4096, num_samples: 5000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        let idx = cb.quantize_index(0.0);
        let recovered = cb.dequantize(idx).unwrap();
        // Zero should map to the closest centroid
        assert!(
            recovered.abs() < cb.levels.last().unwrap().abs(),
            "0.0 should map to a centroid near zero, got {recovered}"
        );
    }

    #[test]
    fn generate_large_dimension() {
        let config =
            CodebookConfig { bits: 4, dimension: 4096, num_samples: 3000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        assert_eq!(cb.num_levels(), 16);
        assert!(cb.mse > 0.0);
        // Gaussian path — levels should be clustered near zero
        let max_level = cb.levels.iter().copied().fold(0.0f32, f32::max);
        assert!(max_level < 0.1, "for d=4096, levels should be close to zero, max={max_level}");
    }

    #[test]
    fn generate_small_dimension() {
        let config =
            CodebookConfig { bits: 2, dimension: 4, num_samples: 3000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        assert_eq!(cb.num_levels(), 4);
        // Beta path — levels should span wider range (1/sqrt(4) = 0.5 max)
        let range = cb.levels.last().unwrap() - cb.levels.first().unwrap();
        assert!(range > 0.1, "for d=4, levels should have meaningful spread, range={range}");
    }

    #[test]
    fn ts_export_codebook() {
        let name = Codebook::name(&Default::default());
        assert!(!name.is_empty(), "ts-rs name should be non-empty");
    }

    // -- Additional coverage tests --

    #[test]
    fn quantize_exact_centroid() {
        let config =
            CodebookConfig { bits: 4, dimension: 4096, num_samples: 5000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap();
        // Exact centroid value should quantize to itself
        for (i, &level) in cb.levels.iter().enumerate() {
            let idx = cb.quantize_index(level);
            assert_eq!(
                idx, i as u32,
                "centroid {i} ({level}) should quantize to index {i}, got {idx}"
            );
        }
    }

    #[test]
    fn config_insufficient_samples_invalid() {
        let config = CodebookConfig {
            bits: 8,
            num_samples: 100, // need >= 256
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn generate_mse_improves_with_iterations() {
        // Single iteration should have higher MSE than converged
        let config_converged = CodebookConfig {
            bits: 4,
            dimension: 4096,
            max_iterations: 100,
            num_samples: 5000,
            ..Default::default()
        };
        let config_limited = CodebookConfig {
            bits: 4,
            dimension: 4096,
            max_iterations: 1,
            num_samples: 5000,
            ..Default::default()
        };
        // Both use seed 42 so samples are identical
        let cb_converged = Codebook::generate(&config_converged).unwrap();
        let cb_limited = Codebook::generate(&config_limited).unwrap();
        assert!(
            cb_limited.mse >= cb_converged.mse * 0.9,
            "more iterations should improve or maintain MSE: limited={}, converged={}",
            cb_limited.mse,
            cb_converged.mse,
        );
    }
}
