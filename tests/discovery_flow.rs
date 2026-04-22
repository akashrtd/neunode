//! Integration tests for the agent discovery flow.
//!
//! Verifies end-to-end: KG agent registration → candidate construction →
//! discovery search (capability matching, reputation/cost/online filtering,
//! max_results limiting, multi-cap matching) → complementarity analysis
//! → capability gap detection → individual scoring → ranked ordering.
//! Tests cross-crate interactions between neunode-discovery,
//! neunode-knowledge, and neunode-storage.

use std::sync::atomic::{AtomicU64, Ordering};

use neunode_discovery::{
    compute_score, find_capability_gaps, find_complementary, jaccard_distance, search,
    AgentCandidate, DiscoveryError, DiscoveryRequest, ScoringWeights,
};
use neunode_knowledge::{
    register_agent, register_bounty, MutationBatch, QueryEngine, StringDictionary,
};
use neunode_storage::db::NeunodeDb;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

static TEST_DB_ID: AtomicU64 = AtomicU64::new(0);

fn temp_db() -> NeunodeDb {
    let id = TEST_DB_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "neunode_discovery_flow_{:?}_{}",
        std::process::id(),
        id
    ));
    let _ = std::fs::remove_dir_all(&dir);
    NeunodeDb::open(&dir).expect("temp db should open")
}

/// Build an `AgentCandidate` with the given fields and sensible defaults.
fn make_candidate(
    did: &str,
    caps: &[&str],
    rep: f64,
    stake: u64,
    avail: f64,
    cost: f64,
    online: bool,
) -> AgentCandidate {
    AgentCandidate {
        did: did.to_string(),
        capabilities: caps.iter().map(|s| s.to_string()).collect(),
        reputation_score: rep,
        stake_amount: stake,
        availability_score: avail,
        latency_ms: 50,
        cost_per_unit: cost,
        is_online: online,
    }
}

/// Build a `DiscoveryRequest`.
fn make_request(
    req_caps: &[&str],
    min_rep: Option<f64>,
    max_cost: Option<f64>,
    online_only: bool,
    max_results: usize,
    requester_caps: &[&str],
) -> DiscoveryRequest {
    DiscoveryRequest {
        required_capabilities: req_caps.iter().map(|s| s.to_string()).collect(),
        min_reputation: min_rep,
        max_cost_per_unit: max_cost,
        must_be_online: online_only,
        max_results,
        requester_capabilities: requester_caps.iter().map(|s| s.to_string()).collect(),
    }
}

