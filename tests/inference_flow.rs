//! Integration tests for the inference marketplace flow.
//!
//! Verifies end-to-end: request validation → provider registration → routing
//! strategies → settlement calculation → batch settlement → verification hash,
//! integrating neunode-core, neunode-crypto, and neunode-inference crates.

use neunode_core::types::{Did, Hash256, Timestamp, TokenAmount};
use neunode_inference::openai::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, Choice, FinishReason, MessageRole,
    Usage,
};
use neunode_inference::provider::{InferenceProvider, ModelInfo, ProviderRegistry, ProviderStatus};
use neunode_inference::router::{Router, RoutingStrategy};
use neunode_inference::settlement::{
    PricingConfig, SettlementEngine, SettlementParams, SettlementResult,
};

fn test_did(n: u32) -> Did {
    Did(format!("did:neunode:0x{n:040x}"))
}

fn make_model(model_id: &str, input_price: u64, output_price: u64) -> ModelInfo {
    ModelInfo {
        id: model_id.to_string(),
        base_model: None,
        context_length: 4096,
        input_price_per_million: TokenAmount(input_price),
        output_price_per_million: TokenAmount(output_price),
        capabilities: vec!["chat".to_string(), "streaming".to_string()],
    }
}

fn make_provider(
    did_num: u32,
    model_id: &str,
    input_price: u64,
    output_price: u64,
    rep: f64,
    latency_ms: u32,
) -> InferenceProvider {
    InferenceProvider {
        did: test_did(did_num),
        name: format!("provider-{did_num}"),
        endpoint: format!("https://p{did_num}.neunode.io/v1"),
        models: vec![make_model(model_id, input_price, output_price)],
        reputation_score: rep,
        stake_amount: TokenAmount(1000),
        status: ProviderStatus::Online,
        last_heartbeat: 1000,
        total_requests_served: 0,
        avg_latency_ms: latency_ms,
    }
}

fn make_request(model: &str, content: &str) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: content.to_string(),
            name: None,
        }],
        temperature: Some(0.7),
        max_tokens: Some(2_000_000),
        top_p: None,
        stream: None,
        stop: None,
        frequency_penalty: None,
        presence_penalty: None,
    }
}

/// Request with 4K char content — estimate ~1000 input tokens
fn make_large_request(model: &str) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: "x".repeat(4_000_000), // ~1M estimated tokens
            name: None,
        }],
        temperature: Some(0.7),
        max_tokens: Some(2_000_000),
        top_p: None,
        stream: None,
        stop: None,
        frequency_penalty: None,
        presence_penalty: None,
    }
}

fn make_response(
    model: &str,
    input_tokens: u32,
    output_tokens: u32,
    ts: Timestamp,
) -> ChatCompletionResponse {
    ChatCompletionResponse {
        id: format!("chatcmpl-{ts}"),
        object: "chat.completion".to_string(),
        created: ts,
        model: model.to_string(),
        choices: vec![Choice {
            index: 0,
            message: ChatMessage {
                role: MessageRole::Assistant,
                content: "Generated response".to_string(),
                name: None,
            },
            finish_reason: FinishReason::Stop,
        }],
        usage: Usage {
            prompt_tokens: input_tokens,
            completion_tokens: output_tokens,
            total_tokens: input_tokens + output_tokens,
        },
    }
}

// ---------------------------------------------------------------------------
// Test 1: Full inference flow — validate → register → route → settle
// ---------------------------------------------------------------------------

#[test]
fn full_inference_request_to_settlement_flow() {
    let request = make_request("neunode/llama-3b", "What is 2+2?");
    request.validate().expect("request should validate");

    let mut registry = ProviderRegistry::new();
    let p1 = make_provider(1, "neunode/llama-3b", 100, 200, 80.0, 150);
    registry.register(p1).expect("register provider");

    let provider_refs = registry.providers_for_model("neunode/llama-3b");
    assert_eq!(provider_refs.len(), 1);

    let router = Router::new(RoutingStrategy::Cheapest);
    let providers: Vec<InferenceProvider> = provider_refs.into_iter().cloned().collect();
    let chosen = router.route(&providers, "neunode/llama-3b", None).expect("should route");
    assert_eq!(chosen.did, test_did(1));

    let response = make_response("neunode/llama-3b", 1, 100, 1700000000);
    let engine = SettlementEngine::new(PricingConfig::default());
    let model_info = make_model("neunode/llama-3b", 100, 200);

    let result = engine
        .settle(&request, &response, test_did(100), test_did(1), &model_info, 1700000000)
        .expect("settle");

    assert_eq!(result.requester_did, test_did(100));
    assert_eq!(result.provider_did, test_did(1));
    assert_eq!(result.input_tokens, 1);
    assert_eq!(result.output_tokens, 100);
    assert!(result.gross_cost.0 > 0);
    assert!(result.protocol_fee.0 > 0);
    assert!(result.net_payout.0 >= 0);
    assert_eq!(result.verification_hash.0.len(), 64);
}

