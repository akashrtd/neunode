

use neunode_storage::cf::*;
use neunode_storage::db::NeunodeDb;

use crate::dictionary::{Hash128, StringDictionary};
use crate::error::{KnowledgeError, Result};

/// The 6 KG index column families.
#[cfg(test)]
const INDEX_CFS: &[&str] =
    &[CF_KG_SPOG, CF_KG_POSG, CF_KG_OSPG, CF_KG_GSPO, CF_KG_GPOS, CF_KG_GOSP];

/// An RDF-style quad: (Subject, Predicate, Object, Graph).
///
/// Each component is stored as its 16-byte SipHash24 hash.
/// Quads are indexed in 6 permutation column families for efficient prefix scans.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Quad {
    pub subject: Hash128,
    pub predicate: Hash128,
    pub object: Hash128,
    pub graph: Hash128,
}

/// Encoding/decoding for 64-byte index keys (4 × Hash128 concatenated).
pub struct TripleCodec;

impl TripleCodec {
    /// Encode 4 hashes into a 64-byte key.
    pub fn encode_key(a: &Hash128, b: &Hash128, c: &Hash128, d: &Hash128) -> [u8; 64] {
        let mut key = [0u8; 64];
        key[..16].copy_from_slice(a);
        key[16..32].copy_from_slice(b);
        key[32..48].copy_from_slice(c);
        key[48..64].copy_from_slice(d);
        key
    }

    /// Decode a 64-byte key back into 4 hashes.
    ///
    /// Returns `KnowledgeError::InvalidTriple` if the key is not exactly 64 bytes.
    pub fn decode_key(key: &[u8]) -> Result<(Hash128, Hash128, Hash128, Hash128)> {
        if key.len() != 64 {
            return Err(KnowledgeError::InvalidTriple(format!(
                "index key must be 64 bytes, got {}",
                key.len()
            )));
        }
        let mut a = [0u8; 16];
        let mut b = [0u8; 16];
        let mut c = [0u8; 16];
        let mut d = [0u8; 16];
        a.copy_from_slice(&key[..16]);
        b.copy_from_slice(&key[16..32]);
        c.copy_from_slice(&key[32..48]);
        d.copy_from_slice(&key[48..64]);
        Ok((a, b, c, d))
    }
}

