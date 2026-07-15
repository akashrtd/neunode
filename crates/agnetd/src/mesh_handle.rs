use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use neunode_feed::rate_limit::RateLimiter;

use anyhow::Result;
use libp2p::multiaddr::Protocol;
use libp2p::Multiaddr;
use libp2p::PeerId;
use neunode_p2p::node::{NodeEvent, P2pNode};
use neunode_storage::db::NeunodeDb;
use neunode_storage::peer_address_store::{PeerAddressRecord, PeerAddressStore};
use tokio::sync::{mpsc, oneshot};

// ---------------------------------------------------------------------------
// MeshCommand — commands sent from foreground to background task
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub enum MeshCommand {
    Publish { topic: String, data: Vec<u8> },
    Subscribe { topic: String },
    Dial { addr: Multiaddr },
    GetStatus { reply: oneshot::Sender<MeshStatus> },
    GetPeers { reply: oneshot::Sender<Vec<PeerId>> },
    Disconnect { peer_id: PeerId },
    Shutdown,
}

// ---------------------------------------------------------------------------
// MeshStatus — queryable snapshot of the mesh state
// ---------------------------------------------------------------------------

#[derive(Debug)]
#[allow(dead_code)]
pub struct MeshStatus {
    pub running: bool,
    pub local_peer_id: String,
    pub listeners: Vec<String>,
    pub connected_peers: Vec<String>,
    pub subscribed_topics: Vec<String>,
}

// ---------------------------------------------------------------------------
// MeshHandle — foreground handle to the background mesh task
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct MeshHandle {
    cmd_tx: mpsc::UnboundedSender<MeshCommand>,
    pub local_peer_id: PeerId,
    pub(crate) join_handle: tokio::task::JoinHandle<()>,
    event_rx: Option<mpsc::UnboundedReceiver<neunode_feed::event::FeedEvent>>,
}

impl MeshHandle {
    /// Publish data to a gossipsub topic.
    pub fn publish(&self, topic: &str, data: &[u8]) -> Result<()> {
        self.cmd_tx
            .send(MeshCommand::Publish { topic: topic.to_string(), data: data.to_vec() })
            .map_err(|_| anyhow::anyhow!("mesh task dropped"))
    }

    /// Subscribe to a gossipsub topic.
    #[allow(dead_code)]
    pub fn subscribe(&self, topic: &str) -> Result<()> {
        self.cmd_tx
            .send(MeshCommand::Subscribe { topic: topic.to_string() })
            .map_err(|_| anyhow::anyhow!("mesh task dropped"))
    }

    /// Dial a remote peer by multiaddr.
    pub fn dial(&self, addr: Multiaddr) -> Result<()> {
        self.cmd_tx
            .send(MeshCommand::Dial { addr })
            .map_err(|_| anyhow::anyhow!("mesh task dropped"))
    }

    /// Disconnect from a specific peer.
    pub fn disconnect(&self, peer_id: PeerId) -> Result<()> {
        self.cmd_tx
            .send(MeshCommand::Disconnect { peer_id })
            .map_err(|_| anyhow::anyhow!("mesh task dropped"))
    }

    pub async fn status(&self) -> Result<MeshStatus> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(MeshCommand::GetStatus { reply: tx })
            .map_err(|_| anyhow::anyhow!("mesh task dropped"))?;
        rx.await.map_err(|_| anyhow::anyhow!("mesh task dropped"))
    }

    pub async fn peers(&self) -> Result<Vec<PeerId>> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(MeshCommand::GetPeers { reply: tx })
            .map_err(|_| anyhow::anyhow!("mesh task dropped"))?;
        rx.await.map_err(|_| anyhow::anyhow!("mesh task dropped"))
    }

    /// Signal the background task to shut down.
    pub fn shutdown(&self) -> Result<()> {
        self.cmd_tx.send(MeshCommand::Shutdown).map_err(|_| anyhow::anyhow!("mesh task dropped"))
    }

    /// Take the event stream receiver (first call returns Some, subsequent calls return None).
    pub fn take_event_stream(
        &mut self,
    ) -> Option<mpsc::UnboundedReceiver<neunode_feed::event::FeedEvent>> {
        self.event_rx.take()
    }
}

// ---------------------------------------------------------------------------
// spawn_mesh_task — create P2pNode, spawn background loop, return handle
// ---------------------------------------------------------------------------

