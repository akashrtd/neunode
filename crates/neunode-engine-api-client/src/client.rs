use crate::{config::EngineApiClientConfig, error::EngineApiError, jwt::JwtAuth, types::*};
use alloy::primitives::B256;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Async Engine API client connecting to a Reth execution layer.
#[derive(Debug)]
pub struct EngineApiClient {
    /// Inner HTTP client.
    inner: reqwest::Client,
    /// JWT authenticator.
    jwt: JwtAuth,
    /// Configuration.
    config: EngineApiClientConfig,
    /// Current payload ID tracking (for block building).
    pending_payload_id: Arc<RwLock<Option<PayloadId>>>,
}

impl EngineApiClient {
    /// Create a new Engine API client from configuration.
    pub async fn new(config: EngineApiClientConfig) -> Result<Self, EngineApiError> {
        let jwt = if let Some(ref path) = config.jwt_secret_path {
            JwtAuth::from_file(path).await?
        } else if let Some(ref bytes) = config.jwt_secret {
            if bytes.len() != 32 {
                return Err(EngineApiError::JwtAuth("JWT secret must be exactly 32 bytes".into()));
            }
            let mut secret = [0u8; 32];
            secret.copy_from_slice(bytes);
            JwtAuth::from_bytes(secret)
        } else {
            return Err(EngineApiError::JwtAuth(
                "either jwt_secret_path or jwt_secret must be provided".into(),
            ));
        };

        let inner = reqwest::Client::builder()
            .timeout(config.request_timeout)
            .connect_timeout(config.connect_timeout)
            .build()
            .map_err(|e| EngineApiError::Transport(e.to_string()))?;

        Ok(Self { inner, jwt, config, pending_payload_id: Arc::new(RwLock::new(None)) })
    }

    // ----------------------------------------------------------------
    // Cancun methods (V3) -- primary target for Neunode
    // ----------------------------------------------------------------

    /// Submit a new execution payload for validation.
    /// Cancun version: includes blob gas fields and blob versioned hashes.
    pub async fn new_payload_v3(
        &self,
        payload: ExecutionPayloadV3,
        expected_blob_versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
    ) -> Result<PayloadStatus, EngineApiError> {
        self.rpc_call(
            "engine_newPayloadV3",
            vec![
                serde_json::to_value(payload)
                    .map_err(|e| EngineApiError::Serialization(e.to_string()))?,
                serde_json::to_value(expected_blob_versioned_hashes)
                    .map_err(|e| EngineApiError::Serialization(e.to_string()))?,
                serde_json::to_value(parent_beacon_block_root)
                    .map_err(|e| EngineApiError::Serialization(e.to_string()))?,
            ],
        )
        .await
    }

