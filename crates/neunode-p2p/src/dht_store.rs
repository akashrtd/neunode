use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, Mutex};

use libp2p::kad::store::{Error, RecordStore, Result};
use libp2p::kad::{ProviderRecord, Record, RecordKey as Key};
use libp2p::PeerId;
use rocksdb::{Options, DB};

/// RocksDB-backed Kademlia record store. Survives restarts.
///
/// Uses namespaced keys in the default column family:
/// - `r:<key>` → record value bytes
/// - `m:<key>` → u64 LE expiration timestamp (0 = no expiry)
/// - `p:<record_key>\0<peer_id>` → provider peer bytes
pub struct RocksRecordStore {
    db: DB,
}

impl RocksRecordStore {
    pub fn open(path: &Path) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open(&opts, path).map_err(|e| {
            tracing::warn!("RocksDB open failed at {}: {e}", path.display());
            Error::MaxRecords
        })?;
        Ok(Self { db })
    }

    fn record_key(k: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(k.len() + 2);
        buf.extend_from_slice(b"r:");
        buf.extend_from_slice(k);
        buf
    }

    fn meta_key(k: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(k.len() + 2);
        buf.extend_from_slice(b"m:");
        buf.extend_from_slice(k);
        buf
    }

    fn provider_composite(key: &Key, peer: &PeerId) -> Vec<u8> {
        let key_bytes = key.as_ref();
        let peer_bytes = peer.to_bytes();
        let mut buf = Vec::with_capacity(2 + 2 + key_bytes.len() + peer_bytes.len());
        buf.extend_from_slice(b"p:");
        buf.extend_from_slice(&(key_bytes.len() as u16).to_be_bytes());
        buf.extend_from_slice(key_bytes);
        buf.extend_from_slice(&peer_bytes);
        buf
    }

    fn provider_prefix(key: &Key) -> Vec<u8> {
        let key_bytes = key.as_ref();
        let mut buf = Vec::with_capacity(2 + 2 + key_bytes.len());
        buf.extend_from_slice(b"p:");
        buf.extend_from_slice(&(key_bytes.len() as u16).to_be_bytes());
        buf.extend_from_slice(key_bytes);
        buf
    }

    fn extract_peer_from_provider_key(k: &[u8]) -> Option<PeerId> {
        if k.len() < 6 {
            return None;
        } // "p:" + 2 byte len + at least 2 bytes
        let key_len = u16::from_be_bytes([k[2], k[3]]) as usize;
        let peer_start = 4 + key_len;
        if peer_start >= k.len() {
            return None;
        }
        PeerId::from_bytes(&k[peer_start..]).ok()
    }

    fn is_expired(&self, key_bytes: &[u8]) -> bool {
        let mk = Self::meta_key(key_bytes);
        let Some(val) = self.db.get(&mk).unwrap_or(None) else { return false };
        let expires = u64_from_le(&val);
        expires > 0 && expires <= now_secs()
    }

    fn evict_expired(&self) {
        let iter =
            self.db.iterator(rocksdb::IteratorMode::From(b"m:", rocksdb::Direction::Forward));
        let mut expired = Vec::new();
        for item in iter {
            let Ok((key, val)) = item else { continue };
            if !key.starts_with(b"m:") {
                break;
            }
            let expires = u64_from_le(&val);
            if expires > 0 && expires <= now_secs() {
                expired.push(key.to_vec());
            }
        }
        for mk in &expired {
            let raw_key = &mk[2..];
            let rk = Self::record_key(raw_key);
            let _ = self.db.delete(&rk);
            let _ = self.db.delete(mk);
        }
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn u64_to_le(v: u64) -> [u8; 8] {
    v.to_le_bytes()
}

fn u64_from_le(bytes: &[u8]) -> u64 {
    let mut arr = [0u8; 8];
    let len = bytes.len().min(8);
    arr[..len].copy_from_slice(&bytes[..len]);
    u64::from_le_bytes(arr)
}

/// Cloneable wrapper — libp2p's `Behaviour<S>` requires `S: Clone`.
#[derive(Clone)]
pub struct SharedRocksStore(pub Arc<Mutex<RocksRecordStore>>);

impl SharedRocksStore {
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self(Arc::new(Mutex::new(RocksRecordStore::open(path)?))))
    }
}

impl RecordStore for SharedRocksStore {
    type RecordsIter<'a> = std::vec::IntoIter<Cow<'a, Record>>;
    type ProvidedIter<'a> = std::vec::IntoIter<Cow<'a, ProviderRecord>>;

