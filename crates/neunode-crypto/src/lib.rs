pub mod ed25519;
pub mod eip712;
pub mod hash;
pub mod secp256k1;

use thiserror::Error;

/// Errors returned by cryptographic operations.
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid signature")]
    InvalidSignature,
    #[error("invalid key: {0}")]
    InvalidKey(String),
    #[error("key generation failed: {0}")]
    KeyGenerationFailed(String),
    #[error("serialization error: {0}")]
    SerializationError(String),
}
