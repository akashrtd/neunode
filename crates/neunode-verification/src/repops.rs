use sha2::{Digest, Sha256};

use crate::hash_util::sha256_hex;

/// IEEE 754 rounding mode for deterministic float execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum FloatRoundingMode {
    RoundToNearestEven,
    RoundTowardZero,
    Stochastic,
}

/// Configuration for RepOps deterministic execution verification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct RepOpsConfig {
    pub float_rounding_mode: FloatRoundingMode,
    pub hash_intermediates: bool,
    #[ts(type = "number")]
    pub checkpoint_interval: u32,
}

impl Default for RepOpsConfig {
    fn default() -> Self {
        Self {
            float_rounding_mode: FloatRoundingMode::RoundToNearestEven,
            hash_intermediates: true,
            checkpoint_interval: 10,
        }
    }
}

/// Result of a RepOps deterministic execution verification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct RepOpsResult {
    pub output_hash: String,
    pub intermediate_hashes: Vec<String>,
    #[ts(type = "number")]
    pub op_count: u32,
    #[ts(type = "number")]
    pub hash_count: u32,
    pub reproducible: bool,
}

/// Deterministic executor that tracks intermediate computation hashes.
pub struct DeterministicExecutor {
    config: RepOpsConfig,
    hasher: Sha256,
    op_count: u32,
    intermediate_hashes: Vec<String>,
}

impl DeterministicExecutor {
    pub fn new(config: RepOpsConfig) -> Self {
        Self { config, hasher: Sha256::new(), op_count: 0, intermediate_hashes: Vec::new() }
    }

    pub fn hash_tensor(&mut self, tensor: &[f32]) -> String {
        let mut hasher = Sha256::new();
        for &val in tensor {
            hasher.update(val.to_le_bytes());
        }
        sha256_hex(&hasher.finalize())
    }

    pub fn execute_op<F>(&mut self, op: F) -> Vec<f32>
    where
        F: FnOnce() -> Vec<f32>,
    {
        let result = op();
        self.op_count += 1;
        result
    }

    pub fn checkpoint(&mut self, tensor: &[f32]) {
        if self.op_count > 0 && self.op_count.is_multiple_of(self.config.checkpoint_interval) {
            let hash = self.hash_tensor(tensor);
            self.intermediate_hashes.push(hash);
        }
    }

    pub fn finalize(mut self, output: &[f32]) -> RepOpsResult {
        let output_hash = self.hash_tensor(output);
        let hash_count = self.intermediate_hashes.len() as u32;
        let has_checkpoints = !self.intermediate_hashes.is_empty();
        RepOpsResult {
            output_hash,
            intermediate_hashes: self.intermediate_hashes,
            op_count: self.op_count,
            hash_count,
            reproducible: has_checkpoints,
        }
    }

    pub fn compare(a: &RepOpsResult, b: &RepOpsResult) -> bool {
        a.output_hash == b.output_hash && a.intermediate_hashes == b.intermediate_hashes
    }

