use crate::cf;
use crate::db::NeunodeDb;
use crate::error::{Result, StorageError};
use crate::token_store::TokenStore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

    /// Atomically create a bounty with escrow: validates balance, transfers tokens
    /// to escrow, and persists the bounty record in a single RocksDB WriteBatch.
    /// All writes target the ledger partition, so the batch is fully atomic.
    pub fn create_with_escrow(
        &self,
        bounty: &BountyData,
        creator_did: &str,
        escrow_did: &str,
        token_type: u8,
        amount: u128,
    ) -> Result<()> {
        let token_store = TokenStore::new(self.db);

        let mut from_balance = token_store.get_balance(creator_did, token_type)?;
        if from_balance.balance < amount {
            return Err(StorageError::InsufficientBalance {
                required: amount,
                available: from_balance.balance,
            });
        }
        let mut to_balance = token_store.get_balance(escrow_did, token_type)?;
        from_balance.balance -= amount;
        to_balance.balance += amount;

        let from_key = cf::token_key(&cf::did_hash_16(creator_did), token_type);
        let to_key = cf::token_key(&cf::did_hash_16(escrow_did), token_type);
        let from_bytes = bincode::serialize(&from_balance)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let to_bytes = bincode::serialize(&to_balance)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        let bounty_key = bincode::serialize(&bounty.id)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let bounty_bytes =
            bincode::serialize(bounty).map_err(|e| StorageError::Serialization(e.to_string()))?;

        // Single atomic WriteBatch — all keys are in the ledger partition.
        self.db.batch_put_raw(&[
            (cf::CF_TOKENS, &from_key, &from_bytes),
            (cf::CF_TOKENS, &to_key, &to_bytes),
            (cf::CF_BOUNTIES, &bounty_key, &bounty_bytes),
        ])
    }

    /// Atomically persist a bounty transition and its accompanying token transfer.
    /// Bounties and token balances both live in the ledger partition, so either all
    /// three records are committed or none are.
    pub fn transition_with_transfer(
        &self,
        bounty: &BountyData,
        from_did: &str,
        to_did: &str,
        token_type: u8,
        amount: u128,
    ) -> Result<()> {
        let token_store = TokenStore::new(self.db);
        let mut from_balance = token_store.get_balance(from_did, token_type)?;
        if from_balance.balance < amount {
            return Err(StorageError::InsufficientBalance {
                required: amount,
                available: from_balance.balance,
            });
        }
        let mut to_balance = token_store.get_balance(to_did, token_type)?;
        from_balance.balance -= amount;
        to_balance.balance = to_balance
            .balance
            .checked_add(amount)
            .ok_or_else(|| StorageError::Serialization("token balance overflow".to_string()))?;

        let from_key = cf::token_key(&cf::did_hash_16(from_did), token_type);
        let to_key = cf::token_key(&cf::did_hash_16(to_did), token_type);
        let from_bytes = bincode::serialize(&from_balance)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let to_bytes = bincode::serialize(&to_balance)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let bounty_key = bincode::serialize(&bounty.id)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let bounty_bytes =
            bincode::serialize(bounty).map_err(|e| StorageError::Serialization(e.to_string()))?;

        self.db.batch_put_raw(&[
            (cf::CF_TOKENS, &from_key, &from_bytes),
            (cf::CF_TOKENS, &to_key, &to_bytes),
            (cf::CF_BOUNTIES, &bounty_key, &bounty_bytes),
        ])
    }

    /// Atomically persist a bounty transition and distribute funds from one
    /// escrow balance to one or more recipients.
    pub fn transition_with_payouts(
        &self,
        bounty: &BountyData,
        escrow_did: &str,
        token_type: u8,
        payouts: &[(&str, u128)],
    ) -> Result<()> {
        let token_store = TokenStore::new(self.db);
        let total = payouts.iter().try_fold(0_u128, |sum, (_, amount)| {
            sum.checked_add(*amount)
                .ok_or_else(|| StorageError::Serialization("payout total overflow".to_string()))
        })?;
        let mut escrow = token_store.get_balance(escrow_did, token_type)?;
        if escrow.balance < total {
            return Err(StorageError::InsufficientBalance {
                required: total,
                available: escrow.balance,
            });
        }
        escrow.balance -= total;

        let mut recipients = BTreeMap::<String, crate::token_store::TokenBalance>::new();
        for (did, amount) in payouts {
            if !recipients.contains_key(*did) {
                recipients.insert((*did).to_string(), token_store.get_balance(did, token_type)?);
            }
            let balance = recipients.get_mut(*did).expect("recipient inserted");
            balance.balance = balance
                .balance
                .checked_add(*amount)
                .ok_or_else(|| StorageError::Serialization("token balance overflow".to_string()))?;
        }

        let escrow_key = cf::token_key(&cf::did_hash_16(escrow_did), token_type);
        let escrow_bytes =
            bincode::serialize(&escrow).map_err(|e| StorageError::Serialization(e.to_string()))?;
        let recipient_records = recipients
            .into_iter()
            .map(|(did, balance)| {
                let key = cf::token_key(&cf::did_hash_16(&did), token_type);
                let bytes = bincode::serialize(&balance)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok((key, bytes))
            })
            .collect::<Result<Vec<_>>>()?;
        let bounty_key = bincode::serialize(&bounty.id)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let bounty_bytes =
            bincode::serialize(bounty).map_err(|e| StorageError::Serialization(e.to_string()))?;

        let mut ops = Vec::with_capacity(recipient_records.len() + 2);
        ops.push((cf::CF_TOKENS, escrow_key.as_slice(), escrow_bytes.as_slice()));
        for (key, bytes) in &recipient_records {
            ops.push((cf::CF_TOKENS, key.as_slice(), bytes.as_slice()));
        }
        ops.push((cf::CF_BOUNTIES, bounty_key.as_slice(), bounty_bytes.as_slice()));
        self.db.batch_put_raw(&ops)
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
    use crate::token_store::{TokenBalance, TokenStore};
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

    #[test]
    fn test_create_with_escrow_atomic() {
        let db = temp_db();
        let token_store = TokenStore::new(&db);
        let bounty_store = BountyStore::new(&db);

        let creator = "did:neunode:creator";
        token_store
            .set_balance(
                creator,
                crate::token_store::TOKEN_COMPUTE,
                &TokenBalance { balance: 5000, staked: 0, last_decay_epoch: 0 },
            )
            .unwrap();

        let bounty = make_bounty("bnty_escrow", "Open");
        let escrow_did = format!("escrow:{}", bounty.id);

        bounty_store
            .create_with_escrow(
                &bounty,
                creator,
                &escrow_did,
                crate::token_store::TOKEN_COMPUTE,
                1000,
            )
            .unwrap();

        // Bounty persisted
        assert!(bounty_store.get("bnty_escrow").unwrap().is_some());

        // Escrow received tokens
        let escrow_bal =
            token_store.get_balance(&escrow_did, crate::token_store::TOKEN_COMPUTE).unwrap();
        assert_eq!(escrow_bal.balance, 1000);

        // Creator debited
        let creator_bal =
            token_store.get_balance(creator, crate::token_store::TOKEN_COMPUTE).unwrap();
        assert_eq!(creator_bal.balance, 4000);
    }

    #[test]
    fn transition_with_transfer_is_atomic_on_insufficient_balance() {
        let db = temp_db();
        let token_store = TokenStore::new(&db);
        let bounty_store = BountyStore::new(&db);
        let original = make_bounty("bnty_atomic", "Open");
        bounty_store.put(&original).unwrap();
        token_store
            .set_balance(
                "did:neunode:poor",
                crate::token_store::TOKEN_COMPUTE,
                &TokenBalance { balance: 10, ..Default::default() },
            )
            .unwrap();

        let mut claimed = original.clone();
        claimed.state = "Claimed".to_string();
        let result = bounty_store.transition_with_transfer(
            &claimed,
            "did:neunode:poor",
            "escrow:bnty_atomic",
            crate::token_store::TOKEN_COMPUTE,
            100,
        );

        assert!(matches!(result, Err(StorageError::InsufficientBalance { .. })));
        assert_eq!(bounty_store.get("bnty_atomic").unwrap(), Some(original));
        assert_eq!(
            token_store
                .get_balance("did:neunode:poor", crate::token_store::TOKEN_COMPUTE)
                .unwrap()
                .balance,
            10
        );
    }

    #[test]
    fn transition_with_transfer_commits_state_and_balances() {
        let db = temp_db();
        let token_store = TokenStore::new(&db);
        let bounty_store = BountyStore::new(&db);
        let original = make_bounty("bnty_claim", "Open");
        bounty_store.put(&original).unwrap();
        token_store
            .set_balance(
                "did:neunode:provider",
                crate::token_store::TOKEN_COMPUTE,
                &TokenBalance { balance: 200, ..Default::default() },
            )
            .unwrap();

        let mut claimed = original;
        claimed.state = "Claimed".to_string();
        claimed.provider_did = Some("did:neunode:provider".to_string());
        claimed.bond = Some(150);
        bounty_store
            .transition_with_transfer(
                &claimed,
                "did:neunode:provider",
                "escrow:bnty_claim",
                crate::token_store::TOKEN_COMPUTE,
                150,
            )
            .unwrap();

        assert_eq!(bounty_store.get("bnty_claim").unwrap(), Some(claimed));
        assert_eq!(
            token_store
                .get_balance("did:neunode:provider", crate::token_store::TOKEN_COMPUTE)
                .unwrap()
                .balance,
            50
        );
        assert_eq!(
            token_store
                .get_balance("escrow:bnty_claim", crate::token_store::TOKEN_COMPUTE)
                .unwrap()
                .balance,
            150
        );
    }

    #[test]
    fn transition_with_payouts_conserves_funds() {
        let db = temp_db();
        let token_store = TokenStore::new(&db);
        let bounty_store = BountyStore::new(&db);
        let mut cancelled = make_bounty("bnty_cancel", "Claimed");
        cancelled.state = "Cancelled".to_string();
        cancelled.provider_did = Some("did:neunode:provider".to_string());
        cancelled.bond = Some(150);
        cancelled.escrow_deposited = 0;
        token_store
            .set_balance(
                "escrow:bnty_cancel",
                crate::token_store::TOKEN_COMPUTE,
                &TokenBalance { balance: 650, ..Default::default() },
            )
            .unwrap();

        bounty_store
            .transition_with_payouts(
                &cancelled,
                "escrow:bnty_cancel",
                crate::token_store::TOKEN_COMPUTE,
                &[("did:neunode:requester", 500), ("did:neunode:provider", 150)],
            )
            .unwrap();

        assert_eq!(
            token_store
                .get_balance("escrow:bnty_cancel", crate::token_store::TOKEN_COMPUTE)
                .unwrap()
                .balance,
            0
        );
        assert_eq!(
            token_store
                .get_balance("did:neunode:requester", crate::token_store::TOKEN_COMPUTE)
                .unwrap()
                .balance,
            500
        );
        assert_eq!(
            token_store
                .get_balance("did:neunode:provider", crate::token_store::TOKEN_COMPUTE)
                .unwrap()
                .balance,
            150
        );
        assert_eq!(bounty_store.get("bnty_cancel").unwrap(), Some(cancelled));
    }

    #[test]
    fn test_create_with_escrow_insufficient_balance() {
        let db = temp_db();
        let token_store = TokenStore::new(&db);
        let bounty_store = BountyStore::new(&db);

        let creator = "did:neunode:poor";
        token_store
            .set_balance(
                creator,
                crate::token_store::TOKEN_COMPUTE,
                &TokenBalance { balance: 100, staked: 0, last_decay_epoch: 0 },
            )
            .unwrap();

        let bounty = make_bounty("bnty_fail", "Open");
        let escrow_did = format!("escrow:{}", bounty.id);

        let result = bounty_store.create_with_escrow(
            &bounty,
            creator,
            &escrow_did,
            crate::token_store::TOKEN_COMPUTE,
            1000,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::InsufficientBalance { required, available } => {
                assert_eq!(required, 1000);
                assert_eq!(available, 100);
            }
            other => panic!("expected InsufficientBalance, got {other}"),
        }

        // Nothing persisted
        assert!(bounty_store.get("bnty_fail").unwrap().is_none());
        let escrow_bal =
            token_store.get_balance(&escrow_did, crate::token_store::TOKEN_COMPUTE).unwrap();
        assert_eq!(escrow_bal.balance, 0);
    }
}
