use thiserror::Error;

#[derive(Debug, Error)]
pub enum LineageError {
    #[error("cycle detected in lineage DAG: {0}")]
    CycleDetected(String),

    #[error("invalid signature for model {cid}")]
    InvalidSignature { cid: String },

    #[error("parent model not found: {0}")]
    ParentNotFound(String),

    #[error("model already registered: {0}")]
    AlreadyRegistered(String),

    #[error("model not found: {0}")]
    ModelNotFound(String),

    #[error("invalid content hash: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("empty DAG")]
    EmptyDag,

    #[error("invalid config: {0}")]
    ConfigInvalid(String),
}

pub type Result<T> = std::result::Result<T, LineageError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_cycle_detected() {
        let err = LineageError::CycleDetected("model-a → model-b → model-a".to_string());
        assert_eq!(format!("{err}"), "cycle detected in lineage DAG: model-a → model-b → model-a");
    }

    #[test]
    fn error_display_invalid_signature() {
        let err = LineageError::InvalidSignature { cid: "sha256:abc123".to_string() };
        assert_eq!(format!("{err}"), "invalid signature for model sha256:abc123");
    }

    #[test]
    fn error_display_parent_not_found() {
        let err = LineageError::ParentNotFound("sha256:missing".to_string());
        assert_eq!(format!("{err}"), "parent model not found: sha256:missing");
    }

    #[test]
    fn error_display_already_registered() {
        let err = LineageError::AlreadyRegistered("sha256:dup".to_string());
        assert_eq!(format!("{err}"), "model already registered: sha256:dup");
    }

    #[test]
    fn error_display_model_not_found() {
        let err = LineageError::ModelNotFound("sha256:gone".to_string());
        assert_eq!(format!("{err}"), "model not found: sha256:gone");
    }

    #[test]
    fn error_display_hash_mismatch() {
        let err =
            LineageError::HashMismatch { expected: "aaa".to_string(), actual: "bbb".to_string() };
        assert_eq!(format!("{err}"), "invalid content hash: expected aaa, got bbb");
    }

    #[test]
    fn error_display_empty_dag() {
        let err = LineageError::EmptyDag;
        assert_eq!(format!("{err}"), "empty DAG");
    }

    #[test]
    fn result_ok() {
        let res: u32 = 42;
        assert_eq!(res, 42);
    }

    #[test]
    fn result_err() {
        let res: Result<u32> = Err(LineageError::EmptyDag);
        assert!(res.is_err());
    }
}