    pub fn reset(&mut self) {
        self.hasher = Sha256::new();
        self.op_count = 0;
        self.intermediate_hashes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_initializes_state() {
        let ex = DeterministicExecutor::new(RepOpsConfig::default());
        assert_eq!(ex.op_count, 0);
        assert!(ex.intermediate_hashes.is_empty());
    }

    #[test]
    fn hash_tensor_deterministic() {
        let mut ex = DeterministicExecutor::new(RepOpsConfig::default());
        let tensor = vec![1.0f32, 2.0, 3.0];
        let h1 = ex.hash_tensor(&tensor);
        let h2 = ex.hash_tensor(&tensor);
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_tensor_different_inputs() {
        let mut ex = DeterministicExecutor::new(RepOpsConfig::default());
        let h1 = ex.hash_tensor(&[1.0f32, 2.0]);
        let h2 = ex.hash_tensor(&[3.0f32, 4.0]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_tensor_empty() {
        let mut ex = DeterministicExecutor::new(RepOpsConfig::default());
        let hash = ex.hash_tensor(&[]);
        assert!(!hash.is_empty());
        // Empty tensor always produces the same hash.
        let hash2 = ex.hash_tensor(&[]);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn hash_tensor_single_element() {
        let mut ex = DeterministicExecutor::new(RepOpsConfig::default());
        let hash = ex.hash_tensor(&[42.0f32]);
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn execute_op_tracks_count() {
        let mut ex = DeterministicExecutor::new(RepOpsConfig::default());
        assert_eq!(ex.op_count, 0);
        ex.execute_op(|| vec![1.0f32]);
        assert_eq!(ex.op_count, 1);
        ex.execute_op(|| vec![2.0f32]);
        assert_eq!(ex.op_count, 2);
    }

    #[test]
    fn checkpoint_hashes_at_interval() {
        let config = RepOpsConfig { checkpoint_interval: 2, ..Default::default() };
        let mut ex = DeterministicExecutor::new(config);
        let tensor = vec![1.0f32];
        ex.execute_op(|| tensor.clone());
        ex.checkpoint(&tensor);
        assert!(ex.intermediate_hashes.is_empty()); // op 1, not at interval

        ex.execute_op(|| tensor.clone());
        ex.checkpoint(&tensor);
        assert_eq!(ex.intermediate_hashes.len(), 1); // op 2, at interval

        ex.execute_op(|| tensor.clone());
        ex.checkpoint(&tensor);
        assert_eq!(ex.intermediate_hashes.len(), 1); // op 3

        ex.execute_op(|| tensor.clone());
        ex.checkpoint(&tensor);
        assert_eq!(ex.intermediate_hashes.len(), 2); // op 4
    }

    #[test]
    fn checkpoint_skips_non_interval() {
        let config = RepOpsConfig { checkpoint_interval: 5, ..Default::default() };
        let mut ex = DeterministicExecutor::new(config);
        let tensor = vec![1.0f32];
        for _ in 0..4 {
            ex.execute_op(|| tensor.clone());
            ex.checkpoint(&tensor);
        }
        assert!(ex.intermediate_hashes.is_empty());
    }

    #[test]
    fn finalize_computes_output_hash() {
        let mut ex = DeterministicExecutor::new(RepOpsConfig::default());
        ex.execute_op(|| vec![1.0f32, 2.0]);
        let output = vec![3.0f32, 4.0];
        let result = ex.finalize(&output);
        assert_eq!(result.output_hash.len(), 64);
        assert_eq!(result.op_count, 1);
    }

    #[test]
    fn finalize_returns_op_count() {
        let mut ex = DeterministicExecutor::new(RepOpsConfig::default());
        for i in 0..5 {
            ex.execute_op(move || vec![i as f32]);
        }
        let result = ex.finalize(&[1.0f32]);
        assert_eq!(result.op_count, 5);
    }

    #[test]
    fn compare_matching_results() {
        let a = RepOpsResult {
            output_hash: "abc".into(),
            intermediate_hashes: vec!["x".into(), "y".into()],
            op_count: 2,
            hash_count: 2,
            reproducible: true,
        };
        let b = a.clone();
        assert!(DeterministicExecutor::compare(&a, &b));
    }

    #[test]
    fn compare_mismatching_output() {
        let a = RepOpsResult {
            output_hash: "abc".into(),
            intermediate_hashes: vec![],
            op_count: 1,
            hash_count: 0,
            reproducible: true,
        };
        let b = RepOpsResult { output_hash: "def".into(), ..a.clone() };
        assert!(!DeterministicExecutor::compare(&a, &b));
    }

    #[test]
    fn compare_mismatching_intermediate() {
        let a = RepOpsResult {
            output_hash: "abc".into(),
            intermediate_hashes: vec!["x".into()],
            op_count: 1,
            hash_count: 1,
            reproducible: true,
        };
        let b = RepOpsResult { intermediate_hashes: vec!["y".into()], ..a.clone() };
        assert!(!DeterministicExecutor::compare(&a, &b));
    }

    #[test]
    fn multiple_checkpoints() {
        let config = RepOpsConfig { checkpoint_interval: 3, ..Default::default() };
        let mut ex = DeterministicExecutor::new(config);
        let tensor = vec![1.0f32];
        for _ in 0..9 {
            ex.execute_op(|| tensor.clone());
            ex.checkpoint(&tensor);
        }
        assert_eq!(ex.intermediate_hashes.len(), 3); // at 3, 6, 9
    }

    #[test]
    fn reset_clears_state() {
        let mut ex = DeterministicExecutor::new(RepOpsConfig::default());
        ex.execute_op(|| vec![1.0f32]);
        ex.execute_op(|| vec![2.0f32]);
        ex.reset();
        assert_eq!(ex.op_count, 0);
        assert!(ex.intermediate_hashes.is_empty());
    }

    #[test]
    fn float_rounding_mode_serde() {
        let modes = vec![
            FloatRoundingMode::RoundToNearestEven,
            FloatRoundingMode::RoundTowardZero,
            FloatRoundingMode::Stochastic,
        ];
        for mode in modes {
            let json = serde_json::to_string(&mode).unwrap();
            let back: FloatRoundingMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, back);
        }
    }

    #[test]
    fn config_default_values() {
        let config = RepOpsConfig::default();
        assert_eq!(config.float_rounding_mode, FloatRoundingMode::RoundToNearestEven);
        assert!(config.hash_intermediates);
        assert_eq!(config.checkpoint_interval, 10);
    }

    #[test]
    fn result_serde_roundtrip() {
        let result = RepOpsResult {
            output_hash: "abc123".into(),
            intermediate_hashes: vec!["h1".into(), "h2".into()],
            op_count: 20,
            hash_count: 2,
            reproducible: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: RepOpsResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.output_hash, back.output_hash);
        assert_eq!(result.op_count, back.op_count);
        assert_eq!(result.reproducible, back.reproducible);
    }

    #[test]
    fn config_serde_roundtrip() {
        let config = RepOpsConfig {
            float_rounding_mode: FloatRoundingMode::Stochastic,
            hash_intermediates: false,
            checkpoint_interval: 5,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: RepOpsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.float_rounding_mode, FloatRoundingMode::Stochastic);
        assert!(!back.hash_intermediates);
        assert_eq!(back.checkpoint_interval, 5);
    }

    #[test]
    fn large_tensor_hash() {
        let mut ex = DeterministicExecutor::new(RepOpsConfig::default());
        let tensor: Vec<f32> = (0..10000).map(|i| i as f32).collect();
        let hash = ex.hash_tensor(&tensor);
        assert_eq!(hash.len(), 64);
    }
}
