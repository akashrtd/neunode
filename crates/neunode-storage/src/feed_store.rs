use crate::cf;
use crate::db::NeunodeDb;
use crate::error::{Result, StorageError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StoredEvent {
    pub kind: u16,
    pub timestamp: u64,
    pub agent_did: String,
    pub sequence: u64,
    pub prev_hash: Vec<u8>,
    pub payload: Vec<u8>,
    pub signature: Vec<u8>,
}

pub struct FeedStore<'a> {
    db: &'a NeunodeDb,
}

impl<'a> FeedStore<'a> {
    pub fn new(db: &'a NeunodeDb) -> FeedStore<'a> {
        FeedStore { db }
    }

    pub fn append(&self, event: &StoredEvent) -> Result<()> {
        let hash = cf::did_hash_16(&event.agent_did);
        let key = cf::feed_event_key(&hash, event.sequence);
        let value = crate::codec::serialize(event)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        self.db.put_raw(cf::CF_FEED_EVENTS, &key, &value)
    }

    pub fn get(&self, agent_did: &str, sequence: u64) -> Result<Option<StoredEvent>> {
        let hash = cf::did_hash_16(agent_did);
        let key = cf::feed_event_key(&hash, sequence);
        match self.db.get_raw(cf::CF_FEED_EVENTS, &key)? {
            Some(bytes) => {
                let event: StoredEvent = crate::codec::deserialize(&bytes)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(event))
            }
            None => Ok(None),
        }
    }

    pub fn latest_sequence(&self, agent_did: &str) -> Result<u64> {
        let hash = cf::did_hash_16(agent_did);
        let prefix = cf::agent_did_hash_prefix(&hash);
        let entries = self.db.prefix_scan(cf::CF_FEED_EVENTS, &prefix)?;
        if entries.is_empty() {
            return Ok(0);
        }
        Ok(entries
            .iter()
            .filter_map(|(k, _)| {
                if k.len() == 24 {
                    let mut seq_bytes = [0u8; 8];
                    seq_bytes.copy_from_slice(&k[16..24]);
                    Some(u64::from_be_bytes(seq_bytes))
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0))
    }

    pub fn get_range(&self, agent_did: &str, from: u64, limit: usize) -> Result<Vec<StoredEvent>> {
        let hash = cf::did_hash_16(agent_did);
        let start = cf::feed_event_key(&hash, from);
        let end = cf::feed_event_key(&hash, u64::MAX);

        let entries = self.db.range_scan(cf::CF_FEED_EVENTS, &start, &end)?;
        entries
            .iter()
            .take(limit)
            .map(|(_, v)| {
                crate::codec::deserialize(v).map_err(|e| StorageError::Serialization(e.to_string()))
            })
            .collect()
    }

    pub fn get_all(&self, agent_did: &str) -> Result<Vec<StoredEvent>> {
        let hash = cf::did_hash_16(agent_did);
        let prefix = cf::agent_did_hash_prefix(&hash);
        let entries = self.db.prefix_scan(cf::CF_FEED_EVENTS, &prefix)?;
        entries
            .iter()
            .map(|(_, v)| {
                crate::codec::deserialize(v).map_err(|e| StorageError::Serialization(e.to_string()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NeunodeDb;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_db() -> NeunodeDb {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "neunode_storage_feed_{:?}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&dir);
        NeunodeDb::open(&dir).unwrap()
    }

    fn make_event(did: &str, seq: u64, kind: u16) -> StoredEvent {
        StoredEvent {
            kind,
            timestamp: 1700000000 + seq,
            agent_did: did.to_string(),
            sequence: seq,
            prev_hash: if seq == 0 { vec![0; 32] } else { vec![seq as u8; 32] },
            payload: format!(r#"{{"action":"event_{seq}"}}"#).into_bytes(),
            signature: vec![0xAA; 64],
        }
    }

    #[test]
    fn test_append_and_get() {
        let db = temp_db();
        let store = FeedStore::new(&db);
        let event = make_event("did:neunode:alice", 1, 1000);

        store.append(&event).unwrap();

        let fetched = store.get("did:neunode:alice", 1).unwrap();
        assert_eq!(fetched, Some(event));
    }

    #[test]
    fn test_get_missing() {
        let db = temp_db();
        let store = FeedStore::new(&db);
        assert!(store.get("did:neunode:nobody", 1).unwrap().is_none());
    }

    #[test]
    fn test_latest_sequence_empty() {
        let db = temp_db();
        let store = FeedStore::new(&db);
        assert_eq!(store.latest_sequence("did:neunode:empty").unwrap(), 0);
    }

    #[test]
    fn test_latest_sequence_multiple_events() {
        let db = temp_db();
        let store = FeedStore::new(&db);

        for seq in 1..=5 {
            store.append(&make_event("did:neunode:bob", seq, 1000)).unwrap();
        }

        assert_eq!(store.latest_sequence("did:neunode:bob").unwrap(), 5);
    }

    #[test]
    fn test_get_range() {
        let db = temp_db();
        let store = FeedStore::new(&db);

        for seq in 1..=10 {
            store.append(&make_event("did:neunode:carol", seq, 2000)).unwrap();
        }

        let range = store.get_range("did:neunode:carol", 3, 4).unwrap();
        assert_eq!(range.len(), 4);
        assert_eq!(range[0].sequence, 3);
        assert_eq!(range[3].sequence, 6);
    }

    #[test]
    fn test_get_range_limit_exceeds_available() {
        let db = temp_db();
        let store = FeedStore::new(&db);

        for seq in 1..=3 {
            store.append(&make_event("did:neunode:dave", seq, 1000)).unwrap();
        }

        let range = store.get_range("did:neunode:dave", 1, 100).unwrap();
        assert_eq!(range.len(), 3);
    }

    #[test]
    fn test_get_all() {
        let db = temp_db();
        let store = FeedStore::new(&db);

        for seq in 1..=5 {
            store.append(&make_event("did:neunode:eve", seq, 3000)).unwrap();
        }

        let all = store.get_all("did:neunode:eve").unwrap();
        assert_eq!(all.len(), 5);
        for (i, evt) in all.iter().enumerate() {
            assert_eq!(evt.sequence, (i + 1) as u64);
        }
    }

    #[test]
    fn test_agents_isolated() {
        let db = temp_db();
        let store = FeedStore::new(&db);

        store.append(&make_event("did:neunode:agent_a", 1, 1000)).unwrap();
        store.append(&make_event("did:neunode:agent_a", 2, 1000)).unwrap();
        store.append(&make_event("did:neunode:agent_b", 1, 2000)).unwrap();

        assert_eq!(store.latest_sequence("did:neunode:agent_a").unwrap(), 2);
        assert_eq!(store.latest_sequence("did:neunode:agent_b").unwrap(), 1);
        assert_eq!(store.get_all("did:neunode:agent_a").unwrap().len(), 2);
        assert_eq!(store.get_all("did:neunode:agent_b").unwrap().len(), 1);
    }

    #[test]
    fn test_event_roundtrip_preserves_all_fields() {
        let db = temp_db();
        let store = FeedStore::new(&db);
        let event = StoredEvent {
            kind: 9999,
            timestamp: 1700000123,
            agent_did: "did:neunode:precise".to_string(),
            sequence: 42,
            prev_hash: vec![0xDE; 32],
            payload: b"{\"complex\":\"payload\"}".to_vec(),
            signature: vec![0xBB; 64],
        };

        store.append(&event).unwrap();
        let fetched = store.get("did:neunode:precise", 42).unwrap().unwrap();
        assert_eq!(fetched, event);
    }
}