// ---------------------------------------------------------------------------
// Test 2: Request validation — valid, empty messages, no user, bad params
// ---------------------------------------------------------------------------

#[test]
fn request_validation_comprehensive() {
    let valid = make_request("model", "hello");
    valid.validate().expect("valid request should pass");

    let empty_msgs = ChatCompletionRequest { messages: vec![], ..make_request("model", "x") };
    assert!(empty_msgs.validate().is_err(), "empty messages should fail");

    let no_user = ChatCompletionRequest {
        messages: vec![ChatMessage {
            role: MessageRole::System,
            content: "sys".to_string(),
            name: None,
        }],
        ..make_request("model", "x")
    };
    assert!(no_user.validate().is_err(), "no user message should fail");

    let bad_temp = ChatCompletionRequest { temperature: Some(5.0), ..make_request("model", "x") };
    assert!(bad_temp.validate().is_err(), "temperature > 2.0 should fail");

    let zero_tokens = ChatCompletionRequest { max_tokens: Some(0), ..make_request("model", "x") };
    assert!(zero_tokens.validate().is_err(), "max_tokens=0 should fail");

    let ok_boundary = ChatCompletionRequest {
        temperature: Some(2.0),
        max_tokens: Some(1),
        ..make_request("model", "x")
    };
    assert!(ok_boundary.validate().is_ok(), "boundary values should pass");
}

// ---------------------------------------------------------------------------
// Test 3: Provider registry — register, deregister, heartbeat, stale removal
// ---------------------------------------------------------------------------

#[test]
fn provider_registry_lifecycle() {
    let mut registry = ProviderRegistry::new();

    let p1 = make_provider(1, "llama-3b", 100, 200, 80.0, 100);
    let p2 = make_provider(2, "llama-3b", 150, 250, 90.0, 80);
    registry.register(p1).expect("register p1");
    registry.register(p2).expect("register p2");
    assert_eq!(registry.online_count(), 2);

    let providers = registry.providers_for_model("llama-3b");
    assert_eq!(providers.len(), 2);

    let got = registry.get(&test_did(1)).expect("should exist");
    assert!(got.has_model("llama-3b"));
    assert!(!got.has_model("gpt-4"));

    let mut p3 = make_provider(3, "gpt-4", 500, 1000, 70.0, 200);
    p3.last_heartbeat = 100;
    p3.status = ProviderStatus::Online;
    registry.register(p3).expect("register p3");

    let stale = registry.remove_stale(1000, 500);
    assert_eq!(stale.len(), 1, "p3 should be stale");
    assert_eq!(registry.all_providers().len(), 2);

    registry.update_heartbeat(&test_did(1), 2000).expect("heartbeat");
    assert_eq!(registry.get(&test_did(1)).expect("exists").last_heartbeat, 2000);
}

// ---------------------------------------------------------------------------
// Test 4: All routing strategies — cheapest, fastest, reputation, random, round-robin
// ---------------------------------------------------------------------------

#[test]
fn all_routing_strategies_select_correctly() {
    let providers = vec![
        make_provider(1, "llama-3b", 1000, 2000, 30.0, 300),
        make_provider(2, "llama-3b", 500, 500, 95.0, 50),
        make_provider(3, "llama-3b", 2000, 3000, 60.0, 150),
    ];

    let cheapest = Router::new(RoutingStrategy::Cheapest);
    assert_eq!(cheapest.route(&providers, "llama-3b", None).expect("ok").did, test_did(2));

    let fastest = Router::new(RoutingStrategy::Fastest);
    assert_eq!(fastest.route(&providers, "llama-3b", None).expect("ok").did, test_did(2));

    let best_rep = Router::new(RoutingStrategy::HighestReputation);
    assert_eq!(best_rep.route(&providers, "llama-3b", None).expect("ok").did, test_did(2));

    let random = Router::new(RoutingStrategy::Random);
    let r0 = random.route(&providers, "llama-3b", Some(0)).expect("ok");
    let r1 = random.route(&providers, "llama-3b", Some(1)).expect("ok");
    assert_ne!(r0.did, r1.did, "different seeds should select different providers");

    let rr = Router::new(RoutingStrategy::RoundRobin);
    let a = rr.route(&providers, "llama-3b", None).expect("ok");
    let b = rr.route(&providers, "llama-3b", None).expect("ok");
    let c = rr.route(&providers, "llama-3b", None).expect("ok");
    let d = rr.route(&providers, "llama-3b", None).expect("ok");
    assert_ne!(a.did, b.did);
    assert_ne!(b.did, c.did);
    assert_eq!(a.did, d.did, "round-robin should cycle back");
}

