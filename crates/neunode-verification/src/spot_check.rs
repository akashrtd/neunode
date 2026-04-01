use sha2::{Digest, Sha256};

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Configuration for random re-execution spot checks.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct SpotCheckConfig {
    pub check_rate: f64,
    #[ts(type = "number")]
    pub max_retries: u32,
    #[ts(type = "number")]
    pub timeout_secs: u64,
}

impl Default for SpotCheckConfig {
    fn default() -> Self {
        Self { check_rate: 0.10, max_retries: 3, timeout_secs: 300 }
    }
}

/// Result of a spot-check re-execution comparison.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct SpotCheckResult {
    pub original_hash: String,
    pub recomputed_hash: String,
    pub match_result: bool,
    #[ts(type = "number")]
    pub retries_used: u32,
    #[ts(type = "number")]
    pub elapsed_ms: u64,
}

/// Performs random re-execution spot checks on task outputs.
pub struct SpotChecker {
    config: SpotCheckConfig,
}

impl SpotChecker {
    pub fn new(config: SpotCheckConfig) -> Self {
        Self { config }
    }

    pub fn should_check(&self) -> bool {
        rand::random::<f64>() < self.config.check_rate
    }

    pub fn verify_output(&self, original: &[u8], recomputed: &[u8]) -> SpotCheckResult {
        let original_hash = sha256_hex(original);
        let recomputed_hash = sha256_hex(recomputed);
        SpotCheckResult {
            match_result: original_hash == recomputed_hash,
            original_hash,
            recomputed_hash,
            retries_used: 0,
            elapsed_ms: 0,
        }
    }

    pub fn verify_hash(&self, original_hash: &str, recomputed: &[u8]) -> SpotCheckResult {
        let recomputed_hash = sha256_hex(recomputed);
        SpotCheckResult {
            match_result: original_hash == recomputed_hash,
            original_hash: original_hash.to_string(),
            recomputed_hash,
            retries_used: 0,
            elapsed_ms: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_check_zero_rate_always_false() {
        let checker = SpotChecker::new(SpotCheckConfig { check_rate: 0.0, ..Default::default() });
        for _ in 0..100 {
            assert!(!checker.should_check());
        }
    }

    #[test]
    fn should_check_full_rate_always_true() {
        let checker = SpotChecker::new(SpotCheckConfig { check_rate: 1.0, ..Default::default() });
        for _ in 0..100 {
            assert!(checker.should_check());
        }
    }

    #[test]
    fn verify_output_match() {
        let checker = SpotChecker::new(SpotCheckConfig::default());
        let data = b"hello world";
        let result = checker.verify_output(data, data);
        assert!(result.match_result);
        assert_eq!(result.original_hash, result.recomputed_hash);
    }

    #[test]
    fn verify_output_mismatch() {
        let checker = SpotChecker::new(SpotCheckConfig::default());
        let result = checker.verify_output(b"hello", b"world");
        assert!(!result.match_result);
        assert_ne!(result.original_hash, result.recomputed_hash);
    }

    #[test]
    fn verify_hash_match() {
        let checker = SpotChecker::new(SpotCheckConfig::default());
        let data = b"test data";
        let hash = sha256_hex(data);
        let result = checker.verify_hash(&hash, data);
        assert!(result.match_result);
    }

    #[test]
    fn verify_hash_mismatch() {
        let checker = SpotChecker::new(SpotCheckConfig::default());
        let result = checker.verify_hash("abc123", b"different data");
        assert!(!result.match_result);
    }

    #[test]
    fn config_default_values() {
        let config = SpotCheckConfig::default();
        assert!((config.check_rate - 0.10).abs() < f64::EPSILON);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_secs, 300);
    }

    #[test]
    fn config_serde_roundtrip() {
        let config = SpotCheckConfig { check_rate: 0.25, max_retries: 5, timeout_secs: 600 };
        let json = serde_json::to_string(&config).unwrap();
        let back: SpotCheckConfig = serde_json::from_str(&json).unwrap();
        assert!((back.check_rate - 0.25).abs() < f64::EPSILON);
        assert_eq!(back.max_retries, 5);
        assert_eq!(back.timeout_secs, 600);
    }

    #[test]
    fn result_serde_roundtrip() {
        let result = SpotCheckResult {
            original_hash: "abc".into(),
            recomputed_hash: "def".into(),
            match_result: false,
            retries_used: 2,
            elapsed_ms: 1500,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: SpotCheckResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.original_hash, back.original_hash);
        assert_eq!(result.retries_used, back.retries_used);
        assert_eq!(result.elapsed_ms, back.elapsed_ms);
    }

    #[test]
    fn verify_deterministic_hash() {
        let checker = SpotChecker::new(SpotCheckConfig::default());
        let r1 = checker.verify_output(b"x", b"x");
        let r2 = checker.verify_output(b"x", b"x");
        assert_eq!(r1.original_hash, r2.original_hash);
    }
}
