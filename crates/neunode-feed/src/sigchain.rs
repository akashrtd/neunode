use std::collections::HashMap;

use neunode_core::constants::feed::GENESIS_PREV_HASH;
use neunode_core::kind::Kind;
use neunode_core::types::{Did, Hash256};

use crate::error::{FeedError, Result};
use crate::event::FeedEvent;

pub struct SigChain {
    author: Did,
    verifying_key: [u8; 32],
    events: Vec<FeedEvent>,
    last_hash: Hash256,
    last_sequence: u64,
    pending: HashMap<u64, FeedEvent>,
}

impl SigChain {
    pub fn new(author: Did, verifying_key: [u8; 32]) -> Self {
        Self {
            author,
            verifying_key,
            events: Vec::new(),
            last_hash: Hash256(GENESIS_PREV_HASH.to_string()),
            last_sequence: 0,
            pending: HashMap::new(),
        }
    }

    pub fn append(
        &mut self,
        kind: Kind,
        content: String,
        signing_key_bytes: &[u8; 32],
    ) -> Result<FeedEvent> {
        let sequence = if self.events.is_empty() { 0 } else { self.last_sequence + 1 };
        let prev_hash = self.last_hash.clone();

        let mut event = FeedEvent::new(kind, self.author.clone(), sequence, prev_hash, content)?;

        event.sign(signing_key_bytes)?;

        self.last_hash = event.compute_hash()?;
        self.last_sequence = sequence;
        self.events.push(event.clone());

        Ok(event)
    }

    pub fn verify_chain(&self) -> Result<()> {
        if self.events.is_empty() {
            return Ok(());
        }

        let first = &self.events[0];
        if first.sequence != 0 {
            return Err(FeedError::InvalidSequence { expected: 0, actual: first.sequence });
        }
        if first.prev_hash.0 != GENESIS_PREV_HASH {
            return Err(FeedError::HashChainBroken { seq: 0 });
        }
        if !first.verify_signature(&self.verifying_key) {
            return Err(FeedError::InvalidSignature("invalid signature at sequence 0".into()));
        }

        for i in 1..self.events.len() {
            let prev = &self.events[i - 1];
            let curr = &self.events[i];

            if curr.sequence != prev.sequence + 1 {
                return Err(FeedError::InvalidSequence {
                    expected: prev.sequence + 1,
                    actual: curr.sequence,
                });
            }

            let expected_hash = prev.compute_hash()?;
            if curr.prev_hash != expected_hash {
                return Err(FeedError::HashChainBroken { seq: curr.sequence });
            }

            if !curr.verify_signature(&self.verifying_key) {
                return Err(FeedError::InvalidSignature(format!(
                    "invalid signature at sequence {}",
                    curr.sequence
                )));
            }
        }

        // Verify cached metadata
        if let Some(last) = self.events.last() {
            if self.last_hash != last.compute_hash()? {
                return Err(FeedError::HashChainBroken { seq: last.sequence });
            }
            if self.last_sequence != last.sequence {
                return Err(FeedError::InvalidSequence {
                    expected: last.sequence,
                    actual: self.last_sequence,
                });
            }
        }

        Ok(())
    }

    pub fn get_event(&self, sequence: u64) -> Option<&FeedEvent> {
        self.events.iter().find(|e| e.sequence == sequence)
    }

    /// Receive an event that may arrive out of order. Buffers if predecessor
    /// hasn't arrived yet, then flushes consecutive buffered events.
    pub fn receive_event(&mut self, event: FeedEvent) -> Result<()> {
        if event.author != self.author {
            return Err(FeedError::Unauthorized("event author mismatch".into()));
        }
        if !event.verify_signature(&self.verifying_key) {
            return Err(FeedError::InvalidSignature(format!(
                "invalid signature at sequence {}",
                event.sequence
            )));
        }

        let expected_seq = if self.events.is_empty() { 0 } else { self.last_sequence + 1 };

        if event.sequence == expected_seq {
            // Verify prev_hash links correctly
            let expected_prev = if event.sequence == 0 {
                Hash256(GENESIS_PREV_HASH.to_string())
            } else {
                self.last_hash.clone()
            };
            if event.prev_hash != expected_prev {
                return Err(FeedError::HashChainBroken { seq: event.sequence });
            }

            self.events.push(event.clone());
            self.last_hash = event.compute_hash()?;
            self.last_sequence = event.sequence;

            // Flush any buffered consecutive events
            self.flush_pending()?;
        } else if event.sequence > expected_seq {
            // Buffer for later — predecessor hasn't arrived yet
            self.pending.insert(event.sequence, event);
        }
        // Silently ignore duplicates (sequence < expected)

        Ok(())
    }

