//! Integration tests for the TurboQuant compression layer.
//!
//! Verifies end-to-end cross-crate interactions between neunode-turboquant
//! components: Int8Quantizer, MseQuantizer, Codebook, AdaptiveSelector,
//! and RotationMatrix.
//!
//! Tests cover:
//! - Int8 quantization roundtrip with bounded reconstruction error
//! - MSE quantization compress/decompress pipeline
//! - Codebook generation with Lloyd-Max properties
//! - AdaptiveSelector strategy dispatch
//! - RotationMatrix energy preservation
//! - Full gradient compression pipeline (rotate → quantize → transmit → dequantize → unrotate)

use neunode_turboquant::{
    AdaptiveSelector, Codebook, CodebookConfig, CompressedVector, CompressionProfile,
    Int8Quantizer, MseConfig, MseQuantizer, QuantizationStrategy, QuantizedGradients,
    RotationMatrix, RotationStrategy, TurboQuantError,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a.iter().zip(b).map(|(x, y)| *x as f64 * *y as f64).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    if norm_a < 1e-10 || norm_b < 1e-10 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Euclidean (L2) norm of a vector.
fn vec_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

/// Mean squared error between two vectors.
fn mse(a: &[f32], b: &[f32]) -> f64 {
    let n = a.len();
    assert_eq!(n, b.len());
    a.iter().zip(b).map(|(x, y)| (*x as f64 - *y as f64).powi(2)).sum::<f64>() / n as f64
}

/// Generate a deterministic pseudo-random gradient vector of the given dimension.
fn gradient_vector(dim: usize) -> Vec<f32> {
    (0..dim).map(|i| (i as f32 * 0.1).sin() * (i as f32 * 0.07).cos()).collect()
}

// ---------------------------------------------------------------------------
// Test 1: Int8Quantizer — fixed scale creation and roundtrip
// ---------------------------------------------------------------------------

#[test]
fn int8_fixed_scale_roundtrip() {
    let scale = 0.05f32;
    let q = Int8Quantizer::new(scale);

    let gradients = vec![0.5, -0.3, 0.1, -0.8, 0.2, 1.0, -1.0, 0.0];
    let quantized = q.quantize(&gradients).expect("quantize should succeed");
    let recovered = q.dequantize(&quantized);

    assert_eq!(recovered.len(), gradients.len(), "length should be preserved");

    // Each element should be within ±scale/2 of the original
    for (i, (orig, rec)) in gradients.iter().zip(&recovered).enumerate() {
        assert!(
            (orig - rec).abs() <= 0.5 * scale + 1e-6,
            "reconstruction error too large at index {i}: orig={orig}, rec={rec}, scale={scale}"
        );
    }

    assert_eq!(quantized.original_len, gradients.len());
    assert!((quantized.scale - scale).abs() < 1e-10);
    assert!((q.scale() - scale).abs() < 1e-10);
}

// ---------------------------------------------------------------------------
// Test 2: Int8Quantizer — auto scale creation
// ---------------------------------------------------------------------------

#[test]
fn int8_auto_scale_roundtrip() {
    let gradients = vec![10.0, -20.0, 30.0, -40.0, 50.0];
    let q = Int8Quantizer::new_auto(&gradients).expect("new_auto should succeed");

    // Auto scale = max_abs / 127 = 50.0 / 127.0
    let expected_scale = 50.0f32 / 127.0;
    assert!(
        (q.scale() - expected_scale).abs() < 1e-6,
        "expected scale ~{expected_scale}, got {}",
        q.scale()
    );

    let quantized = q.quantize(&gradients).expect("quantize should succeed");
    let recovered = q.dequantize(&quantized);

    // Cosine similarity should be very high (exact direction preserved)
    let cosim = cosine_similarity(&gradients, &recovered);
    assert!(cosim > 0.99, "cosine similarity too low: {cosim:.6}");

    // Max value should map to 127 and dequantize back near-exactly
    assert!(
        (recovered[4] - 50.0).abs() < q.scale() * 0.5 + 1e-6,
        "max value should be well preserved"
    );
}

// ---------------------------------------------------------------------------
// Test 3: Int8Quantizer — different gradient magnitudes
// ---------------------------------------------------------------------------

#[test]
fn int8_different_magnitudes() {
    let test_cases: Vec<(&str, Vec<f32>)> = vec![
        ("tiny", vec![0.001, -0.002, 0.003, -0.004]),
        ("small", vec![1.0, -2.0, 3.0, -4.0]),
        ("medium", vec![100.0, -200.0, 300.0, -400.0]),
        ("large", vec![1e4, -2e4, 3e4, -4e4]),
        ("mixed", vec![0.01, 100.0, -0.5, -500.0]),
    ];

    for (label, gradients) in &test_cases {
        let q = Int8Quantizer::new_auto(gradients).unwrap_or_else(|e| {
            panic!("new_auto failed for {label}: {e}");
        });
        let quantized = q.quantize(gradients).unwrap_or_else(|e| {
            panic!("quantize failed for {label}: {e}");
        });
        let recovered = q.dequantize(&quantized);

        let cosim = cosine_similarity(gradients, &recovered);
        assert!(
            cosim > 0.95,
            "cosine similarity too low for {label}: {cosim:.6} (scale={})",
            q.scale()
        );
    }
}

// ---------------------------------------------------------------------------
// Test 4: Int8Quantizer — reconstruction error bounded by scale
// ---------------------------------------------------------------------------

#[test]
fn int8_reconstruction_error_bounded() {
    let gradients: Vec<f32> = (0..256).map(|i| (i as f32 * 0.1).sin()).collect();
    let q = Int8Quantizer::new_auto(&gradients).expect("new_auto should succeed");
    let quantized = q.quantize(&gradients).expect("quantize should succeed");
    let recovered = q.dequantize(&quantized);

    let max_error =
        gradients.iter().zip(&recovered).map(|(o, r)| (o - r).abs()).fold(0.0f32, f32::max);

    // Maximum reconstruction error should be ≤ scale/2 (rounding to nearest)
    assert!(
        max_error <= 0.5 * q.scale() + 1e-5,
        "max reconstruction error {max_error} exceeds scale/2 = {}",
        0.5 * q.scale()
    );

    // RMSE should be small relative to gradient magnitudes
    let rmse = mse(&gradients, &recovered).sqrt();
    let norm = vec_norm(&gradients) as f64;
    assert!(rmse / norm < 0.01, "relative RMSE too high: {rmse:.6} / {norm:.6}");
}

// ---------------------------------------------------------------------------
// Test 5: Int8Quantizer — empty input errors
// ---------------------------------------------------------------------------

#[test]
fn int8_empty_input_errors() {
    let q = Int8Quantizer::new(1.0);
    let result = q.quantize(&[]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty"), "error should mention empty");

    let auto_result = Int8Quantizer::new_auto(&[]);
    assert!(auto_result.is_err());
}

// ---------------------------------------------------------------------------
// Test 6: Int8Quantizer — compression ratio
// ---------------------------------------------------------------------------

#[test]
fn int8_compression_ratio() {
    let q = Int8Quantizer::new(1.0);
    assert!((q.compression_ratio() - 4.0).abs() < 1e-10, "f32→i8 should be 4× compression");
}

// ---------------------------------------------------------------------------
// Test 7: MseQuantizer — 4-bit compress/decompress roundtrip
// ---------------------------------------------------------------------------

#[test]
fn mse_4bit_roundtrip() {
    let config =
        MseConfig { dimension: 64, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 42 };
    let q = MseQuantizer::new(config).expect("quantizer creation should succeed");
    let input = gradient_vector(64);

    let compressed = q.compress(&input).expect("compress should succeed");
    assert_eq!(compressed.indices.len(), 64);
    assert_eq!(compressed.bits, 4);
    assert_eq!(compressed.dimension, 64);

    let output = q.decompress(&compressed).expect("decompress should succeed");
    assert_eq!(output.len(), 64);

    let cosim = cosine_similarity(&input, &output);
    assert!(cosim > 0.7, "4-bit cosine similarity too low: {cosim:.4}");
}

// ---------------------------------------------------------------------------
// Test 8: MseQuantizer — 8-bit compress/decompress with higher fidelity
// ---------------------------------------------------------------------------

#[test]
fn mse_8bit_higher_fidelity() {
    let config =
        MseConfig { dimension: 64, bits: 8, rotation_strategy: RotationStrategy::Wht, seed: 42 };
    let q = MseQuantizer::new(config).expect("quantizer creation should succeed");
    let input = gradient_vector(64);

    let compressed = q.compress(&input).expect("compress should succeed");
    let output = q.decompress(&compressed).expect("decompress should succeed");

    let cosim = cosine_similarity(&input, &output);
    assert!(cosim > 0.8, "8-bit cosine similarity should be high: {cosim:.4}");
}

// ---------------------------------------------------------------------------
// Test 9: MseQuantizer — MSE within acceptable bounds
// ---------------------------------------------------------------------------

#[test]
fn mse_within_bounds() {
    let config =
        MseConfig { dimension: 64, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 42 };
    let q = MseQuantizer::new(config).expect("quantizer creation should succeed");
    let input = gradient_vector(64);

    let compressed = q.compress(&input).expect("compress should succeed");
    assert!(compressed.mse > 0.0, "MSE should be positive for finite-bit quantization");

    let output = q.decompress(&compressed).expect("decompress should succeed");
    let actual_mse = mse(&input, &output);
    // The compressed vector's reported MSE is for the rotated domain,
    // but actual reconstruction MSE should also be finite and reasonable
    assert!(actual_mse.is_finite(), "actual MSE should be finite");
}

// ---------------------------------------------------------------------------
// Test 10: MseQuantizer — dimension mismatch error
// ---------------------------------------------------------------------------

#[test]
fn mse_dimension_mismatch() {
    let config =
        MseConfig { dimension: 8, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 0 };
    let q = MseQuantizer::new(config).expect("quantizer creation should succeed");

    let wrong_len = vec![1.0f32, 2.0, 3.0];
    let err = q.compress(&wrong_len).unwrap_err();
    assert!(
        matches!(err, TurboQuantError::DimensionMismatch { expected: 8, actual: 3 }),
        "expected DimensionMismatch, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 11: MseQuantizer — compression ratio
// ---------------------------------------------------------------------------

#[test]
fn mse_compression_ratio() {
    let config_4bit =
        MseConfig { dimension: 8, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 0 };
    let q4 = MseQuantizer::new(config_4bit).unwrap();
    assert!((q4.compression_ratio() - 8.0).abs() < 1e-10, "4-bit = 8× compression");

    let config_8bit =
        MseConfig { dimension: 8, bits: 8, rotation_strategy: RotationStrategy::Wht, seed: 0 };
    let q8 = MseQuantizer::new(config_8bit).unwrap();
    assert!((q8.compression_ratio() - 4.0).abs() < 1e-10, "8-bit = 4× compression");
}

// ---------------------------------------------------------------------------
// Test 12: Codebook — levels count matches 2^bits
// ---------------------------------------------------------------------------

#[test]
fn codebook_levels_count() {
    for bits in [1u32, 2, 4, 8] {
        let config =
            CodebookConfig { bits, dimension: 256, num_samples: 10_000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap_or_else(|e| {
            panic!("codebook generation failed for bits={bits}: {e}");
        });
        assert_eq!(
            cb.num_levels(),
            1usize << bits,
            "bits={bits}: expected {} levels, got {}",
            1usize << bits,
            cb.num_levels()
        );
    }
}

// ---------------------------------------------------------------------------
// Test 13: Codebook — levels are sorted
// ---------------------------------------------------------------------------

#[test]
fn codebook_levels_sorted() {
    for bits in [2u32, 4, 8] {
        let config =
            CodebookConfig { bits, dimension: 256, num_samples: 10_000, ..Default::default() };
        let cb = Codebook::generate(&config).unwrap_or_else(|e| {
            panic!("codebook generation failed for bits={bits}: {e}");
        });
        for window in cb.levels.windows(2) {
            assert!(
                window[0] <= window[1],
                "bits={bits}: levels not sorted: {} > {}",
                window[0],
                window[1]
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Test 14: Codebook — determinism (same config = same output)
// ---------------------------------------------------------------------------

#[test]
fn codebook_deterministic() {
    let config =
        CodebookConfig { bits: 4, dimension: 256, num_samples: 5000, ..Default::default() };

    let cb1 = Codebook::generate(&config).expect("first generation");
    let cb2 = Codebook::generate(&config).expect("second generation");

    assert_eq!(cb1.levels, cb2.levels, "same config should produce identical levels");
    assert_eq!(cb1.iterations, cb2.iterations, "iteration counts should match");
    assert!((cb1.mse - cb2.mse).abs() < 1e-12, "MSE values should match");
}

// ---------------------------------------------------------------------------
// Test 15: Codebook — MSE decreases with more bits
// ---------------------------------------------------------------------------

#[test]
fn codebook_mse_decreases_with_bits() {
    let configs = [
        CodebookConfig { bits: 2, dimension: 256, num_samples: 5000, ..Default::default() },
        CodebookConfig { bits: 4, dimension: 256, num_samples: 5000, ..Default::default() },
        CodebookConfig { bits: 8, dimension: 256, num_samples: 5000, ..Default::default() },
    ];

    let codebooks: Vec<Codebook> = configs.iter().map(|c| Codebook::generate(c).unwrap()).collect();

    assert!(
        codebooks[0].mse > codebooks[1].mse,
        "2-bit MSE ({}) should > 4-bit MSE ({})",
        codebooks[0].mse,
        codebooks[1].mse
    );
    assert!(
        codebooks[1].mse > codebooks[2].mse,
        "4-bit MSE ({}) should > 8-bit MSE ({})",
        codebooks[1].mse,
        codebooks[2].mse
    );
}

// ---------------------------------------------------------------------------
// Test 16: Codebook — quantize_index and dequantize roundtrip
// ---------------------------------------------------------------------------

#[test]
fn codebook_quantize_dequantize_roundtrip() {
    let config =
        CodebookConfig { bits: 4, dimension: 256, num_samples: 5000, ..Default::default() };
    let cb = Codebook::generate(&config).expect("codebook generation");

    // Test roundtrip on the levels themselves (exact centroids)
    for (i, &level) in cb.levels.iter().enumerate() {
        let idx = cb.quantize_index(level);
        assert_eq!(idx, i as u32, "centroid {i} should map to index {i}");
        let recovered = cb.dequantize(idx).expect("dequantize should succeed");
        assert!(
            (level - recovered).abs() < 1e-4,
            "roundtrip mismatch at index {i}: {level} → {recovered}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 17: Codebook — slice operations
// ---------------------------------------------------------------------------

#[test]
fn codebook_slice_operations() {
    let config =
        CodebookConfig { bits: 4, dimension: 256, num_samples: 5000, ..Default::default() };
    let cb = Codebook::generate(&config).expect("codebook generation");

    let values = vec![0.0f32, 0.01, -0.01, 0.05, -0.05, 0.02];
    let indices = cb.quantize_slice(&values).expect("quantize_slice");
    let recovered = cb.dequantize_slice(&indices).expect("dequantize_slice");

    assert_eq!(indices.len(), values.len());
    assert_eq!(recovered.len(), values.len());

    // Each recovered value should be a valid centroid (close to some level)
    for &r in &recovered {
        let min_dist = cb.levels.iter().map(|&l| (r - l).abs()).fold(f32::MAX, f32::min);
        assert!(min_dist < 1e-4, "recovered value {r} is not a centroid");
    }
}

// ---------------------------------------------------------------------------
// Test 18: AdaptiveSelector — Gradient profile → Int8 strategy
// ---------------------------------------------------------------------------

#[test]
fn adaptive_gradient_returns_int8() {
    let profiles = vec![
        CompressionProfile::Gradient { workers: 1, bandwidth_mbps: 0.1 },
        CompressionProfile::Gradient { workers: 8, bandwidth_mbps: 10.0 },
        CompressionProfile::Gradient { workers: 100, bandwidth_mbps: 10000.0 },
    ];

    for profile in &profiles {
        let strategy = AdaptiveSelector::select(profile);
        assert_eq!(
            strategy,
            QuantizationStrategy::Int8,
            "gradient profiles should always select Int8"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 19: AdaptiveSelector — KvCache profile → Mse with correct bits mapping
// ---------------------------------------------------------------------------

#[test]
fn adaptive_kvcache_returns_mse() {
    // target_bits ≤ 2.0 → Mse { bits: 2.0 }
    let p1 = CompressionProfile::KvCache { target_bits: 1.5, dimension: 4096 };
    assert_eq!(AdaptiveSelector::select(&p1), QuantizationStrategy::Mse { bits: 2.0 });

    // 2.0 < target_bits ≤ 4.0 → Mse { bits: 3.5 }
    let p2 = CompressionProfile::KvCache { target_bits: 3.5, dimension: 4096 };
    assert_eq!(AdaptiveSelector::select(&p2), QuantizationStrategy::Mse { bits: 3.5 });

    // target_bits == 4.0 → still ≤ 4.0 → Mse { bits: 3.5 }
    let p3 = CompressionProfile::KvCache { target_bits: 4.0, dimension: 4096 };
    assert_eq!(AdaptiveSelector::select(&p3), QuantizationStrategy::Mse { bits: 3.5 });

    // target_bits > 4.0 → Mse { bits: 4.0 }
    let p4 = CompressionProfile::KvCache { target_bits: 6.0, dimension: 4096 };
    assert_eq!(AdaptiveSelector::select(&p4), QuantizationStrategy::Mse { bits: 4.0 });
}

// ---------------------------------------------------------------------------
// Test 20: AdaptiveSelector — Custom profile → Mse with specified bits
// ---------------------------------------------------------------------------

#[test]
fn adaptive_custom_returns_mse_with_bits() {
    let p = CompressionProfile::Custom { bits: 5, dimension: 512 };
    let strategy = AdaptiveSelector::select(&p);
    assert_eq!(strategy, QuantizationStrategy::Mse { bits: 5.0 });

    let p2 = CompressionProfile::Custom { bits: 1, dimension: 256 };
    assert_eq!(AdaptiveSelector::select(&p2), QuantizationStrategy::Mse { bits: 1.0 });
}

// ---------------------------------------------------------------------------
// Test 21: RotationMatrix — WHT strategy preserves energy
// ---------------------------------------------------------------------------

#[test]
fn rotation_wht_energy_preservation() {
    let r = RotationMatrix::new(RotationStrategy::Wht, 64, 42).expect("WHT rotation creation");
    let input = gradient_vector(64);

    let rotated = r.apply(&input).expect("apply should succeed");
    assert_eq!(rotated.len(), 64);

    let norm_in = vec_norm(&input) as f64;
    let norm_out = vec_norm(&rotated) as f64;

    assert!(
        (norm_in - norm_out).abs() / norm_in < 1e-5,
        "energy not preserved: in={norm_in:.6}, out={norm_out:.6}"
    );
}

// ---------------------------------------------------------------------------
// Test 22: RotationMatrix — QR strategy preserves energy
// ---------------------------------------------------------------------------

#[test]
fn rotation_qr_energy_preservation() {
    let dim = 7; // non-power-of-2 to exercise QR path
    let r = RotationMatrix::new(RotationStrategy::Qr, dim, 42).expect("QR rotation creation");
    let input: Vec<f32> = (0..dim).map(|i| (i as f32 * 0.3).sin()).collect();

    let rotated = r.apply(&input).expect("apply should succeed");
    assert_eq!(rotated.len(), dim);

    let norm_in = vec_norm(&input) as f64;
    let norm_out = vec_norm(&rotated) as f64;

    assert!(
        (norm_in - norm_out).abs() / norm_in < 1e-4,
        "energy not preserved: in={norm_in:.6}, out={norm_out:.6}"
    );
}

// ---------------------------------------------------------------------------
// Test 23: RotationMatrix — roundtrip identity (rotate → inverse = original)
// ---------------------------------------------------------------------------

#[test]
fn rotation_roundtrip_identity() {
    let r = RotationMatrix::new(RotationStrategy::Wht, 32, 99).expect("rotation creation");
    let input = gradient_vector(32);

    let rotated = r.apply(&input).expect("apply");
    let recovered = r.apply_inverse(&rotated).expect("apply_inverse");

    for (i, (orig, rec)) in input.iter().zip(&recovered).enumerate() {
        assert!((orig - rec).abs() < 1e-4, "roundtrip failed at index {i}: orig={orig}, rec={rec}");
    }
}

// ---------------------------------------------------------------------------
// Test 24: RotationMatrix — WHT dimension validation
// ---------------------------------------------------------------------------

#[test]
fn rotation_wht_requires_power_of_2() {
    let result = RotationMatrix::new(RotationStrategy::Wht, 3, 42);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("power of 2"), "unexpected error: {msg}");
}

// ---------------------------------------------------------------------------
// Test 25: RotationMatrix — zero dimension error
// ---------------------------------------------------------------------------

#[test]
fn rotation_zero_dimension_error() {
    let result = RotationMatrix::new(RotationStrategy::Wht, 0, 42);
    assert!(result.is_err());

    let result_qr = RotationMatrix::new(RotationStrategy::Qr, 0, 42);
    assert!(result_qr.is_err());
}

// ---------------------------------------------------------------------------
// Test 26: Full gradient compression pipeline
// ---------------------------------------------------------------------------
// Simulates the DiLoCo gradient compression pipeline:
// 1. Generate random gradient vector
// 2. Apply random rotation (energy spreading)
// 3. Quantize with Int8
// 4. "Transmit" (simulated by serializing and deserializing QuantizedGradients)
// 5. Dequantize
// 6. Apply inverse rotation
// 7. Measure reconstruction error

#[test]
fn full_gradient_compression_pipeline() {
    let dim = 64;
    let seed = 42u64;

    // Step 1: Generate "gradient" vector
    let gradient = gradient_vector(dim);
    let gradient_norm = vec_norm(&gradient);
    assert!(gradient_norm > 0.0, "gradient should have non-zero energy");

    // Step 2: Apply rotation to spread energy uniformly across dimensions
    let rotation =
        RotationMatrix::new(RotationStrategy::Wht, dim, seed).expect("rotation creation");
    let rotated = rotation.apply(&gradient).expect("rotation apply");

    // Rotation should preserve energy
    let rotated_norm = vec_norm(&rotated);
    assert!(
        (gradient_norm - rotated_norm).abs() / gradient_norm < 1e-5,
        "rotation should preserve energy: before={gradient_norm:.6}, after={rotated_norm:.6}"
    );

    // Step 3: Quantize rotated gradients with Int8
    let quantizer =
        Int8Quantizer::new_auto(&rotated).expect("quantizer creation from rotated gradients");
    let quantized = quantizer.quantize(&rotated).expect("quantization");

    // Step 4: "Transmit" — simulate by serializing and deserializing
    let wire_bytes = serde_json::to_vec(&quantized).expect("serialize");
    let received: QuantizedGradients = serde_json::from_slice(&wire_bytes).expect("deserialize");
    assert_eq!(received.data.len(), dim, "transmitted data length should match");
    assert_eq!(received.original_len, dim);

    // Step 5: Dequantize
    let dequantized = quantizer.dequantize(&received);

    // Step 6: Apply inverse rotation to get back to original domain
    let reconstructed = rotation.apply_inverse(&dequantized).expect("inverse rotation");

    // Step 7: Measure reconstruction quality
    let cosim = cosine_similarity(&gradient, &reconstructed);
    assert!(cosim > 0.95, "pipeline cosine similarity too low: {cosim:.6}");

    let reconstruction_mse = mse(&gradient, &reconstructed);
    let relative_mse = reconstruction_mse / (gradient_norm as f64).powi(2);
    assert!(
        relative_mse < 0.05,
        "relative MSE too high: {relative_mse:.6} (absolute MSE: {reconstruction_mse:.6})"
    );
}

// ---------------------------------------------------------------------------
// Test 27: Full KV cache compression pipeline with MseQuantizer
// ---------------------------------------------------------------------------
// Simulates KV cache compression: vector → rotate → codebook quantize →
// transmit → dequantize → unrotate.

#[test]
fn full_kv_cache_compression_pipeline() {
    let dim = 64;
    let seed = 42u64;

    // Create MseQuantizer (internally does rotate → codebook quantize)
    let config =
        MseConfig { dimension: dim, bits: 4, rotation_strategy: RotationStrategy::Wht, seed };
    let q = MseQuantizer::new(config).expect("MseQuantizer creation");

    let input = gradient_vector(dim);

    // Compress
    let compressed = q.compress(&input).expect("compress");
    assert_eq!(compressed.indices.len(), dim);
    assert_eq!(compressed.dimension, dim);
    assert_eq!(compressed.seed, seed);

    // "Transmit" — serialize and deserialize
    let wire = serde_json::to_vec(&compressed).expect("serialize CompressedVector");
    let received: CompressedVector = serde_json::from_slice(&wire).expect("deserialize");
    assert_eq!(received.indices, compressed.indices);

    // Decompress
    let output = q.decompress(&received).expect("decompress");
    assert_eq!(output.len(), dim);

    // Quality check
    let cosim = cosine_similarity(&input, &output);
    assert!(cosim > 0.7, "KV cache pipeline cosine similarity too low: {cosim:.4}");
}

// ---------------------------------------------------------------------------
// Test 28: Adaptive pipeline — selector drives quantizer choice
// ---------------------------------------------------------------------------
// Verify that AdaptiveSelector output can drive real quantizer instantiation.

#[test]
fn adaptive_selector_drives_quantizer_choice() {
    let gradient_profile = CompressionProfile::Gradient { workers: 4, bandwidth_mbps: 100.0 };
    let strategy = AdaptiveSelector::select(&gradient_profile);
    assert_eq!(strategy, QuantizationStrategy::Int8);

    // Use Int8 strategy to quantize gradients
    let grads = vec![1.0f32, -2.0, 3.0, -4.0, 5.0];
    let q = Int8Quantizer::new_auto(&grads).expect("Int8 from auto");
    let quantized = q.quantize(&grads).expect("quantize");
    let recovered = q.dequantize(&quantized);
    assert_eq!(recovered.len(), grads.len());

    let kvcache_profile = CompressionProfile::KvCache { target_bits: 3.5, dimension: 8 };
    let strategy = AdaptiveSelector::select(&kvcache_profile);
    assert_eq!(strategy, QuantizationStrategy::Mse { bits: 3.5 });

    // Use Mse strategy (bits=3.5 maps to nearest integer for MseConfig)
    let config = MseConfig {
        dimension: 8,
        bits: 4, // nearest integer to 3.5
        rotation_strategy: RotationStrategy::Wht,
        seed: 0,
    };
    let mq = MseQuantizer::new(config).expect("MseQuantizer creation");
    let input = vec![1.0, -2.0, 3.0, -4.0, 5.0, -6.0, 7.0, -8.0];
    let compressed = mq.compress(&input).expect("compress");
    let output = mq.decompress(&compressed).expect("decompress");
    assert_eq!(output.len(), 8);
}

// ---------------------------------------------------------------------------
// Test 29: MseConfig validation
// ---------------------------------------------------------------------------

#[test]
fn mse_config_validation() {
    // Valid config
    let valid =
        MseConfig { dimension: 8, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 0 };
    assert!(valid.validate().is_ok(), "valid config should pass");

    // Zero dimension
    let zero_dim =
        MseConfig { dimension: 0, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 0 };
    assert!(zero_dim.validate().is_err(), "zero dimension should fail");

    // Zero bits
    let zero_bits =
        MseConfig { dimension: 8, bits: 0, rotation_strategy: RotationStrategy::Wht, seed: 0 };
    assert!(zero_bits.validate().is_err(), "zero bits should fail");

    // Bits > 16
    let too_many_bits =
        MseConfig { dimension: 8, bits: 17, rotation_strategy: RotationStrategy::Wht, seed: 0 };
    assert!(too_many_bits.validate().is_err(), "bits > 16 should fail");

    // WHT with non-power-of-2 dimension
    let non_pow2 =
        MseConfig { dimension: 3, bits: 4, rotation_strategy: RotationStrategy::Wht, seed: 0 };
    assert!(non_pow2.validate().is_err(), "WHT with non-power-of-2 should fail");

    // QR with non-power-of-2 is fine
    let qr_non_pow2 =
        MseConfig { dimension: 3, bits: 4, rotation_strategy: RotationStrategy::Qr, seed: 0 };
    assert!(qr_non_pow2.validate().is_ok(), "QR with non-power-of-2 should pass");
}

// ---------------------------------------------------------------------------
// Test 30: CodebookConfig validation
// ---------------------------------------------------------------------------

#[test]
fn codebook_config_validation() {
    // Valid
    let valid = CodebookConfig::default();
    assert!(valid.validate().is_ok());

    // Zero bits
    let zero_bits = CodebookConfig { bits: 0, ..Default::default() };
    assert!(zero_bits.validate().is_err());

    // Bits > 16
    let too_many = CodebookConfig { bits: 17, ..Default::default() };
    assert!(too_many.validate().is_err());

    // Zero dimension
    let zero_dim = CodebookConfig { dimension: 0, ..Default::default() };
    assert!(zero_dim.validate().is_err());

    // Zero max_iterations
    let zero_iter = CodebookConfig { max_iterations: 0, ..Default::default() };
    assert!(zero_iter.validate().is_err());

    // Insufficient samples
    let few_samples = CodebookConfig { bits: 8, num_samples: 100, ..Default::default() };
    assert!(few_samples.validate().is_err(), "num_samples < 2^bits should fail");
}

// ---------------------------------------------------------------------------
// Test 31: RotationMatrix — is_power_of_2 utility
// ---------------------------------------------------------------------------

#[test]
fn rotation_is_power_of_2() {
    assert!(RotationMatrix::is_power_of_2(1));
    assert!(RotationMatrix::is_power_of_2(2));
    assert!(RotationMatrix::is_power_of_2(4));
    assert!(RotationMatrix::is_power_of_2(256));
    assert!(RotationMatrix::is_power_of_2(4096));

    assert!(!RotationMatrix::is_power_of_2(0));
    assert!(!RotationMatrix::is_power_of_2(3));
    assert!(!RotationMatrix::is_power_of_2(5));
    assert!(!RotationMatrix::is_power_of_2(100));
}
