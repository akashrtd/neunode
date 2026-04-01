use std::collections::{HashMap, HashSet, VecDeque};

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;

use crate::error::{LineageError, Result};
use crate::types::{ContributionType, LineageEdge, ModelNode};

pub struct LineageDag {
    pub(crate) graph: DiGraph<ModelNode, ContributionType>,
    pub(crate) cid_index: HashMap<String, NodeIndex>,
}

impl Default for LineageDag {
    fn default() -> Self {
        Self::new()
    }
}

impl LineageDag {
    pub fn new() -> Self {
        Self { graph: DiGraph::new(), cid_index: HashMap::new() }
    }

    pub fn register(&mut self, node: ModelNode) -> Result<()> {
        if self.cid_index.contains_key(&node.cid) {
            return Err(LineageError::AlreadyRegistered(node.cid.clone()));
        }
        for parent_cid in &node.parent_cids {
            if !self.cid_index.contains_key(parent_cid) {
                return Err(LineageError::ParentNotFound(parent_cid.clone()));
            }
        }

        let cid = node.cid.clone();
        let parent_cids = node.parent_cids.clone();
        let ct = node.contribution_type.clone();

        let node_idx = self.graph.add_node(node);

        for parent_cid in &parent_cids {
            let parent_idx = self.cid_index[parent_cid];
            self.graph.add_edge(node_idx, parent_idx, ct.clone());
        }

        match toposort(&self.graph, None) {
            Ok(_) => {
                self.cid_index.insert(cid, node_idx);
                Ok(())
            }
            Err(_) => {
                let edges: Vec<_> = self
                    .graph
                    .edges_directed(node_idx, Direction::Outgoing)
                    .map(|e| e.id())
                    .collect();
                for eid in edges {
                    self.graph.remove_edge(eid);
                }
                self.graph.remove_node(node_idx);
                Err(LineageError::CycleDetected(format!("adding model {cid} would create a cycle")))
            }
        }
    }

    pub fn get(&self, cid: &str) -> Option<&ModelNode> {
        self.cid_index.get(cid).map(|&idx| &self.graph[idx])
    }

    pub fn contains(&self, cid: &str) -> bool {
        self.cid_index.contains_key(cid)
    }

    pub fn parents(&self, cid: &str) -> Result<Vec<&ModelNode>> {
        let &idx =
            self.cid_index.get(cid).ok_or_else(|| LineageError::ModelNotFound(cid.to_string()))?;
        Ok(self
            .graph
            .neighbors_directed(idx, Direction::Outgoing)
            .map(|p| &self.graph[p])
            .collect())
    }

    pub fn children(&self, cid: &str) -> Result<Vec<&ModelNode>> {
        let &idx =
            self.cid_index.get(cid).ok_or_else(|| LineageError::ModelNotFound(cid.to_string()))?;
        Ok(self
            .graph
            .neighbors_directed(idx, Direction::Incoming)
            .map(|c| &self.graph[c])
            .collect())
    }

    pub fn ancestors(&self, cid: &str) -> Result<Vec<&ModelNode>> {
        let &start =
            self.cid_index.get(cid).ok_or_else(|| LineageError::ModelNotFound(cid.to_string()))?;
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        for p in self.graph.neighbors_directed(start, Direction::Outgoing) {
            if visited.insert(p) {
                queue.push_back(p);
            }
        }

        while let Some(idx) = queue.pop_front() {
            result.push(&self.graph[idx]);
            for p in self.graph.neighbors_directed(idx, Direction::Outgoing) {
                if visited.insert(p) {
                    queue.push_back(p);
                }
            }
        }

        Ok(result)
    }

    pub fn descendants(&self, cid: &str) -> Result<Vec<&ModelNode>> {
        let &start =
            self.cid_index.get(cid).ok_or_else(|| LineageError::ModelNotFound(cid.to_string()))?;
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        for c in self.graph.neighbors_directed(start, Direction::Incoming) {
            if visited.insert(c) {
                queue.push_back(c);
            }
        }

        while let Some(idx) = queue.pop_front() {
            result.push(&self.graph[idx]);
            for c in self.graph.neighbors_directed(idx, Direction::Incoming) {
                if visited.insert(c) {
                    queue.push_back(c);
                }
            }
        }

        Ok(result)
    }

    pub fn len(&self) -> usize {
        self.cid_index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cid_index.is_empty()
    }