    /// Flush consecutive buffered events after a successful append.
    fn flush_pending(&mut self) -> Result<()> {
        loop {
            let next_seq = self.last_sequence + 1;
            if let Some(event) = self.pending.remove(&next_seq) {
                if event.prev_hash != self.last_hash {
                    return Err(FeedError::HashChainBroken { seq: event.sequence });
                }
                self.events.push(event.clone());
                self.last_hash = event.compute_hash()?;
                self.last_sequence = event.sequence;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Number of events waiting in the buffer for their predecessor.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn events(&self) -> &[FeedEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_keypair() -> ([u8; 32], [u8; 32]) {
        let seed = [99u8; 32];
        let (sk, vk) = neunode_crypto::ed25519::keypair_from_seed(&seed);
        (
            neunode_crypto::ed25519::signing_key_to_bytes(&sk),
            neunode_crypto::ed25519::verifying_key_to_bytes(&vk),
        )
    }

    fn test_did() -> Did {
        Did("did:neunode:sigchain_test_agent".to_string())
    }

    #[test]
    fn new_chain_is_empty() {
        let (_sk_bytes, vk_bytes) = test_keypair();
        let chain = SigChain::new(test_did(), vk_bytes);
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn append_first_event() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);
        let event = chain
            .append(Kind::AgentMetadata, "first".to_string(), &sk_bytes)
            .expect("append should succeed");

        assert_eq!(event.sequence, 0);
        assert_eq!(event.prev_hash, Hash256(GENESIS_PREV_HASH.to_string()));
        assert_eq!(chain.len(), 1);
        assert!(!chain.is_empty());
    }

    #[test]
    fn append_multiple_events() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        let e0 = chain.append(Kind::AgentMetadata, "first".to_string(), &sk_bytes).expect("ok");
        let e1 = chain.append(Kind::BountyPost, "second".to_string(), &sk_bytes).expect("ok");
        let e2 = chain.append(Kind::Attest, "third".to_string(), &sk_bytes).expect("ok");

        assert_eq!(e0.sequence, 0);
        assert_eq!(e1.sequence, 1);
        assert_eq!(e2.sequence, 2);
        assert_eq!(chain.len(), 3);
    }

    #[test]
    fn append_links_prev_hash() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        let e0 = chain.append(Kind::AgentMetadata, "first".to_string(), &sk_bytes).expect("ok");
        let e1 = chain.append(Kind::BountyPost, "second".to_string(), &sk_bytes).expect("ok");

        assert_eq!(e1.prev_hash, e0.compute_hash().unwrap());
    }

    #[test]
    fn append_auto_increments_sequence() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        for i in 0..5 {
            let event =
                chain.append(Kind::BountyPost, format!("event {}", i), &sk_bytes).expect("ok");
            assert_eq!(event.sequence, i as u64);
        }
        assert_eq!(chain.len(), 5);
    }

