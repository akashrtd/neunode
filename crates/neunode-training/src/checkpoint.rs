use neunode_core::types::{Did, CID};
use neunode_storage::cf::CF_TRAINING;
use neunode_storage::db::NeunodeDb;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ts_rs::TS;

use crate::error::{Result, TrainingError};

/// Metadata for a training checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CheckpointMeta {
    /// CID of the checkpoint data (weights + optimizer state).
    pub cid: CID,
    /// Training job ID this checkpoint belongs to.
    pub job_id: String,
    /// DID of the worker that created this checkpoint.
    pub worker_did: Did,
    /// Outer step number when checkpoint was taken.
    #[ts(type = "number")]
    pub outer_step: u32,
    /// Loss at checkpoint time.
    pub loss: f64,
    /// Timestamp (millis since epoch).
    #[ts(type = "number")]
    pub timestamp_ms: u64,
    /// Size of checkpoint in bytes.
    #[ts(type = "number")]
    pub size_bytes: u64,
    /// Number of workers that contributed to this checkpoint.
    #[ts(type = "number")]
    pub num_workers: u32,
}

/// Manages training checkpoint metadata in RocksDB.
pub struct CheckpointStore<'a> {
    db: &'a NeunodeDb,
}

impl<'a> CheckpointStore<'a> {
    pub fn new(db: &'a NeunodeDb) -> Self {
        Self { db }
    }

    /// Save checkpoint metadata.
    /// Key: raw bytes "ckpt:{job_id}:{outer_step}"
    /// Value: serde_json-encoded CheckpointMeta
    pub fn save(&self, meta: &CheckpointMeta) -> Result<()> {
        let key = checkpoint_key_bytes(&meta.job_id, meta.outer_step);
        let value = serde_json::to_vec(meta)
            .map_err(|e| TrainingError::CheckpointCorrupted(e.to_string()))?;
        self.db.put_raw(CF_TRAINING, &key, &value).map_err(storage_err)?;
        Ok(())
    }

    /// Load checkpoint metadata by job_id and outer_step.
    pub fn load(&self, job_id: &str, outer_step: u32) -> Result<Option<CheckpointMeta>> {
        let key = checkpoint_key_bytes(job_id, outer_step);
        match self.db.get_raw(CF_TRAINING, &key).map_err(storage_err)? {
            Some(bytes) => {
                let meta: CheckpointMeta = serde_json::from_slice(&bytes)
                    .map_err(|e| TrainingError::CheckpointCorrupted(e.to_string()))?;
                Ok(Some(meta))
            }
            None => Ok(None),
        }
    }

    /// Load the latest checkpoint for a job (last entry by key order).
    pub fn latest(&self, job_id: &str) -> Result<Option<CheckpointMeta>> {
        let prefix = job_prefix(job_id);
        let entries = self.db.prefix_scan(CF_TRAINING, &prefix).map_err(storage_err)?;
        match entries.into_iter().last() {
            Some((_, v)) => {
                let meta: CheckpointMeta = serde_json::from_slice(&v)
                    .map_err(|e| TrainingError::CheckpointCorrupted(e.to_string()))?;
                Ok(Some(meta))
            }
            None => Ok(None),
        }
    }

    /// List all checkpoints for a job, ordered by outer_step.
    pub fn list(&self, job_id: &str) -> Result<Vec<CheckpointMeta>> {
        let prefix = job_prefix(job_id);
        let entries = self.db.prefix_scan(CF_TRAINING, &prefix).map_err(storage_err)?;
        let mut checkpoints: Vec<CheckpointMeta> =
            entries.into_iter().filter_map(|(_, v)| serde_json::from_slice(&v).ok()).collect();
        checkpoints.sort_by_key(|c| c.outer_step);
        Ok(checkpoints)
    }

    /// Delete a specific checkpoint.
    pub fn delete(&self, job_id: &str, outer_step: u32) -> Result<()> {
        let key = checkpoint_key_bytes(job_id, outer_step);
        self.db.delete(CF_TRAINING, &key).map_err(storage_err)
    }

    /// Delete all checkpoints for a job. Returns count deleted.
    pub fn delete_all(&self, job_id: &str) -> Result<usize> {
        let prefix = job_prefix(job_id);
        let entries = self.db.prefix_scan(CF_TRAINING, &prefix).map_err(storage_err)?;
        let count = entries.len();
        for (key, _) in &entries {
            self.db.delete(CF_TRAINING, key).map_err(storage_err)?;
        }
        Ok(count)
    }

    /// Check if a checkpoint exists.
    pub fn exists(&self, job_id: &str, outer_step: u32) -> Result<bool> {
        Ok(self.load(job_id, outer_step)?.is_some())
    }
}