    fn get(&self, k: &Key) -> Option<Cow<'_, Record>> {
        let store = self.0.lock().ok()?;
        store.evict_expired();
        if store.is_expired(k.as_ref()) {
            return None;
        }
        let rk = RocksRecordStore::record_key(k.as_ref());
        let raw = store.db.get(&rk).ok()??;
        drop(store);
        Some(Cow::Owned(Record { key: k.clone(), value: raw, publisher: None, expires: None }))
    }

    fn put(&mut self, r: Record) -> Result<()> {
        let store = self.0.lock().map_err(|_| Error::MaxRecords)?;
        let rk = RocksRecordStore::record_key(r.key.as_ref());
        let mk = RocksRecordStore::meta_key(r.key.as_ref());
        let expires_ts = match r.expires {
            Some(_) => now_secs().saturating_add(3600),
            None => 0u64,
        };
        store.db.put(&rk, &r.value).map_err(|_| Error::MaxRecords)?;
        store.db.put(&mk, u64_to_le(expires_ts)).map_err(|_| Error::MaxRecords)?;
        Ok(())
    }

    fn remove(&mut self, k: &Key) {
        let Ok(store) = self.0.lock() else { return };
        let rk = RocksRecordStore::record_key(k.as_ref());
        let mk = RocksRecordStore::meta_key(k.as_ref());
        let _ = store.db.delete(&rk);
        let _ = store.db.delete(&mk);
    }

    fn records(&self) -> Self::RecordsIter<'_> {
        let Ok(store) = self.0.lock() else { return Vec::new().into_iter() };
        store.evict_expired();
        let iter =
            store.db.iterator(rocksdb::IteratorMode::From(b"r:", rocksdb::Direction::Forward));
        let mut records = Vec::new();
        for item in iter {
            let Ok((key, value)) = item else { continue };
            if !key.starts_with(b"r:") {
                break;
            }
            let raw_key = key[2..].to_vec();
            records.push(Cow::Owned(Record {
                key: Key::from(raw_key),
                value: value.to_vec(),
                publisher: None,
                expires: None,
            }));
        }
        drop(store);
        records.into_iter()
    }

    fn add_provider(&mut self, record: ProviderRecord) -> Result<()> {
        let store = self.0.lock().map_err(|_| Error::MaxRecords)?;
        let composite = RocksRecordStore::provider_composite(&record.key, &record.provider);
        let val = record.provider.to_bytes();
        store.db.put(&composite, &val).map_err(|_| Error::MaxProvidedKeys)?;
        Ok(())
    }

    fn providers(&self, key: &Key) -> Vec<ProviderRecord> {
        let Ok(store) = self.0.lock() else { return Vec::new() };
        let prefix = RocksRecordStore::provider_prefix(key);
        let iter =
            store.db.iterator(rocksdb::IteratorMode::From(&prefix, rocksdb::Direction::Forward));
        let mut result = Vec::new();
        for item in iter {
            let Ok((k, _val)) = item else { continue };
            if !k.starts_with(&prefix) {
                break;
            }
            if let Some(peer_id) = RocksRecordStore::extract_peer_from_provider_key(&k) {
                let record = ProviderRecord::new(key.clone(), peer_id, Vec::new());
                result.push(record);
            }
        }
        result
    }

    fn provided(&self) -> Self::ProvidedIter<'_> {
        Vec::new().into_iter()
    }

    fn remove_provider(&mut self, key: &Key, peer: &PeerId) {
        let Ok(store) = self.0.lock() else { return };
        let composite = RocksRecordStore::provider_composite(key, peer);
        let _ = store.db.delete(&composite);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_store() -> SharedRocksStore {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("neunode_dht_test_{:?}_{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&dir);
        SharedRocksStore::open(&dir).expect("store open")
    }

    fn random_peer() -> PeerId {
        libp2p::identity::Keypair::generate_ed25519().public().to_peer_id()
    }

    #[test]
    fn put_and_get() {
        let mut store = temp_store();
        let key = Key::from(b"test-key".to_vec());
        store
            .put(Record {
                key: key.clone(),
                value: b"test-value".to_vec(),
                publisher: None,
                expires: None,
            })
            .unwrap();
        let got = store.get(&key).unwrap();
        assert_eq!(got.value, b"test-value");
    }

    #[test]
    fn get_missing_returns_none() {
        let store = temp_store();
        assert!(store.get(&Key::from(b"missing".to_vec())).is_none());
    }

    #[test]
    fn remove_deletes_record() {
        let mut store = temp_store();
        let key = Key::from(b"removeme".to_vec());
        store
            .put(Record { key: key.clone(), value: b"v".to_vec(), publisher: None, expires: None })
            .unwrap();
        store.remove(&key);
        assert!(store.get(&key).is_none());
    }

    #[test]
    fn records_returns_all() {
        let mut store = temp_store();
        for i in 0u8..5 {
            store
                .put(Record {
                    key: Key::from(vec![i]),
                    value: vec![i],
                    publisher: None,
                    expires: None,
                })
                .unwrap();
        }
        assert_eq!(store.records().count(), 5);
    }

    #[test]
    fn persists_across_reopen() {
        let dir =
            std::env::temp_dir().join(format!("neunode_dht_persist_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        {
            let mut store = SharedRocksStore::open(&dir).unwrap();
            store
                .put(Record {
                    key: Key::from(b"persistent".to_vec()),
                    value: b"data".to_vec(),
                    publisher: None,
                    expires: None,
                })
                .unwrap();
        }

        let store = SharedRocksStore::open(&dir).unwrap();
        let got = store.get(&Key::from(b"persistent".to_vec())).unwrap();
        assert_eq!(got.value, b"data");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clone_shares_state() {
        let mut store = temp_store();
        let key = Key::from(b"shared".to_vec());
        store
            .put(Record { key: key.clone(), value: b"v".to_vec(), publisher: None, expires: None })
            .unwrap();
        let cloned = store.clone();
        assert!(cloned.get(&key).is_some());
    }

    #[test]
    fn add_and_get_providers() {
        let mut store = temp_store();
        let key = Key::from(b"my-data".to_vec());
        let peer = random_peer();
        store.add_provider(ProviderRecord::new(key.clone(), peer, Vec::new())).unwrap();
        let providers = store.providers(&key);
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].provider, peer);
    }

    #[test]
    fn remove_provider() {
        let mut store = temp_store();
        let key = Key::from(b"my-data".to_vec());
        let peer = random_peer();
        store.add_provider(ProviderRecord::new(key.clone(), peer, Vec::new())).unwrap();
        store.remove_provider(&key, &peer);
        assert!(store.providers(&key).is_empty());
    }

    #[test]
    fn provided_returns_empty() {
        let store = temp_store();
        assert!(store.provided().collect::<Vec<_>>().is_empty());
    }
}
