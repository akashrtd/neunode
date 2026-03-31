use std::collections::HashSet;

use libp2p::futures::StreamExt;
use libp2p::gossipsub::IdentTopic;
use libp2p::identity::Keypair;
use libp2p::swarm::dial_opts::DialOpts;
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, PeerId, Swarm};

use crate::behaviour::{build_behaviour, NeunodeBehaviour, NeunodeEvent};
use crate::error::{P2pError, Result};
use crate::gossipsub::all_category_topics;

pub struct P2pNode {
    swarm: Swarm<NeunodeBehaviour>,
    subscribed_topics: HashSet<String>,
}

impl P2pNode {
    pub fn new(keypair: Keypair, _listen_addr: Multiaddr) -> Result<Self> {
        let peer_id = keypair.public().to_peer_id();

        let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair.clone())
            .with_tokio()
            .with_tcp(
                Default::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .map_err(|e| P2pError::ConnectionFailed(format!("tcp setup failed: {e}")))?
            .with_quic()
            .with_relay_client(libp2p::noise::Config::new, libp2p::yamux::Config::default)
            .map_err(|e| P2pError::ConnectionFailed(format!("relay client failed: {e}")))?
            .with_behaviour(move |_, relay_behaviour| {
                let mut inner = build_behaviour(&keypair, peer_id)?;
                inner.relay_client = relay_behaviour;
                Ok(inner)
            })
            .map_err(|e| P2pError::ConnectionFailed(format!("behaviour setup failed: {e}")))?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(std::time::Duration::from_secs(60))
            })
            .build();

        Ok(Self { swarm, subscribed_topics: HashSet::new() })
    }

    pub fn start(&mut self, listen_addr: Multiaddr) -> Result<()> {
        self.swarm
            .listen_on(listen_addr)
            .map_err(|e| P2pError::ConnectionFailed(format!("listen failed: {e}")))?;
        Ok(())
    }

    pub fn dial(&mut self, addr: Multiaddr) -> Result<()> {
        self.swarm.dial(addr).map_err(|e| P2pError::DialFailed(format!("dial failed: {e}")))?;
        Ok(())
    }

    pub fn dial_peer(&mut self, peer_id: PeerId) -> Result<()> {
        let opts = DialOpts::peer_id(peer_id).build();
        self.swarm
            .dial(opts)
            .map_err(|e| P2pError::DialFailed(format!("dial peer failed: {e}")))?;
        Ok(())
    }

    pub fn publish(&mut self, topic: &str, data: &[u8]) -> Result<()> {
        let topic = IdentTopic::new(topic);
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, data.to_vec())
            .map_err(|e| P2pError::PublishFailed(format!("publish failed: {e}")))?;
        Ok(())
    }

    pub fn subscribe(&mut self, topic: &str) -> Result<()> {
        if self.subscribed_topics.contains(topic) {
            return Ok(());
        }
        let topic = IdentTopic::new(topic);
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic)
            .map_err(|e| P2pError::SubscriptionFailed(format!("subscribe failed: {e}")))?;
        self.subscribed_topics.insert(topic.to_string());
        Ok(())
    }

    pub fn unsubscribe(&mut self, topic: &str) -> Result<()> {
        if !self.subscribed_topics.contains(topic) {
            return Ok(());
        }
        let topic = IdentTopic::new(topic);
        let was_subscribed = self.swarm.behaviour_mut().gossipsub.unsubscribe(&topic);
        if !was_subscribed {
            return Err(P2pError::SubscriptionFailed("unsubscribe returned false".to_string()));
        }
        let topic_str = topic.to_string();
        self.subscribed_topics.remove(&topic_str);
        Ok(())
    }

    pub fn subscribe_all_categories(&mut self) -> Result<()> {
        for topic in all_category_topics() {
            self.swarm.behaviour_mut().gossipsub.subscribe(&topic).map_err(|e| {
                P2pError::SubscriptionFailed(format!("subscribe to {} failed: {e}", topic))
            })?;
            self.subscribed_topics.insert(topic.to_string());
        }
        Ok(())
    }

    pub fn local_peer_id(&self) -> PeerId {
        *self.swarm.local_peer_id()
    }

    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.swarm.connected_peers().copied().collect()
    }

    pub fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.swarm.is_connected(peer_id)
    }

    pub fn disconnect(&mut self, peer_id: PeerId) -> Result<()> {
        let _ = self.swarm.disconnect_peer_id(peer_id);
        Ok(())
    }

    pub fn subscribed_topics(&self) -> &HashSet<String> {
        &self.subscribed_topics
    }

    pub fn add_bootstrap_peer(&mut self, addr: Multiaddr) {
        let peer_id =
            crate::discovery::peer_id_from_multiaddr(&addr).unwrap_or_else(|| self.local_peer_id());
        self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
    }

    pub fn bootstrap_dht(&mut self) -> Result<()> {
        self.swarm
            .behaviour_mut()
            .kademlia
            .bootstrap()
            .map_err(|e| P2pError::DhtError(format!("bootstrap failed: {e:?}")))?;
        Ok(())
    }

    pub fn listeners(&self) -> impl Iterator<Item = &Multiaddr> {
        self.swarm.listeners()
    }
}

