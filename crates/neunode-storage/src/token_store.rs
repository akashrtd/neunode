use crate::cf;
use crate::db::NeunodeDb;
use crate::error::{Result, StorageError};
use serde::{Deserialize, Serialize};

pub const TOKEN_COMPUTE: u8 = 0x01;
pub const TOKEN_TRAINING: u8 = 0x02;
pub const TOKEN_BANDWIDTH: u8 = 0x03;
pub const TOKEN_STORAGE: u8 = 0x04;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct TokenBalance {
    pub balance: u128,
    pub staked: u128,
    pub last_decay_epoch: u64,
}

pub struct TokenStore<'a> {
    db: &'a NeunodeDb,
}

impl<'a> TokenStore<'a> {
    pub fn new(db: &'a NeunodeDb) -> TokenStore<'a> {
        TokenStore { db }
    }

    pub fn get_balance(&self, agent_did: &str, token_type: u8) -> Result<TokenBalance> {
        let hash = cf::did_hash_16(agent_did);
        let key = cf::token_key(&hash, token_type);
        match self.db.get_raw(cf::CF_TOKENS, &key)? {
            Some(bytes) => {
                let balance: TokenBalance = bincode::deserialize(&bytes)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(balance)
            }
            None => Ok(TokenBalance::default()),
        }
    }

    pub fn set_balance(
        &self,
        agent_did: &str,
        token_type: u8,
        balance: &TokenBalance,
    ) -> Result<()> {
        let hash = cf::did_hash_16(agent_did);
        let key = cf::token_key(&hash, token_type);
        let bytes =
            bincode::serialize(balance).map_err(|e| StorageError::Serialization(e.to_string()))?;
        self.db.put_raw(cf::CF_TOKENS, &key, &bytes)
    }

    pub fn get_all_balances(&self, agent_did: &str) -> Result<[TokenBalance; 4]> {
        let types = [TOKEN_COMPUTE, TOKEN_TRAINING, TOKEN_BANDWIDTH, TOKEN_STORAGE];
        let balances: Vec<TokenBalance> =
            types.iter().map(|tt| self.get_balance(agent_did, *tt)).collect::<Result<Vec<_>>>()?;
        balances.try_into().map_err(|v: Vec<TokenBalance>| StorageError::TokenCountMismatch {
            expected: 4,
            got: v.len(),
        })
    }

    pub fn transfer(
        &self,
        from_did: &str,
        to_did: &str,
        token_type: u8,
        amount: u128,
    ) -> Result<()> {
        self.db.with_ledger_write(|| self.transfer_locked(from_did, to_did, token_type, amount))
    }

    pub fn stake(&self, agent_did: &str, token_type: u8, amount: u128) -> Result<TokenBalance> {
        self.db.with_ledger_write(|| {
            let mut balance = self.get_balance(agent_did, token_type)?;
            if balance.balance < amount {
                return Err(StorageError::InsufficientBalance {
                    required: amount,
                    available: balance.balance,
                });
            }
            balance.balance -= amount;
            balance.staked = balance.staked.checked_add(amount).ok_or_else(|| {
                StorageError::Serialization("staked balance overflow".to_string())
            })?;
            self.set_balance(agent_did, token_type, &balance)?;
            Ok(balance)
        })
    }

    pub fn unstake_first(
        &self,
        agent_did: &str,
        token_types: &[u8],
        amount: u128,
    ) -> Result<(u8, TokenBalance)> {
        self.db.with_ledger_write(|| {
            let mut largest_stake = 0;
            for token_type in token_types {
                let mut balance = self.get_balance(agent_did, *token_type)?;
                largest_stake = largest_stake.max(balance.staked);
                if balance.staked >= amount {
                    balance.staked -= amount;
                    balance.balance = balance.balance.checked_add(amount).ok_or_else(|| {
                        StorageError::Serialization("token balance overflow".to_string())
                    })?;
                    self.set_balance(agent_did, *token_type, &balance)?;
                    return Ok((*token_type, balance));
                }
            }
            Err(StorageError::InsufficientStakedBalance {
                required: amount,
                available: largest_stake,
            })
        })
    }

    fn transfer_locked(
        &self,
        from_did: &str,
        to_did: &str,
        token_type: u8,
        amount: u128,
    ) -> Result<()> {
        let mut from_balance = self.get_balance(from_did, token_type)?;
        if from_balance.balance < amount {
            return Err(StorageError::InsufficientBalance {
                required: amount,
                available: from_balance.balance,
            });
        }

        let mut to_balance = self.get_balance(to_did, token_type)?;
        from_balance.balance -= amount;
        to_balance.balance += amount;

        let from_key = cf::token_key(&cf::did_hash_16(from_did), token_type);
        let to_key = cf::token_key(&cf::did_hash_16(to_did), token_type);

        let from_bytes = bincode::serialize(&from_balance)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let to_bytes = bincode::serialize(&to_balance)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        self.db.batch_put_raw(&[
            (cf::CF_TOKENS, &from_key, &from_bytes),
            (cf::CF_TOKENS, &to_key, &to_bytes),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NeunodeDb;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Barrier};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_db() -> NeunodeDb {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "neunode_storage_token_{:?}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&dir);
        NeunodeDb::open(&dir).unwrap()
    }

    #[test]
    fn test_default_balance() {
        let db = temp_db();
        let store = TokenStore::new(&db);
        let bal = store.get_balance("did:neunode:newguy", TOKEN_COMPUTE).unwrap();
        assert_eq!(bal, TokenBalance::default());
        assert_eq!(bal.balance, 0);
        assert_eq!(bal.staked, 0);
    }

    #[test]
    fn test_set_and_get_balance() {
        let db = temp_db();
        let store = TokenStore::new(&db);
        let bal = TokenBalance { balance: 1000, staked: 200, last_decay_epoch: 42 };

        store.set_balance("did:neunode:rich", TOKEN_COMPUTE, &bal).unwrap();

        let fetched = store.get_balance("did:neunode:rich", TOKEN_COMPUTE).unwrap();
        assert_eq!(fetched, bal);
    }

    #[test]
    fn test_all_four_token_types() {
        let db = temp_db();
        let store = TokenStore::new(&db);

        let compute = TokenBalance { balance: 100, staked: 0, last_decay_epoch: 0 };
        let training = TokenBalance { balance: 200, staked: 0, last_decay_epoch: 0 };
        let bandwidth = TokenBalance { balance: 300, staked: 0, last_decay_epoch: 0 };
        let storage = TokenBalance { balance: 400, staked: 0, last_decay_epoch: 0 };

        let did = "did:neunode:multi";
        store.set_balance(did, TOKEN_COMPUTE, &compute).unwrap();
        store.set_balance(did, TOKEN_TRAINING, &training).unwrap();
        store.set_balance(did, TOKEN_BANDWIDTH, &bandwidth).unwrap();
        store.set_balance(did, TOKEN_STORAGE, &storage).unwrap();

        let all = store.get_all_balances(did).unwrap();
        assert_eq!(all[0], compute);
        assert_eq!(all[1], training);
        assert_eq!(all[2], bandwidth);
        assert_eq!(all[3], storage);
    }

    #[test]
    fn test_transfer() {
        let db = temp_db();
        let store = TokenStore::new(&db);

        let from_did = "did:neunode:sender";
        let to_did = "did:neunode:receiver";

        store
            .set_balance(
                from_did,
                TOKEN_COMPUTE,
                &TokenBalance { balance: 1000, staked: 0, last_decay_epoch: 0 },
            )
            .unwrap();
        store
            .set_balance(
                to_did,
                TOKEN_COMPUTE,
                &TokenBalance { balance: 100, staked: 0, last_decay_epoch: 0 },
            )
            .unwrap();

        store.transfer(from_did, to_did, TOKEN_COMPUTE, 300).unwrap();

        let from_bal = store.get_balance(from_did, TOKEN_COMPUTE).unwrap();
        let to_bal = store.get_balance(to_did, TOKEN_COMPUTE).unwrap();

        assert_eq!(from_bal.balance, 700);
        assert_eq!(to_bal.balance, 400);
    }

    #[test]
    fn test_transfer_insufficient_balance() {
        let db = temp_db();
        let store = TokenStore::new(&db);

        let from_did = "did:neunode:poor";
        let to_did = "did:neunode:greedy";

        store
            .set_balance(
                from_did,
                TOKEN_TRAINING,
                &TokenBalance { balance: 50, staked: 0, last_decay_epoch: 0 },
            )
            .unwrap();

        let result = store.transfer(from_did, to_did, TOKEN_TRAINING, 100);
        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::InsufficientBalance { required, available } => {
                assert_eq!(required, 100);
                assert_eq!(available, 50);
            }
            other => panic!("expected InsufficientBalance, got {other}"),
        }

        let from_bal = store.get_balance(from_did, TOKEN_TRAINING).unwrap();
        assert_eq!(from_bal.balance, 50, "balance should be unchanged");
    }

    #[test]
    fn test_transfer_does_not_affect_other_token_types() {
        let db = temp_db();
        let store = TokenStore::new(&db);

        let from = "did:neunode:xfer_test";
        let to = "did:neunode:xfer_recv";

        store
            .set_balance(from, TOKEN_COMPUTE, &TokenBalance { balance: 500, ..Default::default() })
            .unwrap();
        store
            .set_balance(from, TOKEN_TRAINING, &TokenBalance { balance: 999, ..Default::default() })
            .unwrap();

        store.transfer(from, to, TOKEN_COMPUTE, 100).unwrap();

        let training_bal = store.get_balance(from, TOKEN_TRAINING).unwrap();
        assert_eq!(training_bal.balance, 999);
    }

    #[test]
    fn concurrent_transfers_are_isolated_and_conserve_balance() {
        let db = Arc::new(temp_db());
        let sender = "did:neunode:concurrent-sender";
        TokenStore::new(&db)
            .set_balance(
                sender,
                TOKEN_COMPUTE,
                &TokenBalance { balance: 1000, ..Default::default() },
            )
            .unwrap();
        let barrier = Arc::new(Barrier::new(11));
        let mut handles = Vec::new();
        for index in 0..10 {
            let db = Arc::clone(&db);
            let barrier = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                TokenStore::new(&db).transfer(
                    sender,
                    &format!("did:neunode:recipient-{index}"),
                    TOKEN_COMPUTE,
                    100,
                )
            }));
        }
        barrier.wait();
        for handle in handles {
            handle.join().unwrap().unwrap();
        }

        let store = TokenStore::new(&db);
        assert_eq!(store.get_balance(sender, TOKEN_COMPUTE).unwrap().balance, 0);
        let recipient_total: u128 = (0..10)
            .map(|index| {
                store
                    .get_balance(&format!("did:neunode:recipient-{index}"), TOKEN_COMPUTE)
                    .unwrap()
                    .balance
            })
            .sum();
        assert_eq!(recipient_total, 1000);
    }
}