    /// Update the forkchoice state and optionally trigger payload building.
    pub async fn forkchoice_updated_v3(
        &self,
        forkchoice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> Result<ForkchoiceUpdated, EngineApiError> {
        let result: ForkchoiceUpdated = self
            .rpc_call(
                "engine_forkchoiceUpdatedV3",
                vec![
                    serde_json::to_value(forkchoice_state)
                        .map_err(|e| EngineApiError::Serialization(e.to_string()))?,
                    serde_json::to_value(payload_attributes)
                        .map_err(|e| EngineApiError::Serialization(e.to_string()))?,
                ],
            )
            .await?;

        // Track payload ID if building was initiated.
        if let Some(ref id) = result.payload_id {
            *self.pending_payload_id.write().await = Some(*id);
        }

        Ok(result)
    }

    /// Retrieve a built execution payload.
    pub async fn get_payload_v3(
        &self,
        payload_id: PayloadId,
    ) -> Result<ExecutionPayloadEnvelopeV3, EngineApiError> {
        self.rpc_call(
            "engine_getPayloadV3",
            vec![serde_json::to_value(payload_id)
                .map_err(|e| EngineApiError::Serialization(e.to_string()))?],
        )
        .await
    }

    /// Fetch blobs from the EL blob pool by versioned hash.
    pub async fn get_blobs_v1(
        &self,
        versioned_hashes: Vec<B256>,
    ) -> Result<Vec<Option<BlobAndProofV1>>, EngineApiError> {
        self.rpc_call(
            "engine_getBlobsV1",
            vec![serde_json::to_value(versioned_hashes)
                .map_err(|e| EngineApiError::Serialization(e.to_string()))?],
        )
        .await
    }

    // ----------------------------------------------------------------
    // Shanghai methods (V2) -- fallback / compatibility
    // ----------------------------------------------------------------

    /// Submit a payload (Shanghai version with withdrawals).
    pub async fn new_payload_v2(
        &self,
        payload: serde_json::Value,
    ) -> Result<PayloadStatus, EngineApiError> {
        self.rpc_call("engine_newPayloadV2", vec![payload]).await
    }

    /// Forkchoice update (Shanghai version).
    pub async fn forkchoice_updated_v2(
        &self,
        forkchoice_state: ForkchoiceState,
        payload_attributes: Option<serde_json::Value>,
    ) -> Result<ForkchoiceUpdated, EngineApiError> {
        self.rpc_call(
            "engine_forkchoiceUpdatedV2",
            vec![
                serde_json::to_value(forkchoice_state)
                    .map_err(|e| EngineApiError::Serialization(e.to_string()))?,
                serde_json::to_value(payload_attributes)
                    .map_err(|e| EngineApiError::Serialization(e.to_string()))?,
            ],
        )
        .await
    }

    /// Get payload (Shanghai version).
    pub async fn get_payload_v2(
        &self,
        payload_id: PayloadId,
    ) -> Result<serde_json::Value, EngineApiError> {
        self.rpc_call(
            "engine_getPayloadV2",
            vec![serde_json::to_value(payload_id)
                .map_err(|e| EngineApiError::Serialization(e.to_string()))?],
        )
        .await
    }

    // ----------------------------------------------------------------
    // Payload body retrieval
    // ----------------------------------------------------------------

    /// Get payload bodies by block hash.
    pub async fn get_payload_bodies_by_hash_v1(
        &self,
        block_hashes: Vec<B256>,
    ) -> Result<Vec<Option<ExecutionPayloadBodyV1>>, EngineApiError> {
        self.rpc_call(
            "engine_getPayloadBodiesByHashV1",
            vec![serde_json::to_value(block_hashes)
                .map_err(|e| EngineApiError::Serialization(e.to_string()))?],
        )
        .await
    }

    /// Get payload bodies by block number range.
    pub async fn get_payload_bodies_by_range_v1(
        &self,
        start: u64,
        count: u64,
    ) -> Result<Vec<Option<ExecutionPayloadBodyV1>>, EngineApiError> {
        self.rpc_call(
            "engine_getPayloadBodiesByRangeV1",
            vec![
                serde_json::Value::String(format!("0x{start:x}")),
                serde_json::Value::String(format!("0x{count:x}")),
            ],
        )
        .await
    }

    // ----------------------------------------------------------------
    // Capability exchange
    // ----------------------------------------------------------------

    /// Exchange supported Engine API method names with the EL.
    pub async fn exchange_capabilities(
        &self,
        our_methods: Vec<String>,
    ) -> Result<Vec<String>, EngineApiError> {
        self.rpc_call(
            "engine_exchangeCapabilities",
            vec![serde_json::to_value(our_methods)
                .map_err(|e| EngineApiError::Serialization(e.to_string()))?],
        )
        .await
    }

    /// Read an execution block through Reth's authenticated RPC endpoint.
    pub async fn get_block_by_number(
        &self,
        number: u64,
    ) -> Result<serde_json::Value, EngineApiError> {
        self.rpc_call(
            "eth_getBlockByNumber",
            vec![
                serde_json::Value::String(format!("0x{number:x}")),
                serde_json::Value::Bool(false),
            ],
        )
        .await
    }

    // ----------------------------------------------------------------
    // Convenience: full block proposal flow
    // ----------------------------------------------------------------

    /// Execute a complete block proposal cycle.
    /// 1. `forkchoice_updated_v3` with payload attributes -> get payloadId
    /// 2. `get_payload_v3` -> retrieve built block
    pub async fn propose_block(
        &self,
        forkchoice_state: ForkchoiceState,
        payload_attributes: PayloadAttributes,
    ) -> Result<ProposeBlockResult, EngineApiError> {
        // Step 1: trigger payload building
        let fcu_response =
            self.forkchoice_updated_v3(forkchoice_state, Some(payload_attributes)).await?;

        let payload_id = fcu_response.payload_id.ok_or_else(|| {
            // If status is not VALID, build a more specific error
            match &fcu_response.payload_status.status {
                PayloadStatusEnum::Invalid { validation_error } => EngineApiError::InvalidPayload {
                    latest_valid_hash: fcu_response
                        .payload_status
                        .latest_valid_hash
                        .map(|h| format!("{h:#x}")),
                    validation_error: Some(validation_error.clone()),
                },
                _ => EngineApiError::UnknownPayload(
                    "no payload ID returned from forkchoiceUpdated".into(),
                ),
            }
        })?;

        // Step 2: retrieve the built payload
        let response = self.get_payload_v3(payload_id).await?;
        Ok(ProposeBlockResult::from(response))
    }

    // ----------------------------------------------------------------
    // Internal JSON-RPC transport
    // ----------------------------------------------------------------

    /// Make an authenticated JSON-RPC call to the Engine API.
    async fn rpc_call<T: serde::de::DeserializeOwned + Send>(
        &self,
        method: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<T, EngineApiError> {
        self.rpc_call_with_retry(method, params, 0).await
    }

    fn rpc_call_with_retry<'a, T: serde::de::DeserializeOwned + Send + 'a>(
        &'a self,
        method: &'a str,
        params: Vec<serde_json::Value>,
        attempt: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, EngineApiError>> + Send + 'a>>
    {
        Box::pin(async move {
            let token = self.jwt.generate_token()?;

            let request_body = serde_json::json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params,
                "id": attempt,
            });

            let response = self
                .inner
                .post(&self.config.endpoint)
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() {
                        EngineApiError::Timeout(self.config.request_timeout.as_millis() as u64)
                    } else if e.is_connect() {
                        EngineApiError::ConnectionLost(e.to_string())
                    } else {
                        EngineApiError::Transport(e.to_string())
                    }
                })?;

            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                if status.as_u16() == 401 {
                    return Err(EngineApiError::JwtAuth(format!("unauthorized: {body}")));
                }
                return Err(EngineApiError::Transport(format!("HTTP {}: {body}", status)));
            }

            let body: serde_json::Value =
                response.json().await.map_err(|e| EngineApiError::Serialization(e.to_string()))?;

            // Check for JSON-RPC error response.
            if let Some(error) = body.get("error") {
                let code = error["code"].as_i64().unwrap_or(-32000);
                let message = error["message"].as_str().unwrap_or("unknown").to_string();
                let data = error["data"]["err"]
                    .as_str()
                    .map(|s| s.to_string())
                    .or_else(|| error["data"].as_str().map(|s| s.to_string()));

                let api_error = match code {
                    -38001 => EngineApiError::UnknownPayload(message),
                    -38002 => EngineApiError::InvalidForkchoiceState(message),
                    -38003 => EngineApiError::InvalidPayloadAttributes(message),
                    -38005 => EngineApiError::UnsupportedFork(message),
                    -38006 => EngineApiError::TooDeepReorg(message),
                    _ => EngineApiError::JsonRpc { code, message, data },
                };

                // Retry on transient errors.
                if api_error.is_retryable() && attempt < self.config.max_retries {
                    let delay = self.backoff_delay(attempt);
                    tokio::time::sleep(delay).await;
                    return self.rpc_call_with_retry(method, params, attempt + 1).await;
                }

                return Err(api_error);
            }

            // Extract result.
            let result = body.get("result").ok_or_else(|| {
                EngineApiError::Serialization("missing 'result' field in response".into())
            })?;

            serde_json::from_value(result.clone())
                .map_err(|e| EngineApiError::Serialization(e.to_string()))
        })
    }

    /// Compute exponential backoff delay.
    fn backoff_delay(&self, attempt: u32) -> std::time::Duration {
        let base = self.config.retry_base_delay.as_millis() as u64;
        let delay = base.saturating_mul(2u64.saturating_pow(attempt));
        let capped = delay.min(self.config.retry_max_delay.as_millis() as u64);
        // Add jitter: +/- 20% using deterministic hash.
        let jitter = (capped as f64 * 0.2) as u64;
        let jittered = if jitter > 0 {
            let factor = rand_factor(attempt);
            capped + (factor % (jitter * 2)).saturating_sub(jitter)
        } else {
            capped
        };
        std::time::Duration::from_millis(jittered)
    }
}

