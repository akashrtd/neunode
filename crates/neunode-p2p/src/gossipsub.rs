use std::time::Duration;

use libp2p::gossipsub::{ConfigBuilder, IdentTopic, MessageId, ValidationMode};
use neunode_core::constants::p2p;
use neunode_core::kind::Kind;
use serde::{Deserialize, Serialize};

use crate::error::P2pError;
use crate::error::Result;
use ts_rs::TS;

const MAX_MESSAGE_SIZE: usize = 1024 * 1024;
const HEARTBEAT_INTERVAL_SECS: u64 = 300;
const FANOUT_TTL_SECS: u64 = 60;
const HISTORY_LENGTH: usize = 10;
const HISTORY_GOSSIP_LENGTH: usize = 10;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[ts(export)]
pub struct GossipsubConfig {
    pub max_message_size: usize,
    pub mesh_degree: usize,
    pub mesh_degree_low: usize,
    pub mesh_degree_high: usize,
    pub gossip_factor: f64,
    pub heartbeat_interval_secs: u64,
    pub fanout_ttl_secs: u64,
    pub history_length: usize,
    pub history_gossip_length: usize,
}

impl Default for GossipsubConfig {
    fn default() -> Self {
        Self {
            max_message_size: MAX_MESSAGE_SIZE,
            mesh_degree: p2p::MESH_DEGREE,
            mesh_degree_low: p2p::MESH_DEGREE_LOW,
            mesh_degree_high: p2p::MESH_DEGREE_HIGH,
            gossip_factor: p2p::GOSSIP_FACTOR,
            heartbeat_interval_secs: HEARTBEAT_INTERVAL_SECS,
            fanout_ttl_secs: FANOUT_TTL_SECS,
            history_length: HISTORY_LENGTH,
            history_gossip_length: HISTORY_GOSSIP_LENGTH,
        }
    }
}

pub fn create_gossipsub_config() -> Result<libp2p::gossipsub::Config> {
    let our_config = GossipsubConfig::default();
    create_gossipsub_config_from(&our_config)
}

pub fn create_gossipsub_config_from(cfg: &GossipsubConfig) -> Result<libp2p::gossipsub::Config> {
    ConfigBuilder::default()
        .max_transmit_size(cfg.max_message_size)
        .mesh_n(cfg.mesh_degree)
        .mesh_n_low(cfg.mesh_degree_low)
        .mesh_n_high(cfg.mesh_degree_high)
        .gossip_factor(cfg.gossip_factor)
        .heartbeat_interval(Duration::from_secs(cfg.heartbeat_interval_secs))
        .fanout_ttl(Duration::from_secs(cfg.fanout_ttl_secs))
        .history_length(cfg.history_length)
        .history_gossip(cfg.history_gossip_length)
        .validation_mode(ValidationMode::Strict)
        .message_id_fn(|msg: &libp2p::gossipsub::Message| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            if let Some(source) = &msg.source {
                std::hash::Hash::hash_slice(&source.to_bytes(), &mut hasher);
            }
            std::hash::Hash::hash_slice(&msg.data, &mut hasher);
            MessageId::from(std::hash::Hasher::finish(&hasher).to_string())
        })
        .build()
        .map_err(|e| P2pError::ConfigError(format!("gossipsub config build failed: {e}")))
}

pub fn topic_for_kind(kind: &Kind) -> IdentTopic {
    IdentTopic::new(kind.gossipsub_topic())
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[ts(export)]
pub struct FeedMessage {
    pub kind: u16,
    pub payload: Vec<u8>,
    pub timestamp: i64,
    pub signature: Vec<u8>,
}

impl FeedMessage {
    pub fn new(kind: Kind, payload: Vec<u8>, timestamp: i64, signature: Vec<u8>) -> Self {
        Self { kind: kind.as_u16(), payload, timestamp, signature }
    }

    pub fn kind(&self) -> std::result::Result<Kind, neunode_core::error::NeunodeError> {
        Kind::from_u16(self.kind)
    }

    pub fn signable_payload(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8 + self.payload.len() + 8);
        buf.extend_from_slice(&self.kind.to_be_bytes());
        buf.extend_from_slice(&self.payload);
        buf.extend_from_slice(&self.timestamp.to_be_bytes());
        buf
    }
}

