use std::collections::HashMap;

use neunode_core::constants::token::{MIN_STAKE, UNBONDING_PERIOD_SECS};
use neunode_core::types::{Did, Timestamp, TokenAmount, TokenType};
use serde::{Deserialize, Serialize};

use crate::balance::BalanceSheet;
use crate::error::{Result, TokenError};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StakeEntry {
    pub amount: TokenAmount,
    pub token_type: TokenType,
    pub staked_at: Timestamp,
    pub unbonding_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Default)]
pub struct StakingManager {
    stakes: HashMap<Did, Vec<StakeEntry>>,
}

impl StakingManager {
    pub fn new() -> Self {
        Self { stakes: HashMap::new() }
    }

    pub fn stake(
        &mut self,
        did: &Did,
        token_type: TokenType,
        amount: TokenAmount,
        balances: &mut BalanceSheet,
    ) -> Result<()> {
        if amount.0 < MIN_STAKE {
            return Err(TokenError::InsufficientStake {
                required: TokenAmount(MIN_STAKE),
                available: amount,
            });
        }

        balances.withdraw(token_type, amount)?;

        let now = current_timestamp();
        let entry = StakeEntry { amount, token_type, staked_at: now, unbonding_at: None };
        self.stakes.entry(did.clone()).or_default().push(entry);
        Ok(())
    }

    pub fn begin_unbonding(
        &mut self,
        did: &Did,
        token_type: TokenType,
        amount: TokenAmount,
    ) -> Result<Timestamp> {
        let entries = self.stakes.get_mut(did).ok_or(TokenError::NotStaked)?;

        // Find a matching non-unbonding entry with sufficient amount
        let entry_idx = entries
            .iter()
            .position(|e| {
                e.token_type == token_type && e.unbonding_at.is_none() && e.amount >= amount
            })
            .ok_or(TokenError::NotStaked)?;

        let entry = &mut entries[entry_idx];
        if entry.amount == amount {
            entry.unbonding_at = Some(current_timestamp() + UNBONDING_PERIOD_SECS);
            Ok(entry.unbonding_at.unwrap_or(0))
        } else {
            // Partial unbonding: split entry
            let remainder = entry.amount.checked_sub(amount).ok_or(TokenError::Overflow)?;
            entry.amount = remainder;

            let unbonding_entry = StakeEntry {
                amount,
                token_type,
                staked_at: entry.staked_at,
                unbonding_at: Some(current_timestamp() + UNBONDING_PERIOD_SECS),
            };
            let unbond_at = unbonding_entry.unbonding_at.unwrap_or(0);
            entries.push(unbonding_entry);
            Ok(unbond_at)
        }
    }

    pub fn complete_unbonding(
        &mut self,
        did: &Did,
        token_type: TokenType,
        now: Timestamp,
    ) -> Result<TokenAmount> {
        let entries = self.stakes.get_mut(did).ok_or(TokenError::NotStaked)?;

        let mut total_unbonded = TokenAmount::ZERO;
        let mut remaining = Vec::new();

        for entry in entries.drain(..) {
            if entry.token_type == token_type {
                if let Some(unbond_at) = entry.unbonding_at {
                    if now >= unbond_at {
                        total_unbonded =
                            total_unbonded.checked_add(entry.amount).ok_or(TokenError::Overflow)?;
                        // Don't keep this entry — tokens are returned
                        continue;
                    }
                }
            }
            remaining.push(entry);
        }

        if total_unbonded == TokenAmount::ZERO {
            *entries = remaining;
            return Err(TokenError::NotStaked);
        }

        *entries = remaining;
        Ok(total_unbonded)
    }

    pub fn total_staked(&self, did: &Did) -> TokenAmount {
        self.stakes
            .get(did)
            .map(|entries| {
                entries.iter().fold(TokenAmount::ZERO, |acc, e| {
                    acc.checked_add(e.amount).unwrap_or(TokenAmount(u64::MAX))
                })
            })
            .unwrap_or(TokenAmount::ZERO)
    }

