use std::collections::{HashMap, VecDeque};

use petgraph::Direction;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::dag::LineageDag;
use crate::error::{LineageError, Result};
use crate::types::{ContributionType, ModelNode};

const RECENCY_DECAY: f64 = 0.85;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct RoyaltyAllocation {
    pub contributor_did: String,
    pub weight: f64,
    pub contribution_type: ContributionType,
    pub hops: u32,
    pub amount_basis_points: u32,
}

fn type_weight(ct: &ContributionType) -> f64 {
    crate::types::type_weight(ct)
}

/// BFS backwards from the serving node, computing royalties for each ancestor.
/// The serving node itself is excluded (it's the payer).
pub fn compute_royalties(
    dag: &LineageDag,
    serving_cid: &str,
    total_basis_points: u32,
) -> Result<Vec<RoyaltyAllocation>> {
    let &start = dag
        .cid_index
        .get(serving_cid)
        .ok_or_else(|| LineageError::ModelNotFound(serving_cid.to_string()))?;

    let mut hop_dist: HashMap<petgraph::graph::NodeIndex, u32> = HashMap::new();
    let mut queue = VecDeque::new();

    for parent in dag.graph.neighbors_directed(start, Direction::Outgoing) {
        if hop_dist.insert(parent, 1).is_none() {
            queue.push_back(parent);
        }
    }

    while let Some(current) = queue.pop_front() {
        let hops = hop_dist[&current];
        for parent in dag.graph.neighbors_directed(current, Direction::Outgoing) {
            if hop_dist.insert(parent, hops + 1).is_none() {
                queue.push_back(parent);
            }
        }
    }

    if hop_dist.is_empty() {
        return Ok(vec![]);
    }

    let mut allocs: Vec<RoyaltyAllocation> = Vec::new();
    for (&idx, &hops) in &hop_dist {
        let node: &ModelNode = &dag.graph[idx];
        let tw = type_weight(&node.contribution_type);
        let raw = tw * RECENCY_DECAY.powi(hops as i32);
        allocs.push(RoyaltyAllocation {
            contributor_did: node.contributor_did.clone(),
            weight: raw,
            contribution_type: node.contribution_type.clone(),
            hops,
            amount_basis_points: 0,
        });
    }

    let total_raw: f64 = allocs.iter().map(|a| a.weight).sum();
    if total_raw > 0.0 {
        for alloc in &mut allocs {
            alloc.weight /= total_raw;
            alloc.amount_basis_points = (alloc.weight * total_basis_points as f64).round() as u32;
        }
    }

    allocs.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));

    Ok(allocs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ModelMetadata;

    fn base_node(cid: &str, did: &str) -> crate::types::ModelNode {
        crate::types::ModelNode {
            cid: cid.to_string(),
            parent_cids: vec![],
            contributor_did: did.to_string(),
            contribution_type: ContributionType::PreTraining,
            signature: vec![0u8; 64],
            created_at: 1000,
            metadata: ModelMetadata::default(),
        }
    }

    fn child_node(
        cid: &str,
        parents: Vec<&str>,
        did: &str,
        ct: ContributionType,
    ) -> crate::types::ModelNode {
        crate::types::ModelNode {
            cid: cid.to_string(),
            parent_cids: parents.iter().map(|s| s.to_string()).collect(),
            contributor_did: did.to_string(),
            contribution_type: ct,
            signature: vec![0u8; 64],
            created_at: 2000,
            metadata: ModelMetadata::default(),
        }
    }

    #[test]
    fn different_contribution_types_weighted() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:pre", "did:pre")).unwrap();
        dag.register(child_node(
            "sha256:ft",
            vec!["sha256:pre"],
            "did:ft",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:served",
            vec!["sha256:ft"],
            "did:served",
            ContributionType::PreTraining,
        ))
        .unwrap();

        let allocs = compute_royalties(&dag, "sha256:served", 1000).unwrap();
        let ft = allocs.iter().find(|a| a.contributor_did == "did:ft").unwrap();
        let pre = allocs.iter().find(|a| a.contributor_did == "did:pre").unwrap();
        assert_eq!(ft.hops, 1);
        assert_eq!(pre.hops, 2);
        // pre=PreTraining(0.30)*0.85^2=0.21675 > ft=FineTune(0.25)*0.85=0.2125
        assert!(pre.amount_basis_points >= ft.amount_basis_points);
    }

    #[test]
    fn single_parent_fine_tune() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:base", "did:a")).unwrap();
        dag.register(child_node(
            "sha256:ft",
            vec!["sha256:base"],
            "did:b",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        let allocs = compute_royalties(&dag, "sha256:ft", 1000).unwrap();
        assert_eq!(allocs.len(), 1);
        assert_eq!(allocs[0].contributor_did, "did:a");
        assert_eq!(allocs[0].hops, 1);
        assert_eq!(allocs[0].amount_basis_points, 1000);
    }

    #[test]
    fn linear_chain_decay() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:a", "did:a")).unwrap();
        dag.register(child_node(
            "sha256:b",
            vec!["sha256:a"],
            "did:b",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:c",
            vec!["sha256:b"],
            "did:c",
            ContributionType::FineTune { lora_rank: 16, lora_alpha: 32.0 },
        ))
        .unwrap();

        let allocs = compute_royalties(&dag, "sha256:c", 10000).unwrap();
        assert_eq!(allocs.len(), 2);

        let b_alloc = allocs.iter().find(|a| a.contributor_did == "did:b").unwrap();
        let a_alloc = allocs.iter().find(|a| a.contributor_did == "did:a").unwrap();
        assert_eq!(b_alloc.hops, 1);
        assert_eq!(a_alloc.hops, 2);
        // a=PreTraining(0.30)*0.85^2=0.21675 > b=FineTune(0.25)*0.85=0.2125
        assert!(a_alloc.weight > b_alloc.weight);
    }

    #[test]
    fn two_parents_merge() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:p1", "did:a")).unwrap();
        dag.register(base_node("sha256:p2", "did:b")).unwrap();
        dag.register(child_node(
            "sha256:merged",
            vec!["sha256:p1", "sha256:p2"],
            "did:c",
            ContributionType::Merge { merge_method: "slerp".to_string() },
        ))
        .unwrap();

        let allocs = compute_royalties(&dag, "sha256:merged", 1000).unwrap();
        assert_eq!(allocs.len(), 2);
        let total: u32 = allocs.iter().map(|a| a.amount_basis_points).sum();
        assert_eq!(total, 1000);
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
    fn recency_decay_per_hop() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:a", "did:a")).unwrap();
        dag.register(child_node(
            "sha256:b",
            vec!["sha256:a"],
            "did:b",
            ContributionType::PreTraining,
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:c",
            vec!["sha256:b"],
            "did:c",
            ContributionType::PreTraining,
        ))
        .unwrap();

        let allocs = compute_royalties(&dag, "sha256:c", 10000).unwrap();
        let a_alloc = allocs.iter().find(|a| a.contributor_did == "did:a").unwrap();
        let b_alloc = allocs.iter().find(|a| a.contributor_did == "did:b").unwrap();
        assert_eq!(a_alloc.hops, 2);
        assert_eq!(b_alloc.hops, 1);
        let ratio = b_alloc.amount_basis_points as f64 / a_alloc.amount_basis_points as f64;
        assert!(ratio > 1.0);
    }

    #[test]
    fn total_distributed_equals_input() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:a", "did:a")).unwrap();
        dag.register(base_node("sha256:b", "did:b")).unwrap();
        dag.register(child_node(
            "sha256:merged",
            vec!["sha256:a", "sha256:b"],
            "did:c",
            ContributionType::Merge { merge_method: "linear".to_string() },
        ))
        .unwrap();

        let total_bp = 500u32;
        let allocs = compute_royalties(&dag, "sha256:merged", total_bp).unwrap();
        let distributed: u32 = allocs.iter().map(|a| a.amount_basis_points).sum();
        assert_eq!(distributed, total_bp);
    }

    #[test]
    fn branching_dag_proportional() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:root", "did:a")).unwrap();
        dag.register(child_node(
            "sha256:left",
            vec!["sha256:root"],
            "did:b",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:right",
            vec!["sha256:root"],
            "did:c",
            ContributionType::Data { dataset_hash: "sha256:ds".to_string() },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:merged",
            vec!["sha256:left", "sha256:right"],
            "did:d",
            ContributionType::Merge { merge_method: "slerp".to_string() },
        ))
        .unwrap();

        let allocs = compute_royalties(&dag, "sha256:merged", 1000).unwrap();
        assert_eq!(allocs.len(), 3);
        let total: u32 = allocs.iter().map(|a| a.amount_basis_points).sum();
        assert!((total as i32 - 1000i32).unsigned_abs() <= 1, "total={total}");
    }

    #[test]
    fn three_level_dag() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:a", "did:a")).unwrap();
        dag.register(child_node(
            "sha256:b",
            vec!["sha256:a"],
            "did:b",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:c",
            vec!["sha256:b"],
            "did:c",
            ContributionType::RL { reward_model_cid: "sha256:rm".to_string() },
        ))
        .unwrap();

        let allocs = compute_royalties(&dag, "sha256:c", 1000).unwrap();
        assert_eq!(allocs.len(), 2);
        assert!(allocs[0].weight >= allocs[1].weight);
    }

    #[test]
    fn zero_basis_points() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:base", "did:a")).unwrap();
        dag.register(child_node(
            "sha256:ft",
            vec!["sha256:base"],
            "did:b",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();

        let allocs = compute_royalties(&dag, "sha256:ft", 0).unwrap();
        assert_eq!(allocs.len(), 1);
        assert_eq!(allocs[0].amount_basis_points, 0);
    }

    #[test]
    fn serde_roundtrip_allocation() {
        let alloc = RoyaltyAllocation {
            contributor_did: "did:neunode:agent1".to_string(),
            weight: 0.42,
            contribution_type: ContributionType::PreTraining,
            hops: 2,
            amount_basis_points: 420,
        };
        let json = serde_json::to_string(&alloc).unwrap();
        let back: RoyaltyAllocation = serde_json::from_str(&json).unwrap();
        assert_eq!(back, alloc);
    }

    #[test]
    fn ts_export_allocation() {
        use ts_rs::TS;
        let cfg = ts_rs::Config::default();
        assert!(!RoyaltyAllocation::decl(&cfg).is_empty());
    }

    #[test]
    fn serving_node_excluded() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:base", "did:a")).unwrap();
        dag.register(child_node(
            "sha256:served",
            vec!["sha256:base"],
            "did:server",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();

        let allocs = compute_royalties(&dag, "sha256:served", 1000).unwrap();
        let server_alloc = allocs.iter().find(|a| a.contributor_did == "did:server");
        assert!(server_alloc.is_none(), "serving node should not receive royalties");
    }

    #[test]
    fn model_not_found_error() {
        let dag = LineageDag::new();
        let res = compute_royalties(&dag, "sha256:ghost", 1000);
        assert!(matches!(res, Err(LineageError::ModelNotFound(_))));
    }

    #[test]
    fn sorted_by_weight_descending() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:a", "did:a")).unwrap();
        dag.register(base_node("sha256:b", "did:b")).unwrap();
        dag.register(child_node(
            "sha256:merged",
            vec!["sha256:a", "sha256:b"],
            "did:c",
            ContributionType::Merge { merge_method: "linear".to_string() },
        ))
        .unwrap();

        let allocs = compute_royalties(&dag, "sha256:merged", 1000).unwrap();
        for window in allocs.windows(2) {
            assert!(window[0].weight >= window[1].weight);
        }
    }

    #[test]
    fn multiple_contributors_same_did() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:a", "did:same")).unwrap();
        dag.register(child_node(
            "sha256:b",
            vec!["sha256:a"],
            "did:same",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();

        let allocs = compute_royalties(&dag, "sha256:b", 1000).unwrap();
        assert_eq!(allocs.len(), 1);
        assert_eq!(allocs[0].contributor_did, "did:same");
        assert_eq!(allocs[0].amount_basis_points, 1000);
    }

    #[test]
    fn recency_decay_constant() {
        assert!((RECENCY_DECAY - 0.85).abs() < f64::EPSILON);
    }
}
