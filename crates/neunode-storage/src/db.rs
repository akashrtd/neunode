use rocksdb::{
    BoundColumnFamily, ColumnFamilyDescriptor, Direction, IteratorMode, Options, WriteBatch, DB,
};
use std::path::Path;
use std::sync::Arc;

use crate::cache::Cache;
use crate::cf;
use crate::error::{Result, StorageError};

pub struct NeunodeDb {
    ledger_db: DB,
    network_db: DB,
    graph_db: DB,
    cache: Cache,
}

impl NeunodeDb {
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_with_cache(path, 10_000, 300)
    }

    pub fn open_with_cache(
        path: &Path,
        max_cache_entries: usize,
        cache_ttl_secs: u64,
    ) -> Result<Self> {
        let ledger_path = path.join("ledger");
        let network_path = path.join("network");
        let graph_path = path.join("graph");

        let ledger_db = Self::open_db_for_cfs(&ledger_path, cf::ledger_column_families())?;
        let network_db = Self::open_db_for_cfs(&network_path, cf::network_column_families())?;
        let graph_db = Self::open_db_for_cfs(&graph_path, cf::graph_column_families())?;

        let cache = Cache::new(max_cache_entries, cache_ttl_secs);

        Ok(Self { ledger_db, network_db, graph_db, cache })
    }

    fn open_db_for_cfs(path: &Path, required: Vec<&'static str>) -> Result<DB> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let required_vec: Vec<String> = required.iter().map(|s| s.to_string()).collect();
        let existing =
            DB::list_cf(&Options::default(), path).unwrap_or_else(|_| vec!["default".into()]);

        let mut all_cfs = required_vec;
        for cf in existing {
            if !all_cfs.contains(&cf) {
                all_cfs.push(cf);
            }
        }

        let descriptors: Vec<ColumnFamilyDescriptor> = all_cfs
            .iter()
            .map(|name| ColumnFamilyDescriptor::new(name.clone(), Options::default()))
            .collect();

        DB::open_cf_descriptors(&opts, path, descriptors).map_err(Into::into)
    }

    pub fn cf_handle(&self, cf_name: &str) -> Result<Arc<BoundColumnFamily<'_>>> {
        let db = self.get_db_for_cf(cf_name)?;
        db.cf_handle(cf_name).ok_or_else(|| StorageError::ColumnFamilyNotFound(cf_name.to_string()))
    }

    fn get_db_for_cf(&self, cf_name: &str) -> Result<&DB> {
        if cf::ledger_column_families().contains(&cf_name) {
            Ok(&self.ledger_db)
        } else if cf::network_column_families().contains(&cf_name) {
            Ok(&self.network_db)
        } else if cf::graph_column_families().contains(&cf_name) {
            Ok(&self.graph_db)
        } else {
            Err(StorageError::ColumnFamilyNotFound(cf_name.to_string()))
        }
    }

    pub fn get_raw(&self, cf_name: &str, key: &[u8]) -> Result<Option<Vec<u8>>> {
        if let Some(cached) = self.cache.get(cf_name, key) {
            return Ok(Some(cached));
        }
        let cf = self.cf_handle(cf_name)?;
        let db = self.get_db_for_cf(cf_name)?;
        match db.get_cf(&cf, key)? {
            Some(bytes) => {
                self.cache.insert(cf_name, key, bytes.clone());
                Ok(Some(bytes))
            }
            None => Ok(None),
        }
    }

    pub fn get<K, V>(&self, cf_name: &str, key: &K) -> Result<Option<V>>
    where
        K: serde::Serialize,
        V: serde::de::DeserializeOwned,
    {
        let key_bytes =
            bincode::serialize(key).map_err(|e| StorageError::Serialization(e.to_string()))?;
        match self.get_raw(cf_name, &key_bytes)? {
            Some(bytes) => {
                let value: V = bincode::deserialize(&bytes)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    pub fn put_raw(&self, cf_name: &str, key: &[u8], value: &[u8]) -> Result<()> {
        let cf = self.cf_handle(cf_name)?;
        let db = self.get_db_for_cf(cf_name)?;
        db.put_cf(&cf, key, value)?;
        self.cache.insert(cf_name, key, value.to_vec());
        Ok(())
    }

    pub fn put<K, V>(&self, cf_name: &str, key: &K, value: &V) -> Result<()>
    where
        K: serde::Serialize,
        V: serde::Serialize,
    {
        let key_bytes =
            bincode::serialize(key).map_err(|e| StorageError::Serialization(e.to_string()))?;
        let value_bytes =
            bincode::serialize(value).map_err(|e| StorageError::Serialization(e.to_string()))?;
        self.put_raw(cf_name, &key_bytes, &value_bytes)
    }

    pub fn delete(&self, cf_name: &str, key: &[u8]) -> Result<()> {
        let cf = self.cf_handle(cf_name)?;
        let db = self.get_db_for_cf(cf_name)?;
        db.delete_cf(&cf, key)?;
        self.cache.invalidate(cf_name, key);
        Ok(())
    }

    // Removed raw write_batch because it doesn't span multiple partitioned DBs properly automatically.
    // Callers must use specific atomic routines or batch_put_raw, which routes by CF.

    pub fn batch_put_raw(&self, ops: &[(&str, &[u8], &[u8])]) -> Result<()> {
        let mut ledger_batch = WriteBatch::default();
        let mut network_batch = WriteBatch::default();
        let mut graph_batch = WriteBatch::default();

        for &(cf_name, key, value) in ops {
            let cf = self.cf_handle(cf_name)?;
            if cf::ledger_column_families().contains(&cf_name) {
                ledger_batch.put_cf(&cf, key, value);
            } else if cf::network_column_families().contains(&cf_name) {
                network_batch.put_cf(&cf, key, value);
            } else {
                graph_batch.put_cf(&cf, key, value);
            }
        }

        self.ledger_db.write(ledger_batch)?;
        self.network_db.write(network_batch)?;
        self.graph_db.write(graph_batch)?;

        for &(cf_name, key, value) in ops {
            self.cache.insert(cf_name, key, value.to_vec());
        }
        Ok(())
    }

    pub fn batch_delete_raw(&self, ops: &[(&str, &[u8])]) -> Result<()> {
        let mut ledger_batch = WriteBatch::default();
        let mut network_batch = WriteBatch::default();
        let mut graph_batch = WriteBatch::default();

        for &(cf_name, key) in ops {
            let cf = self.cf_handle(cf_name)?;
            if cf::ledger_column_families().contains(&cf_name) {
                ledger_batch.delete_cf(&cf, key);
            } else if cf::network_column_families().contains(&cf_name) {
                network_batch.delete_cf(&cf, key);
            } else {
                graph_batch.delete_cf(&cf, key);
            }
        }

        self.ledger_db.write(ledger_batch)?;
        self.network_db.write(network_batch)?;
        self.graph_db.write(graph_batch)?;

        for &(cf_name, key) in ops {
            self.cache.invalidate(cf_name, key);
        }
        Ok(())
    }

    pub fn prefix_scan(&self, cf_name: &str, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let cf = self.cf_handle(cf_name)?;
        let db = self.get_db_for_cf(cf_name)?;
        let iter = db.iterator_cf(&cf, IteratorMode::From(prefix, Direction::Forward));
        let mut results = Vec::new();
        for item in iter {
            let (k, v) = item.map_err(StorageError::RocksDb)?;
            if k.starts_with(prefix) {
                results.push((k.to_vec(), v.to_vec()));
            } else {
                break;
            }
        }
        Ok(results)
    }

    pub fn range_scan(
        &self,
        cf_name: &str,
        start: &[u8],
        end: &[u8],
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let cf = self.cf_handle(cf_name)?;
        let db = self.get_db_for_cf(cf_name)?;
        let iter = db.iterator_cf(&cf, IteratorMode::From(start, Direction::Forward));
        let mut results = Vec::new();
        for item in iter {
            let (k, v) = item.map_err(StorageError::RocksDb)?;
            if &k[..] < end {
                results.push((k.to_vec(), v.to_vec()));
            } else {
                break;
            }
        }
        Ok(results)
    }

    pub fn close(self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_db() -> NeunodeDb {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "neunode_storage_db_{:?}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&dir);
        NeunodeDb::open(&dir).unwrap()
    }

    #[test]
    fn test_open_creates_all_20_cfs() {
        let db = temp_db();
        for cf_name in cf::all_column_families() {
            assert!(db.cf_handle(cf_name).is_ok(), "CF '{cf_name}' should exist after open");
        }
    }

    #[test]
    fn test_cf_handle_not_found() {
        let db = temp_db();
        let result = db.cf_handle("nonexistent_cf");
        assert!(result.is_err());
        match result {
            Err(StorageError::ColumnFamilyNotFound(name)) => {
                assert_eq!(name, "nonexistent_cf");
            }
            Ok(_) => panic!("expected ColumnFamilyNotFound, got Ok"),
            Err(other) => panic!("expected ColumnFamilyNotFound, got {other:?}"),
        }
    }

    #[test]
    fn test_put_get_roundtrip_typed() {
        let db = temp_db();
        let key = "did:neunode:0xABC";
        let value = "hello world".to_string();

        db.put(cf::CF_IDENTITY, &key, &value).unwrap();
        let result: Option<String> = db.get(cf::CF_IDENTITY, &key).unwrap();
        assert_eq!(result, Some("hello world".to_string()));
    }

    #[test]
    fn test_put_get_roundtrip_raw() {
        let db = temp_db();
        db.put_raw(cf::CF_CONFIG, b"my_key", b"my_value").unwrap();
        let result = db.get_raw(cf::CF_CONFIG, b"my_key").unwrap();
        assert_eq!(result, Some(b"my_value".to_vec()));
    }

    #[test]
    fn test_get_missing_returns_none() {
        let db = temp_db();
        let result: Option<String> = db.get(cf::CF_IDENTITY, &"nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_raw_missing_returns_none() {
        let db = temp_db();
        let result = db.get_raw(cf::CF_IDENTITY, b"nope").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete() {
        let db = temp_db();
        db.put_raw(cf::CF_IDENTITY, b"key1", b"val1").unwrap();
        assert!(db.get_raw(cf::CF_IDENTITY, b"key1").unwrap().is_some());

        db.delete(cf::CF_IDENTITY, b"key1").unwrap();
        assert!(db.get_raw(cf::CF_IDENTITY, b"key1").unwrap().is_none());
    }

    #[test]
    fn test_cache_hit_on_second_get() {
        let db = temp_db();
        db.put_raw(cf::CF_IDENTITY, b"ckey", b"cval").unwrap();

        let _first = db.get_raw(cf::CF_IDENTITY, b"ckey").unwrap();
        let second = db.get_raw(cf::CF_IDENTITY, b"ckey").unwrap();
        assert_eq!(second, Some(b"cval".to_vec()));
    }

    #[test]
    fn test_cache_invalidation_on_delete() {
        let db = temp_db();
        db.put_raw(cf::CF_IDENTITY, b"k", b"v").unwrap();
        let _ = db.get_raw(cf::CF_IDENTITY, b"k").unwrap();

        db.delete(cf::CF_IDENTITY, b"k").unwrap();
        assert!(db.cache.get(cf::CF_IDENTITY, b"k").is_none());
        assert!(db.get_raw(cf::CF_IDENTITY, b"k").unwrap().is_none());
    }

    #[test]
    fn test_prefix_scan() {
        let db = temp_db();
        db.put_raw(cf::CF_TOKENS, b"aa_1", b"v1").unwrap();
        db.put_raw(cf::CF_TOKENS, b"aa_2", b"v2").unwrap();
        db.put_raw(cf::CF_TOKENS, b"bb_1", b"v3").unwrap();

        let results = db.prefix_scan(cf::CF_TOKENS, b"aa_").unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|(k, _)| k.starts_with(b"aa_")));
    }

    #[test]
    fn test_range_scan() {
        let db = temp_db();
        db.put_raw(cf::CF_CONFIG, b"key_01", b"v1").unwrap();
        db.put_raw(cf::CF_CONFIG, b"key_02", b"v2").unwrap();
        db.put_raw(cf::CF_CONFIG, b"key_03", b"v3").unwrap();
        db.put_raw(cf::CF_CONFIG, b"key_05", b"v5").unwrap();

        let results = db.range_scan(cf::CF_CONFIG, b"key_02", b"key_04").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_batch_put_raw() {
        let db = temp_db();

        let k1 = b"bp_k1".as_slice();
        let v1 = b"bp_v1".as_slice();
        let k2 = b"bp_k2".as_slice();
        let v2 = b"bp_v2".as_slice();

        db.batch_put_raw(&[(cf::CF_CONFIG, k1, v1), (cf::CF_CONFIG, k2, v2)]).unwrap();

        assert_eq!(db.get_raw(cf::CF_CONFIG, b"bp_k1").unwrap(), Some(b"bp_v1".to_vec()));
        assert_eq!(db.get_raw(cf::CF_CONFIG, b"bp_k2").unwrap(), Some(b"bp_v2".to_vec()));
    }

    #[test]
    fn test_prefix_scan_empty_cf() {
        let db = temp_db();
        let results = db.prefix_scan(cf::CF_MODELS, b"anything").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_reopen_preserves_data() {
        let dir =
            std::env::temp_dir().join(format!("neunode_storage_db_reopen_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        {
            let db = NeunodeDb::open(&dir).unwrap();
            db.put_raw(cf::CF_CONFIG, b"persist_key", b"persist_val").unwrap();
        }

        let db2 = NeunodeDb::open(&dir).unwrap();
        let result = db2.get_raw(cf::CF_CONFIG, b"persist_key").unwrap();
        assert_eq!(result, Some(b"persist_val".to_vec()));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
