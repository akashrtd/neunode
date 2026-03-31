use std::collections::HashMap;

use ed25519_dalek::{SigningKey, VerifyingKey};
use neunode_core::constants::reputation::MAX_ATTESTATION_DEPTH;
use neunode_core::types::{Did, Hash256, Signature, Timestamp};
use neunode_crypto::ed25519;
use neunode_crypto::hash::DOMAIN_ATTESTATION;
use serde::{Deserialize, Serialize};

use crate::error::{ReputationError, Result};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct Attestation {
    pub attester: Did,
    pub target: Did,
    pub claim: String,
    pub score: f64,
    pub evidence_hash: Hash256,
    pub timestamp: Timestamp,
    pub signature: Option<Signature>,
}

impl Attestation {
    pub fn new(
        attester: Did,
        target: Did,
        claim: String,
        score: f64,
        evidence_hash: Hash256,
    ) -> Result<Self> {
        if attester == target {
            return Err(ReputationError::SelfAttestation(attester.to_string()));
        }
        if !(0.0..=100.0).contains(&score) {
            return Err(ReputationError::InvalidScore(score));
        }
        if claim.is_empty() {
            return Err(ReputationError::InvalidAttestation("claim cannot be empty".to_string()));
        }
        Ok(Self { attester, target, claim, score, evidence_hash, timestamp: 0, signature: None })
    }

