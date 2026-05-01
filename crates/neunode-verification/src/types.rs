use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Verification tier — escalation levels with increasing cost/confidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum VerificationTier {
    /// Automated hash/format checks (~1.05x cost).
    Tier1,
    /// RepOps deterministic execution (~1.1x cost).
    Tier2,
    /// 2-of-3 peer review (~1.15x cost).
    Tier3,
    /// ZK proof / arbitration (~1000x cost, critical tasks only).
    Tier4,
}

impl VerificationTier {
    /// Default confidence level for each verification tier.
    pub fn confidence(&self) -> f64 {
        match self {
            Self::Tier1 => 0.85,
            Self::Tier2 => 0.95,
            Self::Tier3 => 0.99,
            Self::Tier4 => 0.999,
        }
    }
}

/// Verification layer — specific method applied during verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum VerificationLayer {
    /// Hash comparison, format validation.
    Automated,
    /// Bitwise reproducibility check.
    RepOps,
    /// 2-of-3 reviewer committee.
    PeerReview,
    /// Verde-style dispute resolution.
    Bisection,
    /// Trusted execution environment proof.
    TeeAttestation,
    /// Zero-knowledge proof (future).
    ZkProof,
    /// Kleros-style final arbitration.
    Arbitration,
}

/// Outcome of a single verification step.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VerificationResult {
    pub layer: VerificationLayer,
    pub passed: bool,
    pub confidence: f64,
    pub evidence_hash: String,
    pub verifier_id: String,
    #[ts(type = "number")]
    pub timestamp_ms: u64,
    pub details: Option<String>,
}

/// Request to verify a work artifact.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VerificationRequest {
    pub artifact_hash: String,
    pub task_type: String,
    pub tier: VerificationTier,
    pub input_hash: String,
    pub output_hash: String,
    pub model_cid: Option<String>,
    pub requester_did: String,
    #[ts(type = "number")]
    pub timestamp_ms: u64,
}

/// Hash of an artifact with algorithm metadata.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArtifactHash {
    pub algorithm: String,
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_serde_roundtrip() {
        let tiers = vec![
            VerificationTier::Tier1,
            VerificationTier::Tier2,
            VerificationTier::Tier3,
            VerificationTier::Tier4,
        ];
        for tier in tiers {
            let json = serde_json::to_string(&tier).unwrap();
            let back: VerificationTier = serde_json::from_str(&json).unwrap();
            assert_eq!(tier, back);
        }
    }

    #[test]
    fn tier_snake_case_serialization() {
        let json = serde_json::to_string(&VerificationTier::Tier1).unwrap();
        assert!(json.contains("tier1"));
        let json = serde_json::to_string(&VerificationTier::Tier4).unwrap();
        assert!(json.contains("tier4"));
    }

    #[test]
    fn tier_confidence_values() {
        assert!((VerificationTier::Tier1.confidence() - 0.85).abs() < f64::EPSILON);
        assert!((VerificationTier::Tier2.confidence() - 0.95).abs() < f64::EPSILON);
        assert!((VerificationTier::Tier3.confidence() - 0.99).abs() < f64::EPSILON);
        assert!((VerificationTier::Tier4.confidence() - 0.999).abs() < f64::EPSILON);
    }

    #[test]
    fn layer_serde_roundtrip() {
        let layers = vec![
            VerificationLayer::Automated,
            VerificationLayer::RepOps,
            VerificationLayer::PeerReview,
            VerificationLayer::Bisection,
            VerificationLayer::TeeAttestation,
            VerificationLayer::ZkProof,
            VerificationLayer::Arbitration,
        ];
        for layer in layers {
            let json = serde_json::to_string(&layer).unwrap();
            let back: VerificationLayer = serde_json::from_str(&json).unwrap();
            assert_eq!(layer, back);
        }
    }

    #[test]
    fn layer_snake_case_serialization() {
        let json = serde_json::to_string(&VerificationLayer::TeeAttestation).unwrap();
        assert!(json.contains("tee_attestation"));
        let json = serde_json::to_string(&VerificationLayer::ZkProof).unwrap();
        assert!(json.contains("zk_proof"));
        let json = serde_json::to_string(&VerificationLayer::PeerReview).unwrap();
        assert!(json.contains("peer_review"));
    }

    #[test]
    fn result_serde_roundtrip() {
        let result = VerificationResult {
            layer: VerificationLayer::Automated,
            passed: true,
            confidence: 0.85,
            evidence_hash: "abc123".to_string(),
            verifier_id: "did:neunode:verifier".to_string(),
            timestamp_ms: 1700000000000,
            details: Some("all checks passed".to_string()),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: VerificationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.layer, back.layer);
        assert_eq!(result.passed, back.passed);
        assert!((result.confidence - back.confidence).abs() < f64::EPSILON);
        assert_eq!(result.evidence_hash, back.evidence_hash);
        assert_eq!(result.verifier_id, back.verifier_id);
        assert_eq!(result.timestamp_ms, back.timestamp_ms);
        assert_eq!(result.details, back.details);
    }

    #[test]
    fn result_with_optional_fields() {
        let result_no_details = VerificationResult {
            layer: VerificationLayer::RepOps,
            passed: false,
            confidence: 0.95,
            evidence_hash: "def456".to_string(),
            verifier_id: "did:neunode:v2".to_string(),
            timestamp_ms: 1700000000000,
            details: None,
        };
        let json = serde_json::to_string(&result_no_details).unwrap();
        let back: VerificationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.details, None);
    }

    #[test]
    fn request_serde_roundtrip() {
        let req = VerificationRequest {
            artifact_hash: "sha256:abcd".to_string(),
            task_type: "inference".to_string(),
            tier: VerificationTier::Tier2,
            input_hash: "sha256:input".to_string(),
            output_hash: "sha256:output".to_string(),
            model_cid: Some("blake3:model".to_string()),
            requester_did: "did:neunode:abc".to_string(),
            timestamp_ms: 1700000000000,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: VerificationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.artifact_hash, back.artifact_hash);
        assert_eq!(req.tier, back.tier);
        assert_eq!(req.model_cid, back.model_cid);
    }

    #[test]
    fn artifact_hash_serde_roundtrip() {
        let hash =
            ArtifactHash { algorithm: "sha256".to_string(), value: "a]b1c2d3e4".to_string() };
        let json = serde_json::to_string(&hash).unwrap();
        let back: ArtifactHash = serde_json::from_str(&json).unwrap();
        assert_eq!(hash.algorithm, back.algorithm);
        assert_eq!(hash.value, back.value);
    }

    #[test]
    fn ts_type_declarations() {
        let cfg = ts_rs::Config::default();
        let _ = <VerificationTier as ts_rs::TS>::decl(&cfg);
        let _ = <VerificationLayer as ts_rs::TS>::decl(&cfg);
        let _ = <VerificationResult as ts_rs::TS>::decl(&cfg);
        let _ = <VerificationRequest as ts_rs::TS>::decl(&cfg);
        let _ = <ArtifactHash as ts_rs::TS>::decl(&cfg);
    }
}
