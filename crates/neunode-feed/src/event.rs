use serde::{Deserialize, Serialize};

use neunode_core::constants::feed::{
    MAX_CONTENT_SIZE, MAX_REFS, MAX_TAGS, MAX_TAG_KEY_LEN, MAX_TAG_VALUE_LEN,
};
use neunode_core::kind::Kind;
use neunode_core::types::{Did, EventId, Hash256, Sequence, Signature, Timestamp};

use crate::error::{FeedError, Result};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub struct EventTag {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub struct EventRef {
    pub event_id: EventId,
    pub author: Did,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, TS)]
#[ts(export)]
pub struct FeedEvent {
    pub id: EventId,
    pub kind: Kind,
    pub author: Did,
    pub sequence: Sequence,
    pub timestamp: Timestamp,
    pub prev_hash: Hash256,
    pub content: String,
    pub tags: Vec<EventTag>,
    pub refs: Vec<EventRef>,
    pub signature: Option<Signature>,
}

#[derive(Serialize)]
struct CanonicalBody {
    author: String,
    content: String,
    kind: u16,
    prev_hash: String,
    refs: Vec<EventRef>,
    sequence: u64,
    tags: Vec<EventTag>,
    timestamp: u64,
}

impl FeedEvent {
    pub fn new(
        kind: Kind,
        author: Did,
        sequence: Sequence,
        prev_hash: Hash256,
        content: String,
    ) -> Result<Self> {
        if content.len() > MAX_CONTENT_SIZE {
            return Err(FeedError::ContentTooLarge { size: content.len(), max: MAX_CONTENT_SIZE });
        }

        let timestamp = chrono::Utc::now().timestamp() as u64;

        let mut event = Self {
            id: EventId(String::new()),
            kind,
            author,
            sequence,
            timestamp,
            prev_hash,
            content,
            tags: Vec::new(),
            refs: Vec::new(),
            signature: None,
        };

        event.id = event.compute_id();
        Ok(event)
    }

    pub fn compute_id(&self) -> EventId {
        let bytes = self.canonical_bytes();
        let hash = neunode_crypto::hash::sha256(&bytes);
        EventId(format!("f{}", to_hex(&hash)))
    }

    pub fn compute_hash(&self) -> Hash256 {
        let full = serde_json::to_string(self).expect("feed event serialization should not fail");
        let hash = neunode_crypto::hash::sha256(full.as_bytes());
        Hash256(to_hex(&hash))
    }

    pub fn sign(&mut self, signing_key_bytes: &[u8; 32]) -> Result<()> {
        let signing_key = neunode_crypto::ed25519::signing_key_from_bytes(signing_key_bytes)
            .map_err(|e| FeedError::InvalidSignature(e.to_string()))?;

        let canonical = self.canonical_bytes();
        let dalek_sig = neunode_crypto::ed25519::sign_domain(
            &signing_key,
            neunode_crypto::hash::DOMAIN_FEED_EVENT,
            &canonical,
        );

        self.signature = Some(Signature(format!("ed25519:{}", to_hex(&dalek_sig.to_bytes()))));
        Ok(())
    }

    pub fn verify_signature(&self, verifying_key_bytes: &[u8; 32]) -> bool {
        let verifying_key =
            match neunode_crypto::ed25519::verifying_key_from_bytes(verifying_key_bytes) {
                Ok(k) => k,
                Err(_) => return false,
            };

        let core_sig = match &self.signature {
            Some(s) => s,
            None => return false,
        };

        let dalek_sig = match core_sig_to_dalek(core_sig) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let canonical = self.canonical_bytes();
        neunode_crypto::ed25519::verify_domain(
            &verifying_key,
            neunode_crypto::hash::DOMAIN_FEED_EVENT,
            &canonical,
            &dalek_sig,
        )
    }

