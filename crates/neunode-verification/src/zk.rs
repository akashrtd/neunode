use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Result, VerificationError};
use crate::types::{VerificationLayer, VerificationResult};

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// ZK proof system types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum ZkProofSystem {
    /// Halo2 (used by EZKL).
    Halo2,
    /// Groth16 (future).
    Groth16,
    /// Plonky2 (future).
    Plonky2,
}

/// ZK proof verification result.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ZkProofResult {
    /// Proof system used.
    pub proof_system: ZkProofSystem,
    /// Whether the proof verified successfully.
    pub verified: bool,
    /// Size of the proof in bytes.
    #[ts(type = "number")]
    pub proof_size_bytes: u64,
    /// Time to verify the proof in milliseconds.
    #[ts(type = "number")]
    pub verify_time_ms: u64,
    /// Public inputs hash (SHA-256 of instance values).
    pub public_inputs_hash: String,
    /// Model CID that was proven.
    pub model_cid: String,
}

/// Placeholder for ZK proof verification.
///
/// Production implementation will use EZKL (zkonduit/ezkl v23.0.5)
/// for ONNX→Halo2 circuit→proof→EVM verification of small models.
/// LLM-scale ZK is infeasible today; use RepOps or TEE instead.
#[derive(Default)]
pub struct ZkVerifier;

impl ZkVerifier {
    pub fn new() -> Self {
        Self
    }

    pub fn verify_proof(
        &self,
        _proof: &[u8],
        _public_inputs: &[u8],
        model_cid: &str,
    ) -> Result<ZkProofResult> {
        Err(VerificationError::Unsupported {
            layer: "zk".to_string(),
            reason: format!(
                "ZK proof verification not yet implemented for model {model_cid}; \
                 use RepOps or TEE attestation for LLM-scale verification"
            ),
        })
    }

    pub fn verify_proof_result(&self, proof_result: &ZkProofResult) -> VerificationResult {
        VerificationResult {
            layer: VerificationLayer::ZkProof,
            passed: proof_result.verified,
            confidence: if proof_result.verified { 1.0 } else { 0.0 },
            evidence_hash: proof_result.public_inputs_hash.clone(),
            verifier_id: "zk_verifier".to_string(),
            timestamp_ms: now_ms(),
            details: Some(format!(
                "proof_system={:?}, size={}bytes",
                proof_result.proof_system, proof_result.proof_size_bytes,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zk_proof_system_serde_roundtrip() {
        let systems = vec![ZkProofSystem::Halo2, ZkProofSystem::Groth16, ZkProofSystem::Plonky2];
        for sys in systems {
            let json = serde_json::to_string(&sys).unwrap();
            let back: ZkProofSystem = serde_json::from_str(&json).unwrap();
            assert_eq!(sys, back);
        }
    }

    #[test]
    fn zk_proof_system_snake_case() {
        let json = serde_json::to_string(&ZkProofSystem::Halo2).unwrap();
        assert_eq!(json, "\"halo2\"");
    }

    #[test]
    fn zk_proof_result_serde_roundtrip() {
        let result = ZkProofResult {
            proof_system: ZkProofSystem::Halo2,
            verified: true,
            proof_size_bytes: 2048,
            verify_time_ms: 150,
            public_inputs_hash: "sha256:abc123".to_string(),
            model_cid: "sha256:model456".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: ZkProofResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.proof_system, back.proof_system);
        assert_eq!(result.verified, back.verified);
        assert_eq!(result.proof_size_bytes, back.proof_size_bytes);
        assert_eq!(result.verify_time_ms, back.verify_time_ms);
        assert_eq!(result.public_inputs_hash, back.public_inputs_hash);
        assert_eq!(result.model_cid, back.model_cid);
    }

    #[test]
    fn verify_proof_returns_unsupported() {
        let verifier = ZkVerifier::new();
        let result = verifier.verify_proof(&[], &[], "test-model");
        let err = result.expect_err("ZK verify must signal unsupported, not pass");
        assert!(
            matches!(err, VerificationError::Unsupported { .. }),
            "expected Unsupported, got {err:?}"
        );
    }

    #[test]
    fn verify_proof_result_verified() {
        let verifier = ZkVerifier::new();
        let proof_result = ZkProofResult {
            proof_system: ZkProofSystem::Halo2,
            verified: true,
            proof_size_bytes: 1024,
            verify_time_ms: 50,
            public_inputs_hash: "sha256:proof".to_string(),
            model_cid: "sha256:model".to_string(),
        };
        let vr = verifier.verify_proof_result(&proof_result);
        assert!(vr.passed);
        assert_eq!(vr.confidence, 1.0);
        assert_eq!(vr.layer, VerificationLayer::ZkProof);
        assert_eq!(vr.evidence_hash, "sha256:proof");
    }

    #[test]
    fn verify_proof_result_failed() {
        let verifier = ZkVerifier::new();
        let proof_result = ZkProofResult {
            proof_system: ZkProofSystem::Groth16,
            verified: false,
            proof_size_bytes: 512,
            verify_time_ms: 10,
            public_inputs_hash: "sha256:bad".to_string(),
            model_cid: "sha256:m".to_string(),
        };
        let vr = verifier.verify_proof_result(&proof_result);
        assert!(!vr.passed);
        assert_eq!(vr.confidence, 0.0);
    }

    #[test]
    fn ts_type_declarations() {
        let cfg = ts_rs::Config::default();
        let _ = <ZkProofSystem as ts_rs::TS>::decl(&cfg);
        let _ = <ZkProofResult as ts_rs::TS>::decl(&cfg);
    }
}
