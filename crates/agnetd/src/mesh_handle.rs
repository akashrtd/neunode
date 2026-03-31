use std::sync::Arc;

use anyhow::Result;
use libp2p::Multiaddr;
use libp2p::PeerId;
use neunode_p2p::node::{NodeEvent, P2pNode};
use neunode_storage::db::NeunodeDb;
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
) -> Result<MeshHandle> {
    let mut node = P2pNode::new(keypair, listen_addr.clone())?;
    node.start(listen_addr)?;

    if subscribe_all {
        node.subscribe_all_categories()?;
    }

    for addr in &bootstrap_peers {
        node.add_bootstrap_peer(addr.clone());
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
    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    None => break,
                    Some(MeshCommand::Publish { topic, data }) => {
                        let _ = node.publish(&topic, &data);
                    }
                    Some(MeshCommand::Subscribe { topic }) => {
                        let _ = node.subscribe(&topic);
                    }
                    Some(MeshCommand::Dial { addr }) => {
                        let _ = node.dial(addr);
                    }
                    Some(MeshCommand::Disconnect { peer_id }) => {
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
                        tracing::info!("Peer connected: {}", peer_id);
                    }
                    NodeEvent::PeerDisconnected(peer_id) => {
                        tracing::info!("Peer disconnected: {}", peer_id);
                    }
                    _ => {}
                }
            }
        }
    }
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
        spawn_mesh_task(keypair, test_listen_addr(), vec![], false, db).unwrap()
    }

    #[tokio::test]
    async fn spawn_mesh_task_creates_handle() {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let expected_peer_id = keypair.public().to_peer_id();
        let db = Arc::new(temp_db());
        let handle = spawn_mesh_task(keypair, test_listen_addr(), vec![], false, db).unwrap();

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
}
