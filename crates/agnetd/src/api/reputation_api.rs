use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::error::ApiError;
use crate::api::state::ApiState;
use crate::api::types;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

fn default_limit() -> usize {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttestRequest {
    pub to: String,
    pub score: u8,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReputationQuery {
    pub agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LeaderboardQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReputationResponse {
    pub agent: String,
    pub score: f64,
    pub grade: String,
    pub attestation_count: usize,
    pub avg_attestation_score: f64,
    pub factors: ReputationFactors,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReputationFactors {
    pub stake: f64,
    pub attest: f64,
    pub activity: f64,
    pub verify: f64,
    pub tenure: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttestationResponse {
    pub attester: String,
    pub target: String,
    pub score: u8,
    pub comment: String,
    pub signed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LeaderboardEntry {
    pub rank: usize,
    pub agent: String,
    pub score: f64,
    pub grade: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FactorBreakdown {
    pub agent: String,
    pub total_score: f64,
    pub grade: String,
    pub factors: Vec<FactorDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FactorDetail {
    pub name: String,
    pub weight: String,
    pub value: f64,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/reputation",
    params(
        ("agent" = Option<String>, Query, description = "Agent DID (defaults to active identity)"),
    ),
    responses(
        (status = 200, description = "Reputation details", body = ReputationResponse),
        (status = 401, description = "No active identity"),
    ),
    tag = "reputation",
)]
pub async fn show_reputation(
    State(state): State<Arc<ApiState>>,
    Query(params): Query<ReputationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let agent_did = match params.agent {
        Some(d) => d,
        None => state.require_did()?.0.clone(),
    };

    let db = &state.db;
    let attestations = load_attestations_for(db, &agent_did);

    let avg_score = if attestations.is_empty() {
        0.0
    } else {
        attestations.iter().map(|a| a.score).sum::<f64>() / attestations.len() as f64
    };

    let inputs = neunode_reputation::score::FactorInputs {
        staked_amount: neunode_core::types::TokenAmount(0),
        total_staked: neunode_core::types::TokenAmount(0),
        attestation_count: attestations.len() as u32,
        avg_attestation_score: avg_score,
        events_per_day: 0.0,
        days_active: 0,
        tasks_completed: 0,
        tasks_failed: 0,
        days_since_creation: 0,
    };
    let score = neunode_reputation::score::ReputationScore::compute_default(&inputs);
    let grade = score.grade();

    Ok(types::ok(ReputationResponse {
        agent: agent_did,
        score: score.total,
        grade: format!("{}", grade),
        attestation_count: attestations.len(),
        avg_attestation_score: avg_score,
        factors: ReputationFactors {
            stake: score.stake_factor,
            attest: score.attest_factor,
            activity: score.activity_factor,
            verify: score.verify_factor,
            tenure: score.tenure_factor,
        },
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/reputation/attest",
    request_body = AttestRequest,
    responses(
        (status = 201, description = "Attestation submitted", body = AttestationResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "No active identity"),
    ),
    tag = "reputation",
)]
pub async fn attest_agent(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<AttestRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.to.is_empty() {
        return Err(ApiError::BadRequest("target DID cannot be empty".into()));
    }
    if !body.to.starts_with("did:") {
        return Err(ApiError::BadRequest("target must be a valid DID (did:...)".into()));
    }
    if body.score > 100 {
        return Err(ApiError::BadRequest(format!(
            "invalid score: {} (must be 0-100)",
            body.score
        )));
    }

    let keyring = state.require_keyring()?;
    let attester_did = state.require_did()?;

    let claim = body.comment.as_deref().unwrap_or("general").to_string();

    let mut attestation = neunode_reputation::attestation::Attestation::new(
        attester_did.clone(),
        neunode_core::types::Did(body.to.clone()),
        claim,
        body.score as f64,
        neunode_core::types::Hash256("0".to_string()),
    )
    .map_err(|e| ApiError::Internal(format!("failed to create attestation: {e}")))?;

    let (ed_bytes, _) = keyring.to_bytes();
    let ed_bytes_fixed: [u8; 32] = ed_bytes
        .as_slice()
        .try_into()
        .map_err(|_| ApiError::Internal("invalid ed25519 key length".into()))?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&ed_bytes_fixed);
    attestation.sign(&signing_key);

    let db = &state.db;
    persist_attestation(db, &attestation)?;

    let comment_str = body.comment.unwrap_or_default();
    Ok(types::created(AttestationResponse {
        attester: attester_did.0.clone(),
        target: body.to,
        score: body.score,
        comment: comment_str,
        signed: attestation.signature.is_some(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/reputation/leaderboard",
    params(
        ("limit" = Option<usize>, Query, description = "Max entries (default 20)"),
    ),
    responses(
        (status = 200, description = "Leaderboard", body = Vec<LeaderboardEntry>),
    ),
    tag = "reputation",
)]
pub async fn leaderboard(
    State(state): State<Arc<ApiState>>,
    Query(params): Query<LeaderboardQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let db = &state.db;
    let entries = db
        .prefix_scan(neunode_storage::cf::CF_REPUTATION, &[])
        .map_err(|e| ApiError::Internal(format!("reputation scan: {e}")))?;

    let mut agent_scores: std::collections::HashMap<String, (f64, usize)> =
        std::collections::HashMap::new();

    for (_, value_bytes) in &entries {
        if let Ok(att) =
            bincode::deserialize::<neunode_reputation::attestation::Attestation>(value_bytes)
        {
            let entry = agent_scores.entry(att.target.0.clone()).or_insert((0.0, 0));
            entry.0 += att.score;
            entry.1 += 1;
        }
    }

    let mut ranked: Vec<(String, f64)> = agent_scores
        .into_iter()
        .map(|(did, (sum, count))| (did, sum / count as f64))
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let leaderboard: Vec<LeaderboardEntry> = ranked
        .iter()
        .take(params.limit)
        .enumerate()
        .map(|(i, (agent, score))| LeaderboardEntry {
            rank: i + 1,
            agent: agent.clone(),
            score: *score,
            grade: format!("{}", neunode_reputation::score::ReputationGrade::from_score(*score)),
        })
        .collect();

    Ok(types::ok(leaderboard))
}

#[utoipa::path(
    get,
    path = "/api/v1/reputation/factors",
    params(
        ("agent" = Option<String>, Query, description = "Agent DID (defaults to 'active')"),
    ),
    responses(
        (status = 200, description = "Factor breakdown", body = FactorBreakdown),
        (status = 400, description = "Bad request"),
    ),
    tag = "reputation",
)]
pub async fn show_factors(
    State(state): State<Arc<ApiState>>,
    Query(params): Query<ReputationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let agent = match params.agent {
        Some(a) if !a.is_empty() => a,
        Some(_) | None => state.require_did()?.0.clone(),
    };

    let db = &state.db;
    let attestations = load_attestations_for(db, &agent);
    let avg_score = if attestations.is_empty() {
        0.0
    } else {
        attestations.iter().map(|a| a.score).sum::<f64>() / attestations.len() as f64
    };

    let inputs = neunode_reputation::score::FactorInputs {
        staked_amount: neunode_core::types::TokenAmount(0),
        total_staked: neunode_core::types::TokenAmount(0),
        attestation_count: attestations.len() as u32,
        avg_attestation_score: avg_score,
        events_per_day: 0.0,
        days_active: 0,
        tasks_completed: 0,
        tasks_failed: 0,
        days_since_creation: 0,
    };

    let weights = neunode_reputation::factors::FactorWeights::default();
    let score = neunode_reputation::score::ReputationScore::compute(&weights, &inputs);

    let factors = vec![
        FactorDetail { name: "Stake".into(), weight: format!("{:.0}%", weights.stake), value: score.stake_factor },
        FactorDetail { name: "Attestation".into(), weight: format!("{:.0}%", weights.attest), value: score.attest_factor },
        FactorDetail { name: "Activity".into(), weight: format!("{:.0}%", weights.activity), value: score.activity_factor },
        FactorDetail { name: "Verification".into(), weight: format!("{:.0}%", weights.verify), value: score.verify_factor },
        FactorDetail { name: "Tenure".into(), weight: format!("{:.0}%", weights.tenure), value: score.tenure_factor },
    ];

    Ok(types::ok(FactorBreakdown {
        agent,
        total_score: score.total,
        grade: format!("{}", score.grade()),
        factors,
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn persist_attestation(
    db: &neunode_storage::db::NeunodeDb,
    attestation: &neunode_reputation::attestation::Attestation,
) -> Result<(), ApiError> {
    let key = format!("att_{}_{}", attestation.attester.0, attestation.timestamp);
    let key_bytes =
        bincode::serialize(&key).map_err(|e| ApiError::Internal(format!("key serialization: {e}")))?;
    let value_bytes = bincode::serialize(attestation)
        .map_err(|e| ApiError::Internal(format!("value serialization: {e}")))?;
    db.put_raw(neunode_storage::cf::CF_REPUTATION, &key_bytes, &value_bytes)
        .map_err(|e| ApiError::Internal(format!("persist attestation: {e}")))?;
    Ok(())
}

fn load_attestations_for(
    db: &neunode_storage::db::NeunodeDb,
    did: &str,
) -> Vec<neunode_reputation::attestation::Attestation> {
    let entries = match db.prefix_scan(neunode_storage::cf::CF_REPUTATION, &[]) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .iter()
        .filter_map(|(_, v)| bincode::deserialize::<neunode_reputation::attestation::Attestation>(v).ok())
        .filter(|a| a.target.0 == did)
        .collect()
}
