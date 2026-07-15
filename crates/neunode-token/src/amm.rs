use neunode_core::types::{TokenAmount, TokenType};
use ruint::aliases::U256;

use crate::error::{Result, TokenError};

pub const DEFAULT_SWAP_FEE_BPS: u16 = 30;
const BPS_DENOMINATOR: u128 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapResult {
    pub amount_in: TokenAmount,
    pub amount_out: TokenAmount,
    pub fee: TokenAmount,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantProductPool {
    token_a: TokenType,
    token_b: TokenType,
    reserve_a: TokenAmount,
    reserve_b: TokenAmount,
    fee_bps: u16,
}

impl ConstantProductPool {
    pub fn new(token_a: TokenType, token_b: TokenType) -> Result<Self> {
        if token_a == token_b {
            return Err(TokenError::IdenticalTokenPair);
        }
        Ok(Self {
            token_a,
            token_b,
            reserve_a: TokenAmount::ZERO,
            reserve_b: TokenAmount::ZERO,
            fee_bps: DEFAULT_SWAP_FEE_BPS,
        })
    }

    pub fn seed(&mut self, amount_a: TokenAmount, amount_b: TokenAmount) -> Result<()> {
        if self.is_initialized() {
            return Err(TokenError::PoolAlreadyInitialized);
        }
        if amount_a == TokenAmount::ZERO || amount_b == TokenAmount::ZERO {
            return Err(TokenError::ZeroAmount);
        }
        self.reserve_a = amount_a;
        self.reserve_b = amount_b;
        Ok(())
    }

    pub fn reserves(&self) -> (TokenAmount, TokenAmount) {
        (self.reserve_a, self.reserve_b)
    }

    pub fn is_initialized(&self) -> bool {
        self.reserve_a != TokenAmount::ZERO && self.reserve_b != TokenAmount::ZERO
    }

    pub fn quote_exact_input(
        &self,
        token_in: TokenType,
        amount_in: TokenAmount,
    ) -> Result<TokenAmount> {
        if !self.is_initialized() {
            return Err(TokenError::PoolNotInitialized);
        }
        if amount_in == TokenAmount::ZERO {
            return Err(TokenError::ZeroAmount);
        }
        let (reserve_in, reserve_out) = self.direction(token_in)?;
        let fee_multiplier = BPS_DENOMINATOR - u128::from(self.fee_bps);
        let amount_in_with_fee = U256::from(amount_in.0) * U256::from(fee_multiplier);
        let numerator = amount_in_with_fee * U256::from(reserve_out.0);
        let denominator =
            U256::from(reserve_in.0) * U256::from(BPS_DENOMINATOR) + amount_in_with_fee;
        let amount_out: u128 =
            (numerator / denominator).try_into().map_err(|_| TokenError::Overflow)?;
        if amount_out == 0 || amount_out >= reserve_out.0 {
            return Err(TokenError::InsufficientLiquidity);
        }
        Ok(TokenAmount(amount_out))
    }

    pub fn swap_exact_input(
        &mut self,
        token_in: TokenType,
        amount_in: TokenAmount,
        minimum_out: TokenAmount,
    ) -> Result<SwapResult> {
        let amount_out = self.quote_exact_input(token_in, amount_in)?;
        if amount_out < minimum_out {
            return Err(TokenError::SlippageExceeded { minimum: minimum_out, actual: amount_out });
        }

        let fee = TokenAmount(
            (U256::from(amount_in.0) * U256::from(self.fee_bps) / U256::from(BPS_DENOMINATOR))
                .try_into()
                .map_err(|_| TokenError::Overflow)?,
        );
        if token_in == self.token_a {
            self.reserve_a = self.reserve_a.checked_add(amount_in).ok_or(TokenError::Overflow)?;
            self.reserve_b = self.reserve_b.checked_sub(amount_out).ok_or(TokenError::Overflow)?;
        } else if token_in == self.token_b {
            self.reserve_b = self.reserve_b.checked_add(amount_in).ok_or(TokenError::Overflow)?;
            self.reserve_a = self.reserve_a.checked_sub(amount_out).ok_or(TokenError::Overflow)?;
        } else {
            return Err(TokenError::InvalidTokenType);
        }
        Ok(SwapResult { amount_in, amount_out, fee })
    }

    fn direction(&self, token_in: TokenType) -> Result<(TokenAmount, TokenAmount)> {
        if token_in == self.token_a {
            Ok((self.reserve_a, self.reserve_b))
        } else if token_in == self.token_b {
            Ok((self.reserve_b, self.reserve_a))
        } else {
            Err(TokenError::InvalidTokenType)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool() -> ConstantProductPool {
        let mut pool = ConstantProductPool::new(TokenType::Compute, TokenType::Storage).unwrap();
        pool.seed(TokenAmount(1_000_000), TokenAmount(1_000_000)).unwrap();
        pool
    }

    #[test]
    fn rejects_identical_pair_and_invalid_seed() {
        assert!(matches!(
            ConstantProductPool::new(TokenType::Compute, TokenType::Compute),
            Err(TokenError::IdenticalTokenPair)
        ));
        let mut pool = ConstantProductPool::new(TokenType::Compute, TokenType::Storage).unwrap();
        assert!(matches!(
            pool.seed(TokenAmount::ZERO, TokenAmount(1)),
            Err(TokenError::ZeroAmount)
        ));
    }

    #[test]
    fn quote_matches_constant_product_fee_formula() {
        assert_eq!(
            pool().quote_exact_input(TokenType::Compute, TokenAmount(10_000)).unwrap(),
            TokenAmount(9_871)
        );
    }

    #[test]
    fn swap_updates_reserves_and_never_decreases_invariant() {
        let mut pool = pool();
        let before = U256::from(1_000_000_u128) * U256::from(1_000_000_u128);
        let result = pool
            .swap_exact_input(TokenType::Compute, TokenAmount(10_000), TokenAmount(9_800))
            .unwrap();
        let (reserve_a, reserve_b) = pool.reserves();
        let after = U256::from(reserve_a.0) * U256::from(reserve_b.0);

        assert_eq!(result.amount_out, TokenAmount(9_871));
        assert_eq!(result.fee, TokenAmount(30));
        assert!(after >= before);
    }

    #[test]
    fn slippage_failure_is_atomic() {
        let mut pool = pool();
        let before = pool.reserves();
        assert!(matches!(
            pool.swap_exact_input(TokenType::Compute, TokenAmount(10_000), TokenAmount(9_900)),
            Err(TokenError::SlippageExceeded { .. })
        ));
        assert_eq!(pool.reserves(), before);
    }

    #[test]
    fn u256_intermediates_support_eighteen_decimal_reserves() {
        let mut pool = ConstantProductPool::new(TokenType::Train, TokenType::Bandwidth).unwrap();
        pool.seed(TokenAmount(10_u128.pow(30)), TokenAmount(5 * 10_u128.pow(29))).unwrap();
        assert!(
            pool.quote_exact_input(TokenType::Train, TokenAmount(10_u128.pow(24))).unwrap().0 > 0
        );
    }
}
