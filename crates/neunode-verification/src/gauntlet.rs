use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Result, VerificationError};
use crate::hash_util::sha256_hex;
use crate::types::{VerificationLayer, VerificationResult};

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

/// A single adversarial known-answer test.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct GauntletTest {
    pub name: String,
    pub input_hash: String,
    pub expected_output_hash: String,
    pub injected: bool,
    #[ts(type = "number")]
    pub difficulty: u32,
}

/// Configuration for gauntlet test injection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct GauntletConfig {
    pub injection_rate: f64,
    pub difficulty_range: (u32, u32),
    #[ts(type = "number")]
    pub seed: u64,
}

impl Default for GauntletConfig {
    fn default() -> Self {
        Self { injection_rate: 0.05, difficulty_range: (3, 8), seed: 42 }
    }
}

/// Manages adversarial known-answer tests for verification.
pub struct Gauntlet {
    config: GauntletConfig,
    tests: Vec<GauntletTest>,
    rng: StdRng,
}

impl Gauntlet {
    pub fn new(config: GauntletConfig) -> Self {
        let rng = StdRng::seed_from_u64(config.seed);
        Self { config, tests: Vec::new(), rng }
    }

    pub fn add_test(&mut self, test: GauntletTest) {
        self.tests.push(test);
    }

    pub fn should_inject(&mut self) -> bool {
        self.rng.gen::<f64>() < self.config.injection_rate
    }

    pub fn select_test(&mut self) -> Option<&GauntletTest> {
        if self.tests.is_empty() {
            return None;
        }
        let idx = self.rng.gen_range(0..self.tests.len());
        Some(&self.tests[idx])
    }

    pub fn verify(
        &self,
        test: &GauntletTest,
        actual_output_hash: &str,
    ) -> Result<VerificationResult> {
        if test.expected_output_hash == actual_output_hash {
            Ok(VerificationResult {
                layer: VerificationLayer::Automated,
                passed: true,
                confidence: 0.85,
                evidence_hash: actual_output_hash.to_string(),
                verifier_id: "gauntlet".to_string(),
                timestamp_ms: now_ms(),
                details: Some(format!("gauntlet test '{}' passed", test.name)),
            })
        } else {
            let reason = format!(
                "hash mismatch: expected {}, got {}",
                test.expected_output_hash, actual_output_hash
            );
            Err(VerificationError::GauntletFailed { test_name: test.name.clone(), reason })
        }
    }

