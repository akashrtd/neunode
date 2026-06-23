use thiserror::Error;

/// Errors that can occur during verification operations.
#[derive(Debug, Error)]
pub enum VerificationError {
    #[error("verification failed at {layer}: {reason}")]
    VerificationFailed { layer: String, reason: String },

    #[error("artifact hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("gauntlet test {test_name} failed: {reason}")]
    GauntletFailed { test_name: String, reason: String },

    #[error("reproducibility check failed: {0} operations disagree")]
    ReproducibilityFailed(u32),

    #[error("bisection failed: {0}")]
    BisectionFailed(String),

    #[error("TEE attestation failed: {0}")]
    TeeAttestationFailed(String),

    #[error("insufficient verifiers: {available} available, {required} required")]
    InsufficientVerifiers { available: u32, required: u32 },

    #[error("timeout: verification took {elapsed_secs}s, max {max_secs}s")]
    Timeout { elapsed_secs: u64, max_secs: u64 },

    #[error("invalid config: {0}")]
    ConfigInvalid(String),

    /// A verification layer is not available in this build/configuration.
    /// Distinct from `ConfigInvalid` (bad inputs) and the layer-specific failure
    /// variants (the check ran and failed): this means the check could not run at
    /// all. Lets callers degrade gracefully (e.g. fall back from ZK to RepOps).
    #[error("verification layer '{layer}' unsupported in this build: {reason}")]
    Unsupported { layer: String, reason: String },
}

pub type Result<T> = std::result::Result<T, VerificationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_failed_display() {
        let err = VerificationError::VerificationFailed {
            layer: "automated".to_string(),
            reason: "hash mismatch".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("automated"));
        assert!(msg.contains("hash mismatch"));
    }

    #[test]
    fn hash_mismatch_display() {
        let err = VerificationError::HashMismatch {
            expected: "abc".to_string(),
            actual: "def".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("abc"));
        assert!(msg.contains("def"));
    }

    #[test]
    fn gauntlet_failed_display() {
        let err = VerificationError::GauntletFailed {
            test_name: "test_1".to_string(),
            reason: "wrong output".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("test_1"));
        assert!(msg.contains("wrong output"));
    }

    #[test]
    fn reproducibility_failed_display() {
        let err = VerificationError::ReproducibilityFailed(5);
        assert!(err.to_string().contains("5"));
    }

    #[test]
    fn bisection_failed_display() {
        let err = VerificationError::BisectionFailed("no convergence".into());
        assert!(err.to_string().contains("no convergence"));
    }

    #[test]
    fn tee_attestation_failed_display() {
        let err = VerificationError::TeeAttestationFailed("invalid quote".into());
        assert!(err.to_string().contains("invalid quote"));
    }

    #[test]
    fn insufficient_verifiers_display() {
        let err = VerificationError::InsufficientVerifiers { available: 1, required: 3 };
        let msg = err.to_string();
        assert!(msg.contains("1"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn timeout_display() {
        let err = VerificationError::Timeout { elapsed_secs: 120, max_secs: 60 };
        let msg = err.to_string();
        assert!(msg.contains("120"));
        assert!(msg.contains("60"));
    }

    #[test]
    fn config_invalid_display() {
        let err = VerificationError::ConfigInvalid("bad rate".into());
        assert!(err.to_string().contains("bad rate"));
    }

    #[test]
    fn unsupported_display() {
        let err = VerificationError::Unsupported {
            layer: "zk".to_string(),
            reason: "not compiled in".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("zk"));
        assert!(msg.contains("not compiled in"));
        assert!(msg.contains("unsupported"));
    }

    #[test]
    fn result_ok() {
        let res: u32 = 42;
        assert_eq!(res, 42);
    }

    #[test]
    fn result_err() {
        let res: Result<u32> = Err(VerificationError::ConfigInvalid("x".into()));
        assert!(res.is_err());
    }
}
