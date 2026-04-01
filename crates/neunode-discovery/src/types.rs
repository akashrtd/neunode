use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The 5-factor scoring weights for discovery ranking.
///
/// Each weight must be in [0.0, 1.0] and they must sum to 1.0.
/// Default weights match the Neunode discovery protocol specification.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScoringWeights {
    /// Weight for capability match (fraction of required caps the agent has).
    /// Default: 0.40
    pub capability_match: f64,
    /// Weight for quality (reputation score normalization).
    /// Default: 0.25
    pub quality: f64,
    /// Weight for availability (uptime and online status).
    /// Default: 0.15
    pub availability: f64,
    /// Weight for cost efficiency (inverse cost, normalized).
    /// Default: 0.10
    pub cost_efficiency: f64,
    /// Weight for complementarity (Jaccard distance with requester).
    /// Default: 0.10
    pub complementarity: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            capability_match: 0.40,
            quality: 0.25,
            availability: 0.15,
            cost_efficiency: 0.10,
            complementarity: 0.10,
        }
    }
}

impl ScoringWeights {
    /// Validate that all weights are non-negative and sum to ~1.0
    /// (within floating-point tolerance of 1e-6).
    pub fn validate(&self) -> bool {
        let sum = self.capability_match
            + self.quality
            + self.availability
            + self.cost_efficiency
            + self.complementarity;
        self.capability_match >= 0.0
            && self.quality >= 0.0
            && self.availability >= 0.0
            && self.cost_efficiency >= 0.0
            && self.complementarity >= 0.0
            && (sum - 1.0).abs() < 1e-6
    }

    /// Sum of all weights.
    pub fn sum(&self) -> f64 {
        self.capability_match
            + self.quality
            + self.availability
            + self.cost_efficiency
            + self.complementarity
    }
}

/// An agent being evaluated as a discovery candidate.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentCandidate {
    /// DID of the agent.
    pub did: String,
    /// Capabilities this agent advertises (e.g., "inference:llm", "training:lora").
    pub capabilities: Vec<String>,
    /// Reputation score from neunode-reputation (0.0–5.0).
    pub reputation_score: f64,
    /// Staked token amount (skin-in-the-game).
    #[ts(type = "number")]
    pub stake_amount: u64,
    /// Availability/uptime score (0.0–1.0, percentage of time online).
    pub availability_score: f64,
    /// Network latency in milliseconds.
    #[ts(type = "number")]
    pub latency_ms: u32,
    /// Cost per compute unit (tokens per compute-hour).
    pub cost_per_unit: f64,
    /// Whether the agent is currently online.
    pub is_online: bool,
}

/// A single scored result from discovery search.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScoredAgent {
    /// The candidate agent that was scored.
    pub candidate: AgentCandidate,
    /// Final weighted score (0.0–1.0 after normalization).
    pub final_score: f64,
    /// Individual capability match score.
    pub capability_score: f64,
    /// Individual quality score.
    pub quality_score: f64,
    /// Individual availability score.
    pub availability_score: f64,
    /// Individual cost efficiency score.
    pub cost_score: f64,
    /// Individual complementarity score.
    pub complementarity_score: f64,
}

/// Discovery search request specifying what capabilities are needed.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DiscoveryRequest {
    /// Required capabilities that candidates must partially or fully match.
    pub required_capabilities: Vec<String>,
    /// Minimum reputation score filter (0.0–5.0).
    pub min_reputation: Option<f64>,
    /// Maximum cost per unit filter.
    pub max_cost_per_unit: Option<f64>,
    /// Whether candidates must be currently online.
    pub must_be_online: bool,
    /// Maximum number of results to return.
    #[ts(type = "number")]
    pub max_results: usize,
    /// Capabilities of the requesting agent (for complementarity scoring).
    pub requester_capabilities: Vec<String>,
}

/// A capability gap — a registered capability with zero available providers.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CapabilityGap {
    /// The capability URI that has no providers.
    pub capability_uri: String,
    /// Number of bounties/tasks requiring this capability.
    #[ts(type = "number")]
    pub demand_count: u32,
}

/// Normalization bounds for a single scoring factor across candidates.
pub struct NormalizedFactor {
    /// Minimum value observed.
    pub min: f64,
    /// Maximum value observed.
    pub max: f64,
    /// Range (max - min). Zero if all values are identical.
    pub range: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_weights_sum_to_one() {
        let w = ScoringWeights::default();
        let sum = w.sum();
        assert!((sum - 1.0).abs() < 1e-6, "default weights must sum to 1.0, got {sum}");
    }

    #[test]
    fn default_weights_validate() {
        let w = ScoringWeights::default();
        assert!(w.validate(), "default weights must be valid");
    }

    #[test]
    fn custom_weights_validate() {
        let w = ScoringWeights {
            capability_match: 0.50,
            quality: 0.20,
            availability: 0.10,
            cost_efficiency: 0.10,
            complementarity: 0.10,
        };
        assert!(w.validate());
    }

