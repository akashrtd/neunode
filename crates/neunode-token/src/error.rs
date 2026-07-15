use neunode_core::types::TokenAmount;
use thiserror::Error;

/// Errors returned by token operations.
#[derive(Debug, Error)]
pub enum TokenError {
    #[error("insufficient balance: need {required}, have {available}")]
    InsufficientBalance { required: TokenAmount, available: TokenAmount },

    #[error("insufficient stake: need {required}, have {available}")]
    InsufficientStake { required: TokenAmount, available: TokenAmount },

    #[error("already staked for this token type")]
    AlreadyStaked,

    #[error("not staked for this token type")]
    NotStaked,

    #[error("unbonding already in progress")]
    UnbondingInProgress,

    #[error("invalid token type")]
    InvalidTokenType,

    #[error("a pool requires two distinct token types")]
    IdenticalTokenPair,

    #[error("liquidity pool is already initialized")]
    PoolAlreadyInitialized,

    #[error("liquidity pool is not initialized")]
    PoolNotInitialized,

    #[error("amount must be greater than zero")]
    ZeroAmount,

    #[error("swap output {actual} is below minimum {minimum}")]
    SlippageExceeded { minimum: TokenAmount, actual: TokenAmount },

    #[error("insufficient liquidity")]
    InsufficientLiquidity,

    #[error("arithmetic overflow")]
    Overflow,

    #[error("storage error: {0}")]
    StorageError(String),

    #[error("critical accounting invariant violation: rollback failed")]
    InvariantViolation,
}

/// Result type alias for token operations.
pub type Result<T> = std::result::Result<T, TokenError>;
