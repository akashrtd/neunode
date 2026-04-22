//! Integration tests for the Knowledge Graph flow.
//!
//! Verifies end-to-end cross-crate interactions between neunode-knowledge
//! and neunode-storage: dictionary interning, quad indexing, mutation batches,
//! query engine prefix scans, ontology helpers, and domain registration
//! functions (agents, models, bounties, training jobs).

use std::sync::atomic::{AtomicU64, Ordering};

use neunode_knowledge::*;
use neunode_storage::db::NeunodeDb;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

static TEST_DB_ID: AtomicU64 = AtomicU64::new(0);

fn temp_db() -> NeunodeDb {
    let id = TEST_DB_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "neunode_knowledge_flow_{:?}_{}",
        std::process::id(),
        id
    ));
    let _ = std::fs::remove_dir_all(&dir);
    NeunodeDb::open(&dir).expect("temp db should open")
}

// ---------------------------------------------------------------------------
// Test 1: StringDictionary — insert, lookup, contains roundtrip
// ---------------------------------------------------------------------------

#[test]
fn dictionary_insert_lookup_roundtrip() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);

    let uri = "did:neunode:0xAlice";
    let h = dict.insert(uri).expect("insert should succeed");
    assert_eq!(h, StringDictionary::hash(uri), "hash should match deterministic computation");

    let resolved = dict.lookup(&h).expect("lookup should succeed");
    assert_eq!(resolved, uri, "lookup should return original string");
}

// ---------------------------------------------------------------------------
// Test 2: StringDictionary — batch_insert + individual lookups
// ---------------------------------------------------------------------------

#[test]
fn dictionary_batch_insert_and_lookup() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);

    let strings = [
        "did:neunode:0xAgent1",
        "https://neunode.io/ontology/Agent",
        "https://neunode.io/ontology/hasCapability",
        "https://neunode.io/ontology/NLP",
        "https://neunode.io/ontology/neunode",
    ];

    let hashes = dict.batch_insert(&strings).expect("batch_insert should succeed");
    assert_eq!(hashes.len(), 5, "should return 5 hashes");

    for (i, hash) in hashes.iter().enumerate() {
        assert!(dict.contains(hash).expect("contains should succeed"));
        let resolved = dict.lookup(hash).expect("lookup should succeed");
        assert_eq!(resolved, strings[i], "lookup should return original string");
    }

    // All hashes should be unique (no collisions)
    let mut seen = std::collections::HashSet::new();
    for h in &hashes {
        assert!(seen.insert(*h), "hash collision detected in batch");
    }
}

// ---------------------------------------------------------------------------
// Test 3: StringDictionary — contains returns false for missing hashes
// ---------------------------------------------------------------------------

#[test]
fn dictionary_contains_missing() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);

    let fake_hash = [0xFF_u8; 16];
    assert!(!dict.contains(&fake_hash).expect("contains should succeed for missing key"));
}

// ---------------------------------------------------------------------------
// Test 4: Quad — from_strings + insert_indexes + delete_indexes lifecycle
// ---------------------------------------------------------------------------

#[test]
fn quad_insert_delete_lifecycle() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);

    let quad = triple::Quad::from_strings(
        &dict,
        "did:neunode:0xAlice",
        "https://neunode.io/ontology/knows",
        "did:neunode:0xBob",
        "https://neunode.io/ontology/neunode",
    )
    .expect("quad from_strings should succeed");

    // Insert into all 6 indexes.
    quad.insert_indexes(&db).expect("insert_indexes should succeed");

    // Verify existence in SPOG.
    assert!(
        quad.exists_in(&db, neunode_storage::cf::CF_KG_SPOG).expect("exists_in should succeed"),
        "quad should exist after insert"
    );

    // Delete from all 6 indexes.
    quad.delete_indexes(&db).expect("delete_indexes should succeed");

    assert!(
        !quad.exists_in(&db, neunode_storage::cf::CF_KG_SPOG).expect("exists_in should succeed"),
        "quad should NOT exist after delete"
    );
}

