// ---------------------------------------------------------------------------
// Column Family Definitions
// ---------------------------------------------------------------------------
// 21 column families organized by subsystem.
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
pub const CF_UNBONDING: &str = "unbonding";
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

/// All 21 column family names in canonical order.
pub fn all_column_families() -> Vec<&'static str> {
    let mut all = ledger_column_families();
    all.extend(network_column_families());
    all.extend(graph_column_families());
    all
}

pub fn ledger_column_families() -> Vec<&'static str> {
    vec![
        CF_IDENTITY,
        CF_CONFIG,
        CF_TOKENS,
        CF_REPUTATION,
        CF_MODELS,
        CF_TRAINING,
        CF_BOUNTIES,
        CF_UNBONDING,
    ]
}

pub fn network_column_families() -> Vec<&'static str> {
    vec![CF_P2P_STATE, CF_MERKLE_NODES, CF_SNAPSHOTS]
}

pub fn graph_column_families() -> Vec<&'static str> {
    vec![
        CF_FEED_EVENTS,
        CF_FEED_INDEX,
        CF_FEED_STATE,
        CF_KG_ID2STR,
        CF_KG_SPOG,
        CF_KG_POSG,
        CF_KG_OSPG,
        CF_KG_GSPO,
        CF_KG_GPOS,
        CF_KG_GOSP,
    ]
}

/// Partition tag for routing CFs to the correct RocksDB instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Partition {
    Ledger,
    Network,
    Graph,
}

/// Build a routing map from CF name to partition, built once at DB open.
pub fn build_partition_map() -> std::collections::HashMap<&'static str, Partition> {
    let mut map = std::collections::HashMap::with_capacity(20);
    for &cf in &ledger_column_families() {
        map.insert(cf, Partition::Ledger);
    }
    for &cf in &network_column_families() {
        map.insert(cf, Partition::Network);
    }
    for &cf in &graph_column_families() {
        map.insert(cf, Partition::Graph);
    }
    map
}

/// Deterministic 16-byte hash of a DID string.
///
/// Uses BLAKE3 truncated to 16 bytes. Collision-resistant and suitable as a
/// key prefix for all per-agent data (feed events, tokens, reputation, etc.).
///
/// # Migration Note
///
/// This replaces the previous `DefaultHasher` (SipHash 1-3) implementation
/// which was not collision-resistant. Any existing data written with the old
/// hash will need re-keying. Since this is a pre-production hotfix, no
/// migration path is provided.
pub fn did_hash_16(did: &str) -> [u8; 16] {
    let hash = neunode_crypto::hash::blake3_hash(did.as_bytes());
    let mut out = [0u8; 16];
    out.copy_from_slice(&hash[..16]);
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
        assert_eq!(cfs.len(), 21, "expected exactly 21 column families");
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
    fn test_did_hash_uses_blake3() {
        // Verify the output matches blake3 truncated to 16 bytes
        let did = "did:neunode:0xABC";
        let expected = &neunode_crypto::hash::blake3_hash(did.as_bytes())[..16];
        let actual = did_hash_16(did);
        assert_eq!(actual.as_slice(), expected);
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
