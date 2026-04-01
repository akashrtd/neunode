//! Rotation matrix module for TurboQuant compression.
//!
//! Two strategies:
//! - **WHT** (primary): Fast Walsh-Hadamard Transform with random sign flips. O(d log d).
//!   Construction: Π = D₁·H·D₂ where D₁,D₂ are random ±1 diagonals.
//! - **QR** (fallback): Gram-Schmidt on random Gaussian matrix. O(d²) per vector.
//!   Produces Haar-uniform rotation matching paper's theoretical bounds.

use rand::prelude::*;
use rand::rngs::StdRng;

use crate::error::{Result, TurboQuantError};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Rotation strategy for TurboQuant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum RotationStrategy {
    /// Fast Walsh-Hadamard Transform with random sign flips. O(d log d).
    /// Production default. Construction: D₁·H·D₂ where D₁,D₂ are random ±1 diagonals.
    Wht,
    /// QR decomposition of random Gaussian matrix. O(d²) per vector.
    /// Slower but produces Haar-uniform rotation matching paper's theoretical bounds.
    Qr,
}

/// A rotation matrix for TurboQuant, generated deterministically from a seed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct RotationMatrix {
    /// The rotation strategy used.
    pub strategy: RotationStrategy,
    /// Dimension of the rotation (must be power of 2 for WHT).
    #[ts(type = "number")]
    pub dim: usize,
    /// Random seed for reproducibility.
    #[ts(type = "number")]
    pub seed: u64,
    // Internal sign vectors for WHT
    signs_d1: Vec<f32>,
    signs_d2: Vec<f32>,
    // Dense row-major matrix for QR
    qr_matrix: Option<Vec<f32>>,
}

// ---------------------------------------------------------------------------
// RotationMatrix implementation
// ---------------------------------------------------------------------------

impl RotationMatrix {
    /// Generate a new rotation matrix from seed and dimension.
    ///
    /// For WHT: `dim` must be a power of 2.
    /// For QR: any `dim > 0` is accepted.
    pub fn new(strategy: RotationStrategy, dim: usize, seed: u64) -> Result<Self> {
        if dim == 0 {
            return Err(TurboQuantError::RotationFailed("dimension must be > 0".to_string()));
        }

        match strategy {
            RotationStrategy::Wht => {
                if !Self::is_power_of_2(dim) {
                    return Err(TurboQuantError::RotationFailed(format!(
                        "WHT requires dimension to be a power of 2, got {dim}"
                    )));
                }
                let mut rng = StdRng::seed_from_u64(seed);
                let signs_d1 = random_signs(&mut rng, dim);
                let signs_d2 = random_signs(&mut rng, dim);
                Ok(Self { strategy, dim, seed, signs_d1, signs_d2, qr_matrix: None })
            }
            RotationStrategy::Qr => {
                let qr_matrix = generate_qr_matrix(dim, seed)?;
                let mut rng = StdRng::seed_from_u64(seed);
                let signs_d1 = random_signs(&mut rng, dim);
                let signs_d2 = random_signs(&mut rng, dim);
                Ok(Self { strategy, dim, seed, signs_d1, signs_d2, qr_matrix: Some(qr_matrix) })
            }
        }
    }

    /// Apply rotation: y = Π · x.
    ///
    /// For WHT: uses in-place FWHT with sign flips.
    /// For QR: dense matrix-vector multiply.
    pub fn apply(&self, input: &[f32]) -> Result<Vec<f32>> {
        if input.len() != self.dim {
            return Err(TurboQuantError::DimensionMismatch {
                expected: self.dim,
                actual: input.len(),
            });
        }

        match self.strategy {
            RotationStrategy::Wht => {
                // y = D₁ · FWHT(D₂ · x)
                let mut data: Vec<f32> =
                    input.iter().zip(self.signs_d2.iter()).map(|(x, s)| x * s).collect();
                fwht(&mut data);
                for (v, s) in data.iter_mut().zip(self.signs_d1.iter()) {
                    *v *= s;
                }
                Ok(data)
            }
            RotationStrategy::Qr => {
                let mat = self.qr_matrix.as_ref().ok_or(TurboQuantError::RotationNotInitialized)?;
                Ok(mat_vec_multiply(mat, input, self.dim))
            }
        }
    }