// ---------------------------------------------------------------------------
// Test 5: Quad — prefix scan returns inserted quads
// ---------------------------------------------------------------------------

#[test]
fn quad_prefix_scan_after_insert() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);

    let shared_subject = "did:neunode:0xAlice";
    let q1 = triple::Quad::from_strings(
        &dict,
        shared_subject,
        "https://neunode.io/ontology/knows",
        "did:neunode:0xBob",
        "https://neunode.io/ontology/neunode",
    )
    .expect("q1 from_strings");

    let q2 = triple::Quad::from_strings(
        &dict,
        shared_subject,
        "https://neunode.io/ontology/ownsModel",
        "ipfs://QmModelHash",
        "https://neunode.io/ontology/neunode",
    )
    .expect("q2 from_strings");

    q1.insert_indexes(&db).expect("insert q1");
    q2.insert_indexes(&db).expect("insert q2");

    let prefix = triple::Quad::prefix_for(
        neunode_storage::cf::CF_KG_SPOG,
        &StringDictionary::hash(shared_subject),
    );
    let results = db
        .prefix_scan(neunode_storage::cf::CF_KG_SPOG, &prefix)
        .expect("prefix_scan should succeed");
    assert_eq!(results.len(), 2, "should find 2 quads with same subject");
}

// ---------------------------------------------------------------------------
// Test 6: Ontology helpers — nn(), all_classes(), all_predicates()
// ---------------------------------------------------------------------------

#[test]
fn ontology_namespace_and_constants() {
    // nn() produces full URIs.
    assert_eq!(nn("Agent"), "https://neunode.io/ontology/Agent");
    assert_eq!(nn("hasCapability"), "https://neunode.io/ontology/hasCapability");

    // all_classes() returns 6 unique classes.
    let classes = all_classes();
    assert_eq!(classes.len(), 6);
    assert!(classes.contains(&CLASS_AGENT));
    assert!(classes.contains(&CLASS_MODEL));
    assert!(classes.contains(&CLASS_BOUNTY));
    assert!(classes.contains(&CLASS_TRAINING_JOB));
    assert!(classes.contains(&CLASS_CAPABILITY));
    assert!(classes.contains(&CLASS_KNOWLEDGE));

    // all_predicates() returns 11 unique predicates.
    let preds = all_predicates();
    assert_eq!(preds.len(), 11);
    assert!(preds.contains(&PRED_TYPE));
    assert!(preds.contains(&PRED_HAS_CAPABILITY));
    assert!(preds.contains(&PRED_OWNS_MODEL));
    assert!(preds.contains(&PRED_PARTICIPATES_IN));
}

// ---------------------------------------------------------------------------
// Test 7: Ontology helpers — type_quad and relation_quad
// ---------------------------------------------------------------------------

#[test]
fn ontology_type_and_relation_quads() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);

    // type_quad: (entity, rdf:type, class, graph)
    let tq = type_quad(&dict, "did:neunode:0xAlice", CLASS_AGENT).expect("type_quad");
    assert_eq!(tq.subject, StringDictionary::hash("did:neunode:0xAlice"));
    assert_eq!(tq.predicate, StringDictionary::hash(&nn(PRED_TYPE)));
    assert_eq!(tq.object, StringDictionary::hash(&nn(CLASS_AGENT)));
    assert_eq!(tq.graph, StringDictionary::hash(&nn(DEFAULT_GRAPH)));

    // relation_quad: (subject, predicate, object, graph)
    let rq = relation_quad(
        &dict,
        "did:neunode:0xAlice",
        PRED_OWNS_MODEL,
        "ipfs://QmModel",
    )
    .expect("relation_quad");
    assert_eq!(rq.subject, StringDictionary::hash("did:neunode:0xAlice"));
    assert_eq!(rq.predicate, StringDictionary::hash(&nn(PRED_OWNS_MODEL)));
    assert_eq!(rq.object, StringDictionary::hash("ipfs://QmModel"));
    assert_eq!(rq.graph, StringDictionary::hash(&nn(DEFAULT_GRAPH)));
}

