use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How a model was derived from its parent(s).
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ContributionType {
    /// Base model trained from scratch.
    PreTraining,
    /// LoRA fine-tune of a parent model.
    FineTune { lora_rank: u32, lora_alpha: f64 },
    /// Merge of two or more parent models.
    Merge { merge_method: String },
    /// RLHF training with a reward model.
    RL { reward_model_cid: String },
    /// Dataset contribution used in training.
    Data { dataset_hash: String },
    /// Compute contribution for training.
    Compute { duration_secs: f64 },
}

/// A node in the model lineage DAG.
///
/// Represents a single model version with its content hash, parent models,
/// and cryptographic signature linking it to its contributor.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ModelNode {
    /// Content hash (SHA-256 of safetensors file).
    pub cid: String,
    /// Parent model CIDs (empty for base models).
    pub parent_cids: Vec<String>,
    /// DID of the agent who contributed this model.
    pub contributor_did: String,
    /// What kind of contribution this represents.
    pub contribution_type: ContributionType,
    /// Ed25519 signature over canonical payload (64 bytes).
    pub signature: Vec<u8>,
    /// Unix timestamp in milliseconds.
    #[ts(type = "number")]
    pub created_at: u64,
    /// Additional metadata about the training run.
    pub metadata: ModelMetadata,
}

/// Optional metadata about a model's training process.
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export)]
pub struct ModelMetadata {
    /// Hash of the training dataset.
    pub dataset_hash: Option<String>,
    /// Hash of the base model used.
    pub base_model_hash: Option<String>,
    /// Hash of the hyperparameter configuration.
    pub hyperparameter_hash: Option<String>,
    /// Wall-clock training duration in seconds.
    pub training_duration_secs: Option<f64>,
    /// JSON string describing hardware used.
    pub hardware_info: Option<String>,
    /// JSON string for custom extensions.
    pub custom: Option<String>,
}

/// A directed edge in the lineage DAG (child → parent).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LineageEdge {
    /// CID of the child (derived) model.
    pub from_cid: String,
    /// CID of the parent (source) model.
    pub to_cid: String,
    /// What kind of contribution this edge represents.
    pub contribution_type: ContributionType,
}

/// Weight for royalty distribution to a contributor.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RoyaltyWeight {
    /// DID of the contributor.
    pub contributor_did: String,
    /// Normalized weight (0.0–1.0).
    pub weight: f64,
    /// What kind of contribution was made.
    pub contribution_type: ContributionType,
    /// Number of hops from the serving node.
    #[ts(type = "number")]
    pub staleness: u32,
}