pub fn validate_feed_message(msg: &FeedMessage) -> Result<()> {
    if msg.payload.is_empty() {
        return Err(P2pError::PublishFailed("payload is empty".to_string()));
    }
    if msg.payload.len() > MAX_MESSAGE_SIZE {
        return Err(P2pError::PublishFailed(format!(
            "payload size {} exceeds max {}",
            msg.payload.len(),
            MAX_MESSAGE_SIZE
        )));
    }
    if msg.signature.is_empty() {
        return Err(P2pError::PublishFailed("signature is empty".to_string()));
    }
    if msg.timestamp <= 0 {
        return Err(P2pError::PublishFailed("timestamp must be positive".to_string()));
    }
    Kind::from_u16(msg.kind).map_err(|e| P2pError::PublishFailed(format!("invalid kind: {e}")))?;
    Ok(())
}

pub fn all_category_topics() -> Vec<IdentTopic> {
    vec![
        IdentTopic::new("neunode/system"),
        IdentTopic::new("neunode/bounty"),
        IdentTopic::new("neunode/training"),
        IdentTopic::new("neunode/attestation"),
        IdentTopic::new("neunode/inference"),
        IdentTopic::new("neunode/governance"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = GossipsubConfig::default();
        assert_eq!(cfg.max_message_size, 1024 * 1024);
        assert_eq!(cfg.mesh_degree, 6);
        assert_eq!(cfg.mesh_degree_low, 4);
        assert_eq!(cfg.mesh_degree_high, 12);
        assert_eq!(cfg.gossip_factor, 0.25);
        assert_eq!(cfg.heartbeat_interval_secs, 300);
        assert_eq!(cfg.fanout_ttl_secs, 60);
        assert_eq!(cfg.history_length, 10);
        assert_eq!(cfg.history_gossip_length, 10);
    }

    #[test]
    fn create_default_gossipsub_config_succeeds() {
        let config = create_gossipsub_config();
        let _ = config;
    }

    #[test]
    fn create_custom_gossipsub_config_succeeds() {
        let custom = GossipsubConfig {
            max_message_size: 2048,
            mesh_degree: 8,
            mesh_degree_low: 5,
            mesh_degree_high: 15,
            gossip_factor: 0.3,
            heartbeat_interval_secs: 60,
            fanout_ttl_secs: 30,
            history_length: 5,
            history_gossip_length: 5,
        };
        let config = create_gossipsub_config_from(&custom);
        let _ = config;
    }

    #[test]
    fn topic_for_system_kind() {
        let topic = topic_for_kind(&Kind::AgentMetadata);
        assert_eq!(topic.to_string(), "neunode/system");
    }

    #[test]
    fn topic_for_bounty_kind() {
        let topic = topic_for_kind(&Kind::BountyPost);
        assert_eq!(topic.to_string(), "neunode/bounty");
    }

    #[test]
    fn topic_for_training_kind() {
        let topic = topic_for_kind(&Kind::JobSubmit);
        assert_eq!(topic.to_string(), "neunode/training");
    }

    #[test]
    fn topic_for_attestation_kind() {
        let topic = topic_for_kind(&Kind::Attest);
        assert_eq!(topic.to_string(), "neunode/attestation");
    }

    #[test]
    fn topic_for_inference_kind() {
        let topic = topic_for_kind(&Kind::ModelAnnounce);
        assert_eq!(topic.to_string(), "neunode/inference");
    }

    #[test]
    fn topic_for_governance_kind() {
        let topic = topic_for_kind(&Kind::Proposal);
        assert_eq!(topic.to_string(), "neunode/governance");
    }

    #[test]
    fn all_category_topics_returns_six() {
        let topics = all_category_topics();
        assert_eq!(topics.len(), 6);
        let names: Vec<String> = topics.iter().map(|t| t.to_string()).collect();
        assert!(names.contains(&"neunode/system".to_string()));
        assert!(names.contains(&"neunode/bounty".to_string()));
        assert!(names.contains(&"neunode/training".to_string()));
        assert!(names.contains(&"neunode/attestation".to_string()));
        assert!(names.contains(&"neunode/inference".to_string()));
        assert!(names.contains(&"neunode/governance".to_string()));
    }

    #[test]
    fn feed_message_new() {
        let msg = FeedMessage::new(Kind::BountyPost, vec![1, 2, 3], 1700000000, vec![4, 5, 6]);
        assert_eq!(msg.kind, 1000);
        assert_eq!(msg.payload, vec![1, 2, 3]);
        assert_eq!(msg.timestamp, 1700000000);
        assert_eq!(msg.signature, vec![4, 5, 6]);
    }

    #[test]
    fn feed_message_kind_roundtrip() {
        let msg = FeedMessage::new(Kind::BountyPost, vec![1], 100, vec![2]);
        assert_eq!(msg.kind().unwrap(), Kind::BountyPost);
    }

    #[test]
    fn feed_message_signable_payload() {
        let msg = FeedMessage::new(Kind::BountyPost, vec![1, 2], 100, vec![3]);
        let payload = msg.signable_payload();
        assert!(!payload.is_empty());
        assert!(payload.len() >= 10);
    }

    #[test]
    fn validate_valid_feed_message() {
        let msg = FeedMessage::new(Kind::BountyPost, vec![1, 2, 3], 1700000000, vec![4, 5]);
        assert!(validate_feed_message(&msg).is_ok());
    }

    #[test]
    fn validate_rejects_empty_payload() {
        let msg = FeedMessage::new(Kind::BountyPost, vec![], 1700000000, vec![4, 5]);
        let err = validate_feed_message(&msg).unwrap_err();
        assert!(err.to_string().contains("payload is empty"));
    }

    #[test]
    fn validate_rejects_oversized_payload() {
        let msg = FeedMessage::new(
            Kind::BountyPost,
            vec![0u8; MAX_MESSAGE_SIZE + 1],
            1700000000,
            vec![4, 5],
        );
        let err = validate_feed_message(&msg).unwrap_err();
        assert!(err.to_string().contains("exceeds max"));
    }

    #[test]
    fn validate_rejects_empty_signature() {
        let msg = FeedMessage::new(Kind::BountyPost, vec![1], 1700000000, vec![]);
        let err = validate_feed_message(&msg).unwrap_err();
        assert!(err.to_string().contains("signature is empty"));
    }

    #[test]
    fn validate_rejects_nonpositive_timestamp() {
        let msg = FeedMessage::new(Kind::BountyPost, vec![1], 0, vec![2]);
        let err = validate_feed_message(&msg).unwrap_err();
        assert!(err.to_string().contains("timestamp must be positive"));
    }

    #[test]
    fn validate_rejects_invalid_kind() {
        let msg = FeedMessage::new(Kind::BountyPost, vec![1], 100, vec![2]);
        let mut bad_msg = msg.clone();
        bad_msg.kind = 9999;
        let err = validate_feed_message(&bad_msg).unwrap_err();
        assert!(err.to_string().contains("invalid kind"));
    }

    #[test]
    fn feed_message_serde_roundtrip() {
        let msg = FeedMessage::new(Kind::BountyPost, vec![1, 2, 3], 1700000000, vec![4, 5, 6]);
        let json = serde_json::to_string(&msg).unwrap();
        let back: FeedMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, back);
    }

    #[test]
    fn gossipsub_config_serde_roundtrip() {
        let cfg = GossipsubConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: GossipsubConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }
}
