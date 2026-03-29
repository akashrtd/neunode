use std::collections::HashMap;

use neunode_core::types::{TokenAmount, TokenType};

use crate::balance::BalanceSheet;
use crate::error::{Result, TokenError};

pub fn mint(balances: &mut BalanceSheet, token_type: TokenType, amount: TokenAmount) -> Result<()> {
    if amount == TokenAmount::ZERO {
        return Ok(());
    }
    balances.deposit(token_type, amount)
}

pub fn burn(balances: &mut BalanceSheet, token_type: TokenType, amount: TokenAmount) -> Result<()> {
    if amount == TokenAmount::ZERO {
        return Ok(());
    }
    balances.withdraw(token_type, amount)
}

pub fn mint_batch(balances: &mut BalanceSheet, mints: &[(TokenType, TokenAmount)]) -> Result<()> {
    // Validate accumulated amounts per type to avoid partial application
    let mut running: HashMap<TokenType, TokenAmount> = HashMap::new();
    for &(token_type, amount) in mints {
        let prev = running.get(&token_type).copied().unwrap_or(TokenAmount::ZERO);
        let added = prev.checked_add(amount).ok_or(TokenError::Overflow)?;
        let current = balances.get_balance(token_type);
        current.checked_add(added).ok_or(TokenError::Overflow)?;
        running.insert(token_type, added);
    }

    for &(token_type, amount) in mints {
        balances.deposit(token_type, amount)?;
    }
    Ok(())
}