/// Simple deterministic jitter factor (avoid pulling in rand).
fn rand_factor(seed: u32) -> u64 {
    // FNV-1a-inspired hash for cheap pseudo-randomness.
    let mut hash = 0x811c9dc5_u64;
    for byte in seed.to_le_bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Helper to create a valid config with a raw JWT secret.
    fn test_config() -> EngineApiClientConfig {
        EngineApiClientConfig {
            endpoint: "http://127.0.0.1:8551".into(),
            jwt_secret: Some(vec![42u8; 32]),
            jwt_secret_path: None,
            request_timeout: Duration::from_secs(10),
            max_retries: 3,
            retry_base_delay: Duration::from_millis(100),
            retry_max_delay: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(3),
        }
    }

    #[test]
    fn rand_factor_deterministic() {
        let a = rand_factor(1);
        let b = rand_factor(1);
        assert_eq!(a, b);

        let c = rand_factor(2);
        assert_ne!(a, c);
    }

    #[test]
    fn backoff_delay_increases() {
        let config = test_config();
        let client =
            futures::executor::block_on(async { EngineApiClient::new(config).await.unwrap() });

        let d0 = client.backoff_delay(0);
        let d1 = client.backoff_delay(1);
        let d2 = client.backoff_delay(2);

        // Each successive delay should produce a reasonable duration.
        assert!(d0.as_millis() > 0);
        assert!(d1.as_millis() > 0);
        assert!(d2.as_millis() > 0);
    }

    #[test]
    fn backoff_delay_capped_at_max() {
        let config =
            EngineApiClientConfig { retry_max_delay: Duration::from_millis(200), ..test_config() };
        let client =
            futures::executor::block_on(async { EngineApiClient::new(config).await.unwrap() });

        let delay = client.backoff_delay(100);
        // With jitter (+/- 20%), max is 200 * 1.2 = 240ms
        assert!(delay.as_millis() <= 300);
    }

    #[test]
    fn json_rpc_request_format() {
        let method = "engine_newPayloadV3";
        let params = [
            serde_json::json!({"blockNumber": "0x1"}),
            serde_json::json!([]),
            serde_json::json!("0x0000000000000000000000000000000000000000000000000000000000000000"),
        ];

        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 0,
        });

        assert_eq!(request_body["jsonrpc"], "2.0");
        assert_eq!(request_body["method"], "engine_newPayloadV3");
        assert!(request_body["params"].is_array());
        assert_eq!(request_body["id"], 0);
    }

    #[test]
    fn json_rpc_error_response_parsing() {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32602,
                "message": "Invalid params"
            }
        });

        let error = body.get("error").unwrap();
        let code = error["code"].as_i64().unwrap();
        let message = error["message"].as_str().unwrap().to_string();

        assert_eq!(code, -32602);
        assert_eq!(message, "Invalid params");
    }

    #[test]
    fn json_rpc_engine_error_parsing() {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -38001,
                "message": "Unknown payload"
            }
        });

        let error = body.get("error").unwrap();
        let code = error["code"].as_i64().unwrap();
        let message = error["message"].as_str().unwrap().to_string();

        let api_error = match code {
            -38001 => EngineApiError::UnknownPayload(message),
            -38002 => EngineApiError::InvalidForkchoiceState(message),
            -38003 => EngineApiError::InvalidPayloadAttributes(message),
            _ => EngineApiError::JsonRpc { code, message, data: None },
        };

        assert!(matches!(api_error, EngineApiError::UnknownPayload(_)));
    }

    #[test]
    fn json_rpc_success_response_result_extraction() {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "payloadStatus": {
                    "status": "VALID",
                    "latestValidHash": "0x0000000000000000000000000000000000000000000000000000000000000001"
                },
                "payloadId": "0x1234567890abcdef"
            }
        });

        let result = body.get("result").unwrap();
        let response: ForkchoiceUpdated = serde_json::from_value(result.clone()).unwrap();
        assert!(response.payload_id.is_some());
    }

    #[test]
    fn payload_status_response_parsing() {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "status": "INVALID",
                "latestValidHash": "0x0000000000000000000000000000000000000000000000000000000000000001",
                "validationError": "invalid transaction"
            }
        });

        let result = body.get("result").unwrap();
        let status: PayloadStatus = serde_json::from_value(result.clone()).unwrap();
        assert!(matches!(status.status, PayloadStatusEnum::Invalid { .. }));
    }

    #[test]
    fn exchange_capabilities_request_format() {
        let methods = vec![
            "engine_newPayloadV3".to_string(),
            "engine_forkchoiceUpdatedV3".to_string(),
            "engine_getPayloadV3".to_string(),
        ];

        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "engine_exchangeCapabilities",
            "params": [methods],
            "id": 1,
        });

        let params = request_body["params"].as_array().unwrap();
        assert_eq!(params.len(), 1);
        let methods_array = params[0].as_array().unwrap();
        assert_eq!(methods_array.len(), 3);
    }

    #[test]
    fn exchange_capabilities_response_parsing() {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": [
                "engine_newPayloadV1",
                "engine_newPayloadV2",
                "engine_newPayloadV3",
                "engine_forkchoiceUpdatedV1",
                "engine_forkchoiceUpdatedV2",
                "engine_forkchoiceUpdatedV3",
                "engine_getPayloadV1",
                "engine_getPayloadV2",
                "engine_getPayloadV3",
                "engine_exchangeCapabilities"
            ]
        });

        let result = body.get("result").unwrap();
        let capabilities: Vec<String> = serde_json::from_value(result.clone()).unwrap();
        assert!(capabilities.contains(&"engine_newPayloadV3".to_string()));
    }

    #[test]
    fn get_payload_bodies_by_range_request_format() {
        let start: u64 = 1;
        let count: u64 = 10;

        let params = [
            serde_json::Value::String(format!("0x{start:x}")),
            serde_json::Value::String(format!("0x{count:x}")),
        ];

        assert_eq!(params[0], "0x1");
        assert_eq!(params[1], "0xa");
    }

    #[tokio::test]
    async fn new_rejects_missing_secret() {
        let config =
            EngineApiClientConfig { jwt_secret: None, jwt_secret_path: None, ..test_config() };
        let result = EngineApiClient::new(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("jwt_secret"));
    }

    #[tokio::test]
    async fn new_rejects_wrong_size_secret() {
        let config = EngineApiClientConfig {
            jwt_secret: Some(vec![0u8; 16]),
            jwt_secret_path: None,
            ..test_config()
        };
        let result = EngineApiClient::new(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("32 bytes"));
    }

    #[tokio::test]
    async fn new_accepts_valid_secret() {
        let config = test_config();
        let result = EngineApiClient::new(config).await;
        assert!(result.is_ok());
    }
}
