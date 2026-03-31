//! Int8 gradient quantization for DiLoCo training.
//!
//! Simple linear quantization: f32 → i8 using a scale factor derived from the
//! maximum absolute value. This is the proven method used by INTELLECT-1 for
//! gradient compression in distributed training, achieving 4× bandwidth
//! reduction (32-bit float → 8-bit int).
//!
//! Pipeline:
//! 1. Compute scale = max(|gradients|) / 127.0
//! 2. Quantize: q_i = clamp(round(f_i / scale), -128, 127)
//! 3. Transmit/store i8 values + scale
//! 4. Dequantize: f_i = q_i * scale

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Result, TurboQuantError};

// ---------------------------------------------------------------------------
// Quantized representation
// ---------------------------------------------------------------------------

/// Int8-quantized gradients with the scale factor needed for reconstruction.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct QuantizedGradients {
    /// Quantized values (i8, range [-128, 127]).
    pub data: Vec<i8>,
    /// Scale factor: maps int8 range to float range.
    /// Dequantize: f_i = q_i * scale.
    pub scale: f32,
    /// Original vector length (for dimension verification on dequantize).
    #[ts(type = "number")]
    pub original_len: usize,
}

// ---------------------------------------------------------------------------
// Quantizer
// ---------------------------------------------------------------------------

/// Int8 linear quantizer for gradient compression in DiLoCo training.
///
/// No random rotation, no codebook — just simple symmetric linear quantization.
/// This is what INTELLECT-1 actually deploys (4× compression, proven at 10B
/// parameter scale across 3 continents).
#[derive(Debug)]
pub struct Int8Quantizer {
    scale: f32,
}

impl Int8Quantizer {
    /// Create a quantizer with a fixed scale factor.
    ///
    /// The scale maps the int8 range to a float range:
    /// `f_i ∈ [-128*scale, 127*scale]`.
    pub fn new(scale: f32) -> Self {
        Self { scale: scale.max(1e-8) }
    }

    /// Create a quantizer by auto-computing scale from gradient magnitudes.
    ///
    /// `scale = max(|gradients|) / 127.0`, with a floor of 1e-8 to prevent
    /// division by zero for all-zero inputs.
    pub fn new_auto(gradients: &[f32]) -> Result<Self> {
        if gradients.is_empty() {
            return Err(TurboQuantError::QuantizationFailed(
                "cannot compute scale from empty gradients".to_string(),
            ));
        }
        let max_abs = gradients.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        let scale = (max_abs / 127.0).max(1e-8);
        Ok(Self { scale })
    }

    /// Quantize f32 gradients to i8.
    ///
    /// Formula: `q_i = clamp(round(f_i / scale), -128, 127)`
    pub fn quantize(&self, gradients: &[f32]) -> Result<QuantizedGradients> {
        if gradients.is_empty() {
            return Err(TurboQuantError::QuantizationFailed(
                "cannot quantize empty gradients".to_string(),
            ));
        }

        let data: Vec<i8> = gradients
            .iter()
            .map(|&v| {
                let q = (v / self.scale).round();
                q.clamp(-128.0, 127.0) as i8
            })
            .collect();

        Ok(QuantizedGradients { data, scale: self.scale, original_len: gradients.len() })
    }

    /// Dequantize i8 values back to f32.
    ///
    /// Formula: `f_i = q_i * scale`
    pub fn dequantize(&self, quantized: &QuantizedGradients) -> Vec<f32> {
        quantized.data.iter().map(|&q| q as f32 * self.scale).collect()
    }

