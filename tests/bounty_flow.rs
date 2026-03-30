use neunode_bounty::escrow::{EscrowManager, FeeBreakdown};
use neunode_bounty::lifecycle::BountyManager;
use neunode_bounty::review::{Review, ReviewOutcome};
use neunode_bounty::state_machine::{BountyData, BountyEvent, BountyStateMachine, Deadlines};
use neunode_bounty::verification::{VerificationLayer, VerificationPipeline};
use neunode_core::types::{BountyId, BountyState, Did, Hash256, Timestamp, TokenAmount, TokenType};

fn test_did(name: &str) -> Did {
    Did(format!("did:neunode:{name}"))
}

fn base_time() -> Timestamp {
    1_700_000_000
}

#[test]
fn full_bounty_lifecycle_open_to_paid() {
    let mut mgr = BountyManager::new();
    let data = mgr.create_bounty(
        test_did("creator"),
        "Train Llama-3B on medical data".to_string(),
        "Fine-tune for >95% accuracy".to_string(),
        TokenAmount(1000),
        TokenType::Compute,
        base_time(),
    );
    assert_eq!(data.state, BountyState::Open);

    mgr.claim_bounty(&data.id, test_did("worker"), TokenAmount(200), base_time() + 100)
        .expect("claim should succeed");

    mgr.submit_work(&data.id, Hash256("artifact_hash_v1".to_string()), base_time() + 200)
        .expect("submit should succeed");

    let reviewers = vec![test_did("r1"), test_did("r2"), test_did("r3")];
    mgr.start_review(&data.id, reviewers, base_time() + 300).expect("start review should succeed");

    mgr.submit_review(&data.id, make_review("r1", 85), base_time() + 400).expect("review r1");
    mgr.submit_review(&data.id, make_review("r2", 90), base_time() + 401).expect("review r2");
    mgr.submit_review(&data.id, make_review("r3", 80), base_time() + 402).expect("review r3");

    let outcome = mgr.complete_review(&data.id, base_time() + 500).expect("complete review");
    assert_eq!(outcome, ReviewOutcome::Approved);
    assert_eq!(mgr.get_state(&data.id), Some(BountyState::Accepted));

    let fees = mgr.pay_bounty(&data.id, base_time() + 600).expect("pay bounty");
    assert_eq!(mgr.get_state(&data.id), Some(BountyState::Paid));
    assert!(fees.protocol_fee.0 > 0, "protocol fee should be deducted");
    assert!(fees.net_amount.0 < fees.gross_amount.0, "net should be less than gross");
}

#[test]
fn bounty_rejection_path_with_escrow_refund() {
    let mut mgr = BountyManager::new();
    let data = mgr.create_bounty(
        test_did("creator"),
        "Bad bounty".to_string(),
        String::new(),
        TokenAmount(500),
        TokenType::Train,
        base_time(),
    );

    mgr.claim_bounty(&data.id, test_did("worker"), TokenAmount(100), base_time() + 100)
        .expect("claim");
    mgr.submit_work(&data.id, Hash256("bad_work".to_string()), base_time() + 200).expect("submit");

    let reviewers = vec![test_did("r1"), test_did("r2"), test_did("r3")];
    mgr.start_review(&data.id, reviewers, base_time() + 300).expect("review");

    mgr.submit_review(&data.id, make_review("r1", 10), base_time() + 400).expect("r1");
    mgr.submit_review(&data.id, make_review("r2", 20), base_time() + 401).expect("r2");
    mgr.submit_review(&data.id, make_review("r3", 15), base_time() + 402).expect("r3");

    let outcome = mgr.complete_review(&data.id, base_time() + 500).expect("complete");
    assert_eq!(outcome, ReviewOutcome::Rejected);
    assert_eq!(mgr.get_state(&data.id), Some(BountyState::Rejected));
}

#[test]
fn bounty_revision_then_approval_path() {
    let mut mgr = BountyManager::new();
    let data = mgr.create_bounty(
        test_did("creator"),
        "Revision test".to_string(),
        String::new(),
        TokenAmount(1000),
        TokenType::Compute,
        base_time(),
    );

    mgr.claim_bounty(&data.id, test_did("worker"), TokenAmount(200), base_time() + 100)
        .expect("claim");
    mgr.submit_work(&data.id, Hash256("v1".to_string()), base_time() + 200).expect("submit v1");

    let revs = vec![test_did("r1"), test_did("r2"), test_did("r3")];
    mgr.start_review(&data.id, revs, base_time() + 300).expect("review 1");

    mgr.submit_review(&data.id, make_review("r1", 50), base_time() + 400).expect("r1");
    mgr.submit_review(&data.id, make_review("r2", 55), base_time() + 401).expect("r2");
    mgr.submit_review(&data.id, make_review("r3", 45), base_time() + 402).expect("r3");

    let outcome = mgr.complete_review(&data.id, base_time() + 500).expect("complete");
    assert_eq!(outcome, ReviewOutcome::NeedsRevision);
    assert_eq!(mgr.get_state(&data.id), Some(BountyState::Revision));

    mgr.submit_work(&data.id, Hash256("v2".to_string()), base_time() + 600).expect("submit v2");
    let revs2 = vec![test_did("r1"), test_did("r2"), test_did("r3")];
    mgr.start_review(&data.id, revs2, base_time() + 700).expect("review 2");

    mgr.submit_review(&data.id, make_review("r1", 90), base_time() + 800).expect("r1");
    mgr.submit_review(&data.id, make_review("r2", 85), base_time() + 801).expect("r2");
    mgr.submit_review(&data.id, make_review("r3", 80), base_time() + 802).expect("r3");

    let outcome2 = mgr.complete_review(&data.id, base_time() + 900).expect("complete 2");
    assert_eq!(outcome2, ReviewOutcome::Approved);
}

