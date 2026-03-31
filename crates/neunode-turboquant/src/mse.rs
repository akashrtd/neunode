//! TQ_mse quantizer: rotate → codebook quantize → dequantize → unrotate.
//!
//! This is the core compression pipeline for MSE-optimized vector compression,
//! used primarily for KV cache compression in the inference marketplace.
//!
//! The pipeline:
//! 1. Apply random rotation (WHT or QR) to make coordinates approximately i.i.d.
//! 2. Scalar quantize each coordinate using a Lloyd-Max codebook.
//! 3. Store quantized indices as the compressed representation.
//! 4. Decompress: dequantize indices → apply inverse rotation.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::codebook::{Codebook, CodebookConfig};
use crate::error::{Result, TurboQuantError};
use crate::rotation::{RotationMatrix, RotationStrategy};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for TQ_mse quantization.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MseConfig {
    /// Vector dimension (must be power of 2 for WHT strategy).
    #[ts(type = "number")]
    pub dimension: usize,
    /// Bits per element. Integer bit-widths supported: 1–16.
    /// The "3.5-bit" claim from the paper refers to an *average* using mixed
    /// bit-widths across layers. Here we use a single integer bit-width.
    pub bits: u32,
    /// Rotation strategy.
    pub rotation_strategy: RotationStrategy,
    /// Seed for deterministic rotation matrix generation.
    #[ts(type = "number")]
    pub seed: u64,
}

impl Default for MseConfig {
    fn default() -> Self {
        Self { dimension: 4096, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 0 }
    }
}