    #[test]
    fn invalid_weights_do_not_validate() {
        let w = ScoringWeights {
            capability_match: 0.50,
            quality: 0.50,
            availability: 0.50,
            cost_efficiency: 0.10,
            complementarity: 0.10,
        };
        assert!(!w.validate(), "sum > 1.0 should not validate");
    }

    #[test]
    fn negative_weights_do_not_validate() {
        let w = ScoringWeights {
            capability_match: -0.10,
            quality: 0.40,
            availability: 0.30,
            cost_efficiency: 0.20,
            complementarity: 0.20,
        };
        assert!(!w.validate(), "negative weight should not validate");
    }

    #[test]
    fn weights_sum_method() {
        let w = ScoringWeights {
            capability_match: 0.1,
            quality: 0.2,
            availability: 0.3,
            cost_efficiency: 0.2,
            complementarity: 0.1,
        };
        assert!((w.sum() - 0.9).abs() < 1e-10);
    }

    #[test]
    fn scoring_weights_serde_roundtrip() {
        let w = ScoringWeights::default();
        let json = serde_json::to_string(&w).unwrap();
        let back: ScoringWeights = serde_json::from_str(&json).unwrap();
        assert!((back.capability_match - w.capability_match).abs() < f64::EPSILON);
        assert!((back.quality - w.quality).abs() < f64::EPSILON);
        assert!((back.availability - w.availability).abs() < f64::EPSILON);
        assert!((back.cost_efficiency - w.cost_efficiency).abs() < f64::EPSILON);
        assert!((back.complementarity - w.complementarity).abs() < f64::EPSILON);
    }

    #[test]
    fn agent_candidate_serde_roundtrip() {
        let c = AgentCandidate {
            did: "did:neunode:0xABC".to_string(),
            capabilities: vec!["inference:llm".to_string(), "training:lora".to_string()],
            reputation_score: 3.5,
            stake_amount: 1000,
            availability_score: 0.95,
            latency_ms: 50,
            cost_per_unit: 10.0,
            is_online: true,
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: AgentCandidate = serde_json::from_str(&json).unwrap();
        assert_eq!(back.did, c.did);
        assert_eq!(back.capabilities, c.capabilities);
        assert_eq!(back.stake_amount, c.stake_amount);
        assert_eq!(back.is_online, c.is_online);
    }

    #[test]
    fn scored_agent_serde_roundtrip() {
        let sa = ScoredAgent {
            candidate: AgentCandidate {
                did: "did:neunode:0xDEF".to_string(),
                capabilities: vec![],
                reputation_score: 4.0,
                stake_amount: 500,
                availability_score: 0.8,
                latency_ms: 100,
                cost_per_unit: 5.0,
                is_online: false,
            },
            final_score: 0.75,
            capability_score: 0.9,
            quality_score: 0.8,
            availability_score: 0.0,
            cost_score: 0.6,
            complementarity_score: 0.5,
        };
        let json = serde_json::to_string(&sa).unwrap();
        let back: ScoredAgent = serde_json::from_str(&json).unwrap();
        assert!((back.final_score - sa.final_score).abs() < f64::EPSILON);
        assert_eq!(back.candidate.did, sa.candidate.did);
    }

    #[test]
    fn discovery_request_serde_roundtrip() {
        let req = DiscoveryRequest {
            required_capabilities: vec!["inference:llm".to_string()],
            min_reputation: Some(2.0),
            max_cost_per_unit: Some(20.0),
            must_be_online: true,
            max_results: 10,
            requester_capabilities: vec!["training:lora".to_string()],
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: DiscoveryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.required_capabilities, req.required_capabilities);
        assert_eq!(back.max_results, req.max_results);
        assert_eq!(back.must_be_online, req.must_be_online);
    }

    #[test]
    fn capability_gap_serde_roundtrip() {
        let gap =
            CapabilityGap { capability_uri: "training:pretrain".to_string(), demand_count: 5 };
        let json = serde_json::to_string(&gap).unwrap();
        let back: CapabilityGap = serde_json::from_str(&json).unwrap();
        assert_eq!(back.capability_uri, gap.capability_uri);
        assert_eq!(back.demand_count, gap.demand_count);
    }

    #[test]
    fn normalized_factor_fields() {
        let nf = NormalizedFactor { min: 1.0, max: 10.0, range: 9.0 };
        assert!((nf.range - 9.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ts_exports_non_empty() {
        let cfg = ts_rs::Config::default();
        assert!(!ScoringWeights::decl(&cfg).is_empty());
        assert!(!AgentCandidate::decl(&cfg).is_empty());
        assert!(!ScoredAgent::decl(&cfg).is_empty());
        assert!(!DiscoveryRequest::decl(&cfg).is_empty());
        assert!(!CapabilityGap::decl(&cfg).is_empty());
    }
}
