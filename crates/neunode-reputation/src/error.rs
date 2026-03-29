use thiserror::Error;

/// Errors returned by reputation operations.
#[derive(Debug, Error)]
pub enum ReputationError {
    #[error("invalid attestation: {0}")]
    InvalidAttestation(String),

    #[error("self-attestation not allowed: agent {0}")]
    SelfAttestation(String),

    #[error("max attestation depth exceeded: {0} > {1}")]
    MaxDepthExceeded(usize, usize),

    #[error("invalid score: {0} (must be 0-100)")]
    InvalidScore(f64),

    #[error("storage error: {0}")]
    StorageError(String),

    #[error("invalid signer: {0}")]
    InvalidSigner(String),
}

/// Result type alias for reputation operations.
pub type Result<T> = std::result::Result<T, ReputationError>;
