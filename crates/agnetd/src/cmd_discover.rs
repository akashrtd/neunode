use anyhow::Result;
use neunode_discovery::{
    compute_score, find_capability_gaps, find_complementary, search, AgentCandidate,
    DiscoveryRequest, ScoringWeights,
};

use crate::cli::{DiscoverCommands, GlobalArgs};
use crate::output::OutputWriter;
use crate::state::AppState;

pub fn execute(cmd: &DiscoverCommands, args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(args.output);
    match cmd {
        DiscoverCommands::Search { capabilities, min_reputation, max_cost, online_only, limit } => {
            handle_search(
                capabilities,
                *min_reputation,
                *max_cost,
                *online_only,
                *limit,
                &writer,
                state,
            )
        }
        DiscoverCommands::Complement { capabilities, limit } => {
            handle_complement(capabilities, *limit, &writer, state)
        }
        DiscoverCommands::Gaps => handle_gaps(&writer, state),
        DiscoverCommands::Score { agent, capabilities } => {
            handle_score(agent, capabilities, &writer, state)
        }
        DiscoverCommands::Weights => handle_weights(&writer),
    }
}

fn parse_comma_list(s: &str) -> Vec<String> {
    s.split(',').map(|c| c.trim().to_string()).filter(|c| !c.is_empty()).collect()
}

fn gather_candidates_from_kg(state: &AppState) -> Result<Vec<(String, Vec<String>)>> {
    let db = state.db();
    let dict = neunode_knowledge::StringDictionary::new(db);
    let engine = neunode_knowledge::QueryEngine::new(db, &dict);

    let pred_uri = neunode_knowledge::nn(neunode_knowledge::PRED_HAS_CAPABILITY);
    let pred_hash = neunode_knowledge::StringDictionary::hash(&pred_uri);
    let pattern =
        neunode_knowledge::QueryPattern { predicate: Some(pred_hash), ..Default::default() };
    let results = engine.query(&pattern)?;

    let mut agent_caps: Vec<(String, Vec<String>)> = Vec::new();
    for row in &results {
        let (agent, cap) = (&row.subject, &row.object);
        if let Some(entry) = agent_caps.iter_mut().find(|(a, _)| a == agent) {
            entry.1.push(cap.clone());
        } else {
            agent_caps.push((agent.clone(), vec![cap.clone()]));
        }
    }
    Ok(agent_caps)
}

fn build_candidates(agent_caps: &[(String, Vec<String>)]) -> Vec<AgentCandidate> {
    agent_caps
        .iter()
        .enumerate()
        .map(|(i, (did, caps))| AgentCandidate {
            did: did.clone(),
            capabilities: caps.clone(),
            reputation_score: 3.0 + (i as f64 * 0.2).min(2.0),
            stake_amount: 500 + (i as u64) * 100,
            availability_score: 0.8 + (i as f64 * 0.02).min(0.2),
            latency_ms: 30 + (i as u32) * 10,
            cost_per_unit: 5.0 + (i as f64) * 2.0,
            is_online: i % 3 != 2,
        })
        .collect()
}

fn handle_search(
    capabilities: &str,
    min_reputation: f64,
    max_cost: Option<f64>,
    online_only: bool,
    limit: usize,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    let required = parse_comma_list(capabilities);
    if required.is_empty() {
        anyhow::bail!("capabilities cannot be empty");
    }
    if !(0.0..=5.0).contains(&min_reputation) {
        anyhow::bail!("min-reputation must be between 0.0 and 5.0");
    }

    let agent_caps = gather_candidates_from_kg(state)?;
    let candidates = build_candidates(&agent_caps);
    let request = DiscoveryRequest {
        required_capabilities: required,
        min_reputation: if min_reputation > 0.0 { Some(min_reputation) } else { None },
        max_cost_per_unit: max_cost,
        must_be_online: online_only,
        max_results: limit,
        requester_capabilities: vec![],
    };
    let weights = ScoringWeights::default();
    let results = search(&candidates, &request, &weights)?;

    let headers = ["DID", "Score", "Caps", "Rep", "Cost", "Online"];
    let rows: Vec<Vec<String>> = results
        .iter()
        .map(|r| {
            vec![
                r.candidate.did.clone(),
                format!("{:.3}", r.final_score),
                r.candidate.capabilities.join(", "),
                format!("{:.1}", r.candidate.reputation_score),
                format!("{:.1}", r.candidate.cost_per_unit),
                if r.candidate.is_online { "yes" } else { "no" }.to_string(),
            ]
        })
        .collect();
    writer.write_table(&headers, &rows);
    writer.write_status(&format!("Found {} agents", results.len()));
    Ok(())
}

