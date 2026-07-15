use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use neunode_core::constants::token::{
    DECAY_BURN_PCT, DECAY_DEV_FUND_PCT, DECAY_STAKING_REWARDS_PCT, DECAY_TREASURY_PCT, MIN_STAKE,
};
use neunode_core::types::{ActivityLevel, TokenType};
use neunode_storage::token_store::{
    TokenStore, TOKEN_BANDWIDTH, TOKEN_COMPUTE, TOKEN_STORAGE, TOKEN_TRAINING,
};
use neunode_storage::unbonding_store::UnbondingStore;
use neunode_token::decay::DecayCalculator;

use super::error::ApiError;
use super::state::ApiState;
use super::types;

// ---------------------------------------------------------------------------
// Request / Query types
// ---------------------------------------------------------------------------

fn default_token() -> String {
    "compute".to_string()
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TransferRequest {
    pub to: String,
    pub amount: u64,
    #[serde(default = "default_token")]
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StakeRequest {
    pub amount: u64,
    #[serde(default = "default_token")]
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UnstakeRequest {
    pub amount: u64,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct BalanceQuery {
    pub token: Option<String>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BalanceResponse {
    pub token: String,
    pub balance: u128,
    pub staked: u128,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AllBalancesResponse {
    pub balances: Vec<BalanceResponse>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TransferResponse {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub token: String,
    pub state: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StakeResponse {
    pub amount: u64,
    pub token: String,
    pub state: String,
    pub unbonding_period_secs: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UnstakeResponse {
    pub id: String,
    pub amount: u64,
    pub token: String,
    pub unbond_at: u64,
    pub state: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StakeStatusResponse {
    pub total_staked: u128,
    pub entries: Vec<StakeEntry>,
    pub unbonding: Vec<UnbondingPosition>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UnbondingPosition {
    pub id: String,
    pub token: String,
    pub amount: u128,
    pub created_at: u64,
    pub unlock_at: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ClaimUnbondedResponse {
    pub claimed_amount: u128,
    pub claimed_positions: usize,
    pub state: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StakeEntry {
    pub amount: u128,
    pub token: String,
    pub available: u128,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DecayInfoResponse {
    pub levels: Vec<DecayLevel>,
    pub redistribution: RedistributionInfo,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DecayLevel {
    pub name: String,
    pub decay_rate_pct: f64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RedistributionInfo {
    pub treasury_pct: f64,
    pub staking_pct: f64,
    pub burned_pct: f64,
    pub dev_fund_pct: f64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn token_type_to_u8(t: &TokenType) -> u8 {
    match t {
        TokenType::Compute => TOKEN_COMPUTE,
        TokenType::Train => TOKEN_TRAINING,
        TokenType::Bandwidth => TOKEN_BANDWIDTH,
        TokenType::Storage => TOKEN_STORAGE,
    }
}

fn token_type_display(t: &TokenType) -> &'static str {
    match t {
        TokenType::Compute => "nCompute",
        TokenType::Train => "nTrain",
        TokenType::Bandwidth => "nBandwidth",
        TokenType::Storage => "nStorage",
    }
}

fn parse_token_type(s: &str) -> Result<TokenType, ApiError> {
    match s.to_lowercase().as_str() {
        "compute" | "ncompute" => Ok(TokenType::Compute),
        "train" | "ntrain" => Ok(TokenType::Train),
        "bandwidth" | "nbandwidth" => Ok(TokenType::Bandwidth),
        "storage" | "nstorage" => Ok(TokenType::Storage),
        _ => Err(ApiError::BadRequest(format!(
            "invalid token type '{s}'. Must be one of: compute, train, bandwidth, storage"
        ))),
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/tokens/balance",
    params(BalanceQuery),
    responses(
        (status = 200, description = "Token balance(s) retrieved", body = AllBalancesResponse)
    ),
    tag = "tokens",
)]
pub async fn token_balance(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<BalanceQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let did = state.require_did()?;
    let store = TokenStore::new(&state.db);

    if let Some(token_str) = query.token {
        let tt = parse_token_type(&token_str)?;
        let token_byte = token_type_to_u8(&tt);
        let bal =
            store.get_balance(&did.0, token_byte).map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(types::ok(BalanceResponse {
            token: token_type_display(&tt).to_string(),
            balance: bal.balance,
            staked: bal.staked,
        }))
    } else {
        let all_tokens =
            [TokenType::Compute, TokenType::Train, TokenType::Bandwidth, TokenType::Storage];
        let mut balances = Vec::new();
        for tt in &all_tokens {
            let token_byte = token_type_to_u8(tt);
            let bal = store
                .get_balance(&did.0, token_byte)
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            balances.push(BalanceResponse {
                token: token_type_display(tt).to_string(),
                balance: bal.balance,
                staked: bal.staked,
            });
        }
        Ok(types::ok(AllBalancesResponse { balances }))
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/tokens/transfer",
    request_body = TransferRequest,
    responses(
        (status = 200, description = "Tokens transferred successfully", body = TransferResponse)
    ),
    tag = "tokens",
)]
pub async fn transfer(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<TransferRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.to.is_empty() {
        return Err(ApiError::BadRequest("recipient DID cannot be empty".to_string()));
    }
    if !body.to.starts_with("did:") {
        return Err(ApiError::BadRequest("recipient must be a valid DID (did:...)".to_string()));
    }
    if body.amount == 0 {
        return Err(ApiError::BadRequest("amount must be greater than 0".to_string()));
    }

    let did = state.require_did()?;
    let tt = parse_token_type(&body.token)?;
    let token_byte = token_type_to_u8(&tt);
    let token_name = token_type_display(&tt).to_string();

    let store = TokenStore::new(&state.db);
    store
        .transfer(&did.0, &body.to, token_byte, body.amount as u128)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(types::ok(TransferResponse {
        from: did.0.clone(),
        to: body.to,
        amount: body.amount,
        token: token_name,
        state: "transferred".to_string(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/tokens/stake",
    request_body = StakeRequest,
    responses(
        (status = 200, description = "Tokens staked successfully", body = StakeResponse)
    ),
    tag = "tokens",
)]
pub async fn stake(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<StakeRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.amount == 0 {
        return Err(ApiError::BadRequest("amount must be greater than 0".to_string()));
    }
    if body.amount < MIN_STAKE {
        return Err(ApiError::BadRequest(format!(
            "amount {} is below minimum stake of {}",
            body.amount, MIN_STAKE
        )));
    }

    let did = state.require_did()?;
    let tt = parse_token_type(&body.token)?;
    let token_byte = token_type_to_u8(&tt);
    let token_name = token_type_display(&tt).to_string();

    let store = TokenStore::new(&state.db);
    store.stake(&did.0, token_byte, body.amount as u128).map_err(|error| match error {
        neunode_storage::error::StorageError::InsufficientBalance { required, available } => {
            ApiError::BadRequest(format!("insufficient balance: have {available}, need {required}"))
        }
        other => ApiError::Internal(other.to_string()),
    })?;

    Ok(types::ok(StakeResponse {
        amount: body.amount,
        token: token_name,
        state: "Staked".to_string(),
        unbonding_period_secs: state.config.app_config.tokens.unbonding_period_secs,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/tokens/unstake",
    request_body = UnstakeRequest,
    responses(
        (status = 200, description = "Tokens unstaked successfully", body = UnstakeResponse)
    ),
    tag = "tokens",
)]
pub async fn unstake(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<UnstakeRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.amount == 0 {
        return Err(ApiError::BadRequest("amount must be greater than 0".to_string()));
    }

    let did = state.require_did()?;
    let token_types = [TOKEN_COMPUTE, TOKEN_TRAINING, TOKEN_BANDWIDTH, TOKEN_STORAGE];
    let entry = UnbondingStore::new(&state.db)
        .begin(
            &did.0,
            &token_types,
            body.amount as u128,
            current_timestamp(),
            state.config.app_config.tokens.unbonding_period_secs,
        )
        .map_err(|error| match error {
            neunode_storage::error::StorageError::InsufficientStakedBalance { .. } => {
                ApiError::BadRequest(format!(
                    "no staked tokens found with sufficient balance to unstake {}",
                    body.amount
                ))
            }
            other => ApiError::Internal(other.to_string()),
        })?;
    let tt = match entry.token_type {
        TOKEN_COMPUTE => TokenType::Compute,
        TOKEN_TRAINING => TokenType::Train,
        TOKEN_BANDWIDTH => TokenType::Bandwidth,
        TOKEN_STORAGE => TokenType::Storage,
        _ => unreachable!("known token type selected"),
    };

    let token_name = token_type_display(&tt).to_string();
    Ok(types::ok(UnstakeResponse {
        id: entry.id,
        amount: body.amount,
        token: token_name,
        unbond_at: entry.unlock_at,
        state: "Unbonding".to_string(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/tokens/claim-unbonded",
    responses(
        (status = 200, description = "Matured unbonding positions claimed", body = ClaimUnbondedResponse)
    ),
    tag = "tokens",
)]
pub async fn claim_unbonded(
    State(state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    let did = state.require_did()?;
    let claimed = UnbondingStore::new(&state.db)
        .claim_matured(&did.0, current_timestamp())
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(types::ok(ClaimUnbondedResponse {
        claimed_amount: claimed.total,
        claimed_positions: claimed.entries.len(),
        state: if claimed.entries.is_empty() { "NothingMatured" } else { "Claimed" }.to_string(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/tokens/stake-status",
    responses(
        (status = 200, description = "Staking status retrieved", body = StakeStatusResponse)
    ),
    tag = "tokens",
)]
pub async fn stake_status(
    State(state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    let did = state.require_did()?;
    let store = TokenStore::new(&state.db);

    let token_types = [
        (TokenType::Compute, TOKEN_COMPUTE),
        (TokenType::Train, TOKEN_TRAINING),
        (TokenType::Bandwidth, TOKEN_BANDWIDTH),
        (TokenType::Storage, TOKEN_STORAGE),
    ];

    let mut total_staked: u128 = 0;
    let mut entries = Vec::new();

    for (tt, byte) in &token_types {
        let bal =
            store.get_balance(&did.0, *byte).map_err(|e| ApiError::Internal(e.to_string()))?;
        if bal.staked > 0 {
            total_staked += bal.staked;
            entries.push(StakeEntry {
                amount: bal.staked,
                token: token_type_display(tt).to_string(),
                available: bal.balance,
            });
        }
    }

    let unbonding = UnbondingStore::new(&state.db)
        .list(&did.0)
        .map_err(|error| ApiError::Internal(error.to_string()))?
        .into_iter()
        .map(|entry| UnbondingPosition {
            id: entry.id,
            token: match entry.token_type {
                TOKEN_COMPUTE => "nCompute",
                TOKEN_TRAINING => "nTrain",
                TOKEN_BANDWIDTH => "nBandwidth",
                TOKEN_STORAGE => "nStorage",
                _ => "Unknown",
            }
            .to_string(),
            amount: entry.amount,
            created_at: entry.created_at,
            unlock_at: entry.unlock_at,
        })
        .collect();
    Ok(types::ok(StakeStatusResponse { total_staked, entries, unbonding }))
}

#[utoipa::path(
    get,
    path = "/api/v1/tokens/decay-info",
    responses(
        (status = 200, description = "Decay info retrieved", body = DecayInfoResponse)
    ),
    tag = "tokens",
)]
pub async fn decay_info(
    State(_state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    let levels = [
        ("Active", ActivityLevel::Active),
        ("Moderate", ActivityLevel::Moderate),
        ("Low", ActivityLevel::Low),
        ("Inactive", ActivityLevel::Inactive),
        ("Dead", ActivityLevel::Dead),
    ];

    let decay_levels: Vec<DecayLevel> = levels
        .iter()
        .map(|(name, level)| DecayLevel {
            name: name.to_string(),
            decay_rate_pct: DecayCalculator::effective_decay_rate(*level),
        })
        .collect();

    let redistribution = RedistributionInfo {
        treasury_pct: DECAY_TREASURY_PCT,
        staking_pct: DECAY_STAKING_REWARDS_PCT,
        burned_pct: DECAY_BURN_PCT,
        dev_fund_pct: DECAY_DEV_FUND_PCT,
    };

    Ok(types::ok(DecayInfoResponse { levels: decay_levels, redistribution }))
}