// ---------------------------------------------------------------------------
// Test 8: MutationBatch — insert + delete + apply
// ---------------------------------------------------------------------------

#[test]
fn mutation_batch_insert_delete_apply() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);

    let q_insert = type_quad(&dict, "did:neunode:0xNew", CLASS_MODEL).expect("type_quad new");
    let q_delete = type_quad(&dict, "did:neunode:0xOld", CLASS_AGENT).expect("type_quad old");

    // Pre-insert the "old" quad so we can delete it.
    q_delete.insert_indexes(&db).expect("pre-insert old");
    assert!(q_delete.exists_in(&db, neunode_storage::cf::CF_KG_SPOG).expect("exists"));

    let mut batch = MutationBatch::new();
    batch.insert(q_insert.clone());
    batch.delete(q_delete.clone());
    assert_eq!(batch.len(), 2);

    batch.apply(&db).expect("apply should succeed");

    // Inserted quad should exist.
    assert!(q_insert.exists_in(&db, neunode_storage::cf::CF_KG_SPOG).expect("exists new"));
    // Deleted quad should be gone.
    assert!(!q_delete.exists_in(&db, neunode_storage::cf::CF_KG_SPOG).expect("not exists old"));
}

// ---------------------------------------------------------------------------
// Test 9: register_agent — creates type + capability quads
// ---------------------------------------------------------------------------

#[test]
fn register_agent_creates_type_and_capabilities() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);

    let batch = register_agent(&dict, "did:neunode:0xAlice", &["NLP", "Training"])
        .expect("register_agent should succeed");

    // 1 type quad + 2 capability quads = 3.
    assert_eq!(batch.len(), 3);

    batch.apply(&db).expect("apply should succeed");

    // Query by subject to verify all 3 quads indexed.
    let engine = QueryEngine::new(&db, &dict);
    let pattern = QueryPattern {
        subject: Some(StringDictionary::hash("did:neunode:0xAlice")),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&pattern).expect("count"), 3);
}

// ---------------------------------------------------------------------------
// Test 10: register_model — with and without lineage
// ---------------------------------------------------------------------------

#[test]
fn register_model_with_and_without_lineage() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);

    // Model with parent (lineage): type + ownsModel + dependsOn = 3.
    let with_parent = register_model(
        &dict,
        "did:neunode:0xDev",
        "ipfs://QmFineTuned",
        Some("ipfs://QmBaseModel"),
    )
    .expect("register_model with parent");
    assert_eq!(with_parent.len(), 3);

    // Model without parent: type + ownsModel = 2.
    let without_parent =
        register_model(&dict, "did:neunode:0xDev", "ipfs://QmStandalone", None)
            .expect("register_model without parent");
    assert_eq!(without_parent.len(), 2);
}

// ---------------------------------------------------------------------------
// Test 11: register_model — apply and verify lineage quads
// ---------------------------------------------------------------------------

#[test]
fn register_model_apply_and_query_lineage() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let engine = QueryEngine::new(&db, &dict);

    let batch = register_model(
        &dict,
        "did:neunode:0xDev",
        "ipfs://QmChild",
        Some("ipfs://QmParent"),
    )
    .expect("register_model");
    batch.apply(&db).expect("apply");

    // Model subject: type(Model) + dependsOn(parent) = 2.
    let model_pattern = QueryPattern {
        subject: Some(StringDictionary::hash("ipfs://QmChild")),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&model_pattern).expect("count model"), 2);

    // Owner subject: ownsModel = 1.
    let owner_pattern = QueryPattern {
        subject: Some(StringDictionary::hash("did:neunode:0xDev")),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&owner_pattern).expect("count owner"), 1);
}

// ---------------------------------------------------------------------------
// Test 12: register_bounty — creates type + required capability quads
// ---------------------------------------------------------------------------

