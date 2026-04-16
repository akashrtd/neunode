use std::collections::HashMap;

use chrono::Utc;
use ed25519_dalek::Verifier;
use neunode_core::{AgentLifecycle, Did, NeunodeError, PeerId, CID};
use neunode_crypto::hash::{multihash_sha256, sha256, DOMAIN_AGENT_CARD};
use serde::{Deserialize, Serialize};

use crate::did::did_to_peer_id;
use crate::keyring::{Keyring, PublicKeyBundle};
use ts_rs::TS;

const BASE32_ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

fn base32_lower_encode(data: &[u8]) -> String {
    let mut result = String::new();
    let mut bits = 0u64;
    let mut n_bits = 0u32;

    for &byte in data {
        bits = (bits << 8) | byte as u64;
        n_bits += 8;
        while n_bits >= 5 {
            n_bits -= 5;
            let idx = ((bits >> n_bits) & 0x1F) as usize;
            result.push(BASE32_ALPHABET[idx] as char);
        }
    }

    if n_bits > 0 {
        let idx = ((bits << (5 - n_bits)) & 0x1F) as usize;
        result.push(BASE32_ALPHABET[idx] as char);
    }

    result
}

/// Agent metadata card — the "profile" of an AI agent on the network.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct AgentCard {
    pub did: Did,
    pub name: String,
    pub version: u32,
    pub capabilities: Vec<String>,
    pub lifecycle: AgentLifecycle,
    pub peer_id: PeerId,
    pub public_key_bundle: PublicKeyBundle,
    pub metadata: HashMap<String, String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A cryptographically signed agent card.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct SignedAgentCard {
    pub card: AgentCard,
    pub signature: Vec<u8>,
    pub signed_at: i64,
}

/// Fluent builder for `AgentCard`.
pub struct AgentCardBuilder {
    name: String,
    capabilities: Vec<String>,
    metadata: HashMap<String, String>,
}

impl AgentCardBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), capabilities: Vec::new(), metadata: HashMap::new() }
    }

    pub fn capability(mut self, cap: impl Into<String>) -> Self {
        self.capabilities.push(cap.into());
        self
    }

    pub fn capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = caps;
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn build(self, keyring: &Keyring) -> std::result::Result<AgentCard, NeunodeError> {
        AgentCard::new(&self.name, keyring, self.capabilities, self.metadata)
    }
}

impl AgentCard {
    /// Create a new agent card from a keyring and basic metadata.
    pub fn new(
        name: &str,
        keyring: &Keyring,
        capabilities: Vec<String>,
        metadata: HashMap<String, String>,
    ) -> std::result::Result<Self, NeunodeError> {
        let now = Utc::now().timestamp();
        let did = keyring.to_did();
        let did_key = keyring.to_did_key();
        let peer_id = did_to_peer_id(&did_key).unwrap_or_else(|_| PeerId("unknown".into()));

        Ok(Self {
            did,
            name: name.to_string(),
            version: 1,
            capabilities,
            lifecycle: AgentLifecycle::Created,
            peer_id,
            public_key_bundle: keyring.export_public(),
            metadata,
            created_at: now,
            updated_at: now,
        })
    }

    fn canonical_hash(&self) -> [u8; 32] {
        let value = serde_json::to_value(self).unwrap_or_default();
        let canonical = serde_json::to_string(&value).unwrap_or_default();
        sha256(canonical.as_bytes())
    }

    pub fn sign(&self, keyring: &Keyring) -> SignedAgentCard {
        let hash = self.canonical_hash();
        let domain_hash = sha256_domain_card(&hash);
        let sig = keyring.sign_ed25519(&domain_hash);
        SignedAgentCard {
            card: self.clone(),
            signature: sig.to_bytes().to_vec(),
            signed_at: Utc::now().timestamp(),
        }
    }

    pub fn to_cid(&self) -> CID {
        let value = serde_json::to_value(self).unwrap_or_default();
        let canonical = serde_json::to_string(&value).unwrap_or_default();
        let mh = multihash_sha256(canonical.as_bytes());

        let mut cid_bytes = Vec::with_capacity(2 + mh.len());
        cid_bytes.push(0x01);
        cid_bytes.push(0x55);
        cid_bytes.extend_from_slice(&mh);

        CID(format!("b{}", base32_lower_encode(&cid_bytes)))
    }
}

impl SignedAgentCard {
    pub fn verify(&self) -> bool {
        let hash = self.card.canonical_hash();
        let domain_hash = sha256_domain_card(&hash);

        let pub_bytes: [u8; 32] = match self.card.public_key_bundle.ed25519.clone().try_into() {
            Ok(b) => b,
            Err(_) => return false,
        };
        let vk = match ed25519_dalek::VerifyingKey::from_bytes(&pub_bytes) {
            Ok(vk) => vk,
            Err(_) => return false,
        };
        let sig_bytes: [u8; 64] = match self.signature.clone().try_into() {
            Ok(b) => b,
            Err(_) => return false,
        };
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        vk.verify(&domain_hash, &sig).is_ok()
    }
}