impl MseConfig {
    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<()> {
        if self.dimension == 0 {
            return Err(TurboQuantError::QuantizationFailed("dimension must be > 0".to_string()));
        }
        if self.bits == 0 || self.bits > 16 {
            return Err(TurboQuantError::InvalidBitWidth {
                bits: self.bits,
                reason: "bits must be between 1 and 16".to_string(),
            });
        }
        if self.rotation_strategy == RotationStrategy::Wht
            && !RotationMatrix::is_power_of_2(self.dimension)
        {
            return Err(TurboQuantError::RotationFailed(format!(
                "WHT requires dimension to be a power of 2, got {}",
                self.dimension
            )));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Compressed vector
// ---------------------------------------------------------------------------

/// Result of compressing a vector with TQ_mse.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CompressedVector {
    /// Quantized indices (one per element).
    pub indices: Vec<u32>,
    /// Bits per element.
    #[ts(type = "number")]
    pub bits: u32,
    /// Original dimension.
    #[ts(type = "number")]
    pub dimension: usize,
    /// Seed used for rotation (needed for deterministic decompression).
    #[ts(type = "number")]
    pub seed: u64,
    /// Rotation strategy used.
    pub rotation_strategy: RotationStrategy,
    /// Computed MSE of this compression pass.
    pub mse: f64,
}

// ---------------------------------------------------------------------------
// Quantizer
// ---------------------------------------------------------------------------

/// The TQ_mse quantizer: rotate → quantize → dequantize → unrotate.
///
/// Holds a rotation matrix and a codebook trained for the target distribution.
/// Both are generated deterministically from the config parameters.
pub struct MseQuantizer {
    config: MseConfig,
    rotation: RotationMatrix,
    codebook: Codebook,
}

impl MseQuantizer {
    /// Create a new TQ_mse quantizer.
    ///
    /// Generates the rotation matrix and trains a Lloyd-Max codebook for the
    /// target dimension and bit-width.
    pub fn new(config: MseConfig) -> Result<Self> {
        config.validate()?;
        let rotation =
            RotationMatrix::new(config.rotation_strategy, config.dimension, config.seed)?;
        let cb_config =
            CodebookConfig { bits: config.bits, dimension: config.dimension, ..Default::default() };
        let codebook = Codebook::generate(&cb_config)?;
        Ok(Self { config, rotation, codebook })
    }

    /// Compress a vector: rotate → quantize.
    ///
    /// Returns a [`CompressedVector`] containing quantized indices and metadata
    /// needed for decompression.
    pub fn compress(&self, input: &[f32]) -> Result<CompressedVector> {
        if input.len() != self.config.dimension {
            return Err(TurboQuantError::DimensionMismatch {
                expected: self.config.dimension,
                actual: input.len(),
            });
        }

        // 1. Apply rotation: rotated = Π · input
        let rotated = self.rotation.apply(input)?;

        // 2. Quantize each element to its nearest centroid index
        let indices: Vec<u32> = rotated.iter().map(|&v| self.codebook.quantize_index(v)).collect();

        // 3. Compute MSE between rotated values and their dequantized counterparts
        let mse: f64 = rotated
            .iter()
            .zip(indices.iter())
            .map(|(&r, &idx)| {
                let d = self.codebook.dequantize(idx).unwrap_or(0.0);
                (r as f64 - d as f64).powi(2)
            })
            .sum::<f64>()
            / self.config.dimension as f64;

        Ok(CompressedVector {
            indices,
            bits: self.config.bits,
            dimension: self.config.dimension,
            seed: self.config.seed,
            rotation_strategy: self.config.rotation_strategy,
            mse,
        })
    }

    /// Decompress a vector: dequantize → unrotate.
    ///
    /// Reconstructs an approximation of the original vector from a
    /// [`CompressedVector`].
    pub fn decompress(&self, compressed: &CompressedVector) -> Result<Vec<f32>> {
        // 1. Dequantize indices to f32 values
        let dequantized = self.codebook.dequantize_slice(&compressed.indices)?;

        // 2. Apply inverse rotation: output = Πᵀ · dequantized
        let output = self.rotation.apply_inverse(&dequantized)?;

        Ok(output)
    }

    /// Get the compression ratio vs raw f32 storage.
    ///
    /// E.g., 4-bit → 32/4 = 8.0× compression.
    pub fn compression_ratio(&self) -> f64 {
        32.0 / self.config.bits as f64
    }

    /// Get a reference to the internal codebook.
    pub fn codebook(&self) -> &Codebook {
        &self.codebook
    }

    /// Get a reference to the config.
    pub fn config(&self) -> &MseConfig {
        &self.config
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: cosine similarity between two vectors.
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
        let dot: f64 = a.iter().zip(b).map(|(x, y)| *x as f64 * *y as f64).sum();
        let norm_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
        if norm_a < 1e-10 || norm_b < 1e-10 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }

    /// Helper: generate a simple test vector of given dimension.
    fn test_vector(dim: usize) -> Vec<f32> {
        (0..dim).map(|i| (i as f32 * 0.1).sin()).collect()
    }

    // --- Config validation ---

    #[test]
    fn config_default_valid() {
        assert!(MseConfig::default().validate().is_ok());
    }

    #[test]
    fn config_zero_dim_invalid() {
        let config =
            MseConfig { dimension: 0, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 0 };
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("dimension"), "unexpected error: {err}");
    }

    #[test]
    fn config_bits_range() {
        // Valid: 1 through 16
        for bits in 1..=16u32 {
            let config =
                MseConfig { dimension: 4, bits, rotation_strategy: RotationStrategy::Wht, seed: 0 };
            assert!(config.validate().is_ok(), "bits={bits} should be valid");
        }
        // Invalid: 0
        let config =
            MseConfig { dimension: 4, bits: 0, rotation_strategy: RotationStrategy::Wht, seed: 0 };
        assert!(config.validate().is_err(), "bits=0 should be invalid");
        // Invalid: 17
        let config =
            MseConfig { dimension: 4, bits: 17, rotation_strategy: RotationStrategy::Wht, seed: 0 };
        assert!(config.validate().is_err(), "bits=17 should be invalid");
    }

    #[test]
    fn config_wht_non_power_of_2() {
        let config =
            MseConfig { dimension: 3, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 0 };
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("power of 2"), "WHT with non-power-of-2 should fail: {err}");
    }