// ---------------------------------------------------------------------------
// Test 5: Settlement cost calculation — basic, sub-million, zero, minimum
// ---------------------------------------------------------------------------

#[test]
fn settlement_cost_calculation_edge_cases() {
    let exact =
        SettlementEngine::calculate_cost(1_000_000, 1_000_000, TokenAmount(100), TokenAmount(200));
    assert_eq!(exact, TokenAmount(300));

    let sub =
        SettlementEngine::calculate_cost(500_000, 500_000, TokenAmount(100), TokenAmount(200));
    assert_eq!(sub, TokenAmount(150));

    let zero = SettlementEngine::calculate_cost(0, 0, TokenAmount(100), TokenAmount(200));
    assert_eq!(zero, TokenAmount(0));

    let minimum = SettlementEngine::calculate_cost(100, 50, TokenAmount(100), TokenAmount(200));
    assert_eq!(minimum, TokenAmount(1), "sub-million tokens should floor to minimum 1");

    let input_only =
        SettlementEngine::calculate_cost(1_000_000, 0, TokenAmount(500), TokenAmount(0));
    assert_eq!(input_only, TokenAmount(500));

    let output_only =
        SettlementEngine::calculate_cost(0, 1_000_000, TokenAmount(0), TokenAmount(300));
    assert_eq!(output_only, TokenAmount(300));
}

// ---------------------------------------------------------------------------
// Test 6: Settlement with fee — default 2%, custom, zero
// ---------------------------------------------------------------------------

#[test]
fn settlement_fee_calculation() {
    let request = make_request("model", "hello");
    let response = make_response("model", 1, 100, 1700000000);
    let model = make_model("model", 100, 200);

    let default_engine = SettlementEngine::new(PricingConfig::default());
    let result = default_engine
        .settle(&request, &response, test_did(1), test_did(2), &model, 1700000000)
        .expect("settle");
    assert_eq!(result.gross_cost, TokenAmount(1));
    assert_eq!(result.protocol_fee, TokenAmount(1), "ceil(1 * 200 / 10000) = 1");
    assert_eq!(result.net_payout, TokenAmount(0));

    let custom_engine = SettlementEngine::new(PricingConfig {
        protocol_fee_bps: 500,
        default_model: "test".to_string(),
    });
    let result5 = custom_engine
        .settle(&request, &response, test_did(1), test_did(2), &model, 1700000000)
        .expect("settle");
    assert_eq!(result5.protocol_fee, TokenAmount(1), "ceil(1 * 500 / 10000) = 1");

    let free_engine = SettlementEngine::new(PricingConfig {
        protocol_fee_bps: 0,
        default_model: "test".to_string(),
    });
    let result0 = free_engine
        .settle(&request, &response, test_did(1), test_did(2), &model, 1700000000)
        .expect("settle");
    assert_eq!(result0.protocol_fee, TokenAmount(0));
    assert_eq!(result0.net_payout, result0.gross_cost);
}

// ---------------------------------------------------------------------------
// Test 7: Verification hash determinism and uniqueness
// ---------------------------------------------------------------------------

#[test]
fn verification_hash_deterministic_and_unique() {
    let did1 = Did("did:neunode:test_requester".to_string());
    let did2 = Did("did:neunode:test_provider".to_string());
    let h1 = SettlementEngine::calculate_verification_hash("model-a", &did1, &did2, "req", "resp");
    let h2 = SettlementEngine::calculate_verification_hash("model-a", &did1, &did2, "req", "resp");
    assert_eq!(h1, h2, "same inputs should produce same hash");

    let h3 = SettlementEngine::calculate_verification_hash("model-b", &did1, &did2, "req", "resp");
    assert_ne!(h1, h3, "different model should produce different hash");

    let h4 = SettlementEngine::calculate_verification_hash("model-a", &did1, &did2, "req2", "resp");
    assert_ne!(h1, h4, "different request should produce different hash");

    let did3 = Did("did:neunode:other_requester".to_string());
    let h5 = SettlementEngine::calculate_verification_hash("model-a", &did3, &did2, "req", "resp");
    assert_ne!(h1, h5, "different requester should produce different hash");

    assert_eq!(h1.0.len(), 64, "hash should be 64 hex chars");
    assert!(h1.0.chars().all(|c| c.is_ascii_hexdigit()), "hash should be hex");
}

// ---------------------------------------------------------------------------
// Test 8: Batch settlement processes multiple requests correctly
// ---------------------------------------------------------------------------

