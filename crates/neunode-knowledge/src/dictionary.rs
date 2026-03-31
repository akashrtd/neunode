use siphasher::sip128::{Hasher128, SipHasher24};
use std::hash::Hash;

use neunode_storage::cf::CF_KG_ID2STR;
use neunode_storage::db::NeunodeDb;

use crate::error::{KnowledgeError, Result};

/// 128-bit hash used as key in the KG string dictionary.
pub type Hash128 = [u8; 16];

/// String dictionary mapping URIs/literals to 128-bit SipHash24 hashes.
///
/// Stores bidirectional mapping in RocksDB `kg_id2str` column family.
/// Key = 16-byte SipHash24 hash, Value = UTF-8 string.
///
/// Follows the Oxigraph pattern: all KG term strings (URIs, literals, blank nodes)
/// are interned through this dictionary for compact storage in the 6 index CFs.
pub struct StringDictionary<'a> {
    db: &'a NeunodeDb,
}

impl<'a> StringDictionary<'a> {
    /// Create a new dictionary backed by the given database.
    pub fn new(db: &'a NeunodeDb) -> Self {
        Self { db }
    }

    /// Compute the deterministic 128-bit SipHash24 hash of a string.
    ///
    /// Uses zero keys for reproducibility across runs and nodes.
    /// This is the same algorithm Oxigraph uses for its string dictionary.
    pub fn hash(s: &str) -> Hash128 {
        let mut hasher = SipHasher24::new_with_keys(0, 0);
        s.hash(&mut hasher);
        let h = hasher.finish128();
        let mut out = [0u8; 16];
        out[..8].copy_from_slice(&h.h1.to_be_bytes());
        out[8..].copy_from_slice(&h.h2.to_be_bytes());
        out
    }

    /// Insert a string into the dictionary. Returns its 128-bit hash.
    ///
    /// Idempotent: if the string already exists, returns the existing hash.
    pub fn insert(&self, s: &str) -> Result<Hash128> {
        let h = Self::hash(s);
        // Check if already exists — idempotent, avoid redundant writes.
        if self
            .db
            .get_raw(CF_KG_ID2STR, &h)
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))?
            .is_some()
        {
            return Ok(h);
        }
        self.db
            .put_raw(CF_KG_ID2STR, &h, s.as_bytes())
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))?;
        Ok(h)
    }

    /// Look up a string by its 128-bit hash.
    ///
    /// Returns `KnowledgeError::DictionaryMiss` if the hash is not found.
    pub fn lookup(&self, hash: &Hash128) -> Result<String> {
        match self.db.get_raw(CF_KG_ID2STR, hash) {
            Ok(Some(bytes)) => String::from_utf8(bytes).map_err(|e| {
                KnowledgeError::IndexCorrupted(format!("invalid UTF-8 in dictionary: {e}"))
            }),
            Ok(None) => Err(KnowledgeError::DictionaryMiss(hex_hash(hash))),
            Err(e) => Err(KnowledgeError::StorageError(e.to_string())),
        }
    }

    /// Check whether a hash exists in the dictionary.
    pub fn contains(&self, hash: &Hash128) -> Result<bool> {
        match self.db.get_raw(CF_KG_ID2STR, hash) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(KnowledgeError::StorageError(e.to_string())),
        }
    }

    /// Insert multiple strings in a single batch write.
    ///
    /// Returns the 128-bit hashes for each input string, in order.
    /// Idempotent for strings that already exist.
    pub fn batch_insert(&self, strings: &[&str]) -> Result<Vec<Hash128>> {
        let hashes: Vec<Hash128> = strings.iter().map(|s| Self::hash(s)).collect();

        // Build batch ops, skipping entries that already exist.
        let mut ops: Vec<(&str, &[u8], &[u8])> = Vec::with_capacity(strings.len());
        for (i, s) in strings.iter().enumerate() {
            if self
                .db
                .get_raw(CF_KG_ID2STR, &hashes[i])
                .map_err(|e| KnowledgeError::StorageError(e.to_string()))?
                .is_none()
            {
                ops.push((CF_KG_ID2STR, &hashes[i] as &[u8], s.as_bytes()));
            }
        }

        if !ops.is_empty() {
            self.db.batch_put_raw(&ops).map_err(|e| KnowledgeError::StorageError(e.to_string()))?;
        }

        Ok(hashes)
    }

    /// Remove a string from the dictionary by its hash.
    ///
    /// No-op if the hash doesn't exist.
    pub fn remove(&self, hash: &Hash128) -> Result<()> {
        self.db.delete(CF_KG_ID2STR, hash).map_err(|e| KnowledgeError::StorageError(e.to_string()))
    }
}

