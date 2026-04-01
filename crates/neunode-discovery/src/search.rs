use crate::error::{DiscoveryError, Result};
use crate::scoring::compute_score;
use crate::types::{AgentCandidate, DiscoveryRequest, ScoredAgent, ScoringWeights};

/// Search for agents matching a discovery request.
///
/// Applies hard constraints (min_reputation, max_cost, must_be_online,
/// capability overlap) to filter candidates, then scores and ranks
/// remaining candidates using the 5-factor formula.
///
/// # Errors
///
/// Returns `DiscoveryError::EmptyPool` if the candidate pool is empty.
/// Returns `DiscoveryError::NoMatches` if no candidates pass filtering.
pub fn search(
    candidates: &[AgentCandidate],
    request: &DiscoveryRequest,
    weights: &ScoringWeights,
) -> Result<Vec<ScoredAgent>> {
    if candidates.is_empty() {
        return Err(DiscoveryError::EmptyPool);
    }

    // Hard constraint filtering
    let filtered: Vec<&AgentCandidate> =
        candidates.iter().filter(|c| passes_constraints(c, request)).collect();

    if filtered.is_empty() {
        return Err(DiscoveryError::NoMatches);
    }

    // Collect filtered candidates for normalization context
    let filtered_owned: Vec<AgentCandidate> = filtered.into_iter().cloned().collect();

    // Score each candidate
    let mut scored: Vec<ScoredAgent> = filtered_owned
        .iter()
        .map(|c| compute_score(c, request, &filtered_owned, weights))
        .collect();

    // Sort by final score descending, then tie-break:
    // reputation DESC, stake DESC, DID hash ASC
    scored.sort_by(|a, b| {
        b.final_score
            .partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.candidate
                    .reputation_score
                    .partial_cmp(&a.candidate.reputation_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| b.candidate.stake_amount.cmp(&a.candidate.stake_amount))
            .then_with(|| a.candidate.did.cmp(&b.candidate.did))
    });

    // Truncate to max_results
    scored.truncate(request.max_results);

    Ok(scored)
}