#[test]
fn batch_settle_multiple_requests() {
    let engine = SettlementEngine::new(PricingConfig::default());
    let model = make_model("neunode/llama-3b", 100, 200);
    let request = make_request("neunode/llama-3b", "batch test");
    let response = make_response("neunode/llama-3b", 1, 50, 1700000000);

    let params = vec![
        SettlementParams {
            request: &request,
            response: &response,
            requester: test_did(10),
            provider: test_did(1),
            model_info: &model,
            timestamp: 1700000000,
        },
        SettlementParams {
            request: &request,
            response: &response,
            requester: test_did(20),
            provider: test_did(2),
            model_info: &model,
            timestamp: 1700000001,
        },
        SettlementParams {
            request: &request,
            response: &response,
            requester: test_did(30),
            provider: test_did(3),
            model_info: &model,
            timestamp: 1700000002,
        },
    ];

    let results = engine.batch_settle(params).expect("batch settle");
    assert_eq!(results.len(), 3);

    assert_eq!(results[0].requester_did, test_did(10));
    assert_eq!(results[1].requester_did, test_did(20));
    assert_eq!(results[2].requester_did, test_did(30));

    assert_ne!(results[0].provider_did, results[1].provider_did);
    assert_eq!(results[0].gross_cost, results[1].gross_cost, "same tokens should have same cost");

    let empty = engine.batch_settle(vec![]).expect("empty batch");
    assert!(empty.is_empty());
}

// ---------------------------------------------------------------------------
// Test 9: route_top_n returns correctly ordered providers
// ---------------------------------------------------------------------------

#[test]
fn route_top_n_ordered_by_strategy() {
    let providers = vec![
        make_provider(1, "llama-3b", 1000, 2000, 30.0, 300),
        make_provider(2, "llama-3b", 500, 500, 95.0, 50),
        make_provider(3, "llama-3b", 2000, 3000, 60.0, 150),
    ];

    let cheapest = Router::new(RoutingStrategy::Cheapest);
    let top2 = cheapest.route_top_n(&providers, "llama-3b", 2, None).expect("top n");
    assert_eq!(top2.len(), 2);
    assert_eq!(top2[0].did, test_did(2), "cheapest first");
    assert_eq!(top2[1].did, test_did(1), "second cheapest");

    let rep = Router::new(RoutingStrategy::HighestReputation);
    let top3 = rep.route_top_n(&providers, "llama-3b", 5, None).expect("top n");
    assert_eq!(top3.len(), 3, "requested 5, only 3 available");
    assert_eq!(top3[0].did, test_did(2), "highest rep first");

    let empty = Router::new(RoutingStrategy::Cheapest);
    assert!(empty.route_top_n(&[], "llama-3b", 3, None).is_err());
}

// ---------------------------------------------------------------------------
// Test 10: Serde roundtrip for request, response, and settlement result
// ---------------------------------------------------------------------------

#[test]
fn serde_roundtrip_all_inference_types() {
    let req = make_request("neunode/llama-3b", "serde test");
    let req_json = serde_json::to_string(&req).expect("serialize request");
    let req_back: ChatCompletionRequest =
        serde_json::from_str(&req_json).expect("deserialize request");
    assert_eq!(req, req_back);

    let resp = make_response("neunode/llama-3b", 100, 50, 1700000000);
    let resp_json = serde_json::to_string(&resp).expect("serialize response");
    let resp_back: ChatCompletionResponse =
        serde_json::from_str(&resp_json).expect("deserialize response");
    assert_eq!(resp, resp_back);

    let result = SettlementResult {
        requester_did: test_did(1),
        provider_did: test_did(2),
        model_id: "neunode/llama-3b".to_string(),
        input_tokens: 100,
        output_tokens: 50,
        gross_cost: TokenAmount(300),
        protocol_fee: TokenAmount(6),
        net_payout: TokenAmount(294),
        verification_hash: Hash256("abc123def456".to_string()),
        timestamp: 1700000000,
    };
    let result_json = serde_json::to_string(&result).expect("serialize result");
    let result_back: SettlementResult =
        serde_json::from_str(&result_json).expect("deserialize result");
    assert_eq!(result, result_back);

    for strategy in [
        RoutingStrategy::Cheapest,
        RoutingStrategy::Fastest,
        RoutingStrategy::HighestReputation,
        RoutingStrategy::Random,
        RoutingStrategy::RoundRobin,
    ] {
        let json = serde_json::to_string(&strategy).expect("serialize strategy");
        let back: RoutingStrategy = serde_json::from_str(&json).expect("deserialize strategy");
        assert_eq!(strategy, back);
    }
}
