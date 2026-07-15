use serde::{Deserialize, Serialize};

use crate::cf;
use crate::db::NeunodeDb;
use crate::error::{Result, StorageError};
use crate::token_store::{TokenBalance, TokenStore};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnbondingEntry {
    pub id: String,
    pub agent_did: String,
    pub token_type: u8,
    pub amount: u128,
    pub created_at: u64,
    pub unlock_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedUnbonding {
    pub entries: Vec<UnbondingEntry>,
    pub total: u128,
}

pub struct UnbondingStore<'a> {
    db: &'a NeunodeDb,
}

impl<'a> UnbondingStore<'a> {
    pub fn new(db: &'a NeunodeDb) -> Self {
        Self { db }
    }

    pub fn begin(
        &self,
        agent_did: &str,
        token_types: &[u8],
        amount: u128,
        now: u64,
        delay_secs: u64,
    ) -> Result<UnbondingEntry> {
        self.db.with_ledger_write(|| {
            let token_store = TokenStore::new(self.db);
            let mut largest_stake = 0;
            for token_type in token_types {
                let mut balance = token_store.get_balance(agent_did, *token_type)?;
                largest_stake = largest_stake.max(balance.staked);
                if balance.staked < amount {
                    continue;
                }
                balance.staked -= amount;
                let unlock_at = now.checked_add(delay_secs).ok_or_else(|| {
                    StorageError::Serialization("unbonding timestamp overflow".to_string())
                })?;
                let entry = UnbondingEntry {
                    id: new_entry_id(agent_did, *token_type, now),
                    agent_did: agent_did.to_string(),
                    token_type: *token_type,
                    amount,
                    created_at: now,
                    unlock_at,
                };
                let balance_key = cf::token_key(&cf::did_hash_16(agent_did), *token_type);
                let balance_bytes = crate::codec::serialize(&balance)
                    .map_err(|error| StorageError::Serialization(error.to_string()))?;
                let entry_key = crate::codec::serialize(&entry.id)
                    .map_err(|error| StorageError::Serialization(error.to_string()))?;
                let entry_bytes = crate::codec::serialize(&entry)
                    .map_err(|error| StorageError::Serialization(error.to_string()))?;
                self.db.batch_put_raw(&[
                    (cf::CF_TOKENS, &balance_key, &balance_bytes),
                    (cf::CF_UNBONDING, &entry_key, &entry_bytes),
                ])?;
                return Ok(entry);
            }
            Err(StorageError::InsufficientStakedBalance {
                required: amount,
                available: largest_stake,
            })
        })
    }