fn handle_complement(
    capabilities: &str,
    limit: usize,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    let requester_caps = parse_comma_list(capabilities);
    if requester_caps.is_empty() {
        anyhow::bail!("capabilities cannot be empty");
    }

    let agent_caps = gather_candidates_from_kg(state)?;
    let candidates = build_candidates(&agent_caps);
    let results = find_complementary(&requester_caps, &candidates, limit);

    if results.is_empty() {
        writer.write_status("No agents available for complement analysis");
        return Ok(());
    }

    let headers = ["DID", "Distance", "Capabilities"];
    let rows: Vec<Vec<String>> = results
        .iter()
        .map(|r| {
            vec![
                r.candidate.did.clone(),
                format!("{:.3}", r.complementarity_score),
                r.candidate.capabilities.join(", "),
            ]
        })
        .collect();
    writer.write_table(&headers, &rows);
    writer.write_status(&format!("Found {} complementary agents", results.len()));
    Ok(())
}

fn handle_gaps(writer: &OutputWriter, state: &AppState) -> Result<()> {
    let db = state.db();
    let dict = neunode_knowledge::StringDictionary::new(db);
    let engine = neunode_knowledge::QueryEngine::new(db, &dict);

    let registered: Vec<String> =
        neunode_knowledge::all_classes().iter().map(|c| neunode_knowledge::nn(c)).collect();

    let cap_pred_hash = neunode_knowledge::StringDictionary::hash(&neunode_knowledge::nn(
        neunode_knowledge::PRED_HAS_CAPABILITY,
    ));
    let cap_pattern =
        neunode_knowledge::QueryPattern { predicate: Some(cap_pred_hash), ..Default::default() };
    let cap_results = engine.query(&cap_pattern)?;
    let agents_with_caps: Vec<(String, Vec<String>)> = {
        let mut map: Vec<(String, Vec<String>)> = Vec::new();
        for row in &cap_results {
            if let Some(entry) = map.iter_mut().find(|(a, _)| a == &row.subject) {
                entry.1.push(row.object.clone());
            } else {
                map.push((row.subject.clone(), vec![row.object.clone()]));
            }
        }
        map
    };

    let req_pred_hash = neunode_knowledge::StringDictionary::hash(&neunode_knowledge::nn(
        neunode_knowledge::PRED_REQUIRES_CAPABILITY,
    ));
    let bounty_pattern =
        neunode_knowledge::QueryPattern { predicate: Some(req_pred_hash), ..Default::default() };
    let bounty_results = engine.query(&bounty_pattern)?;
    let bounty_reqs: Vec<(String, Vec<String>)> = {
        let mut map: Vec<(String, Vec<String>)> = Vec::new();
        for row in &bounty_results {
            if let Some(entry) = map.iter_mut().find(|(b, _)| b == &row.subject) {
                entry.1.push(row.object.clone());
            } else {
                map.push((row.subject.clone(), vec![row.object.clone()]));
            }
        }
        map
    };

    let gaps = find_capability_gaps(&registered, &agents_with_caps, &bounty_reqs);

    if gaps.is_empty() {
        writer.write_status("No capability gaps found");
    } else {
        let headers = ["Capability", "Demand"];
        let rows: Vec<Vec<String>> = gaps
            .iter()
            .map(|g| vec![g.capability_uri.clone(), g.demand_count.to_string()])
            .collect();
        writer.write_table(&headers, &rows);
        writer.write_status(&format!("Found {} capability gaps", gaps.len()));
    }
    Ok(())
}

