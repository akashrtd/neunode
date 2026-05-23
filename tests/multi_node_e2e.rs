//! Multi-node end-to-end integration tests.
//!
//! Exercises the full bounty lifecycle across two P2P agents:
//! gossipsub announcement → claim → submit → review → pay.

use std::time::Duration;

use libp2p::identity::Keypair;
use libp2p::Multiaddr;
use neunode_bounty::lifecycle::BountyManager;
use neunode_bounty::review::{Review, ReviewOutcome};
use neunode_core::kind::Kind;
use neunode_core::types::{BountyId, BountyState, Did, Hash256, Timestamp, TokenAmount, TokenType};
use neunode_crypto::ed25519;
use neunode_feed::event::{EventTag, FeedEvent};
use neunode_p2p::node::{NodeEvent, P2pNode};

const EVENT_TIMEOUT: Duration = Duration::from_secs(10);
const MESH_DRIVE_DURATION: Duration = Duration::from_secs(2);
const DRIVE_POLL_TIMEOUT: Duration = Duration::from_millis(50);
const TOPIC: &str = "neunode/bounty";

// ---------------------------------------------------------------------------
// P2P helpers (copied from p2p_gossipsub.rs — same patterns, no import)
// ---------------------------------------------------------------------------

async fn create_node() -> P2pNode {
    let keypair = Keypair::generate_ed25519();
    let listen_addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
    let data_dir = std::env::temp_dir().join(format!(
        "neunode_e2e_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let mut node = P2pNode::new(keypair, listen_addr.clone(), &data_dir).unwrap();
    node.start(listen_addr).unwrap();
    let _ = tokio::time::timeout(Duration::from_millis(300), node.next_event()).await;
    assert!(node.listeners().count() > 0, "node should have at least one listener after start");
    node
}

fn node_addr(node: &P2pNode) -> Multiaddr {
    node.listeners().next().expect("node should have a listener").clone()
}

fn addr_with_peer(addr: Multiaddr, peer_id: libp2p::PeerId) -> Multiaddr {
    format!("{}/p2p/{}", addr, peer_id).parse().unwrap()
}

async fn wait_for_event<F>(node: &mut P2pNode, predicate: F, timeout: Duration) -> Option<NodeEvent>
where
    F: Fn(&NodeEvent) -> bool,
{
    tokio::time::timeout(timeout, async {
        loop {
            let event = node.next_event().await;
            if predicate(&event) {
                return Some(event);
            }
        }
    })
    .await
    .ok()
    .flatten()
}

async fn drive_mesh(node_a: &mut P2pNode, node_b: &mut P2pNode) {
    let deadline = tokio::time::Instant::now() + MESH_DRIVE_DURATION;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        let poll = remaining.min(DRIVE_POLL_TIMEOUT);
        let _ = tokio::time::timeout(poll, node_a.next_event()).await;

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        let poll = remaining.min(DRIVE_POLL_TIMEOUT);
        let _ = tokio::time::timeout(poll, node_b.next_event()).await;
    }
}

fn subscribe_both(node_a: &mut P2pNode, node_b: &mut P2pNode, topic: &str) {
    node_a.subscribe(topic).unwrap();
    node_b.subscribe(topic).unwrap();
}

async fn connect_and_wait(node_a: &mut P2pNode, node_b: &mut P2pNode) {
    let addr = node_addr(node_a);
    let peer = node_a.local_peer_id();
    let target = addr_with_peer(addr, peer);
    node_b.dial(target).unwrap();

    let mut a_connected = false;
    let mut b_connected = false;
    let deadline = tokio::time::Instant::now() + EVENT_TIMEOUT;

    while !(a_connected && b_connected) {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        let poll = remaining.min(Duration::from_millis(50));

        if !b_connected {
            if let Ok(event) = tokio::time::timeout(poll, node_b.next_event()).await {
                if matches!(event, NodeEvent::PeerConnected(_)) {
                    b_connected = true;
                }
            }
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        let poll = remaining.min(Duration::from_millis(50));

        if !a_connected {
            if let Ok(event) = tokio::time::timeout(poll, node_a.next_event()).await {
                if matches!(event, NodeEvent::PeerConnected(_)) {
                    a_connected = true;
                }
            }
        }
    }

    assert!(a_connected, "Node A should see B connect");
    assert!(b_connected, "Node B should see A connect");
}

async fn setup_two_nodes(topic: &str) -> (P2pNode, P2pNode) {
    let mut node_a = create_node().await;
    let mut node_b = create_node().await;
    subscribe_both(&mut node_a, &mut node_b, topic);
    connect_and_wait(&mut node_a, &mut node_b).await;
    drive_mesh(&mut node_a, &mut node_b).await;
    (node_a, node_b)
}

// ---------------------------------------------------------------------------
// Bounty helpers (adapted from bounty_flow.rs)
// ---------------------------------------------------------------------------

fn test_did(name: &str) -> Did {
    Did(format!("did:neunode:{name}"))
}

fn base_time() -> Timestamp {
    1_700_000_000
}

fn make_review(reviewer: &str, score: u8) -> Review {
    Review::new(test_did(reviewer), score, String::new(), base_time(), None).expect("valid review")
}

fn create_signed_event(
    agent_seed: [u8; 32],
    did: Did,
    kind: Kind,
    content: &str,
    tags: Vec<EventTag>,
) -> FeedEvent {
    let (sk, vk) = ed25519::keypair_from_seed(&agent_seed);
    let sk_bytes = ed25519::signing_key_to_bytes(&sk);
    let mut event = FeedEvent::new(kind, did, 0, Hash256("0".to_string()), content.to_string())
        .expect("event creation should succeed");
    event.tags = tags;
    event.sign(&sk_bytes).expect("signing should succeed");
    let vk_bytes = ed25519::verifying_key_to_bytes(&vk);
    assert!(event.verify_signature(&vk_bytes), "signature should verify");
    event.validate().expect("event should validate");
    event
}

async fn publish_and_receive(
    src: &mut P2pNode,
    dst: &mut P2pNode,
    topic: &str,
    payload: &[u8],
) -> NodeEvent {
    src.publish(topic, payload).expect("publish should succeed");
    // Drain the source's own event (publish acknowledgement / relay echo)
    let _ = tokio::time::timeout(Duration::from_millis(200), src.next_event()).await;
    wait_for_event(dst, |e| matches!(e, NodeEvent::GossipsubMessage { .. }), EVENT_TIMEOUT)
        .await
        .expect("destination node should receive the gossipsub message")
}

/// Run the full bounty review lifecycle on a BountyManager:
/// claim → submit → review (3 reviewers, all passing) → pay.
fn run_full_bounty_lifecycle(
    mgr: &mut BountyManager,
    bounty_id: &BountyId,
    claimant: Did,
    bond: TokenAmount,
) {
    mgr.claim_bounty(bounty_id, claimant, bond, base_time() + 100).expect("claim should succeed");

    mgr.submit_work(bounty_id, Hash256("artifact_hash_v1".to_string()), base_time() + 200)
        .expect("submit should succeed");

    let reviewers = vec![test_did("r1"), test_did("r2"), test_did("r3")];
    mgr.start_review(bounty_id, reviewers, base_time() + 300).expect("start review should succeed");

    mgr.submit_review(bounty_id, make_review("r1", 85), base_time() + 400).expect("review r1");
    mgr.submit_review(bounty_id, make_review("r2", 90), base_time() + 401).expect("review r2");
    mgr.submit_review(bounty_id, make_review("r3", 80), base_time() + 402).expect("review r3");

    let outcome = mgr.complete_review(bounty_id, base_time() + 500).expect("complete review");
    assert_eq!(outcome, ReviewOutcome::Approved);

    let fees = mgr.pay_bounty(bounty_id, base_time() + 600).expect("pay bounty");
    assert_eq!(mgr.get_state(bounty_id), Some(BountyState::Paid));
    assert!(fees.protocol_fee.0 > 0, "protocol fee should be deducted");
    assert!(fees.net_amount.0 < fees.gross_amount.0, "net should be less than gross");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Test 1: Full E2E — two agents, P2P discovery, bounty lifecycle.
/// Agent A creates a bounty, announces it via gossipsub, Agent B receives
/// the feed event, then the full bounty lifecycle runs (claim → pay).
#[tokio::test]
async fn full_e2e_bounty_via_p2p() {
    let (mut node_a, mut node_b) = setup_two_nodes(TOPIC).await;

    // Agent A: create bounty + build feed event
    let mut mgr = BountyManager::new();
    let bounty_data = mgr.create_bounty(
        test_did("agent_a"),
        "Train Llama-3B on medical data".to_string(),
        "Fine-tune for >95% accuracy".to_string(),
        TokenAmount(1000),
        TokenType::Compute,
        base_time(),
    );
    assert_eq!(bounty_data.state, BountyState::Open);

    let bounty_json = serde_json::json!({
        "id": bounty_data.id.0,
        "title": bounty_data.title,
        "reward": bounty_data.reward_amount.0,
        "token": format!("{:?}", bounty_data.reward_token),
        "state": format!("{:?}", bounty_data.state),
    });
    let content = serde_json::to_string(&bounty_json).expect("bounty JSON should serialize");

    let agent_a_seed = [42u8; 32];
    let event =
        create_signed_event(agent_a_seed, test_did("agent_a"), Kind::BountyPost, &content, vec![]);
    let json_bytes = serde_json::to_vec(&event).expect("FeedEvent should serialize to JSON");

    // Agent A: publish via gossipsub
    let msg = publish_and_receive(&mut node_a, &mut node_b, TOPIC, &json_bytes).await;

    // Agent B: receive and deserialize
    let received_data = match msg {
        NodeEvent::GossipsubMessage { data, topic, .. } => {
            assert!(topic.contains("bounty"), "topic should contain 'bounty': got {topic}");
            data
        }
        other => panic!("expected GossipsubMessage, got {other:?}"),
    };

    let feed_event: FeedEvent =
        serde_json::from_slice(&received_data).expect("data should deserialize as FeedEvent");
    assert_eq!(feed_event.kind, Kind::BountyPost);
    assert_eq!(feed_event.author, test_did("agent_a"));
    assert!(feed_event.signature.is_some(), "signature should be preserved");

    // Agent B: extract bounty details from feed event content
    let bounty_info: serde_json::Value =
        serde_json::from_str(&feed_event.content).expect("content should be valid JSON");
    let received_bounty_id =
        bounty_info["id"].as_str().expect("bounty JSON should contain 'id' field");
    assert_eq!(received_bounty_id, bounty_data.id.0, "bounty ID should match what Agent A created");
    assert_eq!(bounty_info["title"].as_str(), Some("Train Llama-3B on medical data"));

    // Run full bounty lifecycle (Agent B claims, submits, gets reviewed, gets paid)
    run_full_bounty_lifecycle(&mut mgr, &bounty_data.id, test_did("agent_b"), TokenAmount(200));

    // Verify final state
    assert_eq!(mgr.get_state(&bounty_data.id), Some(BountyState::Paid));
}

/// Test 2: Agent B receives bounty announcement but bounty is already claimed.
#[tokio::test]
async fn bounty_already_claimed_on_receipt() {
    let (mut node_a, mut node_b) = setup_two_nodes(TOPIC).await;

    // Agent A: create bounty + build feed event
    let mut mgr = BountyManager::new();
    let bounty_data = mgr.create_bounty(
        test_did("agent_a"),
        "Already-claimed bounty".to_string(),
        String::new(),
        TokenAmount(500),
        TokenType::Train,
        base_time(),
    );

    let bounty_json = serde_json::json!({
        "id": bounty_data.id.0,
        "title": bounty_data.title,
        "reward": bounty_data.reward_amount.0,
        "token": format!("{:?}", bounty_data.reward_token),
        "state": format!("{:?}", bounty_data.state),
    });
    let content = serde_json::to_string(&bounty_json).expect("serialize bounty JSON");

    let agent_a_seed = [42u8; 32];
    let event =
        create_signed_event(agent_a_seed, test_did("agent_a"), Kind::BountyPost, &content, vec![]);
    let json_bytes = serde_json::to_vec(&event).expect("serialize FeedEvent");

    // Publish the announcement
    let msg = publish_and_receive(&mut node_a, &mut node_b, TOPIC, &json_bytes).await;

    // Agent A claims it immediately (before B acts)
    mgr.claim_bounty(&bounty_data.id, test_did("agent_a_self"), TokenAmount(100), base_time() + 50)
        .expect("Agent A should claim successfully");
    assert_eq!(mgr.get_state(&bounty_data.id), Some(BountyState::Claimed));

    // Agent B receives and deserializes
    let received_data = match msg {
        NodeEvent::GossipsubMessage { data, .. } => data,
        other => panic!("expected GossipsubMessage, got {other:?}"),
    };
    let feed_event: FeedEvent =
        serde_json::from_slice(&received_data).expect("should deserialize FeedEvent");
    assert_eq!(feed_event.kind, Kind::BountyPost);

    // Agent B tries to claim → fails (already claimed by A)
    let claim_result =
        mgr.claim_bounty(&bounty_data.id, test_did("agent_b"), TokenAmount(100), base_time() + 60);
    assert!(claim_result.is_err(), "Agent B should not be able to claim an already-claimed bounty");

    // Verify state is still Claimed by A
    assert_eq!(mgr.get_state(&bounty_data.id), Some(BountyState::Claimed));
}

/// Test 3: Feed event with bounty metadata tags — tags survive P2P transport.
#[tokio::test]
async fn bounty_feed_event_with_capability_tags() {
    let (mut node_a, mut node_b) = setup_two_nodes(TOPIC).await;

    // Agent A: create bounty + feed event with tags (domain=NLP, priority=high)
    let mut mgr = BountyManager::new();
    let bounty_data = mgr.create_bounty(
        test_did("agent_a"),
        "NLP sentiment analysis".to_string(),
        "Classify medical text sentiment".to_string(),
        TokenAmount(2000),
        TokenType::Compute,
        base_time(),
    );

    let bounty_json = serde_json::json!({
        "id": bounty_data.id.0,
        "title": bounty_data.title,
        "reward": bounty_data.reward_amount.0,
        "token": format!("{:?}", bounty_data.reward_token),
        "state": format!("{:?}", bounty_data.state),
    });
    let content = serde_json::to_string(&bounty_json).expect("serialize bounty JSON");

    let tags = vec![
        EventTag { key: "domain".to_string(), value: "NLP".to_string() },
        EventTag { key: "priority".to_string(), value: "high".to_string() },
    ];

    let agent_a_seed = [42u8; 32];
    let event =
        create_signed_event(agent_a_seed, test_did("agent_a"), Kind::BountyPost, &content, tags);
    let json_bytes = serde_json::to_vec(&event).expect("serialize FeedEvent");

    // Publish via gossipsub
    let msg = publish_and_receive(&mut node_a, &mut node_b, TOPIC, &json_bytes).await;

    // Agent B receives and verifies tags are preserved
    let received_data = match msg {
        NodeEvent::GossipsubMessage { data, .. } => data,
        other => panic!("expected GossipsubMessage, got {other:?}"),
    };
    let feed_event: FeedEvent =
        serde_json::from_slice(&received_data).expect("should deserialize FeedEvent");
    assert_eq!(feed_event.kind, Kind::BountyPost);
    assert_eq!(feed_event.tags.len(), 2, "both tags should survive P2P transport");
    assert_eq!(feed_event.tags[0].key, "domain");
    assert_eq!(feed_event.tags[0].value, "NLP");
    assert_eq!(feed_event.tags[1].key, "priority");
    assert_eq!(feed_event.tags[1].value, "high");

    // Run full bounty lifecycle to completion
    run_full_bounty_lifecycle(&mut mgr, &bounty_data.id, test_did("agent_b"), TokenAmount(300));
    assert_eq!(mgr.get_state(&bounty_data.id), Some(BountyState::Paid));
}