    #[test]
    fn append_signs_event() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        let event = chain.append(Kind::BountyPost, "signed".to_string(), &sk_bytes).expect("ok");
        assert!(event.signature.is_some());
        assert!(event.verify_signature(&vk_bytes));
    }

    #[test]
    fn append_returns_cloned_event() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        let event = chain.append(Kind::BountyPost, "test".to_string(), &sk_bytes).expect("ok");
        assert_eq!(event.id, chain.get_event(0).expect("exists").id);
    }

    #[test]
    fn verify_chain_valid() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);
        chain.append(Kind::AgentMetadata, "first".to_string(), &sk_bytes).expect("ok");
        chain.append(Kind::BountyPost, "second".to_string(), &sk_bytes).expect("ok");
        chain.append(Kind::Attest, "third".to_string(), &sk_bytes).expect("ok");

        assert!(chain.verify_chain().is_ok());
    }

    #[test]
    fn verify_empty_chain() {
        let (_, vk_bytes) = test_keypair();
        let chain = SigChain::new(test_did(), vk_bytes);
        assert!(chain.verify_chain().is_ok());
    }

    #[test]
    fn verify_single_event_chain() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);
        chain.append(Kind::AgentMetadata, "solo".to_string(), &sk_bytes).expect("ok");
        assert!(chain.verify_chain().is_ok());
    }

    #[test]
    fn verify_chain_detects_wrong_prev_hash() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        let _e0 = chain.append(Kind::AgentMetadata, "first".to_string(), &sk_bytes).expect("ok");
        chain.append(Kind::BountyPost, "second".to_string(), &sk_bytes).expect("ok");

        // Tamper with prev_hash of second event
        chain.events[1].prev_hash = Hash256("tampered".to_string());

        let result = chain.verify_chain();
        assert!(result.is_err());
        match result.unwrap_err() {
            FeedError::HashChainBroken { seq } => assert_eq!(seq, 1),
            other => panic!("expected HashChainBroken, got {:?}", other),
        }
    }

    #[test]
    fn verify_chain_detects_wrong_sequence() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        chain.append(Kind::AgentMetadata, "first".to_string(), &sk_bytes).expect("ok");
        chain.append(Kind::BountyPost, "second".to_string(), &sk_bytes).expect("ok");

        // Tamper with sequence number
        chain.events[1].sequence = 99;

        let result = chain.verify_chain();
        assert!(result.is_err());
        match result.unwrap_err() {
            FeedError::InvalidSequence { expected, actual } => {
                assert_eq!(expected, 1);
                assert_eq!(actual, 99);
            }
            other => panic!("expected InvalidSequence, got {:?}", other),
        }
    }

    #[test]
    fn verify_chain_detects_tampered_content() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        chain.append(Kind::AgentMetadata, "original".to_string(), &sk_bytes).expect("ok");

        // Tamper with content (breaks signature)
        chain.events[0].content = "tampered".to_string();

        let result = chain.verify_chain();
        assert!(result.is_err());
        match result.unwrap_err() {
            FeedError::InvalidSignature(msg) => {
                assert!(msg.contains("sequence 0"));
            }
            other => panic!("expected InvalidSignature, got {:?}", other),
        }
    }

    #[test]
    fn verify_chain_detects_tampered_kind() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        chain.append(Kind::AgentMetadata, "data".to_string(), &sk_bytes).expect("ok");

        chain.events[0].kind = Kind::BountyPost;

        assert!(chain.verify_chain().is_err());
    }

    #[test]
    fn verify_chain_detects_genesis_hash_tamper() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        chain.append(Kind::AgentMetadata, "first".to_string(), &sk_bytes).expect("ok");

        chain.events[0].prev_hash = Hash256("not_genesis".to_string());

        let result = chain.verify_chain();
        assert!(result.is_err());
        match result.unwrap_err() {
            FeedError::HashChainBroken { seq } => assert_eq!(seq, 0),
            other => panic!("expected HashChainBroken, got {:?}", other),
        }
    }

    #[test]
    fn get_event_by_sequence() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);
        chain.append(Kind::AgentMetadata, "first".to_string(), &sk_bytes).expect("ok");
        chain.append(Kind::BountyPost, "second".to_string(), &sk_bytes).expect("ok");

        let e0 = chain.get_event(0).expect("should exist");
        assert_eq!(e0.content, "first");

        let e1 = chain.get_event(1).expect("should exist");
        assert_eq!(e1.content, "second");

        assert!(chain.get_event(2).is_none());
    }

    #[test]
    fn events_iterator() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);
        chain.append(Kind::AgentMetadata, "a".to_string(), &sk_bytes).expect("ok");
        chain.append(Kind::BountyPost, "b".to_string(), &sk_bytes).expect("ok");

        let contents: Vec<&str> = chain.events().iter().map(|e| e.content.as_str()).collect();
        assert_eq!(contents, vec!["a", "b"]);
    }

    #[test]
    fn last_hash_updates() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        let e0 = chain.append(Kind::AgentMetadata, "first".to_string(), &sk_bytes).expect("ok");
        let hash0 = e0.compute_hash().unwrap();

        let e1 = chain.append(Kind::BountyPost, "second".to_string(), &sk_bytes).expect("ok");

        assert_eq!(e1.prev_hash, hash0);
    }

    #[test]
    fn append_wrong_signing_key_still_verifies_false() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let wrong_seed = [77u8; 32];
        let (wrong_sk, _) = neunode_crypto::ed25519::keypair_from_seed(&wrong_seed);
        let wrong_sk_bytes = neunode_crypto::ed25519::signing_key_to_bytes(&wrong_sk);

        let mut chain = SigChain::new(test_did(), vk_bytes);
        chain.append(Kind::AgentMetadata, "first".to_string(), &sk_bytes).expect("ok");
        chain
            .append(Kind::BountyPost, "signed_with_wrong_key".to_string(), &wrong_sk_bytes)
            .expect("ok");

        let result = chain.verify_chain();
        assert!(result.is_err());
    }

    #[test]
    fn chain_with_many_events() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        for i in 0..20 {
            chain.append(Kind::BountyPost, format!("event {}", i), &sk_bytes).expect("ok");
        }

        assert_eq!(chain.len(), 20);
        assert!(chain.verify_chain().is_ok());
    }

    #[test]
    fn verify_chain_detects_wrong_author() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        chain.append(Kind::AgentMetadata, "first".to_string(), &sk_bytes).expect("ok");

        // Change author — signature will fail because canonical bytes include author
        chain.events[0].author = Did("did:neunode:impostor".to_string());

        assert!(chain.verify_chain().is_err());
    }

    // ─── Out-of-order receive_event ─────────────────────────────────────────

    #[test]
    fn receive_event_in_order() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        let e0 = chain.append(Kind::AgentMetadata, "first".to_string(), &sk_bytes).unwrap();
        let e1 = chain.append(Kind::BountyPost, "second".to_string(), &sk_bytes).unwrap();

        // New chain receives events in order
        let mut chain2 = SigChain::new(test_did(), vk_bytes);
        chain2.receive_event(e0).unwrap();
        chain2.receive_event(e1).unwrap();
        assert_eq!(chain2.len(), 2);
        assert!(chain2.verify_chain().is_ok());
    }

    #[test]
    fn receive_event_out_of_order_buffers_then_flushes() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain1 = SigChain::new(test_did(), vk_bytes);

        let e0 = chain1.append(Kind::AgentMetadata, "first".to_string(), &sk_bytes).unwrap();
        let e1 = chain1.append(Kind::BountyPost, "second".to_string(), &sk_bytes).unwrap();
        let e2 = chain1.append(Kind::Attest, "third".to_string(), &sk_bytes).unwrap();

        // New chain receives events 2, 0, 1
        let mut chain2 = SigChain::new(test_did(), vk_bytes);
        chain2.receive_event(e2.clone()).unwrap(); // buffered
        assert_eq!(chain2.len(), 0);
        assert_eq!(chain2.pending_count(), 1);

        chain2.receive_event(e0.clone()).unwrap(); // accepted, flushes nothing
        assert_eq!(chain2.len(), 1);
        assert_eq!(chain2.pending_count(), 1);

        chain2.receive_event(e1.clone()).unwrap(); // accepted, flushes e2
        assert_eq!(chain2.len(), 3);
        assert_eq!(chain2.pending_count(), 0);

        assert!(chain2.verify_chain().is_ok());
    }

    #[test]
    fn receive_event_rejects_wrong_signature() {
        let (sk_bytes, vk_bytes) = test_keypair();
        let mut chain = SigChain::new(test_did(), vk_bytes);

        let mut event =
            FeedEvent::new(Kind::AgentMetadata, test_did(), 0, Hash256(GENESIS_PREV_HASH.to_string()), "bad".to_string()).unwrap();
        // Sign with wrong key
        let wrong_seed = [77u8; 32];
        let (wrong_sk, _) = neunode_crypto::ed25519::keypair_from_seed(&wrong_seed);
        let wrong_bytes = neunode_crypto::ed25519::signing_key_to_bytes(&wrong_sk);
        event.sign(&wrong_bytes).unwrap();

        let result = chain.receive_event(event);
        assert!(result.is_err());
    }
}
