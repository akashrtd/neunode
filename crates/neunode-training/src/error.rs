use thiserror::Error;

/// Errors returned by training operations.
#[derive(Debug, Error)]
pub enum TrainingError {
    #[error("coordinator timeout: {0}")]
    CoordinatorTimeout(String),

    #[error("worker {worker_id} failed: {reason}")]
    WorkerFailed { worker_id: String, reason: String },

    #[error("gradient mismatch")]
    GradientMismatch,

    #[error("checkpoint corrupted: {0}")]
    CheckpointCorrupted(String),

    #[error("escrow error: {0}")]
    EscrowError(String),

    #[error("aggregation failed: {0}")]
    AggregationFailed(String),

    #[error("invalid config: {0}")]
    ConfigInvalid(String),

    #[error("peer unavailable: {0}")]
    PeerUnavailable(String),

    #[error("transfer failed: {0}")]
    TransferFailed(String),

    #[error("chunk {index} missing for CID {cid}")]
    ChunkMissing { index: u32, cid: String },

    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("transfer timeout: {0}")]
    TransferTimeout(String),

    #[error("manifest invalid: {0}")]
    ManifestInvalid(String),

    #[error("server unavailable: {0}")]
    ServerUnavailable(String),

    #[error("stale gradient from worker {worker_id}: staleness {staleness} > max {max_allowed}")]
    StaleGradient { worker_id: String, staleness: u32, max_allowed: u32 },

    #[error("quorum not reached: {available} workers available, {required} required")]
    QuorumNotReached { available: u32, required: u32 },

    #[error("collection timeout: {collected}/{required} workers after {elapsed_secs}s")]
    CollectionTimeout { collected: u32, required: u32, elapsed_secs: u64 },
}

/// Result type alias for training operations.
pub type Result<T> = std::result::Result<T, TrainingError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_coordinator_timeout() {
        let err = TrainingError::CoordinatorTimeout("sync deadline exceeded".to_string());
        assert_eq!(format!("{err}"), "coordinator timeout: sync deadline exceeded");
    }

    #[test]
    fn error_display_worker_failed() {
        let err = TrainingError::WorkerFailed {
            worker_id: "worker-7".to_string(),
            reason: "OOM".to_string(),
        };
        assert_eq!(format!("{err}"), "worker worker-7 failed: OOM");
    }

    #[test]
    fn error_display_gradient_mismatch() {
        let err = TrainingError::GradientMismatch;
        assert_eq!(format!("{err}"), "gradient mismatch");
    }

    #[test]
    fn error_display_checkpoint_corrupted() {
        let err = TrainingError::CheckpointCorrupted("sha256 mismatch".to_string());
        assert_eq!(format!("{err}"), "checkpoint corrupted: sha256 mismatch");
    }

    #[test]
    fn error_display_escrow_error() {
        let err = TrainingError::EscrowError("insufficient deposit".to_string());
        assert_eq!(format!("{err}"), "escrow error: insufficient deposit");
    }

    #[test]
    fn error_display_aggregation_failed() {
        let err = TrainingError::AggregationFailed("too few responses".to_string());
        assert_eq!(format!("{err}"), "aggregation failed: too few responses");
    }

    #[test]
    fn error_display_config_invalid() {
        let err = TrainingError::ConfigInvalid("local_steps must be > 0".to_string());
        assert_eq!(format!("{err}"), "invalid config: local_steps must be > 0");
    }

    #[test]
    fn error_display_peer_unavailable() {
        let err = TrainingError::PeerUnavailable("12D3Koo...".to_string());
        assert_eq!(format!("{err}"), "peer unavailable: 12D3Koo...");
    }

    #[test]
    fn error_display_transfer_failed() {
        let err = TrainingError::TransferFailed("connection reset".to_string());
        assert_eq!(format!("{err}"), "transfer failed: connection reset");
    }

    #[test]
    fn error_display_chunk_missing() {
        let err = TrainingError::ChunkMissing { index: 7, cid: "QmX7b...".to_string() };
        assert_eq!(format!("{err}"), "chunk 7 missing for CID QmX7b...");
    }

    #[test]
    fn error_display_hash_mismatch() {
        let err = TrainingError::HashMismatch {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        assert_eq!(format!("{err}"), "hash mismatch: expected abc123, got def456");
    }

    #[test]
    fn error_display_transfer_timeout() {
        let err = TrainingError::TransferTimeout("30s exceeded".to_string());
        assert_eq!(format!("{err}"), "transfer timeout: 30s exceeded");
    }

    #[test]
    fn error_display_manifest_invalid() {
        let err = TrainingError::ManifestInvalid("missing checksums".to_string());
        assert_eq!(format!("{err}"), "manifest invalid: missing checksums");
    }

    #[test]
    fn error_display_server_unavailable() {
        let err = TrainingError::ServerUnavailable("shard-3".to_string());
        assert_eq!(format!("{err}"), "server unavailable: shard-3");
    }

    #[test]
    fn error_display_stale_gradient() {
        let err = TrainingError::StaleGradient {
            worker_id: "worker-5".to_string(),
            staleness: 7,
            max_allowed: 3,
        };
        assert_eq!(format!("{err}"), "stale gradient from worker worker-5: staleness 7 > max 3");
    }

    #[test]
    fn error_display_quorum_not_reached() {
        let err = TrainingError::QuorumNotReached { available: 2, required: 4 };
        assert_eq!(format!("{err}"), "quorum not reached: 2 workers available, 4 required");
    }

    #[test]
    fn error_display_collection_timeout() {
        let err = TrainingError::CollectionTimeout { collected: 3, required: 5, elapsed_secs: 30 };
        assert_eq!(format!("{err}"), "collection timeout: 3/5 workers after 30s");
    }

    #[test]
    fn result_ok() {
        let res: u32 = 42;
        assert_eq!(res, 42);
    }

    #[test]
    fn result_err() {
        let res: Result<u32> = Err(TrainingError::GradientMismatch);
        assert!(res.is_err());
    }
}