impl Quad {
    /// Create a Quad from 4 string components using the dictionary.
    ///
    /// Inserts all 4 strings into the dictionary and returns the Quad
    /// with their 128-bit hashes.
    pub fn from_strings(
        dict: &StringDictionary,
        s: &str,
        p: &str,
        o: &str,
        g: &str,
    ) -> Result<Self> {
        let hashes = dict
            .batch_insert(&[s, p, o, g])
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))?;
        Ok(Self { subject: hashes[0], predicate: hashes[1], object: hashes[2], graph: hashes[3] })
    }

    /// Look up the original strings for this quad using the dictionary.
    pub fn to_strings(&self, dict: &StringDictionary) -> Result<(String, String, String, String)> {
        let s = dict.lookup(&self.subject)?;
        let p = dict.lookup(&self.predicate)?;
        let o = dict.lookup(&self.object)?;
        let g = dict.lookup(&self.graph)?;
        Ok((s, p, o, g))
    }

    /// Compute the 6 index keys for this quad.
    ///
    /// Returns an array of (64-byte key, CF name) pairs, one per index.
    fn index_keys(&self) -> [([u8; 64], &str); 6] {
        let (s, p, o, g) = (&self.subject, &self.predicate, &self.object, &self.graph);
        [
            (TripleCodec::encode_key(s, p, o, g), CF_KG_SPOG),
            (TripleCodec::encode_key(p, o, s, g), CF_KG_POSG),
            (TripleCodec::encode_key(o, s, p, g), CF_KG_OSPG),
            (TripleCodec::encode_key(g, s, p, o), CF_KG_GSPO),
            (TripleCodec::encode_key(g, p, o, s), CF_KG_GPOS),
            (TripleCodec::encode_key(g, o, s, p), CF_KG_GOSP),
        ]
    }

    /// Write this quad to all 6 index CFs using a WriteBatch.
    ///
    /// Each index gets a key-only entry (empty value), following the Oxigraph pattern.
    pub fn insert_indexes(&self, db: &NeunodeDb) -> Result<()> {
        let keys = self.index_keys();
        let mut ops = Vec::with_capacity(6);
        for (key, cf_name) in &keys {
            ops.push((*cf_name, &key[..], &b""[..]));
        }
        db.batch_put_raw(&ops).map_err(|e| KnowledgeError::StorageError(e.to_string()))
    }

    /// Remove this quad from all 6 index CFs using a WriteBatch.
    pub fn delete_indexes(&self, db: &NeunodeDb) -> Result<()> {
        let keys = self.index_keys();
        let mut ops = Vec::with_capacity(6);
        for (key, cf_name) in &keys {
            ops.push((*cf_name, &key[..]));
        }
        db.batch_delete_raw(&ops).map_err(|e| KnowledgeError::StorageError(e.to_string()))
    }

    /// Build a single-component prefix (16 bytes) for prefix-scan queries.
    ///
    /// The returned prefix is the first hash of the named index.
    /// E.g., `prefix_for("spog", &subject)` gives first 16 bytes of the SPOG key.
    pub fn prefix_for(_index: &str, component: &Hash128) -> Vec<u8> {
        component.to_vec()
    }

    /// Build a 2-component prefix (32 bytes) for prefix-scan queries.
    pub fn prefix_for2(_index: &str, c1: &Hash128, c2: &Hash128) -> Vec<u8> {
        let mut prefix = Vec::with_capacity(32);
        prefix.extend_from_slice(c1);
        prefix.extend_from_slice(c2);
        prefix
    }

    /// Check if this quad exists in a specific index.
    pub fn exists_in(&self, db: &NeunodeDb, index: &str) -> Result<bool> {
        let key = match index {
            CF_KG_SPOG => {
                TripleCodec::encode_key(&self.subject, &self.predicate, &self.object, &self.graph)
            }
            CF_KG_POSG => {
                TripleCodec::encode_key(&self.predicate, &self.object, &self.subject, &self.graph)
            }
            CF_KG_OSPG => {
                TripleCodec::encode_key(&self.object, &self.subject, &self.predicate, &self.graph)
            }
            CF_KG_GSPO => {
                TripleCodec::encode_key(&self.graph, &self.subject, &self.predicate, &self.object)
            }
            CF_KG_GPOS => {
                TripleCodec::encode_key(&self.graph, &self.predicate, &self.object, &self.subject)
            }
            CF_KG_GOSP => {
                TripleCodec::encode_key(&self.graph, &self.object, &self.subject, &self.predicate)
            }
            other => return Err(KnowledgeError::QueryFailed(format!("unknown index: {other}"))),
        };
        match db.get_raw(index, &key) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(KnowledgeError::StorageError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_db() -> NeunodeDb {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("neunode_kg_triple_{:?}_{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&dir);
        NeunodeDb::open(&dir).unwrap()
    }

    fn make_quad(db: &NeunodeDb) -> Quad {
        let dict = StringDictionary::new(db);
        Quad::from_strings(
            &dict,
            "http://example.org/agent/alice",
            "http://schema.org/knows",
            "http://example.org/agent/bob",
            "http://example.org/graph/social",
        )
        .unwrap()
    }

    // ── TripleCodec tests ──

    #[test]
    fn encode_decode_roundtrip() {
        let a = StringDictionary::hash("subject");
        let b = StringDictionary::hash("predicate");
        let c = StringDictionary::hash("object");
        let d = StringDictionary::hash("graph");
        let key = TripleCodec::encode_key(&a, &b, &c, &d);
        let (ra, rb, rc, rd) = TripleCodec::decode_key(&key).unwrap();
        assert_eq!(ra, a);
        assert_eq!(rb, b);
        assert_eq!(rc, c);
        assert_eq!(rd, d);
    }

    #[test]
    fn decode_wrong_length() {
        let short = vec![0u8; 63];
        let result = TripleCodec::decode_key(&short);
        assert!(result.is_err());
        match result.unwrap_err() {
            KnowledgeError::InvalidTriple(msg) => {
                assert!(msg.contains("63"), "error should mention length, got: {msg}");
            }
            other => panic!("expected InvalidTriple, got: {other:?}"),
        }
    }

    #[test]
    fn decode_empty() {
        let result = TripleCodec::decode_key(&[]);
        assert!(result.is_err());
        match result.unwrap_err() {
            KnowledgeError::InvalidTriple(msg) => {
                assert!(msg.contains("0"), "error should mention length 0, got: {msg}");
            }
            other => panic!("expected InvalidTriple, got: {other:?}"),
        }
    }

    #[test]
    fn encode_key_deterministic() {
        let a = StringDictionary::hash("s");
        let b = StringDictionary::hash("p");
        let c = StringDictionary::hash("o");
        let d = StringDictionary::hash("g");
        let k1 = TripleCodec::encode_key(&a, &b, &c, &d);
        let k2 = TripleCodec::encode_key(&a, &b, &c, &d);
        assert_eq!(k1, k2, "same inputs must produce same key");
    }

    // ── Quad construction tests ──

    #[test]
    fn quad_from_strings() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let quad = Quad::from_strings(
            &dict,
            "http://example.org/s",
            "http://example.org/p",
            "http://example.org/o",
            "http://example.org/g",
        )
        .unwrap();
        assert_eq!(quad.subject, StringDictionary::hash("http://example.org/s"));
        assert_eq!(quad.predicate, StringDictionary::hash("http://example.org/p"));
        assert_eq!(quad.object, StringDictionary::hash("http://example.org/o"));
        assert_eq!(quad.graph, StringDictionary::hash("http://example.org/g"));
    }

    #[test]
    fn quad_to_strings() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let quad = Quad::from_strings(
            &dict,
            "http://example.org/agent/alice",
            "http://schema.org/knows",
            "http://example.org/agent/bob",
            "http://example.org/graph/social",
        )
        .unwrap();
        let (s, p, o, g) = quad.to_strings(&dict).unwrap();
        assert_eq!(s, "http://example.org/agent/alice");
        assert_eq!(p, "http://schema.org/knows");
        assert_eq!(o, "http://example.org/agent/bob");
        assert_eq!(g, "http://example.org/graph/social");
    }

    #[test]
    fn quad_equality() {
        let db = temp_db();
        let q1 = make_quad(&db);
        let q2 = make_quad(&db);
        assert_eq!(q1, q2, "quads from same strings should be equal");
    }

    #[test]
    fn quad_inequality() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let q1 = Quad::from_strings(&dict, "s1", "p", "o", "g").unwrap();
        let q2 = Quad::from_strings(&dict, "s2", "p", "o", "g").unwrap();
        assert_ne!(q1, q2, "different subjects should produce unequal quads");
    }

    #[test]
    fn quad_with_empty_graph() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let quad = Quad::from_strings(&dict, "s", "p", "o", "").unwrap();
        assert_eq!(quad.graph, StringDictionary::hash(""));
    }

    #[test]
    fn quad_with_long_uris() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let long =
            "http://very.long.namespace.example.org/deep/path/to/resource/with/many/segments";
        let quad = Quad::from_strings(&dict, long, long, long, long).unwrap();
        let (s, _p, _o, _g) = quad.to_strings(&dict).unwrap();
        assert_eq!(s, long);
    }

    #[test]
    fn quad_hash_stability() {
        let h1 = StringDictionary::hash("agent:did:123");
        let h2 = StringDictionary::hash("agent:did:123");
        assert_eq!(h1, h2, "hash must be deterministic across calls");
    }

    // ── Index key tests ──

    #[test]
    fn index_keys_correct_count() {
        let db = temp_db();
        let quad = make_quad(&db);
        let keys = quad.index_keys();
        assert_eq!(keys.len(), 6, "must generate exactly 6 index keys");
    }

    #[test]
    fn index_keys_unique() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let quad = Quad::from_strings(
            &dict,
            "http://example.org/distinct_s",
            "http://example.org/distinct_p",
            "http://example.org/distinct_o",
            "http://example.org/distinct_g",
        )
        .unwrap();
        let keys = quad.index_keys();
        let mut seen = HashSet::new();
        for (key, cf) in &keys {
            assert!(seen.insert(*key), "duplicate key found in index {cf}");
        }
    }

    // ── Insert/delete tests ──

    #[test]
    fn insert_indexes_creates_entries() {
        let db = temp_db();
        let quad = make_quad(&db);
        quad.insert_indexes(&db).unwrap();

        for (key, cf_name) in &quad.index_keys() {
            let val = db.get_raw(cf_name, key).unwrap();
            assert!(val.is_some(), "key should exist in CF {cf_name}");
            assert_eq!(val.unwrap().len(), 0, "value should be empty (key-only index)");
        }
    }

    #[test]
    fn delete_indexes_removes_entries() {
        let db = temp_db();
        let quad = make_quad(&db);
        quad.insert_indexes(&db).unwrap();
        quad.delete_indexes(&db).unwrap();

        for (key, cf_name) in &quad.index_keys() {
            let val = db.get_raw(cf_name, key).unwrap();
            assert!(val.is_none(), "key should be gone from CF {cf_name}");
        }
    }

    #[test]
    fn exists_in_after_insert() {
        let db = temp_db();
        let quad = make_quad(&db);
        quad.insert_indexes(&db).unwrap();

        for cf in INDEX_CFS {
            assert!(quad.exists_in(&db, cf).unwrap(), "quad should exist in CF {cf}");
        }
    }

    #[test]
    fn exists_in_before_insert() {
        let db = temp_db();
        let quad = make_quad(&db);
        for cf in INDEX_CFS {
            assert!(
                !quad.exists_in(&db, cf).unwrap(),
                "quad should NOT exist in CF {cf} before insert"
            );
        }
    }

    // ── Prefix tests ──

    #[test]
    fn prefix_for_spog() {
        let subj = StringDictionary::hash("subject");
        let prefix = Quad::prefix_for(CF_KG_SPOG, &subj);
        assert_eq!(prefix.len(), 16);
        assert_eq!(&prefix[..], &subj[..]);
    }

    #[test]
    fn prefix_for2_spog() {
        let subj = StringDictionary::hash("subject");
        let pred = StringDictionary::hash("predicate");
        let prefix = Quad::prefix_for2(CF_KG_SPOG, &subj, &pred);
        assert_eq!(prefix.len(), 32);
        assert_eq!(&prefix[..16], &subj[..]);
        assert_eq!(&prefix[16..], &pred[..]);
    }

    #[test]
    fn insert_then_prefix_scan() {
        let db = temp_db();
        let quad = make_quad(&db);

        quad.insert_indexes(&db).unwrap();

        let prefix = Quad::prefix_for(CF_KG_SPOG, &quad.subject);
        let results = db.prefix_scan(CF_KG_SPOG, &prefix).unwrap();
        assert_eq!(results.len(), 1, "should find exactly one entry for this subject");
        assert!(results[0].0.starts_with(&prefix));
    }

    #[test]
    fn insert_multiple_quads() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);

        let shared_s = "http://example.org/agent/alice";
        let q1 = Quad::from_strings(&dict, shared_s, "p1", "o1", "g1").unwrap();
        let q2 = Quad::from_strings(&dict, shared_s, "p2", "o2", "g2").unwrap();
        let q3 = Quad::from_strings(&dict, shared_s, "p3", "o3", "g3").unwrap();

        q1.insert_indexes(&db).unwrap();
        q2.insert_indexes(&db).unwrap();
        q3.insert_indexes(&db).unwrap();

        let prefix = Quad::prefix_for(CF_KG_SPOG, &q1.subject);
        let results = db.prefix_scan(CF_KG_SPOG, &prefix).unwrap();
        assert_eq!(results.len(), 3, "prefix scan should find all 3 quads with same subject");
    }
}
