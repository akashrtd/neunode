use neunode_core::types::{Did, Hash256, Timestamp, TokenAmount};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{InferenceError, Result};
use crate::openai::{ChatCompletionChunk, ChatCompletionRequest};
use crate::provider::ModelInfo;
use crate::settlement::{PricingConfig, SettlementEngine, SettlementResult};

// ---------------------------------------------------------------------------
// Streaming accumulator
// ---------------------------------------------------------------------------

/// Tracks cumulative token usage across SSE chunks for a single request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct StreamingAccumulator {
    pub session_id: String,
    pub requester_did: Did,
    pub provider_did: Did,
    pub model_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub chunk_count: u32,
    pub started_at: Timestamp,
    pub last_chunk_at: Timestamp,
    pub settled: bool,
}

impl StreamingAccumulator {
    pub fn new(
        session_id: String,
        requester: Did,
        provider: Did,
        model_id: String,
        input_tokens: u32,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            session_id,
            requester_did: requester,
            provider_did: provider,
            model_id,
            input_tokens,
            output_tokens: 0,
            chunk_count: 0,
            started_at: timestamp,
            last_chunk_at: timestamp,
            settled: false,
        }
    }

    /// Append a streaming chunk. Each chunk is estimated at 1 token (one SSE
    /// delta typically carries a few characters). The final chunk with
    /// `finish_reason: Some(_)` carries the authoritative usage from the
    /// provider if available.
    pub fn append_chunk(&mut self, chunk: &ChatCompletionChunk, timestamp: Timestamp) {
        self.chunk_count += 1;
        self.last_chunk_at = timestamp;

        // Estimate: each non-empty delta choice ~1 token
        let has_content =
            chunk.choices.iter().any(|c| !c.delta.content.is_empty() || c.delta.name.is_some());

        if has_content {
            self.output_tokens = self.output_tokens.saturating_add(1);
        }

        // Final chunk: provider may include exact usage via the `usage` field
        // (OpenAI streaming extensions). We trust that over our estimate.
        let is_final = chunk.choices.iter().any(|c| c.finish_reason.is_some());
        if is_final {
            self.settled = true;
        }
    }

    /// Override output token count with an authoritative value (e.g. from a
    /// provider's final usage report).
    pub fn set_output_tokens(&mut self, tokens: u32) {
        self.output_tokens = tokens;
    }

    /// Compute the running cost so far.
    pub fn running_cost(
        &self,
        model_info: &ModelInfo,
        _config: &PricingConfig,
    ) -> Result<TokenAmount> {
        Ok(SettlementEngine::calculate_cost(
            self.input_tokens,
            self.output_tokens,
            model_info.input_price_per_million,
            model_info.output_price_per_million,
        ))
    }

    /// Finalize settlement once all chunks are received.
    pub fn finalize(
        &self,
        model_info: &ModelInfo,
        config: &PricingConfig,
        verification_hash: Hash256,
    ) -> Result<SettlementResult> {
        if !self.settled {
            return Err(InferenceError::SettlementFailed("stream not yet finished".to_string()));
        }

        let gross_cost = SettlementEngine::calculate_cost(
            self.input_tokens,
            self.output_tokens,
            model_info.input_price_per_million,
            model_info.output_price_per_million,
        );

        let fee_amount = (gross_cost.0 * config.protocol_fee_bps).div_ceil(10_000);
        let protocol_fee = TokenAmount(fee_amount);
        let net_payout = gross_cost
            .checked_sub(protocol_fee)
            .ok_or(InferenceError::FeeExceedsGross { fee: protocol_fee, gross: gross_cost })?;

        Ok(SettlementResult {
            requester_did: self.requester_did.clone(),
            provider_did: self.provider_did.clone(),
            model_id: self.model_id.clone(),
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            gross_cost,
            protocol_fee,
            net_payout,
            verification_hash,
            timestamp: self.last_chunk_at,
        })
    }
}

// ---------------------------------------------------------------------------
// Streaming settlement engine
// ---------------------------------------------------------------------------

/// Manages multiple concurrent streaming settlements.
#[derive(Debug)]
pub struct StreamingSettlementEngine {
    config: PricingConfig,
    sessions: std::collections::HashMap<String, StreamingAccumulator>,
}

impl StreamingSettlementEngine {
    pub fn new(config: PricingConfig) -> Self {
        Self { config, sessions: std::collections::HashMap::new() }
    }