    pub fn sign(&mut self, signing_key: &SigningKey) {
        if self.timestamp == 0 {
            self.timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
        let message = self.signing_payload();
        let sig = ed25519::sign_domain(signing_key, DOMAIN_ATTESTATION, &message);
        self.signature = Some(Signature(format!("ed25519:{}", hex::encode(sig.to_bytes()))));
    }

    pub fn verify(&self, verifying_key: &VerifyingKey) -> bool {
        let Some(sig_str) = &self.signature else {
            return false;
        };
        let sig_bytes = match Self::decode_signature(&sig_str.0) {
            Some(b) => b,
            None => return false,
        };
        let sig = match ed25519_dalek::Signature::from_slice(&sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let message = self.signing_payload();
        ed25519::verify_domain(verifying_key, DOMAIN_ATTESTATION, &message, &sig)
    }

    pub fn validate(&self) -> Result<()> {
        if self.attester == self.target {
            return Err(ReputationError::SelfAttestation(self.attester.to_string()));
        }
        if !(0.0..=100.0).contains(&self.score) {
            return Err(ReputationError::InvalidScore(self.score));
        }
        if self.claim.is_empty() {
            return Err(ReputationError::InvalidAttestation("claim cannot be empty".to_string()));
        }
        Ok(())
    }

    fn signing_payload(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.attester.as_str().as_bytes());
        buf.extend_from_slice(self.target.as_str().as_bytes());
        buf.extend_from_slice(self.claim.as_bytes());
        buf.extend_from_slice(&self.score.to_le_bytes());
        buf.extend_from_slice(self.evidence_hash.0.as_bytes());
        buf.extend_from_slice(&self.timestamp.to_le_bytes());
        buf
    }

    fn decode_signature(sig_str: &str) -> Option<[u8; 64]> {
        let hex_part = sig_str.strip_prefix("ed25519:")?;
        let bytes = hex::decode(hex_part).ok()?;
        if bytes.len() != 64 {
            return None;
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Some(arr)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AttestationGraph {
    incoming: HashMap<Did, Vec<Attestation>>,
    outgoing: HashMap<Did, Vec<Attestation>>,
}

impl AttestationGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_attestation(&mut self, attestation: Attestation) -> Result<()> {
        attestation.validate()?;
        self.outgoing.entry(attestation.attester.clone()).or_default().push(attestation.clone());
        self.incoming.entry(attestation.target.clone()).or_default().push(attestation);
        Ok(())
    }

    pub fn get_attestations_for(&self, did: &Did) -> &[Attestation] {
        self.incoming.get(did).map_or(&[], Vec::as_slice)
    }

    pub fn get_attestations_by(&self, did: &Did) -> &[Attestation] {
        self.outgoing.get(did).map_or(&[], Vec::as_slice)
    }

    pub fn avg_score(&self, did: &Did) -> f64 {
        let attestations = self.incoming.get(did);
        match attestations {
            Some(atts) if !atts.is_empty() => {
                let sum: f64 = atts.iter().map(|a| a.score).sum();
                sum / atts.len() as f64
            }
            _ => 0.0,
        }
    }

    pub fn attestation_count(&self, did: &Did) -> usize {
        self.incoming.get(did).map_or(0, Vec::len)
    }

    pub fn compute_trust_score(&self, did: &Did, max_depth: usize) -> f64 {
        let depth = max_depth.min(MAX_ATTESTATION_DEPTH);
        let mut visited = HashMap::new();
        self.trust_bfs(did, depth, &mut visited)
    }

    fn trust_bfs(&self, did: &Did, remaining_depth: usize, visited: &mut HashMap<Did, f64>) -> f64 {
        if let Some(&score) = visited.get(did) {
            return score;
        }

        let direct = self.incoming.get(did);
        if direct.is_none() || direct.unwrap().is_empty() {
            visited.insert(did.clone(), 0.0);
            return 0.0;
        }

        let direct_attestations = direct.unwrap();

        let mut total_weighted_score = 0.0;
        let mut total_weight = 0.0;

        for att in direct_attestations {
            let attester_weight = if remaining_depth > 0 {
                let attester_score = self.trust_bfs(&att.attester, remaining_depth - 1, visited);
                (attester_score / 100.0).max(0.1)
            } else {
                1.0
            };

            let weight = attester_weight * (att.score / 100.0);
            total_weighted_score += att.score * weight;
            total_weight += weight;
        }

        let score =
            if total_weight > 0.0 { (total_weighted_score / total_weight).min(100.0) } else { 0.0 };

        visited.insert(did.clone(), score);
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_did(name: &str) -> Did {
        Did(format!("did:neunode:{name}"))
    }

    fn make_hash(seed: &str) -> Hash256 {
        let hash = neunode_crypto::hash::sha256(seed.as_bytes());
        Hash256(hex::encode(hash))
    }

    fn make_attestation(attester: &str, target: &str, score: f64) -> Attestation {
        Attestation::new(
            make_did(attester),
            make_did(target),
            format!("{attester} attests {target}"),
            score,
            make_hash(&format!("{attester}->{target}")),
        )
        .unwrap()
    }

    #[test]
    fn attestation_new_valid() {
        let att = Attestation::new(
            make_did("alice"),
            make_did("bob"),
            "completed bounty".to_string(),
            85.0,
            make_hash("evidence1"),
        );
        assert!(att.is_ok());
        let att = att.unwrap();
        assert_eq!(att.attester, make_did("alice"));
        assert_eq!(att.target, make_did("bob"));
        assert!((att.score - 85.0).abs() < f64::EPSILON);
        assert!(att.signature.is_none());
    }

    #[test]
    fn attestation_rejects_self_attestation() {
        let did = make_did("alice");
        let result =
            Attestation::new(did.clone(), did, "self praise".to_string(), 100.0, make_hash("ev"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ReputationError::SelfAttestation(_)));
    }

    #[test]
    fn attestation_rejects_score_above_100() {
        let result = Attestation::new(
            make_did("alice"),
            make_did("bob"),
            "test".to_string(),
            101.0,
            make_hash("ev"),
        );
        assert!(matches!(result.unwrap_err(), ReputationError::InvalidScore(_)));
    }

    #[test]
    fn attestation_rejects_negative_score() {
        let result = Attestation::new(
            make_did("alice"),
            make_did("bob"),
            "test".to_string(),
            -1.0,
            make_hash("ev"),
        );
        assert!(matches!(result.unwrap_err(), ReputationError::InvalidScore(_)));
    }

    #[test]
    fn attestation_rejects_empty_claim() {
        let result = Attestation::new(
            make_did("alice"),
            make_did("bob"),
            "".to_string(),
            50.0,
            make_hash("ev"),
        );
        assert!(matches!(result.unwrap_err(), ReputationError::InvalidAttestation(_)));
    }

    #[test]
    fn attestation_accepts_zero_score() {
        let att = Attestation::new(
            make_did("alice"),
            make_did("bob"),
            "poor work".to_string(),
            0.0,
            make_hash("ev"),
        );
        assert!(att.is_ok());
    }

    #[test]
    fn attestation_accepts_exact_100_score() {
        let att = Attestation::new(
            make_did("alice"),
            make_did("bob"),
            "perfect work".to_string(),
            100.0,
            make_hash("ev"),
        );
        assert!(att.is_ok());
    }

    #[test]
    fn attestation_sign_and_verify() {
        let (sk, vk) = neunode_crypto::ed25519::generate_keypair();
        let mut att = make_attestation("alice", "bob", 90.0);
        att.sign(&sk);
        assert!(att.signature.is_some());
        assert!(att.verify(&vk));
    }

    #[test]
    fn attestation_wrong_key_fails_verify() {
        let (sk, _) = neunode_crypto::ed25519::generate_keypair();
        let (_, wrong_vk) = neunode_crypto::ed25519::generate_keypair();
        let mut att = make_attestation("alice", "bob", 90.0);
        att.sign(&sk);
        assert!(!att.verify(&wrong_vk));
    }

    #[test]
    fn attestation_unsigned_fails_verify() {
        let (_, vk) = neunode_crypto::ed25519::generate_keypair();
        let att = make_attestation("alice", "bob", 90.0);
        assert!(!att.verify(&vk));
    }

    #[test]
    fn attestation_sign_sets_timestamp() {
        let (sk, _) = neunode_crypto::ed25519::generate_keypair();
        let mut att = make_attestation("alice", "bob", 90.0);
        assert_eq!(att.timestamp, 0);
        att.sign(&sk);
        assert!(att.timestamp > 0);
    }

    #[test]
    fn attestation_validate_rejects_self() {
        let (sk, _) = neunode_crypto::ed25519::generate_keypair();
        let mut att = make_attestation("alice", "bob", 80.0);
        att.attester = make_did("bob");
        att.target = make_did("bob");
        att.sign(&sk);
        assert!(att.validate().is_err());
    }

    #[test]
    fn attestation_serde_roundtrip() {
        let (sk, _) = neunode_crypto::ed25519::generate_keypair();
        let mut att = make_attestation("alice", "bob", 75.0);
        att.sign(&sk);
        let json = serde_json::to_string(&att).unwrap();
        let back: Attestation = serde_json::from_str(&json).unwrap();
        assert_eq!(att, back);
    }

    #[test]
    fn graph_add_attestation() {
        let mut graph = AttestationGraph::new();
        let att = make_attestation("alice", "bob", 80.0);
        assert!(graph.add_attestation(att).is_ok());
        assert_eq!(graph.attestation_count(&make_did("bob")), 1);
    }

    #[test]
    fn graph_rejects_self_attestation() {
        let did = make_did("alice");
        let err = Attestation::new(did.clone(), did, "self".to_string(), 50.0, make_hash("ev"))
            .unwrap_err();
        assert!(matches!(err, ReputationError::SelfAttestation(_)));
    }

    #[test]
    fn graph_multiple_attestations() {
        let mut graph = AttestationGraph::new();
        graph.add_attestation(make_attestation("alice", "bob", 80.0)).unwrap();
        graph.add_attestation(make_attestation("carol", "bob", 90.0)).unwrap();
        graph.add_attestation(make_attestation("dave", "bob", 70.0)).unwrap();
        assert_eq!(graph.attestation_count(&make_did("bob")), 3);
    }

    #[test]
    fn graph_avg_score() {
        let mut graph = AttestationGraph::new();
        graph.add_attestation(make_attestation("alice", "bob", 80.0)).unwrap();
        graph.add_attestation(make_attestation("carol", "bob", 90.0)).unwrap();
        let avg = graph.avg_score(&make_did("bob"));
        assert!((avg - 85.0).abs() < f64::EPSILON);
    }

    #[test]
    fn graph_avg_score_no_attestations() {
        let graph = AttestationGraph::new();
        assert!((graph.avg_score(&make_did("nobody")) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn graph_get_attestations_for_empty() {
        let graph = AttestationGraph::new();
        assert!(graph.get_attestations_for(&make_did("nobody")).is_empty());
    }

    #[test]
    fn graph_get_attestations_by() {
        let mut graph = AttestationGraph::new();
        graph.add_attestation(make_attestation("alice", "bob", 80.0)).unwrap();
        graph.add_attestation(make_attestation("alice", "carol", 90.0)).unwrap();
        assert_eq!(graph.get_attestations_by(&make_did("alice")).len(), 2);
    }

    #[test]
    fn trust_score_single_level() {
        let mut graph = AttestationGraph::new();
        graph.add_attestation(make_attestation("alice", "bob", 80.0)).unwrap();
        graph.add_attestation(make_attestation("carol", "bob", 90.0)).unwrap();
        let score = graph.compute_trust_score(&make_did("bob"), 1);
        assert!(score > 0.0 && score <= 100.0);
    }

    #[test]
    fn trust_score_no_attestations() {
        let graph = AttestationGraph::new();
        let score = graph.compute_trust_score(&make_did("nobody"), 3);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn trust_score_transitive() {
        let mut graph = AttestationGraph::new();
        // dave attests alice (high score) -> alice attests bob (high score)
        // bob should get a higher trust score with transitive depth
        graph.add_attestation(make_attestation("dave", "alice", 90.0)).unwrap();
        graph.add_attestation(make_attestation("alice", "bob", 80.0)).unwrap();

        let score_d0 = graph.compute_trust_score(&make_did("bob"), 0);
        let score_d1 = graph.compute_trust_score(&make_did("bob"), 1);
        let score_d2 = graph.compute_trust_score(&make_did("bob"), 2);

        // Deeper trust chains should still produce valid scores
        assert!(score_d0 > 0.0);
        assert!(score_d1 > 0.0);
        assert!(score_d2 > 0.0);
        assert!(score_d2 <= 100.0);
    }

    #[test]
    fn trust_score_respects_max_depth() {
        let graph = AttestationGraph::new();
        // Requesting depth > MAX_ATTESTATION_DEPTH should be clamped
        let score = graph.compute_trust_score(&make_did("nobody"), 100);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn graph_serde_roundtrip() {
        let mut graph = AttestationGraph::new();
        graph.add_attestation(make_attestation("alice", "bob", 80.0)).unwrap();
        graph.add_attestation(make_attestation("carol", "bob", 90.0)).unwrap();
        let json = serde_json::to_string(&graph).unwrap();
        let back: AttestationGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(back.attestation_count(&make_did("bob")), 2);
        assert!((back.avg_score(&make_did("bob")) - 85.0).abs() < f64::EPSILON);
    }
}
