use crate::types::CapabilityGap;

/// Find capability gaps — registered capabilities with zero available providers.
///
/// A gap exists when a capability appears in `registered_capabilities` but
/// no agent in `agents_with_capabilities` advertises it. The `demand_count`
/// for each gap is computed from how many bounties require it.
pub fn find_capability_gaps(
    registered_capabilities: &[String],
    agents_with_capabilities: &[(String, Vec<String>)],
    bounty_requirements: &[(String, Vec<String>)],
) -> Vec<CapabilityGap> {
    // Collect all capabilities that at least one agent provides
    let provided: std::collections::HashSet<&str> = agents_with_capabilities
        .iter()
        .flat_map(|(_, caps)| caps.iter().map(|s| s.as_str()))
        .collect();

    // Count bounty demand for each capability
    let mut demand_counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for (_, req_caps) in bounty_requirements {
        for cap in req_caps {
            *demand_counts.entry(cap.as_str()).or_insert(0) += 1;
        }
    }

    // Find registered capabilities with no providers
    let mut gaps: Vec<CapabilityGap> = registered_capabilities
        .iter()
        .filter(|cap| !provided.contains(cap.as_str()))
        .map(|cap| CapabilityGap {
            capability_uri: cap.clone(),
            demand_count: demand_counts.get(cap.as_str()).copied().unwrap_or(0),
        })
        .collect();

    // Sort by demand descending, then alphabetically for determinism
    gaps.sort_by(|a, b| {
        b.demand_count.cmp(&a.demand_count).then_with(|| a.capability_uri.cmp(&b.capability_uri))
    });

    gaps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_gaps_when_all_provided() {
        let registered = vec!["a".to_string(), "b".to_string()];
        let agents = vec![
            ("did:1".to_string(), vec!["a".to_string()]),
            ("did:2".to_string(), vec!["b".to_string()]),
        ];
        let bounties: Vec<(String, Vec<String>)> = vec![];
        let gaps = find_capability_gaps(&registered, &agents, &bounties);
        assert!(gaps.is_empty());
    }

    #[test]
    fn finds_gap_with_demand() {
        let registered = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let agents = vec![("did:1".to_string(), vec!["a".to_string()])];
        let bounties = vec![
            ("bounty1".to_string(), vec!["b".to_string()]),
            ("bounty2".to_string(), vec!["b".to_string()]),
            ("bounty3".to_string(), vec!["c".to_string()]),
        ];
        let gaps = find_capability_gaps(&registered, &agents, &bounties);
        assert_eq!(gaps.len(), 2);
        // "b" has demand 2, "c" has demand 1 — sorted by demand desc
        assert_eq!(gaps[0].capability_uri, "b");
        assert_eq!(gaps[0].demand_count, 2);
        assert_eq!(gaps[1].capability_uri, "c");
        assert_eq!(gaps[1].demand_count, 1);
    }

    #[test]
    fn gap_with_zero_demand() {
        let registered = vec!["orphan".to_string()];
        let agents: Vec<(String, Vec<String>)> = vec![];
        let bounties: Vec<(String, Vec<String>)> = vec![];
        let gaps = find_capability_gaps(&registered, &agents, &bounties);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].capability_uri, "orphan");
        assert_eq!(gaps[0].demand_count, 0);
    }

    #[test]
    fn empty_registered_no_gaps() {
        let registered: Vec<String> = vec![];
        let agents = vec![("did:1".to_string(), vec!["a".to_string()])];
        let bounties: Vec<(String, Vec<String>)> = vec![];
        let gaps = find_capability_gaps(&registered, &agents, &bounties);
        assert!(gaps.is_empty());
    }

    #[test]
    fn empty_agents_all_gaps() {
        let registered = vec!["a".to_string(), "b".to_string()];
        let agents: Vec<(String, Vec<String>)> = vec![];
        let bounties: Vec<(String, Vec<String>)> = vec![];
        let gaps = find_capability_gaps(&registered, &agents, &bounties);
        assert_eq!(gaps.len(), 2);
    }

    #[test]
    fn duplicate_capabilities_in_agents_no_effect() {
        let registered = vec!["a".to_string(), "b".to_string()];
        let agents = vec![
            ("did:1".to_string(), vec!["a".to_string()]),
            ("did:2".to_string(), vec!["a".to_string()]),
        ];
        let bounties: Vec<(String, Vec<String>)> = vec![];
        let gaps = find_capability_gaps(&registered, &agents, &bounties);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].capability_uri, "b");
    }

    #[test]
    fn gap_sorted_by_demand_descending() {
        let registered =
            vec!["low_demand".to_string(), "high_demand".to_string(), "med_demand".to_string()];
        let agents: Vec<(String, Vec<String>)> = vec![];
        let bounties = vec![
            ("b1".to_string(), vec!["low_demand".to_string()]),
            ("b2".to_string(), vec!["high_demand".to_string()]),
            ("b3".to_string(), vec!["high_demand".to_string()]),
            ("b4".to_string(), vec!["high_demand".to_string()]),
            ("b5".to_string(), vec!["med_demand".to_string()]),
            ("b6".to_string(), vec!["med_demand".to_string()]),
        ];
        let gaps = find_capability_gaps(&registered, &agents, &bounties);
        assert_eq!(gaps[0].capability_uri, "high_demand");
        assert_eq!(gaps[0].demand_count, 3);
        assert_eq!(gaps[1].capability_uri, "med_demand");
        assert_eq!(gaps[1].demand_count, 2);
        assert_eq!(gaps[2].capability_uri, "low_demand");
        assert_eq!(gaps[2].demand_count, 1);
    }

    #[test]
    fn capability_not_registered_not_counted() {
        let registered = vec!["a".to_string()];
        let agents: Vec<(String, Vec<String>)> = vec![];
        let bounties = vec![("b1".to_string(), vec!["unregistered".to_string()])];
        let gaps = find_capability_gaps(&registered, &agents, &bounties);
        // "unregistered" is not in registered_capabilities, so not a gap
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].capability_uri, "a");
        assert_eq!(gaps[0].demand_count, 0);
    }
}
