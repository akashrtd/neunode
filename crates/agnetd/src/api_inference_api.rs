use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use neunode_core::types::TokenAmount;
use neunode_inference::openai::{ChatCompletionRequest, ChatMessage, MessageRole};
use neunode_inference::provider::{InferenceProvider, ModelInfo, ProviderStatus};
use neunode_inference::router::{Router, RoutingStrategy};
use neunode_storage::db::NeunodeDb;

use super::error::ApiError;
use super::state::ApiState;
use super::types;

// ---------------------------------------------------------------------------
// Request / Query types
// ---------------------------------------------------------------------------

fn default_max_tokens() -> u32 {
    256
}

fn default_temp() -> f64 {
    0.7
}

fn default_strategy() -> String {
    "cheapest".to_string()
}

fn default_input() -> u32 {
    0
}

fn default_output() -> u32 {
    0
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InferenceRequest {
    pub model: String,
    pub prompt: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temp")]
    pub temperature: f64,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RegisterProviderRequest {
    pub name: String,
    pub endpoint: String,
    pub models: Vec<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ModelsQuery {
    pub provider: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ProvidersQuery {
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct RouteQuery {
    pub model: String,
    #[serde(default = "default_strategy")]
    pub strategy: String,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct PricingQuery {
    pub model: String,
    #[serde(default = "default_input")]
    pub input_tokens: u32,
    #[serde(default = "default_output")]
    pub output_tokens: u32,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema, Clone, Deserialize)]
pub struct InferenceResponse {
    pub model: String,
    pub prompt: String,
    pub max_tokens: u32,
    pub temperature: f64,
    pub estimated_input_tokens: u32,
    pub status: String,
    pub request_id: String,
    #[schema(value_type = Option<Object>)]
    pub completion: Option<neunode_inference::openai::ChatCompletionResponse>,
    pub settlement: Option<crate::inference_service::Receipt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<PricingEstimate>,
}

#[derive(Debug, Serialize, utoipa::ToSchema, Clone, Deserialize)]
pub struct PricingEstimate {
    pub input_price_per_mtok: String,
    pub output_price_per_mtok: String,
    pub estimated_cost: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ModelEntry {
    pub id: String,
    pub input_price_per_million: String,
    pub output_price_per_million: String,
    pub context_length: u32,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ModelsResponse {
    pub models: Vec<ModelEntry>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProviderEntry {
    pub name: String,
    pub did: String,
    pub status: String,
    pub reputation_score: f64,
    pub avg_latency_ms: u32,
    pub model_count: usize,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProvidersResponse {
    pub providers: Vec<ProviderEntry>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RouteResponse {
    pub model: String,
    pub strategy: String,
    pub selected_provider: Option<String>,
    pub provider_name: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PricingResponse {
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub input_cost: String,
    pub output_cost: String,
    pub total_cost: String,
    pub protocol_fee: String,
    pub net_payout: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RegisterProviderResponse {
    pub did: String,
    pub name: String,
    pub endpoint: String,
    pub models: Vec<String>,
    pub status: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn load_all_providers(db: &NeunodeDb) -> Vec<InferenceProvider> {
    let entries = match db.prefix_scan(neunode_storage::cf::CF_MODELS, &[]) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .iter()
        .filter(|(k, _)| {
            let key_str = neunode_storage::codec::deserialize::<String>(k).unwrap_or_default();
            key_str.starts_with("prov:")
        })
        .filter_map(|(_, v)| neunode_storage::codec::deserialize::<InferenceProvider>(v).ok())
        .collect()
}

fn load_model(db: &NeunodeDb, model_id: &str) -> Result<Option<ModelInfo>, ApiError> {
    let key = format!("model:{model_id}");
    let key = neunode_storage::codec::serialize(&key)
        .map_err(|error| ApiError::Internal(format!("serialize model key: {error}")))?;
    db.get_raw(neunode_storage::cf::CF_MODELS, &key)
        .map_err(|error| ApiError::Internal(format!("load model: {error}")))?
        .map(|value| {
            neunode_storage::codec::deserialize(&value)
                .map_err(|error| ApiError::Internal(format!("decode model: {error}")))
        })
        .transpose()
}

fn store_provider(db: &NeunodeDb, provider: &InferenceProvider) -> Result<(), ApiError> {
    let key = neunode_storage::codec::serialize(&format!("prov:{}", provider.did))
        .map_err(|error| ApiError::Internal(format!("serialize provider key: {error}")))?;
    let value = neunode_storage::codec::serialize(provider)
        .map_err(|error| ApiError::Internal(format!("encode provider: {error}")))?;
    db.put_raw(neunode_storage::cf::CF_MODELS, &key, &value)
        .map_err(|error| ApiError::Internal(format!("store provider: {error}")))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/inference/providers",
    request_body = RegisterProviderRequest,
    responses(
        (status = 201, description = "Inference provider registered", body = RegisterProviderResponse)
    ),
    tag = "inference",
)]
pub async fn register_provider(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<RegisterProviderRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("provider name cannot be empty".into()));
    }
    let endpoint = body
        .endpoint
        .parse::<axum::http::Uri>()
        .map_err(|error| ApiError::BadRequest(format!("invalid provider endpoint: {error}")))?;
    if !matches!(endpoint.scheme_str(), Some("http" | "https")) || endpoint.authority().is_none() {
        return Err(ApiError::BadRequest(
            "provider endpoint must be an absolute HTTP(S) URL".into(),
        ));
    }
    if body.models.is_empty() {
        return Err(ApiError::BadRequest("at least one model is required".into()));
    }

    let mut models = Vec::with_capacity(body.models.len());
    for model_id in &body.models {
        let model_id = model_id.trim();
        if model_id.is_empty() || models.iter().any(|model: &ModelInfo| model.id == model_id) {
            return Err(ApiError::BadRequest(format!("invalid or duplicate model ID: {model_id}")));
        }
        models.push(
            load_model(&state.db, model_id)?
                .ok_or_else(|| ApiError::NotFound(format!("model not found: {model_id}")))?,
        );
    }

    let did = state.require_did()?.clone();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let provider = InferenceProvider {
        did: did.clone(),
        name: name.to_string(),
        endpoint: endpoint.to_string(),
        models,
        reputation_score: 0.0,
        stake_amount: TokenAmount(0),
        status: ProviderStatus::Online,
        last_heartbeat: now,
        total_requests_served: 0,
        avg_latency_ms: 0,
    };
    store_provider(&state.db, &provider)?;
    Ok(types::created(RegisterProviderResponse {
        did: did.0,
        name: provider.name,
        endpoint: provider.endpoint,
        models: provider.models.into_iter().map(|model| model.id).collect(),
        status: "online".to_string(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/inference/request",
    request_body = InferenceRequest,
    responses(
        (status = 200, description = "Inference request submitted", body = InferenceResponse)
    ),
    tag = "inference",
)]
pub async fn request_inference(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<InferenceRequest>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(types::ok(execute_inference(&state, body).await?))
}

pub(crate) fn submit_inference(
    db: &NeunodeDb,
    body: InferenceRequest,
) -> Result<InferenceResponse, ApiError> {
    if body.model.is_empty() {
        return Err(ApiError::BadRequest("model cannot be empty".to_string()));
    }
    if body.prompt.is_empty() {
        return Err(ApiError::BadRequest("prompt cannot be empty".to_string()));
    }
    if body.max_tokens == 0 {
        return Err(ApiError::BadRequest("max_tokens must be greater than 0".to_string()));
    }
    if !(0.0..=2.0).contains(&body.temperature) {
        return Err(ApiError::BadRequest(format!(
            "temperature {} out of range (0.0-2.0)",
            body.temperature
        )));
    }

    let request = ChatCompletionRequest {
        model: body.model.clone(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: body.prompt.clone(),
            name: None,
        }],
        temperature: Some(body.temperature),
        max_tokens: Some(body.max_tokens),
        top_p: None,
        stream: None,
        stop: None,
        frequency_penalty: None,
        presence_penalty: None,
    };

    request.validate().map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let estimated_tokens = request.estimate_tokens();

    let providers = load_all_providers(db);
    let pricing_info = providers
        .iter()
        .find_map(|p| p.find_model(&body.model))
        .map(|m| {
            Ok::<_, ApiError>(PricingEstimate {
                input_price_per_mtok: m.input_price_per_million.0.to_string(),
                output_price_per_mtok: m.output_price_per_million.0.to_string(),
                estimated_cost: crate::inference_service::price(
                    estimated_tokens,
                    body.max_tokens,
                    m,
                )?
                .to_string(),
            })
        })
        .transpose()?;

    Ok(InferenceResponse {
        model: body.model,
        prompt: body.prompt,
        max_tokens: body.max_tokens,
        temperature: body.temperature,
        estimated_input_tokens: estimated_tokens,
        status: "validated".to_string(),
        request_id: String::new(),
        completion: None,
        settlement: None,
        pricing: pricing_info,
    })
}

pub(crate) async fn execute_inference(
    state: &Arc<ApiState>,
    body: InferenceRequest,
) -> Result<InferenceResponse, ApiError> {
    let validated = submit_inference(&state.db, body.clone())?;
    let provider = load_all_providers(&state.db)
        .into_iter()
        .find(|provider| {
            provider.status == ProviderStatus::Online && provider.find_model(&body.model).is_some()
        })
        .ok_or_else(|| {
            ApiError::NotFound(format!("no online provider for model {}", body.model))
        })?;
    let requester = state.require_did()?.0;
    let db = Arc::clone(&state.db);
    // A disconnected HTTP caller must not cancel accounting after funds are reserved.
    tokio::spawn(async move {
        crate::inference_service::execute(db, requester, provider, body, validated).await
    })
    .await
    .map_err(|error| ApiError::Internal(format!("inference task failed: {error}")))?
}

#[utoipa::path(
    get,
    path = "/api/v1/inference/models",
    params(ModelsQuery),
    responses(
        (status = 200, description = "List of available models", body = ModelsResponse)
    ),
    tag = "inference",
)]
pub async fn list_models(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<ModelsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let providers = load_all_providers(&state.db);

    let mut models: Vec<&ModelInfo> = Vec::new();
    for p in &providers {
        for m in &p.models {
            if !models.iter().any(|existing| existing.id == m.id) {
                models.push(m);
            }
        }
    }

    let filtered: Vec<&ModelInfo> = models
        .iter()
        .filter(|m| query.provider.as_deref().is_none_or(|p| m.id.contains(p)))
        .copied()
        .collect();

    let entries: Vec<ModelEntry> = filtered
        .into_iter()
        .map(|m| ModelEntry {
            id: m.id.clone(),
            input_price_per_million: m.input_price_per_million.0.to_string(),
            output_price_per_million: m.output_price_per_million.0.to_string(),
            context_length: m.context_length,
        })
        .collect();

    Ok(types::ok(ModelsResponse { models: entries }))
}

#[utoipa::path(
    get,
    path = "/api/v1/inference/providers",
    params(ProvidersQuery),
    responses(
        (status = 200, description = "List of inference providers", body = ProvidersResponse)
    ),
    tag = "inference",
)]
pub async fn list_providers(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<ProvidersQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let providers = load_all_providers(&state.db);

    let filtered: Vec<&InferenceProvider> = providers
        .iter()
        .filter(|p| query.model.as_deref().is_none_or(|m| p.has_model(m)))
        .collect();

    let entries: Vec<ProviderEntry> = filtered
        .into_iter()
        .map(|p| {
            let status = match p.status {
                ProviderStatus::Online => "online",
                ProviderStatus::Degraded => "degraded",
                ProviderStatus::Offline => "offline",
            };
            ProviderEntry {
                name: p.name.clone(),
                did: p.did.0.clone(),
                status: status.to_string(),
                reputation_score: p.reputation_score,
                avg_latency_ms: p.avg_latency_ms,
                model_count: p.models.len(),
            }
        })
        .collect();

    Ok(types::ok(ProvidersResponse { providers: entries }))
}

#[utoipa::path(
    get,
    path = "/api/v1/inference/route",
    params(RouteQuery),
    responses(
        (status = 200, description = "Routing result", body = RouteResponse)
    ),
    tag = "inference",
)]
pub async fn show_route(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<RouteQuery>,
) -> Result<impl IntoResponse, ApiError> {
    if query.model.is_empty() {
        return Err(ApiError::BadRequest("model cannot be empty".to_string()));
    }

    let strat = match query.strategy.to_lowercase().as_str() {
        "cheapest" => RoutingStrategy::Cheapest,
        "fastest" => RoutingStrategy::Fastest,
        "reputation" | "highest_reputation" => RoutingStrategy::HighestReputation,
        "random" => RoutingStrategy::Random,
        "round_robin" => RoutingStrategy::RoundRobin,
        _ => {
            return Err(ApiError::BadRequest(format!(
                "invalid strategy '{}'. Must be: cheapest, fastest, reputation, random, round_robin",
                query.strategy
            )));
        }
    };

    let providers = load_all_providers(&state.db);

    if providers.is_empty() {
        return Ok(types::ok(RouteResponse {
            model: query.model,
            strategy: query.strategy,
            selected_provider: None,
            provider_name: None,
            status: "no_providers".to_string(),
        }));
    }

    let router = Router::new(strat);
    let chosen = router
        .route(&providers, &query.model, Some(0))
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(types::ok(RouteResponse {
        model: query.model,
        strategy: query.strategy,
        selected_provider: Some(chosen.did.0.clone()),
        provider_name: Some(chosen.name.clone()),
        status: "routed".to_string(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/inference/pricing",
    params(PricingQuery),
    responses(
        (status = 200, description = "Pricing estimate", body = PricingResponse)
    ),
    tag = "inference",
)]
pub async fn show_pricing(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<PricingQuery>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(types::ok(pricing_response(&state.db, query)?))
}

pub(crate) fn pricing_response(
    db: &NeunodeDb,
    query: PricingQuery,
) -> Result<PricingResponse, ApiError> {
    if query.model.is_empty() {
        return Err(ApiError::BadRequest("model cannot be empty".to_string()));
    }
    if query.input_tokens == 0 && query.output_tokens == 0 {
        return Err(ApiError::BadRequest(
            "at least one of input_tokens or output_tokens must be > 0".to_string(),
        ));
    }

    let providers = load_all_providers(db);
    let model_info = providers
        .iter()
        .find_map(|p| p.find_model(&query.model))
        .ok_or_else(|| ApiError::NotFound("no registered pricing for model".into()))?;
    let input_rate = model_info.input_price_per_million.0;
    let input_cost = (input_rate / 1_000_000)
        .checked_mul(u128::from(query.input_tokens))
        .and_then(|whole| {
            whole.checked_add(input_rate % 1_000_000 * u128::from(query.input_tokens) / 1_000_000)
        })
        .ok_or_else(|| ApiError::BadRequest("input price exceeds amount bounds".into()))?;
    let output_rate = model_info.output_price_per_million.0;
    let output_cost = (output_rate / 1_000_000)
        .checked_mul(u128::from(query.output_tokens))
        .and_then(|whole| {
            whole.checked_add(output_rate % 1_000_000 * u128::from(query.output_tokens) / 1_000_000)
        })
        .ok_or_else(|| ApiError::BadRequest("output price exceeds amount bounds".into()))?;
    let total_cost =
        crate::inference_service::price(query.input_tokens, query.output_tokens, model_info)?;
    let protocol_fee = total_cost / 50 + u128::from(total_cost % 50 != 0);
    let net_payout = total_cost - protocol_fee;

    Ok(PricingResponse {
        model: query.model,
        input_tokens: query.input_tokens,
        output_tokens: query.output_tokens,
        input_cost: input_cost.to_string(),
        output_cost: output_cost.to_string(),
        total_cost: total_cost.to_string(),
        protocol_fee: protocol_fee.to_string(),
        net_payout: net_payout.to_string(),
    })
}