    pub fn edges(&self) -> Vec<LineageEdge> {
        self.graph
            .edge_references()
            .map(|edge| {
                let from = edge.source();
                let to = edge.target();
                LineageEdge {
                    from_cid: self.graph[from].cid.clone(),
                    to_cid: self.graph[to].cid.clone(),
                    contribution_type: edge.weight().clone(),
                }
            })
            .collect()
    }

    /// Longest path from node to any root (0 = base model with no parents).
    pub fn lineage_depth(&self, cid: &str) -> Result<u32> {
        let &start =
            self.cid_index.get(cid).ok_or_else(|| LineageError::ModelNotFound(cid.to_string()))?;
        let mut depths: HashMap<NodeIndex, u32> = HashMap::new();
        let depth = self.compute_longest_depth(start, &mut depths);
        Ok(depth)
    }

    fn compute_longest_depth(&self, idx: NodeIndex, depths: &mut HashMap<NodeIndex, u32>) -> u32 {
        if let Some(&d) = depths.get(&idx) {
            return d;
        }
        let parents: Vec<_> = self.graph.neighbors_directed(idx, Direction::Outgoing).collect();
        let depth = if parents.is_empty() {
            0
        } else {
            parents.iter().map(|&p| self.compute_longest_depth(p, depths)).max().unwrap_or(0) + 1
        };
        depths.insert(idx, depth);
        depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ContributionType, ModelMetadata};

    fn base_node(cid: &str, did: &str) -> ModelNode {
        ModelNode {
            cid: cid.to_string(),
            parent_cids: vec![],
            contributor_did: did.to_string(),
            contribution_type: ContributionType::PreTraining,
            signature: vec![0u8; 64],
            created_at: 1000,
            metadata: ModelMetadata::default(),
        }
    }

    fn child_node(cid: &str, parents: Vec<&str>, did: &str, ct: ContributionType) -> ModelNode {
        ModelNode {
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
    fn new_empty_dag() {
        let dag = LineageDag::new();
        assert!(dag.is_empty());
        assert_eq!(dag.len(), 0);
    }

    #[test]
    fn register_base_model() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:base", "did:agent:1")).unwrap();
        assert_eq!(dag.len(), 1);
        assert!(dag.contains("sha256:base"));
    }

    #[test]
    fn register_fine_tune() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:base", "did:agent:1")).unwrap();
        dag.register(child_node(
            "sha256:ft",
            vec!["sha256:base"],
            "did:agent:2",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        assert_eq!(dag.len(), 2);
    }

    #[test]
    fn register_merge_two_parents() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:a", "did:agent:1")).unwrap();
        dag.register(base_node("sha256:b", "did:agent:2")).unwrap();
        dag.register(child_node(
            "sha256:merged",
            vec!["sha256:a", "sha256:b"],
            "did:agent:3",
            ContributionType::Merge { merge_method: "slerp".to_string() },
        ))
        .unwrap();
        assert_eq!(dag.len(), 3);
    }

    #[test]
    fn get_existing() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:x", "did:agent:1")).unwrap();
        let node = dag.get("sha256:x").unwrap();
        assert_eq!(node.cid, "sha256:x");
    }

    #[test]
    fn get_missing() {
        let dag = LineageDag::new();
        assert!(dag.get("sha256:nope").is_none());
    }