/// Check if a candidate passes all hard constraints from the request.
fn passes_constraints(candidate: &AgentCandidate, request: &DiscoveryRequest) -> bool {
    // Must be online (if required)
    if request.must_be_online && !candidate.is_online {
        return false;
    }

    // Minimum reputation filter
    if let Some(min_rep) = request.min_reputation {
        if candidate.reputation_score < min_rep {
            return false;
        }
    }

    // Maximum cost filter
    if let Some(max_cost) = request.max_cost_per_unit {
        if candidate.cost_per_unit > max_cost {
            return false;
        }
    }

    // Must have at least one matching capability (if any are required)
    if !request.required_capabilities.is_empty() {
        let candidate_set: std::collections::HashSet<&str> =
            candidate.capabilities.iter().map(|s| s.as_str()).collect();
        let has_match =
            request.required_capabilities.iter().any(|cap| candidate_set.contains(cap.as_str()));
        if !has_match {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candidate(
        did: &str,
        caps: &[&str],
        rep: f64,
        stake: u64,
        avail: f64,
        cost: f64,
        online: bool,
    ) -> AgentCandidate {
        AgentCandidate {
            did: did.to_string(),
            capabilities: caps.iter().map(|s| s.to_string()).collect(),
            reputation_score: rep,
            stake_amount: stake,
            availability_score: avail,
            latency_ms: 50,
            cost_per_unit: cost,
            is_online: online,
        }
    }

    fn make_request(
        req_caps: &[&str],
        min_rep: Option<f64>,
        max_cost: Option<f64>,
        online_only: bool,
        max_results: usize,
    ) -> DiscoveryRequest {
        DiscoveryRequest {
            required_capabilities: req_caps.iter().map(|s| s.to_string()).collect(),
            min_reputation: min_rep,
            max_cost_per_unit: max_cost,
            must_be_online: online_only,
            max_results,
            requester_capabilities: vec![],
        }
    }

    #[test]
    fn basic_search_returns_ranked() {
        let candidates = vec![
            make_candidate("did:1", &["a"], 4.0, 1000, 0.9, 10.0, true),
            make_candidate("did:2", &["a"], 2.0, 500, 0.7, 5.0, true),
            make_candidate("did:3", &["b"], 5.0, 2000, 0.99, 20.0, true),
        ];
        let request = make_request(&["a"], None, None, false, 10);
        let weights = ScoringWeights::default();
        let results = search(&candidates, &request, &weights).unwrap();
        assert_eq!(results.len(), 2);
        // Both should have "a" capability
        assert!(results.iter().all(|r| r.candidate.capabilities.contains(&"a".to_string())));
        // Should be sorted by final_score desc
        for window in results.windows(2) {
            assert!(window[0].final_score >= window[1].final_score);
        }
    }

    #[test]
    fn filter_by_min_reputation() {
        let candidates = vec![
            make_candidate("did:1", &["a"], 4.0, 1000, 0.9, 10.0, true),
            make_candidate("did:2", &["a"], 1.0, 500, 0.7, 5.0, true),
        ];
        let request = make_request(&["a"], Some(3.0), None, false, 10);
        let weights = ScoringWeights::default();
        let results = search(&candidates, &request, &weights).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].candidate.did, "did:1");
    }

    #[test]
    fn filter_by_max_cost() {
        let candidates = vec![
            make_candidate("did:1", &["a"], 4.0, 1000, 0.9, 15.0, true),
            make_candidate("did:2", &["a"], 3.0, 500, 0.7, 5.0, true),
        ];
        let request = make_request(&["a"], None, Some(10.0), false, 10);
        let weights = ScoringWeights::default();
        let results = search(&candidates, &request, &weights).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].candidate.did, "did:2");
    }

    #[test]
    fn filter_by_online() {
        let candidates = vec![
            make_candidate("did:1", &["a"], 4.0, 1000, 0.9, 10.0, true),
            make_candidate("did:2", &["a"], 4.5, 2000, 0.99, 5.0, false),
        ];
        let request = make_request(&["a"], None, None, true, 10);
        let weights = ScoringWeights::default();
        let results = search(&candidates, &request, &weights).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].candidate.is_online);
    }

    #[test]
    fn multi_capability_matching() {
        let candidates = vec![
            make_candidate("did:1", &["a", "b", "c"], 4.0, 1000, 0.9, 10.0, true),
            make_candidate("did:2", &["a"], 4.0, 1000, 0.9, 10.0, true),
        ];
        let request = make_request(&["a", "b"], None, None, false, 10);
        let weights = ScoringWeights::default();
        let results = search(&candidates, &request, &weights).unwrap();
        assert_eq!(results.len(), 2);
        // did:1 matches 2/2 caps, did:2 matches 1/2 caps → did:1 scores higher
        assert_eq!(results[0].candidate.did, "did:1");
        assert!(results[0].capability_score > results[1].capability_score);
    }

    #[test]
    fn max_results_limiting() {
        let candidates = vec![
            make_candidate("did:1", &["a"], 5.0, 3000, 0.99, 1.0, true),
            make_candidate("did:2", &["a"], 4.0, 2000, 0.9, 5.0, true),
            make_candidate("did:3", &["a"], 3.0, 1000, 0.8, 10.0, true),
        ];
        let request = make_request(&["a"], None, None, false, 2);
        let weights = ScoringWeights::default();
        let results = search(&candidates, &request, &weights).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn empty_pool_error() {
        let candidates: Vec<AgentCandidate> = vec![];
        let request = make_request(&["a"], None, None, false, 10);
        let weights = ScoringWeights::default();
        let result = search(&candidates, &request, &weights);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DiscoveryError::EmptyPool));
    }

    #[test]
    fn no_matches_error() {
        let candidates = vec![make_candidate("did:1", &["x"], 4.0, 1000, 0.9, 10.0, true)];
        let request = make_request(&["a"], None, None, false, 10);
        let weights = ScoringWeights::default();
        let result = search(&candidates, &request, &weights);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DiscoveryError::NoMatches));
    }

    #[test]
    fn no_required_caps_returns_all() {
        let candidates = vec![
            make_candidate("did:1", &["x"], 4.0, 1000, 0.9, 10.0, true),
            make_candidate("did:2", &["y"], 3.0, 500, 0.7, 15.0, true),
        ];
        let request = make_request(&[], None, None, false, 10);
        let weights = ScoringWeights::default();
        let results = search(&candidates, &request, &weights).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn tie_break_by_reputation_then_stake_then_did() {
        // Create two candidates with identical scoring characteristics
        let c1 = AgentCandidate {
            did: "did:neunode:0xAAA".to_string(),
            capabilities: vec!["a".to_string()],
            reputation_score: 4.0,
            stake_amount: 1000,
            availability_score: 0.9,
            latency_ms: 50,
            cost_per_unit: 10.0,
            is_online: true,
        };
        let c2 = AgentCandidate {
            did: "did:neunode:0xBBB".to_string(),
            capabilities: vec!["a".to_string()],
            reputation_score: 4.0,
            stake_amount: 500,
            availability_score: 0.9,
            latency_ms: 50,
            cost_per_unit: 10.0,
            is_online: true,
        };
        let candidates = vec![c1.clone(), c2.clone()];
        let request = make_request(&["a"], None, None, false, 10);
        let weights = ScoringWeights::default();
        let results = search(&candidates, &request, &weights).unwrap();
        // Same score, same reputation → stake breaks tie: c1 (1000) > c2 (500)
        assert_eq!(results[0].candidate.stake_amount, 1000);
    }
}