#[test]
fn register_bounty_creates_type_and_requirements() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);

    let batch =
        register_bounty(&dict, "bounty:sentiment-analysis", &["NLP", "RLHF", "DataLabeling"])
            .expect("register_bounty");

    // 1 type + 3 required capabilities = 4.
    assert_eq!(batch.len(), 4);

    batch.apply(&db).expect("apply");

    let engine = QueryEngine::new(&db, &dict);
    let pattern = QueryPattern {
        subject: Some(StringDictionary::hash("bounty:sentiment-analysis")),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&pattern).expect("count bounty"), 4);
}

// ---------------------------------------------------------------------------
// Test 13: join_training_job — creates participatesIn quad
// ---------------------------------------------------------------------------

#[test]
fn join_training_job_creates_participation() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);

    let batch = join_training_job(&dict, "did:neunode:0xWorker", "job:distributed-001")
        .expect("join_training_job");
    assert_eq!(batch.len(), 1);

    batch.apply(&db).expect("apply");

    let engine = QueryEngine::new(&db, &dict);
    let pattern = QueryPattern {
        subject: Some(StringDictionary::hash("did:neunode:0xWorker")),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&pattern).expect("count"), 1);

    // Verify the quad resolves to correct strings.
    let results = engine.query(&pattern).expect("query");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].subject, "did:neunode:0xWorker");
    assert_eq!(results[0].predicate, nn(PRED_PARTICIPATES_IN));
    assert_eq!(results[0].object, "job:distributed-001");
    assert_eq!(results[0].graph, nn(DEFAULT_GRAPH));
}

// ---------------------------------------------------------------------------
// Test 14: QueryEngine — query by subject, predicate, object
// ---------------------------------------------------------------------------

#[test]
fn query_engine_by_subject_predicate_object() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let engine = QueryEngine::new(&db, &dict);

    // Insert quads via register_agent.
    register_agent(&dict, "did:neunode:0xAlice", &["NLP", "Vision"])
        .expect("register_agent")
        .apply(&db)
        .expect("apply");

    register_agent(&dict, "did:neunode:0xBob", &["NLP"])
        .expect("register_agent")
        .apply(&db)
        .expect("apply");

    // Query by subject (Alice) → 3 quads (type + 2 caps).
    let by_subject = QueryPattern {
        subject: Some(StringDictionary::hash("did:neunode:0xAlice")),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&by_subject).expect("count"), 3);

    // Query by predicate (hasCapability) → 3 quads (Alice×2 + Bob×1).
    let by_predicate = QueryPattern {
        predicate: Some(StringDictionary::hash(&nn(PRED_HAS_CAPABILITY))),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&by_predicate).expect("count"), 3);

    // Query by object (NLP) → 2 quads (Alice + Bob).
    let by_object = QueryPattern {
        object: Some(StringDictionary::hash(&nn("NLP"))),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&by_object).expect("count"), 2);
}

// ---------------------------------------------------------------------------
// Test 15: QueryEngine — query returns correctly resolved strings
// ---------------------------------------------------------------------------

#[test]
fn query_engine_resolves_strings() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let engine = QueryEngine::new(&db, &dict);

    let agent_did = "did:neunode:0xAlice";
    register_agent(&dict, agent_did, &["NLP"])
        .expect("register_agent")
        .apply(&db)
        .expect("apply");

    let pattern = QueryPattern {
        subject: Some(StringDictionary::hash(agent_did)),
        predicate: Some(StringDictionary::hash(&nn(PRED_HAS_CAPABILITY))),
        ..QueryPattern::default()
    };
    let results = engine.query(&pattern).expect("query");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].subject, agent_did);
    assert_eq!(results[0].predicate, nn(PRED_HAS_CAPABILITY));
    assert_eq!(results[0].object, nn("NLP"));
    assert_eq!(results[0].graph, nn(DEFAULT_GRAPH));
}

// ---------------------------------------------------------------------------
// Test 16: QueryEngine — empty result for nonexistent subject
// ---------------------------------------------------------------------------

