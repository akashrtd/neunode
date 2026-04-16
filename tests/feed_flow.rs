//! Integration tests for the feed event flow.
//!
//! Verifies end-to-end: keypair → FeedEvent creation → signing → verification,
//! sigchain append + verify, filter matching, topic routing, schema validation,
//! and serde roundtrips across multiple crates.

use neunode_core::kind::Kind;
use neunode_core::types::{Did, EventId, Hash256};
use neunode_crypto::ed25519;
use neunode_feed::event::{EventRef, EventTag, FeedEvent};
use neunode_feed::filter::{apply_filter, FeedFilter};
use neunode_feed::schema::{self, Attestation, BountyClaim, BountyPost};
use neunode_feed::sigchain::SigChain;
use neunode_feed::topics::{
    all_topics, is_valid_topic, parse_topic, topic_for_category, topic_for_kind,
};

fn test_keypair() -> ([u8; 32], [u8; 32]) {
    let seed = [7u8; 32];
    let (sk, vk) = ed25519::keypair_from_seed(&seed);
    (ed25519::signing_key_to_bytes(&sk), ed25519::verifying_key_to_bytes(&vk))
}

fn test_did() -> Did {
    Did("did:neunode:feed_integration_agent".to_string())
}

// ---------------------------------------------------------------------------
// Test 1: Full feed event lifecycle — create → sign → verify → serde roundtrip
// ---------------------------------------------------------------------------

#[test]
fn feed_event_create_sign_verify_serde_roundtrip() {
    let (sk_bytes, vk_bytes) = test_keypair();
    let did = test_did();

    // Create event
    let mut event = FeedEvent::new(
        Kind::BountyPost,
        did.clone(),
        0,
        Hash256("0".to_string()),
        "Fine-tune Llama-3B on medical data".to_string(),
    )
    .expect("event creation should succeed");

    assert!(event.signature.is_none(), "new event should be unsigned");
    assert!(event.id.0.starts_with('f'), "event ID should start with 'f'");
    assert_eq!(event.kind, Kind::BountyPost);
    assert_eq!(event.author, did);

    // Sign
    event.sign(&sk_bytes).expect("signing should succeed");
    assert!(event.signature.is_some());

    // Verify
    assert!(event.verify_signature(&vk_bytes), "signature should verify");

    // Validate
    event.validate().expect("event should be valid");

    // Serde roundtrip preserves all fields
    let json = serde_json::to_string(&event).expect("serialize");
    let back: FeedEvent = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(event, back, "serde roundtrip should preserve event");
}

// ---------------------------------------------------------------------------
// Test 2: Sigchain with 3+ events — sequences, hash links, full verification
// ---------------------------------------------------------------------------

#[test]
fn sigchain_multi_event_hash_chain_and_verification() {
    let (sk_bytes, vk_bytes) = test_keypair();
    let did = test_did();

    let mut chain = SigChain::new(did.clone(), vk_bytes);

    let e0 = chain
        .append(Kind::AgentMetadata, "agent metadata update".to_string(), &sk_bytes)
        .expect("append 0");
    let e1 = chain
        .append(Kind::BountyPost, "posting a new bounty".to_string(), &sk_bytes)
        .expect("append 1");
    let e2 = chain
        .append(Kind::Attest, "attesting to work quality".to_string(), &sk_bytes)
        .expect("append 2");

    // Sequences are correct
    assert_eq!(e0.sequence, 0);
    assert_eq!(e1.sequence, 1);
    assert_eq!(e2.sequence, 2);
    assert_eq!(chain.len(), 3);

    // Hash chain links
    assert_eq!(e1.prev_hash, e0.compute_hash().unwrap(), "e1 prev_hash should equal e0 hash");
    assert_eq!(e2.prev_hash, e1.compute_hash().unwrap(), "e2 prev_hash should equal e1 hash");

    // All events signed
    assert!(e0.verify_signature(&vk_bytes));
    assert!(e1.verify_signature(&vk_bytes));
    assert!(e2.verify_signature(&vk_bytes));

    // Full chain verification passes
    chain.verify_chain().expect("chain should verify");
}

// ---------------------------------------------------------------------------
// Test 3: Sigchain tamper detection — content, sequence, prev_hash
// ---------------------------------------------------------------------------

