use std::path::Path;

use libp2p::gossipsub;
use libp2p::identity::Keypair;
use libp2p::ping;
use libp2p::relay::client;
use libp2p::swarm::NetworkBehaviour;
use libp2p::PeerId;

use crate::dht_store::SharedRocksStore;
use crate::error::{P2pError, Result};
use crate::gossipsub::create_gossipsub_config;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "NeunodeEvent")]
pub struct NeunodeBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: libp2p::kad::Behaviour<SharedRocksStore>,
    pub identify: libp2p::identify::Behaviour,
    pub ping: ping::Behaviour,
    pub relay_client: client::Behaviour,
    pub autonat: libp2p::autonat::Behaviour,
    pub dcutr: libp2p::dcutr::Behaviour,
}

#[derive(Debug)]
pub enum NeunodeEvent {
    Gossipsub(gossipsub::Event),
    Kademlia(libp2p::kad::Event),
    Identify(Box<libp2p::identify::Event>),
    Ping(ping::Event),
    RelayClient(client::Event),
    Autonat(libp2p::autonat::Event),
    Dcutr(libp2p::dcutr::Event),
}

impl From<gossipsub::Event> for NeunodeEvent {
    fn from(event: gossipsub::Event) -> Self {
        NeunodeEvent::Gossipsub(event)
    }
}

impl From<libp2p::kad::Event> for NeunodeEvent {
    fn from(event: libp2p::kad::Event) -> Self {
        NeunodeEvent::Kademlia(event)
    }
}

impl From<libp2p::identify::Event> for NeunodeEvent {
    fn from(event: libp2p::identify::Event) -> Self {
        NeunodeEvent::Identify(Box::new(event))
    }
}

impl From<ping::Event> for NeunodeEvent {
    fn from(event: ping::Event) -> Self {
        NeunodeEvent::Ping(event)
    }
}

impl From<client::Event> for NeunodeEvent {
    fn from(event: client::Event) -> Self {
        NeunodeEvent::RelayClient(event)
    }
}

impl From<libp2p::autonat::Event> for NeunodeEvent {
    fn from(event: libp2p::autonat::Event) -> Self {
        NeunodeEvent::Autonat(event)
    }
}

impl From<libp2p::dcutr::Event> for NeunodeEvent {
    fn from(event: libp2p::dcutr::Event) -> Self {
        NeunodeEvent::Dcutr(event)
    }
}

pub fn build_behaviour(
    keypair: &Keypair,
    local_peer_id: PeerId,
    data_dir: &Path,
) -> Result<NeunodeBehaviour> {
    let gs_config = create_gossipsub_config()?;
    let message_authenticity = gossipsub::MessageAuthenticity::Signed(keypair.clone());
    let gossipsub = gossipsub::Behaviour::new(message_authenticity, gs_config)
        .map_err(|e| P2pError::ConnectionFailed(e.to_string()))?;

    let dht_path = data_dir.join("dht");
    let store = SharedRocksStore::open(&dht_path)
        .map_err(|e| P2pError::ConfigError(format!("failed to open DHT store: {e}")))?;
    let kad_config = libp2p::kad::Config::new(
        libp2p::StreamProtocol::try_from_owned(
            neunode_core::constants::p2p::DHT_PROTOCOL.to_string(),
        )
        .map_err(|e| P2pError::ConfigError(format!("invalid DHT protocol name: {e}")))?,
    );
    let kademlia = libp2p::kad::Behaviour::with_config(local_peer_id, store, kad_config);

    let identify = libp2p::identify::Behaviour::new(
        libp2p::identify::Config::new(
            neunode_core::constants::p2p::IDENTIFY_PROTOCOL.to_string(),
            keypair.public(),
        )
        .with_push_listen_addr_updates(true),
    );

    let ping = ping::Behaviour::new(ping::Config::new());

    let (_relay_transport, relay_client) = client::new(local_peer_id);

    let autonat =
        libp2p::autonat::Behaviour::new(local_peer_id, libp2p::autonat::Config::default());

    let dcutr = libp2p::dcutr::Behaviour::new(local_peer_id);

    Ok(NeunodeBehaviour { gossipsub, kademlia, identify, ping, relay_client, autonat, dcutr })
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::kad::store::RecordStore;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_data_dir() -> std::path::PathBuf {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "neunode_behaviour_test_{:?}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn build_behaviour_succeeds() {
        let keypair = Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();
        let dir = temp_data_dir();
        let behaviour = build_behaviour(&keypair, peer_id, &dir);
        assert!(behaviour.is_ok());
    }

    #[test]
    fn behaviour_exposes_all_sub_behaviours() {
        let keypair = Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();
        let dir = temp_data_dir();
        let behaviour = build_behaviour(&keypair, peer_id, &dir).expect("construction succeeds");
        let _ = &behaviour.gossipsub;
        let _ = &behaviour.kademlia;
        let _ = &behaviour.identify;
        let _ = &behaviour.ping;
        let _ = &behaviour.relay_client;
        let _ = &behaviour.autonat;
        let _ = &behaviour.dcutr;
    }

    #[test]
    fn event_from_gossipsub() {
        let keypair = Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();
        let topic = gossipsub::IdentTopic::new("test");
        let gs_event = gossipsub::Event::Subscribed { peer_id, topic: topic.hash() };
        let neunode_event: NeunodeEvent = gs_event.into();
        assert!(matches!(neunode_event, NeunodeEvent::Gossipsub(_)));
    }

    #[test]
    fn event_from_ping() {
        let keypair = Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();
        let ping_event = ping::Event {
            peer: peer_id,
            connection: libp2p::swarm::ConnectionId::new_unchecked(1),
            result: Ok(std::time::Duration::from_millis(50)),
        };
        let neunode_event: NeunodeEvent = ping_event.into();
        assert!(matches!(neunode_event, NeunodeEvent::Ping(_)));
    }

    #[test]
    fn dht_store_persists_on_disk() {
        let dir = temp_data_dir();
        let keypair = Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();

        {
            let mut behaviour = build_behaviour(&keypair, peer_id, &dir).unwrap();
            behaviour
                .kademlia
                .store_mut()
                .put(libp2p::kad::Record {
                    key: libp2p::kad::RecordKey::from(b"persist-test".to_vec()),
                    value: b"hello".to_vec(),
                    publisher: None,
                    expires: None,
                })
                .unwrap();
        }

        let mut behaviour = build_behaviour(&keypair, peer_id, &dir).unwrap();
        let got = behaviour
            .kademlia
            .store_mut()
            .get(&libp2p::kad::RecordKey::from(b"persist-test".to_vec()));
        assert!(got.is_some());
        assert_eq!(got.unwrap().value, b"hello");
    }
}
