use anyhow::Result;
use neunode_feed::event::FeedEvent;
use neunode_storage::feed_store::StoredEvent;

pub fn serialize_feed_event(event: &FeedEvent) -> Result<Vec<u8>> {
    serde_json::to_vec(event).map_err(|e| anyhow::anyhow!("feed event serialization failed: {e}"))
}

pub fn deserialize_feed_event(data: &[u8]) -> Result<FeedEvent> {
    serde_json::from_slice(data)
        .map_err(|e| anyhow::anyhow!("feed event deserialization failed: {e}"))
}

pub fn feed_event_to_stored(event: &FeedEvent) -> StoredEvent {
    StoredEvent {
        kind: event.kind.as_u16(),
        timestamp: event.timestamp,
        agent_did: event.author.0.clone(),
        sequence: event.sequence,
        prev_hash: event.prev_hash.0.as_bytes().to_vec(),
        payload: event.content.as_bytes().to_vec(),
        signature: event.signature.as_ref().map(|s| s.0.as_bytes().to_vec()).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neunode_core::kind::Kind;
    use neunode_core::types::{Did, Hash256};

    fn test_event() -> FeedEvent {
        FeedEvent::new(
            Kind::BountyPost,
            Did("did:neunode:test_agent_001".to_string()),
            0,
            Hash256("0".to_string()),
            "test content for wire".to_string(),
        )
        .expect("event creation should succeed")
    }

    #[test]
    fn roundtrip_serialization() {
        let event = test_event();
        let bytes = serialize_feed_event(&event).unwrap();
        let back = deserialize_feed_event(&bytes).unwrap();
        assert_eq!(event.id, back.id);
        assert_eq!(event.kind, back.kind);
        assert_eq!(event.author, back.author);
        assert_eq!(event.content, back.content);
        assert_eq!(event.sequence, back.sequence);
    }

    #[test]
    fn deserialize_invalid_json_fails() {
        assert!(deserialize_feed_event(b"not json").is_err());
    }

    #[test]
    fn deserialize_empty_fails() {
        assert!(deserialize_feed_event(b"").is_err());
    }

    #[test]
    fn to_stored_preserves_fields() {
        let event = test_event();
        let stored = feed_event_to_stored(&event);
        assert_eq!(stored.kind, event.kind.as_u16());
        assert_eq!(stored.agent_did, event.author.0);
        assert_eq!(stored.sequence, event.sequence);
    }
}