/// Register agents in the KG and return a candidate list for discovery.
fn setup_agents_and_candidates(
    db: &NeunodeDb,
    dict: &StringDictionary,
) -> Vec<AgentCandidate> {
    // Agent 1: inference specialist, high reputation, online
    let batch1 = register_agent(
        dict,
        "did:neunode:0xAlice",
        &["inference:llm", "training:lora"],
    )
    .expect("register agent 1");
    batch1.apply(db).expect("apply agent 1");

    // Agent 2: training specialist, medium reputation, online, cheap
    let batch2 =
        register_agent(dict, "did:neunode:0xBob", &["training:lora", "data:labeling"])
            .expect("register agent 2");
    batch2.apply(db).expect("apply agent 2");

    // Agent 3: offline, high reputation, expensive
    let batch3 = register_agent(
        dict,
        "did:neunode:0xCharlie",
        &["inference:llm", "training:pretrain"],
    )
    .expect("register agent 3");
    batch3.apply(db).expect("apply agent 3");

    // Agent 4: low reputation, online
    let batch4 = register_agent(dict, "did:neunode:0xDave", &["inference:llm"])
        .expect("register agent 4");
    batch4.apply(db).expect("apply agent 4");

    vec![
        make_candidate(
            "did:neunode:0xAlice",
            &["inference:llm", "training:lora"],
            4.5,
            2000,
            0.95,
            12.0,
            true,
        ),
        make_candidate(
            "did:neunode:0xBob",
            &["training:lora", "data:labeling"],
            3.0,
            800,
            0.85,
            5.0,
            true,
        ),
        make_candidate(
            "did:neunode:0xCharlie",
            &["inference:llm", "training:pretrain"],
            4.8,
            5000,
            0.70,
            25.0,
            false,
        ),
        make_candidate(
            "did:neunode:0xDave",
            &["inference:llm"],
            1.2,
            100,
            0.60,
            3.0,
            true,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Test 1: KG registration → candidate list → verify quads
// ---------------------------------------------------------------------------

#[test]
fn kg_registration_and_query() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let candidates = setup_agents_and_candidates(&db, &dict);

    // Should have 4 agents
    assert_eq!(candidates.len(), 4);

    // Verify that Alice's agent type quad exists in the KG
    let engine = QueryEngine::new(&db, &dict);
    let alice_hash = StringDictionary::hash("did:neunode:0xAlice");
    let pat = neunode_knowledge::QueryPattern {
        subject: Some(alice_hash),
        ..Default::default()
    };
    let results = engine.query(&pat).expect("query should succeed");
    // type(Agent) + hasCapability(inference:llm) + hasCapability(training:lora) = 3
    assert_eq!(results.len(), 3, "Alice should have 3 quads (1 type + 2 capabilities)");

    // Verify capability quads include expected predicates
    let predicates: Vec<&str> =
        results.iter().map(|r| r.predicate.as_str()).collect();
    assert!(
        predicates.iter().any(|p| p.contains("hasCapability")),
        "should have capability quads"
    );
}

// ---------------------------------------------------------------------------
// Test 2: Basic capability matching — only agents with matching cap returned
// ---------------------------------------------------------------------------

#[test]
fn basic_capability_matching() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let candidates = setup_agents_and_candidates(&db, &dict);

    // Request "inference:llm" — Alice, Charlie, Dave have it
    let request =
        make_request(&["inference:llm"], None, None, false, 10, &[]);
    let weights = ScoringWeights::default();
    let results = search(&candidates, &request, &weights).expect("search should succeed");

    assert_eq!(results.len(), 3, "3 agents have inference:llm");
    for r in &results {
        assert!(
            r.candidate.capabilities.contains(&"inference:llm".to_string()),
            "result should have inference:llm"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 3: min_reputation filtering
// ---------------------------------------------------------------------------

#[test]
fn min_reputation_filtering() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let candidates = setup_agents_and_candidates(&db, &dict);

    // Request inference:llm with min_reputation = 3.0
    // Alice (4.5) ✓, Charlie (4.8) ✓, Dave (1.2) ✗
    let request =
        make_request(&["inference:llm"], Some(3.0), None, false, 10, &[]);
    let weights = ScoringWeights::default();
    let results = search(&candidates, &request, &weights).expect("search should succeed");

    assert_eq!(results.len(), 2, "only Alice and Charlie pass min_reputation=3.0");
    for r in &results {
        assert!(
            r.candidate.reputation_score >= 3.0,
            "all results should have reputation >= 3.0"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 4: max_cost_per_unit filtering
// ---------------------------------------------------------------------------

#[test]
fn max_cost_filtering() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let candidates = setup_agents_and_candidates(&db, &dict);

    // Request inference:llm with max_cost = 15.0
    // Alice (12.0) ✓, Charlie (25.0) ✗, Dave (3.0) ✓
    let request =
        make_request(&["inference:llm"], None, Some(15.0), false, 10, &[]);
    let weights = ScoringWeights::default();
    let results = search(&candidates, &request, &weights).expect("search should succeed");

    assert_eq!(results.len(), 2, "Alice and Dave pass max_cost=15.0");
    for r in &results {
        assert!(
            r.candidate.cost_per_unit <= 15.0,
            "all results should have cost <= 15.0"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 5: must_be_online filtering
// ---------------------------------------------------------------------------

#[test]
fn must_be_online_filtering() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let candidates = setup_agents_and_candidates(&db, &dict);

    // Request inference:llm, online only
    // Alice (online) ✓, Charlie (offline) ✗, Dave (online) ✓
    let request =
        make_request(&["inference:llm"], None, None, true, 10, &[]);
    let weights = ScoringWeights::default();
    let results = search(&candidates, &request, &weights).expect("search should succeed");

    assert_eq!(results.len(), 2, "only online agents should be returned");
    for r in &results {
        assert!(r.candidate.is_online, "all results should be online");
    }
}

// ---------------------------------------------------------------------------
// Test 6: max_results limiting
// ---------------------------------------------------------------------------

#[test]
fn max_results_limiting() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let candidates = setup_agents_and_candidates(&db, &dict);

    // Request inference:llm with max_results = 1
    let request =
        make_request(&["inference:llm"], None, None, false, 1, &[]);
    let weights = ScoringWeights::default();
    let results = search(&candidates, &request, &weights).expect("search should succeed");

    assert_eq!(results.len(), 1, "should be capped to 1 result");
    // The top result should have the highest final_score
    assert!(results[0].final_score > 0.0);
}

// ---------------------------------------------------------------------------
// Test 7: Multi-capability matching — partial match ranks lower
// ---------------------------------------------------------------------------

#[test]
fn multi_capability_matching() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let candidates = setup_agents_and_candidates(&db, &dict);

    // Request both "inference:llm" and "training:lora"
    // Alice has both (2/2), Bob has only training:lora (1/2),
    // Charlie has only inference:llm (1/2), Dave has only inference:llm (1/2)
    let request = make_request(
        &["inference:llm", "training:lora"],
        None,
        None,
        false,
        10,
        &[],
    );
    let weights = ScoringWeights::default();
    let results = search(&candidates, &request, &weights).expect("search should succeed");

    // All 4 agents match at least one required capability
    assert_eq!(results.len(), 4, "all agents match at least one cap");
    // Alice should have the highest capability score (2/2 = 1.0)
    assert_eq!(results[0].candidate.did, "did:neunode:0xAlice");
    assert!(
        (results[0].capability_score - 1.0).abs() < 1e-10,
        "Alice should have capability_score = 1.0"
    );
}

// ---------------------------------------------------------------------------
// Test 8: find_complementary with Jaccard distance
// ---------------------------------------------------------------------------

#[test]
fn find_complementary_with_jaccard() {
    let requester_caps = vec!["inference:llm".to_string()];
    let candidates = vec![
        make_candidate("did:1", &["inference:llm"], 4.0, 1000, 0.9, 10.0, true),
        make_candidate("did:2", &["training:lora"], 3.0, 500, 0.8, 8.0, true),
        make_candidate("did:3", &["data:labeling", "training:pretrain"], 4.5, 2000, 0.95, 15.0, true),
    ];

    let results = find_complementary(&requester_caps, &candidates, 10);

    assert_eq!(results.len(), 3, "all 3 agents should be returned");

    // Agent with most different capabilities should rank highest
    // did:3 has completely different caps → Jaccard distance = 1.0
    // did:2 has completely different caps → Jaccard distance = 1.0
    // did:1 has identical cap → Jaccard distance = 0.0
    assert!(
        results[0].complementarity_score > 0.0,
        "top complementary agent should have positive distance"
    );
    // The last one should be the identical agent
    assert!(
        (results.last().unwrap().complementarity_score - 0.0).abs() < f64::EPSILON,
        "agent with identical caps should have complementarity_score = 0.0"
    );

    // Results should be sorted by final_score (complementarity) descending
    for window in results.windows(2) {
        assert!(
            window[0].final_score >= window[1].final_score,
            "results should be sorted by complementarity desc"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 9: Jaccard distance correctness
// ---------------------------------------------------------------------------

#[test]
fn jaccard_distance_correctness() {
    // Identical sets → distance 0
    let a = vec!["x".to_string(), "y".to_string()];
    let b = vec!["x".to_string(), "y".to_string()];
    assert!((jaccard_distance(&a, &b) - 0.0).abs() < f64::EPSILON);

    // Disjoint sets → distance 1
    let c = vec!["a".to_string()];
    let d = vec!["b".to_string()];
    assert!((jaccard_distance(&c, &d) - 1.0).abs() < f64::EPSILON);

    // Partial overlap: {a,b} ∩ {b,c} = {b}, union = {a,b,c}
    // distance = 1 - 1/3 = 2/3
    let e = vec!["a".to_string(), "b".to_string()];
    let f = vec!["b".to_string(), "c".to_string()];
    assert!((jaccard_distance(&e, &f) - 2.0 / 3.0).abs() < 1e-10);

    // Both empty → distance 0
    let empty: Vec<String> = vec![];
    assert!((jaccard_distance(&empty, &empty) - 0.0).abs() < f64::EPSILON);

    // One empty → distance 1
    assert!((jaccard_distance(&a, &empty) - 1.0).abs() < f64::EPSILON);
}

// ---------------------------------------------------------------------------
// Test 10: find_capability_gaps
// ---------------------------------------------------------------------------

#[test]
fn find_capability_gaps_detection() {
    let registered = vec![
        "inference:llm".to_string(),
        "training:lora".to_string(),
        "training:pretrain".to_string(),
        "data:labeling".to_string(),
    ];
    let agents = vec![
        ("did:neunode:0xAlice".to_string(), vec!["inference:llm".to_string(), "training:lora".to_string()]),
        ("did:neunode:0xBob".to_string(), vec!["training:lora".to_string()]),
    ];
    let bounties = vec![
        ("bounty:1".to_string(), vec!["training:pretrain".to_string()]),
        ("bounty:2".to_string(), vec!["training:pretrain".to_string()]),
        ("bounty:3".to_string(), vec!["data:labeling".to_string()]),
    ];

    let gaps = find_capability_gaps(&registered, &agents, &bounties);

    // "training:pretrain" has 0 providers, demand 2
    // "data:labeling" has 0 providers, demand 1
    assert_eq!(gaps.len(), 2, "2 capability gaps expected");
    assert_eq!(gaps[0].capability_uri, "training:pretrain");
    assert_eq!(gaps[0].demand_count, 2);
    assert_eq!(gaps[1].capability_uri, "data:labeling");
    assert_eq!(gaps[1].demand_count, 1);
}

// ---------------------------------------------------------------------------
// Test 11: find_capability_gaps with no gaps
// ---------------------------------------------------------------------------

#[test]
fn find_capability_gaps_no_gaps() {
    let registered = vec!["inference:llm".to_string()];
    let agents = vec![("did:1".to_string(), vec!["inference:llm".to_string()])];
    let bounties: Vec<(String, Vec<String>)> = vec![];

    let gaps = find_capability_gaps(&registered, &agents, &bounties);
    assert!(gaps.is_empty(), "no gaps when all capabilities are provided");
}

// ---------------------------------------------------------------------------
// Test 12: compute_score for individual agents
// ---------------------------------------------------------------------------

#[test]
fn compute_score_individual() {
    let candidates = vec![
        make_candidate("did:1", &["a", "b"], 4.0, 1000, 0.9, 10.0, true),
        make_candidate("did:2", &["c"], 2.0, 200, 0.5, 20.0, true),
    ];
    let request = make_request(&["a", "b"], None, None, false, 10, &["a"]);
    let weights = ScoringWeights::default();

    let scored = compute_score(&candidates[0], &request, &candidates, &weights);

    // Agent 1 matches 2/2 capabilities → capability_score = 1.0
    assert!(
        (scored.capability_score - 1.0).abs() < 1e-10,
        "agent with all required caps should score 1.0"
    );

    // Quality: 4.0 / 5.0 = 0.8
    assert!(
        (scored.quality_score - 0.8).abs() < 1e-10,
        "quality should be 4.0/5.0 = 0.8"
    );

    // Availability: online with 0.9 uptime
    assert!(
        (scored.availability_score - 0.9).abs() < 1e-10,
        "availability should be 0.9"
    );

    // Cost: 10.0 is the cheaper of [10.0, 20.0] → score = 1.0
    assert!(
        (scored.cost_score - 1.0).abs() < 1e-10,
        "cheapest agent should have cost_score 1.0"
    );

    // Complementarity: requester has ["a"], candidate has ["a","b"]
    // intersection = {a}, union = {a,b}, Jaccard = 1/2, distance = 0.5
    assert!(
        (scored.complementarity_score - 0.5).abs() < 1e-10,
        "complementarity should be Jaccard distance = 0.5"
    );

    // Final score should be weighted sum
    let expected_final = weights.capability_match * scored.capability_score
        + weights.quality * scored.quality_score
        + weights.availability * scored.availability_score
        + weights.cost_efficiency * scored.cost_score
        + weights.complementarity * scored.complementarity_score;
    assert!(
        (scored.final_score - expected_final).abs() < 1e-10,
        "final_score should equal weighted sum"
    );

    // All individual scores in [0, 1]
    assert!(scored.capability_score >= 0.0 && scored.capability_score <= 1.0);
    assert!(scored.quality_score >= 0.0 && scored.quality_score <= 1.0);
    assert!(scored.availability_score >= 0.0 && scored.availability_score <= 1.0);
    assert!(scored.cost_score >= 0.0 && scored.cost_score <= 1.0);
    assert!(scored.complementarity_score >= 0.0 && scored.complementarity_score <= 1.0);
}

// ---------------------------------------------------------------------------
// Test 13: ScoringWeights validation and defaults
// ---------------------------------------------------------------------------

#[test]
fn scoring_weights_defaults_valid() {
    let w = ScoringWeights::default();
    assert!(w.validate(), "default weights must be valid");
    assert!((w.sum() - 1.0).abs() < 1e-6, "default weights must sum to 1.0");
    assert_eq!(w.capability_match, 0.40);
    assert_eq!(w.quality, 0.25);
    assert_eq!(w.availability, 0.15);
    assert_eq!(w.cost_efficiency, 0.10);
    assert_eq!(w.complementarity, 0.10);
}

#[test]
fn scoring_weights_custom_valid() {
    let w = ScoringWeights {
        capability_match: 0.50,
        quality: 0.20,
        availability: 0.15,
        cost_efficiency: 0.10,
        complementarity: 0.05,
    };
    assert!(w.validate(), "custom weights summing to 1.0 should be valid");
}

#[test]
fn scoring_weights_invalid_sum() {
    let w = ScoringWeights {
        capability_match: 0.50,
        quality: 0.50,
        availability: 0.10,
        cost_efficiency: 0.10,
        complementarity: 0.10,
    };
    assert!(!w.validate(), "weights summing > 1.0 should not validate");
}

#[test]
fn scoring_weights_negative_rejected() {
    let w = ScoringWeights {
        capability_match: -0.10,
        quality: 0.40,
        availability: 0.30,
        cost_efficiency: 0.20,
        complementarity: 0.20,
    };
    assert!(!w.validate(), "negative weight should not validate");
}

// ---------------------------------------------------------------------------
// Test 14: Ranked ordering — results sorted by final_score descending
// ---------------------------------------------------------------------------

#[test]
fn ranked_ordering_by_final_score() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let candidates = setup_agents_and_candidates(&db, &dict);

    let request =
        make_request(&["inference:llm"], None, None, false, 10, &[]);
    let weights = ScoringWeights::default();
    let results = search(&candidates, &request, &weights).expect("search should succeed");

    assert!(results.len() >= 2, "need at least 2 results to verify ordering");

    // Verify descending order
    for window in results.windows(2) {
        assert!(
            window[0].final_score >= window[1].final_score,
            "results should be sorted by final_score descending, \
             got {} > {}",
            window[0].candidate.did,
            window[1].candidate.did
        );
    }
}

// ---------------------------------------------------------------------------
// Test 15: Error case — EmptyPool
// ---------------------------------------------------------------------------

#[test]
fn error_empty_pool() {
    let candidates: Vec<AgentCandidate> = vec![];
    let request = make_request(&["inference:llm"], None, None, false, 10, &[]);
    let weights = ScoringWeights::default();

    let result = search(&candidates, &request, &weights);
    assert!(result.is_err(), "empty pool should return error");
    match result.unwrap_err() {
        DiscoveryError::EmptyPool => {}
        other => panic!("expected EmptyPool, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test 16: Error case — NoMatches
// ---------------------------------------------------------------------------

#[test]
fn error_no_matches() {
    let candidates = vec![
        make_candidate("did:1", &["training:lora"], 4.0, 1000, 0.9, 10.0, true),
    ];
    let request =
        make_request(&["inference:llm"], None, None, false, 10, &[]);
    let weights = ScoringWeights::default();

    let result = search(&candidates, &request, &weights);
    assert!(result.is_err(), "no matching agents should return error");
    match result.unwrap_err() {
        DiscoveryError::NoMatches => {}
        other => panic!("expected NoMatches, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test 17: Error case — NoMatches due to strict filters
// ---------------------------------------------------------------------------

#[test]
fn error_no_matches_due_to_filters() {
    let candidates = vec![
        make_candidate("did:1", &["inference:llm"], 2.0, 100, 0.5, 50.0, false),
    ];
    // All filters exclude the only candidate
    let request = make_request(
        &["inference:llm"],
        Some(4.0),  // candidate has 2.0
        Some(10.0), // candidate costs 50.0
        true,       // candidate is offline
        10,
        &[],
    );
    let weights = ScoringWeights::default();

    let result = search(&candidates, &request, &weights);
    assert!(result.is_err());
    match result.unwrap_err() {
        DiscoveryError::NoMatches => {}
        other => panic!("expected NoMatches, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test 18: Combined filtering — reputation + cost + online
// ---------------------------------------------------------------------------

#[test]
fn combined_filtering() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);
    let candidates = setup_agents_and_candidates(&db, &dict);

    // Request inference:llm, min_rep=3.0, max_cost=15.0, online only
    // Alice: rep=4.5 ✓, cost=12.0 ✓, online ✓ → matches
    // Charlie: rep=4.8 ✓, cost=25.0 ✗ → filtered out
    // Dave: rep=1.2 ✗ → filtered out
    let request = make_request(
        &["inference:llm"],
        Some(3.0),
        Some(15.0),
        true,
        10,
        &[],
    );
    let weights = ScoringWeights::default();
    let results = search(&candidates, &request, &weights).expect("search should succeed");

    assert_eq!(results.len(), 1, "only Alice should pass all filters");
    assert_eq!(results[0].candidate.did, "did:neunode:0xAlice");
}

// ---------------------------------------------------------------------------
// Test 19: Bounty registration in KG + capability gap cross-check
// ---------------------------------------------------------------------------

#[test]
fn bounty_registration_and_gap_cross_check() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);

    // Register agents with limited capabilities
    let batch1 =
        register_agent(&dict, "did:neunode:0xFelix", &["inference:llm"]).expect("register felix");
    batch1.apply(&db).expect("apply felix");

    // Register a bounty requiring a capability no agent has
    let batch2 = register_bounty(&dict, "bounty:rlhf-job", &["training:rlhf", "data:labeling"])
        .expect("register bounty");
    batch2.apply(&db).expect("apply bounty");

    // Verify bounty quads exist in KG
    let engine = QueryEngine::new(&db, &dict);
    let bounty_hash = StringDictionary::hash("bounty:rlhf-job");
    let pat = neunode_knowledge::QueryPattern {
        subject: Some(bounty_hash),
        ..Default::default()
    };
    let results = engine.query(&pat).expect("query should succeed");
    // type(Bounty) + requiresCapability(training:rlhf) + requiresCapability(data:labeling) = 3
    assert_eq!(results.len(), 3, "bounty should have 3 quads");

    // Now find capability gaps
    let registered = vec![
        "inference:llm".to_string(),
        "training:rlhf".to_string(),
        "data:labeling".to_string(),
    ];
    let agents = vec![(
        "did:neunode:0xFelix".to_string(),
        vec!["inference:llm".to_string()],
    )];
    let bounties = vec![(
        "bounty:rlhf-job".to_string(),
        vec!["training:rlhf".to_string(), "data:labeling".to_string()],
    )];

    let gaps = find_capability_gaps(&registered, &agents, &bounties);
    assert_eq!(gaps.len(), 2, "2 gaps: training:rlhf and data:labeling");
    assert!(gaps.iter().any(|g| g.capability_uri == "training:rlhf"));
    assert!(gaps.iter().any(|g| g.capability_uri == "data:labeling"));
}

// ---------------------------------------------------------------------------
// Test 20: MutationBatch usage within discovery context
// ---------------------------------------------------------------------------

#[test]
fn mutation_batch_for_discovery_setup() {
    let db = temp_db();
    let dict = StringDictionary::new(&db);

    let batch = MutationBatch::new();
    assert!(batch.is_empty());

    // Register 3 agents in one batch
    let b1 = register_agent(&dict, "did:neunode:0xA", &["cap:x"]).expect("register A");
    let b2 = register_agent(&dict, "did:neunode:0xB", &["cap:y"]).expect("register B");
    let b3 = register_agent(&dict, "did:neunode:0xC", &["cap:z"]).expect("register C");

    // Each register_agent returns a batch — apply them all
    b1.apply(&db).expect("apply b1");
    b2.apply(&db).expect("apply b2");
    b3.apply(&db).expect("apply b3");

    // Build candidates from the registered agents
    let candidates = vec![
        make_candidate("did:neunode:0xA", &["cap:x"], 4.0, 1000, 0.9, 10.0, true),
        make_candidate("did:neunode:0xB", &["cap:y"], 3.5, 800, 0.8, 8.0, true),
        make_candidate("did:neunode:0xC", &["cap:z"], 2.0, 300, 0.6, 15.0, false),
    ];

    let request = make_request(&["cap:x", "cap:y"], None, None, false, 10, &[]);
    let weights = ScoringWeights::default();
    let results = search(&candidates, &request, &weights).expect("search should succeed");

    // A matches cap:x (1/2), B matches cap:y (1/2), C matches none
    assert_eq!(results.len(), 2, "A and B match at least one required cap");
}

// ---------------------------------------------------------------------------
// Test 21: Offline agent penalized in availability score
// ---------------------------------------------------------------------------

#[test]
fn offline_agent_penalized() {
    let candidates = vec![
        make_candidate("did:online", &["a"], 4.0, 1000, 0.9, 10.0, true),
        make_candidate("did:offline", &["a"], 4.0, 1000, 0.9, 10.0, false),
    ];
    let request = make_request(&["a"], None, None, false, 10, &[]);
    let weights = ScoringWeights::default();

    let scored_online = compute_score(&candidates[0], &request, &candidates, &weights);
    let scored_offline = compute_score(&candidates[1], &request, &candidates, &weights);

    assert_eq!(scored_online.availability_score, 0.9, "online agent gets uptime score");
    assert!(
        (scored_offline.availability_score - 0.0).abs() < f64::EPSILON,
        "offline agent gets availability score 0"
    );
    assert!(
        scored_online.final_score > scored_offline.final_score,
        "online agent should score higher overall"
    );
}

// ---------------------------------------------------------------------------
// Test 22: No required capabilities returns all candidates
// ---------------------------------------------------------------------------

#[test]
fn no_required_capabilities_returns_all() {
    let candidates = vec![
        make_candidate("did:1", &["x"], 4.0, 1000, 0.9, 10.0, true),
        make_candidate("did:2", &["y"], 3.0, 500, 0.7, 15.0, true),
    ];
    let request = make_request(&[], None, None, false, 10, &[]);
    let weights = ScoringWeights::default();
    let results = search(&candidates, &request, &weights).expect("search should succeed");
    assert_eq!(results.len(), 2, "no required caps should return all candidates");
}