fn handle_score(
    agent: &str,
    capabilities: &str,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    if agent.is_empty() {
        anyhow::bail!("agent DID cannot be empty");
    }
    let required = parse_comma_list(capabilities);
    if required.is_empty() {
        anyhow::bail!("capabilities cannot be empty");
    }

    let agent_caps = gather_candidates_from_kg(state)?;
    let candidates = build_candidates(&agent_caps);
    let target =
        candidates.iter().find(|c| c.did == agent).cloned().unwrap_or_else(|| AgentCandidate {
            did: agent.to_string(),
            capabilities: vec![],
            reputation_score: 0.0,
            stake_amount: 0,
            availability_score: 0.0,
            latency_ms: 1000,
            cost_per_unit: f64::MAX,
            is_online: false,
        });

    let request = DiscoveryRequest {
        required_capabilities: required,
        min_reputation: None,
        max_cost_per_unit: None,
        must_be_online: false,
        max_results: 1,
        requester_capabilities: vec![],
    };
    let weights = ScoringWeights::default();
    let scored = compute_score(&target, &request, &candidates, &weights);

    let out = serde_json::json!({
        "did": scored.candidate.did,
        "final_score": format!("{:.4}", scored.final_score),
        "capability": format!("{:.4}", scored.capability_score),
        "quality": format!("{:.4}", scored.quality_score),
        "availability": format!("{:.4}", scored.availability_score),
        "cost_efficiency": format!("{:.4}", scored.cost_score),
        "complementarity": format!("{:.4}", scored.complementarity_score),
    });
    writer.write_json(&out);
    writer.write_status(&format!("Score for {}: {:.4}", agent, scored.final_score));
    Ok(())
}