    /// Start a new streaming session.
    pub fn start_session(
        &mut self,
        session_id: String,
        request: &ChatCompletionRequest,
        requester: Did,
        provider: Did,
        model_info: &ModelInfo,
        timestamp: Timestamp,
    ) -> Result<()> {
        if self.sessions.contains_key(&session_id) {
            return Err(InferenceError::SettlementFailed(format!(
                "session {session_id} already exists"
            )));
        }

        // Estimate input tokens from request
        let input_tokens = request.estimate_tokens();

        let acc = StreamingAccumulator::new(
            session_id.clone(),
            requester,
            provider,
            model_info.id.clone(),
            input_tokens,
            timestamp,
        );
        self.sessions.insert(session_id, acc);
        Ok(())
    }

    /// Feed a chunk to the appropriate session.
    pub fn on_chunk(
        &mut self,
        session_id: &str,
        chunk: &ChatCompletionChunk,
        timestamp: Timestamp,
    ) -> Result<()> {
        let acc = self.sessions.get_mut(session_id).ok_or_else(|| {
            InferenceError::SettlementFailed(format!("unknown session: {session_id}"))
        })?;
        acc.append_chunk(chunk, timestamp);
        Ok(())
    }

    /// Get the running cost for an active session.
    pub fn running_cost(&self, session_id: &str, model_info: &ModelInfo) -> Result<TokenAmount> {
        let acc = self.sessions.get(session_id).ok_or_else(|| {
            InferenceError::SettlementFailed(format!("unknown session: {session_id}"))
        })?;
        acc.running_cost(model_info, &self.config)
    }

    /// Finalize a completed session and return the settlement.
    pub fn finalize(
        &mut self,
        session_id: &str,
        request: &ChatCompletionRequest,
        model_info: &ModelInfo,
    ) -> Result<SettlementResult> {
        let acc = self.sessions.get(session_id).ok_or_else(|| {
            InferenceError::SettlementFailed(format!("unknown session: {session_id}"))
        })?;

        if !acc.settled {
            return Err(InferenceError::SettlementFailed(
                "stream not yet finished — wait for final chunk".to_string(),
            ));
        }

        let request_json = serde_json::to_string(request)
            .map_err(|e| InferenceError::SettlementFailed(e.to_string()))?;
        let verification_hash = SettlementEngine::calculate_verification_hash(
            &model_info.id,
            &request_json,
            &format!(
                "streaming:{}:chunks:{}:tokens:{}",
                session_id, acc.chunk_count, acc.output_tokens
            ),
        );

        let result = acc.finalize(model_info, &self.config, verification_hash)?;
        self.sessions.remove(session_id);
        Ok(result)
    }

    /// Force-settle a session even if the final chunk was not received (e.g.
    /// timeout). Uses the accumulated token count as-is.
    pub fn force_settle(
        &mut self,
        session_id: &str,
        request: &ChatCompletionRequest,
        model_info: &ModelInfo,
    ) -> Result<SettlementResult> {
        let acc = self.sessions.get_mut(session_id).ok_or_else(|| {
            InferenceError::SettlementFailed(format!("unknown session: {session_id}"))
        })?;
        acc.settled = true;

        let request_json = serde_json::to_string(request)
            .map_err(|e| InferenceError::SettlementFailed(e.to_string()))?;
        let verification_hash = SettlementEngine::calculate_verification_hash(
            &model_info.id,
            &request_json,
            &format!(
                "force_settle:{}:chunks:{}:tokens:{}",
                session_id, acc.chunk_count, acc.output_tokens
            ),
        );

        let result = acc.finalize(model_info, &self.config, verification_hash)?;
        self.sessions.remove(session_id);
        Ok(result)
    }

    /// List all active (non-finalized) session IDs.
    pub fn active_sessions(&self) -> Vec<&str> {
        self.sessions.keys().map(|s| s.as_str()).collect()
    }