    /// Apply inverse rotation: x = Πᵀ · y.
    ///
    /// Since Π is orthogonal, Π⁻¹ = Πᵀ.
    /// For WHT: FWHT is self-inverse with 1/√d normalization.
    /// For QR: transpose multiply.
    pub fn apply_inverse(&self, input: &[f32]) -> Result<Vec<f32>> {
        if input.len() != self.dim {
            return Err(TurboQuantError::DimensionMismatch {
                expected: self.dim,
                actual: input.len(),
            });
        }

        match self.strategy {
            RotationStrategy::Wht => {
                // Π = D₁ · H · D₂, so Πᵀ = D₂ · Hᵀ · D₁ = D₂ · H · D₁
                // (H is symmetric and self-inverse with normalization, Dᵀ = D)
                // x = D₂ · FWHT(D₁ · y)
                let mut data: Vec<f32> =
                    input.iter().zip(self.signs_d1.iter()).map(|(y, s)| y * s).collect();
                fwht(&mut data);
                for (v, s) in data.iter_mut().zip(self.signs_d2.iter()) {
                    *v *= s;
                }
                Ok(data)
            }
            RotationStrategy::Qr => {
                let mat = self.qr_matrix.as_ref().ok_or(TurboQuantError::RotationNotInitialized)?;
                Ok(mat_t_vec_multiply(mat, input, self.dim))
            }
        }
    }

    /// Check if dimension is a power of 2.
    pub fn is_power_of_2(dim: usize) -> bool {
        dim > 0 && (dim & (dim - 1)) == 0
    }
}

// ---------------------------------------------------------------------------
// FWHT — Fast Walsh-Hadamard Transform (in-place, orthogonal normalized)
// ---------------------------------------------------------------------------

/// In-place Fast Walsh-Hadamard Transform.
///
/// Input length must be a power of 2.
/// Result is scaled by 1/√n so the transform is orthogonal.
fn fwht(data: &mut [f32]) {
    let n = data.len();
    if n <= 1 {
        return;
    }

    let mut step = 1;
    while step < n {
        for i in (0..n).step_by(step * 2) {
            for j in 0..step {
                let idx1 = i + j;
                let idx2 = i + j + step;
                let a = data[idx1];
                let b = data[idx2];
                data[idx1] = a + b;
                data[idx2] = a - b;
            }
        }
        step *= 2;
    }

    // Normalize by 1/√n for orthogonality
    let scale = 1.0 / (n as f32).sqrt();
    for x in data.iter_mut() {
        *x *= scale;
    }
}

// ---------------------------------------------------------------------------
// QR — Gram-Schmidt on random Gaussian matrix
// ---------------------------------------------------------------------------

/// Generate a random orthogonal matrix via seeded Gram-Schmidt.
fn generate_qr_matrix(dim: usize, seed: u64) -> Result<Vec<f32>> {
    let mut rng = StdRng::seed_from_u64(seed);

    // Generate dim × dim Gaussian matrix
    let mut rows: Vec<Vec<f64>> =
        (0..dim).map(|_| (0..dim).map(|_| box_muller_normal(&mut rng)).collect()).collect();

    // Gram-Schmidt orthogonalization
    gram_schmidt(&mut rows)?;

    // Flatten to row-major f32
    let flat: Vec<f32> = rows.iter().flat_map(|row| row.iter().map(|&x| x as f32)).collect();

    Ok(flat)
}

