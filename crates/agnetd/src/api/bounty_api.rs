use std::sync::Arc;

use axum::extract::{Path, Query, State};
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateBountyRequest {
    pub title: String,
    pub description: String,
    pub reward: u64,
    #[serde(default = "default_token")]
    pub token: String,
    #[serde(default = "default_claim")]
    pub claim_deadline: u64,
    #[serde(default = "default_work")]
    pub work_deadline: u64,
}

fn default_token() -> String {
    "compute".to_string()
}
fn default_claim() -> u64 {
    72
}
fn default_work() -> u64 {
    168
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClaimBountyRequest {
    pub stake: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SubmitBountyRequest {
    pub artifact: String,
    pub evidence: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReviewBountyRequest {
    pub score: u8,
    pub feedback: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CancelBountyRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BountyListQuery {
    pub state: Option<String>,
    pub creator: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BountyResponse {
    pub id: String,
    pub title: String,
    pub description: String,
    pub state: String,
    pub creator: String,
    pub claimant: Option<String>,
    pub reward: u64,
    pub reward_token_type: u8,
    pub escrow_deposited: u64,
    pub created_at: u64,
    pub claim_deadline: u64,
    pub work_deadline: u64,
    pub review_deadline: u64,
    pub artifact_hash: Option<String>,
    pub bond: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BountyListItem {
    pub id: String,
    pub title: String,
    pub state: String,
    pub reward: u64,
    pub creator: String,
    pub created_at: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BountyClaimResponse {
    pub bounty_id: String,
    pub claimant: String,
    pub bond: u64,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BountySubmitResponse {
    pub bounty_id: String,
    pub artifact_cid: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BountyReviewResponse {
    pub bounty_id: String,
    pub score: u8,
    pub feedback: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BountyCancelResponse {
    pub bounty_id: String,
    pub state: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BountyPayResponse {
    pub bounty_id: String,
    pub claimant: String,
    pub amount_paid: u64,
    pub bond_returned: u64,
    pub state: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn current_timestamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn generate_bounty_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let cnt = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("bnty_{:012x}{:04x}", ts, cnt & 0xFFFF)
}

fn parse_token_type(s: &str) -> Result<neunode_core::types::TokenType, ApiError> {
    match s.to_lowercase().as_str() {
        "compute" | "ncompute" => Ok(neunode_core::types::TokenType::Compute),
        "train" | "ntrain" => Ok(neunode_core::types::TokenType::Train),
        "bandwidth" | "nbandwidth" => Ok(neunode_core::types::TokenType::Bandwidth),
        "storage" | "nstorage" => Ok(neunode_core::types::TokenType::Storage),
        _ => Err(ApiError::BadRequest(format!(
            "invalid token type '{}'. Must be one of: compute, train, bandwidth, storage",
            s
        ))),
    }
}

fn token_type_to_u8(t: &neunode_core::types::TokenType) -> u8 {
    match t {
        neunode_core::types::TokenType::Compute => 0x01,
        neunode_core::types::TokenType::Train => 0x02,
        neunode_core::types::TokenType::Bandwidth => 0x03,
        neunode_core::types::TokenType::Storage => 0x04,
    }
}

fn bounty_data_to_response(data: &neunode_storage::bounty_store::BountyData) -> BountyResponse {
    BountyResponse {
        id: data.id.clone(),
        title: data.title.clone(),
        description: data.description.clone(),
        state: data.state.clone(),
        creator: data.requester_did.clone(),
        claimant: data.provider_did.clone(),
        reward: data.reward_amount,
        reward_token_type: data.reward_token_type,
        escrow_deposited: data.escrow_deposited,
        created_at: data.created_at,
        claim_deadline: data.claim_deadline,
        work_deadline: data.work_deadline,
        review_deadline: data.review_deadline,
        artifact_hash: data.artifact_hash.clone(),
        bond: data.bond,
    }
}

fn service_error(error: crate::bounty_service::BountyServiceError) -> ApiError {
    use crate::bounty_service::BountyServiceError;
    match error {
        BountyServiceError::NotFound(id) => ApiError::NotFound(format!("bounty '{id}' not found")),
        BountyServiceError::Invalid(message) => ApiError::BadRequest(message),
        BountyServiceError::Storage(
            neunode_storage::error::StorageError::InsufficientBalance { required, available },
        ) => ApiError::BadRequest(format!(
            "insufficient balance: required {required}, available {available}"
        )),
        BountyServiceError::Storage(error) => ApiError::Internal(error.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/bounties",
    params(
        ("state" = Option<String>, Query, description = "Filter by bounty state"),
        ("creator" = Option<String>, Query, description = "Filter by creator DID"),
        ("limit" = Option<usize>, Query, description = "Max results (default 50)"),
    ),
    responses(
        (status = 200, description = "List of bounties", body = Vec<BountyListItem>),
    ),
    tag = "bounty",
)]
pub async fn list_bounties(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<BountyListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    let all = store.list_all().map_err(|e| ApiError::Internal(e.to_string()))?;

    let state_filter = &query.state;
    let creator_filter = &query.creator;

    let filtered: Vec<BountyListItem> = all
        .iter()
        .filter(|b| {
            state_filter.as_ref().is_none_or(|sf| sf.to_lowercase() == b.state.to_lowercase())
        })
        .filter(|b| creator_filter.as_ref().is_none_or(|cf| b.requester_did.contains(cf.as_str())))
        .take(query.limit)
        .map(|b| BountyListItem {
            id: b.id.clone(),
            title: b.title.clone(),
            state: b.state.clone(),
            reward: b.reward_amount,
            creator: b.requester_did.clone(),
            created_at: b.created_at,
        })
        .collect();

    Ok(types::ok(filtered))
}

#[utoipa::path(
    post,
    path = "/api/v1/bounties",
    request_body = CreateBountyRequest,
    responses(
        (status = 201, description = "Bounty created", body = BountyResponse),
        (status = 400, description = "Invalid parameters"),
        (status = 401, description = "No active identity"),
    ),
    tag = "bounty",
)]
pub async fn create_bounty(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<CreateBountyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.title.is_empty() {
        return Err(ApiError::BadRequest("title cannot be empty".to_string()));
    }
    if body.description.is_empty() {
        return Err(ApiError::BadRequest("description cannot be empty".to_string()));
    }
    if body.reward == 0 {
        return Err(ApiError::BadRequest("reward must be greater than 0".to_string()));
    }

    let token_type = parse_token_type(&body.token)?;
    let now = current_timestamp();
    let creator = state.require_did()?;
    let bounty_id = generate_bounty_id();

    let claim_deadline_ts = now.saturating_add(body.claim_deadline * 3600);
    let work_deadline_ts = now.saturating_add(body.work_deadline * 3600);
    let review_deadline_ts = work_deadline_ts.saturating_add(3 * 86400);

    let bounty = neunode_storage::bounty_store::BountyData {
        id: bounty_id.clone(),
        state: "Open".to_string(),
        requester_did: creator.0.clone(),
        provider_did: None,
        reward_amount: body.reward,
        reward_token_type: token_type_to_u8(&token_type),
        deadline: work_deadline_ts,
        created_at: now,
        escrow_deposited: body.reward,
        title: body.title.clone(),
        description: body.description.clone(),
        claim_deadline: claim_deadline_ts,
        work_deadline: work_deadline_ts,
        review_deadline: review_deadline_ts,
        artifact_hash: None,
        bond: None,
    };

    let token_byte = token_type_to_u8(&token_type);
    let escrow_did = format!("escrow:{bounty_id}");
    let store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    store
        .create_with_escrow(&bounty, &creator.0, &escrow_did, token_byte, body.reward as u128)
        .map_err(|e| match e {
            neunode_storage::error::StorageError::InsufficientBalance { required, available } => {
                ApiError::BadRequest(format!(
                    "insufficient balance: required {required}, available {available}"
                ))
            }
            other => ApiError::Internal(other.to_string()),
        })?;

    let resp = bounty_data_to_response(&bounty);
    Ok(types::created(resp))
}

#[utoipa::path(
    get,
    path = "/api/v1/bounties/{id}",
    params(("id" = String, Path, description = "Bounty ID")),
    responses(
        (status = 200, description = "Bounty details", body = BountyResponse),
        (status = 404, description = "Bounty not found"),
    ),
    tag = "bounty",
)]
pub async fn show_bounty(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    let bounty = store
        .get(&id)
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("bounty '{id}' not found")))?;

    Ok(types::ok(bounty_data_to_response(&bounty)))
}

#[utoipa::path(
    post,
    path = "/api/v1/bounties/{id}/claim",
    params(("id" = String, Path, description = "Bounty ID")),
    request_body = ClaimBountyRequest,
    responses(
        (status = 200, description = "Bounty claimed", body = BountyClaimResponse),
        (status = 400, description = "Invalid stake or state transition"),
        (status = 401, description = "No active identity"),
        (status = 404, description = "Bounty not found"),
    ),
    tag = "bounty",
)]
pub async fn claim_bounty(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(body): Json<ClaimBountyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let claimant = state.require_did()?;
    let updated =
        crate::bounty_service::claim(&state.db, &id, claimant, body.stake, current_timestamp())
            .map_err(service_error)?;

    let resp = BountyClaimResponse {
        bounty_id: id,
        claimant: updated.provider_did.unwrap_or_else(|| claimant.0.clone()),
        bond: updated.bond.unwrap_or(body.stake),
        state: updated.state,
    };
    Ok(types::ok(resp))
}

#[utoipa::path(
    post,
    path = "/api/v1/bounties/{id}/submit",
    params(("id" = String, Path, description = "Bounty ID")),
    request_body = SubmitBountyRequest,
    responses(
        (status = 200, description = "Work submitted", body = BountySubmitResponse),
        (status = 400, description = "Invalid artifact or state transition"),
        (status = 401, description = "No active identity"),
        (status = 404, description = "Bounty not found"),
    ),
    tag = "bounty",
)]
pub async fn submit_bounty(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(body): Json<SubmitBountyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let artifact_cid = body.artifact;
    let actor = state.require_did()?;
    let updated =
        crate::bounty_service::submit(&state.db, &id, actor, &artifact_cid, current_timestamp())
            .map_err(service_error)?;

    let resp = BountySubmitResponse { bounty_id: id, artifact_cid, state: updated.state };
    Ok(types::ok(resp))
}

#[utoipa::path(
    post,
    path = "/api/v1/bounties/{id}/review",
    params(("id" = String, Path, description = "Bounty ID")),
    request_body = ReviewBountyRequest,
    responses(
        (status = 200, description = "Review submitted", body = BountyReviewResponse),
        (status = 400, description = "Invalid score or state transition"),
        (status = 401, description = "No active identity"),
        (status = 404, description = "Bounty not found"),
    ),
    tag = "bounty",
)]
pub async fn review_bounty(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(body): Json<ReviewBountyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let reviewer = state.require_did()?;
    let updated = crate::bounty_service::review(
        &state.db,
        &id,
        reviewer,
        body.score,
        &body.feedback,
        current_timestamp(),
    )
    .map_err(service_error)?;

    let resp = BountyReviewResponse {
        bounty_id: id,
        score: body.score,
        feedback: body.feedback,
        state: updated.state,
    };
    Ok(types::ok(resp))
}

