use serde::{Deserialize, Serialize};

use crate::cf::CF_P2P_STATE;
use crate::db::NeunodeDb;
use crate::error::{Result, StorageError};

const PEER_ADDRESS_PREFIX: &[u8] = b"peer_address:";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerAddressRecord {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub updated_at: u64,
}

pub struct PeerAddressStore<'a> {
    db: &'a NeunodeDb,
}

impl<'a> PeerAddressStore<'a> {
    pub fn new(db: &'a NeunodeDb) -> Self {
        Self { db }
    }

    pub fn put(&self, record: &PeerAddressRecord) -> Result<()> {
        if record.peer_id.is_empty() {
            return Err(StorageError::InvalidKeyFormat("peer ID cannot be empty".to_string()));
        }
        let value = crate::codec::serialize(record)
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        self.db.put_raw(CF_P2P_STATE, &record_key(&record.peer_id), &value)
    }

    pub fn get(&self, peer_id: &str) -> Result<Option<PeerAddressRecord>> {
        self.db
            .get_raw(CF_P2P_STATE, &record_key(peer_id))?
            .map(|value| {
                crate::codec::deserialize(&value)
                    .map_err(|error| StorageError::Serialization(error.to_string()))
            })
            .transpose()
    }

    pub fn list(&self) -> Result<Vec<PeerAddressRecord>> {
        self.db
            .prefix_scan(CF_P2P_STATE, PEER_ADDRESS_PREFIX)?
            .into_iter()
            .map(|(_, value)| {
                crate::codec::deserialize(&value)
                    .map_err(|error| StorageError::Serialization(error.to_string()))
            })
            .collect()
    }
}

fn record_key(peer_id: &str) -> Vec<u8> {
    let mut key = Vec::with_capacity(PEER_ADDRESS_PREFIX.len() + peer_id.len());
    key.extend_from_slice(PEER_ADDRESS_PREFIX);
    key.extend_from_slice(peer_id.as_bytes());
    key
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_db() -> NeunodeDb {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "neunode_peer_address_store_{}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&dir);
        NeunodeDb::open(&dir).unwrap()
    }

    #[test]
    fn persists_and_lists_peer_addresses() {
        let db = temp_db();
        let store = PeerAddressStore::new(&db);
        let record = PeerAddressRecord {
            peer_id: "peer-a".to_string(),
            addresses: vec!["/ip4/127.0.0.1/tcp/4001/p2p/peer-a".to_string()],
            updated_at: 42,
        };
        store.put(&record).unwrap();

        assert_eq!(store.get("peer-a").unwrap(), Some(record.clone()));
        assert_eq!(store.list().unwrap(), vec![record]);
    }

    #[test]
    fn rejects_empty_peer_id() {
        let db = temp_db();
        let record =
            PeerAddressRecord { peer_id: String::new(), addresses: Vec::new(), updated_at: 0 };
        assert!(matches!(
            PeerAddressStore::new(&db).put(&record),
            Err(StorageError::InvalidKeyFormat(_))
        ));
    }
}
