use serde::{Deserialize, Serialize};

/// Catchup request broadcast when a node reconnects or detects sequence gaps.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CatchupRequest {
    /// The DID of the agent whose events we need.
    pub author_did: String,
    /// The first missing sequence number.
    pub from_sequence: u64,
    /// The last missing sequence number (inclusive). None means "up to latest".
    pub to_sequence: Option<u64>,
}

/// Catchup response sent by a peer that has the requested events.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CatchupResponse {
    /// The DID of the agent whose events are included.
    pub author_did: String,
    /// Serialized feed events in sequence order.
    pub events: Vec<Vec<u8>>,
    /// The sequence range covered.
    pub from_sequence: u64,
    pub to_sequence: u64,
}

/// Gossipsub topic for catchup protocol.
pub const CATCHUP_TOPIC: &str = "neunode/catchup";

impl CatchupRequest {
    pub fn new(author_did: String, from_sequence: u64) -> Self {
        Self { author_did, from_sequence, to_sequence: None }
    }

    pub fn with_to_sequence(mut self, to: u64) -> Self {
        self.to_sequence = Some(to);
        self
    }

    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        serde_json::from_slice(data).ok()
    }
}

impl CatchupResponse {
    pub fn new(author_did: String, events: Vec<Vec<u8>>, from: u64, to: u64) -> Self {
        Self { author_did, events, from_sequence: from, to_sequence: to }
    }

    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        serde_json::from_slice(data).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catchup_request_roundtrip() {
        let req = CatchupRequest::new("did:neunode:abc".to_string(), 5).with_to_sequence(10);
        let bytes = req.serialize();
        let back = CatchupRequest::deserialize(&bytes).unwrap();
        assert_eq!(req, back);
    }

    #[test]
    fn catchup_request_no_to_sequence() {
        let req = CatchupRequest::new("did:neunode:abc".to_string(), 0);
        assert!(req.to_sequence.is_none());
        let bytes = req.serialize();
        let back = CatchupRequest::deserialize(&bytes).unwrap();
        assert_eq!(back.to_sequence, None);
    }

    #[test]
    fn catchup_response_roundtrip() {
        let resp =
            CatchupResponse::new("did:neunode:abc".to_string(), vec![b"event1".to_vec()], 0, 0);
        let bytes = resp.serialize();
        let back = CatchupResponse::deserialize(&bytes).unwrap();
        assert_eq!(resp, back);
    }

    #[test]
    fn catchup_request_deserialize_garbage_returns_none() {
        assert!(CatchupRequest::deserialize(b"garbage").is_none());
    }

    #[test]
    fn catchup_response_deserialize_garbage_returns_none() {
        assert!(CatchupResponse::deserialize(b"garbage").is_none());
    }
}
