//! P2P integration tests — two nodes on localhost exchanging messages via gossipsub.

use std::time::Duration;

use libp2p::identity::Keypair;
use libp2p::Multiaddr;
use neunode_core::kind::Kind;
use neunode_core::types::{Did, Hash256};
use neunode_crypto::ed25519;
use neunode_feed::event::{EventTag, FeedEvent};
use neunode_p2p::node::{NodeEvent, P2pNode};

const EVENT_TIMEOUT: Duration = Duration::from_secs(10);
const MESH_DRIVE_DURATION: Duration = Duration::from_secs(2);
const DRIVE_POLL_TIMEOUT: Duration = Duration::from_millis(50);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a P2pNode on a random port, driving the swarm so the listen
/// address is available via `listeners()`.  The `NewListenAddr` swarm event
/// is consumed internally by `next_event()` (it doesn't surface as a
/// `NodeEvent`), so we must poll at least once after `start()`.
static NODE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

async fn create_node() -> P2pNode {
    let keypair = Keypair::generate_ed25519();
    let listen_addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
    let id = NODE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let data_dir = std::env::temp_dir().join(format!(
        "neunode_p2p_test_{}_{}_{}",
        std::process::id(),
        id,
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

/// Drains unrelated events (Identify, Ping, etc.) until one matching
/// `predicate` is found or `timeout` elapses. Returns `None` on timeout.
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

/// Alternates short polls on both nodes so gossipsub can exchange GRAFT
/// messages and form the topic mesh.
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

    // Both swarms must be polled to complete the dial: B initiates the
    // TCP connection (needs polling), A accepts it (needs polling).
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

fn create_signed_event(content: &str, tags: Vec<EventTag>) -> FeedEvent {
    let seed = [42u8; 32];
    let (sk, vk) = ed25519::keypair_from_seed(&seed);
    let sk_bytes = ed25519::signing_key_to_bytes(&sk);

    let did = Did("did:neunode:test_agent_a".to_string());
    let mut event =
        FeedEvent::new(Kind::BountyPost, did, 0, Hash256("0".to_string()), content.to_string())
            .unwrap();

    event.tags = tags;
    event.sign(&sk_bytes).unwrap();

    let vk_bytes = ed25519::verifying_key_to_bytes(&vk);
    assert!(event.verify_signature(&vk_bytes), "signature should verify");
    event.validate().unwrap();
    event
}

async fn publish_and_receive(
    src: &mut P2pNode,
    dst: &mut P2pNode,
    topic: &str,
    payload: &[u8],
) -> NodeEvent {
    src.publish(topic, payload).unwrap();
    let _ = tokio::time::timeout(Duration::from_millis(200), src.next_event()).await;
    let received =
        wait_for_event(dst, |e| matches!(e, NodeEvent::GossipsubMessage { .. }), EVENT_TIMEOUT)
            .await;

    received.expect("destination node should receive the gossipsub message")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Test 1: Two nodes connect on localhost and see each other as peers.
#[tokio::test]
async fn two_nodes_connect() {
    let mut node_a = create_node().await;
    let mut node_b = create_node().await;

    subscribe_both(&mut node_a, &mut node_b, "neunode/bounty");
    connect_and_wait(&mut node_a, &mut node_b).await;

    assert!(node_a.is_connected(&node_b.local_peer_id()), "A should see B as connected");
    assert!(node_b.is_connected(&node_a.local_peer_id()), "B should see A as connected");
}

/// Test 2: Gossipsub message delivery between two connected nodes.
#[tokio::test]
async fn gossipsub_message_delivery() {
    let (mut node_a, mut node_b) = setup_two_nodes("neunode/bounty").await;

    let payload = b"hello neunode from node A".to_vec();
    let event = publish_and_receive(&mut node_a, &mut node_b, "neunode/bounty", &payload).await;

    match event {
        NodeEvent::GossipsubMessage { data, topic, .. } => {
            assert_eq!(data, payload, "payload should match");
            assert!(topic.contains("bounty"), "topic should contain 'bounty': got {topic}");
        }
        other => panic!("expected GossipsubMessage, got {other:?}"),
    }
}

/// Test 3: Multiple messages delivered in sequence.
#[tokio::test]
async fn gossipsub_multiple_messages() {
    let (mut node_a, mut node_b) = setup_two_nodes("neunode/bounty").await;

    let messages: Vec<Vec<u8>> = (0..3).map(|i| format!("message {i}").into_bytes()).collect();

    for msg in &messages {
        node_a.publish("neunode/bounty", msg).unwrap();
        let _ = tokio::time::timeout(Duration::from_millis(100), node_a.next_event()).await;
    }

    let mut received: Vec<Vec<u8>> = Vec::new();
    let deadline = tokio::time::Instant::now() + EVENT_TIMEOUT;

    while received.len() < 3 && tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, node_b.next_event()).await {
            Ok(NodeEvent::GossipsubMessage { data, .. }) => {
                received.push(data);
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    assert_eq!(received.len(), 3, "should receive exactly 3 messages");
    for msg in &messages {
        assert!(
            received.iter().any(|r| r == msg),
            "message {:?} should be in received set",
            String::from_utf8_lossy(msg)
        );
    }
}

/// Test 4: FeedEvent publish → gossipsub → receive → deserialize roundtrip.
#[tokio::test]
async fn feed_event_publish_receive_roundtrip() {
    let (mut node_a, mut node_b) = setup_two_nodes("neunode/bounty").await;

    let event = create_signed_event("test bounty post", vec![]);
    let json_bytes = serde_json::to_vec(&event).unwrap();

    let msg = publish_and_receive(&mut node_a, &mut node_b, "neunode/bounty", &json_bytes).await;

    match msg {
        NodeEvent::GossipsubMessage { data, .. } => {
            let received: FeedEvent =
                serde_json::from_slice(&data).expect("data should deserialize as FeedEvent");
            assert_eq!(received.content, "test bounty post");
            assert_eq!(received.kind, Kind::BountyPost);
            assert_eq!(received.author, event.author);
            assert!(received.signature.is_some(), "signature should be preserved");
            assert!(received.id.0.starts_with('f'), "event ID should start with 'f'");
        }
        other => panic!("expected GossipsubMessage, got {other:?}"),
    }
}

/// Test 5: FeedEvent with tags roundtrip — tags survive gossipsub transport.
#[tokio::test]
async fn feed_event_with_tags_roundtrip() {
    let (mut node_a, mut node_b) = setup_two_nodes("neunode/bounty").await;

    let tags = vec![
        EventTag { key: "domain".to_string(), value: "nlp".to_string() },
        EventTag { key: "priority".to_string(), value: "high".to_string() },
    ];
    let event = create_signed_event("tagged bounty", tags);
    let json_bytes = serde_json::to_vec(&event).unwrap();

    let msg = publish_and_receive(&mut node_a, &mut node_b, "neunode/bounty", &json_bytes).await;

    match msg {
        NodeEvent::GossipsubMessage { data, .. } => {
            let received: FeedEvent =
                serde_json::from_slice(&data).expect("data should deserialize as FeedEvent");
            assert_eq!(received.content, "tagged bounty");
            assert_eq!(received.tags.len(), 2, "both tags should be preserved");
            assert_eq!(received.tags[0].key, "domain");
            assert_eq!(received.tags[0].value, "nlp");
            assert_eq!(received.tags[1].key, "priority");
            assert_eq!(received.tags[1].value, "high");
        }
        other => panic!("expected GossipsubMessage, got {other:?}"),
    }
}

/// Test 6: Invalid data is delivered as raw bytes but cannot be deserialized as FeedEvent.
#[tokio::test]
async fn invalid_data_not_deserialized() {
    let (mut node_a, mut node_b) = setup_two_nodes("neunode/bounty").await;

    let garbage = b"this is not valid json or a feed event".to_vec();
    let msg = publish_and_receive(&mut node_a, &mut node_b, "neunode/bounty", &garbage).await;

    match msg {
        NodeEvent::GossipsubMessage { data, .. } => {
            assert_eq!(data, garbage, "raw bytes should be delivered unchanged");
            let result: Result<FeedEvent, _> = serde_json::from_slice(&data);
            assert!(result.is_err(), "garbage data should not deserialize as FeedEvent");
        }
        other => panic!("expected GossipsubMessage, got {other:?}"),
    }
}
