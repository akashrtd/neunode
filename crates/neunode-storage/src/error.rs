use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("RocksDB error: {0}")]
    RocksDb(#[from] rocksdb::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("column family not found: {0}")]
    ColumnFamilyNotFound(String),

    #[error("key not found in {cf}: {key}")]
    KeyNotFound { cf: String, key: String },

    #[error("cache error: {0}")]
    CacheError(String),

    #[error("invalid key format: {0}")]
    InvalidKeyFormat(String),

    #[error("sigchain verification failed at seq {seq}: {reason}")]
    SigchainVerificationFailed { seq: u64, reason: String },

    #[error("insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u128, available: u128 },

    #[error("insufficient staked balance: required {required}, available {available}")]
    InsufficientStakedBalance { required: u128, available: u128 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("token count mismatch: expected {expected}, got {got}")]
    TokenCountMismatch { expected: usize, got: usize },

    #[error("ledger mutation lock is poisoned")]
    LedgerLockPoisoned,

    #[error("atomic batch cannot span storage partitions")]
    CrossPartitionBatch,

    #[error("audit log verification failed at sequence {sequence}: {reason}")]
    AuditVerificationFailed { sequence: u64, reason: String },
}

pub type Result<T> = std::result::Result<T, StorageError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = StorageError::ColumnFamilyNotFound("test_cf".to_string());
        assert_eq!(format!("{err}"), "column family not found: test_cf");

        let err =
            StorageError::KeyNotFound { cf: "identity".to_string(), key: "did:test".to_string() };
        assert!(format!("{err}").contains("identity"));
        assert!(format!("{err}").contains("did:test"));

        let err = StorageError::InsufficientBalance { required: 100, available: 50 };
        assert!(format!("{err}").contains("100"));
        assert!(format!("{err}").contains("50"));

        let err = StorageError::SigchainVerificationFailed {
            seq: 5,
            reason: "hash mismatch".to_string(),
        };
        assert!(format!("{err}").contains("seq 5"));
        assert!(format!("{err}").contains("hash mismatch"));
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_ok() -> Result<String> {
            Ok("success".to_string())
        }
        assert_eq!(returns_ok().unwrap(), "success");
    }
}