    /// Returns the scale factor used by this quantizer.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Compression ratio: f32 → i8 = 4.0× (32 bits / 8 bits).
    pub fn compression_ratio(&self) -> f32 {
        4.0
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

    // --- Basic quantize / dequantize ---

    #[test]
    fn quantize_dequantize_basic() {
        let q = Int8Quantizer::new(1.0);
        let grads = vec![0.0, 1.0, -1.0, 50.0, -50.0, 127.0];
        let quantized = q.quantize(&grads).unwrap();
        let recovered = q.dequantize(&quantized);
        assert_eq!(recovered.len(), grads.len());
        // Values within scale of 1.0 should be exact for integers
        for (i, (orig, rec)) in grads.iter().zip(&recovered).enumerate() {
            assert!(
                (orig - rec).abs() <= 0.5 * q.scale,
                "mismatch at index {i}: orig={orig}, rec={rec}"
            );
        }
    }

    #[test]
    fn quantize_dequantize_small_scale() {
        let q = Int8Quantizer::new(0.01);
        let grads = vec![0.5, -0.3, 0.1, 1.27];
        let quantized = q.quantize(&grads).unwrap();
        let recovered = q.dequantize(&quantized);
        for (orig, rec) in grads.iter().zip(&recovered) {
            assert!((orig - rec).abs() <= 0.5 * q.scale, "mismatch: orig={orig}, rec={rec}");
        }
    }

    #[test]
    fn quantize_dequantize_large_scale() {
        let q = Int8Quantizer::new(100.0);
        let grads = vec![10000.0, -5000.0, 12700.0];
        let quantized = q.quantize(&grads).unwrap();
        let recovered = q.dequantize(&quantized);
        for (orig, rec) in grads.iter().zip(&recovered) {
            assert!((orig - rec).abs() <= 0.5 * q.scale, "mismatch: orig={orig}, rec={rec}");
        }
    }

    // --- Auto scale ---

    #[test]
    fn new_auto_computes_scale() {
        let grads = vec![0.0, 50.0, -100.0, 25.0];
        let q = Int8Quantizer::new_auto(&grads).unwrap();
        let expected_scale = 100.0 / 127.0;
        assert!(
            (q.scale() - expected_scale).abs() < 1e-6,
            "scale={}, expected={expected_scale}",
            q.scale()
        );
    }

    #[test]
    fn new_auto_single_value() {
        let q = Int8Quantizer::new_auto(&[42.0]).unwrap();
        let expected = 42.0 / 127.0;
        assert!((q.scale() - expected).abs() < 1e-6, "scale={}, expected={expected}", q.scale());
    }

    // --- Zero gradients ---

    #[test]
    fn zero_gradients_quantize_to_zero() {
        let grads = vec![0.0f32; 10];
        let q = Int8Quantizer::new_auto(&grads).unwrap();
        let quantized = q.quantize(&grads).unwrap();
        assert!(quantized.data.iter().all(|&v| v == 0), "all zeros should quantize to 0");
    }

    #[test]
    fn zero_gradients_recover_near_zero() {
        let grads = vec![0.0f32; 10];
        let q = Int8Quantizer::new_auto(&grads).unwrap();
        let quantized = q.quantize(&grads).unwrap();
        let recovered = q.dequantize(&quantized);
        for (i, v) in recovered.iter().enumerate() {
            assert!(v.abs() < 1e-6, "recovered[{i}]={v}, expected ~0");
        }
    }

    // --- Max value preservation ---

    #[test]
    fn max_value_maps_to_127() {
        let grads = vec![100.0, -100.0, 50.0];
        let q = Int8Quantizer::new_auto(&grads).unwrap();
        let quantized = q.quantize(&grads).unwrap();
        // max abs = 100, scale = 100/127
        // 100 / scale = 127, -100 / scale = -127 (i8 range is asymmetric)
        assert_eq!(quantized.data[0], 127, "max positive should map to 127");
        assert_eq!(quantized.data[1], -127, "max negative should map to -127");
    }

    #[test]
    fn max_value_dequantizes_accurately() {
        let grads = vec![100.0, -100.0, 0.0];
        let q = Int8Quantizer::new_auto(&grads).unwrap();
        let quantized = q.quantize(&grads).unwrap();
        let recovered = q.dequantize(&quantized);
        // The max value should be preserved exactly (127 * scale = 100)
        assert!(
            (recovered[0] - 100.0).abs() < 1e-4,
            "recovered max={}, expected 100.0",
            recovered[0]
        );
    }

    // --- Roundtrip quality (cosine similarity) ---

    #[test]
    fn roundtrip_cosine_similarity_large() {
        let grads: Vec<f32> = (0..256).map(|i| (i as f32 * 0.1).sin()).collect();
        let q = Int8Quantizer::new_auto(&grads).unwrap();
        let quantized = q.quantize(&grads).unwrap();
        let recovered = q.dequantize(&quantized);
        let cosim = cosine_similarity(&grads, &recovered);
        assert!(cosim > 0.99, "roundtrip cosine similarity too low: {cosim:.6}");
    }

    #[test]
    fn roundtrip_cosine_similarity_mixed() {
        let grads: Vec<f32> = (0..128).map(|i| (i as f32 * 0.05).cos() * (i as f32)).collect();
        let q = Int8Quantizer::new_auto(&grads).unwrap();
        let quantized = q.quantize(&grads).unwrap();
        let recovered = q.dequantize(&quantized);
        let cosim = cosine_similarity(&grads, &recovered);
        assert!(cosim > 0.99, "roundtrip cosine similarity too low: {cosim:.6}");
    }

    // --- Empty input error ---

    #[test]
    fn quantize_empty_returns_error() {
        let q = Int8Quantizer::new(1.0);
        let result = q.quantize(&[]);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("empty"), "unexpected error: {msg}");
    }

