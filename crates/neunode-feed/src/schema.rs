use serde::{Deserialize, Serialize};

use crate::error::{FeedError, Result};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[ts(export)]
pub struct BountyPost {
    pub title: String,
    pub description: String,
    pub reward_amount: u64,
    pub reward_token: String,
    pub deadline: u64,
    pub required_capabilities: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[ts(export)]
pub struct BountyClaim {
    pub bounty_id: String,
    pub stake_amount: u64,
    pub stake_token: String,
    pub proposer_did: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[ts(export)]
pub struct Attestation {
    pub target_did: String,
    pub claim: String,
    pub evidence: Vec<String>,
    pub score: f64,
}

impl BountyPost {
    pub fn from_json(content: &str) -> Result<Self> {
        let post: BountyPost = serde_json::from_str(content).map_err(|e| {
            FeedError::SchemaValidationError(format!("invalid BountyPost JSON: {}", e))
        })?;

        if post.title.is_empty() {
            return Err(FeedError::SchemaValidationError("title is required".into()));
        }
        if post.description.is_empty() {
            return Err(FeedError::SchemaValidationError("description is required".into()));
        }
        if post.reward_token.is_empty() {
            return Err(FeedError::SchemaValidationError("reward_token is required".into()));
        }
        Ok(post)
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| FeedError::SerializationError(e.to_string()))
    }
}

impl BountyClaim {
    pub fn from_json(content: &str) -> Result<Self> {
        let claim: BountyClaim = serde_json::from_str(content).map_err(|e| {
            FeedError::SchemaValidationError(format!("invalid BountyClaim JSON: {}", e))
        })?;

        if claim.bounty_id.is_empty() {
            return Err(FeedError::SchemaValidationError("bounty_id is required".into()));
        }
        if claim.proposer_did.is_empty() {
            return Err(FeedError::SchemaValidationError("proposer_did is required".into()));
        }
        if claim.stake_token.is_empty() {
            return Err(FeedError::SchemaValidationError("stake_token is required".into()));
        }
        Ok(claim)
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| FeedError::SerializationError(e.to_string()))
    }
}

impl Attestation {
    pub fn from_json(content: &str) -> Result<Self> {
        let att: Attestation = serde_json::from_str(content).map_err(|e| {
            FeedError::SchemaValidationError(format!("invalid Attestation JSON: {}", e))
        })?;

        if att.target_did.is_empty() {
            return Err(FeedError::SchemaValidationError("target_did is required".into()));
        }
        if att.claim.is_empty() {
            return Err(FeedError::SchemaValidationError("claim is required".into()));
        }
        if !(0.0..=100.0).contains(&att.score) {
            return Err(FeedError::SchemaValidationError(
                "score must be between 0.0 and 100.0".into(),
            ));
        }
        Ok(att)
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| FeedError::SerializationError(e.to_string()))
    }
}

pub fn validate_bounty_post(content: &str) -> Result<BountyPost> {
    BountyPost::from_json(content)
}

pub fn validate_bounty_claim(content: &str) -> Result<BountyClaim> {
    BountyClaim::from_json(content)
}