    #[test]
    fn contains_check() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:yes", "did:agent:1")).unwrap();
        assert!(dag.contains("sha256:yes"));
        assert!(!dag.contains("sha256:no"));
    }

    #[test]
    fn parents_of_base_model_empty() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:base", "did:agent:1")).unwrap();
        let parents = dag.parents("sha256:base").unwrap();
        assert!(parents.is_empty());
    }

    #[test]
    fn parents_of_fine_tune() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:base", "did:agent:1")).unwrap();
        dag.register(child_node(
            "sha256:ft",
            vec!["sha256:base"],
            "did:agent:2",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        let parents = dag.parents("sha256:ft").unwrap();
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].cid, "sha256:base");
    }

    #[test]
    fn children_of_base_model() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:base", "did:agent:1")).unwrap();
        dag.register(child_node(
            "sha256:ft1",
            vec!["sha256:base"],
            "did:agent:2",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:ft2",
            vec!["sha256:base"],
            "did:agent:3",
            ContributionType::FineTune { lora_rank: 16, lora_alpha: 32.0 },
        ))
        .unwrap();
        let children = dag.children("sha256:base").unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn ancestors_linear_chain() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:a", "did:agent:1")).unwrap();
        dag.register(child_node(
            "sha256:b",
            vec!["sha256:a"],
            "did:agent:2",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:c",
            vec!["sha256:b"],
            "did:agent:3",
            ContributionType::FineTune { lora_rank: 16, lora_alpha: 32.0 },
        ))
        .unwrap();
        let ancestors = dag.ancestors("sha256:c").unwrap();
        assert_eq!(ancestors.len(), 2);
        let cids: Vec<&str> = ancestors.iter().map(|n| n.cid.as_str()).collect();
        assert!(cids.contains(&"sha256:a"));
        assert!(cids.contains(&"sha256:b"));
    }

    #[test]
    fn ancestors_branching() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:root", "did:agent:1")).unwrap();
        dag.register(child_node(
            "sha256:left",
            vec!["sha256:root"],
            "did:agent:2",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:right",
            vec!["sha256:root"],
            "did:agent:3",
            ContributionType::FineTune { lora_rank: 16, lora_alpha: 32.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:merged",
            vec!["sha256:left", "sha256:right"],
            "did:agent:4",
            ContributionType::Merge { merge_method: "slerp".to_string() },
        ))
        .unwrap();
        let ancestors = dag.ancestors("sha256:merged").unwrap();
        assert_eq!(ancestors.len(), 3);
    }

    #[test]
    fn descendants() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:root", "did:agent:1")).unwrap();
        dag.register(child_node(
            "sha256:child1",
            vec!["sha256:root"],
            "did:agent:2",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:child2",
            vec!["sha256:root"],
            "did:agent:3",
            ContributionType::FineTune { lora_rank: 16, lora_alpha: 32.0 },
        ))
        .unwrap();
        let desc = dag.descendants("sha256:root").unwrap();
        assert_eq!(desc.len(), 2);
    }

    #[test]
    fn lineage_depth_base_model() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:base", "did:agent:1")).unwrap();
        assert_eq!(dag.lineage_depth("sha256:base").unwrap(), 0);
    }

    #[test]
    fn lineage_depth_linear_chain() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:a", "did:agent:1")).unwrap();
        dag.register(child_node(
            "sha256:b",
            vec!["sha256:a"],
            "did:agent:2",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:c",
            vec!["sha256:b"],
            "did:agent:3",
            ContributionType::FineTune { lora_rank: 16, lora_alpha: 32.0 },
        ))
        .unwrap();
        assert_eq!(dag.lineage_depth("sha256:c").unwrap(), 2);
        assert_eq!(dag.lineage_depth("sha256:b").unwrap(), 1);
    }

    #[test]
    fn lineage_depth_branching() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:root", "did:agent:1")).unwrap();
        dag.register(child_node(
            "sha256:deep",
            vec!["sha256:root"],
            "did:agent:2",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:deeper",
            vec!["sha256:deep"],
            "did:agent:3",
            ContributionType::FineTune { lora_rank: 16, lora_alpha: 32.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:merged",
            vec!["sha256:deeper", "sha256:root"],
            "did:agent:4",
            ContributionType::Merge { merge_method: "slerp".to_string() },
        ))
        .unwrap();
        // merged → deeper (depth 2) → deep (depth 1) → root (depth 0)
        // merged → root (depth 1) — shorter path
        // longest path: 3 (merged→deeper→deep→root)
        assert_eq!(dag.lineage_depth("sha256:merged").unwrap(), 3);
    }

    #[test]
    fn cycle_detection_simple() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:a", "did:agent:1")).unwrap();
        let res = dag.register(child_node(
            "sha256:b",
            vec!["sha256:a"],
            "did:agent:2",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ));
        assert!(res.is_ok());

        // Try to register a with parent b — creates a→b→a cycle
        // But a already exists, so it's AlreadyRegistered
        // Need a third node to create an indirect cycle
        let res2 = dag.register(ModelNode {
            cid: "sha256:a".to_string(),
            parent_cids: vec!["sha256:b".to_string()],
            contributor_did: "did:agent:1".to_string(),
            contribution_type: ContributionType::PreTraining,
            signature: vec![0u8; 64],
            created_at: 3000,
            metadata: ModelMetadata::default(),
        });
        assert!(matches!(res2, Err(LineageError::AlreadyRegistered(_))));
    }

    #[test]
    fn cycle_detection_indirect() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:a", "did:agent:1")).unwrap();
        dag.register(child_node(
            "sha256:b",
            vec!["sha256:a"],
            "did:agent:2",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:c",
            vec!["sha256:b"],
            "did:agent:3",
            ContributionType::FineTune { lora_rank: 16, lora_alpha: 32.0 },
        ))
        .unwrap();

        let res = dag.register(child_node(
            "sha256:d",
            vec!["sha256:c"],
            "did:agent:4",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ));
        assert!(res.is_ok());
    }

    #[test]
    fn duplicate_cid_rejected() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:dup", "did:agent:1")).unwrap();
        let res = dag.register(base_node("sha256:dup", "did:agent:2"));
        assert!(matches!(res, Err(LineageError::AlreadyRegistered(_))));
    }

    #[test]
    fn parent_not_found() {
        let mut dag = LineageDag::new();
        let res = dag.register(child_node(
            "sha256:child",
            vec!["sha256:ghost"],
            "did:agent:1",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ));
        assert!(matches!(res, Err(LineageError::ParentNotFound(_))));
    }

    #[test]
    fn edges_list() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:a", "did:agent:1")).unwrap();
        dag.register(base_node("sha256:b", "did:agent:2")).unwrap();
        dag.register(child_node(
            "sha256:c",
            vec!["sha256:a", "sha256:b"],
            "did:agent:3",
            ContributionType::Merge { merge_method: "linear".to_string() },
        ))
        .unwrap();
        let edges = dag.edges();
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn register_with_metadata() {
        let mut dag = LineageDag::new();
        let mut node = base_node("sha256:base", "did:agent:1");
        node.metadata = ModelMetadata {
            dataset_hash: Some("sha256:ds".to_string()),
            training_duration_secs: Some(3600.0),
            ..Default::default()
        };
        dag.register(node).unwrap();
        let stored = dag.get("sha256:base").unwrap();
        assert_eq!(stored.metadata.dataset_hash, Some("sha256:ds".to_string()));
    }

    #[test]
    fn three_level_dag() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:root", "did:a")).unwrap();
        dag.register(child_node(
            "sha256:l1",
            vec!["sha256:root"],
            "did:b",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:r1",
            vec!["sha256:root"],
            "did:c",
            ContributionType::FineTune { lora_rank: 16, lora_alpha: 32.0 },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:l2",
            vec!["sha256:l1"],
            "did:d",
            ContributionType::RL { reward_model_cid: "sha256:rm".to_string() },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:r2",
            vec!["sha256:r1"],
            "did:e",
            ContributionType::Data { dataset_hash: "sha256:ds".to_string() },
        ))
        .unwrap();
        dag.register(child_node(
            "sha256:merged",
            vec!["sha256:l2", "sha256:r2"],
            "did:f",
            ContributionType::Merge { merge_method: "slerp".to_string() },
        ))
        .unwrap();
        assert_eq!(dag.len(), 6);
        assert_eq!(dag.edges().len(), 6);
        assert_eq!(dag.lineage_depth("sha256:merged").unwrap(), 3);
    }

    #[test]
    fn many_models_stress() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:m0", "did:agent:0")).unwrap();
        for i in 1..=50 {
            let parent = format!("sha256:m{}", i - 1);
            let cid = format!("sha256:m{i}");
            dag.register(child_node(
                &cid,
                vec![&parent],
                &format!("did:agent:{i}"),
                ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
            ))
            .unwrap();
        }
        assert_eq!(dag.len(), 51);
        assert_eq!(dag.lineage_depth("sha256:m50").unwrap(), 50);
    }

    #[test]
    fn model_not_found_errors() {
        let dag = LineageDag::new();
        assert!(matches!(dag.parents("sha256:ghost"), Err(LineageError::ModelNotFound(_))));
        assert!(matches!(dag.children("sha256:ghost"), Err(LineageError::ModelNotFound(_))));
        assert!(matches!(dag.ancestors("sha256:ghost"), Err(LineageError::ModelNotFound(_))));
        assert!(matches!(dag.descendants("sha256:ghost"), Err(LineageError::ModelNotFound(_))));
        assert!(matches!(dag.lineage_depth("sha256:ghost"), Err(LineageError::ModelNotFound(_))));
    }

    #[test]
    fn ancestors_of_base_model_empty() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:base", "did:a")).unwrap();
        let ancestors = dag.ancestors("sha256:base").unwrap();
        assert!(ancestors.is_empty());
    }

    #[test]
    fn descendants_of_leaf_empty() {
        let mut dag = LineageDag::new();
        dag.register(base_node("sha256:base", "did:a")).unwrap();
        dag.register(child_node(
            "sha256:leaf",
            vec!["sha256:base"],
            "did:b",
            ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 },
        ))
        .unwrap();
        let desc = dag.descendants("sha256:leaf").unwrap();
        assert!(desc.is_empty());
    }
}
