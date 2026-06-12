use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::error::ApiError;
use crate::api::state::ApiState;
use crate::api::types;
use neunode_storage::error::StorageError;
use neunode_storage::token_store::TokenStore;

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
pub struct BountyActionResponse {
    pub bounty_id: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BountyPayResponse {
    pub bounty_id: String,
    pub claimant: String,
    pub amount_paid: u64,
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

fn map_token_transfer_error(err: StorageError) -> ApiError {
    match err {
        StorageError::InsufficientBalance { required, available } => ApiError::BadRequest(format!(
            "insufficient balance: required {required}, available {available}"
        )),
        other => ApiError::Internal(other.to_string()),
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
    let token_byte = token_type_to_u8(&token_type);
    let creator_did = creator.0.clone();
    let escrow_did = format!("escrow:{bounty_id}");

    let token_store = TokenStore::new(&state.db);
    token_store
        .transfer(&creator_did, &escrow_did, token_byte, body.reward as u128)
        .map_err(map_token_transfer_error)?;

    let claim_deadline_ts = now.saturating_add(body.claim_deadline * 3600);
    let work_deadline_ts = now.saturating_add(body.work_deadline * 3600);
    let review_deadline_ts = work_deadline_ts.saturating_add(3 * 86400);

    let bounty = neunode_storage::bounty_store::BountyData {
        id: bounty_id.clone(),
        state: "Open".to_string(),
        requester_did: creator_did.clone(),
        provider_did: None,
        reward_amount: body.reward,
        reward_token_type: token_byte,
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

    let store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    if let Err(err) = store.put(&bounty) {
        if let Err(refund_err) =
            token_store.transfer(&escrow_did, &creator_did, token_byte, body.reward as u128)
        {
            tracing::error!(
                "escrow rollback failed after bounty persistence error: {}",
                refund_err
            );
        }
        return Err(ApiError::Internal(err.to_string()));
    }

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
        (status = 200, description = "Bounty claimed", body = BountyActionResponse),
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
    if body.stake == 0 {
        return Err(ApiError::BadRequest("stake must be greater than 0".to_string()));
    }

    let claimant = state.require_did()?;
    let store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    let bounty = store
        .get(&id)
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("bounty '{id}' not found")))?;

    if bounty.state != "Open" {
        return Err(ApiError::BadRequest(format!(
            "bounty is in '{}' state, expected 'Open'",
            bounty.state
        )));
    }

    let mut updated = bounty;
    updated.state = "Claimed".to_string();
    updated.provider_did = Some(claimant.0.clone());
    updated.bond = Some(body.stake);

    store.put(&updated).map_err(|e| ApiError::Internal(e.to_string()))?;

    let resp = BountyActionResponse { bounty_id: id, state: "Claimed".to_string() };
    Ok(types::ok(resp))
}

#[utoipa::path(
    post,
    path = "/api/v1/bounties/{id}/submit",
    params(("id" = String, Path, description = "Bounty ID")),
    request_body = SubmitBountyRequest,
    responses(
        (status = 200, description = "Work submitted", body = BountyActionResponse),
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
    if body.artifact.is_empty() {
        return Err(ApiError::BadRequest("artifact CID cannot be empty".to_string()));
    }

    let store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    let bounty = store
        .get(&id)
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("bounty '{id}' not found")))?;

    if bounty.state != "Claimed" {
        return Err(ApiError::BadRequest(format!(
            "bounty is in '{}' state, expected 'Claimed'",
            bounty.state
        )));
    }

    let mut updated = bounty;
    updated.state = "Submitted".to_string();
    updated.artifact_hash = Some(body.artifact);

    store.put(&updated).map_err(|e| ApiError::Internal(e.to_string()))?;

    let resp = BountyActionResponse { bounty_id: id, state: "Submitted".to_string() };
    Ok(types::ok(resp))
}

#[utoipa::path(
    post,
    path = "/api/v1/bounties/{id}/review",
    params(("id" = String, Path, description = "Bounty ID")),
    request_body = ReviewBountyRequest,
    responses(
        (status = 200, description = "Review submitted", body = BountyActionResponse),
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
    if body.score > 100 {
        return Err(ApiError::BadRequest(format!("invalid score: {} (must be 0-100)", body.score)));
    }

    let store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    let bounty = store
        .get(&id)
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("bounty '{id}' not found")))?;

    if bounty.state != "Submitted" && bounty.state != "UnderReview" {
        return Err(ApiError::BadRequest(format!(
            "bounty is in '{}' state, expected 'Submitted' or 'UnderReview'",
            bounty.state
        )));
    }

    let mut updated = bounty;

    // Transition to UnderReview if still Submitted
    if updated.state == "Submitted" {
        updated.state = "UnderReview".to_string();
    }

    // Score-based decision following the CLI pattern
    if body.score >= 60 {
        updated.state = "Accepted".to_string();
    } else if body.score < 40 {
        updated.state = "Rejected".to_string();
    }

    store.put(&updated).map_err(|e| ApiError::Internal(e.to_string()))?;

    let resp = BountyActionResponse { bounty_id: id, state: updated.state };
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
    let store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    let bounty = store
        .get(&id)
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("bounty '{id}' not found")))?;

    if bounty.state != "Accepted" {
        return Err(ApiError::BadRequest(format!(
            "bounty is in '{}' state, expected 'Accepted'",
            bounty.state
        )));
    }

    let claimant = bounty
        .provider_did
        .clone()
        .ok_or_else(|| ApiError::BadRequest("bounty has no claimant".to_string()))?;

    let mut updated = bounty;
    updated.state = "Paid".to_string();

    store.put(&updated).map_err(|e| ApiError::Internal(e.to_string()))?;

    let resp = BountyPayResponse {
        bounty_id: id,
        claimant,
        amount_paid: updated.reward_amount,
        state: "Paid".to_string(),
    };
    Ok(types::ok(resp))
}