    pub fn validate(&self) -> Result<()> {
        if self.content.len() > MAX_CONTENT_SIZE {
            return Err(FeedError::ContentTooLarge {
                size: self.content.len(),
                max: MAX_CONTENT_SIZE,
            });
        }

        if self.tags.len() > MAX_TAGS {
            return Err(FeedError::TooManyTags { count: self.tags.len(), max: MAX_TAGS });
        }

        if self.refs.len() > MAX_REFS {
            return Err(FeedError::TooManyRefs { count: self.refs.len(), max: MAX_REFS });
        }

        for tag in &self.tags {
            if tag.key.len() > MAX_TAG_KEY_LEN {
                return Err(FeedError::InvalidEvent(format!(
                    "tag key too long: {} bytes, max {}",
                    tag.key.len(),
                    MAX_TAG_KEY_LEN
                )));
            }
            if tag.value.len() > MAX_TAG_VALUE_LEN {
                return Err(FeedError::InvalidEvent(format!(
                    "tag value too long: {} bytes, max {}",
                    tag.value.len(),
                    MAX_TAG_VALUE_LEN
                )));
            }
        }

        Ok(())
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let body = CanonicalBody {
            author: self.author.0.clone(),
            content: self.content.clone(),
            kind: self.kind.as_u16(),
            prev_hash: self.prev_hash.0.clone(),
            refs: self.refs.clone(),
            sequence: self.sequence,
            tags: self.tags.clone(),
            timestamp: self.timestamp,
        };
        serde_json::to_vec(&body).expect("canonical serialization of feed event should not fail")
    }
}