/// Format a 128-bit hash as lowercase hex for error messages.
fn hex_hash(h: &Hash128) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_db() -> NeunodeDb {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("neunode_kg_dict_{:?}_{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&dir);
        NeunodeDb::open(&dir).unwrap()
    }

    // ── Hash function tests ──

    #[test]
    fn hash_deterministic() {
        let a = StringDictionary::hash("http://example.org/resource/1");
        let b = StringDictionary::hash("http://example.org/resource/1");
        assert_eq!(a, b, "same string must produce same hash");
    }

    #[test]
    fn hash_different_strings() {
        let a = StringDictionary::hash("http://example.org/a");
        let b = StringDictionary::hash("http://example.org/b");
        assert_ne!(a, b, "different strings must produce different hashes");
    }

    #[test]
    fn hash_empty_string() {
        let h = StringDictionary::hash("");
        assert_eq!(h.len(), 16, "empty string must produce 16-byte hash");
    }

    #[test]
    fn hash_unicode() {
        let h = StringDictionary::hash("日本語テスト 🦀 émojis");
        assert_eq!(h.len(), 16);
        // Deterministic for unicode
        assert_eq!(h, StringDictionary::hash("日本語テスト 🦀 émojis"));
    }

    #[test]
    fn hash_length_16() {
        for s in &["a", "ab", "hello world", &"x".repeat(1000)] {
            assert_eq!(StringDictionary::hash(s).len(), 16, "hash must always be 16 bytes");
        }
    }

    #[test]
    fn hash_long_string() {
        let long = "x".repeat(10_000);
        let h = StringDictionary::hash(&long);
        assert_eq!(h.len(), 16);
        assert_eq!(h, StringDictionary::hash(&long));
    }

    #[test]
    fn hash_special_chars() {
        let s = "://#?&%=!@${}[]()<>\\\"'+-_,;|~`";
        let h = StringDictionary::hash(s);
        assert_eq!(h.len(), 16);
        assert_eq!(h, StringDictionary::hash(s));
    }

    // ── Insert + Lookup roundtrip tests ──

    #[test]
    fn insert_lookup_roundtrip() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let original = "http://example.org/predicate/knows";
        let h = dict.insert(original).unwrap();
        let looked_up = dict.lookup(&h).unwrap();
        assert_eq!(looked_up, original);
    }

    #[test]
    fn insert_idempotent() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let s = "http://example.org/same";
        let h1 = dict.insert(s).unwrap();
        let h2 = dict.insert(s).unwrap();
        assert_eq!(h1, h2, "inserting twice must return same hash");
        // Also verify only one entry exists
        let all = db.prefix_scan(CF_KG_ID2STR, &[]).unwrap();
        let matching = all.iter().filter(|(k, _)| k.as_slice() == h1).count();
        assert_eq!(matching, 1, "should be exactly one entry");
    }

    #[test]
    fn lookup_missing() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let fake_hash = [0xAB_u8; 16];
        let result = dict.lookup(&fake_hash);
        assert!(result.is_err());
        match result.unwrap_err() {
            KnowledgeError::DictionaryMiss(msg) => {
                assert!(msg.contains("abababab"), "error should contain hex of hash, got: {msg}");
            }
            other => panic!("expected DictionaryMiss, got: {other:?}"),
        }
    }

    #[test]
    fn contains_existing() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let h = dict.insert("http://example.org/exists").unwrap();
        assert!(dict.contains(&h).unwrap());
    }

    #[test]
    fn contains_missing() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let fake = [0xFF_u8; 16];
        assert!(!dict.contains(&fake).unwrap());
    }

    // ── Batch insert tests ──

    #[test]
    fn batch_insert() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let strings = [
            "http://example.org/s1",
            "http://example.org/s2",
            "http://example.org/s3",
            "http://example.org/s4",
            "http://example.org/s5",
        ];
        let hashes = dict.batch_insert(&strings).unwrap();
        assert_eq!(hashes.len(), 5);

        // All lookups succeed
        for (i, h) in hashes.iter().enumerate() {
            let looked_up = dict.lookup(h).unwrap();
            assert_eq!(looked_up, strings[i]);
        }
    }

    #[test]
    fn batch_insert_returns_correct_count() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let strings: Vec<String> =
            (0..20).map(|i| format!("http://example.org/item/{i}")).collect();
        let refs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
        let hashes = dict.batch_insert(&refs).unwrap();
        assert_eq!(hashes.len(), 20);
    }

    #[test]
    fn batch_insert_idempotent_with_existing() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        // Pre-insert one
        let pre_h = dict.insert("http://example.org/pre").unwrap();

        let strings = ["http://example.org/pre", "http://example.org/new"];
        let hashes = dict.batch_insert(&strings).unwrap();

        assert_eq!(hashes[0], pre_h, "existing entry should get same hash");
        assert_eq!(dict.lookup(&hashes[1]).unwrap(), "http://example.org/new");
    }

    // ── Remove tests ──

    #[test]
    fn remove_existing() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let h = dict.insert("http://example.org/temp").unwrap();
        assert!(dict.contains(&h).unwrap());

        dict.remove(&h).unwrap();
        assert!(!dict.contains(&h).unwrap());
        assert!(dict.lookup(&h).is_err());
    }

    #[test]
    fn remove_missing() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let fake = [0x00_u8; 16];
        // Should not error on missing key
        dict.remove(&fake).unwrap();
    }

    // ── Collision resistance test ──

    #[test]
    fn insert_many_no_collisions() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let n = 1000;
        let strings: Vec<String> =
            (0..n).map(|i| format!("http://example.org/resource/{i}")).collect();
        let refs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();

        let hashes = dict.batch_insert(&refs).unwrap();
        assert_eq!(hashes.len(), n);

        // Verify all hashes are unique
        let mut unique = std::collections::HashSet::new();
        for h in &hashes {
            assert!(unique.insert(*h), "hash collision detected!");
        }

        // Verify all lookups return correct strings
        for (i, h) in hashes.iter().enumerate() {
            assert_eq!(dict.lookup(h).unwrap(), strings[i]);
        }
    }

    // ── String preservation tests ──

    #[test]
    fn lookup_preserves_exact_string() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);

        let cases = [
            "hello world",
            "  leading spaces",
            "trailing spaces  ",
            "line\nbreak",
            "tab\there",
            "\"quoted\"",
            "emoji 🎉",
            "mixed\r\n\tlines",
        ];

        for original in &cases {
            let h = dict.insert(original).unwrap();
            let retrieved = dict.lookup(&h).unwrap();
            assert_eq!(retrieved, *original, "string not preserved for: {:?}", original);
        }
    }

    #[test]
    fn lookup_preserves_uri_forms() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);

        let uris = [
            "http://schema.org/name",
            "https://www.w3.org/2019/wot/td",
            "urn:isbn:0451450523",
            "did:neunode:0xABC123DEF456",
            "ipfs://QmX7bN4MDjGfioRrGmJqVhMiFpHgBpMkv",
            "neunode:agent:capability/training",
        ];

        for uri in &uris {
            let h = dict.insert(uri).unwrap();
            assert_eq!(dict.lookup(&h).unwrap(), *uri);
        }
    }

    // ── Edge case tests ──

    #[test]
    fn insert_empty_string() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let h = dict.insert("").unwrap();
        assert_eq!(dict.lookup(&h).unwrap(), "");
    }

    #[test]
    fn insert_single_byte() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let h = dict.insert("a").unwrap();
        assert_eq!(dict.lookup(&h).unwrap(), "a");
    }

    #[test]
    fn insert_null_byte_in_string() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let s = "before\0after";
        let h = dict.insert(s).unwrap();
        assert_eq!(dict.lookup(&h).unwrap(), s);
    }

    #[test]
    fn new_creates_valid_dict() {
        let db = temp_db();
        let _dict = StringDictionary::new(&db);
        // If we get here, construction succeeded
    }

    #[test]
    fn multiple_dicts_same_db() {
        let db = temp_db();
        let dict1 = StringDictionary::new(&db);
        let dict2 = StringDictionary::new(&db);

        let h1 = dict1.insert("shared_string").unwrap();
        let result = dict2.lookup(&h1).unwrap();
        assert_eq!(result, "shared_string", "two dicts on same db should share data");
    }

    #[test]
    fn remove_and_reinsert() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let s = "http://example.org/ephemeral";

        let h = dict.insert(s).unwrap();
        dict.remove(&h).unwrap();
        assert!(dict.lookup(&h).is_err());

        // Reinsert should work and return same hash (deterministic)
        let h2 = dict.insert(s).unwrap();
        assert_eq!(h2, h);
        assert_eq!(dict.lookup(&h2).unwrap(), s);
    }

    #[test]
    fn batch_insert_empty() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let hashes = dict.batch_insert(&[]).unwrap();
        assert!(hashes.is_empty());
    }
}
