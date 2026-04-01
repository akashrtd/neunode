use crate::types::{AgentCandidate, ScoredAgent};

/// Compute Jaccard distance between two capability sets.
///
/// Jaccard distance = 1 - Jaccard index = 1 - |intersection| / |union|.
/// Returns 0.0 for identical sets, 1.0 for disjoint sets.
/// Returns 0.0 if both sets are empty (convention: no distance).
pub fn jaccard_distance(set_a: &[String], set_b: &[String]) -> f64 {
    if set_a.is_empty() && set_b.is_empty() {
        return 0.0;
    }
    let a: std::collections::HashSet<&str> = set_a.iter().map(|s| s.as_str()).collect();
    let b: std::collections::HashSet<&str> = set_b.iter().map(|s| s.as_str()).collect();
    let intersection = a.intersection(&b).count();
    let union = a.union(&b).count();
    if union == 0 {
        return 0.0;
    }
    1.0 - (intersection as f64 / union as f64)
}

/// Find agents that best complement a given capability set.
///
/// Agents with higher Jaccard distance (more complementary capabilities)
/// are ranked higher. Results are sorted by complementarity score descending.
pub fn find_complementary(
    requester_caps: &[String],
    candidates: &[AgentCandidate],
    max_results: usize,
) -> Vec<ScoredAgent> {
    let mut scored: Vec<ScoredAgent> = candidates
        .iter()
        .map(|c| {
            let comp_score = jaccard_distance(requester_caps, &c.capabilities);
            ScoredAgent {
                candidate: c.clone(),
                final_score: comp_score,
                capability_score: 0.0,
                quality_score: 0.0,
                availability_score: 0.0,
                cost_score: 0.0,
                complementarity_score: comp_score,
            }
        })
        .collect();
    scored.sort_by(|a, b| {
        b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(max_results);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jaccard_identical_sets() {
        let a = vec!["x".to_string(), "y".to_string()];
        let b = vec!["x".to_string(), "y".to_string()];
        let d = jaccard_distance(&a, &b);
        assert!((d - 0.0).abs() < f64::EPSILON, "identical sets should have distance 0");
    }

    #[test]
    fn jaccard_disjoint_sets() {
        let a = vec!["x".to_string()];
        let b = vec!["y".to_string()];
        let d = jaccard_distance(&a, &b);
        assert!((d - 1.0).abs() < f64::EPSILON, "disjoint sets should have distance 1");
    }

    #[test]
    fn jaccard_partial_overlap() {
        let a = vec!["a".to_string(), "b".to_string()];
        let b = vec!["b".to_string(), "c".to_string()];
        let d = jaccard_distance(&a, &b);
        // intersection = {b} = 1, union = {a,b,c} = 3
        // distance = 1 - 1/3 = 2/3
        assert!((d - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn jaccard_both_empty() {
        let a: Vec<String> = vec![];
        let b: Vec<String> = vec![];
        let d = jaccard_distance(&a, &b);
        assert!((d - 0.0).abs() < f64::EPSILON, "both empty should be distance 0");
    }

    #[test]
    fn jaccard_one_empty() {
        let a = vec!["x".to_string()];
        let b: Vec<String> = vec![];
        let d = jaccard_distance(&a, &b);
        assert!((d - 1.0).abs() < f64::EPSILON, "one empty set should have distance 1");
    }

    #[test]
    fn jaccard_superset() {
        let a = vec!["x".to_string()];
        let b = vec!["x".to_string(), "y".to_string()];
        let d = jaccard_distance(&a, &b);
        // intersection = {x} = 1, union = {x,y} = 2
        // distance = 1 - 1/2 = 0.5
        assert!((d - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_duplicates_ignored() {
        let a = vec!["x".to_string(), "x".to_string()];
        let b = vec!["x".to_string()];
        let d = jaccard_distance(&a, &b);
        assert!((d - 0.0).abs() < f64::EPSILON, "duplicates should be ignored via set");
    }

    #[test]
    fn find_complementary_ranks_by_distance() {
        let requester = vec!["a".to_string(), "b".to_string()];
        let candidates = vec![
            AgentCandidate {
                did: "did:neunode:0xSame".to_string(),
                capabilities: vec!["a".to_string(), "b".to_string()],
                reputation_score: 5.0,
                stake_amount: 1000,
                availability_score: 1.0,
                latency_ms: 10,
                cost_per_unit: 1.0,
                is_online: true,
            },
            AgentCandidate {
                did: "did:neunode:0xDiff".to_string(),
                capabilities: vec!["c".to_string(), "d".to_string()],
                reputation_score: 1.0,
                stake_amount: 10,
                availability_score: 0.1,
                latency_ms: 500,
                cost_per_unit: 100.0,
                is_online: false,
            },
            AgentCandidate {
                did: "did:neunode:0xPartial".to_string(),
                capabilities: vec!["b".to_string(), "c".to_string()],
                reputation_score: 3.0,
                stake_amount: 500,
                availability_score: 0.5,
                latency_ms: 100,
                cost_per_unit: 10.0,
                is_online: true,
            },
        ];
        let results = find_complementary(&requester, &candidates, 3);
        assert_eq!(results.len(), 3);
        // Disjoint agent should be first (highest distance)
        assert_eq!(results[0].candidate.did, "did:neunode:0xDiff");
        // Partial overlap second
        assert_eq!(results[1].candidate.did, "did:neunode:0xPartial");
        // Identical last
        assert_eq!(results[2].candidate.did, "did:neunode:0xSame");
    }

    #[test]
    fn find_complementary_respects_max_results() {
        let requester = vec!["a".to_string()];
        let candidates = vec![
            AgentCandidate {
                did: "did:neunode:0x1".to_string(),
                capabilities: vec!["b".to_string()],
                reputation_score: 3.0,
                stake_amount: 100,
                availability_score: 0.5,
                latency_ms: 50,
                cost_per_unit: 10.0,
                is_online: true,
            },
            AgentCandidate {
                did: "did:neunode:0x2".to_string(),
                capabilities: vec!["c".to_string()],
                reputation_score: 3.0,
                stake_amount: 100,
                availability_score: 0.5,
                latency_ms: 50,
                cost_per_unit: 10.0,
                is_online: true,
            },
        ];
        let results = find_complementary(&requester, &candidates, 1);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn find_complementary_empty_candidates() {
        let requester = vec!["a".to_string()];
        let results = find_complementary(&requester, &[], 10);
        assert!(results.is_empty());
    }
}