/// Box-Muller transform for standard normal variates (avoids rand_distr dependency).
fn box_muller_normal(rng: &mut StdRng) -> f64 {
    let u1: f64 = rng.random();
    let u2: f64 = rng.random();
    // Guard against log(0)
    let u1 = u1.max(1e-300);
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

/// Modified Gram-Schmidt orthogonalization (numerically more stable than classical).
fn gram_schmidt(rows: &mut [Vec<f64>]) -> Result<()> {
    let n = rows.len();
    for i in 0..n {
        // Subtract projections onto previous orthonormal vectors
        for j in 0..i {
            let dot: f64 = rows[i].iter().zip(&rows[j]).map(|(a, b)| a * b).sum();
            let norm_sq: f64 = rows[j].iter().map(|x| x * x).sum();
            if norm_sq < 1e-30 {
                return Err(TurboQuantError::RotationFailed(
                    "Gram-Schmidt encountered near-zero norm vector".to_string(),
                ));
            }
            let coeff = dot / norm_sq;
            // Indexed access required: simultaneous mutable rows[i] and immutable rows[j]
            #[allow(clippy::needless_range_loop)]
            for k in 0..n {
                rows[i][k] -= coeff * rows[j][k];
            }
        }
        // Normalize
        let norm: f64 = rows[i].iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm < 1e-30 {
            return Err(TurboQuantError::RotationFailed(
                "Gram-Schmidt produced zero vector (linearly dependent columns)".to_string(),
            ));
        }
        for row_i in rows[i].iter_mut().take(n) {
            *row_i /= norm;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Matrix-vector operations
// ---------------------------------------------------------------------------

/// Dense matrix-vector multiply: y = M · x (row-major storage).
fn mat_vec_multiply(mat: &[f32], vec: &[f32], dim: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; dim];
    for (i, out_i) in out.iter_mut().enumerate() {
        let row_start = i * dim;
        let mut sum = 0.0f32;
        for j in 0..dim {
            sum += mat[row_start + j] * vec[j];
        }
        *out_i = sum;
    }
    out
}

/// Transpose matrix-vector multiply: y = Mᵀ · x (row-major storage).
fn mat_t_vec_multiply(mat: &[f32], vec: &[f32], dim: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; dim];
    for (i, vec_i) in vec.iter().enumerate() {
        let row_start = i * dim;
        for j in 0..dim {
            out[j] += mat[row_start + j] * vec_i;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a vector of random ±1 signs.
fn random_signs(rng: &mut StdRng, dim: usize) -> Vec<f32> {
    (0..dim).map(|_| if rng.random::<bool>() { 1.0f32 } else { -1.0f32 }).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    fn vec_approx_eq(a: &[f32], b: &[f32], tol: f32) -> bool {
        a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| approx_eq(*x, *y, tol))
    }

    fn vec_norm(v: &[f32]) -> f32 {
        v.iter().map(|x| x * x).sum::<f32>().sqrt()
    }

    // --- is_power_of_2 ---

    #[test]
    fn power_of_2_true() {
        for &d in &[1usize, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024] {
            assert!(RotationMatrix::is_power_of_2(d), "expected {d} to be power of 2");
        }
    }

    #[test]
    fn power_of_2_false() {
        for &d in &[0usize, 3, 5, 6, 7, 9, 10, 15, 100] {
            assert!(!RotationMatrix::is_power_of_2(d), "expected {d} to NOT be power of 2");
        }
    }

    // --- WHT construction ---

    #[test]
    fn wht_new_power_of_2() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 4, 42);
        assert!(r.is_ok());
        let r = r.unwrap();
        assert_eq!(r.dim, 4);
        assert_eq!(r.strategy, RotationStrategy::Wht);
        assert_eq!(r.signs_d1.len(), 4);
        assert_eq!(r.signs_d2.len(), 4);
        assert!(r.qr_matrix.is_none());
    }

    #[test]
    fn wht_new_non_power_of_2() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 3, 42);
        assert!(r.is_err());
        let msg = r.unwrap_err().to_string();
        assert!(msg.contains("power of 2"), "unexpected error: {msg}");
    }

    #[test]
    fn wht_new_dim_zero() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 0, 42);
        assert!(r.is_err());
        let msg = r.unwrap_err().to_string();
        assert!(msg.contains("must be > 0"), "unexpected error: {msg}");
    }

    // --- Determinism ---

    #[test]
    fn wht_deterministic() {
        let r1 = RotationMatrix::new(RotationStrategy::Wht, 8, 12345).unwrap();
        let r2 = RotationMatrix::new(RotationStrategy::Wht, 8, 12345).unwrap();
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let y1 = r1.apply(&input).unwrap();
        let y2 = r2.apply(&input).unwrap();
        assert!(vec_approx_eq(&y1, &y2, 1e-10), "same seed must produce same output");
    }

    #[test]
    fn wht_different_seeds() {
        let r1 = RotationMatrix::new(RotationStrategy::Wht, 8, 1).unwrap();
        let r2 = RotationMatrix::new(RotationStrategy::Wht, 8, 2).unwrap();
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let y1 = r1.apply(&input).unwrap();
        let y2 = r2.apply(&input).unwrap();
        assert!(!vec_approx_eq(&y1, &y2, 0.1), "different seeds should produce different outputs");
    }

    // --- Orthogonality & roundtrip ---

    #[test]
    fn wht_roundtrip() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 16, 99).unwrap();
        let x = vec![
            3.0, -1.0, 4.0, 1.5, -5.0, 9.0, 2.6, -7.0, 0.0, 8.0, -3.3, 6.1, 1.0, -2.0, 4.4, 5.5,
        ];
        let y = r.apply(&x).unwrap();
        let x2 = r.apply_inverse(&y).unwrap();
        assert!(vec_approx_eq(&x, &x2, TOLERANCE), "roundtrip must recover original");
    }

    #[test]
    fn wht_preserves_norm() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 8, 77).unwrap();
        let x = vec![1.0, -2.0, 3.0, -4.0, 5.0, -6.0, 7.0, -8.0];
        let norm_x = vec_norm(&x);
        let y = r.apply(&x).unwrap();
        let norm_y = vec_norm(&y);
        assert!(approx_eq(norm_x, norm_y, TOLERANCE), "norm before={norm_x}, after={norm_y}");
    }

    // --- Various dimensions ---

    #[test]
    fn wht_dim_2() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 2, 10).unwrap();
        let x = vec![1.0, 0.0];
        let y = r.apply(&x).unwrap();
        assert_eq!(y.len(), 2);
        assert!(vec_norm(&y) > 0.0);
    }

    #[test]
    fn wht_dim_4() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 4, 20).unwrap();
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = r.apply(&x).unwrap();
        assert_eq!(y.len(), 4);
        assert!(approx_eq(vec_norm(&x), vec_norm(&y), TOLERANCE));
    }

    #[test]
    fn wht_dim_8() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 8, 30).unwrap();
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let x2 = r.apply_inverse(&r.apply(&x).unwrap()).unwrap();
        assert!(vec_approx_eq(&x, &x2, TOLERANCE));
    }

    #[test]
    fn wht_dim_64() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 64, 40).unwrap();
        let x: Vec<f32> = (0..64).map(|i| (i as f32 * 0.1).sin()).collect();
        let y = r.apply(&x).unwrap();
        assert!(approx_eq(vec_norm(&x), vec_norm(&y), TOLERANCE));
        let x2 = r.apply_inverse(&y).unwrap();
        assert!(vec_approx_eq(&x, &x2, TOLERANCE));
    }

    #[test]
    fn wht_dim_1024() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 1024, 50).unwrap();
        let x: Vec<f32> = (0..1024).map(|i| (i as f32 * 0.01).cos()).collect();
        let y = r.apply(&x).unwrap();
        assert!(approx_eq(vec_norm(&x), vec_norm(&y), 1e-4));
        let x2 = r.apply_inverse(&y).unwrap();
        assert!(vec_approx_eq(&x, &x2, 1e-4));
    }

    // --- Error cases ---

    #[test]
    fn wht_wrong_input_length() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 4, 42).unwrap();
        let short = vec![1.0, 2.0];
        let err = r.apply(&short).unwrap_err();
        assert!(matches!(err, TurboQuantError::DimensionMismatch { expected: 4, actual: 2 }));
        let long = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let err = r.apply(&long).unwrap_err();
        assert!(matches!(err, TurboQuantError::DimensionMismatch { expected: 4, actual: 5 }));
    }

    #[test]
    fn inverse_wrong_input_length() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 4, 42).unwrap();
        let err = r.apply_inverse(&[1.0]).unwrap_err();
        assert!(matches!(err, TurboQuantError::DimensionMismatch { expected: 4, actual: 1 }));
    }

    // --- QR construction ---

    #[test]
    fn qr_new_any_dim() {
        let r = RotationMatrix::new(RotationStrategy::Qr, 3, 42);
        assert!(r.is_ok());
        let r = r.unwrap();
        assert_eq!(r.dim, 3);
        assert_eq!(r.strategy, RotationStrategy::Qr);
        assert!(r.qr_matrix.is_some());
        assert_eq!(r.qr_matrix.as_ref().unwrap().len(), 9); // 3×3
    }

    #[test]
    fn qr_dim_zero_fails() {
        let r = RotationMatrix::new(RotationStrategy::Qr, 0, 42);
        assert!(r.is_err());
    }

    #[test]
    fn qr_deterministic() {
        let r1 = RotationMatrix::new(RotationStrategy::Qr, 4, 9999).unwrap();
        let r2 = RotationMatrix::new(RotationStrategy::Qr, 4, 9999).unwrap();
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y1 = r1.apply(&x).unwrap();
        let y2 = r2.apply(&x).unwrap();
        assert!(vec_approx_eq(&y1, &y2, 1e-10));
    }

    #[test]
    fn qr_orthogonality() {
        let r = RotationMatrix::new(RotationStrategy::Qr, 4, 555).unwrap();
        let mat = r.qr_matrix.as_ref().unwrap();
        let dim = 4;
        // Check Qᵀ·Q ≈ I
        for i in 0..dim {
            for j in 0..dim {
                let dot: f32 = (0..dim).map(|k| mat[k * dim + i] * mat[k * dim + j]).sum();
                let expected = if i == j { 1.0f32 } else { 0.0f32 };
                assert!(
                    approx_eq(dot, expected, 1e-4),
                    "QᵀQ[{i},{j}] = {dot}, expected {expected}"
                );
            }
        }
    }

    #[test]
    fn qr_roundtrip() {
        let r = RotationMatrix::new(RotationStrategy::Qr, 5, 777).unwrap();
        let x = vec![1.0, -2.0, 3.0, -4.0, 5.0];
        let y = r.apply(&x).unwrap();
        let x2 = r.apply_inverse(&y).unwrap();
        assert!(vec_approx_eq(&x, &x2, 1e-4));
    }

    #[test]
    fn qr_preserves_norm() {
        let r = RotationMatrix::new(RotationStrategy::Qr, 4, 333).unwrap();
        let x = vec![2.0, -3.0, 1.5, 4.5];
        let norm_x = vec_norm(&x);
        let y = r.apply(&x).unwrap();
        let norm_y = vec_norm(&y);
        assert!(approx_eq(norm_x, norm_y, 1e-4), "norm before={norm_x}, after={norm_y}");
    }

    #[test]
    fn qr_wrong_input_length() {
        let r = RotationMatrix::new(RotationStrategy::Qr, 3, 42).unwrap();
        let err = r.apply(&[1.0, 2.0]).unwrap_err();
        assert!(matches!(err, TurboQuantError::DimensionMismatch { expected: 3, actual: 2 }));
    }

    // --- Serde ---

    #[test]
    fn strategy_debug_format() {
        assert!(format!("{:?}", RotationStrategy::Wht).contains("Wht"));
        assert!(format!("{:?}", RotationStrategy::Qr).contains("Qr"));
    }

    #[test]
    fn strategy_equality() {
        assert_eq!(RotationStrategy::Wht, RotationStrategy::Wht);
        assert_eq!(RotationStrategy::Qr, RotationStrategy::Qr);
        assert_ne!(RotationStrategy::Wht, RotationStrategy::Qr);
    }

    #[test]
    fn rotation_matrix_clone_preserves_fields_wht() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 8, 42).unwrap();
        let cloned = r.clone();
        assert_eq!(r.strategy, cloned.strategy);
        assert_eq!(r.dim, cloned.dim);
        assert_eq!(r.seed, cloned.seed);
        assert_eq!(r.signs_d1, cloned.signs_d1);
        assert_eq!(r.signs_d2, cloned.signs_d2);
        assert!(cloned.qr_matrix.is_none());
    }

    #[test]
    fn rotation_matrix_clone_preserves_fields_qr() {
        let r = RotationMatrix::new(RotationStrategy::Qr, 3, 42).unwrap();
        let cloned = r.clone();
        assert_eq!(r.strategy, cloned.strategy);
        assert_eq!(r.dim, cloned.dim);
        assert_eq!(r.seed, cloned.seed);
        assert_eq!(r.qr_matrix, cloned.qr_matrix);
    }

    #[test]
    fn rotation_matrix_apply_matches_clone_wht() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 8, 42).unwrap();
        let cloned = r.clone();
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let y1 = r.apply(&x).unwrap();
        let y2 = cloned.apply(&x).unwrap();
        assert!(vec_approx_eq(&y1, &y2, 1e-10), "cloned matrix should produce identical output");
    }

    #[test]
    fn rotation_matrix_apply_matches_clone_qr() {
        let r = RotationMatrix::new(RotationStrategy::Qr, 4, 42).unwrap();
        let cloned = r.clone();
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y1 = r.apply(&x).unwrap();
        let y2 = cloned.apply(&x).unwrap();
        assert!(vec_approx_eq(&y1, &y2, 1e-10), "cloned matrix should produce identical output");
    }

    // --- ts-rs ---

    #[test]
    fn rotation_matrix_ts_export() {
        use ts_rs::TS;
        let name = RotationMatrix::name(&Default::default());
        assert!(!name.is_empty(), "ts-rs name should not be empty");
    }

    #[test]
    fn strategy_ts_export() {
        use ts_rs::TS;
        let name = RotationStrategy::name(&Default::default());
        assert!(!name.is_empty());
    }

    // --- Special vectors ---

    #[test]
    fn wht_zero_vector() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 4, 42).unwrap();
        let zero = vec![0.0f32; 4];
        let y = r.apply(&zero).unwrap();
        assert!(y.iter().all(|&v| v == 0.0), "rotating zero should give zero");
    }

    #[test]
    fn wht_unit_vector() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 4, 42).unwrap();
        let e = vec![0.0f32, 1.0, 0.0, 0.0];
        let y = r.apply(&e).unwrap();
        assert!(approx_eq(vec_norm(&e), vec_norm(&y), TOLERANCE));
    }

    #[test]
    fn wht_basis_vectors_orthogonal() {
        let r = RotationMatrix::new(RotationStrategy::Wht, 4, 42).unwrap();
        let mut rotated: Vec<Vec<f32>> = Vec::new();
        for i in 0..4 {
            let mut e = vec![0.0f32; 4];
            e[i] = 1.0;
            let y = r.apply(&e).unwrap();
            // Each should be unit norm
            assert!(approx_eq(vec_norm(&y), 1.0, TOLERANCE), "basis {i} norm");
            rotated.push(y);
        }
        // Pairwise orthogonal
        for i in 0..4 {
            for j in (i + 1)..4 {
                let dot: f32 = rotated[i].iter().zip(&rotated[j]).map(|(a, b)| a * b).sum();
                assert!(
                    approx_eq(dot, 0.0, TOLERANCE),
                    "rotated basis {i}·{j} = {dot}, expected ~0"
                );
            }
        }
    }

    // --- FWHT unit tests ---

    #[test]
    fn fwht_dim_1() {
        let mut data = [3.14f32];
        fwht(&mut data);
        assert!(approx_eq(data[0], 3.14, TOLERANCE));
    }

    #[test]
    fn fwht_dim_2() {
        let mut data = [1.0f32, 1.0];
        fwht(&mut data);
        // H₂ = [1 1; 1 -1] / √2 → [√2, 0]
        assert!(approx_eq(data[0], std::f32::consts::SQRT_2, TOLERANCE));
        assert!(approx_eq(data[1], 0.0, TOLERANCE));
    }

    #[test]
    fn fwht_preserves_norm() {
        let mut data = [1.0f32, 2.0, 3.0, 4.0];
        let norm_before = vec_norm(&data);
        fwht(&mut data);
        let norm_after = vec_norm(&data);
        assert!(approx_eq(norm_before, norm_after, TOLERANCE));
    }

    // --- Box-Muller ---

    #[test]
    fn box_muller_produces_values() {
        let mut rng = StdRng::seed_from_u64(42);
        let values: Vec<f64> = (0..100).map(|_| box_muller_normal(&mut rng)).collect();
        // Should not all be the same
        let first = values[0];
        assert!(!values.iter().all(|&v| v == first), "values should vary");
        // Mean should be roughly 0
        let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        assert!(mean.abs() < 0.5, "mean should be near 0, got {mean}");
    }
}