    /// Number of active sessions.
    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }

    /// Abort a session without settlement (e.g. client disconnect).
    pub fn abort(&mut self, session_id: &str) -> Result<()> {
        self.sessions
            .remove(session_id)
            .ok_or_else(|| {
                InferenceError::SettlementFailed(format!("unknown session: {session_id}"))
            })
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai::{ChatMessage, ChunkChoice, FinishReason, MessageRole};

    fn test_did(n: u32) -> Did {
        Did(format!("did:neunode:0x{n:040x}"))
    }

    fn test_model_info() -> ModelInfo {
        ModelInfo {
            id: "neunode/llama-3b".to_string(),
            base_model: None,
            context_length: 4096,
            input_price_per_million: TokenAmount(100),
            output_price_per_million: TokenAmount(200),
            capabilities: vec!["chat".to_string(), "streaming".to_string()],
        }
    }

    fn test_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "neunode/llama-3b".to_string(),
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "Hello".to_string(),
                name: None,
            }],
            temperature: None,
            max_tokens: None,
            top_p: None,
            stream: Some(true),
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
        }
    }

    fn content_chunk(content: &str) -> ChatCompletionChunk {
        ChatCompletionChunk {
            id: "chatcmpl-test".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1700000000,
            model: "neunode/llama-3b".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChatMessage {
                    role: MessageRole::Assistant,
                    content: content.to_string(),
                    name: None,
                },
                finish_reason: None,
            }],
        }
    }

    fn final_chunk() -> ChatCompletionChunk {
        ChatCompletionChunk {
            id: "chatcmpl-test".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1700000000,
            model: "neunode/llama-3b".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChatMessage {
                    role: MessageRole::Assistant,
                    content: String::new(),
                    name: None,
                },
                finish_reason: Some(FinishReason::Stop),
            }],
        }
    }

    // --- Accumulator tests ---

    #[test]
    fn accumulator_new() {
        let acc = StreamingAccumulator::new(
            "sess-1".into(),
            test_did(1),
            test_did(2),
            "neunode/llama-3b".into(),
            10,
            1700000000,
        );
        assert_eq!(acc.input_tokens, 10);
        assert_eq!(acc.output_tokens, 0);
        assert_eq!(acc.chunk_count, 0);
        assert!(!acc.settled);
    }

    #[test]
    fn accumulator_append_chunks() {
        let mut acc = StreamingAccumulator::new(
            "sess-1".into(),
            test_did(1),
            test_did(2),
            "neunode/llama-3b".into(),
            10,
            1700000000,
        );
        acc.append_chunk(&content_chunk("Hello"), 1700000001);
        acc.append_chunk(&content_chunk(" world"), 1700000002);
        assert_eq!(acc.output_tokens, 2);
        assert_eq!(acc.chunk_count, 2);
        assert!(!acc.settled);
    }

    #[test]
    fn accumulator_final_chunk_marks_settled() {
        let mut acc = StreamingAccumulator::new(
            "sess-1".into(),
            test_did(1),
            test_did(2),
            "neunode/llama-3b".into(),
            10,
            1700000000,
        );
        acc.append_chunk(&content_chunk("Hi"), 1700000001);
        acc.append_chunk(&final_chunk(), 1700000002);
        assert!(acc.settled);
        // final chunk has empty content, so only 1 output token
        assert_eq!(acc.output_tokens, 1);
        assert_eq!(acc.chunk_count, 2);
    }

    #[test]
    fn accumulator_set_output_tokens() {
        let mut acc = StreamingAccumulator::new(
            "sess-1".into(),
            test_did(1),
            test_did(2),
            "neunode/llama-3b".into(),
            10,
            1700000000,
        );
        acc.append_chunk(&content_chunk("x"), 1700000001);
        assert_eq!(acc.output_tokens, 1);
        acc.set_output_tokens(50);
        assert_eq!(acc.output_tokens, 50);
    }

    #[test]
    fn accumulator_running_cost() {
        let model = test_model_info();
        let config = PricingConfig::default();
        let mut acc = StreamingAccumulator::new(
            "sess-1".into(),
            test_did(1),
            test_did(2),
            "neunode/llama-3b".into(),
            1_000_000,
            1700000000,
        );
        acc.append_chunk(&content_chunk("a"), 1700000001);
        acc.append_chunk(&content_chunk("b"), 1700000002);

        let cost = acc.running_cost(&model, &config).unwrap();
        // input: 1M tokens * 100 / 1M = 100; output: 2 * 200 / 1M = 0 (400/1M = 0)
        // total = (100_000_000 + 400) / 1_000_000 = 100
        assert_eq!(cost, TokenAmount(100));
    }

    #[test]
    fn accumulator_finalize_before_settled_fails() {
        let model = test_model_info();
        let config = PricingConfig::default();
        let acc = StreamingAccumulator::new(
            "sess-1".into(),
            test_did(1),
            test_did(2),
            "neunode/llama-3b".into(),
            10,
            1700000000,
        );
        let result = acc.finalize(&model, &config, Hash256("abc".into()));
        assert!(result.is_err());
    }

    #[test]
    fn accumulator_finalize_after_settled() {
        let model = test_model_info();
        let config = PricingConfig::default();
        let mut acc = StreamingAccumulator::new(
            "sess-1".into(),
            test_did(1),
            test_did(2),
            "neunode/llama-3b".into(),
            1_000_000,
            1700000000,
        );
        acc.append_chunk(&content_chunk("Hello"), 1700000001);
        acc.append_chunk(&final_chunk(), 1700000002);

        let result = acc.finalize(&model, &config, Hash256("abc".into())).unwrap();
        assert_eq!(result.requester_did, test_did(1));
        assert_eq!(result.provider_did, test_did(2));
        assert_eq!(result.input_tokens, 1_000_000);
        assert_eq!(result.output_tokens, 1);
    }

    // --- Engine tests ---

    #[test]
    fn engine_start_session() {
        let mut engine = StreamingSettlementEngine::new(PricingConfig::default());
        let model = test_model_info();
        let req = test_request();

        engine
            .start_session("s1".into(), &req, test_did(1), test_did(2), &model, 1700000000)
            .unwrap();
        assert_eq!(engine.active_count(), 1);
        assert_eq!(engine.active_sessions(), vec!["s1"]);
    }

    #[test]
    fn engine_duplicate_session_fails() {
        let mut engine = StreamingSettlementEngine::new(PricingConfig::default());
        let model = test_model_info();
        let req = test_request();

        engine
            .start_session("s1".into(), &req, test_did(1), test_did(2), &model, 1700000000)
            .unwrap();
        assert!(engine
            .start_session("s1".into(), &req, test_did(1), test_did(2), &model, 1700000001)
            .is_err());
    }

    #[test]
    fn engine_on_chunk_unknown_session_fails() {
        let mut engine = StreamingSettlementEngine::new(PricingConfig::default());
        assert!(engine.on_chunk("unknown", &content_chunk("x"), 1700000000).is_err());
    }

    #[test]
    fn engine_full_stream_flow() {
        let mut engine = StreamingSettlementEngine::new(PricingConfig::default());
        let model = test_model_info();
        let req = test_request();

        engine
            .start_session("s1".into(), &req, test_did(1), test_did(2), &model, 1700000000)
            .unwrap();

        engine.on_chunk("s1", &content_chunk("Hello"), 1700000001).unwrap();
        engine.on_chunk("s1", &content_chunk(" world"), 1700000002).unwrap();
        engine.on_chunk("s1", &final_chunk(), 1700000003).unwrap();

        let cost = engine.running_cost("s1", &model).unwrap();
        assert!(cost.0 > 0);

        let result = engine.finalize("s1", &req, &model).unwrap();
        assert_eq!(result.output_tokens, 2);
        assert!(result.gross_cost.0 > 0);

        // Session removed after finalize
        assert_eq!(engine.active_count(), 0);
    }

    #[test]
    fn engine_finalize_before_final_chunk_fails() {
        let mut engine = StreamingSettlementEngine::new(PricingConfig::default());
        let model = test_model_info();
        let req = test_request();

        engine
            .start_session("s1".into(), &req, test_did(1), test_did(2), &model, 1700000000)
            .unwrap();
        engine.on_chunk("s1", &content_chunk("Hi"), 1700000001).unwrap();

        assert!(engine.finalize("s1", &req, &model).is_err());
    }

    #[test]
    fn engine_force_settle() {
        let mut engine = StreamingSettlementEngine::new(PricingConfig::default());
        let model = test_model_info();
        let req = test_request();

        engine
            .start_session("s1".into(), &req, test_did(1), test_did(2), &model, 1700000000)
            .unwrap();
        engine.on_chunk("s1", &content_chunk("Hi"), 1700000001).unwrap();

        // Force settle without final chunk
        let result = engine.force_settle("s1", &req, &model).unwrap();
        assert_eq!(result.output_tokens, 1);
        assert_eq!(engine.active_count(), 0);
    }

    #[test]
    fn engine_abort() {
        let mut engine = StreamingSettlementEngine::new(PricingConfig::default());
        let model = test_model_info();
        let req = test_request();

        engine
            .start_session("s1".into(), &req, test_did(1), test_did(2), &model, 1700000000)
            .unwrap();
        engine.abort("s1").unwrap();
        assert_eq!(engine.active_count(), 0);
    }

    #[test]
    fn engine_abort_unknown_fails() {
        let mut engine = StreamingSettlementEngine::new(PricingConfig::default());
        assert!(engine.abort("nope").is_err());
    }

    #[test]
    fn engine_multiple_concurrent_sessions() {
        let mut engine = StreamingSettlementEngine::new(PricingConfig::default());
        let model = test_model_info();
        let req = test_request();

        engine
            .start_session("s1".into(), &req, test_did(1), test_did(2), &model, 1700000000)
            .unwrap();
        engine
            .start_session("s2".into(), &req, test_did(3), test_did(4), &model, 1700000000)
            .unwrap();

        assert_eq!(engine.active_count(), 2);

        engine.on_chunk("s1", &content_chunk("A"), 1700000001).unwrap();
        engine.on_chunk("s2", &content_chunk("B"), 1700000001).unwrap();

        engine.on_chunk("s1", &final_chunk(), 1700000002).unwrap();
        let r1 = engine.finalize("s1", &req, &model).unwrap();
        assert_eq!(r1.output_tokens, 1);

        assert_eq!(engine.active_count(), 1);
        engine.abort("s2").unwrap();
        assert_eq!(engine.active_count(), 0);
    }
}
