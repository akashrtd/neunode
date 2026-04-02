use crate::cf;
use crate::db::NeunodeDb;
use crate::error::{Result, StorageError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BountyData {
    pub id: String,
    pub state: String,
    pub requester_did: String,
    pub provider_did: Option<String>,
    pub reward_amount: u64,
    pub reward_token_type: u8,
    pub deadline: u64,
    pub created_at: u64,
    pub escrow_deposited: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub claim_deadline: u64,
    #[serde(default)]
    pub work_deadline: u64,
    #[serde(default)]
    pub review_deadline: u64,
    #[serde(default)]
    pub artifact_hash: Option<String>,
    #[serde(default)]
    pub bond: Option<u64>,
}

pub struct BountyStore<'a> {
    db: &'a NeunodeDb,
}

impl<'a> BountyStore<'a> {
    pub fn new(db: &'a NeunodeDb) -> BountyStore<'a> {
        BountyStore { db }
    }

    pub fn put(&self, bounty: &BountyData) -> Result<()> {
        let key_bytes = bincode::serialize(&bounty.id)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let value_bytes =
            bincode::serialize(bounty).map_err(|e| StorageError::Serialization(e.to_string()))?;
        self.db.put_raw(cf::CF_BOUNTIES, &key_bytes, &value_bytes)
    }

    pub fn get(&self, bounty_id: &str) -> Result<Option<BountyData>> {
        let key_bytes = bincode::serialize(bounty_id)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        match self.db.get_raw(cf::CF_BOUNTIES, &key_bytes)? {
            Some(bytes) => {
                let bounty: BountyData = bincode::deserialize(&bytes)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(bounty))
            }
            None => Ok(None),
        }
    }

    pub fn update_state(&self, bounty_id: &str, new_state: &str) -> Result<()> {
        let mut bounty = self.get(bounty_id)?.ok_or_else(|| StorageError::KeyNotFound {
            cf: cf::CF_BOUNTIES.to_string(),
            key: bounty_id.to_string(),
        })?;
        bounty.state = new_state.to_string();
        self.put(&bounty)
    }

    pub fn list_all(&self) -> Result<Vec<BountyData>> {
        let entries = self.db.prefix_scan(cf::CF_BOUNTIES, &[])?;
        let mut results = Vec::with_capacity(entries.len());
        for (key, value) in &entries {
            match bincode::deserialize(value) {
                Ok(bounty) => results.push(bounty),
                Err(e) => {
                    tracing::warn!("skipping corrupt bounty entry (key {:?}): {}", key, e);
                }
            }
        }
        Ok(results)
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
            "neunode_storage_bounty_{:?}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&dir);
        NeunodeDb::open(&dir).unwrap()
    }

    fn make_bounty(id: &str, state: &str) -> BountyData {
        BountyData {
            id: id.to_string(),
            state: state.to_string(),
            requester_did: "did:neunode:requester".to_string(),
            provider_did: None,
            reward_amount: 500,
            reward_token_type: 0x01,
            deadline: 1700001000,
            created_at: 1700000000,
            escrow_deposited: 500,
            title: String::new(),
            description: String::new(),
            claim_deadline: 1700000000 + 7 * 86400,
            work_deadline: 1700000000 + 14 * 86400,
            review_deadline: 1700000000 + 17 * 86400,
            artifact_hash: None,
            bond: None,
        }
    }

    #[test]
    fn test_put_and_get() {
        let db = temp_db();
        let store = BountyStore::new(&db);
        let bounty = make_bounty("bnty_001", "Open");

        store.put(&bounty).unwrap();

        let fetched = store.get("bnty_001").unwrap();
        assert_eq!(fetched, Some(bounty));
    }

    #[test]
    fn test_get_missing() {
        let db = temp_db();
        let store = BountyStore::new(&db);
        assert!(store.get("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_update_state() {
        let db = temp_db();
        let store = BountyStore::new(&db);
        store.put(&make_bounty("bnty_002", "Open")).unwrap();

        store.update_state("bnty_002", "Claimed").unwrap();

        let fetched = store.get("bnty_002").unwrap().unwrap();
        assert_eq!(fetched.state, "Claimed");
    }

    #[test]
    fn test_update_state_not_found() {
        let db = temp_db();
        let store = BountyStore::new(&db);
        let err = store.update_state("ghost", "Claimed").unwrap_err();
        assert!(matches!(err, StorageError::KeyNotFound { .. }));
    }

    #[test]
    fn test_list_all() {
        let db = temp_db();
        let store = BountyStore::new(&db);

        store.put(&make_bounty("bnty_a", "Open")).unwrap();
        store.put(&make_bounty("bnty_b", "Open")).unwrap();
        store.put(&make_bounty("bnty_c", "Paid")).unwrap();

        let all = store.list_all().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_list_all_empty() {
        let db = temp_db();
        let store = BountyStore::new(&db);
        let all = store.list_all().unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn test_overwrite_bounty() {
        let db = temp_db();
        let store = BountyStore::new(&db);

        let mut bounty = make_bounty("bnty_ow", "Open");
        store.put(&bounty).unwrap();

        bounty.state = "Submitted".to_string();
        bounty.provider_did = Some("did:neunode:provider".to_string());
        store.put(&bounty).unwrap();

        let fetched = store.get("bnty_ow").unwrap().unwrap();
        assert_eq!(fetched.state, "Submitted");
        assert_eq!(fetched.provider_did, Some("did:neunode:provider".to_string()));
    }

    #[test]
    fn test_list_all_skips_corrupt() {
        let db = temp_db();
        let store = BountyStore::new(&db);

        // Insert a valid bounty
        store.put(&make_bounty("bnty_valid", "Open")).unwrap();

        // Insert a raw corrupt entry directly into the CF
        let corrupt_key = bincode::serialize("bnty_corrupt").unwrap();
        let corrupt_value = b"this is not valid bincode data".as_slice();
        db.put_raw(cf::CF_BOUNTIES, &corrupt_key, corrupt_value).unwrap();

        let all = store.list_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "bnty_valid");
    }

    #[test]
    fn test_bounty_full_lifecycle() {
        let db = temp_db();
        let store = BountyStore::new(&db);

        let id = "bnty_lifecycle";
        store.put(&make_bounty(id, "Open")).unwrap();

        let states = ["Claimed", "Submitted", "UnderReview", "Accepted", "Paid"];
        for state in states {
            store.update_state(id, state).unwrap();
            assert_eq!(store.get(id).unwrap().unwrap().state, state);
        }
    }
}