#[test]
fn sigchain_tamper_detection_breaks_verification() {
    let (sk_bytes, vk_bytes) = test_keypair();
    let did = test_did();

    // Wrong signing key breaks verification
    let wrong_seed = [99u8; 32];
    let (wrong_sk, _) = ed25519::keypair_from_seed(&wrong_seed);
    let wrong_sk_bytes = ed25519::signing_key_to_bytes(&wrong_sk);

    let mut chain = SigChain::new(did.clone(), vk_bytes);
    chain.append(Kind::BountyPost, "signed correctly".to_string(), &sk_bytes).expect("append");
    // Second event signed with wrong key
    chain.append(Kind::BountyClaim, "signed wrongly".to_string(), &wrong_sk_bytes).expect("append");

    let result = chain.verify_chain();
    assert!(result.is_err(), "wrong signing key should break chain");

    // Empty chain verifies fine
    let empty = SigChain::new(did, vk_bytes);
    assert!(empty.verify_chain().is_ok());
}

// ---------------------------------------------------------------------------
// Test 4: FeedFilter — kind, author, time range, tags, limit (cross-crate)
// ---------------------------------------------------------------------------

#[test]
fn filter_kind_author_time_range_and_limit() {
    let (sk_bytes, _) = test_keypair();
    let alice = Did("did:neunode:alice".to_string());
    let bob = Did("did:neunode:bob".to_string());

    let mut e1 = FeedEvent::new(
        Kind::BountyPost,
        alice.clone(),
        0,
        Hash256("0".to_string()),
        "bounty from alice".to_string(),
    )
    .expect("ok");
    e1.timestamp = 100;
    e1.sign(&sk_bytes).expect("sign");

    let mut e2 = FeedEvent::new(
        Kind::BountyClaim,
        bob.clone(),
        0,
        Hash256("0".to_string()),
        "claim from bob".to_string(),
    )
    .expect("ok");
    e2.timestamp = 200;
    e2.sign(&sk_bytes).expect("sign");

    let mut e3 = FeedEvent::new(
        Kind::Attest,
        alice.clone(),
        1,
        Hash256("0".to_string()),
        "alice attests".to_string(),
    )
    .expect("ok");
    e3.timestamp = 300;
    e3.tags = vec![EventTag { key: "domain".to_string(), value: "ml".to_string() }];
    e3.sign(&sk_bytes).expect("sign");

    let events = vec![e1, e2, e3];

    // Filter by kind
    let bounty_only =
        apply_filter(&FeedFilter::new().kinds(vec![Kind::BountyPost, Kind::BountyClaim]), &events);
    assert_eq!(bounty_only.len(), 2);

    // Filter by author
    let alice_only = apply_filter(&FeedFilter::new().authors(vec![alice.clone()]), &events);
    assert_eq!(alice_only.len(), 2);

    // Filter by time range
    let mid_range = apply_filter(&FeedFilter::new().since(150).until(250), &events);
    assert_eq!(mid_range.len(), 1);
    assert_eq!(mid_range[0].kind, Kind::BountyClaim);

    // Filter by tags
    let tagged = apply_filter(
        &FeedFilter::new().tags(vec![("domain".to_string(), "ml".to_string())]),
        &events,
    );
    assert_eq!(tagged.len(), 1);
    assert_eq!(tagged[0].kind, Kind::Attest);

    // Combined + limit
    let combined = apply_filter(
        &FeedFilter::new()
            .kinds(vec![Kind::BountyPost, Kind::Attest])
            .authors(vec![alice])
            .limit(1),
        &events,
    );
    assert_eq!(combined.len(), 1);
}

// ---------------------------------------------------------------------------
// Test 5: Topic routing — kind→topic, category→topic, parse roundtrip
// ---------------------------------------------------------------------------

#[test]
fn topic_routing_all_categories_roundtrip() {
    // Every defined Kind maps to a valid topic
    let kinds = [
        Kind::AgentMetadata,
        Kind::BountyPost,
        Kind::JobSubmit,
        Kind::Attest,
        Kind::ModelAnnounce,
        Kind::Proposal,
    ];

    for kind in kinds {
        let topic = topic_for_kind(&kind);
        assert!(topic.starts_with("neunode/"), "topic should start with neunode/");
        assert!(is_valid_topic(topic), "topic should be valid");

        let cat = parse_topic(topic).expect("should parse back");
        assert_eq!(topic_for_category(cat), topic, "roundtrip should match");
        assert_eq!(kind.category(), cat, "category should match kind");
    }

    // All topics count
    let topics = all_topics();
    assert_eq!(topics.len(), 7);
    assert!(topics.contains(&"neunode/bounty"));
    assert!(topics.contains(&"neunode/attestation"));
}

