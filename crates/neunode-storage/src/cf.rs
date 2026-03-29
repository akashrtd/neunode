// ---------------------------------------------------------------------------
// Column Family Definitions
// ---------------------------------------------------------------------------
// 20 column families organized by subsystem.
//
// Key schemas:
//   feed_events: [did_hash(16) | sequence(u64 BE)]
//   tokens:      [did_hash(16) | token_type(u8)]
//   identity:    raw DID string (bincode-encoded)
//   bounties:    bounty ID string (bincode-encoded)
//   KG indexes:  6-tuple encoded per Oxigraph pattern (SPOG, POSG, ...)
// ---------------------------------------------------------------------------

pub const CF_IDENTITY: &str = "identity";
pub const CF_CONFIG: &str = "config";
pub const CF_FEED_EVENTS: &str = "feed_events";
pub const CF_FEED_INDEX: &str = "feed_index";
pub const CF_FEED_STATE: &str = "feed_state";
pub const CF_TOKENS: &str = "tokens";
pub const CF_REPUTATION: &str = "reputation";
pub const CF_MODELS: &str = "models";
pub const CF_TRAINING: &str = "training";
pub const CF_BOUNTIES: &str = "bounties";
pub const CF_P2P_STATE: &str = "p2p_state";
pub const CF_MERKLE_NODES: &str = "merkle_nodes";
pub const CF_SNAPSHOTS: &str = "snapshots";
pub const CF_KG_ID2STR: &str = "kg_id2str";
pub const CF_KG_SPOG: &str = "spog";
pub const CF_KG_POSG: &str = "posg";
pub const CF_KG_OSPG: &str = "ospg";
pub const CF_KG_GSPO: &str = "gspo";
pub const CF_KG_GPOS: &str = "gpos";
pub const CF_KG_GOSP: &str = "gosp";

/// All 20 column family names in canonical order.
pub fn all_column_families() -> Vec<&'static str> {
    vec![
        CF_IDENTITY,
        CF_CONFIG,
        CF_FEED_EVENTS,
        CF_FEED_INDEX,
        CF_FEED_STATE,
        CF_TOKENS,
        CF_REPUTATION,
        CF_MODELS,
        CF_TRAINING,
        CF_BOUNTIES,
        CF_P2P_STATE,
        CF_MERKLE_NODES,
        CF_SNAPSHOTS,
        CF_KG_ID2STR,
        CF_KG_SPOG,
        CF_KG_POSG,
        CF_KG_OSPG,
        CF_KG_GSPO,
        CF_KG_GPOS,
        CF_KG_GOSP,
    ]
}

/// Deterministic 16-byte hash of a DID string.
/// Uses two rounds of std DefaultHasher (SipHash variant) for 16 bytes total.
/// Not cryptographically secure — used only as a key prefix for locality.
pub fn did_hash_16(did: &str) -> [u8; 16] {
    use std::hash::{Hash, Hasher};
    let mut h1 = std::collections::hash_map::DefaultHasher::new();
    did.hash(&mut h1);
    let v1 = h1.finish();

    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    format!("{did}\x00salt").hash(&mut h2);
    let v2 = h2.finish();

    let mut out = [0u8; 16];
    out[..8].copy_from_slice(&v1.to_be_bytes());
    out[8..].copy_from_slice(&v2.to_be_bytes());
    out
}

/// Build a feed-event key: `[did_hash(16) | sequence(u64 big-endian)]`.
pub fn feed_event_key(agent_did_hash: &[u8; 16], sequence: u64) -> Vec<u8> {
    let mut key = Vec::with_capacity(24);
    key.extend_from_slice(agent_did_hash);
    key.extend_from_slice(&sequence.to_be_bytes());
    key
}

/// Build a token-balance key: `[did_hash(16) | token_type(u8)]`.
pub fn token_key(agent_did_hash: &[u8; 16], token_type: u8) -> Vec<u8> {
    let mut key = Vec::with_capacity(17);
    key.extend_from_slice(agent_did_hash);
    key.push(token_type);
    key
}

/// Return the 16-byte DID hash prefix (for prefix scans).
pub fn agent_did_hash_prefix(agent_did_hash: &[u8; 16]) -> Vec<u8> {
    agent_did_hash.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_cfs_count() {
        let cfs = all_column_families();
        assert_eq!(cfs.len(), 20, "expected exactly 20 column families");
    }

    #[test]
    fn test_all_cfs_unique() {
        let cfs = all_column_families();
        let mut seen = std::collections::HashSet::new();
        for cf in &cfs {
            assert!(seen.insert(cf), "duplicate CF name: {cf}");
        }
    }

    #[test]
    fn test_did_hash_deterministic() {
        let a = did_hash_16("did:neunode:0xABC");
        let b = did_hash_16("did:neunode:0xABC");
        assert_eq!(a, b, "same DID must produce same hash");
    }

    #[test]
    fn test_did_hash_different_inputs() {
        let a = did_hash_16("did:neunode:0xABC");
        let b = did_hash_16("did:neunode:0xDEF");
        assert_ne!(a, b, "different DIDs must produce different hashes");
    }

    #[test]
    fn test_did_hash_length() {
        let h = did_hash_16("did:neunode:test");
        assert_eq!(h.len(), 16);
    }

    #[test]
    fn test_feed_event_key_layout() {
        let hash = [0xAB_u8; 16];
        let key = feed_event_key(&hash, 42);
        assert_eq!(key.len(), 24);
        assert_eq!(&key[..16], &hash[..]);
        let seq = u64::from_be_bytes(key[16..].try_into().unwrap());
        assert_eq!(seq, 42);
    }

    #[test]
    fn test_feed_event_key_ordering() {
        let hash = [0u8; 16];
        let k1 = feed_event_key(&hash, 1);
        let k2 = feed_event_key(&hash, 2);
        let k100 = feed_event_key(&hash, 100);
        assert!(k1 < k2);
        assert!(k2 < k100);
    }

    #[test]
    fn test_token_key_layout() {
        let hash = [0xCD_u8; 16];
        let key = token_key(&hash, 0x01);
        assert_eq!(key.len(), 17);
        assert_eq!(&key[..16], &hash[..]);
        assert_eq!(key[16], 0x01);
    }

    #[test]
    fn test_agent_did_hash_prefix() {
        let hash = [0xEF_u8; 16];
        let prefix = agent_did_hash_prefix(&hash);
        assert_eq!(prefix.len(), 16);
        assert_eq!(&prefix[..], &hash[..]);
    }
}
