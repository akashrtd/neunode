use thiserror::Error;

/// Errors that can occur during bounty operations.
#[derive(Debug, Error)]
pub enum BountyError {
    #[error("invalid state transition from {from:?} via {event} to {to:?}")]
    InvalidTransition {
        from: neunode_core::BountyState,
        event: String,
        to: neunode_core::BountyState,
    },

    #[error("invalid bounty state: {0:?}")]
    InvalidState(neunode_core::BountyState),

    #[error("deadline exceeded: {deadline_type} (deadline={deadline}, now={now})")]
    DeadlineExceeded {
        deadline_type: String,
        deadline: neunode_core::Timestamp,
        now: neunode_core::Timestamp,
    },

    #[error("insufficient bond: required={required}, provided={provided}")]
    InsufficientBond { required: neunode_core::TokenAmount, provided: neunode_core::TokenAmount },

    #[error("insufficient funds: required={required}, available={available}")]
    InsufficientFunds { required: neunode_core::TokenAmount, available: neunode_core::TokenAmount },

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("bounty already claimed by {0}")]
    AlreadyClaimed(String),

    #[error("bounty not claimed")]
    NotClaimed,

    #[error("review incomplete: {submitted}/{required} reviews submitted")]
    ReviewIncomplete { submitted: usize, required: usize },

    #[error("verification failed at layer {layer}: {reason}")]
    VerificationFailed { layer: usize, reason: String },

    #[error("escrow error: {0}")]
    EscrowError(String),

    #[error("storage error: {0}")]
    StorageError(String),

    #[error("bounty not found: {0}")]
    NotFound(String),

    #[error("bounty is in terminal state: {0:?}")]
    TerminalState(neunode_core::BountyState),

    #[error("bounty already exists: {0}")]
    AlreadyExists(String),

    #[error("duplicate reviewer: {0}")]
    DuplicateReviewer(String),

    #[error("reviewer not on committee: {0}")]
    ReviewerNotOnCommittee(String),

    #[error("invalid score: {0} (must be 0-100)")]
    InvalidScore(u8),
}

pub type Result<T> = std::result::Result<T, BountyError>;