#[test]
fn escrow_fees_calculated_correctly() {
    let fb = FeeBreakdown::from_gross(TokenAmount(1000));
    assert_eq!(fb.gross_amount, TokenAmount(1000));
    assert!(fb.protocol_fee.0 > 0, "protocol fee should be non-zero");
    assert!(fb.reviewer_fee.0 > 0, "reviewer fee should be non-zero");
    assert!(fb.net_amount.0 > 0, "net should be positive");
    let total_fees = fb.protocol_fee.0 + fb.reviewer_fee.0;
    assert!(total_fees <= fb.gross_amount.0, "fees cannot exceed gross");
}

#[test]
fn escrow_lifecycle_create_release() {
    let mut mgr = EscrowManager::new();
    let id = BountyId("bnty_escrow_test".to_string());
    mgr.create_escrow(id.clone(), test_did("creator"), TokenAmount(1000), TokenType::Compute, 100)
        .expect("create escrow");

    let escrow = mgr.get_escrow(&id).expect("should exist");
    assert_eq!(escrow.amount, TokenAmount(1000));

    mgr.release(&id, test_did("worker")).expect("release should succeed");
    let escrow_after = mgr.get_escrow(&id).expect("should still exist");
    assert!(escrow_after.beneficiary.is_some());
}

#[test]
fn state_machine_allowed_transitions_match_reality() {
    let data = BountyData {
        id: BountyId("bnty_transitions".to_string()),
        creator: test_did("creator"),
        title: "Transitions".to_string(),
        description: String::new(),
        reward_amount: TokenAmount(1000),
        reward_token: TokenType::Compute,
        state: BountyState::Open,
        claimant: None,
        created_at: base_time(),
        deadlines: Deadlines::from_created_at(base_time()),
        artifact_hash: None,
        bond: None,
    };
    let sm = BountyStateMachine::new(data);
    let transitions = sm.allowed_transitions();
    assert!(transitions.contains(&BountyState::Claimed));
    assert!(transitions.contains(&BountyState::Expired));
    assert!(transitions.contains(&BountyState::Cancelled));
}

#[test]
fn verification_pipeline_runs_layers() {
    let pipeline = VerificationPipeline::default();
    let results = pipeline.run(&Hash256("artifact123".to_string()), "accuracy > 95%");
    assert_eq!(results.len(), 3, "default pipeline has 3 layers");
    assert_eq!(results[0].layer, VerificationLayer::Layer1);
    assert!(results.iter().all(|r| r.passed), "valid artifact should pass all layers");
    assert!(
        results[2].confidence > results[0].confidence,
        "confidence should increase with layer depth"
    );
}

#[test]
fn cancel_bounty_refunds_escrow() {
    let mut mgr = BountyManager::new();
    let data = mgr.create_bounty(
        test_did("creator"),
        "Cancel me".to_string(),
        String::new(),
        TokenAmount(800),
        TokenType::Bandwidth,
        base_time(),
    );

    mgr.cancel_bounty(&data.id, base_time() + 100).expect("cancel");
    assert_eq!(mgr.get_state(&data.id), Some(BountyState::Cancelled));

    let escrow = mgr.get_escrow_manager().get_escrow(&data.id).expect("escrow should exist");
    assert!(
        escrow.state == neunode_bounty::escrow::EscrowState::Refunded,
        "cancelled bounty should have refunded escrow"
    );
}

#[test]
fn bond_requirement_enforced() {
    let data = BountyData {
        id: BountyId("bnty_bond".to_string()),
        creator: test_did("creator"),
        title: "Bond test".to_string(),
        description: String::new(),
        reward_amount: TokenAmount(1000),
        reward_token: TokenType::Compute,
        state: BountyState::Open,
        claimant: None,
        created_at: base_time(),
        deadlines: Deadlines::from_created_at(base_time()),
        artifact_hash: None,
        bond: None,
    };
    let required = data.required_bond();
    assert_eq!(required, TokenAmount(150), "15% of 1000 = 150");

    let mut sm = BountyStateMachine::new(data);
    let result = sm.try_transition(
        BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(10) },
        base_time() + 100,
    );
    assert!(result.is_err(), "insufficient bond should be rejected");
}

fn make_review(reviewer: &str, score: u8) -> Review {
    Review::new(test_did(reviewer), score, String::new(), base_time(), None).expect("valid review")
}
