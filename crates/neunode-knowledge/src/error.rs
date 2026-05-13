use thiserror::Error;

/// Errors returned by knowledge graph operations.
#[derive(Debug, Error)]
pub enum KnowledgeError {
    #[error("dictionary miss: {0}")]
    DictionaryMiss(String),

    #[error("invalid triple: {0}")]
    InvalidTriple(String),

    #[error("query failed: {0}")]
    QueryFailed(String),

    #[error("index corrupted: {0}")]
    IndexCorrupted(String),

    #[error("cache miss: {0}")]
    CacheMiss(String),

    #[error("storage error: {0}")]
    StorageError(String),

    #[error("authorization error: {0}")]
    AuthorizationError(String),
}

/// Result type alias for knowledge graph operations.
pub type Result<T> = std::result::Result<T, KnowledgeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_dictionary_miss() {
        let err = KnowledgeError::DictionaryMiss("key not found".to_string());
        assert_eq!(format!("{err}"), "dictionary miss: key not found");
    }

    #[test]
    fn error_display_invalid_triple() {
        let err = KnowledgeError::InvalidTriple("missing predicate".to_string());
        assert_eq!(format!("{err}"), "invalid triple: missing predicate");
    }

    #[test]
    fn error_display_query_failed() {
        let err = KnowledgeError::QueryFailed("join overflow".to_string());
        assert_eq!(format!("{err}"), "query failed: join overflow");
    }

    #[test]
    fn error_display_index_corrupted() {
        let err = KnowledgeError::IndexCorrupted("bad checksum".to_string());
        assert_eq!(format!("{err}"), "index corrupted: bad checksum");
    }

    #[test]
    fn error_display_cache_miss() {
        let err = KnowledgeError::CacheMiss("entry expired".to_string());
        assert_eq!(format!("{err}"), "cache miss: entry expired");
    }

    #[test]
    fn error_display_storage_error() {
        let err = KnowledgeError::StorageError("write failed".to_string());
        assert_eq!(format!("{err}"), "storage error: write failed");
    }

    #[test]
    fn error_display_authorization_error() {
        let err = KnowledgeError::AuthorizationError("bad signature".to_string());
        assert_eq!(format!("{err}"), "authorization error: bad signature");
    }

    #[test]
    fn result_ok() {
        let res: u32 = 42;
        assert_eq!(res, 42);
    }

    #[test]
    fn result_err() {
        let res: Result<u32> = Err(KnowledgeError::QueryFailed("timeout".to_string()));
        assert!(res.is_err());
    }
}