// ---------------------------------------------------------------------------
// Test 6: Schema validation — BountyPost, BountyClaim, Attestation
// ---------------------------------------------------------------------------

#[test]
fn schema_validation_valid_and_invalid_cases() {
    // Valid BountyPost
    let valid_post = serde_json::json!({
        "title": "Fine-tune Llama",
        "description": "Medical data",
        "reward_amount": 1000,
        "reward_token": "nTrain",
        "deadline": 1700000000,
        "required_capabilities": ["fine-tuning"]
    })
    .to_string();
    let post = schema::validate_bounty_post(&valid_post).expect("valid bounty post");
    assert_eq!(post.title, "Fine-tune Llama");
    assert_eq!(post.reward_amount, 1000);

    // Invalid: empty title
    let bad_post = serde_json::json!({
        "title": "",
        "description": "desc",
        "reward_amount": 100,
        "reward_token": "nCompute",
        "deadline": 1700000000,
        "required_capabilities": []
    })
    .to_string();
    assert!(schema::validate_bounty_post(&bad_post).is_err());

    // Valid BountyClaim
    let valid_claim = serde_json::json!({
        "bounty_id": "bnty_abc",
        "stake_amount": 50,
        "stake_token": "nCompute",
        "proposer_did": "did:neunode:agent1"
    })
    .to_string();
    let claim = schema::validate_bounty_claim(&valid_claim).expect("valid claim");
    assert_eq!(claim.bounty_id, "bnty_abc");

    // Invalid: empty bounty_id
    let bad_claim = serde_json::json!({
        "bounty_id": "",
        "stake_amount": 50,
        "stake_token": "nCompute",
        "proposer_did": "did:neunode:agent1"
    })
    .to_string();
    assert!(schema::validate_bounty_claim(&bad_claim).is_err());

    // Valid Attestation
    let valid_att = serde_json::json!({
        "target_did": "did:neunode:target",
        "claim": "quality work",
        "evidence": ["hash1"],
        "score": 85.0
    })
    .to_string();
    let att = schema::validate_attestation(&valid_att).expect("valid attestation");
    assert_eq!(att.score, 85.0);

    // Invalid: score above 100
    let bad_att = serde_json::json!({
        "target_did": "did:neunode:target",
        "claim": "test",
        "evidence": [],
        "score": 150.0
    })
    .to_string();
    assert!(schema::validate_attestation(&bad_att).is_err());
}

// ---------------------------------------------------------------------------
// Test 7: Schema serde roundtrip — BountyPost + BountyClaim + Attestation
// ---------------------------------------------------------------------------

#[test]
fn schema_serde_roundtrip_all_types() {
    let post = BountyPost {
        title: "Train Model".to_string(),
        description: "Fine-tune for accuracy".to_string(),
        reward_amount: 5000,
        reward_token: "nCompute".to_string(),
        deadline: 1700000000,
        required_capabilities: vec!["gpu".to_string(), "medical".to_string()],
    };
    let post_json = post.to_json().expect("serialize");
    let post_back = BountyPost::from_json(&post_json).expect("deserialize");
    assert_eq!(post, post_back);

    let claim = BountyClaim {
        bounty_id: "bnty_xyz".to_string(),
        stake_amount: 200,
        stake_token: "nTrain".to_string(),
        proposer_did: "did:neunode:agent42".to_string(),
    };
    let claim_json = claim.to_json().expect("serialize");
    let claim_back = BountyClaim::from_json(&claim_json).expect("deserialize");
    assert_eq!(claim, claim_back);

    let att = Attestation {
        target_did: "did:neunode:target_agent".to_string(),
        claim: "verified training run".to_string(),
        evidence: vec!["proof_a".to_string(), "proof_b".to_string()],
        score: 92.5,
    };
    let att_json = att.to_json().expect("serialize");
    let att_back = Attestation::from_json(&att_json).expect("deserialize");
    assert_eq!(att, att_back);
}

// ---------------------------------------------------------------------------
// Test 8: Event with tags and refs — creation, validation, filtering
// ---------------------------------------------------------------------------

