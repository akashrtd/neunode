use std::collections::HashMap;

use neunode_core::types::{TokenAmount, TokenType};

use crate::error::{Result, TokenError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BalanceSheet {
    balances: HashMap<TokenType, TokenAmount>,
}

impl BalanceSheet {
    pub fn new() -> Self {
        let mut balances = HashMap::new();
        balances.insert(TokenType::Compute, TokenAmount::ZERO);
        balances.insert(TokenType::Train, TokenAmount::ZERO);
        balances.insert(TokenType::Bandwidth, TokenAmount::ZERO);
        balances.insert(TokenType::Storage, TokenAmount::ZERO);
        Self { balances }
    }

    pub fn get_balance(&self, token_type: TokenType) -> TokenAmount {
        self.balances.get(&token_type).copied().unwrap_or(TokenAmount::ZERO)
    }

    pub fn deposit(&mut self, token_type: TokenType, amount: TokenAmount) -> Result<()> {
        let current = self.get_balance(token_type);
        let new_balance = current.checked_add(amount).ok_or(TokenError::Overflow)?;
        self.balances.insert(token_type, new_balance);
        Ok(())
    }

    pub fn withdraw(&mut self, token_type: TokenType, amount: TokenAmount) -> Result<()> {
        let current = self.get_balance(token_type);
        if current < amount {
            return Err(TokenError::InsufficientBalance { required: amount, available: current });
        }
        let new_balance = current.checked_sub(amount).ok_or(TokenError::Overflow)?;
        self.balances.insert(token_type, new_balance);
        Ok(())
    }

    pub fn transfer(
        &mut self,
        to: &mut BalanceSheet,
        token_type: TokenType,
        amount: TokenAmount,
    ) -> Result<()> {
        self.withdraw(token_type, amount)?;
        match to.deposit(token_type, amount) {
            Ok(()) => Ok(()),
            Err(e) => {
                // Rollback withdrawal on deposit failure
                let _ = self.deposit(token_type, amount);
                Err(e)
            }
        }
    }

    pub fn total_value(&self) -> TokenAmount {
        let mut total: u64 = 0;
        for &token_type in
            &[TokenType::Compute, TokenType::Train, TokenType::Bandwidth, TokenType::Storage]
        {
            total = total.saturating_add(self.get_balance(token_type).0);
        }
        TokenAmount(total)
    }
}

impl Default for BalanceSheet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_at_zero() {
        let sheet = BalanceSheet::new();
        for tt in [TokenType::Compute, TokenType::Train, TokenType::Bandwidth, TokenType::Storage] {
            assert_eq!(sheet.get_balance(tt), TokenAmount::ZERO);
        }
    }

    #[test]
    fn default_is_new() {
        assert_eq!(BalanceSheet::default(), BalanceSheet::new());
    }

    #[test]
    fn deposit_adds_tokens() {
        let mut sheet = BalanceSheet::new();
        sheet.deposit(TokenType::Compute, TokenAmount(500)).unwrap();
        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount(500));
    }

    #[test]
    fn deposit_accumulates() {
        let mut sheet = BalanceSheet::new();
        sheet.deposit(TokenType::Compute, TokenAmount(100)).unwrap();
        sheet.deposit(TokenType::Compute, TokenAmount(200)).unwrap();
        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount(300));
    }

    #[test]
    fn deposit_overflow() {
        let mut sheet = BalanceSheet::new();
        sheet.deposit(TokenType::Compute, TokenAmount(u64::MAX)).unwrap();
        let result = sheet.deposit(TokenType::Compute, TokenAmount(1));
        assert!(matches!(result, Err(TokenError::Overflow)));
    }

    #[test]
    fn withdraw_subtracts_tokens() {
        let mut sheet = BalanceSheet::new();
        sheet.deposit(TokenType::Train, TokenAmount(1000)).unwrap();
        sheet.withdraw(TokenType::Train, TokenAmount(300)).unwrap();
        assert_eq!(sheet.get_balance(TokenType::Train), TokenAmount(700));
    }

    #[test]
    fn withdraw_insufficient_balance() {
        let mut sheet = BalanceSheet::new();
        sheet.deposit(TokenType::Train, TokenAmount(100)).unwrap();
        let result = sheet.withdraw(TokenType::Train, TokenAmount(200));
        assert!(matches!(
            result,
            Err(TokenError::InsufficientBalance {
                required: TokenAmount(200),
                available: TokenAmount(100),
            })
        ));
    }

    #[test]
    fn withdraw_insufficient_balance_zero() {
        let mut sheet = BalanceSheet::new();
        let result = sheet.withdraw(TokenType::Bandwidth, TokenAmount(1));
        assert!(matches!(
            result,
            Err(TokenError::InsufficientBalance {
                required: TokenAmount(1),
                available: TokenAmount(0),
            })
        ));
    }

    #[test]
    fn withdraw_exact_balance() {
        let mut sheet = BalanceSheet::new();
        sheet.deposit(TokenType::Storage, TokenAmount(500)).unwrap();
        sheet.withdraw(TokenType::Storage, TokenAmount(500)).unwrap();
        assert_eq!(sheet.get_balance(TokenType::Storage), TokenAmount::ZERO);
    }

    #[test]
    fn transfer_moves_tokens() {
        let mut from = BalanceSheet::new();
        let mut to = BalanceSheet::new();
        from.deposit(TokenType::Compute, TokenAmount(1000)).unwrap();

        from.transfer(&mut to, TokenType::Compute, TokenAmount(300)).unwrap();

        assert_eq!(from.get_balance(TokenType::Compute), TokenAmount(700));
        assert_eq!(to.get_balance(TokenType::Compute), TokenAmount(300));
    }

    #[test]
    fn transfer_insufficient_balance() {
        let mut from = BalanceSheet::new();
        let mut to = BalanceSheet::new();
        from.deposit(TokenType::Compute, TokenAmount(100)).unwrap();

        let result = from.transfer(&mut to, TokenType::Compute, TokenAmount(200));
        assert!(result.is_err());
        assert_eq!(from.get_balance(TokenType::Compute), TokenAmount(100));
        assert_eq!(to.get_balance(TokenType::Compute), TokenAmount::ZERO);
    }

    #[test]
    fn transfer_isolates_token_types() {
        let mut from = BalanceSheet::new();
        let mut to = BalanceSheet::new();
        from.deposit(TokenType::Compute, TokenAmount(1000)).unwrap();
        from.deposit(TokenType::Train, TokenAmount(500)).unwrap();

        from.transfer(&mut to, TokenType::Compute, TokenAmount(100)).unwrap();

        assert_eq!(from.get_balance(TokenType::Compute), TokenAmount(900));
        assert_eq!(from.get_balance(TokenType::Train), TokenAmount(500));
        assert_eq!(to.get_balance(TokenType::Compute), TokenAmount(100));
        assert_eq!(to.get_balance(TokenType::Train), TokenAmount::ZERO);
    }

    #[test]
    fn multi_token_isolation() {
        let mut sheet = BalanceSheet::new();
        sheet.deposit(TokenType::Compute, TokenAmount(100)).unwrap();
        sheet.deposit(TokenType::Train, TokenAmount(200)).unwrap();
        sheet.deposit(TokenType::Bandwidth, TokenAmount(300)).unwrap();
        sheet.deposit(TokenType::Storage, TokenAmount(400)).unwrap();

        sheet.withdraw(TokenType::Train, TokenAmount(50)).unwrap();

        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount(100));
        assert_eq!(sheet.get_balance(TokenType::Train), TokenAmount(150));
        assert_eq!(sheet.get_balance(TokenType::Bandwidth), TokenAmount(300));
        assert_eq!(sheet.get_balance(TokenType::Storage), TokenAmount(400));
    }

    #[test]
    fn total_value_sums_all() {
        let mut sheet = BalanceSheet::new();
        sheet.deposit(TokenType::Compute, TokenAmount(100)).unwrap();
        sheet.deposit(TokenType::Train, TokenAmount(200)).unwrap();
        sheet.deposit(TokenType::Bandwidth, TokenAmount(300)).unwrap();
        sheet.deposit(TokenType::Storage, TokenAmount(400)).unwrap();

        assert_eq!(sheet.total_value(), TokenAmount(1000));
    }

    #[test]
    fn total_value_saturating() {
        let mut sheet = BalanceSheet::new();
        sheet.deposit(TokenType::Compute, TokenAmount(u64::MAX)).unwrap();
        sheet.deposit(TokenType::Train, TokenAmount(u64::MAX)).unwrap();
        assert_eq!(sheet.total_value(), TokenAmount(u64::MAX));
    }

    #[test]
    fn total_value_empty_is_zero() {
        let sheet = BalanceSheet::new();
        assert_eq!(sheet.total_value(), TokenAmount::ZERO);
    }
}