    #[test]
    fn new_auto_empty_returns_error() {
        let result = Int8Quantizer::new_auto(&[]);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("empty"), "unexpected error: {msg}");
    }

    // --- Scale accessor ---

    #[test]
    fn scale_accessor() {
        let q = Int8Quantizer::new(0.5);
        assert!((q.scale() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn new_auto_scale_floor() {
        // All-zero gradients should produce scale = 1e-8 (the floor)
        let grads = vec![0.0f32; 5];
        let q = Int8Quantizer::new_auto(&grads).unwrap();
        assert!(
            (q.scale() - 1e-8).abs() < 1e-12,
            "scale should be floored at 1e-8, got {}",
            q.scale()
        );
    }

    // --- Serde roundtrip ---

    #[test]
    fn quantized_gradients_serde_roundtrip() {
        let q = Int8Quantizer::new(0.5);
        let grads = vec![1.0, -2.0, 3.0, -4.0, 5.0];
        let quantized = q.quantize(&grads).unwrap();
        let json = serde_json::to_string(&quantized).unwrap();
        let recovered: QuantizedGradients = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.data, quantized.data);
        assert!((recovered.scale - quantized.scale).abs() < 1e-10);
        assert_eq!(recovered.original_len, quantized.original_len);
    }

    // --- Determinism ---

    #[test]
    fn quantize_deterministic() {
        let grads: Vec<f32> = (0..64).map(|i| (i as f32 * 0.1).sin()).collect();
        let q = Int8Quantizer::new_auto(&grads).unwrap();
        let q1 = q.quantize(&grads).unwrap();
        let q2 = q.quantize(&grads).unwrap();
        assert_eq!(q1.data, q2.data, "same input must produce same quantized output");
        assert_eq!(q1.scale, q2.scale);
        assert_eq!(q1.original_len, q2.original_len);
    }

    // --- Boundary values (min/max i8) ---

    #[test]
    fn boundary_clamps_to_i8_range() {
        // scale = 1.0, values beyond [-128, 127] should clamp
        let q = Int8Quantizer::new(1.0);
        let grads = vec![200.0, -200.0, 128.0, -129.0];
        let quantized = q.quantize(&grads).unwrap();
        assert_eq!(quantized.data[0], 127, "200 should clamp to 127");
        assert_eq!(quantized.data[1], -128, "-200 should clamp to -128");
        assert_eq!(quantized.data[2], 127, "128 should clamp to 127");
        assert_eq!(quantized.data[3], -128, "-129 should clamp to -128");
    }

    #[test]
    fn boundary_exact_i8_range() {
        // scale = 1.0, values exactly at i8 boundaries
        let q = Int8Quantizer::new(1.0);
        let grads = vec![127.0, -128.0];
        let quantized = q.quantize(&grads).unwrap();
        assert_eq!(quantized.data[0], 127);
        assert_eq!(quantized.data[1], -128);
    }

    // --- Compression ratio ---

    #[test]
    fn compression_ratio_is_4() {
        let q = Int8Quantizer::new(1.0);
        assert!((q.compression_ratio() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn compression_ratio_invariant() {
        // Compression ratio should always be 4.0 regardless of scale
        for scale in [0.001, 0.5, 1.0, 100.0, 10000.0] {
            let q = Int8Quantizer::new(scale);
            assert!((q.compression_ratio() - 4.0).abs() < 1e-10, "failed for scale={scale}");
        }
    }

    // --- QuantizedGradients metadata ---

    #[test]
    fn quantized_gradients_original_len() {
        let q = Int8Quantizer::new(1.0);
        let grads = vec![1.0f32; 42];
        let quantized = q.quantize(&grads).unwrap();
        assert_eq!(quantized.original_len, 42);
        assert_eq!(quantized.data.len(), 42);
    }

    #[test]
    fn quantized_gradients_scale_preserved() {
        let q = Int8Quantizer::new(2.5);
        let grads = vec![10.0, -10.0];
        let quantized = q.quantize(&grads).unwrap();
        assert!((quantized.scale - 2.5).abs() < 1e-10);
    }

    // --- ts-rs export ---

    #[test]
    fn ts_export_quantized_gradients() {
        let name = QuantizedGradients::name(&Default::default());
        assert!(!name.is_empty(), "ts-rs name should be non-empty");
    }
}