fn handle_weights(writer: &OutputWriter) -> Result<()> {
    let w = ScoringWeights::default();
    let headers = ["Factor", "Weight", "Pct"];
    let factors = [
        ("capability_match", w.capability_match),
        ("quality", w.quality),
        ("availability", w.availability),
        ("cost_efficiency", w.cost_efficiency),
        ("complementarity", w.complementarity),
    ];
    let rows: Vec<Vec<String>> = factors
        .iter()
        .map(|(name, weight)| {
            vec![name.to_string(), format!("{:.2}", weight), format!("{:.0}%", weight * 100.0)]
        })
        .collect();
    writer.write_table(&headers, &rows);
    writer.write_status("Current discovery scoring weights");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{human_writer, json_writer as test_writer, test_state};

    fn make_test_candidates() -> Vec<AgentCandidate> {
        vec![
            AgentCandidate {
                did: "did:neunode:0xA".to_string(),
                capabilities: vec!["inference:llm".to_string(), "training:lora".to_string()],
                reputation_score: 4.5,
                stake_amount: 2000,
                availability_score: 0.95,
                latency_ms: 30,
                cost_per_unit: 8.0,
                is_online: true,
            },
            AgentCandidate {
                did: "did:neunode:0xB".to_string(),
                capabilities: vec!["inference:llm".to_string()],
                reputation_score: 3.0,
                stake_amount: 500,
                availability_score: 0.80,
                latency_ms: 100,
                cost_per_unit: 12.0,
                is_online: true,
            },
            AgentCandidate {
                did: "did:neunode:0xC".to_string(),
                capabilities: vec!["training:lora".to_string(), "training:diloco".to_string()],
                reputation_score: 2.5,
                stake_amount: 300,
                availability_score: 0.70,
                latency_ms: 200,
                cost_per_unit: 5.0,
                is_online: false,
            },
        ]
    }

    #[test]
    fn parse_comma_list_basic() {
        let list = parse_comma_list("a,b,c");
        assert_eq!(list, vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_comma_list_trims_whitespace() {
        let list = parse_comma_list(" a , b , c ");
        assert_eq!(list, vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_comma_list_empty_string() {
        let list = parse_comma_list("");
        assert!(list.is_empty());
    }

    #[test]
    fn parse_comma_list_single() {
        let list = parse_comma_list("inference:llm");
        assert_eq!(list, vec!["inference:llm"]);
    }

    #[test]
    fn parse_comma_list_filters_empty_segments() {
        let list = parse_comma_list("a,,b,,");
        assert_eq!(list, vec!["a", "b"]);
    }

    #[test]
    fn search_filters_by_capability() {
        let candidates = make_test_candidates();
        let request = DiscoveryRequest {
            required_capabilities: vec!["inference:llm".to_string()],
            min_reputation: None,
            max_cost_per_unit: None,
            must_be_online: false,
            max_results: 10,
            requester_capabilities: vec![],
        };
        let weights = ScoringWeights::default();
        let results = search(&candidates, &request, &weights).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .all(|r| { r.candidate.capabilities.contains(&"inference:llm".to_string()) }));
    }

    #[test]
    fn search_filters_by_reputation() {
        let candidates = make_test_candidates();
        let request = DiscoveryRequest {
            required_capabilities: vec!["inference:llm".to_string()],
            min_reputation: Some(4.0),
            max_cost_per_unit: None,
            must_be_online: false,
            max_results: 10,
            requester_capabilities: vec![],
        };
        let weights = ScoringWeights::default();
        let results = search(&candidates, &request, &weights).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].candidate.did, "did:neunode:0xA");
    }

    #[test]
    fn search_filters_by_online() {
        let candidates = make_test_candidates();
        let request = DiscoveryRequest {
            required_capabilities: vec![],
            min_reputation: None,
            max_cost_per_unit: None,
            must_be_online: true,
            max_results: 10,
            requester_capabilities: vec![],
        };
        let weights = ScoringWeights::default();
        let results = search(&candidates, &request, &weights).unwrap();
        assert!(results.iter().all(|r| r.candidate.is_online));
    }

    #[test]
    fn search_respects_limit() {
        let candidates = make_test_candidates();
        let request = DiscoveryRequest {
            required_capabilities: vec![],
            min_reputation: None,
            max_cost_per_unit: None,
            must_be_online: false,
            max_results: 1,
            requester_capabilities: vec![],
        };
        let weights = ScoringWeights::default();
        let results = search(&candidates, &request, &weights).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_empty_pool_errors() {
        let candidates: Vec<AgentCandidate> = vec![];
        let request = DiscoveryRequest {
            required_capabilities: vec!["a".to_string()],
            min_reputation: None,
            max_cost_per_unit: None,
            must_be_online: false,
            max_results: 10,
            requester_capabilities: vec![],
        };
        let weights = ScoringWeights::default();
        let result = search(&candidates, &request, &weights);
        assert!(result.is_err());
    }

    #[test]
    fn search_no_matches_errors() {
        let candidates = make_test_candidates();
        let request = DiscoveryRequest {
            required_capabilities: vec!["nonexistent".to_string()],
            min_reputation: None,
            max_cost_per_unit: None,
            must_be_online: false,
            max_results: 10,
            requester_capabilities: vec![],
        };
        let weights = ScoringWeights::default();
        let result = search(&candidates, &request, &weights);
        assert!(result.is_err());
    }

    #[test]
    fn complement_ranks_by_jaccard_distance() {
        let candidates = make_test_candidates();
        let requester = vec!["inference:llm".to_string()];
        let results = find_complementary(&requester, &candidates, 10);
        assert_eq!(results.len(), 3);
        assert!(results[0].complementarity_score >= results[1].complementarity_score);
    }

    #[test]
    fn complement_respects_limit() {
        let candidates = make_test_candidates();
        let requester = vec!["a".to_string()];
        let results = find_complementary(&requester, &candidates, 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn complement_empty_candidates() {
        let requester = vec!["a".to_string()];
        let results = find_complementary(&requester, &[], 10);
        assert!(results.is_empty());
    }

    #[test]
    fn gap_analysis_finds_missing() {
        let registered = vec![
            "inference:llm".to_string(),
            "training:lora".to_string(),
            "training:diloco".to_string(),
        ];
        let agents = vec![("did:1".to_string(), vec!["inference:llm".to_string()])];
        let bounties: Vec<(String, Vec<String>)> =
            vec![("b1".to_string(), vec!["training:lora".to_string()])];
        let gaps = find_capability_gaps(&registered, &agents, &bounties);
        assert_eq!(gaps.len(), 2);
        assert_eq!(gaps[0].capability_uri, "training:lora");
        assert_eq!(gaps[0].demand_count, 1);
        assert_eq!(gaps[1].capability_uri, "training:diloco");
        assert_eq!(gaps[1].demand_count, 0);
    }

    #[test]
    fn gap_analysis_no_gaps() {
        let registered = vec!["a".to_string()];
        let agents = vec![("did:1".to_string(), vec!["a".to_string()])];
        let bounties: Vec<(String, Vec<String>)> = vec![];
        let gaps = find_capability_gaps(&registered, &agents, &bounties);
        assert!(gaps.is_empty());
    }

    #[test]
    fn gap_analysis_empty_registered() {
        let registered: Vec<String> = vec![];
        let agents: Vec<(String, Vec<String>)> = vec![];
        let bounties: Vec<(String, Vec<String>)> = vec![];
        let gaps = find_capability_gaps(&registered, &agents, &bounties);
        assert!(gaps.is_empty());
    }

    #[test]
    fn compute_score_returns_all_factors() {
        let candidates = make_test_candidates();
        let request = DiscoveryRequest {
            required_capabilities: vec!["inference:llm".to_string()],
            min_reputation: None,
            max_cost_per_unit: None,
            must_be_online: false,
            max_results: 10,
            requester_capabilities: vec![],
        };
        let weights = ScoringWeights::default();
        let scored = compute_score(&candidates[0], &request, &candidates, &weights);
        assert!(scored.final_score > 0.0);
        assert!((0.0..=1.0).contains(&scored.capability_score));
        assert!((0.0..=1.0).contains(&scored.quality_score));
        assert!((0.0..=1.0).contains(&scored.availability_score));
        assert!((0.0..=1.0).contains(&scored.cost_score));
        assert!((0.0..=1.0).contains(&scored.complementarity_score));
    }

    #[test]
    fn handle_weights_displays_table() {
        let writer = test_writer();
        handle_weights(&writer).unwrap();
    }

    #[test]
    fn handle_weights_human_format() {
        let writer = human_writer();
        handle_weights(&writer).unwrap();
    }

    #[test]
    fn handle_search_empty_caps_fails() {
        let state = test_state();
        let writer = test_writer();
        let result = handle_search("", 0.0, None, false, 10, &writer, &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn handle_search_bad_reputation_fails() {
        let state = test_state();
        let writer = test_writer();
        let result = handle_search("inference:llm", 6.0, None, false, 10, &writer, &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("0.0 and 5.0"));
    }

    #[test]
    fn handle_search_negative_reputation_fails() {
        let state = test_state();
        let writer = test_writer();
        let result = handle_search("inference:llm", -1.0, None, false, 10, &writer, &state);
        assert!(result.is_err());
    }

    #[test]
    fn handle_complement_empty_caps_fails() {
        let state = test_state();
        let writer = test_writer();
        let result = handle_complement("", 10, &writer, &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn handle_score_empty_agent_fails() {
        let state = test_state();
        let writer = test_writer();
        let result = handle_score("", "inference:llm", &writer, &state);
        assert!(result.is_err());
    }

    #[test]
    fn handle_score_empty_caps_fails() {
        let state = test_state();
        let writer = test_writer();
        let result = handle_score("did:neunode:0xABC", "", &writer, &state);
        assert!(result.is_err());
    }

    #[test]
    fn handle_gaps_no_kg_data() {
        let state = test_state();
        let writer = test_writer();
        let result = handle_gaps(&writer, &state);
        assert!(result.is_ok());
    }

    #[test]
    fn build_candidates_from_empty() {
        let agent_caps: Vec<(String, Vec<String>)> = vec![];
        let candidates = build_candidates(&agent_caps);
        assert!(candidates.is_empty());
    }

    #[test]
    fn build_candidates_assigns_sane_defaults() {
        let agent_caps = vec![
            ("did:1".to_string(), vec!["a".to_string(), "b".to_string()]),
            ("did:2".to_string(), vec!["c".to_string()]),
        ];
        let candidates = build_candidates(&agent_caps);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].did, "did:1");
        assert!(candidates[0].reputation_score > 0.0);
        assert!(candidates[0].is_online);
    }

    #[test]
    fn search_with_cost_filter() {
        let candidates = make_test_candidates();
        let request = DiscoveryRequest {
            required_capabilities: vec!["inference:llm".to_string()],
            min_reputation: None,
            max_cost_per_unit: Some(10.0),
            must_be_online: false,
            max_results: 10,
            requester_capabilities: vec![],
        };
        let weights = ScoringWeights::default();
        let results = search(&candidates, &request, &weights).unwrap();
        assert!(results.iter().all(|r| r.candidate.cost_per_unit <= 10.0));
    }

    #[test]
    fn jaccard_distance_computed_correctly() {
        let dist = neunode_discovery::jaccard_distance(
            &["a".to_string(), "b".to_string()],
            &["b".to_string(), "c".to_string()],
        );
        assert!((dist - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn gather_candidates_empty_db() {
        let state = test_state();
        let result = gather_candidates_from_kg(&state);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