fn core_sig_to_dalek(
    sig: &Signature,
) -> std::result::Result<neunode_crypto::ed25519::Signature, FeedError> {
    let hex_str = sig
        .0
        .strip_prefix("ed25519:")
        .ok_or_else(|| FeedError::InvalidSignature("missing ed25519: prefix".into()))?;
    let bytes = from_hex(hex_str)?;
    let arr: [u8; 64] = bytes
        .try_into()
        .map_err(|_| FeedError::InvalidSignature("wrong signature length".into()))?;
    Ok(neunode_crypto::ed25519::Signature::from_bytes(&arr))
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn from_hex(s: &str) -> std::result::Result<Vec<u8>, FeedError> {
    if !s.len().is_multiple_of(2) {
        return Err(FeedError::InvalidSignature("odd hex length".into()));
    }
    let mut result = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16)
            .map_err(|e| FeedError::InvalidSignature(format!("invalid hex: {}", e)))?;
        result.push(byte);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_keypair() -> ([u8; 32], [u8; 32]) {
        let seed = [42u8; 32];
        let (sk, vk) = neunode_crypto::ed25519::keypair_from_seed(&seed);
        (
            neunode_crypto::ed25519::signing_key_to_bytes(&sk),
            neunode_crypto::ed25519::verifying_key_to_bytes(&vk),
        )
    }

    fn test_did() -> Did {
        Did("did:neunode:test_agent_001".to_string())
    }

    fn make_event(content: &str) -> FeedEvent {
        FeedEvent::new(
            Kind::BountyPost,
            test_did(),
            0,
            Hash256("0".to_string()),
            content.to_string(),
        )
        .expect("event creation should succeed")
    }

    #[test]
    fn create_event_success() {
        let event = make_event("hello world");
        assert_eq!(event.kind, Kind::BountyPost);
        assert_eq!(event.author, test_did());
        assert_eq!(event.sequence, 0);
        assert_eq!(event.prev_hash, Hash256("0".to_string()));
        assert_eq!(event.content, "hello world");
        assert!(event.signature.is_none());
        assert!(event.tags.is_empty());
        assert!(event.refs.is_empty());
        assert!(!event.id.0.is_empty());
        assert!(event.timestamp > 0);
    }

    #[test]
    fn create_event_content_too_large() {
        let big = "x".repeat(MAX_CONTENT_SIZE + 1);
        let result = FeedEvent::new(Kind::BountyPost, test_did(), 0, Hash256("0".to_string()), big);
        assert!(result.is_err());
        match result.unwrap_err() {
            FeedError::ContentTooLarge { size, max } => {
                assert_eq!(size, MAX_CONTENT_SIZE + 1);
                assert_eq!(max, MAX_CONTENT_SIZE);
            }
            other => panic!("expected ContentTooLarge, got {:?}", other),
        }
    }

    #[test]
    fn create_event_max_content_allowed() {
        let exact = "x".repeat(MAX_CONTENT_SIZE);
        let result =
            FeedEvent::new(Kind::BountyPost, test_did(), 0, Hash256("0".to_string()), exact);
        assert!(result.is_ok());
    }

    #[test]
    fn compute_id_deterministic() {
        let e1 = make_event("test content");
        let mut e2 = make_event("test content");
        e2.timestamp = e1.timestamp;
        e2.id = EventId(String::new());
        assert_eq!(e1.compute_id(), e2.compute_id());
    }

    #[test]
    fn compute_id_excludes_id_and_signature() {
        let mut event = make_event("test");
        let id_before = event.compute_id();
        event.id = EventId("different_id".to_string());
        assert_eq!(event.compute_id(), id_before);

        let (sk_bytes, _) = test_keypair();
        event.sign(&sk_bytes).expect("sign should succeed");
        assert_eq!(event.compute_id(), id_before);
    }

    #[test]
    fn different_content_different_id() {
        let e1 = make_event("content A");
        let e2 = make_event("content B");
        assert_ne!(e1.compute_id(), e2.compute_id());
    }

    #[test]
    fn compute_hash_deterministic() {
        let e1 = make_event("test content");
        let _e2 = make_event("test content");
        let hash1 = e1.compute_hash();
        assert_eq!(e1.compute_hash(), hash1);
    }

    #[test]
    fn sign_and_verify_success() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut event = make_event("sign me");
        assert!(event.signature.is_none());

        event.sign(&sk_bytes).expect("sign should succeed");
        assert!(event.signature.is_some());
        assert!(event.verify_signature(&vk_bytes));
    }

    #[test]
    fn verify_wrong_key_fails() {
        let (sk_bytes, _) = test_keypair();
        let other_seed = [99u8; 32];
        let (_, other_vk) = neunode_crypto::ed25519::keypair_from_seed(&other_seed);
        let other_vk_bytes = neunode_crypto::ed25519::verifying_key_to_bytes(&other_vk);

        let mut event = make_event("sign me");
        event.sign(&sk_bytes).expect("sign should succeed");
        assert!(!event.verify_signature(&other_vk_bytes));
    }

    #[test]
    fn verify_unsigned_event_fails() {
        let (_, vk_bytes) = test_keypair();
        let event = make_event("unsigned");
        assert!(!event.verify_signature(&vk_bytes));
    }

    #[test]
    fn verify_invalid_key_bytes_fails() {
        let (sk_bytes, _) = test_keypair();
        let mut event = make_event("test");
        event.sign(&sk_bytes).expect("sign should succeed");

        let bad_key = [0xFFu8; 32];
        assert!(!event.verify_signature(&bad_key));
    }

    #[test]
    fn sign_changes_hash() {
        let (sk_bytes, _) = test_keypair();
        let mut event = make_event("test");
        let hash_before = event.compute_hash();
        event.sign(&sk_bytes).expect("sign should succeed");
        let hash_after = event.compute_hash();
        assert_ne!(hash_before, hash_after);
    }

    #[test]
    fn validate_success() {
        let event = make_event("valid content");
        assert!(event.validate().is_ok());
    }

    #[test]
    fn validate_too_many_tags() {
        let mut event = make_event("test");
        event.tags = (0..=MAX_TAGS)
            .map(|i| EventTag { key: format!("k{}", i), value: format!("v{}", i) })
            .collect();
        let result = event.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            FeedError::TooManyTags { count, max } => {
                assert_eq!(count, MAX_TAGS + 1);
                assert_eq!(max, MAX_TAGS);
            }
            other => panic!("expected TooManyTags, got {:?}", other),
        }
    }

    #[test]
    fn validate_max_tags_allowed() {
        let mut event = make_event("test");
        event.tags = (0..MAX_TAGS)
            .map(|i| EventTag { key: format!("k{}", i), value: "v".to_string() })
            .collect();
        assert!(event.validate().is_ok());
    }

    #[test]
    fn validate_too_many_refs() {
        let mut event = make_event("test");
        event.refs = (0..=MAX_REFS)
            .map(|i| EventRef { event_id: EventId(format!("evt_{}", i)), author: test_did() })
            .collect();
        let result = event.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            FeedError::TooManyRefs { count, max } => {
                assert_eq!(count, MAX_REFS + 1);
                assert_eq!(max, MAX_REFS);
            }
            other => panic!("expected TooManyRefs, got {:?}", other),
        }
    }

    #[test]
    fn validate_max_refs_allowed() {
        let mut event = make_event("test");
        event.refs = (0..MAX_REFS)
            .map(|i| EventRef { event_id: EventId(format!("evt_{}", i)), author: test_did() })
            .collect();
        assert!(event.validate().is_ok());
    }

    #[test]
    fn validate_tag_key_too_long() {
        let mut event = make_event("test");
        event.tags =
            vec![EventTag { key: "k".repeat(MAX_TAG_KEY_LEN + 1), value: "v".to_string() }];
        let result = event.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            FeedError::InvalidEvent(msg) => {
                assert!(msg.contains("tag key too long"));
            }
            other => panic!("expected InvalidEvent, got {:?}", other),
        }
    }

    #[test]
    fn validate_tag_value_too_long() {
        let mut event = make_event("test");
        event.tags =
            vec![EventTag { key: "k".to_string(), value: "v".repeat(MAX_TAG_VALUE_LEN + 1) }];
        let result = event.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            FeedError::InvalidEvent(msg) => {
                assert!(msg.contains("tag value too long"));
            }
            other => panic!("expected InvalidEvent, got {:?}", other),
        }
    }

    #[test]
    fn serde_roundtrip() {
        let (sk_bytes, _) = test_keypair();
        let mut event = make_event("serde test");
        event.tags = vec![EventTag { key: "env".to_string(), value: "test".to_string() }];
        event.refs =
            vec![EventRef { event_id: EventId("ref_123".to_string()), author: test_did() }];
        event.sign(&sk_bytes).expect("sign should succeed");

        let json = serde_json::to_string(&event).expect("serialize");
        let back: FeedEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event, back);
    }

    #[test]
    fn event_tag_serde_roundtrip() {
        let tag = EventTag { key: "domain".to_string(), value: "ml-training".to_string() };
        let json = serde_json::to_string(&tag).expect("serialize");
        let back: EventTag = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(tag, back);
    }

    #[test]
    fn event_ref_serde_roundtrip() {
        let r#ref = EventRef {
            event_id: EventId("evt_abc".to_string()),
            author: Did("did:neunode:agent42".to_string()),
        };
        let json = serde_json::to_string(&r#ref).expect("serialize");
        let back: EventRef = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r#ref, back);
    }

    #[test]
    fn id_format_starts_with_f() {
        let event = make_event("test");
        assert!(event.id.0.starts_with('f'));
        assert_eq!(event.id.0.len(), 65); // "f" + 64 hex chars
    }

    #[test]
    fn hash_format_is_hex() {
        let event = make_event("test");
        let hash = event.compute_hash();
        assert_eq!(hash.0.len(), 64); // 32 bytes = 64 hex chars
        assert!(hash.0.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn signature_format_has_prefix() {
        let (sk_bytes, _) = test_keypair();
        let mut event = make_event("test");
        event.sign(&sk_bytes).expect("sign should succeed");
        let sig = event.signature.expect("should have signature");
        assert!(sig.0.starts_with("ed25519:"));
        let hex_part = &sig.0["ed25519:".len()..];
        assert_eq!(hex_part.len(), 128); // 64 bytes = 128 hex chars
    }

    #[test]
    fn genesis_prev_hash() {
        let event = make_event("genesis");
        assert_eq!(event.prev_hash, Hash256("0".to_string()));
    }

    #[test]
    fn different_kinds_different_ids() {
        let e1 = FeedEvent::new(
            Kind::BountyPost,
            test_did(),
            0,
            Hash256("0".to_string()),
            "same content".to_string(),
        )
        .expect("ok");
        let e2 = FeedEvent::new(
            Kind::BountyClaim,
            test_did(),
            0,
            Hash256("0".to_string()),
            "same content".to_string(),
        )
        .expect("ok");
        assert_ne!(e1.id, e2.id);
    }

    #[test]
    fn different_sequences_different_ids() {
        let e1 = FeedEvent::new(
            Kind::BountyPost,
            test_did(),
            0,
            Hash256("0".to_string()),
            "content".to_string(),
        )
        .expect("ok");
        let e2 = FeedEvent::new(
            Kind::BountyPost,
            test_did(),
            1,
            Hash256("0".to_string()),
            "content".to_string(),
        )
        .expect("ok");
        assert_ne!(e1.id, e2.id);
    }

    #[test]
    fn tags_and_refs_included_in_id() {
        let e1 = make_event("content");
        let mut e2 = make_event("content");
        e2.timestamp = e1.timestamp;
        e2.tags = vec![EventTag { key: "test".to_string(), value: "value".to_string() }];
        assert_ne!(e1.compute_id(), e2.compute_id());
    }

    #[test]
    fn validate_content_at_max_size() {
        let content = "x".repeat(MAX_CONTENT_SIZE);
        let event =
            FeedEvent::new(Kind::BountyPost, test_did(), 0, Hash256("0".to_string()), content)
                .expect("should succeed at max size");
        assert!(event.validate().is_ok());
    }
}
