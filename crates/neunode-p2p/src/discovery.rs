use std::str::FromStr;
use std::time::Duration;

use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId as LpPeerId};
use neunode_core::constants::p2p;
use serde::{Deserialize, Serialize};

use crate::error::{P2pError, Result};

const DEFAULT_DISCOVERY_INTERVAL_SECS: u64 = 300;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DiscoveryConfig {
    pub bootstrap_peers: Vec<String>,
    pub discovery_interval_secs: u64,
    pub max_peers: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            bootstrap_peers: Vec::new(),
            discovery_interval_secs: DEFAULT_DISCOVERY_INTERVAL_SECS,
            max_peers: p2p::MAX_PEER_CONNECTIONS,
        }
    }
}

impl DiscoveryConfig {
    pub fn discovery_interval(&self) -> Duration {
        Duration::from_secs(self.discovery_interval_secs)
    }

    pub fn bootstrap_timeout(&self) -> Duration {
        Duration::from_secs(p2p::BOOTSTRAP_TIMEOUT_SECS)
    }
}

pub fn parse_multiaddr(addr: &str) -> Result<Multiaddr> {
    Multiaddr::from_str(addr)
        .map_err(|e| P2pError::InvalidAddress(format!("failed to parse multiaddr '{addr}': {e}")))
}

pub fn bootstrap_peers_from_config(config: &DiscoveryConfig) -> Vec<Multiaddr> {
    config.bootstrap_peers.iter().filter_map(|addr| parse_multiaddr(addr).ok()).collect()
}

pub fn peer_id_from_multiaddr(addr: &Multiaddr) -> Option<LpPeerId> {
    addr.iter().find_map(|proto| match proto {
        Protocol::P2p(peer_id) => Some(peer_id),
        _ => None,
    })
}

pub fn multiaddr_to_core_peer_id(addr: &Multiaddr) -> Option<neunode_core::types::PeerId> {
    peer_id_from_multiaddr(addr).map(|lp_id| neunode_core::types::PeerId(lp_id.to_string()))
}

pub fn core_peer_id_to_libp2p(peer_id: &neunode_core::types::PeerId) -> Result<LpPeerId> {
    LpPeerId::from_str(peer_id.as_str()).map_err(|e| {
        P2pError::PeerNotFound(format!("invalid libp2p PeerId '{}': {e}", peer_id.as_str()))
    })
}

pub fn libp2p_to_core_peer_id(peer_id: &LpPeerId) -> neunode_core::types::PeerId {
    neunode_core::types::PeerId(peer_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_config_default() {
        let cfg = DiscoveryConfig::default();
        assert!(cfg.bootstrap_peers.is_empty());
        assert_eq!(cfg.discovery_interval_secs, 300);
        assert_eq!(cfg.max_peers, 100);
    }

    #[test]
    fn discovery_config_intervals() {
        let cfg = DiscoveryConfig::default();
        assert_eq!(cfg.discovery_interval(), Duration::from_secs(300));
        assert_eq!(cfg.bootstrap_timeout(), Duration::from_secs(30));
    }

    #[test]
    fn parse_valid_tcp_multiaddr() {
        let addr = parse_multiaddr("/ip4/127.0.0.1/tcp/4001").unwrap();
        assert_eq!(addr.to_string(), "/ip4/127.0.0.1/tcp/4001");
    }

    #[test]
    fn parse_valid_tcp_multiaddr_with_peer() {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();
        let addr_str = format!("/ip4/127.0.0.1/tcp/4001/p2p/{peer_id}");
        let addr = parse_multiaddr(&addr_str).unwrap();
        let components: Vec<_> = addr.iter().collect();
        assert!(matches!(components[0], Protocol::Ip4(_)));
        assert!(matches!(components[1], Protocol::Tcp(4001)));
    }

    #[test]
    fn parse_valid_quic_multiaddr() {
        let addr = parse_multiaddr("/ip4/127.0.0.1/udp/4001/quic-v1").unwrap();
        assert!(addr.to_string().contains("quic-v1"));
    }

    #[test]
    fn parse_invalid_multiaddr_returns_error() {
        let result = parse_multiaddr("not-a-multiaddr");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), P2pError::InvalidAddress(_)));
    }

    #[test]
    fn parse_garbage_returns_error() {
        let result = parse_multiaddr("garbage::///not-valid");
        assert!(result.is_err());
    }

    #[test]
    fn bootstrap_peers_from_empty_config() {
        let cfg = DiscoveryConfig::default();
        let peers = bootstrap_peers_from_config(&cfg);
        assert!(peers.is_empty());
    }

    #[test]
    fn bootstrap_peers_from_config_filters_invalid() {
        let cfg = DiscoveryConfig {
            bootstrap_peers: vec![
                "/ip4/127.0.0.1/tcp/4001".to_string(),
                "invalid-addr".to_string(),
                "/ip4/1.2.3.4/tcp/4001".to_string(),
            ],
            ..Default::default()
        };
        let peers = bootstrap_peers_from_config(&cfg);
        assert_eq!(peers.len(), 2);
    }

    #[test]
    fn peer_id_from_multiaddr_with_p2p_component() {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();
        let addr = format!("/ip4/127.0.0.1/tcp/4001/p2p/{peer_id}");
        let multiaddr = parse_multiaddr(&addr).unwrap();
        let extracted = peer_id_from_multiaddr(&multiaddr);
        assert_eq!(extracted, Some(peer_id));
    }

    #[test]
    fn peer_id_from_multiaddr_without_p2p_returns_none() {
        let addr = parse_multiaddr("/ip4/127.0.0.1/tcp/4001").unwrap();
        let result = peer_id_from_multiaddr(&addr);
        assert!(result.is_none());
    }

    #[test]
    fn core_to_libp2p_roundtrip() {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let lp_id = keypair.public().to_peer_id();
        let core_id = libp2p_to_core_peer_id(&lp_id);
        let back = core_peer_id_to_libp2p(&core_id).unwrap();
        assert_eq!(lp_id, back);
    }

    #[test]
    fn core_peer_id_to_libp2p_invalid_returns_error() {
        let bad = neunode_core::types::PeerId("not-a-peer-id".to_string());
        assert!(core_peer_id_to_libp2p(&bad).is_err());
    }

    #[test]
    fn multiaddr_to_core_peer_id_with_p2p() {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();
        let addr = format!("/ip4/127.0.0.1/tcp/4001/p2p/{peer_id}");
        let multiaddr = parse_multiaddr(&addr).unwrap();
        let core_id = multiaddr_to_core_peer_id(&multiaddr);
        assert!(core_id.is_some());
        assert_eq!(core_id.unwrap().as_str(), peer_id.to_string());
    }

    #[test]
    fn discovery_config_serde_roundtrip() {
        let cfg = DiscoveryConfig {
            bootstrap_peers: vec!["/ip4/1.2.3.4/tcp/4001".to_string()],
            discovery_interval_secs: 60,
            max_peers: 50,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: DiscoveryConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }
}