/// Returns the protocol weight for a contribution type.
///
/// These weights determine how royalties are distributed
/// across different kinds of model contributions.
pub fn type_weight(ct: &ContributionType) -> f64 {
    match ct {
        ContributionType::PreTraining => 0.30,
        ContributionType::FineTune { .. } => 0.25,
        ContributionType::RL { .. } => 0.20,
        ContributionType::Data { .. } => 0.15,
        ContributionType::Merge { .. } => 0.05,
        ContributionType::Compute { .. } => 0.05,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ts_rs::TS;

    #[test]
    fn contribution_type_serde_pre_training() {
        let ct = ContributionType::PreTraining;
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, "\"pre_training\"");
        let back: ContributionType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ct);
    }

    #[test]
    fn contribution_type_serde_fine_tune() {
        let ct = ContributionType::FineTune { lora_rank: 16, lora_alpha: 32.0 };
        let json = serde_json::to_string(&ct).unwrap();
        assert!(json.contains("\"lora_rank\":16"));
        assert!(json.contains("\"lora_alpha\":32.0"));
        let back: ContributionType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ct);
    }

    #[test]
    fn contribution_type_serde_merge() {
        let ct = ContributionType::Merge { merge_method: "slerp".to_string() };
        let json = serde_json::to_string(&ct).unwrap();
        assert!(json.contains("\"merge_method\":\"slerp\""));
        let back: ContributionType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ct);
    }

    #[test]
    fn contribution_type_serde_rl() {
        let ct = ContributionType::RL { reward_model_cid: "sha256:abc".to_string() };
        let json = serde_json::to_string(&ct).unwrap();
        assert!(json.contains("\"reward_model_cid\":\"sha256:abc\""));
        let back: ContributionType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ct);
    }

    #[test]
    fn contribution_type_serde_data() {
        let ct = ContributionType::Data { dataset_hash: "sha256:def".to_string() };
        let json = serde_json::to_string(&ct).unwrap();
        assert!(json.contains("\"dataset_hash\":\"sha256:def\""));
        let back: ContributionType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ct);
    }

    #[test]
    fn contribution_type_serde_compute() {
        let ct = ContributionType::Compute { duration_secs: 3600.0 };
        let json = serde_json::to_string(&ct).unwrap();
        assert!(json.contains("\"duration_secs\":3600.0"));
        let back: ContributionType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ct);
    }

    #[test]
    fn contribution_type_snake_case() {
        let ct = ContributionType::PreTraining;
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, "\"pre_training\"");
    }

    #[test]
    fn model_node_serde_roundtrip() {
        let node = ModelNode {
            cid: "sha256:abc123".to_string(),
            parent_cids: vec!["sha256:parent1".to_string()],
            contributor_did: "did:neunode:agent1".to_string(),
            contribution_type: ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
            signature: vec![0u8; 64],
            created_at: 1712000000000,
            metadata: ModelMetadata::default(),
        };
        let json = serde_json::to_string(&node).unwrap();
        let back: ModelNode = serde_json::from_str(&json).unwrap();
        assert_eq!(back.cid, node.cid);
        assert_eq!(back.parent_cids, node.parent_cids);
        assert_eq!(back.contributor_did, node.contributor_did);
        assert_eq!(back.signature, node.signature);
        assert_eq!(back.created_at, node.created_at);
    }

    #[test]
    fn model_metadata_default_all_none() {
        let meta = ModelMetadata::default();
        assert!(meta.dataset_hash.is_none());
        assert!(meta.base_model_hash.is_none());
        assert!(meta.hyperparameter_hash.is_none());
        assert!(meta.training_duration_secs.is_none());
        assert!(meta.hardware_info.is_none());
        assert!(meta.custom.is_none());
    }

    #[test]
    fn model_metadata_with_fields() {
        let meta = ModelMetadata {
            dataset_hash: Some("sha256:ds".to_string()),
            base_model_hash: Some("sha256:base".to_string()),
            hyperparameter_hash: Some("sha256:hp".to_string()),
            training_duration_secs: Some(7200.0),
            hardware_info: Some("{\"gpu\":\"H100\"}".to_string()),
            custom: None,
        };
        let json = serde_json::to_string(&meta).unwrap();
        let back: ModelMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back.dataset_hash, meta.dataset_hash);
        assert_eq!(back.hardware_info, meta.hardware_info);
    }

    #[test]
    fn lineage_edge_serde_roundtrip() {
        let edge = LineageEdge {
            from_cid: "sha256:child".to_string(),
            to_cid: "sha256:parent".to_string(),
            contribution_type: ContributionType::PreTraining,
        };
        let json = serde_json::to_string(&edge).unwrap();
        let back: LineageEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(back.from_cid, edge.from_cid);
        assert_eq!(back.to_cid, edge.to_cid);
    }

    #[test]
    fn royalty_weight_serde_roundtrip() {
        let rw = RoyaltyWeight {
            contributor_did: "did:neunode:agent1".to_string(),
            weight: 0.42,
            contribution_type: ContributionType::Data { dataset_hash: "sha256:ds".to_string() },
            staleness: 3,
        };
        let json = serde_json::to_string(&rw).unwrap();
        let back: RoyaltyWeight = serde_json::from_str(&json).unwrap();
        assert_eq!(back.contributor_did, rw.contributor_did);
        assert!((back.weight - rw.weight).abs() < f64::EPSILON);
        assert_eq!(back.staleness, rw.staleness);
    }

    #[test]
    fn type_weight_values() {
        assert!((type_weight(&ContributionType::PreTraining) - 0.30).abs() < f64::EPSILON);
        assert!(
            (type_weight(&ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 }) - 0.25)
                .abs()
                < f64::EPSILON
        );
        assert!(
            (type_weight(&ContributionType::RL { reward_model_cid: String::new() }) - 0.20).abs()
                < f64::EPSILON
        );
        assert!(
            (type_weight(&ContributionType::Data { dataset_hash: String::new() }) - 0.15).abs()
                < f64::EPSILON
        );
        assert!(
            (type_weight(&ContributionType::Merge { merge_method: String::new() }) - 0.05).abs()
                < f64::EPSILON
        );
        assert!(
            (type_weight(&ContributionType::Compute { duration_secs: 0.0 }) - 0.05).abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn ts_exports_non_empty() {
        let cfg = ts_rs::Config::default();
        assert!(!ContributionType::decl(&cfg).is_empty());
        assert!(!ModelNode::decl(&cfg).is_empty());
        assert!(!ModelMetadata::decl(&cfg).is_empty());
        assert!(!LineageEdge::decl(&cfg).is_empty());
        assert!(!RoyaltyWeight::decl(&cfg).is_empty());
    }

    #[test]
    fn model_node_base_model_empty_parents() {
        let node = ModelNode {
            cid: "sha256:base".to_string(),
            parent_cids: vec![],
            contributor_did: "did:neunode:agent1".to_string(),
            contribution_type: ContributionType::PreTraining,
            signature: vec![0u8; 64],
            created_at: 1712000000000,
            metadata: ModelMetadata::default(),
        };
        assert!(node.parent_cids.is_empty());
    }

    #[test]
    fn model_node_merge_two_parents() {
        let node = ModelNode {
            cid: "sha256:merged".to_string(),
            parent_cids: vec!["sha256:parent1".to_string(), "sha256:parent2".to_string()],
            contributor_did: "did:neunode:agent2".to_string(),
            contribution_type: ContributionType::Merge { merge_method: "slerp".to_string() },
            signature: vec![1u8; 64],
            created_at: 1712000000001,
            metadata: ModelMetadata::default(),
        };
        assert_eq!(node.parent_cids.len(), 2);
    }

    #[test]
    fn contribution_type_equality() {
        assert_eq!(ContributionType::PreTraining, ContributionType::PreTraining);
        assert_ne!(ContributionType::PreTraining, ContributionType::Compute { duration_secs: 0.0 });
        assert_eq!(
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 }
        );
    }

    #[test]
    fn signature_bytes_length_64() {
        let node = ModelNode {
            cid: "sha256:abc".to_string(),
            parent_cids: vec![],
            contributor_did: "did:neunode:agent".to_string(),
            contribution_type: ContributionType::PreTraining,
            signature: vec![42u8; 64],
            created_at: 0,
            metadata: ModelMetadata::default(),
        };
        assert_eq!(node.signature.len(), 64);
    }
}