    pub fn generate_test(
        &mut self,
        name: &str,
        input: &[u8],
        expected_output: &[u8],
    ) -> GauntletTest {
        let difficulty =
            self.rng.gen_range(self.config.difficulty_range.0..=self.config.difficulty_range.1);
        GauntletTest {
            name: name.to_string(),
            input_hash: sha256_hex(input),
            expected_output_hash: sha256_hex(expected_output),
            injected: true,
            difficulty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_empty_pool() {
        let g = Gauntlet::new(GauntletConfig::default());
        assert!(g.tests.is_empty());
    }

    #[test]
    fn add_test_adds_to_pool() {
        let mut g = Gauntlet::new(GauntletConfig::default());
        let test = GauntletTest {
            name: "t1".into(),
            input_hash: "abc".into(),
            expected_output_hash: "def".into(),
            injected: false,
            difficulty: 5,
        };
        g.add_test(test);
        assert_eq!(g.tests.len(), 1);
    }

    #[test]
    fn should_inject_at_normal_rate() {
        let mut g = Gauntlet::new(GauntletConfig { injection_rate: 0.5, ..Default::default() });
        let mut injected = 0u32;
        for _ in 0..1000 {
            if g.should_inject() {
                injected += 1;
            }
        }
        // Should be roughly 50% but allow wide margin.
        assert!(injected > 300 && injected < 700);
    }

    #[test]
    fn injection_at_zero_always_false() {
        let mut g = Gauntlet::new(GauntletConfig { injection_rate: 0.0, ..Default::default() });
        for _ in 0..100 {
            assert!(!g.should_inject());
        }
    }

    #[test]
    fn injection_at_one_always_true() {
        let mut g = Gauntlet::new(GauntletConfig { injection_rate: 1.0, ..Default::default() });
        for _ in 0..100 {
            assert!(g.should_inject());
        }
    }

    #[test]
    fn select_test_returns_some_with_tests() {
        let mut g = Gauntlet::new(GauntletConfig::default());
        g.add_test(GauntletTest {
            name: "a".into(),
            input_hash: "1".into(),
            expected_output_hash: "2".into(),
            injected: false,
            difficulty: 3,
        });
        g.add_test(GauntletTest {
            name: "b".into(),
            input_hash: "3".into(),
            expected_output_hash: "4".into(),
            injected: true,
            difficulty: 7,
        });
        assert!(g.select_test().is_some());
    }

    #[test]
    fn select_test_returns_none_when_empty() {
        let mut g = Gauntlet::new(GauntletConfig::default());
        assert!(g.select_test().is_none());
    }

    #[test]
    fn verify_passes_with_matching_hash() {
        let g = Gauntlet::new(GauntletConfig::default());
        let test = GauntletTest {
            name: "test1".into(),
            input_hash: "abc".into(),
            expected_output_hash: "def".into(),
            injected: false,
            difficulty: 5,
        };
        let result = g.verify(&test, "def").unwrap();
        assert!(result.passed);
        assert_eq!(result.layer, VerificationLayer::Automated);
    }

    #[test]
    fn verify_fails_with_mismatched_hash() {
        let g = Gauntlet::new(GauntletConfig::default());
        let test = GauntletTest {
            name: "test1".into(),
            input_hash: "abc".into(),
            expected_output_hash: "def".into(),
            injected: false,
            difficulty: 5,
        };
        let err = g.verify(&test, "xyz").unwrap_err();
        match err {
            VerificationError::GauntletFailed { test_name, .. } => {
                assert_eq!(test_name, "test1");
            }
            _ => panic!("expected GauntletFailed"),
        }
    }

    #[test]
    fn generate_test_hashes_input_and_output() {
        let mut g =
            Gauntlet::new(GauntletConfig { difficulty_range: (5, 5), ..Default::default() });
        let input = b"hello";
        let output = b"world";
        let test = g.generate_test("gen_test", input, output);
        assert_eq!(test.name, "gen_test");
        assert_eq!(test.input_hash, sha256_hex(input));
        assert_eq!(test.expected_output_hash, sha256_hex(output));
        assert!(test.injected);
        assert_eq!(test.difficulty, 5);
    }

    #[test]
    fn deterministic_rng_same_seed() {
        let mut g1 = Gauntlet::new(GauntletConfig { seed: 123, ..Default::default() });
        let mut g2 = Gauntlet::new(GauntletConfig { seed: 123, ..Default::default() });
        for _ in 0..50 {
            assert_eq!(g1.should_inject(), g2.should_inject());
        }
    }

    #[test]
    fn gauntlet_test_serde_roundtrip() {
        let test = GauntletTest {
            name: "serde_test".into(),
            input_hash: "abc".into(),
            expected_output_hash: "def".into(),
            injected: true,
            difficulty: 7,
        };
        let json = serde_json::to_string(&test).unwrap();
        let back: GauntletTest = serde_json::from_str(&json).unwrap();
        assert_eq!(test.name, back.name);
        assert_eq!(test.difficulty, back.difficulty);
    }

    #[test]
    fn gauntlet_config_serde_roundtrip() {
        let config = GauntletConfig { injection_rate: 0.1, difficulty_range: (2, 9), seed: 999 };
        let json = serde_json::to_string(&config).unwrap();
        let back: GauntletConfig = serde_json::from_str(&json).unwrap();
        assert!((back.injection_rate - 0.1).abs() < f64::EPSILON);
        assert_eq!(back.difficulty_range, (2, 9));
    }

    #[test]
    fn gauntlet_config_default_values() {
        let config = GauntletConfig::default();
        assert!((config.injection_rate - 0.05).abs() < f64::EPSILON);
        assert_eq!(config.difficulty_range, (3, 8));
        assert_eq!(config.seed, 42);
    }
}