pub fn burn_batch(balances: &mut BalanceSheet, burns: &[(TokenType, TokenAmount)]) -> Result<()> {
    let mut running: HashMap<TokenType, TokenAmount> = HashMap::new();
    for &(token_type, amount) in burns {
        let prev = running.get(&token_type).copied().unwrap_or(TokenAmount::ZERO);
        let added = prev.checked_add(amount).ok_or(TokenError::Overflow)?;
        running.insert(token_type, added);
    }
    for (&token_type, &total_burn) in &running {
        let current = balances.get_balance(token_type);
        if current < total_burn {
            return Err(TokenError::InsufficientBalance {
                required: total_burn,
                available: current,
            });
        }
    }

    for &(token_type, amount) in burns {
        balances.withdraw(token_type, amount)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_adds_tokens() {
        let mut sheet = BalanceSheet::new();
        mint(&mut sheet, TokenType::Compute, TokenAmount(500)).unwrap();
        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount(500));
    }

    #[test]
    fn mint_accumulates() {
        let mut sheet = BalanceSheet::new();
        mint(&mut sheet, TokenType::Train, TokenAmount(100)).unwrap();
        mint(&mut sheet, TokenType::Train, TokenAmount(200)).unwrap();
        assert_eq!(sheet.get_balance(TokenType::Train), TokenAmount(300));
    }

    #[test]
    fn mint_zero_is_noop() {
        let mut sheet = BalanceSheet::new();
        mint(&mut sheet, TokenType::Compute, TokenAmount::ZERO).unwrap();
        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount::ZERO);
    }

    #[test]
    fn mint_overflow() {
        let mut sheet = BalanceSheet::new();
        mint(&mut sheet, TokenType::Compute, TokenAmount(u64::MAX)).unwrap();
        let result = mint(&mut sheet, TokenType::Compute, TokenAmount(1));
        assert!(matches!(result, Err(TokenError::Overflow)));
    }

    #[test]
    fn burn_removes_tokens() {
        let mut sheet = BalanceSheet::new();
        mint(&mut sheet, TokenType::Bandwidth, TokenAmount(1000)).unwrap();
        burn(&mut sheet, TokenType::Bandwidth, TokenAmount(300)).unwrap();
        assert_eq!(sheet.get_balance(TokenType::Bandwidth), TokenAmount(700));
    }

    #[test]
    fn burn_exact_balance() {
        let mut sheet = BalanceSheet::new();
        mint(&mut sheet, TokenType::Storage, TokenAmount(500)).unwrap();
        burn(&mut sheet, TokenType::Storage, TokenAmount(500)).unwrap();
        assert_eq!(sheet.get_balance(TokenType::Storage), TokenAmount::ZERO);
    }

    #[test]
    fn burn_more_than_balance() {
        let mut sheet = BalanceSheet::new();
        mint(&mut sheet, TokenType::Compute, TokenAmount(100)).unwrap();
        let result = burn(&mut sheet, TokenType::Compute, TokenAmount(200));
        assert!(matches!(
            result,
            Err(TokenError::InsufficientBalance {
                required: TokenAmount(200),
                available: TokenAmount(100),
            })
        ));
        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount(100));
    }

    #[test]
    fn burn_zero_is_noop() {
        let mut sheet = BalanceSheet::new();
        mint(&mut sheet, TokenType::Compute, TokenAmount(500)).unwrap();
        burn(&mut sheet, TokenType::Compute, TokenAmount::ZERO).unwrap();
        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount(500));
    }

    #[test]
    fn mint_batch_multiple_types() {
        let mut sheet = BalanceSheet::new();
        mint_batch(
            &mut sheet,
            &[
                (TokenType::Compute, TokenAmount(100)),
                (TokenType::Train, TokenAmount(200)),
                (TokenType::Bandwidth, TokenAmount(300)),
            ],
        )
        .unwrap();

        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount(100));
        assert_eq!(sheet.get_balance(TokenType::Train), TokenAmount(200));
        assert_eq!(sheet.get_balance(TokenType::Bandwidth), TokenAmount(300));
    }

    #[test]
    fn mint_batch_same_type_accumulates() {
        let mut sheet = BalanceSheet::new();
        mint_batch(
            &mut sheet,
            &[(TokenType::Compute, TokenAmount(100)), (TokenType::Compute, TokenAmount(200))],
        )
        .unwrap();
        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount(300));
    }

    #[test]
    fn mint_batch_overflow_no_partial() {
        let mut sheet = BalanceSheet::new();
        let result = mint_batch(
            &mut sheet,
            &[(TokenType::Compute, TokenAmount(u64::MAX)), (TokenType::Compute, TokenAmount(1))],
        );
        assert!(matches!(result, Err(TokenError::Overflow)));
        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount::ZERO);
    }

    #[test]
    fn burn_batch_multiple_types() {
        let mut sheet = BalanceSheet::new();
        mint_batch(
            &mut sheet,
            &[(TokenType::Compute, TokenAmount(500)), (TokenType::Train, TokenAmount(500))],
        )
        .unwrap();

        burn_batch(
            &mut sheet,
            &[(TokenType::Compute, TokenAmount(100)), (TokenType::Train, TokenAmount(200))],
        )
        .unwrap();

        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount(400));
        assert_eq!(sheet.get_balance(TokenType::Train), TokenAmount(300));
    }

    #[test]
    fn burn_batch_insufficient_no_partial() {
        let mut sheet = BalanceSheet::new();
        mint(&mut sheet, TokenType::Compute, TokenAmount(100)).unwrap();

        let result = burn_batch(
            &mut sheet,
            &[(TokenType::Compute, TokenAmount(50)), (TokenType::Train, TokenAmount(200))],
        );
        assert!(matches!(result, Err(TokenError::InsufficientBalance { .. })));
        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount(100));
    }

    #[test]
    fn mint_batch_empty() {
        let mut sheet = BalanceSheet::new();
        mint_batch(&mut sheet, &[]).unwrap();
        assert_eq!(sheet.total_value(), TokenAmount::ZERO);
    }

    #[test]
    fn burn_batch_empty() {
        let mut sheet = BalanceSheet::new();
        burn_batch(&mut sheet, &[]).unwrap();
    }

    #[test]
    fn burn_from_empty_balance() {
        let mut sheet = BalanceSheet::new();
        let result = burn(&mut sheet, TokenType::Compute, TokenAmount(1));
        assert!(matches!(result, Err(TokenError::InsufficientBalance { .. })));
    }

    #[test]
    fn mint_burn_roundtrip() {
        let mut sheet = BalanceSheet::new();
        mint(&mut sheet, TokenType::Compute, TokenAmount(1000)).unwrap();
        burn(&mut sheet, TokenType::Compute, TokenAmount(600)).unwrap();
        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount(400));
    }

    #[test]
    fn batch_mint_burn_roundtrip() {
        let mut sheet = BalanceSheet::new();
        mint_batch(
            &mut sheet,
            &[(TokenType::Compute, TokenAmount(100)), (TokenType::Train, TokenAmount(200))],
        )
        .unwrap();

        burn_batch(
            &mut sheet,
            &[(TokenType::Compute, TokenAmount(50)), (TokenType::Train, TokenAmount(100))],
        )
        .unwrap();

        assert_eq!(sheet.get_balance(TokenType::Compute), TokenAmount(50));
        assert_eq!(sheet.get_balance(TokenType::Train), TokenAmount(100));
    }
}