    #[test]
    fn config_qr_non_power_of_2_ok() {
        // QR strategy should accept any dimension > 0
        let config =
            MseConfig { dimension: 3, bits: 4, rotation_strategy: RotationStrategy::Qr, seed: 0 };
        assert!(config.validate().is_ok());
    }

    // --- Quantizer construction ---

    #[test]
    fn new_succeeds() {
        let config =
            MseConfig { dimension: 8, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 42 };
        let q = MseQuantizer::new(config);
        assert!(q.is_ok());
        let q = q.unwrap();
        assert_eq!(q.config().dimension, 8);
        assert_eq!(q.config().bits, 4);
        assert_eq!(q.codebook().num_levels(), 16);
    }

    #[test]
    fn new_invalid_config_propagates() {
        let config =
            MseConfig { dimension: 0, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 0 };
        assert!(MseQuantizer::new(config).is_err());
    }

    // --- Compress error cases ---

    #[test]
    fn compress_dim_mismatch() {
        let config =
            MseConfig { dimension: 4, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 0 };
        let q = MseQuantizer::new(config).unwrap();
        let short = vec![1.0f32, 2.0];
        let err = q.compress(&short).unwrap_err();
        assert!(
            matches!(err, TurboQuantError::DimensionMismatch { expected: 4, actual: 2 }),
            "expected DimensionMismatch, got {err:?}"
        );
    }

    // --- Roundtrip quality ---

    #[test]
    fn compress_decompress_shape() {
        let config =
            MseConfig { dimension: 8, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 42 };
        let q = MseQuantizer::new(config).unwrap();
        let input = test_vector(8);
        let compressed = q.compress(&input).unwrap();
        assert_eq!(compressed.indices.len(), 8);
        let output = q.decompress(&compressed).unwrap();
        assert_eq!(output.len(), 8);
    }

    #[test]
    fn compress_decompress_4bit() {
        let config = MseConfig {
            dimension: 64,
            bits: 4,
            rotation_strategy: RotationStrategy::Wht,
            seed: 42,
        };
        let q = MseQuantizer::new(config).unwrap();
        let input = test_vector(64);
        let compressed = q.compress(&input).unwrap();
        let output = q.decompress(&compressed).unwrap();
        let cosim = cosine_similarity(&input, &output);
        assert!(cosim > 0.7, "4-bit roundtrip cosine similarity too low: {cosim:.4}");
    }

    #[test]
    fn compress_decompress_8bit() {
        let config = MseConfig {
            dimension: 64,
            bits: 8,
            rotation_strategy: RotationStrategy::Wht,
            seed: 42,
        };
        let q = MseQuantizer::new(config).unwrap();
        let input = test_vector(64);
        let compressed = q.compress(&input).unwrap();
        let output = q.decompress(&compressed).unwrap();
        let cosim = cosine_similarity(&input, &output);
        assert!(cosim > 0.8, "8-bit roundtrip cosine similarity too low: {cosim:.4}");
    }

    // --- MSE properties ---

    #[test]
    fn mse_positive() {
        let config =
            MseConfig { dimension: 8, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 42 };
        let q = MseQuantizer::new(config).unwrap();
        let input = test_vector(8);
        let compressed = q.compress(&input).unwrap();
        assert!(compressed.mse > 0.0, "MSE should be positive for finite-bit quantization");
    }

    #[test]
    fn mse_decreases_with_bits() {
        let input = test_vector(64);

        let config_4 = MseConfig {
            dimension: 64,
            bits: 4,
            rotation_strategy: RotationStrategy::Wht,
            seed: 42,
        };
        let config_8 = MseConfig {
            dimension: 64,
            bits: 8,
            rotation_strategy: RotationStrategy::Wht,
            seed: 42,
        };

        let q4 = MseQuantizer::new(config_4).unwrap();
        let q8 = MseQuantizer::new(config_8).unwrap();

        let c4 = q4.compress(&input).unwrap();
        let c8 = q8.compress(&input).unwrap();

        assert!(c4.mse > c8.mse, "4-bit MSE ({}) should be > 8-bit MSE ({})", c4.mse, c8.mse);
    }

