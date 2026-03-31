use neunode_storage::cf::*;
use neunode_storage::db::NeunodeDb;

use crate::dictionary::{Hash128, StringDictionary};
use crate::error::{KnowledgeError, Result};
use crate::triple::{Quad, TripleCodec};

/// A query pattern where `Some(component)` is bound and `None` is a wildcard.
///
/// The engine chooses the most selective index based on which components
/// are bound, then performs a prefix scan and filters results.
#[derive(Debug, Clone, Default, Hash, Eq, PartialEq)]
pub struct QueryPattern {
    pub subject: Option<Hash128>,
    pub predicate: Option<Hash128>,
    pub object: Option<Hash128>,
    pub graph: Option<Hash128>,
}

/// A resolved query result with human-readable string components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryResult {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub graph: String,
}

/// Knowledge graph query engine.
///
/// Performs prefix-scan queries across the 6 permutation indexes (SPOG, POSG,
/// OSPG, GSPO, GPOS, GOSP) to find matching quads efficiently.
pub struct QueryEngine<'a> {
    db: &'a NeunodeDb,
    dict: &'a StringDictionary<'a>,
}

impl<'a> QueryEngine<'a> {
    /// Create a new query engine backed by the given database and dictionary.
    pub fn new(db: &'a NeunodeDb, dict: &'a StringDictionary<'a>) -> Self {
        Self { db, dict }
    }

    /// Execute a query pattern, returning all matching quads as strings.
    ///
    /// 1. Choose the best index for the pattern.
    /// 2. Prefix-scan that index.
    /// 3. Decode keys and filter against all bound components.
    /// 4. Resolve hashes to strings via the dictionary.
    pub fn query(&self, pattern: &QueryPattern) -> Result<Vec<QueryResult>> {
        let quads = self.query_quads(pattern)?;
        let mut results = Vec::with_capacity(quads.len());
        for quad in &quads {
            let (s, p, o, g) = quad.to_strings(self.dict)?;
            results.push(QueryResult { subject: s, predicate: p, object: o, graph: g });
        }
        Ok(results)
    }

