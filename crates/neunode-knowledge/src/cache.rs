use moka::sync::Cache;

use crate::query::{QueryPattern, QueryResult};

/// L1 in-process cache for knowledge graph query results.
///
/// Wraps a [`moka::sync::Cache`] keyed by [`QueryPattern`] with bounded capacity.
/// Results are cached after a query resolves and invalidated on any KG mutation
/// (conservative: `invalidate_all` on writes, `invalidate` for targeted removal).
///
/// Thread-safe: moka's `sync::Cache` uses interior mutability via concurrent
/// hash map, so `&KgCache` is safe to share across threads.
pub struct KgCache {
    inner: Cache<QueryPattern, Vec<QueryResult>>,
}

impl KgCache {
    /// Create a new cache with the given maximum capacity.
    ///
    /// When the cache exceeds `max_capacity`, moka evicts entries using an
    /// approximation of LRU (tinyLFU admission policy).
    pub fn new(max_capacity: usize) -> Self {
        let inner = Cache::builder().max_capacity(max_capacity as u64).build();
        Self { inner }
    }

    /// Look up a cached result for the given query pattern.
    ///
    /// Returns `None` on cache miss (pattern never queried or evicted).
    /// Returns `Some(results)` on hit — note this clones the result vector
    /// since moka returns owned values from its concurrent map.
    pub fn get(&self, pattern: &QueryPattern) -> Option<Vec<QueryResult>> {
        self.inner.get(pattern)
    }

    /// Insert a query result into the cache.
    ///
    /// Overwrites any existing entry for the same pattern.
    /// If the cache is at capacity, an existing entry may be evicted.
    pub fn put(&self, pattern: QueryPattern, results: Vec<QueryResult>) {
        self.inner.insert(pattern, results);
    }

    /// Invalidate a single cached entry.
    ///
    /// Use when a specific pattern's underlying data has changed.
    /// No-op if the pattern is not cached.
    pub fn invalidate(&self, pattern: &QueryPattern) {
        self.inner.invalidate(pattern);
    }

    /// Invalidate all cached entries.
    ///
    /// Called after any KG mutation (insert or delete of quads) to ensure
    /// stale results are never returned. This is the conservative invalidation
    /// strategy — a single quad mutation can affect many query patterns.
    pub fn invalidate_all(&self) {
        self.inner.invalidate_all();
    }

