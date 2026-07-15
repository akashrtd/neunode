use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use neunode_discovery::{
    compute_score, find_capability_gaps, find_complementary, search, AgentCandidate,
    DiscoveryRequest, ScoringWeights,
};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::state::ApiState;
use crate::api::types;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    capabilities: String,
    #[serde(default)]
    min_reputation: f64,
    max_cost: Option<f64>,
    #[serde(default)]
    online_only: bool,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
pub struct ComplementQuery {
    capabilities: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
pub struct ScoreQuery {
    agent: String,
    capabilities: String,
}

fn default_limit() -> usize {
    20
}

pub async fn search_agents(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let required = parse_capabilities(&query.capabilities)?;
    if !(0.0..=5.0).contains(&query.min_reputation) {
        return Err(ApiError::BadRequest("minReputation must be between 0.0 and 5.0".into()));
    }
    let candidates = candidates(&state)?;
    if candidates.is_empty() {
        return Ok(types::ok(serde_json::json!({ "data": [] })));
    }
    let request = DiscoveryRequest {
        required_capabilities: required,
        min_reputation: (query.min_reputation > 0.0).then_some(query.min_reputation),
        max_cost_per_unit: query.max_cost,
        must_be_online: query.online_only,
        max_results: query.limit,
        requester_capabilities: Vec::new(),
    };
    let results = search(&candidates, &request, &ScoringWeights::default())
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(types::ok(serde_json::json!({ "data": results })))
}

pub async fn complement_agents(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<ComplementQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let required = parse_capabilities(&query.capabilities)?;
    let results = find_complementary(&required, &candidates(&state)?, query.limit);
    Ok(types::ok(serde_json::json!({ "data": results })))
}

pub async fn capability_gaps(
    State(state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    let dict = neunode_knowledge::StringDictionary::new(&state.db);
    let engine = neunode_knowledge::QueryEngine::new(&state.db, &dict);
    let registered = neunode_knowledge::all_classes()
        .iter()
        .map(|class| neunode_knowledge::nn(class))
        .collect::<Vec<_>>();
    let agents = grouped_objects(&engine, neunode_knowledge::PRED_HAS_CAPABILITY)?;
    let bounties = grouped_objects(&engine, neunode_knowledge::PRED_REQUIRES_CAPABILITY)?;
    let gaps = find_capability_gaps(&registered, &agents, &bounties);
    Ok(types::ok(serde_json::json!({ "data": gaps })))
}

pub async fn score_agent(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<ScoreQuery>,
) -> Result<impl IntoResponse, ApiError> {
    if query.agent.is_empty() {
        return Err(ApiError::BadRequest("agent DID cannot be empty".into()));
    }
    let required = parse_capabilities(&query.capabilities)?;
    let candidates = candidates(&state)?;
    let target = candidates
        .iter()
        .find(|candidate| candidate.did == query.agent)
        .cloned()
        .unwrap_or_else(|| unavailable_candidate(query.agent.clone()));
    let request = DiscoveryRequest {
        required_capabilities: required,
        min_reputation: None,
        max_cost_per_unit: None,
        must_be_online: false,
        max_results: 1,
        requester_capabilities: Vec::new(),
    };
    let scored = compute_score(&target, &request, &candidates, &ScoringWeights::default());
    Ok(types::ok(serde_json::json!({
        "did": scored.candidate.did,
        "final_score": format!("{:.4}", scored.final_score),
        "capability": format!("{:.4}", scored.capability_score),
        "quality": format!("{:.4}", scored.quality_score),
        "availability": format!("{:.4}", scored.availability_score),
        "cost_efficiency": format!("{:.4}", scored.cost_score),
        "complementarity": format!("{:.4}", scored.complementarity_score),
    })))
}

pub async fn scoring_weights() -> impl IntoResponse {
    let weights = ScoringWeights::default();
    let factors = [
        ("capability_match", weights.capability_match),
        ("quality", weights.quality),
        ("availability", weights.availability),
        ("cost_efficiency", weights.cost_efficiency),
        ("complementarity", weights.complementarity),
    ];
    let rows = factors.map(|(factor, weight)| {
        serde_json::json!({
            "factor": factor,
            "weight": format!("{weight:.2}"),
            "pct": format!("{:.0}%", weight * 100.0),
        })
    });
    types::ok(serde_json::json!({ "data": rows }))
}

fn parse_capabilities(value: &str) -> Result<Vec<String>, ApiError> {
    let capabilities = value
        .split(',')
        .map(str::trim)
        .filter(|capability| !capability.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if capabilities.is_empty() {
        return Err(ApiError::BadRequest("capabilities cannot be empty".into()));
    }
    Ok(capabilities)
}

fn candidates(state: &ApiState) -> Result<Vec<AgentCandidate>, ApiError> {
    let dict = neunode_knowledge::StringDictionary::new(&state.db);
    let engine = neunode_knowledge::QueryEngine::new(&state.db, &dict);
    let grouped = grouped_objects(&engine, neunode_knowledge::PRED_HAS_CAPABILITY)?;
    Ok(grouped
        .into_iter()
        .enumerate()
        .map(|(index, (did, capabilities))| AgentCandidate {
            did,
            capabilities,
            reputation_score: 3.0 + (index as f64 * 0.2).min(2.0),
            stake_amount: 500 + index as u64 * 100,
            availability_score: 0.8 + (index as f64 * 0.02).min(0.2),
            latency_ms: 30 + index as u32 * 10,
            cost_per_unit: 5.0 + index as f64 * 2.0,
            is_online: index % 3 != 2,
        })
        .collect())
}

fn grouped_objects(
    engine: &neunode_knowledge::QueryEngine<'_>,
    predicate: &str,
) -> Result<Vec<(String, Vec<String>)>, ApiError> {
    let predicate = neunode_knowledge::StringDictionary::hash(&neunode_knowledge::nn(predicate));
    let pattern =
        neunode_knowledge::QueryPattern { predicate: Some(predicate), ..Default::default() };
    let mut grouped = Vec::<(String, Vec<String>)>::new();
    for row in engine.query(&pattern)? {
        if let Some((_, objects)) = grouped.iter_mut().find(|(subject, _)| *subject == row.subject)
        {
            objects.push(row.object);
        } else {
            grouped.push((row.subject, vec![row.object]));
        }
    }
    Ok(grouped)
}

fn unavailable_candidate(did: String) -> AgentCandidate {
    AgentCandidate {
        did,
        capabilities: Vec::new(),
        reputation_score: 0.0,
        stake_amount: 0,
        availability_score: 0.0,
        latency_ms: 1000,
        cost_per_unit: f64::MAX,
        is_online: false,
    }
}