    // --- Compression ratio ---

    #[test]
    fn compression_ratio_4bit() {
        let config =
            MseConfig { dimension: 8, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 0 };
        let q = MseQuantizer::new(config).unwrap();
        assert!((q.compression_ratio() - 8.0).abs() < 1e-10);
    }

    #[test]
    fn compression_ratio_8bit() {
        let config =
            MseConfig { dimension: 8, bits: 8, rotation_strategy: RotationStrategy::Wht, seed: 0 };
        let q = MseQuantizer::new(config).unwrap();
        assert!((q.compression_ratio() - 4.0).abs() < 1e-10);
    }

    // --- Determinism ---

    #[test]
    fn compress_deterministic() {
        let config =
            MseConfig { dimension: 8, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 42 };
        let q = MseQuantizer::new(config).unwrap();
        let input = test_vector(8);
        let c1 = q.compress(&input).unwrap();
        let c2 = q.compress(&input).unwrap();
        assert_eq!(c1.indices, c2.indices, "same input must produce same indices");
        assert!((c1.mse - c2.mse).abs() < 1e-12);
    }

    #[test]
    fn decompress_deterministic() {
        let config =
            MseConfig { dimension: 8, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 42 };
        let q = MseQuantizer::new(config).unwrap();
        let input = test_vector(8);
        let compressed = q.compress(&input).unwrap();
        let out1 = q.decompress(&compressed).unwrap();
        let out2 = q.decompress(&compressed).unwrap();
        assert_eq!(out1, out2, "same compressed must produce same output");
    }

    // --- Special vectors ---

    #[test]
    fn compress_zero_vector() {
        let config =
            MseConfig { dimension: 8, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 42 };
        let q = MseQuantizer::new(config).unwrap();
        let zero = vec![0.0f32; 8];
        let compressed = q.compress(&zero).unwrap();
        let output = q.decompress(&compressed).unwrap();
        // Zero → rotation spreads nothing → quantized to centroids near zero
        // Reconstructed values should be small
        let max_val = output.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        assert!(max_val < 0.1, "zero vector reconstruction should be near zero, max={max_val}");
    }

    #[test]
    fn compress_unit_vector() {
        let config =
            MseConfig { dimension: 8, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 42 };
        let q = MseQuantizer::new(config).unwrap();
        let mut e = vec![0.0f32; 8];
        e[3] = 1.0;
        let compressed = q.compress(&e).unwrap();
        let output = q.decompress(&compressed).unwrap();
        // Should reconstruct something with similar energy
        let norm_in: f64 = e.iter().map(|v| (*v as f64).powi(2)).sum::<f64>().sqrt();
        let norm_out: f64 = output.iter().map(|v| (*v as f64).powi(2)).sum::<f64>().sqrt();
        assert!(
            (norm_in - norm_out).abs() / norm_in < 1.0,
            "unit vector norm: in={norm_in}, out={norm_out}"
        );
    }

    // --- Serde ---

