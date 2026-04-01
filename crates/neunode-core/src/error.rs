use thiserror::Error;

/// Core error type for all Neunode operations.
#[derive(Error, Debug)]
pub enum NeunodeError {
    // Identity errors
    #[error("invalid DID: {0}")]
    InvalidDid(String),
    #[error("key not found: {0}")]
    KeyNotFound(String),
    #[error("signature verification failed")]
    SignatureVerificationFailed,
    #[error("key rotation failed: {0}")]
    KeyRotationFailed(String),
    #[error("key material corrupted")]
    KeyCorrupted,
    #[error("invalid public key: {0}")]
    InvalidPublicKey(String),

    // Feed errors
    #[error("invalid event: {0}")]
    InvalidEvent(String),
    #[error("sigchain broken at sequence {seq}")]
    SigchainBroken { seq: u64 },
    #[error("invalid kind: {0}")]
    InvalidKind(u16),
    #[error("schema validation failed: {0}")]
    SchemaValidationFailed(String),

    // Storage errors
    #[error("storage error: {0}")]
    StorageError(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),

    // Token errors
    #[error("insufficient balance: have {have}, need {need}")]
    InsufficientBalance { have: u64, need: u64 },
    #[error("staking failed: {0}")]
    StakingFailed(String),
    #[error("unbonding period not elapsed")]
    UnbondingPeriodNotElapsed,

    // Bounty errors
    #[error("invalid state transition: from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },
    #[error("timeout expired: {0}")]
    TimeoutExpired(String),
    #[error("escrow error: {0}")]
    EscrowError(String),