    /// Count matching quads without materializing string results.
    pub fn count(&self, pattern: &QueryPattern) -> Result<usize> {
        let (cf_name, prefix) = self.choose_index(pattern)?;
        let raw = self
            .db
            .prefix_scan(cf_name, &prefix)
            .map_err(|e| KnowledgeError::StorageError(format!("prefix_scan on {cf_name}: {e}")))?;
        let mut count = 0usize;
        for (key, _val) in &raw {
            if self.matches_pattern(cf_name, key, pattern)? {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Query returning raw `Quad`s (no string resolution, faster for bulk ops).
    pub fn query_quads(&self, pattern: &QueryPattern) -> Result<Vec<Quad>> {
        let (cf_name, prefix) = self.choose_index(pattern)?;
        let raw = self
            .db
            .prefix_scan(cf_name, &prefix)
            .map_err(|e| KnowledgeError::StorageError(format!("prefix_scan on {cf_name}: {e}")))?;
        let mut quads = Vec::with_capacity(raw.len());
        for (key, _val) in &raw {
            if self.matches_pattern(cf_name, key, pattern)? {
                quads.push(decode_index_key_to_quad(cf_name, key)?);
            }
        }
        Ok(quads)
    }

    /// Choose the most selective index for a given query pattern.
    ///
    /// Returns `(column_family_name, prefix_bytes)`.
    ///
    /// Priority (most selective first):
    /// - S bound → SPOG index
    /// - P bound (no S) → POSG index
    /// - O bound (no S, no P) → OSPG index
    /// - G bound (no S, no P, no O) → GSPO index
    /// - All wildcards → error (too broad)
    fn choose_index(&self, pattern: &QueryPattern) -> Result<(&'static str, Vec<u8>)> {
        let s = pattern.subject;
        let p = pattern.predicate;
        let o = pattern.object;
        let g = pattern.graph;

        if let Some(sh) = s {
            let mut prefix = Vec::with_capacity(48);
            prefix.extend_from_slice(&sh);
            if let Some(ph) = p {
                prefix.extend_from_slice(&ph);
                if let Some(oh) = o {
                    prefix.extend_from_slice(&oh);
                }
            }
            return Ok((CF_KG_SPOG, prefix));
        }

        if let Some(ph) = p {
            let mut prefix = Vec::with_capacity(32);
            prefix.extend_from_slice(&ph);
            if let Some(oh) = o {
                prefix.extend_from_slice(&oh);
            }
            return Ok((CF_KG_POSG, prefix));
        }

        if let Some(oh) = o {
            let mut prefix = Vec::with_capacity(16);
            prefix.extend_from_slice(&oh);
            return Ok((CF_KG_OSPG, prefix));
        }

        if let Some(gh) = g {
            let mut prefix = Vec::with_capacity(16);
            prefix.extend_from_slice(&gh);
            return Ok((CF_KG_GSPO, prefix));
        }

        Err(KnowledgeError::QueryFailed(
            "query pattern has no bound components — all-wildcard queries are not supported"
                .to_string(),
        ))
    }

    /// Check whether a decoded index key matches ALL bound components in the pattern.
    ///
    /// The prefix scan may return extra results (e.g., a 16-byte S prefix
    /// returns all quads with that subject regardless of P/O/G bindings),
    /// so we must post-filter.
    fn matches_pattern(&self, index: &str, key: &[u8], pattern: &QueryPattern) -> Result<bool> {
        let (s, p, o, g) = decode_index_key_components(index, key)?;
        if let Some(ps) = &pattern.subject {
            if &s != ps {
                return Ok(false);
            }
        }
        if let Some(pp) = &pattern.predicate {
            if &p != pp {
                return Ok(false);
            }
        }
        if let Some(po) = &pattern.object {
            if &o != po {
                return Ok(false);
            }
        }
        if let Some(pg) = &pattern.graph {
            if &g != pg {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// Decode an index key into its (S, P, O, G) components.
///
/// Each index stores its 4 components in a different order; this function
/// maps back to canonical (subject, predicate, object, graph).
fn decode_index_key_components(
    index: &str,
    key: &[u8],
) -> Result<(Hash128, Hash128, Hash128, Hash128)> {
    let (a, b, c, d) = TripleCodec::decode_key(key)?;
    match index {
        // SPOG: (S, P, O, G)
        CF_KG_SPOG => Ok((a, b, c, d)),
        // POSG: (P, O, S, G) → a=P, b=O, c=S, d=G
        CF_KG_POSG => Ok((c, a, b, d)),
        // OSPG: (O, S, P, G) → a=O, b=S, c=P, d=G
        CF_KG_OSPG => Ok((b, c, a, d)),
        // GSPO: (G, S, P, O) → a=G, b=S, c=P, d=O
        CF_KG_GSPO => Ok((b, c, d, a)),
        // GPOS: (G, P, O, S) → a=G, b=P, c=O, d=S
        CF_KG_GPOS => Ok((d, b, c, a)),
        // GOSP: (G, O, S, P) → a=G, b=O, c=S, d=P
        CF_KG_GOSP => Ok((c, d, b, a)),
        _ => Err(KnowledgeError::QueryFailed(format!("unknown index: {index}"))),
    }
}

/// Decode an index key into a `Quad`.
fn decode_index_key_to_quad(index: &str, key: &[u8]) -> Result<Quad> {
    let (s, p, o, g) = decode_index_key_components(index, key)?;
    Ok(Quad { subject: s, predicate: p, object: o, graph: g })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_db() -> NeunodeDb {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("neunode_kg_query_{:?}_{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&dir);
        NeunodeDb::open(&dir).unwrap()
    }

    fn insert_quad(
        dict: &StringDictionary,
        db: &NeunodeDb,
        s: &str,
        p: &str,
        o: &str,
        g: &str,
    ) -> Quad {
        let q = Quad::from_strings(dict, s, p, o, g).unwrap();
        q.insert_indexes(db).unwrap();
        q
    }

    fn pat() -> QueryPattern {
        QueryPattern::default()
    }

    fn hash(s: &str) -> Hash128 {
        StringDictionary::hash(s)
    }

    // ── choose_index tests ──

    #[test]
    fn choose_index_subject() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);
        let p = QueryPattern { subject: Some(hash("s1")), ..pat() };
        let (cf, prefix) = engine.choose_index(&p).unwrap();
        assert_eq!(cf, CF_KG_SPOG);
        assert_eq!(prefix.len(), 16);
    }

    #[test]
    fn choose_index_subject_predicate() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);
        let p = QueryPattern { subject: Some(hash("s1")), predicate: Some(hash("p1")), ..pat() };
        let (cf, prefix) = engine.choose_index(&p).unwrap();
        assert_eq!(cf, CF_KG_SPOG);
        assert_eq!(prefix.len(), 32);
        assert_eq!(&prefix[..16], &hash("s1")[..]);
        assert_eq!(&prefix[16..], &hash("p1")[..]);
    }

    #[test]
    fn choose_index_subject_predicate_object() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);
        let p = QueryPattern {
            subject: Some(hash("s1")),
            predicate: Some(hash("p1")),
            object: Some(hash("o1")),
            ..pat()
        };
        let (cf, prefix) = engine.choose_index(&p).unwrap();
        assert_eq!(cf, CF_KG_SPOG);
        assert_eq!(prefix.len(), 48);
    }

    #[test]
    fn choose_index_predicate() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);
        let p = QueryPattern { predicate: Some(hash("p1")), ..pat() };
        let (cf, prefix) = engine.choose_index(&p).unwrap();
        assert_eq!(cf, CF_KG_POSG);
        assert_eq!(prefix.len(), 16);
    }