fn sha256_domain_card(hash: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(8 + 32);
    data.extend_from_slice(DOMAIN_AGENT_CARD);
    data.extend_from_slice(hash);
    sha256(&data).to_vec()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_keyring() -> Keyring {
        Keyring::generate()
    }

    #[test]
    fn create_card_basic() {
        let kr = make_keyring();
        let card =
            AgentCard::new("test-agent", &kr, vec!["inference".into()], HashMap::new()).unwrap();
        assert_eq!(card.name, "test-agent");
        assert_eq!(card.version, 1);
        assert_eq!(card.lifecycle, AgentLifecycle::Created);
        assert_eq!(card.capabilities, vec!["inference"]);
        assert!(card.did.is_neunode());
        assert!(card.peer_id.as_str().starts_with("12D3Koo"));
    }

    #[test]
    fn card_sign_verify_roundtrip() {
        let kr = make_keyring();
        let card = AgentCard::new("sign-test", &kr, vec![], HashMap::new()).unwrap();
        let signed = card.sign(&kr);
        assert!(signed.verify());
    }

    #[test]
    fn card_tampered_fails_verification() {
        let kr = make_keyring();
        let card = AgentCard::new("tamper-test", &kr, vec![], HashMap::new()).unwrap();
        let mut signed = card.sign(&kr);
        signed.card.name = "tampered-name".to_string();
        assert!(!signed.verify());
    }

    #[test]
    fn card_wrong_key_fails_verification() {
        let kr1 = make_keyring();
        let kr2 = make_keyring();
        let card = AgentCard::new("wrong-key", &kr1, vec![], HashMap::new()).unwrap();
        let signed = card.sign(&kr2);
        assert!(!signed.verify());
    }

    #[test]
    fn card_cid_is_deterministic() {
        let kr = make_keyring();
        let card = AgentCard::new("cid-test", &kr, vec![], HashMap::new()).unwrap();
        let cid1 = card.to_cid();
        let cid2 = card.to_cid();
        assert_eq!(cid1, cid2);
    }

    #[test]
    fn card_cid_format() {
        let kr = make_keyring();
        let card = AgentCard::new("cid-fmt", &kr, vec![], HashMap::new()).unwrap();
        let cid = card.to_cid();
        assert!(cid.as_str().starts_with('b'));
        assert!(cid.as_str().len() > 50);
    }

    #[test]
    fn card_different_content_different_cids() {
        let kr = make_keyring();
        let c1 = AgentCard::new("agent-a", &kr, vec![], HashMap::new()).unwrap();
        let c2 = AgentCard::new("agent-b", &kr, vec![], HashMap::new()).unwrap();
        assert_ne!(c1.to_cid(), c2.to_cid());
    }

    #[test]
    fn card_serialization_roundtrip() {
        let kr = make_keyring();
        let card =
            AgentCard::new("serde-test", &kr, vec!["training".into(), "inference".into()], {
                let mut m = HashMap::new();
                m.insert("framework".to_string(), "pytorch".to_string());
                m
            })
            .unwrap();
        let json = serde_json::to_string(&card).expect("serialize");
        let back: AgentCard = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(card.did, back.did);
        assert_eq!(card.name, back.name);
        assert_eq!(card.version, back.version);
        assert_eq!(card.capabilities, back.capabilities);
        assert_eq!(card.lifecycle, back.lifecycle);
        assert_eq!(card.metadata, back.metadata);
    }

    #[test]
    fn signed_card_serialization_roundtrip() {
        let kr = make_keyring();
        let card = AgentCard::new("signed-serde", &kr, vec![], HashMap::new()).unwrap();
        let signed = card.sign(&kr);
        let json = serde_json::to_string(&signed).expect("serialize");
        let back: SignedAgentCard = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(signed.card.did, back.card.did);
        assert_eq!(signed.signature, back.signature);
        assert_eq!(signed.signed_at, back.signed_at);
        assert!(back.verify());
    }

    #[test]
    fn builder_pattern() {
        let kr = make_keyring();
        let card = AgentCardBuilder::new("builder-test")
            .capability("inference")
            .capability("training")
            .metadata("gpu", "H100")
            .build(&kr)
            .unwrap();
        assert_eq!(card.name, "builder-test");
        assert_eq!(card.capabilities, vec!["inference", "training"]);
        assert_eq!(card.metadata.get("gpu").unwrap(), "H100");
    }

    #[test]
    fn builder_with_capabilities_vec() {
        let kr = make_keyring();
        let card = AgentCardBuilder::new("builder-vec")
            .capabilities(vec!["a".into(), "b".into(), "c".into()])
            .build(&kr)
            .unwrap();
        assert_eq!(card.capabilities.len(), 3);
    }

    #[test]
    fn card_timestamps_set() {
        let kr = make_keyring();
        let card = AgentCard::new("ts-test", &kr, vec![], HashMap::new()).unwrap();
        assert!(card.created_at > 0);
        assert_eq!(card.created_at, card.updated_at);
    }

    #[test]
    fn card_public_key_bundle_matches_keyring() {
        let kr = make_keyring();
        let card = AgentCard::new("pkb-test", &kr, vec![], HashMap::new()).unwrap();
        let bundle = kr.export_public();
        assert_eq!(card.public_key_bundle.ed25519, bundle.ed25519);
        assert_eq!(card.public_key_bundle.secp256k1, bundle.secp256k1);
        assert_eq!(card.public_key_bundle.did, bundle.did);
    }

    #[test]
    fn base32_encode_basic() {
        let data = b"f";
        let mh = multihash_sha256(data);
        let mut cid_bytes = vec![0x01u8, 0x55];
        cid_bytes.extend_from_slice(&mh);
        let encoded = format!("b{}", base32_lower_encode(&cid_bytes));
        assert!(encoded.starts_with("bafkrei"), "got: {encoded}");
    }
}