#[test]
fn query_engine_empty_result() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let engine = QueryEngine::new(&db, &dict);

    let pattern = QueryPattern {
        subject: Some(StringDictionary::hash("did:neunode:0xNobody")),
        ..QueryPattern::default()
    };
    let results = engine.query(&pattern).expect("query");
    assert!(results.is_empty());
    assert_eq!(engine.count(&pattern).expect("count"), 0);
}

// ---------------------------------------------------------------------------
// Test 17: Full end-to-end workflow — agents, models, bounties, training
// ---------------------------------------------------------------------------

#[test]
fn full_knowledge_graph_workflow() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let engine = QueryEngine::new(&db, &dict);

    // Step 1: Register 2 agents with different capabilities.
    register_agent(&dict, "did:neunode:0xAlice", &["NLP", "Training"])
        .expect("register alice")
        .apply(&db)
        .expect("apply alice");

    register_agent(&dict, "did:neunode:0xBob", &["Vision", "RLHF"])
        .expect("register bob")
        .apply(&db)
        .expect("apply bob");

    // Step 2: Register a model with lineage (fine-tuned from base).
    register_model(
        &dict,
        "did:neunode:0xAlice",
        "ipfs://QmFineTuned",
        Some("ipfs://QmBaseModel"),
    )
    .expect("register model")
    .apply(&db)
    .expect("apply model");

    // Step 3: Register a bounty requiring NLP capability.
    register_bounty(&dict, "bounty:sentiment", &["NLP", "RLHF"])
        .expect("register bounty")
        .apply(&db)
        .expect("apply bounty");

    // Step 4: Both agents join a training job.
    join_training_job(&dict, "did:neunode:0xAlice", "job:train-001")
        .expect("join alice")
        .apply(&db)
        .expect("apply alice join");

    join_training_job(&dict, "did:neunode:0xBob", "job:train-001")
        .expect("join bob")
        .apply(&db)
        .expect("apply bob join");

    // Verify: Alice should have type + 2 caps + ownsModel + participatesIn = 5.
    let alice_pattern = QueryPattern {
        subject: Some(StringDictionary::hash("did:neunode:0xAlice")),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&alice_pattern).expect("count alice"), 5);

    // Verify: Bob should have type + 2 caps + participatesIn = 4.
    let bob_pattern = QueryPattern {
        subject: Some(StringDictionary::hash("did:neunode:0xBob")),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&bob_pattern).expect("count bob"), 4);

    // Verify: Model subject has type + dependsOn = 2.
    let model_pattern = QueryPattern {
        subject: Some(StringDictionary::hash("ipfs://QmFineTuned")),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&model_pattern).expect("count model"), 2);

    // Verify: Bounty subject has type + 2 requiresCapability = 3.
    let bounty_pattern = QueryPattern {
        subject: Some(StringDictionary::hash("bounty:sentiment")),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&bounty_pattern).expect("count bounty"), 3);

    // Verify: Query by predicate (participatesIn) → 2 results.
    let participation_pattern = QueryPattern {
        predicate: Some(StringDictionary::hash(&nn(PRED_PARTICIPATES_IN))),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&participation_pattern).expect("count participation"), 2);

    // Verify: Query by object (the training job) → 2 agents participate.
    let job_pattern = QueryPattern {
        object: Some(StringDictionary::hash("job:train-001")),
        ..QueryPattern::default()
    };
    assert_eq!(engine.count(&job_pattern).expect("count job"), 2);

    // Verify: Dictionary contains all interned strings.
    assert!(dict.contains(&StringDictionary::hash("did:neunode:0xAlice")).expect("contains"));
    assert!(dict.contains(&StringDictionary::hash("ipfs://QmFineTuned")).expect("contains"));
    assert!(dict.contains(&StringDictionary::hash("job:train-001")).expect("contains"));
    assert!(dict.contains(&StringDictionary::hash(&nn(PRED_TYPE))).expect("contains"));
    assert!(dict.contains(&StringDictionary::hash(&nn(CLASS_AGENT))).expect("contains"));
}