/// Compute a content-addressed CID from checkpoint data using BLAKE3.
pub fn compute_cid(data: &[u8]) -> CID {
    let hash = blake3::hash(data);
    let hex = hex::encode(hash.as_bytes());
    CID::from_blake3_hex(&hex)
}

/// Get the local filesystem path for a checkpoint blob.
pub fn blob_path(base_dir: &Path, cid: &CID) -> PathBuf {
    base_dir.join("checkpoints").join(format!("{}.bin", cid.0.replace(':', "_")))
}

/// Store checkpoint bytes to local disk.
pub fn store_blob(base_dir: &Path, cid: &CID, data: &[u8]) -> Result<()> {
    let path = blob_path(base_dir, cid);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| TrainingError::CheckpointCorrupted(e.to_string()))?;
    }
    std::fs::write(&path, data).map_err(|e| TrainingError::CheckpointCorrupted(e.to_string()))
}

/// Load checkpoint bytes from local disk.
pub fn load_blob(base_dir: &Path, cid: &CID) -> Result<Vec<u8>> {
    let path = blob_path(base_dir, cid);
    std::fs::read(&path).map_err(|e| TrainingError::CheckpointCorrupted(e.to_string()))
}

/// Check if a checkpoint blob exists on local disk.
pub fn blob_exists(base_dir: &Path, cid: &CID) -> bool {
    blob_path(base_dir, cid).exists()
}

fn storage_err(e: neunode_storage::error::StorageError) -> TrainingError {
    TrainingError::CheckpointCorrupted(e.to_string())
}

/// Build the storage key for a checkpoint as raw bytes.
fn checkpoint_key(job_id: &str, outer_step: u32) -> String {
    format!("ckpt:{job_id}:{outer_step}")
}

fn checkpoint_key_bytes(job_id: &str, outer_step: u32) -> Vec<u8> {
    checkpoint_key(job_id, outer_step).into_bytes()
}

