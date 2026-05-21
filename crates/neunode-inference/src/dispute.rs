use neunode_core::types::{Did, Hash256, Timestamp, TokenAmount};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{InferenceError, Result};

/// Default dispute window in seconds (1 hour).
pub const DEFAULT_DISPUTE_WINDOW_SECS: u64 = 3600;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub enum SettlementStatus {
    Settled,
    Disputed,
    Resolved,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct InferenceDispute {
    pub settlement_hash: Hash256,
    pub requester_did: Did,
    pub provider_did: Did,
    pub reason: String,
    pub evidence_hash: Hash256,
    pub challenge_time: Timestamp,
    pub resolution_deadline: Timestamp,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct DisputeConfig {
    pub window_secs: u64,
}

impl Default for DisputeConfig {
    fn default() -> Self {
        Self { window_secs: DEFAULT_DISPUTE_WINDOW_SECS }
    }
}

#[derive(Debug)]
pub struct DisputeEngine {
    config: DisputeConfig,
}

impl DisputeEngine {
    pub fn new(config: DisputeConfig) -> Self {
        Self { config }
    }

    pub fn dispute_window_secs(&self) -> u64 {
        self.config.window_secs
    }

    /// Check if a settlement is still within the dispute window.
    pub fn is_within_window(&self, settlement_time: Timestamp, now: Timestamp) -> bool {
        now <= settlement_time + self.config.window_secs
    }

    /// Open a dispute for a settlement. Returns the dispute record.
    pub fn challenge(
        &self,
        settlement_hash: Hash256,
        requester_did: Did,
        provider_did: Did,
        reason: String,
        evidence_hash: Hash256,
        settlement_time: Timestamp,
        now: Timestamp,
    ) -> Result<InferenceDispute> {
        if !self.is_within_window(settlement_time, now) {
            return Err(InferenceError::SettlementFailed(
                "dispute window has expired".to_string(),
            ));
        }
        if reason.is_empty() {
            return Err(InferenceError::InvalidRequest(
                "dispute reason is required".to_string(),
            ));
        }
        if evidence_hash.0.is_empty() {
            return Err(InferenceError::InvalidRequest(
                "evidence hash is required".to_string(),
            ));
        }
        let resolution_deadline = now + self.config.window_secs;
        Ok(InferenceDispute {
            settlement_hash,
            requester_did,
            provider_did,
            reason,
            evidence_hash,
            challenge_time: now,
            resolution_deadline,
            resolved: false,
        })
    }

    /// Resolve a dispute — marks it as resolved.
    pub fn resolve(dispute: &mut InferenceDispute, _now: Timestamp) -> Result<()> {
        if dispute.resolved {
            return Err(InferenceError::SettlementFailed(
                "dispute already resolved".to_string(),
            ));
        }
        dispute.resolved = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_did(n: u32) -> Did {
        Did(format!("did:neunode:0x{n:040x}"))
    }

    fn test_hash(n: u8) -> Hash256 {
        Hash256(hex::encode([n; 32]))
    }

    #[test]
    fn default_config() {
        let config = DisputeConfig::default();
        assert_eq!(config.window_secs, 3600);
    }

    #[test]
    fn is_within_window_true() {
        let engine = DisputeEngine::new(DisputeConfig::default());
        assert!(engine.is_within_window(1000, 2000));
    }

    #[test]
    fn is_within_window_false() {
        let engine = DisputeEngine::new(DisputeConfig::default());
        assert!(!engine.is_within_window(0, 3601));
    }

    #[test]
    fn is_within_window_exact_boundary() {
        let engine = DisputeEngine::new(DisputeConfig::default());
        assert!(engine.is_within_window(0, 3600));
    }

    #[test]
    fn challenge_succeeds_within_window() {
        let engine = DisputeEngine::new(DisputeConfig::default());
        let dispute = engine
            .challenge(
                test_hash(1),
                test_did(1),
                test_did(2),
                "provider returned gibberish".to_string(),
                test_hash(2),
                1000,
                2000,
            )
            .unwrap();
        assert!(!dispute.resolved);
        assert_eq!(dispute.requester_did, test_did(1));
        assert_eq!(dispute.provider_did, test_did(2));
    }

    #[test]
    fn challenge_fails_after_window() {
        let engine = DisputeEngine::new(DisputeConfig { window_secs: 60 });
        let result = engine.challenge(
            test_hash(1),
            test_did(1),
            test_did(2),
            "bad output".to_string(),
            test_hash(2),
            0,
            61,
        );
        assert!(result.is_err());
    }

    #[test]
    fn challenge_fails_empty_reason() {
        let engine = DisputeEngine::new(DisputeConfig::default());
        let result = engine.challenge(
            test_hash(1),
            test_did(1),
            test_did(2),
            "".to_string(),
            test_hash(2),
            1000,
            2000,
        );
        assert!(result.is_err());
    }

    #[test]
    fn resolve_dispute() {
        let engine = DisputeEngine::new(DisputeConfig::default());
        let mut dispute = engine
            .challenge(
                test_hash(1),
                test_did(1),
                test_did(2),
                "garbage output".to_string(),
                test_hash(2),
                1000,
                2000,
            )
            .unwrap();
        DisputeEngine::resolve(&mut dispute, 3000).unwrap();
        assert!(dispute.resolved);
    }

    #[test]
    fn resolve_already_resolved_fails() {
        let engine = DisputeEngine::new(DisputeConfig::default());
        let mut dispute = engine
            .challenge(
                test_hash(1),
                test_did(1),
                test_did(2),
                "bad".to_string(),
                test_hash(2),
                0,
                0,
            )
            .unwrap();
        DisputeEngine::resolve(&mut dispute, 1).unwrap();
        let result = DisputeEngine::resolve(&mut dispute, 2);
        assert!(result.is_err());
    }

    #[test]
    fn custom_window_config() {
        let engine = DisputeEngine::new(DisputeConfig { window_secs: 7200 });
        assert!(engine.is_within_window(0, 7200));
        assert!(!engine.is_within_window(0, 7201));
    }

    #[test]
    fn dispute_config_serde_roundtrip() {
        let config = DisputeConfig { window_secs: 1800 };
        let json = serde_json::to_string(&config).unwrap();
        let back: DisputeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, back);
    }

    #[test]
    fn inference_dispute_serde_roundtrip() {
        let engine = DisputeEngine::new(DisputeConfig::default());
        let dispute = engine
            .challenge(
                test_hash(1),
                test_did(1),
                test_did(2),
                "test dispute".to_string(),
                test_hash(3),
                100,
                200,
            )
            .unwrap();
        let json = serde_json::to_string(&dispute).unwrap();
        let back: InferenceDispute = serde_json::from_str(&json).unwrap();
        assert_eq!(dispute, back);
    }
}