#[utoipa::path(
    post,
    path = "/api/v1/bounties/{id}/pay",
    params(("id" = String, Path, description = "Bounty ID")),
    responses(
        (status = 200, description = "Bounty paid out", body = BountyPayResponse),
        (status = 400, description = "Invalid state for payout"),
        (status = 401, description = "No active identity"),
        (status = 404, description = "Bounty not found"),
    ),
    tag = "bounty",
)]
pub async fn pay_bounty(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = state.require_did()?;
    let payment = crate::bounty_service::pay(&state.db, &id, actor, current_timestamp())
        .map_err(service_error)?;

    let resp = BountyPayResponse {
        bounty_id: id,
        claimant: payment.claimant,
        amount_paid: payment.reward_paid,
        bond_returned: payment.bond_returned,
        state: payment.bounty.state,
    };
    Ok(types::ok(resp))
}

#[utoipa::path(
    post,
    path = "/api/v1/bounties/{id}/cancel",
    params(("id" = String, Path, description = "Bounty ID")),
    request_body = CancelBountyRequest,
    responses(
        (status = 200, description = "Bounty cancelled", body = BountyCancelResponse),
        (status = 400, description = "Invalid state for cancellation"),
        (status = 404, description = "Bounty not found"),
    ),
    tag = "bounty",
)]
pub async fn cancel_bounty(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(body): Json<CancelBountyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = state.require_did()?;
    let updated = crate::bounty_service::cancel(&state.db, &id, actor, current_timestamp())
        .map_err(service_error)?;

    let resp = BountyCancelResponse {
        bounty_id: id,
        state: updated.state,
        reason: body.reason.unwrap_or_default(),
    };
    Ok(types::ok(resp))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_bounty_request_defaults() {
        let req: CreateBountyRequest =
            serde_json::from_str(r#"{"title":"T","description":"D","reward":100}"#).unwrap();
        assert_eq!(req.token, "compute");
        assert_eq!(req.claim_deadline, 72);
        assert_eq!(req.work_deadline, 168);
    }

    #[test]
    fn create_bounty_request_custom() {
        let req: CreateBountyRequest = serde_json::from_str(
            r#"{"title":"T","description":"D","reward":500,"token":"train","claim_deadline":48,"work_deadline":96}"#,
        )
        .unwrap();
        assert_eq!(req.token, "train");
        assert_eq!(req.claim_deadline, 48);
        assert_eq!(req.work_deadline, 96);
    }

    #[test]
    fn claim_bounty_request_parse() {
        let req: ClaimBountyRequest = serde_json::from_str(r#"{"stake":200}"#).unwrap();
        assert_eq!(req.stake, 200);
    }

    #[test]
    fn submit_bounty_request_with_evidence() {
        let req: SubmitBountyRequest =
            serde_json::from_str(r#"{"artifact":"ipfs://QmX7b","evidence":"proof.txt"}"#).unwrap();
        assert_eq!(req.artifact, "ipfs://QmX7b");
        assert_eq!(req.evidence.as_deref(), Some("proof.txt"));
    }

    #[test]
    fn submit_bounty_request_without_evidence() {
        let req: SubmitBountyRequest =
            serde_json::from_str(r#"{"artifact":"ipfs://QmX7b"}"#).unwrap();
        assert!(req.evidence.is_none());
    }

    #[test]
    fn review_bounty_request_parse() {
        let req: ReviewBountyRequest =
            serde_json::from_str(r#"{"score":85,"feedback":"Good work"}"#).unwrap();
        assert_eq!(req.score, 85);
        assert_eq!(req.feedback, "Good work");
    }

    #[test]
    fn cancel_bounty_request_with_reason() {
        let req: CancelBountyRequest =
            serde_json::from_str(r#"{"reason":"No longer needed"}"#).unwrap();
        assert_eq!(req.reason.as_deref(), Some("No longer needed"));
    }

    #[test]
    fn cancel_bounty_request_without_reason() {
        let req: CancelBountyRequest = serde_json::from_str(r#"{}"#).unwrap();
        assert!(req.reason.is_none());
    }

    #[test]
    fn bounty_list_query_defaults() {
        let query: BountyListQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.limit, 50);
        assert!(query.state.is_none());
        assert!(query.creator.is_none());
    }

    #[test]
    fn parse_token_type_all_variants() {
        assert!(matches!(parse_token_type("compute"), Ok(neunode_core::types::TokenType::Compute)));
        assert!(matches!(parse_token_type("train"), Ok(neunode_core::types::TokenType::Train)));
        assert!(matches!(
            parse_token_type("bandwidth"),
            Ok(neunode_core::types::TokenType::Bandwidth)
        ));
        assert!(matches!(parse_token_type("storage"), Ok(neunode_core::types::TokenType::Storage)));
        assert!(parse_token_type("invalid").is_err());
    }

    #[test]
    fn bounty_data_to_response_maps_fields() {
        let data = neunode_storage::bounty_store::BountyData {
            id: "bnty_001".to_string(),
            state: "Open".to_string(),
            requester_did: "did:neunode:abc".to_string(),
            provider_did: None,
            reward_amount: 1000,
            reward_token_type: 0x01,
            deadline: 999,
            created_at: 100,
            escrow_deposited: 1000,
            title: "Test".to_string(),
            description: "Desc".to_string(),
            claim_deadline: 200,
            work_deadline: 300,
            review_deadline: 400,
            artifact_hash: None,
            bond: None,
        };
        let resp = bounty_data_to_response(&data);
        assert_eq!(resp.id, "bnty_001");
        assert_eq!(resp.state, "Open");
        assert_eq!(resp.reward, 1000);
        assert!(resp.claimant.is_none());
    }
}