    /// Current number of entries in the cache.
    ///
    /// This is an approximate count due to the concurrent nature of moka.
    /// Prefer using `get()` for exact presence checks.
    pub fn len(&self) -> usize {
        self.inner.entry_count() as usize
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::StringDictionary;

    fn make_pattern(subject: &str) -> QueryPattern {
        QueryPattern { subject: Some(StringDictionary::hash(subject)), ..QueryPattern::default() }
    }

    fn make_pattern_full(s: &str, p: &str, o: &str, g: &str) -> QueryPattern {
        QueryPattern {
            subject: Some(StringDictionary::hash(s)),
            predicate: Some(StringDictionary::hash(p)),
            object: Some(StringDictionary::hash(o)),
            graph: Some(StringDictionary::hash(g)),
        }
    }

    fn make_result(s: &str, p: &str, o: &str, g: &str) -> QueryResult {
        QueryResult {
            subject: s.to_string(),
            predicate: p.to_string(),
            object: o.to_string(),
            graph: g.to_string(),
        }
    }

    // ── Basic get/put tests ──

    #[test]
    fn cache_miss_on_empty() {
        let cache = KgCache::new(100);
        let p = make_pattern("nonexistent");
        assert!(cache.get(&p).is_none());
    }

    #[test]
    fn cache_hit_after_put() {
        let cache = KgCache::new(100);
        let p = make_pattern("agent:alice");
        let results = vec![make_result("agent:alice", "knows", "agent:bob", "g1")];

        cache.put(p.clone(), results.clone());
        let cached = cache.get(&p);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), results);
    }

    #[test]
    fn cache_put_overwrites() {
        let cache = KgCache::new(100);
        let p = make_pattern("agent:alice");
        let v1 = vec![make_result("s1", "p1", "o1", "g1")];
        let v2 = vec![make_result("s2", "p2", "o2", "g2")];

        cache.put(p.clone(), v1);
        cache.put(p.clone(), v2.clone());

        let cached = cache.get(&p).unwrap();
        assert_eq!(cached, v2);
    }

    #[test]
    fn cache_multiple_patterns() {
        let cache = KgCache::new(100);

        let p1 = make_pattern("s1");
        let p2 = make_pattern("s2");
        let p3 = make_pattern("s3");

        cache.put(p1.clone(), vec![make_result("s1", "p1", "o1", "g1")]);
        cache.put(p2.clone(), vec![make_result("s2", "p2", "o2", "g2")]);
        cache.put(p3.clone(), vec![make_result("s3", "p3", "o3", "g3")]);

        assert!(cache.get(&p1).is_some());
        assert!(cache.get(&p2).is_some());
        assert!(cache.get(&p3).is_some());
    }

    #[test]
    fn cache_empty_results_vec() {
        let cache = KgCache::new(100);
        let p = make_pattern("empty_query");
        cache.put(p.clone(), vec![]);
        let cached = cache.get(&p).unwrap();
        assert!(cached.is_empty());
    }

    // ── Invalidation tests ──

    #[test]
    fn invalidate_single_pattern() {
        let cache = KgCache::new(100);
        let p1 = make_pattern("keep");
        let p2 = make_pattern("remove");

        cache.put(p1.clone(), vec![make_result("keep", "p", "o", "g")]);
        cache.put(p2.clone(), vec![make_result("remove", "p", "o", "g")]);

        assert!(cache.get(&p1).is_some());
        assert!(cache.get(&p2).is_some());

        cache.invalidate(&p2);

        assert!(cache.get(&p1).is_some(), "p1 should still be cached");
        assert!(cache.get(&p2).is_none(), "p2 should be invalidated");
    }

    #[test]
    fn invalidate_nonexistent_is_noop() {
        let cache = KgCache::new(100);
        let p = make_pattern("ghost");
        cache.invalidate(&p);
        assert!(cache.get(&p).is_none());
    }

    #[test]
    fn invalidate_all_clears_everything() {
        let cache = KgCache::new(100);

        for i in 0..10 {
            let p = make_pattern(&format!("s{i}"));
            cache.put(p.clone(), vec![make_result(&format!("s{i}"), "p", "o", "g")]);
        }

        // All 10 should be cached
        for i in 0..10 {
            assert!(cache.get(&make_pattern(&format!("s{i}"))).is_some());
        }

        cache.invalidate_all();

        for i in 0..10 {
            assert!(
                cache.get(&make_pattern(&format!("s{i}"))).is_none(),
                "entry s{i} should be gone after invalidate_all"
            );
        }
    }

    #[test]
    fn invalidate_all_then_refill() {
        let cache = KgCache::new(100);
        let p = make_pattern("refill");
        cache.put(p.clone(), vec![make_result("old", "p", "o", "g")]);

        cache.invalidate_all();
        assert!(cache.get(&p).is_none());

        let new_results = vec![make_result("new", "p", "o", "g")];
        cache.put(p.clone(), new_results.clone());

        let cached = cache.get(&p).unwrap();
        assert_eq!(cached, new_results);
    }

    // ── LRU eviction at capacity ──

    #[test]
    fn evicts_at_capacity() {
        let cache = KgCache::new(3);

        let p0 = make_pattern("s0");
        let p1 = make_pattern("s1");
        let p2 = make_pattern("s2");

        cache.put(p0.clone(), vec![make_result("s0", "p", "o", "g")]);
        cache.put(p1.clone(), vec![make_result("s1", "p", "o", "g")]);
        cache.put(p2.clone(), vec![make_result("s2", "p", "o", "g")]);

        // All 3 should be present
        assert!(cache.get(&p0).is_some());
        assert!(cache.get(&p1).is_some());
        assert!(cache.get(&p2).is_some());

        // Insert one more — should trigger eviction
        let p_new = make_pattern("s_new");
        cache.put(p_new.clone(), vec![make_result("s_new", "p", "o", "g")]);

        // New entry must be present
        assert!(cache.get(&p_new).is_some());
    }

    #[test]
    fn capacity_one_keeps_latest() {
        let cache = KgCache::new(1);

        cache.put(make_pattern("first"), vec![make_result("first", "p", "o", "g")]);
        cache.put(make_pattern("second"), vec![make_result("second", "p", "o", "g")]);

        assert!(cache.get(&make_pattern("second")).is_some());
    }

    // ── Pattern equality tests ──

    #[test]
    fn same_pattern_hits_same_cache_entry() {
        let cache = KgCache::new(100);
        let p1 = make_pattern_full("s", "p", "o", "g");
        let p2 = make_pattern_full("s", "p", "o", "g");

        assert_eq!(p1, p2);

        let results = vec![make_result("s", "p", "o", "g")];
        cache.put(p1, results.clone());

        let cached = cache.get(&p2).unwrap();
        assert_eq!(cached, results);
    }

    #[test]
    fn different_patterns_are_separate_entries() {
        let cache = KgCache::new(100);
        let p1 = make_pattern("alpha");
        let p2 = make_pattern("beta");

        assert_ne!(p1, p2);

        cache.put(p1.clone(), vec![make_result("alpha", "p", "o", "g")]);
        cache.put(p2.clone(), vec![make_result("beta", "p", "o", "g")]);

        let r1 = cache.get(&p1).unwrap();
        let r2 = cache.get(&p2).unwrap();
        assert_eq!(r1[0].subject, "alpha");
        assert_eq!(r2[0].subject, "beta");
    }

    #[test]
    fn wildcard_pattern_as_key() {
        let cache = KgCache::new(100);
        let p = QueryPattern {
            predicate: Some(StringDictionary::hash("knows")),
            ..QueryPattern::default()
        };
        let results =
            vec![make_result("s1", "knows", "o1", "g1"), make_result("s2", "knows", "o2", "g2")];

        cache.put(p.clone(), results.clone());
        let cached = cache.get(&p).unwrap();
        assert_eq!(cached.len(), 2);
    }

    // ── Len/is_empty tests ──

    #[test]
    fn new_cache_is_empty() {
        let cache = KgCache::new(50);
        assert_eq!(cache.len(), 0);
    }

    // ── Multiple result entries ──

    #[test]
    fn cache_stores_multiple_results() {
        let cache = KgCache::new(100);
        let p = make_pattern("multi");

        let results = vec![
            make_result("multi", "knows", "agent1", "g1"),
            make_result("multi", "knows", "agent2", "g2"),
            make_result("multi", "ownsModel", "model1", "g3"),
        ];

        cache.put(p.clone(), results.clone());
        let cached = cache.get(&p).unwrap();
        assert_eq!(cached.len(), 3);
        assert_eq!(cached[0].object, "agent1");
        assert_eq!(cached[1].object, "agent2");
        assert_eq!(cached[2].object, "model1");
    }

    #[test]
    fn pattern_with_all_bound_fields() {
        let cache = KgCache::new(100);
        let p = make_pattern_full("s", "p", "o", "g");
        let results = vec![make_result("s", "p", "o", "g")];

        cache.put(p.clone(), results.clone());
        assert!(cache.get(&p).is_some());

        let different = make_pattern_full("s", "p", "o", "other_g");
        assert!(cache.get(&different).is_none());
    }

    #[test]
    fn pattern_default_is_wildcard() {
        let p = QueryPattern::default();
        assert!(p.subject.is_none());
        assert!(p.predicate.is_none());
        assert!(p.object.is_none());
        assert!(p.graph.is_none());
    }

    #[test]
    fn invalidate_all_on_empty_is_noop() {
        let cache = KgCache::new(100);
        cache.invalidate_all();
        assert!(cache.get(&make_pattern("anything")).is_none());
    }

    #[test]
    fn put_after_invalidate_single() {
        let cache = KgCache::new(100);
        let p = make_pattern("reinsert");
        cache.put(p.clone(), vec![make_result("old", "p", "o", "g")]);

        cache.invalidate(&p);
        assert!(cache.get(&p).is_none());

        let new_results = vec![make_result("new", "p", "o", "g")];
        cache.put(p.clone(), new_results.clone());

        let cached = cache.get(&p).unwrap();
        assert_eq!(cached, new_results);
    }
}
