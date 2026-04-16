use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use neunode_crypto::ed25519::{sign_domain, verify_domain};
use neunode_crypto::hash::DOMAIN_MODEL_LINEAGE;

use crate::types::ModelNode;

pub fn sign_model_node(signing_key: &SigningKey, node: &ModelNode) -> Signature {
    let payload = canonical_payload(node);
    sign_domain(signing_key, DOMAIN_MODEL_LINEAGE, payload.as_bytes())
}

pub fn verify_model_node(verifying_key: &VerifyingKey, node: &ModelNode) -> bool {
    let payload = canonical_payload(node);
    let sig = match Signature::from_slice(&node.signature) {
        Ok(s) => s,
        Err(_) => return false,
    };
    verify_domain(verifying_key, DOMAIN_MODEL_LINEAGE, payload.as_bytes(), &sig)
}

fn canonical_payload(node: &ModelNode) -> String {
    let payload = serde_json::json!({
        "cid": node.cid,
        "parent_cids": node.parent_cids,
        "contributor_did": node.contributor_did,
        "contribution_type": node.contribution_type,
        "created_at": node.created_at,
    });
    serde_json::to_string(&payload).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ContributionType, ModelMetadata};

    fn make_test_node(cid: &str, parent_cids: Vec<&str>, did: &str, created_at: u64) -> ModelNode {
        ModelNode {
            cid: cid.to_string(),
            parent_cids: parent_cids.iter().map(|s| s.to_string()).collect(),
            contributor_did: did.to_string(),
            contribution_type: ContributionType::PreTraining,
            signature: vec![0u8; 64],
            created_at,
            metadata: ModelMetadata::default(),
        }
    }

    fn signed_node(
        signing_key: &SigningKey,
        cid: &str,
        parent_cids: Vec<&str>,
        did: &str,
        created_at: u64,
    ) -> ModelNode {
        let mut node = make_test_node(cid, parent_cids, did, created_at);
        let sig = sign_model_node(signing_key, &node);
        node.signature = sig.to_bytes().to_vec();
        node
    }

    #[test]
    fn sign_verify_roundtrip() {
        let (sk, vk) = neunode_crypto::ed25519::generate_keypair();
        let node = signed_node(&sk, "sha256:abc", vec![], "did:agent:1", 100);
        assert!(verify_model_node(&vk, &node));
    }

    #[test]
    fn wrong_key_fails() {
        let (sk1, _) = neunode_crypto::ed25519::generate_keypair();
        let (_, vk2) = neunode_crypto::ed25519::generate_keypair();
        let node = signed_node(&sk1, "sha256:abc", vec![], "did:agent:1", 100);
        assert!(!verify_model_node(&vk2, &node));
    }

    #[test]
    fn tampered_cid_fails() {
        let (sk, vk) = neunode_crypto::ed25519::generate_keypair();
        let mut node = signed_node(&sk, "sha256:original", vec![], "did:agent:1", 100);
        node.cid = "sha256:tampered".to_string();
        assert!(!verify_model_node(&vk, &node));
    }

    #[test]
    fn tampered_parent_fails() {
        let (sk, vk) = neunode_crypto::ed25519::generate_keypair();
        let mut node = signed_node(&sk, "sha256:child", vec!["sha256:p1"], "did:agent:1", 100);
        node.parent_cids.push("sha256:injected".to_string());
        assert!(!verify_model_node(&vk, &node));
    }

    #[test]
    fn tampered_did_fails() {
        let (sk, vk) = neunode_crypto::ed25519::generate_keypair();
        let mut node = signed_node(&sk, "sha256:abc", vec![], "did:agent:1", 100);
        node.contributor_did = "did:agent:evil".to_string();
        assert!(!verify_model_node(&vk, &node));
    }

    #[test]
    fn tampered_timestamp_fails() {
        let (sk, vk) = neunode_crypto::ed25519::generate_keypair();
        let mut node = signed_node(&sk, "sha256:abc", vec![], "did:agent:1", 100);
        node.created_at = 999;
        assert!(!verify_model_node(&vk, &node));
    }

    #[test]
    fn different_nodes_different_signatures() {
        let (sk, _) = neunode_crypto::ed25519::generate_keypair();
        let node1 = signed_node(&sk, "sha256:a", vec![], "did:agent:1", 100);
        let node2 = signed_node(&sk, "sha256:b", vec![], "did:agent:1", 100);
        assert_ne!(node1.signature, node2.signature);
    }

    #[test]
    fn canonical_payload_deterministic() {
        let node = make_test_node("sha256:abc", vec!["sha256:p1"], "did:agent:1", 100);
        let p1 = canonical_payload(&node);
        let p2 = canonical_payload(&node);
        assert_eq!(p1, p2);
    }

    #[test]
    fn canonical_payload_excludes_signature() {
        let mut node = make_test_node("sha256:abc", vec![], "did:agent:1", 100);
        node.signature = vec![0u8; 64];
        let p1 = canonical_payload(&node);
        node.signature = vec![255u8; 64];
        let p2 = canonical_payload(&node);
        assert_eq!(p1, p2);
    }

    #[test]
    fn canonical_payload_excludes_metadata() {
        let mut node = make_test_node("sha256:abc", vec![], "did:agent:1", 100);
        node.metadata.dataset_hash = Some("sha256:ds".to_string());
        let p1 = canonical_payload(&node);
        node.metadata = ModelMetadata::default();
        let p2 = canonical_payload(&node);
        assert_eq!(p1, p2);
    }

    #[test]
    fn canonical_payload_includes_contribution_type() {
        let mut node1 = make_test_node("sha256:abc", vec![], "did:agent:1", 100);
        node1.contribution_type = ContributionType::PreTraining;
        let mut node2 = make_test_node("sha256:abc", vec![], "did:agent:1", 100);
        node2.contribution_type = ContributionType::Merge { merge_method: "slerp".to_string() };
        let p1 = canonical_payload(&node1);
        let p2 = canonical_payload(&node2);
        assert_ne!(p1, p2, "different contribution types must produce different payloads");
    }

    #[test]
    fn signature_bytes_length_64() {
        let (sk, _) = neunode_crypto::ed25519::generate_keypair();
        let node = make_test_node("sha256:abc", vec![], "did:agent:1", 100);
        let sig = sign_model_node(&sk, &node);
        assert_eq!(sig.to_bytes().len(), 64);
    }

    #[test]
    fn serde_roundtrip_on_model_node() {
        let (sk, _) = neunode_crypto::ed25519::generate_keypair();
        let node = signed_node(&sk, "sha256:abc", vec!["sha256:p1"], "did:agent:1", 1712000000000);
        let json = serde_json::to_string(&node).unwrap();
        let back: ModelNode = serde_json::from_str(&json).unwrap();
        assert_eq!(back.cid, node.cid);
        assert_eq!(back.signature, node.signature);
        assert!(verify_model_node(&sk.verifying_key(), &back,));
    }

    #[test]
    fn contribution_type_serde() {
        let ct = ContributionType::FineTune { lora_rank: 16, lora_alpha: 32.0 };
        let json = serde_json::to_string(&ct).unwrap();
        let back: ContributionType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ct);
    }

    #[test]
    fn contribution_type_snake_case() {
        let json = serde_json::to_string(&ContributionType::PreTraining).unwrap();
        assert_eq!(json, "\"pre_training\"");
    }

    #[test]
    fn ts_exports_non_empty() {
        use ts_rs::TS;
        let cfg = ts_rs::Config::default();
        assert!(!crate::types::ModelNode::decl(&cfg).is_empty());
    }

    #[test]
    fn invalid_signature_bytes_fails() {
        let (_, vk) = neunode_crypto::ed25519::generate_keypair();
        let mut node = make_test_node("sha256:abc", vec![], "did:agent:1", 100);
        node.signature = vec![0u8; 10];
        assert!(!verify_model_node(&vk, &node));
    }

    #[test]
    fn domain_separated_from_raw() {
        let (sk, _) = neunode_crypto::ed25519::generate_keypair();
        let node = make_test_node("sha256:abc", vec![], "did:agent:1", 100);
        let domain_sig = sign_model_node(&sk, &node);
        let raw_sig = neunode_crypto::ed25519::sign(&sk, b"test");
        assert_ne!(domain_sig, raw_sig);
    }

    #[test]
    fn verify_with_contribution_type_change() {
        // SECURITY: Changing contribution_type AFTER signing MUST invalidate the signature.
        // Previously, contribution_type was excluded from the canonical payload, allowing
        // an attacker to change a PreTraining contribution to Merge (or any other type)
        // without invalidating the signature — directly affecting royalty distribution.
        let (sk, vk) = neunode_crypto::ed25519::generate_keypair();
        let mut node = signed_node(&sk, "sha256:abc", vec![], "did:agent:1", 100);
        node.contribution_type = ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 };
        assert!(
            !verify_model_node(&vk, &node),
            "changing contribution_type after signing MUST invalidate the signature"
        );
    }
}