#[utoipa::path(
    post,
    path = "/api/v1/bounties/{id}/cancel",
    params(("id" = String, Path, description = "Bounty ID")),
    request_body = CancelBountyRequest,
    responses(
        (status = 200, description = "Bounty cancelled", body = BountyActionResponse),
        (status = 400, description = "Invalid state for cancellation"),
        (status = 404, description = "Bounty not found"),
    ),
    tag = "bounty",
)]
pub async fn cancel_bounty(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(_body): Json<CancelBountyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    let bounty = store
        .get(&id)
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("bounty '{id}' not found")))?;

    if bounty.state != "Open" && bounty.state != "Claimed" {
        return Err(ApiError::BadRequest(format!(
            "bounty in '{}' state cannot be cancelled",
            bounty.state
        )));
    }

    let mut updated = bounty;
    updated.state = "Cancelled".to_string();

    store.put(&updated).map_err(|e| ApiError::Internal(e.to_string()))?;

    let resp = BountyActionResponse { bounty_id: id, state: "Cancelled".to_string() };
    Ok(types::ok(resp))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> axum::Router<Arc<ApiState>> {
    axum::Router::new()
        .route("/api/v1/bounties", axum::routing::get(list_bounties).post(create_bounty))
        .route("/api/v1/bounties/{id}/claim", axum::routing::post(claim_bounty))
        .route("/api/v1/bounties/{id}/submit", axum::routing::post(submit_bounty))
        .route("/api/v1/bounties/{id}/review", axum::routing::post(review_bounty))
        .route("/api/v1/bounties/{id}/pay", axum::routing::post(pay_bounty))
        .route("/api/v1/bounties/{id}/cancel", axum::routing::post(cancel_bounty))
        .route("/api/v1/bounties/{id}", axum::routing::get(show_bounty))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn test_api_state() -> Arc<ApiState> {
        let mut state = crate::testutil::test_state();
        let (feed_tx, _) = tokio::sync::broadcast::channel(16);
        Arc::new(ApiState {
            db: state.db.clone(),
            active_did: state.active_did.clone(),
            active_keyring: Arc::new(Mutex::new(state.active_keyring.take())),
            mesh_handle: Arc::new(tokio::sync::RwLock::new(None)),
            config: state.config.clone(),
            feed_tx,
        })
    }

    fn seed_api_token_balance(state: &ApiState, token_byte: u8, balance: u128) {
        let did = state.active_did.as_ref().unwrap();
        let store = TokenStore::new(&state.db);
        store
            .set_balance(
                &did.0,
                token_byte,
                &neunode_storage::token_store::TokenBalance {
                    balance,
                    staked: 0,
                    last_decay_epoch: 0,
                },
            )
            .unwrap();
    }

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

    #[tokio::test]
    async fn create_bounty_escrows_tokens_before_persisting() {
        let state = test_api_state();
        seed_api_token_balance(&state, 0x01, 5_000);
        let did = state.active_did.as_ref().unwrap().0.clone();

        let req = CreateBountyRequest {
            title: "Escrowed API bounty".to_string(),
            description: "Desc".to_string(),
            reward: 1_000,
            token: "compute".to_string(),
            claim_deadline: 72,
            work_deadline: 168,
        };
        create_bounty(State(state.clone()), Json(req)).await.unwrap();

        let bounties =
            neunode_storage::bounty_store::BountyStore::new(&state.db).list_all().unwrap();
        assert_eq!(bounties.len(), 1);
        let bounty_id = &bounties[0].id;

        let token_store = TokenStore::new(&state.db);
        let escrow_balance = token_store.get_balance(&format!("escrow:{bounty_id}"), 0x01).unwrap();
        assert_eq!(escrow_balance.balance, 1_000);

        let creator_balance = token_store.get_balance(&did, 0x01).unwrap();
        assert_eq!(creator_balance.balance, 4_000);
    }

    #[tokio::test]
    async fn create_bounty_insufficient_balance_does_not_persist() {
        let state = test_api_state();
        let req = CreateBountyRequest {
            title: "No funds".to_string(),
            description: "Desc".to_string(),
            reward: 1_000,
            token: "compute".to_string(),
            claim_deadline: 72,
            work_deadline: 168,
        };

        let result = create_bounty(State(state.clone()), Json(req)).await;

        assert!(
            matches!(result, Err(ApiError::BadRequest(message)) if message.contains("insufficient balance"))
        );
        let bounties =
            neunode_storage::bounty_store::BountyStore::new(&state.db).list_all().unwrap();
        assert!(bounties.is_empty());
    }
}