    // P2P errors
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("peer not found: {0}")]
    PeerNotFound(String),
    #[error("gossipsub error: {0}")]
    GossipsubError(String),

    // Config errors
    #[error("config error: {0}")]
    ConfigError(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    // Crypto errors
    #[error("crypto error: {0}")]
    CryptoError(String),
    #[error("encoding error: {0}")]
    EncodingError(String),

    // General
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, NeunodeError>;

#[cfg(test)]
mod tests {
    use super::NeunodeError;

    #[test]
    fn invalid_did_display() {
        let err = NeunodeError::InvalidDid("bad-did".to_string());
        assert_eq!(err.to_string(), "invalid DID: bad-did");
    }

    #[test]
    fn key_not_found_display() {
        let err = NeunodeError::KeyNotFound("ed25519".to_string());
        assert_eq!(err.to_string(), "key not found: ed25519");
    }

    #[test]
    fn signature_verification_failed_display() {
        let err = NeunodeError::SignatureVerificationFailed;
        assert_eq!(err.to_string(), "signature verification failed");
    }

    #[test]
    fn key_rotation_failed_display() {
        let err = NeunodeError::KeyRotationFailed("expired".to_string());
        assert_eq!(err.to_string(), "key rotation failed: expired");
    }

    #[test]
    fn key_corrupted_display() {
        let err = NeunodeError::KeyCorrupted;
        assert_eq!(err.to_string(), "key material corrupted");
    }

    #[test]
    fn invalid_public_key_display() {
        let err = NeunodeError::InvalidPublicKey("bad ed25519 key".to_string());
        assert_eq!(err.to_string(), "invalid public key: bad ed25519 key");
    }

    #[test]
    fn invalid_event_display() {
        let err = NeunodeError::InvalidEvent("missing signature".to_string());
        assert_eq!(err.to_string(), "invalid event: missing signature");
    }

    #[test]
    fn sigchain_broken_display() {
        let err = NeunodeError::SigchainBroken { seq: 42 };
        assert_eq!(err.to_string(), "sigchain broken at sequence 42");
    }

    #[test]
    fn invalid_kind_display() {
        let err = NeunodeError::InvalidKind(999);
        assert_eq!(err.to_string(), "invalid kind: 999");
    }

    #[test]
    fn schema_validation_failed_display() {
        let err = NeunodeError::SchemaValidationFailed("missing field".to_string());
        assert_eq!(err.to_string(), "schema validation failed: missing field");
    }

    #[test]
    fn storage_error_display() {
        let err = NeunodeError::StorageError("corruption".to_string());
        assert_eq!(err.to_string(), "storage error: corruption");
    }

    #[test]
    fn not_found_display() {
        let err = NeunodeError::NotFound("agent:123".to_string());
        assert_eq!(err.to_string(), "not found: agent:123");
    }

    #[test]
    fn already_exists_display() {
        let err = NeunodeError::AlreadyExists("did:neunode:abc".to_string());
        assert_eq!(err.to_string(), "already exists: did:neunode:abc");
    }

    #[test]
    fn insufficient_balance_display() {
        let err = NeunodeError::InsufficientBalance { have: 10, need: 50 };
        assert_eq!(err.to_string(), "insufficient balance: have 10, need 50");
    }

    #[test]
    fn staking_failed_display() {
        let err = NeunodeError::StakingFailed("below minimum".to_string());
        assert_eq!(err.to_string(), "staking failed: below minimum");
    }

    #[test]
    fn unbonding_period_not_elapsed_display() {
        let err = NeunodeError::UnbondingPeriodNotElapsed;
        assert_eq!(err.to_string(), "unbonding period not elapsed");
    }

    #[test]
    fn invalid_state_transition_display() {
        let err = NeunodeError::InvalidStateTransition {
            from: "Open".to_string(),
            to: "Paid".to_string(),
        };
        assert_eq!(err.to_string(), "invalid state transition: from Open to Paid");
    }

    #[test]
    fn timeout_expired_display() {
        let err = NeunodeError::TimeoutExpired("claim deadline".to_string());
        assert_eq!(err.to_string(), "timeout expired: claim deadline");
    }

    #[test]
    fn escrow_error_display() {
        let err = NeunodeError::EscrowError("insufficient deposit".to_string());
        assert_eq!(err.to_string(), "escrow error: insufficient deposit");
    }

    #[test]
    fn connection_failed_display() {
        let err = NeunodeError::ConnectionFailed("refused".to_string());
        assert_eq!(err.to_string(), "connection failed: refused");
    }

    #[test]
    fn peer_not_found_display() {
        let err = NeunodeError::PeerNotFound("12D3Koo...".to_string());
        assert_eq!(err.to_string(), "peer not found: 12D3Koo...");
    }

    #[test]
    fn gossipsub_error_display() {
        let err = NeunodeError::GossipsubError("topic full".to_string());
        assert_eq!(err.to_string(), "gossipsub error: topic full");
    }

    #[test]
    fn config_error_display() {
        let err = NeunodeError::ConfigError("missing field".to_string());
        assert_eq!(err.to_string(), "config error: missing field");
    }

    #[test]
    fn invalid_argument_display() {
        let err = NeunodeError::InvalidArgument("negative amount".to_string());
        assert_eq!(err.to_string(), "invalid argument: negative amount");
    }

    #[test]
    fn crypto_error_display() {
        let err = NeunodeError::CryptoError("invalid key".to_string());
        assert_eq!(err.to_string(), "crypto error: invalid key");
    }

    #[test]
    fn encoding_error_display() {
        let err = NeunodeError::EncodingError("invalid hex".to_string());
        assert_eq!(err.to_string(), "encoding error: invalid hex");
    }

    #[test]
    fn io_error_display() {
        let err =
            NeunodeError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "file gone"));
        assert!(err.to_string().contains("file gone"));
    }

    #[test]
    fn serialization_error_display() {
        let err = NeunodeError::SerializationError("json parse".to_string());
        assert_eq!(err.to_string(), "serialization error: json parse");
    }

    #[test]
    fn error_debug_formats_correctly() {
        let err = NeunodeError::InvalidKind(42);
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidKind"));
        assert!(debug_str.contains("42"));
    }

    #[test]
    fn result_type_ok() {
        let res: super::Result<u32> = Ok(42);
        assert_eq!(res.unwrap(), 42);
    }

    #[test]
    fn result_type_err() {
        let res: super::Result<u32> = Err(NeunodeError::NotFound("x".to_string()));
        assert!(res.is_err());
    }
}
