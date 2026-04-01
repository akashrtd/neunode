use crate::complement::jaccard_distance;
use crate::types::{AgentCandidate, DiscoveryRequest, ScoredAgent, ScoringWeights};

/// Compute the 5-factor discovery score for a single candidate.
///
/// Each factor is first computed as a raw score, then cost is min-max
/// normalized across all candidates. The final score is the weighted sum.
pub fn compute_score(
    candidate: &AgentCandidate,
    request: &DiscoveryRequest,
    all_candidates: &[AgentCandidate],
    weights: &ScoringWeights,
) -> ScoredAgent {
    let capability_score =
        score_capability_match(&candidate.capabilities, &request.required_capabilities);
    let quality_score = score_quality(candidate.reputation_score);
    let availability_score = score_availability(candidate.is_online, candidate.availability_score);

    let all_costs: Vec<f64> = all_candidates.iter().map(|c| c.cost_per_unit).collect();
    let cost_score = score_cost(candidate.cost_per_unit, &all_costs);

    let complementarity_score =
        score_complementarity(&request.requester_capabilities, &candidate.capabilities);

    let final_score = weights.capability_match * capability_score
        + weights.quality * quality_score
        + weights.availability * availability_score
        + weights.cost_efficiency * cost_score
        + weights.complementarity * complementarity_score;

    ScoredAgent {
        candidate: candidate.clone(),
        final_score,
        capability_score,
        quality_score,
        availability_score,
        cost_score,
        complementarity_score,
    }
}

/// Normalize a value to [0.0, 1.0] using min-max scaling.
///
/// Returns 1.0 if all values are identical (range == 0) to avoid division by zero.
/// Higher values are better, so we invert: (max - value) / range.
fn min_max_normalize(value: f64, min: f64, max: f64) -> f64 {
    if (max - min).abs() < f64::EPSILON {
        return 1.0;
    }
    (max - value) / (max - min)
}

/// Score capability match: fraction of required capabilities the agent has.
///
/// Returns 1.0 if agent has all required caps, 0.0 if it has none.
/// Returns 0.0 if no capabilities are required.
pub fn score_capability_match(candidate_caps: &[String], required_caps: &[String]) -> f64 {
    if required_caps.is_empty() {
        return 0.0;
    }
    let candidate_set: std::collections::HashSet<&str> =
        candidate_caps.iter().map(|s| s.as_str()).collect();
    let matched = required_caps.iter().filter(|cap| candidate_set.contains(cap.as_str())).count();
    matched as f64 / required_caps.len() as f64
}

/// Score quality: normalize reputation from [0.0, 5.0] to [0.0, 1.0].
pub fn score_quality(reputation: f64) -> f64 {
    (reputation / 5.0).clamp(0.0, 1.0)
}

/// Score availability: combines online status with uptime percentage.
///
/// Online agents get their uptime score directly.
/// Offline agents get a heavy penalty (0.0 regardless of uptime).
pub fn score_availability(is_online: bool, uptime: f64) -> f64 {
    if !is_online {
        return 0.0;
    }
    uptime.clamp(0.0, 1.0)
}

/// Score cost efficiency: inverse cost normalized across all candidates.
///
/// Lower cost = higher score. Uses min-max normalization where the cheapest
/// agent scores 1.0 and the most expensive scores 0.0.
pub fn score_cost(cost_per_unit: f64, all_costs: &[f64]) -> f64 {
    if all_costs.is_empty() {
        return 0.0;
    }
    let min = all_costs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = all_costs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    min_max_normalize(cost_per_unit, min, max)
}

