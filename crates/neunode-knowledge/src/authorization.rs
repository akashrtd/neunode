use ed25519_dalek::Signature;
use neunode_crypto::ed25519;
use neunode_crypto::hash::DOMAIN_KG_MUTATION;

use crate::error::{KnowledgeError, Result};

#[derive(Debug, Clone)]
pub struct MutationAuthorization {
    pub signer_did: String,
    pub verifying_key: [u8; 32],
    pub signature: Signature,
}

impl MutationAuthorization {
    pub fn sign(
        signer_did: String,
        verifying_key: [u8; 32],
        signing_key_bytes: &[u8; 32],
        payload: &[u8],
    ) -> Result<Self> {
        let signing_key = ed25519::signing_key_from_bytes(signing_key_bytes)
            .map_err(|e| KnowledgeError::AuthorizationError(format!("invalid signing key: {e}")))?;
        let signature = ed25519::sign_domain(&signing_key, DOMAIN_KG_MUTATION, payload);
        Ok(Self { signer_did, verifying_key, signature })
    }

    pub fn verify(&self, payload: &[u8]) -> Result<()> {
        let verifying_key =
            ed25519::verifying_key_from_bytes(&self.verifying_key).map_err(|e| {
                KnowledgeError::AuthorizationError(format!("invalid verifying key: {e}"))
            })?;
        let valid =
            ed25519::verify_domain(&verifying_key, DOMAIN_KG_MUTATION, payload, &self.signature);
        if valid {
            Ok(())
        } else {
            Err(KnowledgeError::AuthorizationError(
                "mutation signature verification failed".to_string(),
            ))
        }
    }
}

pub fn canonical_register_agent(agent_did: &str, capabilities: &[&str]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "mutation": "register_agent",
        "agent_did": agent_did,
        "capabilities": capabilities,
    }))
    .unwrap_or_default()
}

pub fn canonical_register_model(
    owner_did: &str,
    model_cid: &str,
    parent_cid: Option<&str>,
) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "mutation": "register_model",
        "owner_did": owner_did,
        "model_cid": model_cid,
        "parent_cid": parent_cid,
    }))
    .unwrap_or_default()
}

pub fn canonical_register_bounty(bounty_id: &str, capabilities: &[&str]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "mutation": "register_bounty",
        "bounty_id": bounty_id,
        "capabilities": capabilities,
    }))
    .unwrap_or_default()
}

pub fn canonical_join_training_job(agent_did: &str, job_id: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "mutation": "join_training_job",
        "agent_did": agent_did,
        "job_id": job_id,
    }))
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use neunode_crypto::ed25519;

    fn test_keypair() -> (ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey) {
        let (sk, vk) = ed25519::generate_keypair();
        (sk, vk)
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let (sk, vk) = test_keypair();
        let payload = canonical_register_agent("did:neunode:test", &["NLP"]);
        let sk_bytes = ed25519::signing_key_to_bytes(&sk);
        let vk_bytes = ed25519::verifying_key_to_bytes(&vk);
        let auth =
            MutationAuthorization::sign("did:neunode:test".into(), vk_bytes, &sk_bytes, &payload)
                .unwrap();
        auth.verify(&payload).unwrap();
    }

    #[test]
    fn verify_wrong_payload_fails() {
        let (sk, vk) = test_keypair();
        let payload_a = canonical_register_agent("did:neunode:a", &["NLP"]);
        let payload_b = canonical_register_agent("did:neunode:b", &["Vision"]);
        let sk_bytes = ed25519::signing_key_to_bytes(&sk);
        let vk_bytes = ed25519::verifying_key_to_bytes(&vk);
        let auth =
            MutationAuthorization::sign("did:neunode:a".into(), vk_bytes, &sk_bytes, &payload_a)
                .unwrap();
        assert!(auth.verify(&payload_b).is_err());
    }

    #[test]
    fn verify_wrong_key_fails() {
        let (sk1, _) = test_keypair();
        let (_, vk2) = test_keypair();
        let payload = canonical_register_agent("did:neunode:test", &["NLP"]);
        let sk1_bytes = ed25519::signing_key_to_bytes(&sk1);
        let vk2_bytes = ed25519::verifying_key_to_bytes(&vk2);
        let auth =
            MutationAuthorization::sign("did:neunode:test".into(), vk2_bytes, &sk1_bytes, &payload)
                .unwrap();
        assert!(auth.verify(&payload).is_err());
    }

    #[test]
    fn canonical_register_agent_deterministic() {
        let p1 = canonical_register_agent("did:neunode:test", &["NLP", "Vision"]);
        let p2 = canonical_register_agent("did:neunode:test", &["NLP", "Vision"]);
        assert_eq!(p1, p2);
    }

    #[test]
    fn canonical_register_model_with_parent() {
        let p =
            canonical_register_model("did:neunode:dev", "ipfs://QmChild", Some("ipfs://QmParent"));
        let s = String::from_utf8(p).unwrap();
        assert!(s.contains("parent_cid"));
        assert!(s.contains("ipfs://QmParent"));
    }

    #[test]
    fn canonical_register_model_without_parent() {
        let p = canonical_register_model("did:neunode:dev", "ipfs://QmModel", None);
        let s = String::from_utf8(p).unwrap();
        assert!(s.contains("null"));
    }

    #[test]
    fn canonical_join_training_job_format() {
        let p = canonical_join_training_job("did:neunode:worker", "job:101");
        let s = String::from_utf8(p).unwrap();
        assert!(s.contains("join_training_job"));
        assert!(s.contains("job:101"));
    }
}