#[derive(Debug)]
pub enum NodeEvent {
    GossipsubMessage { source: Option<PeerId>, topic: String, data: Vec<u8> },
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    IdentifyReceived { peer_id: PeerId, agent_version: String },
    PingResult { peer_id: PeerId, rtt_ms: u64 },
    KademliaEvent(libp2p::kad::Event),
}

impl P2pNode {
    pub async fn next_event(&mut self) -> NodeEvent {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(NeunodeEvent::Gossipsub(gs_event)) => {
                    if let Some(node_event) = convert_gossipsub_event(gs_event) {
                        return node_event;
                    }
                }
                SwarmEvent::Behaviour(NeunodeEvent::Identify(id_event)) => {
                    if let Some(node_event) = convert_identify_event(*id_event) {
                        return node_event;
                    }
                }
                SwarmEvent::Behaviour(NeunodeEvent::Ping(ping_event)) => {
                    if let Some(node_event) = convert_ping_event(ping_event) {
                        return node_event;
                    }
                }
                SwarmEvent::Behaviour(NeunodeEvent::Kademlia(kad_event)) => {
                    return NodeEvent::KademliaEvent(kad_event);
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    return NodeEvent::PeerConnected(peer_id);
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    return NodeEvent::PeerDisconnected(peer_id);
                }
                _ => {}
            }
        }
    }
}

fn convert_gossipsub_event(event: libp2p::gossipsub::Event) -> Option<NodeEvent> {
    match event {
        libp2p::gossipsub::Event::Message { propagation_source, message, .. } => {
            Some(NodeEvent::GossipsubMessage {
                source: Some(propagation_source),
                topic: message.topic.to_string(),
                data: message.data,
            })
        }
        _ => None,
    }
}

fn convert_identify_event(event: libp2p::identify::Event) -> Option<NodeEvent> {
    match event {
        libp2p::identify::Event::Received { peer_id, info, .. } => {
            Some(NodeEvent::IdentifyReceived { peer_id, agent_version: info.agent_version })
        }
        _ => None,
    }
}

fn convert_ping_event(event: libp2p::ping::Event) -> Option<NodeEvent> {
    match event.result {
        Ok(rtt) => {
            Some(NodeEvent::PingResult { peer_id: event.peer, rtt_ms: rtt.as_millis() as u64 })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_node() -> P2pNode {
        let keypair = Keypair::generate_ed25519();
        let listen_addr = "/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap();
        P2pNode::new(keypair, listen_addr).expect("node creation should succeed")
    }

    #[test]
    fn node_construction_succeeds() {
        let node = create_test_node();
        assert!(!node.local_peer_id().to_string().is_empty());
    }

    #[test]
    fn local_peer_id_matches_keypair() {
        let keypair = Keypair::generate_ed25519();
        let expected = keypair.public().to_peer_id();
        let listen_addr = "/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap();
        let node = P2pNode::new(keypair, listen_addr).unwrap();
        assert_eq!(node.local_peer_id(), expected);
    }

    #[test]
    fn connected_peers_initially_empty() {
        let node = create_test_node();
        assert!(node.connected_peers().is_empty());
    }

    #[test]
    fn subscribed_topics_initially_empty() {
        let node = create_test_node();
        assert!(node.subscribed_topics().is_empty());
    }

    #[tokio::test]
    async fn start_listening() {
        let mut node = create_test_node();
        let addr = "/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap();
        node.start(addr).expect("start should succeed");
    }

    #[tokio::test]
    async fn subscribe_to_topic() {
        let mut node = create_test_node();
        let addr = "/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap();
        node.start(addr).unwrap();
        node.subscribe("neunode/bounty").expect("subscribe should succeed");
        assert!(node.subscribed_topics().contains("neunode/bounty"));
    }

    #[tokio::test]
    async fn unsubscribe_from_topic() {
        let mut node = create_test_node();
        let addr = "/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap();
        node.start(addr).unwrap();
        node.subscribe("neunode/bounty").unwrap();
        node.unsubscribe("neunode/bounty").expect("unsubscribe should succeed");
        assert!(!node.subscribed_topics().contains("neunode/bounty"));
    }

    #[tokio::test]
    async fn subscribe_idempotent() {
        let mut node = create_test_node();
        let addr = "/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap();
        node.start(addr).unwrap();
        node.subscribe("neunode/bounty").unwrap();
        node.subscribe("neunode/bounty").unwrap();
        assert_eq!(node.subscribed_topics().len(), 1);
    }

    #[tokio::test]
    async fn subscribe_all_categories() {
        let mut node = create_test_node();
        let addr = "/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap();
        node.start(addr).unwrap();
        node.subscribe_all_categories().expect("subscribe all should succeed");
        assert_eq!(node.subscribed_topics().len(), 6);
    }

    #[test]
    fn node_listeners_initially_empty() {
        let node = create_test_node();
        assert_eq!(node.listeners().count(), 0);
    }

    #[test]
    fn is_connected_returns_false_for_unknown_peer() {
        let node = create_test_node();
        let random_peer = Keypair::generate_ed25519().public().to_peer_id();
        assert!(!node.is_connected(&random_peer));
    }
}