pub fn validate_attestation(content: &str) -> Result<Attestation> {
    Attestation::from_json(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounty_post_valid_json() {
        let json = serde_json::json!({
            "title": "Train Llama-3B",
            "description": "Fine-tune on medical data",
            "reward_amount": 1000,
            "reward_token": "nTrain",
            "deadline": 1700000000,
            "required_capabilities": ["fine-tuning", "medical"]
        })
        .to_string();
        let post = validate_bounty_post(&json).expect("should parse");
        assert_eq!(post.title, "Train Llama-3B");
        assert_eq!(post.reward_amount, 1000);
        assert_eq!(post.required_capabilities.len(), 2);
    }

    #[test]
    fn bounty_post_missing_field() {
        let json = serde_json::json!({
            "title": "Test",
            "description": "Missing reward_amount"
        })
        .to_string();
        let result = validate_bounty_post(&json);
        assert!(result.is_err());
    }

    #[test]
    fn bounty_post_empty_title() {
        let json = serde_json::json!({
            "title": "",
            "description": "Has desc",
            "reward_amount": 100,
            "reward_token": "nCompute",
            "deadline": 1700000000,
            "required_capabilities": []
        })
        .to_string();
        let result = validate_bounty_post(&json);
        assert!(result.is_err());
        match result.unwrap_err() {
            FeedError::SchemaValidationError(msg) => assert!(msg.contains("title")),
            other => panic!("expected SchemaValidationError, got {:?}", other),
        }
    }

    #[test]
    fn bounty_post_empty_description() {
        let json = serde_json::json!({
            "title": "Has title",
            "description": "",
            "reward_amount": 100,
            "reward_token": "nCompute",
            "deadline": 1700000000,
            "required_capabilities": []
        })
        .to_string();
        assert!(validate_bounty_post(&json).is_err());
    }

    #[test]
    fn bounty_post_empty_reward_token() {
        let json = serde_json::json!({
            "title": "T",
            "description": "D",
            "reward_amount": 100,
            "reward_token": "",
            "deadline": 1700000000,
            "required_capabilities": []
        })
        .to_string();
        assert!(validate_bounty_post(&json).is_err());
    }

    #[test]
    fn bounty_post_invalid_json() {
        let result = validate_bounty_post("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn bounty_post_to_json_roundtrip() {
        let post = BountyPost {
            title: "Test Bounty".to_string(),
            description: "A test".to_string(),
            reward_amount: 500,
            reward_token: "nCompute".to_string(),
            deadline: 1700000000,
            required_capabilities: vec!["gpu".to_string()],
        };
        let json = post.to_json().expect("serialize");
        let back = BountyPost::from_json(&json).expect("parse");
        assert_eq!(post, back);
    }

    #[test]
    fn bounty_claim_valid_json() {
        let json = serde_json::json!({
            "bounty_id": "bnty_abc123",
            "stake_amount": 50,
            "stake_token": "nCompute",
            "proposer_did": "did:neunode:agent42"
        })
        .to_string();
        let claim = validate_bounty_claim(&json).expect("should parse");
        assert_eq!(claim.bounty_id, "bnty_abc123");
        assert_eq!(claim.stake_amount, 50);
    }

    #[test]
    fn bounty_claim_missing_field() {
        let json = serde_json::json!({
            "bounty_id": "bnty_abc"
        })
        .to_string();
        assert!(validate_bounty_claim(&json).is_err());
    }

    #[test]
    fn bounty_claim_empty_bounty_id() {
        let json = serde_json::json!({
            "bounty_id": "",
            "stake_amount": 50,
            "stake_token": "nCompute",
            "proposer_did": "did:neunode:agent42"
        })
        .to_string();
        assert!(validate_bounty_claim(&json).is_err());
    }

    #[test]
    fn bounty_claim_empty_proposer_did() {
        let json = serde_json::json!({
            "bounty_id": "bnty_abc",
            "stake_amount": 50,
            "stake_token": "nCompute",
            "proposer_did": ""
        })
        .to_string();
        assert!(validate_bounty_claim(&json).is_err());
    }

    #[test]
    fn bounty_claim_empty_stake_token() {
        let json = serde_json::json!({
            "bounty_id": "bnty_abc",
            "stake_amount": 50,
            "stake_token": "",
            "proposer_did": "did:neunode:agent42"
        })
        .to_string();
        assert!(validate_bounty_claim(&json).is_err());
    }

    #[test]
    fn bounty_claim_invalid_json() {
        assert!(validate_bounty_claim("{invalid}").is_err());
    }

    #[test]
    fn bounty_claim_to_json_roundtrip() {
        let claim = BountyClaim {
            bounty_id: "bnty_xyz".to_string(),
            stake_amount: 100,
            stake_token: "nTrain".to_string(),
            proposer_did: "did:neunode:agent1".to_string(),
        };
        let json = claim.to_json().expect("serialize");
        let back = BountyClaim::from_json(&json).expect("parse");
        assert_eq!(claim, back);
    }

    #[test]
    fn attestation_valid_json() {
        let json = serde_json::json!({
            "target_did": "did:neunode:target",
            "claim": "completed training",
            "evidence": ["hash1", "hash2"],
            "score": 85.5
        })
        .to_string();
        let att = validate_attestation(&json).expect("should parse");
        assert_eq!(att.target_did, "did:neunode:target");
        assert_eq!(att.score, 85.5);
        assert_eq!(att.evidence.len(), 2);
    }

    #[test]
    fn attestation_missing_field() {
        let json = serde_json::json!({
            "target_did": "did:neunode:target"
        })
        .to_string();
        assert!(validate_attestation(&json).is_err());
    }

    #[test]
    fn attestation_empty_target_did() {
        let json = serde_json::json!({
            "target_did": "",
            "claim": "test",
            "evidence": [],
            "score": 50.0
        })
        .to_string();
        assert!(validate_attestation(&json).is_err());
    }

    #[test]
    fn attestation_empty_claim() {
        let json = serde_json::json!({
            "target_did": "did:neunode:target",
            "claim": "",
            "evidence": [],
            "score": 50.0
        })
        .to_string();
        assert!(validate_attestation(&json).is_err());
    }

    #[test]
    fn attestation_score_below_range() {
        let json = serde_json::json!({
            "target_did": "did:neunode:target",
            "claim": "test",
            "evidence": [],
            "score": -1.0
        })
        .to_string();
        assert!(validate_attestation(&json).is_err());
    }

    #[test]
    fn attestation_score_above_range() {
        let json = serde_json::json!({
            "target_did": "did:neunode:target",
            "claim": "test",
            "evidence": [],
            "score": 100.1
        })
        .to_string();
        assert!(validate_attestation(&json).is_err());
    }

    #[test]
    fn attestation_score_boundary_zero() {
        let json = serde_json::json!({
            "target_did": "did:neunode:target",
            "claim": "test",
            "evidence": [],
            "score": 0.0
        })
        .to_string();
        assert!(validate_attestation(&json).is_ok());
    }

    #[test]
    fn attestation_score_boundary_hundred() {
        let json = serde_json::json!({
            "target_did": "did:neunode:target",
            "claim": "test",
            "evidence": [],
            "score": 100.0
        })
        .to_string();
        assert!(validate_attestation(&json).is_ok());
    }

    #[test]
    fn attestation_invalid_json() {
        assert!(validate_attestation("not json").is_err());
    }

    #[test]
    fn attestation_to_json_roundtrip() {
        let att = Attestation {
            target_did: "did:neunode:target".to_string(),
            claim: "verified work".to_string(),
            evidence: vec!["proof_a".to_string()],
            score: 92.3,
        };
        let json = att.to_json().expect("serialize");
        let back = Attestation::from_json(&json).expect("parse");
        assert_eq!(att, back);
    }
}