    #[test]
    fn choose_index_predicate_object() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);
        let p = QueryPattern { predicate: Some(hash("p1")), object: Some(hash("o1")), ..pat() };
        let (cf, prefix) = engine.choose_index(&p).unwrap();
        assert_eq!(cf, CF_KG_POSG);
        assert_eq!(prefix.len(), 32);
    }

    #[test]
    fn choose_index_object() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);
        let p = QueryPattern { object: Some(hash("o1")), ..pat() };
        let (cf, prefix) = engine.choose_index(&p).unwrap();
        assert_eq!(cf, CF_KG_OSPG);
        assert_eq!(prefix.len(), 16);
    }

    #[test]
    fn choose_index_graph() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);
        let p = QueryPattern { graph: Some(hash("g1")), ..pat() };
        let (cf, prefix) = engine.choose_index(&p).unwrap();
        assert_eq!(cf, CF_KG_GSPO);
        assert_eq!(prefix.len(), 16);
    }

    #[test]
    fn choose_index_no_bounds() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);
        let p = QueryPattern { ..pat() };
        let result = engine.choose_index(&p);
        assert!(result.is_err());
        match result.unwrap_err() {
            KnowledgeError::QueryFailed(msg) => {
                assert!(msg.contains("no bound components"));
            }
            other => panic!("expected QueryFailed, got: {other:?}"),
        }
    }

    // ── Query tests ──

    #[test]
    fn query_by_subject() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);

        let shared_s = "http://example.org/agent/alice";
        insert_quad(&dict, &db, shared_s, "p1", "o1", "g1");
        insert_quad(&dict, &db, shared_s, "p2", "o2", "g2");
        insert_quad(&dict, &db, shared_s, "p3", "o3", "g3");
        // Unrelated
        insert_quad(&dict, &db, "other_s", "p4", "o4", "g4");

        let p = QueryPattern { subject: Some(hash(shared_s)), ..pat() };
        let results = engine.query(&p).unwrap();
        assert_eq!(results.len(), 3);
        for r in &results {
            assert_eq!(r.subject, shared_s);
        }
    }

    #[test]
    fn query_by_predicate() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);

        let shared_p = "http://schema.org/knows";
        insert_quad(&dict, &db, "s1", shared_p, "o1", "g1");
        insert_quad(&dict, &db, "s2", shared_p, "o2", "g2");
        insert_quad(&dict, &db, "s3", shared_p, "o3", "g3");
        // Unrelated quad
        insert_quad(&dict, &db, "s4", "other_p", "o4", "g4");

        let p = QueryPattern { predicate: Some(hash(shared_p)), ..pat() };
        let results = engine.query(&p).unwrap();
        assert_eq!(results.len(), 3);
        for r in &results {
            assert_eq!(r.predicate, shared_p);
        }
    }

    #[test]
    fn query_by_graph() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);

        let shared_g = "http://example.org/graph/social";
        insert_quad(&dict, &db, "s1", "p1", "o1", shared_g);
        insert_quad(&dict, &db, "s2", "p2", "o2", shared_g);
        // Unrelated graph
        insert_quad(&dict, &db, "s3", "p3", "o3", "other_g");

        let p = QueryPattern { graph: Some(hash(shared_g)), ..pat() };
        let results = engine.query(&p).unwrap();
        assert_eq!(results.len(), 2);
        for r in &results {
            assert_eq!(r.graph, shared_g);
        }
    }

    #[test]
    fn query_by_object() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);

        let shared_o = "http://example.org/agent/bob";
        insert_quad(&dict, &db, "s1", "p1", shared_o, "g1");
        insert_quad(&dict, &db, "s2", "p2", shared_o, "g2");
        // Unrelated
        insert_quad(&dict, &db, "s3", "p3", "other_o", "g3");

        let p = QueryPattern { object: Some(hash(shared_o)), ..pat() };
        let results = engine.query(&p).unwrap();
        assert_eq!(results.len(), 2);
        for r in &results {
            assert_eq!(r.object, shared_o);
        }
    }

    #[test]
    fn query_subject_predicate() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);

        insert_quad(&dict, &db, "s1", "p1", "o1", "g1");
        insert_quad(&dict, &db, "s1", "p1", "o2", "g2");
        insert_quad(&dict, &db, "s1", "p2", "o3", "g3");
        insert_quad(&dict, &db, "s2", "p1", "o4", "g4");

        let p = QueryPattern { subject: Some(hash("s1")), predicate: Some(hash("p1")), ..pat() };
        let results = engine.query(&p).unwrap();
        assert_eq!(results.len(), 2);
        for r in &results {
            assert_eq!(r.subject, "s1");
            assert_eq!(r.predicate, "p1");
        }
    }

    #[test]
    fn query_empty_result() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);

        insert_quad(&dict, &db, "s1", "p1", "o1", "g1");

        let p = QueryPattern { subject: Some(hash("nonexistent")), ..pat() };
        let results = engine.query(&p).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn count_matches() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);

        let shared_s = "http://example.org/agent/alice";
        insert_quad(&dict, &db, shared_s, "p1", "o1", "g1");
        insert_quad(&dict, &db, shared_s, "p2", "o2", "g2");
        insert_quad(&dict, &db, shared_s, "p3", "o3", "g3");

        let p = QueryPattern { subject: Some(hash(shared_s)), ..pat() };
        assert_eq!(engine.count(&p).unwrap(), 3);
        assert_eq!(engine.count(&p).unwrap(), engine.query(&p).unwrap().len());
    }

    #[test]
    fn query_quads_raw() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);

        let expected = insert_quad(&dict, &db, "s1", "p1", "o1", "g1");

        let p = QueryPattern { subject: Some(hash("s1")), ..pat() };
        let quads = engine.query_quads(&p).unwrap();
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0], expected);
    }

    #[test]
    fn query_resolves_strings() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);

        insert_quad(
            &dict,
            &db,
            "http://example.org/agent/alice",
            "http://schema.org/knows",
            "http://example.org/agent/bob",
            "http://example.org/graph/social",
        );

        let p = QueryPattern { subject: Some(hash("http://example.org/agent/alice")), ..pat() };
        let results = engine.query(&p).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].subject, "http://example.org/agent/alice");
        assert_eq!(results[0].predicate, "http://schema.org/knows");
        assert_eq!(results[0].object, "http://example.org/agent/bob");
        assert_eq!(results[0].graph, "http://example.org/graph/social");
    }

    #[test]
    fn query_multiple_graphs() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);

        // Same SPO in different graphs = distinct quads
        insert_quad(&dict, &db, "s1", "p1", "o1", "g1");
        insert_quad(&dict, &db, "s1", "p1", "o1", "g2");

        let p = QueryPattern {
            subject: Some(hash("s1")),
            predicate: Some(hash("p1")),
            object: Some(hash("o1")),
            ..pat()
        };
        let results = engine.query(&p).unwrap();
        assert_eq!(results.len(), 2);

        let graphs: Vec<&str> = results.iter().map(|r| r.graph.as_str()).collect();
        assert!(graphs.contains(&"g1"));
        assert!(graphs.contains(&"g2"));
    }

    #[test]
    fn query_predicate_object() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);

        insert_quad(&dict, &db, "s1", "p1", "o1", "g1");
        insert_quad(&dict, &db, "s2", "p1", "o1", "g2");
        insert_quad(&dict, &db, "s3", "p1", "o2", "g3");

        let p = QueryPattern { predicate: Some(hash("p1")), object: Some(hash("o1")), ..pat() };
        let results = engine.query(&p).unwrap();
        assert_eq!(results.len(), 2);
        for r in &results {
            assert_eq!(r.predicate, "p1");
            assert_eq!(r.object, "o1");
        }
    }

    // ── Roundtrip decode tests ──

    #[test]
    fn decode_index_key_spog_roundtrip() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let quad = Quad::from_strings(&dict, "s", "p", "o", "g").unwrap();

        // SPOG key = (S, P, O, G)
        let key =
            TripleCodec::encode_key(&quad.subject, &quad.predicate, &quad.object, &quad.graph);
        let decoded = decode_index_key_to_quad(CF_KG_SPOG, &key).unwrap();
        assert_eq!(decoded, quad);
    }

    #[test]
    fn decode_index_key_posg_roundtrip() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let quad = Quad::from_strings(&dict, "s", "p", "o", "g").unwrap();

        // POSG key = (P, O, S, G)
        let key =
            TripleCodec::encode_key(&quad.predicate, &quad.object, &quad.subject, &quad.graph);
        let decoded = decode_index_key_to_quad(CF_KG_POSG, &key).unwrap();
        assert_eq!(decoded, quad);
    }

    #[test]
    fn decode_index_key_ospg_roundtrip() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let quad = Quad::from_strings(&dict, "s", "p", "o", "g").unwrap();

        // OSPG key = (O, S, P, G)
        let key =
            TripleCodec::encode_key(&quad.object, &quad.subject, &quad.predicate, &quad.graph);
        let decoded = decode_index_key_to_quad(CF_KG_OSPG, &key).unwrap();
        assert_eq!(decoded, quad);
    }

    #[test]
    fn decode_index_key_gspo_roundtrip() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let quad = Quad::from_strings(&dict, "s", "p", "o", "g").unwrap();

        // GSPO key = (G, S, P, O)
        let key =
            TripleCodec::encode_key(&quad.graph, &quad.subject, &quad.predicate, &quad.object);
        let decoded = decode_index_key_to_quad(CF_KG_GSPO, &key).unwrap();
        assert_eq!(decoded, quad);
    }

    #[test]
    fn decode_index_key_gpos_roundtrip() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let quad = Quad::from_strings(&dict, "s", "p", "o", "g").unwrap();

        // GPOS key = (G, P, O, S)
        let key =
            TripleCodec::encode_key(&quad.graph, &quad.predicate, &quad.object, &quad.subject);
        let decoded = decode_index_key_to_quad(CF_KG_GPOS, &key).unwrap();
        assert_eq!(decoded, quad);
    }

    #[test]
    fn decode_index_key_gosp_roundtrip() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let quad = Quad::from_strings(&dict, "s", "p", "o", "g").unwrap();

        // GOSP key = (G, O, S, P)
        let key =
            TripleCodec::encode_key(&quad.graph, &quad.object, &quad.subject, &quad.predicate);
        let decoded = decode_index_key_to_quad(CF_KG_GOSP, &key).unwrap();
        assert_eq!(decoded, quad);
    }

    // ── Edge case tests ──

    #[test]
    fn query_exact_quad() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);

        insert_quad(&dict, &db, "s1", "p1", "o1", "g1");
        insert_quad(&dict, &db, "s1", "p1", "o2", "g1");

        // Bind all 4 components
        let p = QueryPattern {
            subject: Some(hash("s1")),
            predicate: Some(hash("p1")),
            object: Some(hash("o1")),
            graph: Some(hash("g1")),
        };
        let results = engine.query(&p).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].object, "o1");
    }

    #[test]
    fn count_empty_pattern() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let engine = QueryEngine::new(&db, &dict);

        let p = QueryPattern { ..pat() };
        let result = engine.count(&p);
        assert!(result.is_err());
    }
}
