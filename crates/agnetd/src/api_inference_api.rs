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

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InferenceRequest {
    pub model: String,
    pub prompt: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temp")]
    pub temperature: f64,
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct InferenceResponse {
    pub model: String,
    pub prompt: String,
    pub max_tokens: u32,
    pub temperature: f64,
    pub estimated_input_tokens: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<PricingEstimate>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PricingEstimate {
    pub input_price_per_mtok: u64,
    pub output_price_per_mtok: u64,
    pub estimated_cost: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ModelEntry {
    pub id: String,
    pub input_price_per_million: u64,
    pub output_price_per_million: u64,
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
    pub input_cost: u64,
    pub output_cost: u64,
    pub total_cost: u64,
    pub protocol_fee: u64,
    pub net_payout: u64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_all_providers(db: &NeunodeDb) -> Vec<InferenceProvider> {
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

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

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

    let providers = load_all_providers(&state.db);
    let pricing_info = providers.iter().find_map(|p| p.find_model(&body.model)).map(|m| {
        let cost =
            ((estimated_tokens as u128 * m.input_price_per_million.0 / 1_000_000).max(1)) as u64;
        PricingEstimate {
            input_price_per_mtok: m.input_price_per_million.0 as u64,
            output_price_per_mtok: m.output_price_per_million.0 as u64,
            estimated_cost: cost,
        }
    });

    Ok(types::ok(InferenceResponse {
        model: body.model,
        prompt: body.prompt,
        max_tokens: body.max_tokens,
        temperature: body.temperature,
        estimated_input_tokens: estimated_tokens,
        status: "submitted".to_string(),
        pricing: pricing_info,
    }))
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
            input_price_per_million: m.input_price_per_million.0 as u64,
            output_price_per_million: m.output_price_per_million.0 as u64,
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
    if query.model.is_empty() {
        return Err(ApiError::BadRequest("model cannot be empty".to_string()));
    }
    if query.input_tokens == 0 && query.output_tokens == 0 {
        return Err(ApiError::BadRequest(
            "at least one of input_tokens or output_tokens must be > 0".to_string(),
        ));
    }

    let providers = load_all_providers(&state.db);
    let model_info =
        providers.iter().find_map(|p| p.find_model(&query.model)).cloned().unwrap_or_else(|| {
            ModelInfo {
                id: query.model.clone(),
                base_model: None,
                context_length: 4096,
                input_price_per_million: TokenAmount(100),
                output_price_per_million: TokenAmount(200),
                capabilities: vec!["chat".to_string()],
            }
        });

    let input_cost =
        ((query.input_tokens as u128) * model_info.input_price_per_million.0 / 1_000_000) as u64;
    let output_cost =
        ((query.output_tokens as u128) * model_info.output_price_per_million.0 / 1_000_000) as u64;
    let total = input_cost.saturating_add(output_cost);
    let total_cost = if total == 0 && (query.input_tokens > 0 || query.output_tokens > 0) {
        1u64
    } else {
        total
    };

    let protocol_fee = ((total_cost as f64) * 2.0 / 100.0).ceil() as u64;
    let net_payout = total_cost.saturating_sub(protocol_fee);

    Ok(types::ok(PricingResponse {
        model: query.model,
        input_tokens: query.input_tokens,
        output_tokens: query.output_tokens,
        input_cost,
        output_cost,
        total_cost,
        protocol_fee,
        net_payout,
    }))
}
