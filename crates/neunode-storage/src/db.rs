use rocksdb::{
    BoundColumnFamily, ColumnFamilyDescriptor, Direction, IteratorMode, Options, WriteBatch, DB,
};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::cache::Cache;
use crate::cf::{self, Partition};
use crate::error::{Result, StorageError};

pub struct NeunodeDb {
    ledger_db: DB,
    network_db: DB,
    graph_db: DB,
    cache: Cache,
    partition_map: std::collections::HashMap<&'static str, Partition>,
    ledger_write_lock: Mutex<()>,
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
        let partition_map = cf::build_partition_map();

        Ok(Self {
            ledger_db,
            network_db,
            graph_db,
            cache,
            partition_map,
            ledger_write_lock: Mutex::new(()),
        })
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
        match self.partition_map.get(cf_name) {
            Some(Partition::Ledger) => Ok(&self.ledger_db),
            Some(Partition::Network) => Ok(&self.network_db),
            Some(Partition::Graph) => Ok(&self.graph_db),
            None => Err(StorageError::ColumnFamilyNotFound(cf_name.to_string())),
        }
    }

    fn partition_for_cf(&self, cf_name: &str) -> Result<Partition> {
        self.partition_map
            .get(cf_name)
            .copied()
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(cf_name.to_string()))
    }

    /// Serialize a complete ledger read-validate-write operation.
    ///
    /// RocksDB prevents multiple processes from opening the same database path;
    /// this lock supplies the missing isolation between threads in the owning
    /// daemon process. Callers must keep the closure synchronous and short.
    pub fn with_ledger_write<T, E>(
        &self,
        operation: impl FnOnce() -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E>
    where
        E: From<StorageError>,
    {
        let _guard =
            self.ledger_write_lock.lock().map_err(|_| E::from(StorageError::LedgerLockPoisoned))?;
        operation()
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
            crate::codec::serialize(key).map_err(|e| StorageError::Serialization(e.to_string()))?;
        match self.get_raw(cf_name, &key_bytes)? {
            Some(bytes) => {
                let value: V = crate::codec::deserialize(&bytes)
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
            crate::codec::serialize(key).map_err(|e| StorageError::Serialization(e.to_string()))?;
        let value_bytes = crate::codec::serialize(value)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
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
        self.batch_write_raw(ops, &[])
    }

    pub fn batch_delete_raw(&self, ops: &[(&str, &[u8])]) -> Result<()> {
        self.batch_write_raw(&[], ops)
    }

    /// Atomically apply puts and deletes within one physical storage partition.
    pub fn batch_write_raw(
        &self,
        puts: &[(&str, &[u8], &[u8])],
        deletes: &[(&str, &[u8])],
    ) -> Result<()> {
        let mut partition = None;
        for cf_name in puts
            .iter()
            .map(|(cf_name, _, _)| *cf_name)
            .chain(deletes.iter().map(|(cf_name, _)| *cf_name))
        {
            let current = self.partition_for_cf(cf_name)?;
            if partition.is_some_and(|expected| expected != current) {
                return Err(StorageError::CrossPartitionBatch);
            }
            partition = Some(current);
        }
        let Some(partition) = partition else {
            return Ok(());
        };

        let mut batch = WriteBatch::default();
        for &(cf_name, key, value) in puts {
            batch.put_cf(&self.cf_handle(cf_name)?, key, value);
        }
        for &(cf_name, key) in deletes {
            batch.delete_cf(&self.cf_handle(cf_name)?, key);
        }
        match partition {
            Partition::Ledger => self.ledger_db.write(batch)?,
            Partition::Network => self.network_db.write(batch)?,
            Partition::Graph => self.graph_db.write(batch)?,
        }

        for &(cf_name, key, value) in puts {
            self.cache.insert(cf_name, key, value.to_vec());
        }
        for &(cf_name, key) in deletes {
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

    /// Scan forward from `start`, returning at most `limit` records.
    pub fn scan_from_limit(
        &self,
        cf_name: &str,
        start: &[u8],
        limit: usize,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let cf = self.cf_handle(cf_name)?;
        let db = self.get_db_for_cf(cf_name)?;
        let iter = db.iterator_cf(&cf, IteratorMode::From(start, Direction::Forward));
        let mut results = Vec::with_capacity(limit.min(1024));
        for item in iter.take(limit) {
            let (key, value) = item.map_err(StorageError::RocksDb)?;
            results.push((key.to_vec(), value.to_vec()));
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

    /// Return the lexicographically greatest key/value pair in a column family.
    pub fn last_raw(&self, cf_name: &str) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
        let cf = self.cf_handle(cf_name)?;
        let db = self.get_db_for_cf(cf_name)?;
        let mut iter = db.iterator_cf(&cf, IteratorMode::End);
        match iter.next() {
            Some(item) => {
                let (key, value) = item.map_err(StorageError::RocksDb)?;
                Ok(Some((key.to_vec(), value.to_vec())))
            }
            None => Ok(None),
        }
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
    fn test_open_creates_all_column_families() {
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
    fn test_last_raw() {
        let db = temp_db();
        assert_eq!(db.last_raw(cf::CF_CONFIG).unwrap(), None);
        db.put_raw(cf::CF_CONFIG, b"a", b"first").unwrap();
        db.put_raw(cf::CF_CONFIG, b"z", b"last").unwrap();
        db.put_raw(cf::CF_CONFIG, b"m", b"middle").unwrap();

        assert_eq!(db.last_raw(cf::CF_CONFIG).unwrap(), Some((b"z".to_vec(), b"last".to_vec())));
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
    fn test_batch_write_raw_combines_puts_and_deletes() {
        let db = temp_db();
        db.put_raw(cf::CF_CONFIG, b"old", b"value").unwrap();

        db.batch_write_raw(&[(cf::CF_CONFIG, b"new", b"replacement")], &[(cf::CF_CONFIG, b"old")])
            .unwrap();

        assert_eq!(db.get_raw(cf::CF_CONFIG, b"old").unwrap(), None);
        assert_eq!(db.get_raw(cf::CF_CONFIG, b"new").unwrap(), Some(b"replacement".to_vec()));
    }

    #[test]
    fn test_batch_write_rejects_cross_partition_operations() {
        let db = temp_db();
        let error = db
            .batch_write_raw(
                &[(cf::CF_CONFIG, b"ledger", b"value")],
                &[(cf::CF_FEED_STATE, b"graph")],
            )
            .unwrap_err();

        assert!(matches!(error, StorageError::CrossPartitionBatch));
        assert_eq!(db.get_raw(cf::CF_CONFIG, b"ledger").unwrap(), None);
    }

    #[test]
    fn test_prefix_scan_empty_cf() {
        let db = temp_db();
        let results = db.prefix_scan(cf::CF_MODELS, b"anything").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_from_limit() {
        let db = temp_db();
        for key in [b"a", b"b", b"c", b"d"] {
            db.put_raw(cf::CF_CONFIG, key, key).unwrap();
        }
        let results = db.scan_from_limit(cf::CF_CONFIG, b"b", 2).unwrap();
        assert_eq!(results, vec![(b"b".to_vec(), b"b".to_vec()), (b"c".to_vec(), b"c".to_vec())]);
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
