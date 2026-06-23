use neunode_core::types::{Did, Hash256, Timestamp, TokenAmount};
use serde::{Deserialize, Serialize};

use crate::error::{InferenceError, Result};
use crate::openai::{ChatCompletionRequest, ChatCompletionResponse};
use crate::provider::ModelInfo;
use ts_rs::TS;

/// Maximum multiplier allowed between provider-reported and estimated token counts.
const TOKEN_TOLERANCE: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct PricingConfig {
    pub protocol_fee_bps: u64,
    pub default_model: String,
}

impl Default for PricingConfig {
    fn default() -> Self {
        Self { protocol_fee_bps: 200, default_model: "neunode/default".to_string() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct SettlementResult {
    pub requester_did: Did,
    pub provider_did: Did,
    pub model_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub gross_cost: TokenAmount,
    pub protocol_fee: TokenAmount,
    pub net_payout: TokenAmount,
    pub verification_hash: Hash256,
    pub timestamp: Timestamp,
}

pub struct SettlementParams<'a> {
    pub request: &'a ChatCompletionRequest,
    pub response: &'a ChatCompletionResponse,
    pub requester: Did,
    pub provider: Did,
    pub model_info: &'a ModelInfo,
    pub timestamp: Timestamp,
}

#[derive(Debug)]
pub struct SettlementEngine {
    config: PricingConfig,
}

impl SettlementEngine {
    pub fn new(config: PricingConfig) -> Self {
        Self { config }
    }

    pub fn calculate_cost(
        input_tokens: u32,
        output_tokens: u32,
        input_price: TokenAmount,
        output_price: TokenAmount,
    ) -> TokenAmount {
        let input_cost = (input_tokens as u128) * input_price.0;
        let output_cost = (output_tokens as u128) * output_price.0;
        let total = input_cost.saturating_add(output_cost) / 1_000_000;
        if total == 0 && (input_tokens > 0 || output_tokens > 0) {
            TokenAmount(1)
        } else {
            TokenAmount(total)
        }
    }

    /// Rough estimate of input tokens from request messages (~4 chars per token).
    pub fn estimate_input_tokens(request: &ChatCompletionRequest) -> u32 {
        let char_count: usize = request.messages.iter().map(|m| m.content.len()).sum();
        (char_count as u32 / 4).max(1)
    }

    /// Validate provider-reported token counts against requester estimates.
    /// - Input tokens must not exceed estimated count * tolerance
    /// - Output tokens must not exceed max_tokens * tolerance (if set)
    pub fn validate_token_counts(
        request: &ChatCompletionRequest,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Result<()> {
        let estimated_input = Self::estimate_input_tokens(request);
        let max_input = estimated_input.saturating_mul(TOKEN_TOLERANCE);
        if input_tokens > max_input {
            return Err(InferenceError::TokenCountExceedsEstimate {
                provider_tokens: input_tokens,
                max_allowed: max_input,
                token_type: "input".to_string(),
            });
        }

        if let Some(max_tok) = request.max_tokens {
            let max_output = max_tok.saturating_mul(TOKEN_TOLERANCE);
            if output_tokens > max_output {
                return Err(InferenceError::TokenCountExceedsEstimate {
                    provider_tokens: output_tokens,
                    max_allowed: max_output,
                    token_type: "output".to_string(),
                });
            }
        }

        Ok(())
    }

    pub fn settle(
        &self,
        request: &ChatCompletionRequest,
        response: &ChatCompletionResponse,
        requester: Did,
        provider: Did,
        model_info: &ModelInfo,
        timestamp: Timestamp,
    ) -> Result<SettlementResult> {
        let input_tokens = response.usage.prompt_tokens;
        let output_tokens = response.usage.completion_tokens;

        Self::validate_token_counts(request, input_tokens, output_tokens)?;

        let gross_cost = Self::calculate_cost(
            input_tokens,
            output_tokens,
            model_info.input_price_per_million,
            model_info.output_price_per_million,
        );

        let fee_amount = (gross_cost.0 * self.config.protocol_fee_bps as u128).div_ceil(10_000);
        let protocol_fee = TokenAmount(fee_amount);
        let net_payout = gross_cost
            .checked_sub(protocol_fee)
            .ok_or(InferenceError::FeeExceedsGross { fee: protocol_fee, gross: gross_cost })?;

        let request_json = serde_json::to_string(request)
            .map_err(|e| InferenceError::SettlementFailed(e.to_string()))?;
        let response_json = serde_json::to_string(response)
            .map_err(|e| InferenceError::SettlementFailed(e.to_string()))?;
        let verification_hash = Self::calculate_verification_hash(
            &model_info.id,
            &requester,
            &provider,
            &request_json,
            &response_json,
        );

        Ok(SettlementResult {
            requester_did: requester,
            provider_did: provider,
            model_id: model_info.id.clone(),
            input_tokens,
            output_tokens,
            gross_cost,
            protocol_fee,
            net_payout,
            verification_hash,
            timestamp,
        })
    }

    pub fn calculate_verification_hash(
        model_id: &str,
        requester_did: &Did,
        provider_did: &Did,
        request_json: &str,
        response_json: &str,
    ) -> Hash256 {
        let combined =
            format!("{model_id}:{requester_did}:{provider_did}:{request_json}:{response_json}");
        let hash_bytes = neunode_crypto::hash::sha256(combined.as_bytes());
        Hash256(hex::encode(hash_bytes))
    }

    pub fn batch_settle<'a>(
        &self,
        settlements: Vec<SettlementParams<'a>>,
    ) -> Result<Vec<SettlementResult>> {
        settlements
            .into_iter()
            .map(|params| {
                self.settle(
                    params.request,
                    params.response,
                    params.requester,
                    params.provider,
                    params.model_info,
                    params.timestamp,
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai::{ChatMessage, FinishReason, MessageRole, Usage};
    use crate::provider::ModelInfo;
    use neunode_core::types::Did;

    fn test_did(n: u32) -> Did {
        Did(format!("did:neunode:0x{n:040x}"))
    }

    fn test_model_info(model_id: &str, input_price: u64, output_price: u64) -> ModelInfo {
        ModelInfo {
            id: model_id.to_string(),
            base_model: None,
            context_length: 4096,
            input_price_per_million: TokenAmount(input_price as u128),
            output_price_per_million: TokenAmount(output_price as u128),
            capabilities: vec!["chat".to_string()],
        }
    }

    fn test_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "neunode/llama-3b".to_string(),
            messages: vec![ChatMessage {
                role: MessageRole::User,
                // 4000 chars => ~1000 estimated input tokens, max allowed = 2000
                content: "a".repeat(4000),
                name: None,
            }],
            temperature: None,
            max_tokens: Some(1_000_000),
            top_p: None,
            stream: None,
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
        }
    }

    fn test_response(input_tokens: u32, output_tokens: u32) -> ChatCompletionResponse {
        ChatCompletionResponse {
            id: "chatcmpl-test".to_string(),
            object: "chat.completion".to_string(),
            created: 1700000000,
            model: "neunode/llama-3b".to_string(),
            choices: vec![crate::openai::Choice {
                index: 0,
                message: ChatMessage {
                    role: MessageRole::Assistant,
                    content: "Hi there!".to_string(),
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

    /// Response with 1K input + 1K output — realistic for test_request's 4K chars
    fn test_response_default() -> ChatCompletionResponse {
        test_response(1_000, 1_000)
    }

    #[test]
    fn pricing_config_default() {
        let config = PricingConfig::default();
        assert_eq!(config.protocol_fee_bps, 200);
        assert_eq!(config.default_model, "neunode/default");
    }

    #[test]
    fn calculate_cost_basic() {
        let cost = SettlementEngine::calculate_cost(
            1_000_000,
            1_000_000,
            TokenAmount(100),
            TokenAmount(200),
        );
        assert_eq!(cost, TokenAmount(300));
    }

    #[test]
    fn calculate_cost_sub_million() {
        let cost =
            SettlementEngine::calculate_cost(500_000, 500_000, TokenAmount(100), TokenAmount(200));
        assert_eq!(cost, TokenAmount(150));
    }

    #[test]
    fn calculate_cost_zero_tokens() {
        let cost = SettlementEngine::calculate_cost(0, 0, TokenAmount(100), TokenAmount(200));
        assert_eq!(cost, TokenAmount(0));
    }

    #[test]
    fn calculate_cost_minimum_one_when_tokens_used() {
        let cost = SettlementEngine::calculate_cost(100, 50, TokenAmount(100), TokenAmount(200));
        assert_eq!(cost, TokenAmount(1));
    }

    #[test]
    fn calculate_cost_large_numbers() {
        let cost = SettlementEngine::calculate_cost(
            10_000_000,
            10_000_000,
            TokenAmount(5_000_000),
            TokenAmount(10_000_000),
        );
        assert_eq!(cost, TokenAmount(150_000_000));
    }

    #[test]
    fn calculate_cost_input_only() {
        let cost = SettlementEngine::calculate_cost(1_000_000, 0, TokenAmount(500), TokenAmount(0));
        assert_eq!(cost, TokenAmount(500));
    }

    #[test]
    fn calculate_cost_output_only() {
        let cost = SettlementEngine::calculate_cost(0, 1_000_000, TokenAmount(0), TokenAmount(300));
        assert_eq!(cost, TokenAmount(300));
    }

    #[test]
    fn settle_basic() {
        let engine = SettlementEngine::new(PricingConfig::default());
        let model = test_model_info("neunode/llama-3b", 100, 200);
        let request = test_request();
        let response = test_response_default();

        let result = engine
            .settle(&request, &response, test_did(1), test_did(2), &model, 1700000000)
            .unwrap();

        assert_eq!(result.requester_did, test_did(1));
        assert_eq!(result.provider_did, test_did(2));
        assert_eq!(result.model_id, "neunode/llama-3b");
        assert_eq!(result.input_tokens, 1_000);
        assert_eq!(result.output_tokens, 1_000);
        assert_eq!(result.gross_cost, TokenAmount(1)); // 300k / 1M = 0.3 → min 1
        assert_eq!(result.protocol_fee, TokenAmount(1)); // ceil(1 * 200 / 10000) = 1
        assert_eq!(result.net_payout, TokenAmount(0));
        assert_eq!(result.timestamp, 1700000000);
    }

    #[test]
    fn settle_fee_calculation() {
        let engine =
            SettlementEngine::new(PricingConfig { protocol_fee_bps: 500, ..Default::default() });
        let model = test_model_info("test-model", 1000, 1000);
        let request = test_request();
        let response = test_response_default();

        let result = engine
            .settle(&request, &response, test_did(1), test_did(2), &model, 1700000000)
            .unwrap();

        assert_eq!(result.gross_cost, TokenAmount(2));
        assert_eq!(result.protocol_fee, TokenAmount(1));
        assert_eq!(result.net_payout, TokenAmount(1));
    }

    #[test]
    fn settle_zero_fee() {
        let engine = SettlementEngine::new(PricingConfig {
            protocol_fee_bps: 0,
            default_model: "test".to_string(),
        });
        let model = test_model_info("test-model", 100, 200);
        let request = test_request();
        let response = test_response_default();

        let result = engine
            .settle(&request, &response, test_did(1), test_did(2), &model, 1700000000)
            .unwrap();

        assert_eq!(result.protocol_fee, TokenAmount(0));
        assert_eq!(result.net_payout, result.gross_cost);
    }

    #[test]
    fn settle_verification_hash_deterministic() {
        let did1 = test_did(1);
        let did2 = test_did(2);
        let hash1 = SettlementEngine::calculate_verification_hash(
            "model-id",
            &did1,
            &did2,
            r#"{"key":"val"}"#,
            r#"{"result":42}"#,
        );
        let hash2 = SettlementEngine::calculate_verification_hash(
            "model-id",
            &did1,
            &did2,
            r#"{"key":"val"}"#,
            r#"{"result":42}"#,
        );
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn settle_verification_hash_differs_on_input() {
        let did1 = test_did(1);
        let did2 = test_did(2);
        let hash1 =
            SettlementEngine::calculate_verification_hash("model-a", &did1, &did2, "req", "resp");
        let hash2 =
            SettlementEngine::calculate_verification_hash("model-b", &did1, &did2, "req", "resp");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn settle_verification_hash_differs_on_requester() {
        let did1 = test_did(1);
        let did2 = test_did(2);
        let did3 = test_did(3);
        let hash1 =
            SettlementEngine::calculate_verification_hash("model-id", &did1, &did2, "req", "resp");
        let hash2 =
            SettlementEngine::calculate_verification_hash("model-id", &did3, &did2, "req", "resp");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn settle_verification_hash_is_hex() {
        let did1 = test_did(1);
        let did2 = test_did(2);
        let hash = SettlementEngine::calculate_verification_hash("m", &did1, &did2, "r", "s");
        assert_eq!(hash.0.len(), 64);
        assert!(hash.0.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn batch_settle_multiple() {
        let engine = SettlementEngine::new(PricingConfig::default());
        let model = test_model_info("neunode/llama-3b", 100, 200);
        let request = test_request();
        let response = test_response_default();

        let params = vec![
            SettlementParams {
                request: &request,
                response: &response,
                requester: test_did(1),
                provider: test_did(10),
                model_info: &model,
                timestamp: 1700000000,
            },
            SettlementParams {
                request: &request,
                response: &response,
                requester: test_did(2),
                provider: test_did(20),
                model_info: &model,
                timestamp: 1700000001,
            },
        ];

        let results = engine.batch_settle(params).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].requester_did, test_did(1));
        assert_eq!(results[1].requester_did, test_did(2));
        // Different requesters must produce different verification hashes
        assert_ne!(results[0].verification_hash, results[1].verification_hash);
    }

    #[test]
    fn batch_settle_empty() {
        let engine = SettlementEngine::new(PricingConfig::default());
        let results = engine.batch_settle(vec![]).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn settlement_result_serde_roundtrip() {
        let result = SettlementResult {
            requester_did: test_did(1),
            provider_did: test_did(2),
            model_id: "neunode/llama-3b".to_string(),
            input_tokens: 1000,
            output_tokens: 500,
            gross_cost: TokenAmount(300),
            protocol_fee: TokenAmount(6),
            net_payout: TokenAmount(294),
            verification_hash: Hash256("abc123".to_string()),
            timestamp: 1700000000,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: SettlementResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, back);
    }

    #[test]
    fn pricing_config_serde_roundtrip() {
        let config = PricingConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let back: PricingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, back);
    }

    // ─── Token Validation ─────────────────────────────────────────────────

    #[test]
    fn estimate_input_tokens_basic() {
        let request = test_request(); // 4000 chars => 1000 tokens
        let estimate = SettlementEngine::estimate_input_tokens(&request);
        assert_eq!(estimate, 1000);
    }

    #[test]
    fn estimate_input_tokens_long() {
        let request = ChatCompletionRequest {
            model: "test".to_string(),
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "a".repeat(4000),
                name: None,
            }],
            temperature: None,
            max_tokens: None,
            top_p: None,
            stream: None,
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
        };
        let estimate = SettlementEngine::estimate_input_tokens(&request);
        assert_eq!(estimate, 1000); // 4000 / 4
    }

    #[test]
    fn validate_token_counts_within_bounds() {
        let request = test_request();
        assert!(SettlementEngine::validate_token_counts(&request, 2, 50).is_ok());
    }

    #[test]
    fn validate_token_counts_rejects_inflated_input() {
        let request = test_request(); // 4000 chars => estimate 1000, max 2000
        let result = SettlementEngine::validate_token_counts(&request, 5000, 50);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("input"));
    }

    #[test]
    fn validate_token_counts_rejects_inflated_output() {
        let mut request = test_request();
        request.max_tokens = Some(100);
        // output 201 > 100 * 2
        let result = SettlementEngine::validate_token_counts(&request, 1, 201);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("output"));
    }

    #[test]
    fn validate_token_counts_no_max_tokens_allows_any_output() {
        let mut request = test_request();
        request.max_tokens = None;
        assert!(SettlementEngine::validate_token_counts(&request, 1, 999_999).is_ok());
    }

    #[test]
    fn settle_rejects_inflated_input() {
        let engine = SettlementEngine::new(PricingConfig::default());
        let model = test_model_info("test", 100, 200);
        let request = test_request(); // 4000 chars, estimate = 1000, max = 2000
                                      // Provider reports 5M input tokens — way over estimate
        let response = test_response(5_000_000, 100);

        let result = engine.settle(&request, &response, test_did(1), test_did(2), &model, 0);
        assert!(result.is_err());
    }
}