    pub fn total_staked_by_type(&self, did: &Did, token_type: TokenType) -> TokenAmount {
        self.stakes
            .get(did)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|e| e.token_type == token_type)
                    .fold(TokenAmount::ZERO, |acc, e| {
                        acc.checked_add(e.amount).unwrap_or(TokenAmount(u64::MAX))
                    })
            })
            .unwrap_or(TokenAmount::ZERO)
    }

    pub fn is_unbonding_complete(&self, did: &Did, entry_index: usize, now: Timestamp) -> bool {
        self.stakes
            .get(did)
            .and_then(|entries| entries.get(entry_index))
            .map(|entry| entry.unbonding_at.is_some_and(|unbond_at| now >= unbond_at))
            .unwrap_or(false)
    }

    pub fn get_stakes(&self, did: &Did) -> &[StakeEntry] {
        self.stakes.get(did).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

fn current_timestamp() -> Timestamp {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_did() -> Did {
        Did("did:neunode:test_agent".to_string())
    }

    fn test_did_other() -> Did {
        Did("did:neunode:other_agent".to_string())
    }

    #[test]
    fn stake_locks_tokens() {
        let mut mgr = StakingManager::new();
        let mut balances = BalanceSheet::new();
        balances.deposit(TokenType::Compute, TokenAmount(500)).unwrap();

        mgr.stake(&test_did(), TokenType::Compute, TokenAmount(200), &mut balances).unwrap();

        assert_eq!(balances.get_balance(TokenType::Compute), TokenAmount(300));
        assert_eq!(mgr.total_staked(&test_did()), TokenAmount(200));
    }

    #[test]
    fn stake_below_minimum() {
        let mut mgr = StakingManager::new();
        let mut balances = BalanceSheet::new();
        balances.deposit(TokenType::Compute, TokenAmount(500)).unwrap();

        let result = mgr.stake(&test_did(), TokenType::Compute, TokenAmount(50), &mut balances);
        assert!(matches!(
            result,
            Err(TokenError::InsufficientStake {
                required: TokenAmount(100),
                available: TokenAmount(50),
            })
        ));
    }

    #[test]
    fn stake_at_minimum() {
        let mut mgr = StakingManager::new();
        let mut balances = BalanceSheet::new();
        balances.deposit(TokenType::Compute, TokenAmount(500)).unwrap();

        mgr.stake(&test_did(), TokenType::Compute, TokenAmount(MIN_STAKE), &mut balances).unwrap();
        assert_eq!(mgr.total_staked(&test_did()), TokenAmount(MIN_STAKE));
    }

    #[test]
    fn stake_insufficient_balance() {
        let mut mgr = StakingManager::new();
        let mut balances = BalanceSheet::new();
        balances.deposit(TokenType::Compute, TokenAmount(50)).unwrap();

        let result = mgr.stake(&test_did(), TokenType::Compute, TokenAmount(100), &mut balances);
        assert!(matches!(result, Err(TokenError::InsufficientBalance { .. })));
    }

    #[test]
    fn stake_multiple_entries() {
        let mut mgr = StakingManager::new();
        let mut balances = BalanceSheet::new();
        balances.deposit(TokenType::Compute, TokenAmount(1000)).unwrap();

        mgr.stake(&test_did(), TokenType::Compute, TokenAmount(200), &mut balances).unwrap();
        mgr.stake(&test_did(), TokenType::Compute, TokenAmount(300), &mut balances).unwrap();

        assert_eq!(mgr.total_staked(&test_did()), TokenAmount(500));
        assert_eq!(mgr.get_stakes(&test_did()).len(), 2);
    }

    #[test]
    fn stake_different_agents_isolated() {
        let mut mgr = StakingManager::new();
        let mut bal_a = BalanceSheet::new();
        let mut bal_b = BalanceSheet::new();
        bal_a.deposit(TokenType::Compute, TokenAmount(500)).unwrap();
        bal_b.deposit(TokenType::Train, TokenAmount(500)).unwrap();

        mgr.stake(&test_did(), TokenType::Compute, TokenAmount(200), &mut bal_a).unwrap();
        mgr.stake(&test_did_other(), TokenType::Train, TokenAmount(300), &mut bal_b).unwrap();

        assert_eq!(mgr.total_staked_by_type(&test_did(), TokenType::Compute), TokenAmount(200));
        assert_eq!(mgr.total_staked_by_type(&test_did(), TokenType::Train), TokenAmount::ZERO);
        assert_eq!(mgr.total_staked_by_type(&test_did_other(), TokenType::Train), TokenAmount(300));
    }

    #[test]
    fn begin_unbonding_sets_timestamp() {
        let mut mgr = StakingManager::new();
        let mut balances = BalanceSheet::new();
        balances.deposit(TokenType::Compute, TokenAmount(500)).unwrap();

        mgr.stake(&test_did(), TokenType::Compute, TokenAmount(200), &mut balances).unwrap();

        let unbond_at =
            mgr.begin_unbonding(&test_did(), TokenType::Compute, TokenAmount(200)).unwrap();
        assert!(unbond_at > 0);

        let entry = &mgr.get_stakes(&test_did())[0];
        assert_eq!(entry.unbonding_at, Some(unbond_at));
    }

    #[test]
    fn begin_unbonding_not_staked() {
        let mut mgr = StakingManager::new();
        let result = mgr.begin_unbonding(&test_did(), TokenType::Compute, TokenAmount(100));
        assert!(matches!(result, Err(TokenError::NotStaked)));
    }

    #[test]
    fn begin_unbonding_partial_amount() {
        let mut mgr = StakingManager::new();
        let mut balances = BalanceSheet::new();
        balances.deposit(TokenType::Compute, TokenAmount(500)).unwrap();

        mgr.stake(&test_did(), TokenType::Compute, TokenAmount(300), &mut balances).unwrap();

        let unbond_at =
            mgr.begin_unbonding(&test_did(), TokenType::Compute, TokenAmount(100)).unwrap();
        assert!(unbond_at > 0);

        // Original entry should be reduced, new unbonding entry added
        let stakes = mgr.get_stakes(&test_did());
        assert_eq!(stakes.len(), 2);
        assert_eq!(mgr.total_staked(&test_did()), TokenAmount(300));
    }

    #[test]
    fn complete_unbonding_after_period() {
        let mut mgr = StakingManager::new();
        let mut balances = BalanceSheet::new();
        balances.deposit(TokenType::Compute, TokenAmount(500)).unwrap();

        mgr.stake(&test_did(), TokenType::Compute, TokenAmount(200), &mut balances).unwrap();

        let unbond_at =
            mgr.begin_unbonding(&test_did(), TokenType::Compute, TokenAmount(200)).unwrap();

        let returned = mgr.complete_unbonding(&test_did(), TokenType::Compute, unbond_at).unwrap();
        assert_eq!(returned, TokenAmount(200));
        assert_eq!(mgr.total_staked(&test_did()), TokenAmount::ZERO);
    }

    #[test]
    fn complete_unbonding_too_early() {
        let mut mgr = StakingManager::new();
        let mut balances = BalanceSheet::new();
        balances.deposit(TokenType::Compute, TokenAmount(500)).unwrap();

        mgr.stake(&test_did(), TokenType::Compute, TokenAmount(200), &mut balances).unwrap();

        let unbond_at =
            mgr.begin_unbonding(&test_did(), TokenType::Compute, TokenAmount(200)).unwrap();

        // Try to complete 1 second before unbonding period
        let result = mgr.complete_unbonding(&test_did(), TokenType::Compute, unbond_at - 1);
        assert!(matches!(result, Err(TokenError::NotStaked)));
        // Stake should still be there
        assert_eq!(mgr.total_staked(&test_did()), TokenAmount(200));
    }

    #[test]
    fn is_unbonding_complete_true() {
        let mut mgr = StakingManager::new();
        let mut balances = BalanceSheet::new();
        balances.deposit(TokenType::Compute, TokenAmount(500)).unwrap();

        mgr.stake(&test_did(), TokenType::Compute, TokenAmount(200), &mut balances).unwrap();

        let unbond_at =
            mgr.begin_unbonding(&test_did(), TokenType::Compute, TokenAmount(200)).unwrap();

        assert!(mgr.is_unbonding_complete(&test_did(), 0, unbond_at));
    }

    #[test]
    fn is_unbonding_complete_false() {
        let mut mgr = StakingManager::new();
        let mut balances = BalanceSheet::new();
        balances.deposit(TokenType::Compute, TokenAmount(500)).unwrap();

        mgr.stake(&test_did(), TokenType::Compute, TokenAmount(200), &mut balances).unwrap();

        let unbond_at =
            mgr.begin_unbonding(&test_did(), TokenType::Compute, TokenAmount(200)).unwrap();

        assert!(!mgr.is_unbonding_complete(&test_did(), 0, unbond_at - 1));
    }

    #[test]
    fn is_unbonding_complete_no_unbonding() {
        let mut mgr = StakingManager::new();
        let mut balances = BalanceSheet::new();
        balances.deposit(TokenType::Compute, TokenAmount(500)).unwrap();

        mgr.stake(&test_did(), TokenType::Compute, TokenAmount(200), &mut balances).unwrap();

        assert!(!mgr.is_unbonding_complete(&test_did(), 0, u64::MAX));
    }

    #[test]
    fn is_unbonding_complete_invalid_index() {
        let mgr = StakingManager::new();
        assert!(!mgr.is_unbonding_complete(&test_did(), 99, 0));
    }

    #[test]
    fn total_staked_empty() {
        let mgr = StakingManager::new();
        assert_eq!(mgr.total_staked(&test_did()), TokenAmount::ZERO);
    }

    #[test]
    fn total_staked_by_type_empty() {
        let mgr = StakingManager::new();
        assert_eq!(mgr.total_staked_by_type(&test_did(), TokenType::Compute), TokenAmount::ZERO);
    }

    #[test]
    fn full_stake_unbond_flow() {
        let mut mgr = StakingManager::new();
        let mut balances = BalanceSheet::new();
        balances.deposit(TokenType::Compute, TokenAmount(1000)).unwrap();

        mgr.stake(&test_did(), TokenType::Compute, TokenAmount(300), &mut balances).unwrap();
        assert_eq!(balances.get_balance(TokenType::Compute), TokenAmount(700));
        assert_eq!(mgr.total_staked(&test_did()), TokenAmount(300));

        let unbond_at =
            mgr.begin_unbonding(&test_did(), TokenType::Compute, TokenAmount(300)).unwrap();

        let returned = mgr.complete_unbonding(&test_did(), TokenType::Compute, unbond_at).unwrap();
        assert_eq!(returned, TokenAmount(300));

        balances.deposit(TokenType::Compute, returned).unwrap();
        assert_eq!(balances.get_balance(TokenType::Compute), TokenAmount(1000));
        assert_eq!(mgr.total_staked(&test_did()), TokenAmount::ZERO);
    }
}