pub fn spawn_mesh_task(
    keypair: libp2p::identity::Keypair,
    listen_addr: Multiaddr,
    bootstrap_peers: Vec<Multiaddr>,
    subscribe_all: bool,
    db: Arc<NeunodeDb>,
    data_dir: PathBuf,
) -> Result<MeshHandle> {
    let mut node = P2pNode::new(keypair, listen_addr.clone(), &data_dir)?;
    node.start(listen_addr)?;

    if subscribe_all {
        node.subscribe_all_categories()?;
    }

    for addr in &bootstrap_peers {
        node.add_bootstrap_peer(addr.clone())?;
    }

    if !bootstrap_peers.is_empty() {
        let _ = node.bootstrap_dht();
    }

    let local_peer_id = node.local_peer_id();
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    let join_handle = tokio::spawn(mesh_event_loop(node, cmd_rx, db, event_tx));

    Ok(MeshHandle { cmd_tx, local_peer_id, join_handle, event_rx: Some(event_rx) })
}

// ---------------------------------------------------------------------------
// mesh_event_loop — background task handling commands + P2P events
// ---------------------------------------------------------------------------

async fn mesh_event_loop(
    mut node: P2pNode,
    mut cmd_rx: mpsc::UnboundedReceiver<MeshCommand>,
    db: Arc<NeunodeDb>,
    event_tx: mpsc::UnboundedSender<neunode_feed::event::FeedEvent>,
) {
    // 10 events per DID per 60-second window
    let mut rate_limiter = RateLimiter::new(10, 60);
    let mut address_book = load_address_book(&db);
    let mut retries = HashMap::<PeerId, RetryState>::new();
    let mut manual_disconnects = HashSet::<PeerId>::new();
    let mut retry_tick = tokio::time::interval(Duration::from_secs(1));
    retry_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    for peer_id in address_book.keys().copied() {
        retries.insert(peer_id, RetryState::new(0));
    }

    loop {
        tokio::select! {
            _ = retry_tick.tick() => {
                let due = retries
                    .iter()
                    .filter(|(_, retry)| retry.next_attempt <= tokio::time::Instant::now())
                    .map(|(peer_id, _)| *peer_id)
                    .collect::<Vec<_>>();
                for peer_id in due {
                    if node.is_connected(&peer_id) || manual_disconnects.contains(&peer_id) {
                        retries.remove(&peer_id);
                        continue;
                    }
                    let Some(address) = address_book.get(&peer_id).and_then(|addrs| addrs.first()) else {
                        retries.remove(&peer_id);
                        continue;
                    };
                    if let Err(error) = node.dial(address.clone()) {
                        tracing::debug!(%peer_id, %error, "peer redial attempt rejected");
                    }
                    let attempt = retries.get(&peer_id).map_or(0, |retry| retry.attempt + 1);
                    retries.insert(peer_id, RetryState::new(attempt));
                }
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    None => break,
                    Some(MeshCommand::Publish { topic, data }) => {
                        if let Err(e) = node.publish(&topic, &data) {
                            tracing::error!("mesh publish to {topic} failed: {e}");
                        }
                    }
                    Some(MeshCommand::Subscribe { topic }) => {
                        if let Err(e) = node.subscribe(&topic) {
                            tracing::error!("mesh subscribe to {topic} failed: {e}");
                        }
                    }
                    Some(MeshCommand::Dial { addr }) => {
                        remember_address(&db, &mut address_book, &addr);
                        if let Some(peer_id) = neunode_p2p::discovery::peer_id_from_multiaddr(&addr) {
                            manual_disconnects.remove(&peer_id);
                        }
                        if let Err(e) = node.dial(addr) {
                            tracing::error!("mesh dial failed: {e}");
                        }
                    }
                    Some(MeshCommand::Disconnect { peer_id }) => {
                        manual_disconnects.insert(peer_id);
                        retries.remove(&peer_id);
                        let _ = node.disconnect(peer_id);
                    }
                    Some(MeshCommand::GetStatus { reply }) => {
                        let status = MeshStatus {
                            running: true,
                            local_peer_id: node.local_peer_id().to_string(),
                            listeners: node.listeners().map(|a| a.to_string()).collect(),
                            connected_peers: node
                                .connected_peers()
                                .iter()
                                .map(|p| p.to_string())
                                .collect(),
                            subscribed_topics: node
                                .subscribed_topics()
                                .iter()
                                .cloned()
                                .collect(),
                        };
                        let _ = reply.send(status);
                    }
                    Some(MeshCommand::GetPeers { reply }) => {
                        let _ = reply.send(node.connected_peers());
                    }
                    Some(MeshCommand::Shutdown) => break,
                }
            }
            event = node.next_event() => {
                match event {
                    NodeEvent::GossipsubMessage { source, topic, data } => {
                        match crate::feed_wire::deserialize_feed_event(&data) {
                            Ok(feed_event) => {
                                if let Err(e) = feed_event.validate() {
                                    tracing::warn!("Invalid feed event from {:?}: {}", source, e);
                                } else {
                                    let now = chrono::Utc::now().timestamp() as u64;
                                    let author_did = &feed_event.author.0;
                                    if !rate_limiter.allow(author_did, now) {
                                        tracing::warn!(
                                            "Rate limited feed event from {} on {}",
                                            author_did,
                                            topic
                                        );
                                        continue;
                                    }
                                    let stored = crate::feed_wire::feed_event_to_stored(&feed_event);
                                    let store = neunode_storage::feed_store::FeedStore::new(&db);
                                    let event_id = feed_event.id.to_string();
                                    let event_author = feed_event.author.0.clone();
                                    match store.append(&stored) {
                                        Ok(()) => {
                                            tracing::info!(
                                                "Stored feed event {} from {} on {}",
                                                event_id,
                                                event_author,
                                                topic
                                            );
                                            let _ = event_tx.send(feed_event);
                                        }
                                        Err(e) => {
                                            tracing::warn!("Failed to store feed event: {}", e)
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to deserialize feed event from {:?}: {}",
                                    source,
                                    e
                                );
                            }
                        }
                    }
                    NodeEvent::PeerConnected(peer_id) => {
                        retries.remove(&peer_id);
                        manual_disconnects.remove(&peer_id);
                        tracing::info!("Peer connected: {}", peer_id);
                    }
                    NodeEvent::PeerDisconnected(peer_id) => {
                        if !node.is_connected(&peer_id)
                            && !manual_disconnects.contains(&peer_id)
                            && address_book.contains_key(&peer_id)
                        {
                            retries.entry(peer_id).or_insert_with(|| RetryState::new(0));
                        }
                        tracing::info!("Peer disconnected: {}", peer_id);
                    }
                    NodeEvent::IdentifyReceived { peer_id, listen_addresses, .. } => {
                        let addresses = listen_addresses
                            .into_iter()
                            .map(|address| with_peer_id(address, peer_id))
                            .collect::<Vec<_>>();
                        if !addresses.is_empty() {
                            address_book.insert(peer_id, addresses.clone());
                            persist_peer_addresses(&db, peer_id, &addresses);
                        }
                    }
                    NodeEvent::NatStatusChanged(status) => {
                        tracing::info!(?status, "AutoNAT reachability changed");
                    }
                    NodeEvent::HolePunchSucceeded { peer_id } => {
                        tracing::info!(%peer_id, "DCUtR upgraded relayed connection to direct");
                    }
                    NodeEvent::HolePunchFailed { peer_id, error } => {
                        tracing::warn!(%peer_id, %error, "DCUtR direct connection upgrade failed");
                    }
                    _ => {}
                }
            }
        }
    }
}

const MAX_RETRY_DELAY_SECS: u64 = 300;

struct RetryState {
    attempt: u32,
    next_attempt: tokio::time::Instant,
}

impl RetryState {
    fn new(attempt: u32) -> Self {
        Self { attempt, next_attempt: tokio::time::Instant::now() + retry_delay(attempt) }
    }
}

fn retry_delay(attempt: u32) -> Duration {
    Duration::from_secs(
        1_u64
            .checked_shl(attempt.min(63))
            .unwrap_or(MAX_RETRY_DELAY_SECS)
            .min(MAX_RETRY_DELAY_SECS),
    )
}

fn with_peer_id(mut address: Multiaddr, peer_id: PeerId) -> Multiaddr {
    if neunode_p2p::discovery::peer_id_from_multiaddr(&address).is_none() {
        address.push(Protocol::P2p(peer_id));
    }
    address
}

fn remember_address(
    db: &NeunodeDb,
    address_book: &mut HashMap<PeerId, Vec<Multiaddr>>,
    address: &Multiaddr,
) {
    let Some(peer_id) = neunode_p2p::discovery::peer_id_from_multiaddr(address) else { return };
    address_book.insert(peer_id, vec![address.clone()]);
    persist_peer_addresses(db, peer_id, std::slice::from_ref(address));
}

fn persist_peer_addresses(db: &NeunodeDb, peer_id: PeerId, addresses: &[Multiaddr]) {
    let record = PeerAddressRecord {
        peer_id: peer_id.to_string(),
        addresses: addresses.iter().map(ToString::to_string).collect(),
        updated_at: chrono::Utc::now().timestamp().max(0) as u64,
    };
    if let Err(error) = PeerAddressStore::new(db).put(&record) {
        tracing::warn!(%peer_id, %error, "failed to persist peer addresses");
    }
}

fn load_address_book(db: &NeunodeDb) -> HashMap<PeerId, Vec<Multiaddr>> {
    PeerAddressStore::new(db)
        .list()
        .unwrap_or_else(|error| {
            tracing::warn!(%error, "failed to restore peer address book");
            Vec::new()
        })
        .into_iter()
        .filter_map(|record| {
            let peer_id = record.peer_id.parse().ok()?;
            let addresses = record
                .addresses
                .iter()
                .filter_map(|address| address.parse().ok())
                .collect::<Vec<_>>();
            (!addresses.is_empty()).then_some((peer_id, addresses))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_db() -> NeunodeDb {
        static TEST_ID: AtomicU64 = AtomicU64::new(0);
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("mesh_test_{}_{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        NeunodeDb::open(&dir).unwrap()
    }

    fn test_listen_addr() -> Multiaddr {
        "/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap()
    }

    fn spawn_test_node() -> MeshHandle {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let db = Arc::new(temp_db());
        let data_dir = std::env::temp_dir().join(format!(
            "mesh_node_data_{}_{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        spawn_mesh_task(keypair, test_listen_addr(), vec![], false, db, data_dir).unwrap()
    }

    #[tokio::test]
    async fn spawn_mesh_task_creates_handle() {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let expected_peer_id = keypair.public().to_peer_id();
        let db = Arc::new(temp_db());
        let data_dir = std::env::temp_dir().join(format!(
            "mesh_handle_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let handle =
            spawn_mesh_task(keypair, test_listen_addr(), vec![], false, db, data_dir).unwrap();

        assert_eq!(handle.local_peer_id, expected_peer_id);

        handle.shutdown().unwrap();
        let _ = handle.join_handle.await;
    }

    #[tokio::test]
    async fn shutdown_terminates_task() {
        let handle = spawn_test_node();
        handle.shutdown().unwrap();
        let result = handle.join_handle.await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn status_query_returns_data() {
        let handle = spawn_test_node();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let status = handle.status().await.unwrap();
        assert!(status.running);
        assert!(!status.local_peer_id.is_empty());
        assert_eq!(status.connected_peers.len(), 0);

        handle.shutdown().unwrap();
        let _ = handle.join_handle.await;
    }

    #[tokio::test]
    async fn publish_command_no_error() {
        let handle = spawn_test_node();
        let result = handle.publish("neunode/bounty", b"hello world");
        assert!(result.is_ok());

        handle.shutdown().unwrap();
        let _ = handle.join_handle.await;
    }

    #[tokio::test]
    async fn subscribe_command_no_error() {
        let handle = spawn_test_node();
        let result = handle.subscribe("neunode/bounty");
        assert!(result.is_ok());

        handle.shutdown().unwrap();
        let _ = handle.join_handle.await;
    }

    #[tokio::test]
    async fn peers_query_returns_empty_initially() {
        let handle = spawn_test_node();
        let peers = handle.peers().await.unwrap();
        assert!(peers.is_empty());

        handle.shutdown().unwrap();
        let _ = handle.join_handle.await;
    }

    #[tokio::test]
    async fn dial_command_no_error() {
        let handle = spawn_test_node();
        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/9999".parse().unwrap();
        // dial will fail to connect (nothing listening) but the command should send OK
        let result = handle.dial(addr);
        assert!(result.is_ok());

        handle.shutdown().unwrap();
        let _ = handle.join_handle.await;
    }

    #[test]
    fn retry_delay_is_exponential_and_capped() {
        assert_eq!(retry_delay(0), Duration::from_secs(1));
        assert_eq!(retry_delay(1), Duration::from_secs(2));
        assert_eq!(retry_delay(8), Duration::from_secs(256));
        assert_eq!(retry_delay(9), Duration::from_secs(300));
        assert_eq!(retry_delay(u32::MAX), Duration::from_secs(300));
    }

    #[test]
    fn address_book_survives_database_restart() {
        let db = temp_db();
        let peer_id = libp2p::identity::Keypair::generate_ed25519().public().to_peer_id();
        let address: Multiaddr = format!("/ip4/127.0.0.1/tcp/4001/p2p/{peer_id}").parse().unwrap();
        let mut address_book = HashMap::new();
        remember_address(&db, &mut address_book, &address);

        let restored = load_address_book(&db);
        assert_eq!(restored.get(&peer_id), Some(&vec![address]));
    }

    #[test]
    fn identify_address_is_normalized_with_peer_id() {
        let peer_id = libp2p::identity::Keypair::generate_ed25519().public().to_peer_id();
        let address: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        let normalized = with_peer_id(address, peer_id);
        assert_eq!(neunode_p2p::discovery::peer_id_from_multiaddr(&normalized), Some(peer_id));
    }
}
