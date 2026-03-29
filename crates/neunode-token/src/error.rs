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

    #[error("arithmetic overflow")]
    Overflow,

    #[error("storage error: {0}")]
    StorageError(String),
}

/// Result type alias for token operations.
pub type Result<T> = std::result::Result<T, TokenError>;