    pub fn list(&self, agent_did: &str) -> Result<Vec<UnbondingEntry>> {
        let mut entries = self
            .db
            .prefix_scan(cf::CF_UNBONDING, &[])?
            .into_iter()
            .map(|(_, bytes)| {
                crate::codec::deserialize::<UnbondingEntry>(&bytes)
                    .map_err(|error| StorageError::Serialization(error.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        entries.retain(|entry| entry.agent_did == agent_did);
        entries.sort_by_key(|entry| (entry.unlock_at, entry.id.clone()));
        Ok(entries)
    }

    pub fn claim_matured(&self, agent_did: &str, now: u64) -> Result<ClaimedUnbonding> {
        self.db.with_ledger_write(|| {
            let matured: Vec<_> =
                self.list(agent_did)?.into_iter().filter(|entry| entry.unlock_at <= now).collect();
            if matured.is_empty() {
                return Ok(ClaimedUnbonding { entries: Vec::new(), total: 0 });
            }

            let token_store = TokenStore::new(self.db);
            let mut balances = std::collections::BTreeMap::<u8, TokenBalance>::new();
            let mut total = 0_u128;
            for entry in &matured {
                total = total.checked_add(entry.amount).ok_or_else(|| {
                    StorageError::Serialization("unbonding claim overflow".to_string())
                })?;
                if let std::collections::btree_map::Entry::Vacant(slot) =
                    balances.entry(entry.token_type)
                {
                    slot.insert(token_store.get_balance(agent_did, entry.token_type)?);
                }
                let balance = balances.get_mut(&entry.token_type).expect("balance inserted");
                balance.balance = balance.balance.checked_add(entry.amount).ok_or_else(|| {
                    StorageError::Serialization("token balance overflow".to_string())
                })?;
            }

            let balance_records = balances
                .into_iter()
                .map(|(token_type, balance)| {
                    let key = cf::token_key(&cf::did_hash_16(agent_did), token_type);
                    let bytes = crate::codec::serialize(&balance)
                        .map_err(|error| StorageError::Serialization(error.to_string()))?;
                    Ok((key, bytes))
                })
                .collect::<Result<Vec<_>>>()?;
            let delete_keys = matured
                .iter()
                .map(|entry| {
                    crate::codec::serialize(&entry.id)
                        .map_err(|error| StorageError::Serialization(error.to_string()))
                })
                .collect::<Result<Vec<_>>>()?;
            let puts: Vec<_> = balance_records
                .iter()
                .map(|(key, bytes)| (cf::CF_TOKENS, key.as_slice(), bytes.as_slice()))
                .collect();
            let deletes: Vec<_> =
                delete_keys.iter().map(|key| (cf::CF_UNBONDING, key.as_slice())).collect();
            self.db.batch_write_raw(&puts, &deletes)?;
            Ok(ClaimedUnbonding { entries: matured, total })
        })
    }
}

fn next_sequence() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    SEQUENCE.fetch_add(1, Ordering::Relaxed)
}

fn new_entry_id(agent_did: &str, token_type: u8, now: u64) -> String {
    use std::fmt::Write;

    let wall_clock_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let material = format!("{agent_did}:{token_type}:{now}:{wall_clock_nanos}:{}", next_sequence());
    let digest = neunode_crypto::hash::blake3_hash(material.as_bytes());
    let mut id = String::with_capacity(7 + 32);
    id.push_str("unbond_");
    for byte in &digest[..16] {
        write!(&mut id, "{byte:02x}").expect("writing to String cannot fail");
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_store::TOKEN_COMPUTE;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "neunode_unbonding_{}_{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn unbonding_is_locked_until_maturity_and_survives_restart() {
        let path = temp_path();
        let did = "did:neunode:staker";
        {
            let db = NeunodeDb::open(&path).unwrap();
            let tokens = TokenStore::new(&db);
            tokens
                .set_balance(
                    did,
                    TOKEN_COMPUTE,
                    &TokenBalance { balance: 0, staked: 500, last_decay_epoch: 0 },
                )
                .unwrap();
            let entry =
                UnbondingStore::new(&db).begin(did, &[TOKEN_COMPUTE], 200, 1_000, 100).unwrap();
            assert_eq!(entry.unlock_at, 1_100);
            let balance = tokens.get_balance(did, TOKEN_COMPUTE).unwrap();
            assert_eq!(balance.balance, 0, "unbonding tokens must not be spendable");
            assert_eq!(balance.staked, 300);
            assert!(tokens.transfer(did, "did:neunode:receiver", TOKEN_COMPUTE, 1).is_err());
        }

        let db = NeunodeDb::open(&path).unwrap();
        let store = UnbondingStore::new(&db);
        assert_eq!(store.list(did).unwrap().len(), 1, "pending lot must survive restart");
        assert_eq!(store.claim_matured(did, 1_099).unwrap().total, 0);
        assert_eq!(TokenStore::new(&db).get_balance(did, TOKEN_COMPUTE).unwrap().balance, 0);
        let claimed = store.claim_matured(did, 1_100).unwrap();
        assert_eq!(claimed.total, 200);
        assert_eq!(claimed.entries.len(), 1);
        assert!(store.list(did).unwrap().is_empty());
        assert_eq!(TokenStore::new(&db).get_balance(did, TOKEN_COMPUTE).unwrap().balance, 200);
    }

    #[test]
    fn claim_matured_atomically_claims_only_ready_lots() {
        let db = NeunodeDb::open(&temp_path()).unwrap();
        let did = "did:neunode:multi-lot";
        TokenStore::new(&db)
            .set_balance(
                did,
                TOKEN_COMPUTE,
                &TokenBalance { balance: 10, staked: 500, last_decay_epoch: 0 },
            )
            .unwrap();
        let store = UnbondingStore::new(&db);
        store.begin(did, &[TOKEN_COMPUTE], 100, 1_000, 100).unwrap();
        store.begin(did, &[TOKEN_COMPUTE], 150, 1_050, 200).unwrap();

        let claimed = store.claim_matured(did, 1_150).unwrap();

        assert_eq!(claimed.total, 100);
        assert_eq!(store.list(did).unwrap().len(), 1);
        let balance = TokenStore::new(&db).get_balance(did, TOKEN_COMPUTE).unwrap();
        assert_eq!(balance.balance, 110);
        assert_eq!(balance.staked, 250);
    }
}