/// Build the prefix for scanning all checkpoints of a job.
fn job_prefix(job_id: &str) -> Vec<u8> {
    format!("ckpt:{job_id}:").into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_db() -> NeunodeDb {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "neunode_training_ckpt_{:?}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&dir);
        NeunodeDb::open(&dir).unwrap()
    }

    fn test_meta(step: u32) -> CheckpointMeta {
        CheckpointMeta {
            cid: CID(format!("bafkrei_step_{step}")),
            job_id: "test-job".to_string(),
            worker_did: Did("did:neunode:0xABC".to_string()),
            outer_step: step,
            loss: 1.0 / (step as f64 + 1.0),
            timestamp_ms: 1000 + step as u64 * 100,
            size_bytes: 14_000_000_000,
            num_workers: 8,
        }
    }

    fn test_meta_for_job(job_id: &str, step: u32) -> CheckpointMeta {
        let mut m = test_meta(step);
        m.job_id = job_id.to_string();
        m
    }

    #[test]
    fn checkpoint_key_format() {
        assert_eq!(checkpoint_key("job123", 42), "ckpt:job123:42");
    }

    #[test]
    fn job_prefix_format() {
        assert_eq!(job_prefix("job123"), b"ckpt:job123:");
    }

    #[test]
    fn save_and_load() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        let meta = test_meta(100);
        store.save(&meta).unwrap();

        let loaded = store.load("test-job", 100).unwrap().unwrap();
        assert_eq!(loaded.cid.0, meta.cid.0);
        assert_eq!(loaded.job_id, meta.job_id);
        assert_eq!(loaded.outer_step, meta.outer_step);
        assert!((loaded.loss - meta.loss).abs() < f64::EPSILON);
        assert_eq!(loaded.timestamp_ms, meta.timestamp_ms);
        assert_eq!(loaded.size_bytes, meta.size_bytes);
        assert_eq!(loaded.num_workers, meta.num_workers);
    }

    #[test]
    fn load_missing() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        assert!(store.load("nonexistent", 1).unwrap().is_none());
    }

    #[test]
    fn save_and_exists() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        store.save(&test_meta(50)).unwrap();
        assert!(store.exists("test-job", 50).unwrap());
    }

    #[test]
    fn exists_missing() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        assert!(!store.exists("ghost", 99).unwrap());
    }

    #[test]
    fn latest_single() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        store.save(&test_meta(10)).unwrap();
        let latest = store.latest("test-job").unwrap().unwrap();
        assert_eq!(latest.outer_step, 10);
    }

    #[test]
    fn latest_multiple() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        store.save(&test_meta(10)).unwrap();
        store.save(&test_meta(20)).unwrap();
        store.save(&test_meta(30)).unwrap();
        let latest = store.latest("test-job").unwrap().unwrap();
        assert_eq!(latest.outer_step, 30);
    }

    #[test]
    fn list_ordered() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        store.save(&test_meta(30)).unwrap();
        store.save(&test_meta(10)).unwrap();
        store.save(&test_meta(20)).unwrap();
        let list = store.list("test-job").unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].outer_step, 10);
        assert_eq!(list[1].outer_step, 20);
        assert_eq!(list[2].outer_step, 30);
    }

    #[test]
    fn list_empty() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        let list = store.list("no-such-job").unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn delete_specific() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        store.save(&test_meta(10)).unwrap();
        store.save(&test_meta(20)).unwrap();

        store.delete("test-job", 10).unwrap();
        assert!(store.load("test-job", 10).unwrap().is_none());
        assert!(store.load("test-job", 20).unwrap().is_some());
    }

    #[test]
    fn delete_missing() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        assert!(store.delete("ghost", 999).is_ok());
    }

    #[test]
    fn delete_all() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        store.save(&test_meta(10)).unwrap();
        store.save(&test_meta(20)).unwrap();
        store.save(&test_meta(30)).unwrap();

        let count = store.delete_all("test-job").unwrap();
        assert_eq!(count, 3);
        assert!(store.list("test-job").unwrap().is_empty());
    }

    #[test]
    fn delete_all_empty() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        let count = store.delete_all("nothing-here").unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn checkpoint_meta_serde() {
        let meta = test_meta(42);
        let json = serde_json::to_string(&meta).unwrap();
        let back: CheckpointMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back.cid.0, meta.cid.0);
        assert_eq!(back.outer_step, meta.outer_step);
        assert_eq!(back.worker_did.0, meta.worker_did.0);
        assert!((back.loss - meta.loss).abs() < f64::EPSILON);
    }

    #[test]
    fn save_overwrite() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        let mut meta = test_meta(10);
        meta.loss = 5.0;
        store.save(&meta).unwrap();

        meta.loss = 0.1;
        store.save(&meta).unwrap();

        let loaded = store.load("test-job", 10).unwrap().unwrap();
        assert!((loaded.loss - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn multiple_jobs() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        store.save(&test_meta_for_job("job-a", 10)).unwrap();
        store.save(&test_meta_for_job("job-a", 20)).unwrap();
        store.save(&test_meta_for_job("job-b", 10)).unwrap();

        let list_a = store.list("job-a").unwrap();
        assert_eq!(list_a.len(), 2);

        let list_b = store.list("job-b").unwrap();
        assert_eq!(list_b.len(), 1);
        assert_eq!(list_b[0].job_id, "job-b");

        assert!(store.list("job-c").unwrap().is_empty());
    }

    #[test]
    fn ts_export_checkpoint_meta() {
        use ts_rs::Config;
        let cfg = Config::new();
        let name = CheckpointMeta::name(&cfg);
        assert!(!name.is_empty());
    }

    #[test]
    fn checkpoint_with_large_step() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        let mut meta = test_meta(u32::MAX);
        meta.job_id = "big-step-job".to_string();
        store.save(&meta).unwrap();

        let loaded = store.load("big-step-job", u32::MAX).unwrap().unwrap();
        assert_eq!(loaded.outer_step, u32::MAX);
    }

    #[test]
    fn checkpoint_with_zero_loss() {
        let db = temp_db();
        let store = CheckpointStore::new(&db);
        let mut meta = test_meta(5);
        meta.loss = 0.0;
        meta.job_id = "zero-loss".to_string();
        store.save(&meta).unwrap();

        let loaded = store.load("zero-loss", 5).unwrap().unwrap();
        assert_eq!(loaded.loss, 0.0);
    }

    #[test]
    fn compute_cid_deterministic() {
        let data = b"checkpoint weights data";
        let cid1 = compute_cid(data);
        let cid2 = compute_cid(data);
        assert_eq!(cid1, cid2);
        assert!(cid1.is_blake3());
    }

    #[test]
    fn compute_cid_different_data() {
        let cid1 = compute_cid(b"data1");
        let cid2 = compute_cid(b"data2");
        assert_ne!(cid1, cid2);
    }

    #[test]
    fn blob_store_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let data = b"model weights binary data";
        let cid = compute_cid(data);
        store_blob(dir.path(), &cid, data).unwrap();
        assert!(blob_exists(dir.path(), &cid));
        let loaded = load_blob(dir.path(), &cid).unwrap();
        assert_eq!(loaded, data);
    }

    #[test]
    fn blob_load_missing_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let cid = CID::from_blake3_hex("nonexistent");
        assert!(load_blob(dir.path(), &cid).is_err());
    }

    #[test]
    fn blob_path_format() {
        let cid = CID::from_blake3_hex("abc123");
        let path = blob_path(Path::new("/tmp/test"), &cid);
        assert_eq!(path, PathBuf::from("/tmp/test/checkpoints/blake3_abc123.bin"));
    }
}