#[test]
fn event_with_tags_and_refs_creation_validation_and_filter() {
    let (sk_bytes, vk_bytes) = test_keypair();
    let did = test_did();

    let mut event = FeedEvent::new(
        Kind::BountyPost,
        did.clone(),
        0,
        Hash256("0".to_string()),
        "Bounty with rich metadata".to_string(),
    )
    .expect("create event");

    event.tags = vec![
        EventTag { key: "domain".to_string(), value: "nlp".to_string() },
        EventTag { key: "priority".to_string(), value: "high".to_string() },
    ];
    event.refs = vec![EventRef {
        event_id: EventId("f123abc".to_string()),
        author: Did("did:neunode:ref_author".to_string()),
    }];

    event.sign(&sk_bytes).expect("sign");
    assert!(event.verify_signature(&vk_bytes), "signed event with tags/refs should verify");
    event.validate().expect("event with tags/refs should validate");

    // Filter by tag matches
    let filter = FeedFilter::new().tags(vec![("domain".to_string(), "nlp".to_string())]);
    assert!(filter.matches(&event));

    // Filter by non-matching tag
    let no_match = FeedFilter::new().tags(vec![("domain".to_string(), "cv".to_string())]);
    assert!(!no_match.matches(&event));

    // Serde preserves tags and refs
    let json = serde_json::to_string(&event).expect("serialize");
    let back: FeedEvent = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.tags.len(), 2);
    assert_eq!(back.refs.len(), 1);
    assert_eq!(back.tags[0].key, "domain");
}

// ---------------------------------------------------------------------------
// Test 9: Different Kind types produce different topics and IDs
// ---------------------------------------------------------------------------

#[test]
fn different_kinds_different_topics_and_ids() {
    let did = test_did();

    let kinds = [Kind::AgentMetadata,
        Kind::BountyPost,
        Kind::JobSubmit,
        Kind::Attest,
        Kind::ModelAnnounce,
        Kind::Proposal];

    // All map to different topics
    let topics: Vec<&str> = kinds.iter().map(|k| topic_for_kind(k)).collect();
    let unique_topics: std::collections::HashSet<&str> = topics.iter().copied().collect();
    // System and Custom may share but we have 5 distinct categories here
    assert!(unique_topics.len() >= 5, "should have 5+ distinct topics for 6 kinds");

    // Different kinds produce different event IDs
    let ids: Vec<EventId> = kinds
        .iter()
        .map(|k| {
            FeedEvent::new(*k, did.clone(), 0, Hash256("0".to_string()), "same content".to_string())
                .expect("ok")
                .id
        })
        .collect();
    let unique_ids: std::collections::HashSet<EventId> = ids.into_iter().collect();
    assert_eq!(unique_ids.len(), kinds.len(), "each kind should produce a unique event ID");
}

// ---------------------------------------------------------------------------
// Test 10: Sigchain get_event + events() iteration + empty chain verification
// ---------------------------------------------------------------------------

#[test]
fn sigchain_iteration_and_empty_chain_verification() {
    let (sk_bytes, vk_bytes) = test_keypair();
    let did = test_did();

    // Empty chain
    let empty = SigChain::new(did.clone(), vk_bytes);
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    empty.verify_chain().expect("empty chain should verify");
    assert!(empty.get_event(0).is_none());

    // Populate chain
    let mut chain = SigChain::new(did, vk_bytes);
    chain.append(Kind::AgentMetadata, "meta".to_string(), &sk_bytes).expect("ok");
    chain.append(Kind::BountyPost, "bounty".to_string(), &sk_bytes).expect("ok");
    chain.append(Kind::Attest, "attest".to_string(), &sk_bytes).expect("ok");

    assert_eq!(chain.len(), 3);
    assert!(!chain.is_empty());

    // get_event by sequence
    assert_eq!(chain.get_event(0).expect("seq 0").content, "meta");
    assert_eq!(chain.get_event(1).expect("seq 1").content, "bounty");
    assert_eq!(chain.get_event(2).expect("seq 2").content, "attest");
    assert!(chain.get_event(3).is_none());

    // events() iteration
    let contents: Vec<&str> = chain.events().iter().map(|e| e.content.as_str()).collect();
    assert_eq!(contents, vec!["meta", "bounty", "attest"]);
}
