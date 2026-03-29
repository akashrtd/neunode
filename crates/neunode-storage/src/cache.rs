use moka::sync::Cache as MokaCache;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

pub struct Cache {
    inner: MokaCache<Vec<u8>, Vec<u8>>,
    cf_keys: Mutex<HashMap<String, HashSet<Vec<u8>>>>,
}

fn cache_key(cf: &str, key: &[u8]) -> Vec<u8> {
    let mut ck = Vec::with_capacity(cf.len() + 1 + key.len());
    ck.extend_from_slice(cf.as_bytes());
    ck.push(0xFF);
    ck.extend_from_slice(key);
    ck
}

impl Cache {
    pub fn new(max_entries: usize, ttl_secs: u64) -> Self {
        let inner = MokaCache::builder()
            .max_capacity(max_entries as u64)
            .time_to_idle(Duration::from_secs(ttl_secs))
            .build();
        Self { inner, cf_keys: Mutex::new(HashMap::new()) }
    }

    pub fn get(&self, cf: &str, key: &[u8]) -> Option<Vec<u8>> {
        let ck = cache_key(cf, key);
        self.inner.get(&ck)
    }

    pub fn insert(&self, cf: &str, key: &[u8], value: Vec<u8>) {
        let ck = cache_key(cf, key);
        self.inner.insert(ck.clone(), value);
        if let Ok(mut guard) = self.cf_keys.lock() {
            guard.entry(cf.to_string()).or_default().insert(ck);
        }
    }

    pub fn invalidate(&self, cf: &str, key: &[u8]) {
        let ck = cache_key(cf, key);
        self.inner.invalidate(&ck);
        if let Ok(mut guard) = self.cf_keys.lock() {
            if let Some(keys) = guard.get_mut(cf) {
                keys.remove(&ck);
            }
        }
    }

    pub fn invalidate_cf(&self, cf: &str) {
        if let Ok(mut guard) = self.cf_keys.lock() {
            if let Some(keys) = guard.remove(cf) {
                for ck in keys {
                    self.inner.invalidate(&ck);
                }
            }
        }
    }

    pub fn invalidate_all(&self) {
        self.inner.invalidate_all();
        if let Ok(mut guard) = self.cf_keys.lock() {
            guard.clear();
        }
    }

    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_miss() {
        let cache = Cache::new(100, 60);
        assert!(cache.get("identity", b"key1").is_none());
    }

    #[test]
    fn test_cache_insert_and_hit() {
        let cache = Cache::new(100, 60);
        cache.insert("identity", b"key1", b"value1".to_vec());
        assert_eq!(cache.get("identity", b"key1"), Some(b"value1".to_vec()));
    }

    #[test]
    fn test_cache_invalidate() {
        let cache = Cache::new(100, 60);
        cache.insert("identity", b"key1", b"value1".to_vec());
        assert!(cache.get("identity", b"key1").is_some());

        cache.invalidate("identity", b"key1");
        assert!(cache.get("identity", b"key1").is_none());
    }

    #[test]
    fn test_cache_invalidate_cf() {
        let cache = Cache::new(100, 60);
        cache.insert("identity", b"k1", b"v1".to_vec());
        cache.insert("identity", b"k2", b"v2".to_vec());
        cache.insert("tokens", b"k3", b"v3".to_vec());

        cache.invalidate_cf("identity");

        assert!(cache.get("identity", b"k1").is_none());
        assert!(cache.get("identity", b"k2").is_none());
        assert_eq!(cache.get("tokens", b"k3"), Some(b"v3".to_vec()));
    }

    #[test]
    fn test_cache_invalidate_all() {
        let cache = Cache::new(100, 60);
        cache.insert("identity", b"k1", b"v1".to_vec());
        cache.insert("tokens", b"k2", b"v2".to_vec());

        cache.invalidate_all();

        assert!(cache.get("identity", b"k1").is_none());
        assert!(cache.get("tokens", b"k2").is_none());
    }

    #[test]
    fn test_cache_overwrite() {
        let cache = Cache::new(100, 60);
        cache.insert("cf", b"key", b"v1".to_vec());
        cache.insert("cf", b"key", b"v2".to_vec());
        assert_eq!(cache.get("cf", b"key"), Some(b"v2".to_vec()));
    }

    #[test]
    fn test_cache_key_isolation() {
        let cache = Cache::new(100, 60);
        cache.insert("cf_a", b"same_key", b"val_a".to_vec());
        cache.insert("cf_b", b"same_key", b"val_b".to_vec());

        assert_eq!(cache.get("cf_a", b"same_key"), Some(b"val_a".to_vec()));
        assert_eq!(cache.get("cf_b", b"same_key"), Some(b"val_b".to_vec()));
    }

    #[test]
    fn test_cache_key_format() {
        let ck = cache_key("identity", b"\x01\x02");
        assert_eq!(&ck, b"identity\xFF\x01\x02");
    }
}
