use thiserror::Error;

/// Errors returned by feed operations.
#[derive(Error, Debug)]
pub enum FeedError {
    #[error("invalid event: {0}")]
    InvalidEvent(String),

    #[error("invalid signature: {0}")]
    InvalidSignature(String),

    #[error("invalid sequence: expected {expected}, got {actual}")]
    InvalidSequence { expected: u64, actual: u64 },

    #[error("hash chain broken at sequence {seq}")]
    HashChainBroken { seq: u64 },

    #[error("content too large: {size} bytes, max {max}")]
    ContentTooLarge { size: usize, max: usize },

    #[error("too many tags: {count}, max {max}")]
    TooManyTags { count: usize, max: usize },

    #[error("too many refs: {count}, max {max}")]
    TooManyRefs { count: usize, max: usize },

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("storage error: {0}")]
    StorageError(String),

    #[error("schema validation error: {0}")]
    SchemaValidationError(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("fork detected at sequence {seq}: local hash {local}, incoming hash {incoming}")]
    ForkDetected { seq: u64, local: String, incoming: String },
}

pub type Result<T> = std::result::Result<T, FeedError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_event_display() {
        let err = FeedError::InvalidEvent("missing field".to_string());
        assert_eq!(err.to_string(), "invalid event: missing field");
    }

    #[test]
    fn invalid_signature_display() {
        let err = FeedError::InvalidSignature("bad sig".to_string());
        assert_eq!(err.to_string(), "invalid signature: bad sig");
    }

    #[test]
    fn invalid_sequence_display() {
        let err = FeedError::InvalidSequence { expected: 5, actual: 3 };
        assert_eq!(err.to_string(), "invalid sequence: expected 5, got 3");
    }

    #[test]
    fn hash_chain_broken_display() {
        let err = FeedError::HashChainBroken { seq: 42 };
        assert_eq!(err.to_string(), "hash chain broken at sequence 42");
    }

    #[test]
    fn content_too_large_display() {
        let err = FeedError::ContentTooLarge { size: 2_000_000, max: 1_048_576 };
        assert_eq!(err.to_string(), "content too large: 2000000 bytes, max 1048576");
    }

    #[test]
    fn too_many_tags_display() {
        let err = FeedError::TooManyTags { count: 150, max: 100 };
        assert_eq!(err.to_string(), "too many tags: 150, max 100");
    }

    #[test]
    fn too_many_refs_display() {
        let err = FeedError::TooManyRefs { count: 60, max: 50 };
        assert_eq!(err.to_string(), "too many refs: 60, max 50");
    }

    #[test]
    fn serialization_error_display() {
        let err = FeedError::SerializationError("json parse failed".to_string());
        assert_eq!(err.to_string(), "serialization error: json parse failed");
    }

    #[test]
    fn storage_error_display() {
        let err = FeedError::StorageError("corruption".to_string());
        assert_eq!(err.to_string(), "storage error: corruption");
    }

    #[test]
    fn schema_validation_error_display() {
        let err = FeedError::SchemaValidationError("missing title".to_string());
        assert_eq!(err.to_string(), "schema validation error: missing title");
    }

    #[test]
    fn result_type_ok() {
        let res: u32 = 42;
        assert_eq!(res, 42);
    }

    #[test]
    fn result_type_err() {
        let res: Result<u32> = Err(FeedError::InvalidEvent("test".to_string()));
        assert!(res.is_err());
    }

    #[test]
    fn error_debug_format() {
        let err = FeedError::HashChainBroken { seq: 7 };
        let debug = format!("{:?}", err);
        assert!(debug.contains("HashChainBroken"));
    }
}