    #[test]
    fn config_serde_roundtrip() {
        let config = MseConfig {
            dimension: 128,
            bits: 3,
            rotation_strategy: RotationStrategy::Qr,
            seed: 999,
        };
        let json = serde_json::to_string(&config).unwrap();
        let config2: MseConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.dimension, config2.dimension);
        assert_eq!(config.bits, config2.bits);
        assert_eq!(config.rotation_strategy, config2.rotation_strategy);
        assert_eq!(config.seed, config2.seed);
    }

    #[test]
    fn compressed_vector_serde_roundtrip() {
        let config =
            MseConfig { dimension: 8, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 42 };
        let q = MseQuantizer::new(config).unwrap();
        let input = test_vector(8);
        let compressed = q.compress(&input).unwrap();
        let json = serde_json::to_string(&compressed).unwrap();
        let compressed2: CompressedVector = serde_json::from_str(&json).unwrap();
        assert_eq!(compressed.indices, compressed2.indices);
        assert_eq!(compressed.bits, compressed2.bits);
        assert_eq!(compressed.dimension, compressed2.dimension);
        assert_eq!(compressed.seed, compressed2.seed);
    }

    // --- ts-rs exports ---

    #[test]
    fn ts_export_mse_config() {
        let name = MseConfig::name(&Default::default());
        assert!(!name.is_empty(), "ts-rs name should be non-empty");
    }

    #[test]
    fn ts_export_compressed_vector() {
        let name = CompressedVector::name(&Default::default());
        assert!(!name.is_empty(), "ts-rs name should be non-empty");
    }

    // --- Various dimensions ---

    #[test]
    fn compress_dim_4() {
        let config =
            MseConfig { dimension: 4, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 42 };
        let q = MseQuantizer::new(config).unwrap();
        let input = vec![1.0, -2.0, 3.0, -4.0];
        let compressed = q.compress(&input).unwrap();
        assert_eq!(compressed.indices.len(), 4);
        let output = q.decompress(&compressed).unwrap();
        assert_eq!(output.len(), 4);
    }

    #[test]
    fn compress_dim_256() {
        let config = MseConfig {
            dimension: 256,
            bits: 4,
            rotation_strategy: RotationStrategy::Wht,
            seed: 42,
        };
        let q = MseQuantizer::new(config).unwrap();
        let input = test_vector(256);
        let compressed = q.compress(&input).unwrap();
        assert_eq!(compressed.indices.len(), 256);
        let output = q.decompress(&compressed).unwrap();
        assert_eq!(output.len(), 256);
        // Larger dimension → better quantization after rotation
        let cosim = cosine_similarity(&input, &output);
        assert!(cosim > 0.7, "dim-256 roundtrip cosim={cosim:.4}");
    }

    // --- QR strategy roundtrip ---

    #[test]
    fn qr_strategy_roundtrip() {
        let config = MseConfig {
            dimension: 5, // non-power-of-2
            bits: 4,
            rotation_strategy: RotationStrategy::Qr,
            seed: 42,
        };
        let q = MseQuantizer::new(config).unwrap();
        let input: Vec<f32> = vec![1.0, -2.0, 3.0, -4.0, 5.0];
        let compressed = q.compress(&input).unwrap();
        let output = q.decompress(&compressed).unwrap();
        let cosim = cosine_similarity(&input, &output);
        assert!(cosim > 0.5, "QR roundtrip cosim={cosim:.4}");
    }

    // --- Accessor tests ---

    #[test]
    fn codebook_accessor() {
        let config =
            MseConfig { dimension: 8, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 0 };
        let q = MseQuantizer::new(config).unwrap();
        assert_eq!(q.codebook().bits, 4);
        assert_eq!(q.codebook().num_levels(), 16);
    }

    #[test]
    fn config_accessor() {
        let config = MseConfig {
            dimension: 16,
            bits: 8,
            rotation_strategy: RotationStrategy::Wht,
            seed: 123,
        };
        let q = MseQuantizer::new(config).unwrap();
        assert_eq!(q.config().dimension, 16);
        assert_eq!(q.config().bits, 8);
        assert_eq!(q.config().seed, 123);
    }

    // --- Compressed vector metadata ---

    #[test]
    fn compressed_vector_metadata() {
        let config =
            MseConfig { dimension: 8, bits: 3, rotation_strategy: RotationStrategy::Wht, seed: 77 };
        let q = MseQuantizer::new(config).unwrap();
        let input = test_vector(8);
        let compressed = q.compress(&input).unwrap();
        assert_eq!(compressed.bits, 3);
        assert_eq!(compressed.dimension, 8);
        assert_eq!(compressed.seed, 77);
        assert_eq!(compressed.rotation_strategy, RotationStrategy::Wht);
    }
}