/// Score complementarity using Jaccard distance between capability sets.
///
/// Agents with capabilities the requester lacks score higher (more complementary).
/// Delegates to `jaccard_distance` from the complement module.
pub fn score_complementarity(requester_caps: &[String], candidate_caps: &[String]) -> f64 {
    jaccard_distance(requester_caps, candidate_caps)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_candidate() -> AgentCandidate {
        AgentCandidate {
            did: "did:neunode:0xTest".to_string(),
            capabilities: vec!["inference:llm".to_string(), "training:lora".to_string()],
            reputation_score: 4.0,
            stake_amount: 1000,
            availability_score: 0.95,
            latency_ms: 50,
            cost_per_unit: 10.0,
            is_online: true,
        }
    }

    fn test_request() -> DiscoveryRequest {
        DiscoveryRequest {
            required_capabilities: vec!["inference:llm".to_string()],
            min_reputation: None,
            max_cost_per_unit: None,
            must_be_online: false,
            max_results: 10,
            requester_capabilities: vec!["training:lora".to_string()],
        }
    }

    fn test_candidates() -> Vec<AgentCandidate> {
        vec![
            test_candidate(),
            AgentCandidate {
                did: "did:neunode:0xCheap".to_string(),
                capabilities: vec!["inference:llm".to_string()],
                reputation_score: 3.0,
                stake_amount: 500,
                availability_score: 0.80,
                latency_ms: 100,
                cost_per_unit: 5.0,
                is_online: true,
            },
            AgentCandidate {
                did: "did:neunode:0xExpensive".to_string(),
                capabilities: vec!["training:lora".to_string()],
                reputation_score: 4.5,
                stake_amount: 2000,
                availability_score: 0.99,
                latency_ms: 20,
                cost_per_unit: 20.0,
                is_online: false,
            },
        ]
    }

    // --- capability match tests ---

    #[test]
    fn capability_match_exact() {
        let caps = vec!["a".to_string(), "b".to_string()];
        let req = vec!["a".to_string(), "b".to_string()];
        let score = score_capability_match(&caps, &req);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn capability_match_partial() {
        let caps = vec!["a".to_string(), "b".to_string()];
        let req = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let score = score_capability_match(&caps, &req);
        assert!((score - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn capability_match_none() {
        let caps = vec!["x".to_string()];
        let req = vec!["a".to_string(), "b".to_string()];
        let score = score_capability_match(&caps, &req);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn capability_match_empty_required() {
        let caps = vec!["a".to_string()];
        let req: Vec<String> = vec![];
        let score = score_capability_match(&caps, &req);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn capability_match_empty_candidate() {
        let caps: Vec<String> = vec![];
        let req = vec!["a".to_string()];
        let score = score_capability_match(&caps, &req);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn capability_match_single_match() {
        let caps = vec!["b".to_string(), "c".to_string()];
        let req = vec!["a".to_string(), "b".to_string()];
        let score = score_capability_match(&caps, &req);
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    // --- quality scoring tests ---

    #[test]
    fn quality_max_reputation() {
        assert!((score_quality(5.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn quality_zero_reputation() {
        assert!((score_quality(0.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn quality_mid_reputation() {
        assert!((score_quality(2.5) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn quality_clamped_above_max() {
        // Reputation above 5.0 should be clamped to 1.0
        assert!((score_quality(7.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn quality_clamped_below_zero() {
        assert!((score_quality(-1.0) - 0.0).abs() < f64::EPSILON);
    }

    // --- availability scoring tests ---

    #[test]
    fn availability_online_high_uptime() {
        assert!((score_availability(true, 0.95) - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn availability_offline_zero() {
        assert!((score_availability(false, 0.95) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn availability_online_full_uptime() {
        assert!((score_availability(true, 1.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn availability_online_zero_uptime() {
        assert!((score_availability(true, 0.0) - 0.0).abs() < f64::EPSILON);
    }

    // --- cost scoring tests ---

    #[test]
    fn cost_cheapest_gets_one() {
        let costs = vec![5.0, 10.0, 20.0];
        assert!((score_cost(5.0, &costs) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_most_expensive_gets_zero() {
        let costs = vec![5.0, 10.0, 20.0];
        assert!((score_cost(20.0, &costs) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_mid_normalized() {
        let costs = vec![5.0, 10.0, 20.0];
        // (20 - 10) / (20 - 5) = 10/15 ≈ 0.667
        assert!((score_cost(10.0, &costs) - 10.0 / 15.0).abs() < 1e-10);
    }

    #[test]
    fn cost_all_same_returns_one() {
        let costs = vec![10.0, 10.0, 10.0];
        assert!((score_cost(10.0, &costs) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_empty_costs_returns_zero() {
        assert!((score_cost(10.0, &[]) - 0.0).abs() < f64::EPSILON);
    }

    // --- complementarity scoring tests ---

    #[test]
    fn complementarity_disjoint_sets() {
        let req = vec!["a".to_string(), "b".to_string()];
        let cand = vec!["c".to_string(), "d".to_string()];
        // Jaccard distance of disjoint sets = 1.0
        assert!((score_complementarity(&req, &cand) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn complementarity_identical_sets() {
        let caps = vec!["a".to_string(), "b".to_string()];
        // Jaccard distance of identical sets = 0.0
        assert!((score_complementarity(&caps, &caps) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn complementarity_partial_overlap() {
        let req = vec!["a".to_string(), "b".to_string()];
        let cand = vec!["b".to_string(), "c".to_string()];
        // Union = {a,b,c} size 3, Intersection = {b} size 1
        // Jaccard index = 1/3, distance = 1 - 1/3 = 2/3
        assert!((score_complementarity(&req, &cand) - 2.0 / 3.0).abs() < 1e-10);
    }

    // --- full integration test ---

    #[test]
    fn full_score_computation() {
        let candidates = test_candidates();
        let request = test_request();
        let weights = ScoringWeights::default();
        let scored = compute_score(&candidates[0], &request, &candidates, &weights);
        // Verify all factor scores are in [0,1]
        assert!(scored.capability_score >= 0.0 && scored.capability_score <= 1.0);
        assert!(scored.quality_score >= 0.0 && scored.quality_score <= 1.0);
        assert!(scored.availability_score >= 0.0 && scored.availability_score <= 1.0);
        assert!(scored.cost_score >= 0.0 && scored.cost_score <= 1.0);
        assert!(scored.complementarity_score >= 0.0 && scored.complementarity_score <= 1.0);
        // Final score should be weighted sum
        let expected = weights.capability_match * scored.capability_score
            + weights.quality * scored.quality_score
            + weights.availability * scored.availability_score
            + weights.cost_efficiency * scored.cost_score
            + weights.complementarity * scored.complementarity_score;
        assert!((scored.final_score - expected).abs() < 1e-10);
    }

    #[test]
    fn offline_agent_scores_lower_than_online() {
        let candidates = test_candidates();
        let request = test_request();
        let weights = ScoringWeights::default();
        let online_score = compute_score(&candidates[0], &request, &candidates, &weights);
        let offline_score = compute_score(&candidates[2], &request, &candidates, &weights);
        assert!(online_score.availability_score > offline_score.availability_score);
    }

    #[test]
    fn tie_break_by_reputation() {
        // Same capabilities but different reputation
        let c1 = AgentCandidate {
            did: "did:neunode:0xHigh".to_string(),
            capabilities: vec!["a".to_string()],
            reputation_score: 4.0,
            stake_amount: 100,
            availability_score: 0.9,
            latency_ms: 50,
            cost_per_unit: 10.0,
            is_online: true,
        };
        let c2 = AgentCandidate {
            did: "did:neunode:0xLow".to_string(),
            capabilities: vec!["a".to_string()],
            reputation_score: 2.0,
            stake_amount: 100,
            availability_score: 0.9,
            latency_ms: 50,
            cost_per_unit: 10.0,
            is_online: true,
        };
        let candidates = vec![c1.clone(), c2.clone()];
        let request = DiscoveryRequest {
            required_capabilities: vec!["a".to_string()],
            min_reputation: None,
            max_cost_per_unit: None,
            must_be_online: false,
            max_results: 2,
            requester_capabilities: vec![],
        };
        let weights = ScoringWeights::default();
        let s1 = compute_score(&c1, &request, &candidates, &weights);
        let s2 = compute_score(&c2, &request, &candidates, &weights);
        assert!(s1.final_score > s2.final_score, "higher reputation should score higher");
    }
}
