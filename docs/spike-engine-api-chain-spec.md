# Spike: Engine API Client + Chain Spec Design

Spike for GitHub #24 (Engine API Client) and #28 (Chain Spec).

Date: 2026-05-29

---

## Part 1: Engine API Client (GitHub #24)

### 1.1 Engine API Methods -- Complete Reference

The Engine API is versioned per-method. Each hardfork introduces new versions. Below is the full method set across all forks (Paris through Prague).

#### Core Methods (Paris -- V1)

| Method | Version | Timeout | Purpose |
|---|---|---|---|
| `engine_newPayloadV1` | Paris | 8s | Submit an execution payload for validation |
| `engine_forkchoiceUpdatedV1` | Paris | 8s | Update head/safe/finalized blocks; optionally trigger payload building |
| `engine_getPayloadV1` | Paris | 1s | Retrieve a built execution payload by ID |
| `engine_exchangeTransitionConfigurationV1` | Paris | 1s | Verify CL/EL transition config match (deprecated after Cancun) |

#### Shanghai Additions (V2)

| Method | Version | Timeout | Purpose |
|---|---|---|---|
| `engine_newPayloadV2` | Shanghai | 8s | Like V1, handles withdrawals |
| `engine_forkchoiceUpdatedV2` | Shanghai | 8s | Like V1, accepts PayloadAttributesV2 (with withdrawals) |
| `engine_getPayloadV2` | Shanghai | 1s | Returns ExecutionPayloadV2 + blockValue |
| `engine_getPayloadBodiesByHashV1` | Shanghai | 10s | Fetch payload bodies by block hash array |
| `engine_getPayloadBodiesByRangeV1` | Shanghai | 10s | Fetch payload bodies by block number range |

#### Cancun Additions (V3)

| Method | Version | Timeout | Purpose |
|---|---|---|---|
| `engine_newPayloadV3` | Cancun | 8s | Adds blobGasUsed, excessBlobGas, expectedBlobVersionedHashes, parentBeaconBlockRoot |
| `engine_forkchoiceUpdatedV3` | Cancun | 8s | Accepts PayloadAttributesV3 (with parentBeaconBlockRoot) |
| `engine_getPayloadV3` | Cancun | 1s | Returns ExecutionPayloadV3 + blockValue + blobsBundle + shouldOverrideBuilder |
| `engine_getBlobsV1` | Cancun | 1s | Fetch blobs from EL blob pool by versioned hash |

#### Prague Additions (V4)

| Method | Version | Timeout | Purpose |
|---|---|---|---|
| `engine_newPayloadV4` | Prague | 8s | Adds depositRequests, withdrawalRequests, consolidationRequests to payload |
| `engine_getPayloadV4` | Prague | 1s | Returns ExecutionPayloadV4 + blobsBundle + executionRequests |
| `engine_getPayloadBodiesByHashV2` | Prague | 10s | Returns ExecutionPayloadBodyV2 (includes deposit/withdrawal/consolidation requests) |
| `engine_getPayloadBodiesByRangeV2` | Prague | 10s | Returns ExecutionPayloadBodyV2 by range |

#### Capability Exchange (all forks)

| Method | Timeout | Purpose |
|---|---|---|
| `engine_exchangeCapabilities` | 1s | Exchange supported method names (no version suffix) |

### 1.2 JWT Authentication

Per the Engine API authentication spec:

**Key Distribution:**
- Shared secret: 256-bit (32 bytes), hex-encoded in a file (e.g., `jwt.hex`)
- Both EL and CL read the same file
- If no file is specified, the EL generates one and writes it to `jwt.hex` in its data directory

**Token Format:**
- Algorithm: HMAC-SHA256 (HS256) -- mandatory support
- Algorithm `none` MUST be rejected
- Claims:
  - `iat` (issued-at): Required. EL SHOULD only accept tokens within +/-60 seconds of current time
  - `id`: Optional. Unique identifier for the CL client instance
  - `clv`: Optional. CL client type/version
- Unknown claims MUST be ignored

**Transport:**
- HTTP: JWT in `Authorization: Bearer <token>` header on every request
- WebSocket: JWT only on initial handshake upgrade request
- IPC (inproc): No authentication required

### 1.3 Data Structures

#### ExecutionPayloadV3 (Cancun -- our target)

```
ExecutionPayloadV3:
  parentHash:          B256 (32 bytes)
  feeRecipient:        Address (20 bytes)
  stateRoot:           B256 (32 bytes)
  receiptsRoot:        B256 (32 bytes)
  logsBloom:           Bytes (256 bytes)
  prevRandao:          B256 (32 bytes)
  blockNumber:         u64
  gasLimit:            u64
  gasUsed:             u64
  timestamp:           u64
  extraData:           Bytes (0..32)
  baseFeePerGas:       U256
  blockHash:           B256 (32 bytes)
  transactions:        Vec<Bytes>    (EIP-2718 encoded)
  withdrawals:         Vec<WithdrawalV1>
  blobGasUsed:         u64
  excessBlobGas:       u64
```

#### PayloadAttributesV3 (Cancun)

```
PayloadAttributesV3:
  timestamp:              u64
  prevRandao:             B256 (32 bytes)
  suggestedFeeRecipient:  Address (20 bytes)
  withdrawals:            Vec<WithdrawalV1>
  parentBeaconBlockRoot:  B256 (32 bytes)
```

#### ForkchoiceStateV1

```
ForkchoiceStateV1:
  headBlockHash:      B256
  safeBlockHash:      B256
  finalizedBlockHash: B256
```

#### PayloadStatusV1

```
PayloadStatusV1:
  status:           enum { VALID, INVALID, SYNCING, ACCEPTED, INVALID_BLOCK_HASH }
  latestValidHash:  Option<B256>
  validationError:  Option<String>
```

#### BlobsBundleV1 (Cancun)

```
BlobsBundleV1:
  commitments:  Vec<Bytes>  (48 bytes each, KZGCommitment)
  proofs:       Vec<Bytes>  (48 bytes each, KZGProof)
  blobs:        Vec<Bytes>  (131072 bytes each, SSZ-encoded Blob)
```

#### WithdrawalV1

```
WithdrawalV1:
  index:           u64
  validatorIndex:  u64
  address:         Address
  amount:          u64  (Gwei, big-endian)
```

### 1.4 Block Proposal Flow (End-to-End)

Neunode's consensus layer (CL) drives block production via the Engine API. The flow for proposing a new block:

```
CL                                          EL (Reth)
|                                              |
|  1. engine_forkchoiceUpdatedV3               |
|     (forkchoiceState, payloadAttributesV3)   |
|--------------------------------------------->|
|                                              |  Validates head block
|                                              |  Updates forkchoice state
|                                              |  Begins payload building
|  Response: {payloadStatus: VALID,            |
|             payloadId: "0x..."}              |
|<---------------------------------------------|
|                                              |
|     ... wait for slot duration or until      |
|     sufficient transactions collected ...    |
|                                              |
|  2. engine_getPayloadV3                      |
|     (payloadId)                              |
|--------------------------------------------->|
|                                              |  Stops building
|  Response: {executionPayload,                |
|             blockValue,                      |
|             blobsBundle,                     |
|             shouldOverrideBuilder}           |
|<---------------------------------------------|
|                                              |
|  3. CL broadcasts block via consensus p2p    |
|                                              |
|  4. engine_newPayloadV3                      |
|     (executionPayload,                       |
|      expectedBlobVersionedHashes,            |
|      parentBeaconBlockRoot)                  |
|--------------------------------------------->|
|                                              |  Validates payload
|  Response: {status: VALID, ...}              |
|<---------------------------------------------|
|                                              |
|  5. engine_forkchoiceUpdatedV3               |
|     (forkchoiceState with new head,          |
|      payloadAttributes: null)                |
|--------------------------------------------->|
|                                              |  Updates head to new block
|  Response: {payloadStatus: VALID,            |
|             payloadId: null}                 |
|<---------------------------------------------|
```

Key ordering constraints:
- `engine_forkchoiceUpdated` calls MUST be processed in order
- `engine_getPayload` MUST be called before the payload is needed (within slot)
- `engine_newPayload` can be called out-of-order for syncing, but the CL MUST respect forkchoice ordering

### 1.5 Error Handling

#### JSON-RPC Error Codes

| Code | Message | Meaning |
|---|---|---|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid Request | Not a valid Request object |
| -32601 | Method not found | Method does not exist |
| -32602 | Invalid params | Invalid method parameters |
| -32603 | Internal error | Internal JSON-RPC error |
| -32000 | Server error | Generic client error (includes `data.err` details) |
| -38001 | Unknown payload | Payload build process not found |
| -38002 | Invalid forkchoice state | Forkchoice state invalid/inconsistent |
| -38003 | Invalid payload attributes | Payload attributes invalid/inconsistent |
| -38004 | Too large request | Too many entities requested |
| -38005 | Unsupported fork | Payload belongs to unsupported fork |
| -38006 | Too deep reorg | Reorg depth exceeds limitation |

#### PayloadStatus Values

- `VALID`: Payload passes all validation. `latestValidHash` = payload.blockHash
- `INVALID`: Payload fails validation. `latestValidHash` = last valid ancestor or null
- `SYNCING`: Requisite data missing, sync initiated
- `ACCEPTED`: Payload valid on non-canonical branch (transactions non-empty, blockHash valid)
- `INVALID_BLOCK_HASH`: Only in V1. Supplanted by `INVALID` in V2+

#### Client-Side Error Handling Strategy

1. **Retryable errors**: SYNCING status, network timeouts, connection refused -- retry with exponential backoff
2. **Fatal errors**: INVALID status with latestValidHash -- log, alert, do not retry the same payload
3. **Configuration errors**: -38002, -38003, -38005 -- check chain spec alignment, fix configuration
4. **Reconnection**: If the HTTP connection drops, reconnect and re-validate the current forkchoice state

### 1.6 Rust Client Design: `neunode-engine-api-client`

#### Dependencies

```toml
[package]
name = "neunode-engine-api-client"
version = "0.1.0"
edition = "2021"

[dependencies]
# JSON-RPC client
jsonrpsee = { version = "0.24", features = ["http-client", "ws-client"] }

# HTTP client (fallback, for JWT header injection)
reqwest = { version = "0.12", features = ["json"] }

# Async runtime
tokio = { version = "1.44", features = ["rt", "macros", "time", "sync"] }

# Crypto
hmac = "0.12"
sha2 = "0.10"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Ethereum types (consistent with existing alloy dep)
alloy = { version = "1.8", default-features = false, features = [
    "reqwest-rustls-tls", "rpc-types", "consensus",
] }

# Error handling
thiserror = "2.0"

# Logging
tracing = "0.1"

# JWT
jsonwebtoken = "9"
```

#### Error Types

```rust
// src/error.rs

use thiserror::Error;

/// Errors from the Engine API client.
#[derive(Error, Debug)]
pub enum EngineApiError {
    /// HTTP/transport layer error.
    #[error("transport error: {0}")]
    Transport(String),

    /// JSON-RPC error response from the EL.
    #[error("JSON-RPC error {code}: {message}")]
    JsonRpc {
        code: i64,
        message: String,
        data: Option<String>,
    },

    /// Payload validation returned INVALID.
    #[error("invalid payload: {validation_error:?} (latestValidHash: {latest_valid_hash:?})")]
    InvalidPayload {
        latest_valid_hash: Option<String>,
        validation_error: Option<String>,
    },

    /// EL is syncing; payload not yet validated.
    #[error("EL is syncing")]
    Syncing,

    /// Unknown payload ID returned by getPayload.
    #[error("unknown payload: {0}")]
    UnknownPayload(String),

    /// Forkchoice state is invalid.
    #[error("invalid forkchoice state: {0}")]
    InvalidForkchoiceState(String),

    /// Payload attributes rejected.
    #[error("invalid payload attributes: {0}")]
    InvalidPayloadAttributes(String),

    /// Unsupported fork.
    #[error("unsupported fork: {0}")]
    UnsupportedFork(String),

    /// Reorg too deep.
    #[error("too deep reorg: {0}")]
    TooDeepReorg(String),

    /// JWT authentication failure.
    #[error("JWT auth error: {0}")]
    JwtAuth(String),

    /// Request timed out.
    #[error("request timed out after {0}ms")]
    Timeout(u64),

    /// Connection lost; retry in progress.
    #[error("connection lost: {0}")]
    ConnectionLost(String),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(String),
}

impl EngineApiError {
    /// Whether the caller should retry this operation.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Syncing | Self::Timeout(_) | Self::ConnectionLost(_)
        )
    }
}
```

#### JWT Authentication

```rust
// src/jwt.rs

use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// JWT claims for Engine API authentication.
#[derive(Debug, Serialize, Deserialize)]
pub struct EngineApiClaims {
    /// Required: issued-at timestamp (seconds since epoch).
    pub iat: u64,
    /// Optional: unique CL client identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Optional: CL client type/version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clv: Option<String>,
}

/// Manages JWT token generation for Engine API authentication.
pub struct JwtAuth {
    /// The 256-bit shared secret.
    secret: [u8; 32],
    /// Optional client identifier.
    client_id: Option<String>,
    /// Optional client version string.
    client_version: Option<String>,
}

impl JwtAuth {
    /// Create a new JWT authenticator from a hex-encoded secret.
    pub fn from_hex_secret(hex: &str) -> Result<Self, EngineApiError> {
        let bytes = hex::decode(hex.trim_start_matches("0x"))
            .map_err(|e| EngineApiError::JwtAuth(format!("invalid hex secret: {e}")))?;
        if bytes.len() != 32 {
            return Err(EngineApiError::JwtAuth(
                "JWT secret must be exactly 256 bits (32 bytes)".into(),
            ));
        }
        let mut secret = [0u8; 32];
        secret.copy_from_slice(&bytes);
        Ok(Self {
            secret,
            client_id: None,
            client_version: None,
        })
    }

    /// Generate a new JWT token valid for the current time.
    pub fn generate_token(&self) -> Result<String, EngineApiError> {
        let key = HmacSha256::new_from_slice(&self.secret)
            .expect("HMAC accepts any key length");
        let claims = EngineApiClaims {
            iat: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time went backwards")
                .as_secs(),
            id: self.client_id.clone(),
            clv: self.client_version.clone(),
        };
        claims
            .sign_with_key(&key)
            .map_err(|e| EngineApiError::JwtAuth(format!("token signing failed: {e}")))
    }
}
```

#### Core Client Types

```rust
// src/types.rs

use alloy::primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};

/// Payload validation status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayloadStatus {
    Valid,
    Invalid,
    Syncing,
    Accepted,
}

/// Result of payload validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadStatusV1 {
    pub status: PayloadStatus,
    #[serde(with = "alloy::serde::quantity")]
    pub latest_valid_hash: Option<B256>,
    pub validation_error: Option<String>,
}

/// Forkchoice state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkchoiceStateV1 {
    pub head_block_hash: B256,
    pub safe_block_hash: B256,
    pub finalized_block_hash: B256,
}

/// Withdrawal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawalV1 {
    #[serde(with = "alloy::serde::quantity")]
    pub index: u64,
    #[serde(with = "alloy::serde::quantity", rename = "validatorIndex")]
    pub validator_index: u64,
    pub address: Address,
    #[serde(with = "alloy::serde::quantity")]
    pub amount: u64,
}

/// Payload attributes for Cancun (V3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadAttributesV3 {
    #[serde(with = "alloy::serde::quantity")]
    pub timestamp: u64,
    pub prev_randao: B256,
    pub suggested_fee_recipient: Address,
    pub withdrawals: Vec<WithdrawalV1>,
    pub parent_beacon_block_root: B256,
}

/// Execution payload (Cancun/V3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPayloadV3 {
    pub parent_hash: B256,
    pub fee_recipient: Address,
    pub state_root: B256,
    pub receipts_root: B256,
    pub logs_bloom: alloy::primitives::Bytes,
    pub prev_randao: B256,
    #[serde(with = "alloy::serde::quantity")]
    pub block_number: u64,
    #[serde(with = "alloy::serde::quantity")]
    pub gas_limit: u64,
    #[serde(with = "alloy::serde::quantity")]
    pub gas_used: u64,
    #[serde(with = "alloy::serde::quantity")]
    pub timestamp: u64,
    pub extra_data: alloy::primitives::Bytes,
    #[serde(with = "alloy::serde::quantity")]
    pub base_fee_per_gas: U256,
    pub block_hash: B256,
    pub transactions: Vec<alloy::primitives::Bytes>,
    pub withdrawals: Vec<WithdrawalV1>,
    #[serde(with = "alloy::serde::quantity")]
    pub blob_gas_used: u64,
    #[serde(with = "alloy::serde::quantity")]
    pub excess_blob_gas: u64,
}

/// Blobs bundle (Cancun).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobsBundleV1 {
    pub commitments: Vec<alloy::primitives::Bytes>,
    pub proofs: Vec<alloy::primitives::Bytes>,
    pub blobs: Vec<alloy::primitives::Bytes>,
}

/// Response from engine_getPayloadV3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPayloadV3Response {
    pub execution_payload: ExecutionPayloadV3,
    #[serde(with = "alloy::serde::quantity")]
    pub block_value: U256,
    pub blobs_bundle: BlobsBundleV1,
    pub should_override_builder: bool,
}

/// Response from engine_forkchoiceUpdated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkchoiceUpdatedResponse {
    pub payload_status: PayloadStatusV1,
    pub payload_id: Option<[u8; 8]>,
}

/// Transition configuration (Paris, deprecated after Cancun).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionConfigurationV1 {
    #[serde(with = "alloy::serde::quantity")]
    pub terminal_total_difficulty: U256,
    pub terminal_block_hash: B256,
    #[serde(with = "alloy::serde::quantity")]
    pub terminal_block_number: u64,
}
```

#### Client Configuration

```rust
// src/config.rs

use std::time::Duration;

/// Configuration for the Engine API client.
#[derive(Debug, Clone)]
pub struct EngineApiClientConfig {
    /// URL of the EL Engine API endpoint.
    /// Default: "http://127.0.0.1:8551"
    pub endpoint: String,

    /// Path to the JWT secret file (hex-encoded 256-bit key).
    pub jwt_secret_path: Option<std::path::PathBuf>,

    /// Raw JWT secret bytes (alternative to file-based loading).
    pub jwt_secret: Option<Vec<u8>>,

    /// HTTP request timeout.
    /// Default: 10 seconds (covers 8s engine methods).
    pub request_timeout: Duration,

    /// Maximum number of retry attempts for transient failures.
    /// Default: 5.
    pub max_retries: u32,

    /// Base delay for exponential backoff on retries.
    /// Default: 500ms.
    pub retry_base_delay: Duration,

    /// Maximum backoff delay cap.
    /// Default: 30 seconds.
    pub retry_max_delay: Duration,

    /// Connection timeout for establishing TCP connection.
    /// Default: 5 seconds.
    pub connect_timeout: Duration,
}

impl Default for EngineApiClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:8551".into(),
            jwt_secret_path: None,
            jwt_secret: None,
            request_timeout: Duration::from_secs(10),
            max_retries: 5,
            retry_base_delay: Duration::from_millis(500),
            retry_max_delay: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(5),
        }
    }
}
```

#### Client Implementation

```rust
// src/client.rs

use crate::{
    config::EngineApiClientConfig,
    error::EngineApiError,
    jwt::JwtAuth,
    types::*,
};
use alloy::primitives::B256;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Async Engine API client connecting to a Reth execution layer.
pub struct EngineApiClient {
    /// Inner HTTP client with JWT auth.
    inner: reqwest::Client,
    /// JWT authenticator.
    jwt: JwtAuth,
    /// Configuration.
    config: EngineApiClientConfig,
    /// Current payload ID tracking (for block building).
    pending_payload_id: Arc<RwLock<Option<[u8; 8]>>>,
}

impl EngineApiClient {
    /// Create a new Engine API client from configuration.
    pub async fn new(config: EngineApiClientConfig) -> Result<Self, EngineApiError> {
        let jwt = if let Some(ref path) = config.jwt_secret_path {
            let hex = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| EngineApiError::JwtAuth(format!("cannot read JWT file: {e}")))?;
            JwtAuth::from_hex_secret(&hex)?
        } else if let Some(ref bytes) = config.jwt_secret {
            JwtAuth::from_hex_secret(&hex::encode(bytes))?
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

        Ok(Self {
            inner,
            jwt,
            config,
            pending_payload_id: Arc::new(RwLock::new(None)),
        })
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
    ) -> Result<PayloadStatusV1, EngineApiError> {
        self.rpc_call(
            "engine_newPayloadV3",
            vec![
                serde_json::to_value(payload).map_err(|e| EngineApiError::Serialization(e.to_string()))?,
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
        forkchoice_state: ForkchoiceStateV1,
        payload_attributes: Option<PayloadAttributesV3>,
    ) -> Result<ForkchoiceUpdatedResponse, EngineApiError> {
        let result: ForkchoiceUpdatedResponse = self
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
        if let Some(id) = result.payload_id {
            *self.pending_payload_id.write().await = Some(id);
        }

        Ok(result)
    }

    /// Retrieve a built execution payload.
    pub async fn get_payload_v3(
        &self,
        payload_id: [u8; 8],
    ) -> Result<GetPayloadV3Response, EngineApiError> {
        let hex_id = format!("0x{}", hex::encode(payload_id));
        self.rpc_call(
            "engine_getPayloadV3",
            vec![serde_json::Value::String(hex_id)],
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
    ) -> Result<PayloadStatusV1, EngineApiError> {
        self.rpc_call("engine_newPayloadV2", vec![payload]).await
    }

    /// Forkchoice update (Shanghai version).
    pub async fn forkchoice_updated_v2(
        &self,
        forkchoice_state: ForkchoiceStateV1,
        payload_attributes: Option<serde_json::Value>,
    ) -> Result<ForkchoiceUpdatedResponse, EngineApiError> {
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
        payload_id: [u8; 8],
    ) -> Result<serde_json::Value, EngineApiError> {
        let hex_id = format!("0x{}", hex::encode(payload_id));
        self.rpc_call("engine_getPayloadV2", vec![serde_json::Value::String(hex_id)])
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

    // ----------------------------------------------------------------
    // Convenience: full block proposal flow
    // ----------------------------------------------------------------

    /// Execute a complete block proposal cycle.
    /// 1. forkchoiceUpdated with payload attributes -> get payloadId
    /// 2. Wait for block building (or caller drives timing)
    /// 3. getPayload -> retrieve built block
    pub async fn propose_block(
        &self,
        forkchoice_state: ForkchoiceStateV1,
        payload_attributes: PayloadAttributesV3,
    ) -> Result<GetPayloadV3Response, EngineApiError> {
        // Step 1: trigger payload building
        let fcu_response = self
            .forkchoice_updated_v3(forkchoice_state, Some(payload_attributes))
            .await?;

        if fcu_response.payload_status.status != PayloadStatus::Valid {
            return Err(EngineApiError::InvalidPayload {
                latest_valid_hash: fcu_response
                    .payload_status
                    .latest_valid_hash
                    .map(|h| format!("{h:#x}")),
                validation_error: fcu_response.payload_status.validation_error,
            });
        }

        let payload_id = fcu_response
            .payload_id
            .ok_or(EngineApiError::UnknownPayload(
                "no payload ID returned from forkchoiceUpdated".into(),
            ))?;

        // Step 2: retrieve the built payload
        self.get_payload_v3(payload_id).await
    }

    // ----------------------------------------------------------------
    // Internal JSON-RPC transport
    // ----------------------------------------------------------------

    /// Make an authenticated JSON-RPC call to the Engine API.
    async fn rpc_call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<T, EngineApiError> {
        self.rpc_call_with_retry(method, params, 0).await
    }

    async fn rpc_call_with_retry<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Vec<serde_json::Value>,
        attempt: u32,
    ) -> Result<T, EngineApiError> {
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

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| EngineApiError::Serialization(e.to_string()))?;

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
    }

    /// Compute exponential backoff delay.
    fn backoff_delay(&self, attempt: u32) -> std::time::Duration {
        let base = self.config.retry_base_delay.as_millis() as u64;
        let delay = base.saturating_mul(2u64.saturating_pow(attempt));
        let capped = delay.min(self.config.retry_max_delay.as_millis() as u64);
        // Add jitter: +/- 20%.
        let jitter = (capped as f64 * 0.2) as u64;
        let jittered = if jitter > 0 {
            capped + (rand_factor(attempt) % (jitter * 2)).saturating_sub(jitter)
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
```

#### Module Structure

```
crates/neunode-engine-api-client/
  Cargo.toml
  src/
    lib.rs           -- re-exports
    client.rs        -- EngineApiClient
    config.rs        -- EngineApiClientConfig
    error.rs         -- EngineApiError
    jwt.rs           -- JwtAuth, claims
    types.rs         -- all Engine API data structures
```

#### Usage Example

```rust
use neunode_engine_api_client::{
    EngineApiClient, EngineApiClientConfig,
    ForkchoiceStateV1, PayloadAttributesV3,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = EngineApiClientConfig {
        endpoint: "http://127.0.0.1:8551".into(),
        jwt_secret_path: Some("/data/reth/jwt.hex".into()),
        ..Default::default()
    };

    let client = EngineApiClient::new(config).await?;

    // Check capabilities
    let capabilities = client.exchange_capabilities(vec![
        "engine_newPayloadV3".into(),
        "engine_forkchoiceUpdatedV3".into(),
        "engine_getPayloadV3".into(),
    ]).await?;
    println!("EL supports: {capabilities:?}");

    // Propose a block
    let fcu = ForkchoiceStateV1 {
        head_block_hash: head_hash,
        safe_block_hash: safe_hash,
        finalized_block_hash: finalized_hash,
    };
    let attrs = PayloadAttributesV3 {
        timestamp: slot_timestamp,
        prev_randao: randao_reveal,
        suggested_fee_recipient: fee_recipient,
        withdrawals: vec![],
        parent_beacon_block_root: parent_root,
    };

    let block = client.propose_block(fcu, attrs).await?;
    println!("Block value: {} wei", block.block_value);

    Ok(())
}
```

### 1.7 Design Decisions

1. **reqwest over jsonrpsee**: Direct HTTP + JWT header control is simpler than jsonrpsee's transport abstraction for the authenticated Engine API. jsonrpsee would require a custom transport layer for JWT injection. reqwest gives us direct header control.

2. **Typed responses**: All Engine API methods return strongly-typed Rust structs with serde deserialization. The `alloy` crate (already in workspace) provides `B256`, `Address`, `U256`, and `Bytes` with serde support.

3. **Retry strategy**: Exponential backoff with jitter for SYNCING/timeout/connection errors. No retry for INVALID payloads or configuration errors.

4. **No WebSocket support initially**: The Engine API is primarily HTTP-based. WebSocket could be added later for streaming notifications, but the spec only requires HTTP with JWT per-request.

5. **Target Cancun (V3)**: Neunode should target the Cancun hardfork for its initial deployment. V3 is the current production version. Prague (V4) can be added when the spec stabilizes.

6. **JWT token caching**: The client generates a fresh JWT per request (iat must be within +/-60s). This is cheap and avoids clock skew issues.

---

## Part 2: Chain Spec Design (GitHub #28)

### 2.1 How Reth Defines Chain Specs

Reth supports two approaches:

**A. genesis.json file** (recommended for custom chains):
- Standard JSON format compatible with geth/Reth
- Loaded via `reth node --chain /path/to/genesis.json` or `reth init`
- Parsed into `alloy_genesis::Genesis`, then converted to `ChainSpec` via `impl From<Genesis> for ChainSpec`

**B. Rust builder API** (for programmatic construction):
```rust
let spec = ChainSpec::builder()
    .chain(Chain::from_id(CHAIN_ID))
    .genesis(genesis)
    .with_fork(EthereumHardfork::Frontier, ForkCondition::Block(0))
    .with_fork(EthereumHardfork::London, ForkCondition::Block(0))
    .with_fork(EthereumHardfork::Shanghai, ForkCondition::Timestamp(0))
    .with_fork(EthereumHardfork::Cancun, ForkCondition::Timestamp(0))
    .build();
```

The `ChainSpec` struct (from Reth source code) contains:
- `chain: Chain` -- chain ID + metadata
- `genesis: Genesis` -- genesis block configuration
- `genesis_header: SealedHeader` -- computed genesis header
- `hardforks: ChainHardforks` -- which hardforks are active when
- `deposit_contract: Option<DepositContract>` -- PoS deposit contract
- `base_fee_params: BaseFeeParamsKind` -- EIP-1559 parameters
- `blob_params: BlobScheduleBlobParams` -- EIP-4844 blob parameters
- `paris_block_and_final_difficulty: Option<(u64, U256)>` -- merge info

### 2.2 Custom Gas Token

**Key insight**: At the EVM execution layer, the native token is just "wei" -- it has no name or symbol at the protocol level. The concept of "ETH" vs "nTOKEN" is purely a UX/display concern. The EVM uses unsigned integers for value transfers and gas calculations; it does not know or care what the token is called.

What changes for a non-ETH gas token:
- **Chain ID**: Must be unique (avoids replay attacks with ETH mainnet)
- **Display conventions**: Wallets, explorers, and SDKs need to know the token name/symbol/decimals
- **No protocol-level changes**: The EVM does not have a concept of "token name" -- that is purely application layer

Reth does not require any code changes to support a differently-named native token. The `genesis.json` just defines `alloc` balances in wei. The token metadata is communicated via:
- `chainId` in the config
- A chain metadata file or EIP-3326 wallet `eth_switchEthereumChain` response
- Custom explorer/wallet configuration

### 2.3 Predeployed Contracts at Genesis

Contracts are predeployed by including their deployed bytecode and storage slots in the `alloc` section of `genesis.json`:

```json
{
  "alloc": {
    "0xContractAddress...": {
      "balance": "0x0",
      "code": "0x608060405234801561001057...",   // deployed bytecode
      "storage": {
        "0x0000000000000000000000000000000000000000000000000000000000000000": "0x...",
        "0x01": "0x..."
      }
    }
  }
}
```

Required system predeploys (activated by hardfork):
- **EIP-4788** (Cancun): Beacon roots contract at `0x000F3df6D732807Ef1319fB7B8bB8522d0Beac02` -- Reth handles this via system call, but the contract must be in genesis if Cancun is activated at genesis
- **EIP-2935** (Prague): Block hash history contract at `0x0000F90827F1C53a10cb7A02335B175320bd9765` -- Reth handles via system call

For Neunode-specific predeploys (Diamond proxy, resource tokens, etc.), we include their deployed bytecode in the genesis `alloc`.

### 2.4 EIP-1559 Parameters for Microtransactions

The default EIP-1559 parameters (used by Ethereum mainnet) are:

```
BaseFeeParams {
    max_change_denominator: 8,       // base fee changes by at most 1/8 per block
    elasticity_multiplier: 2,        // gas target = gas_limit / 2
}
```

For AI agent microtransactions, we want:
- Lower absolute gas fees
- More stable fee market
- Smaller price swings between blocks

Recommended adjustments for Neunode:

```
BaseFeeParams {
    max_change_denominator: 16,      // Slower base fee changes (1/16 vs 1/8)
    elasticity_multiplier: 4,        // Wider target band (gas_target = gas_limit / 4)
}
```

**Rationale:**
- `max_change_denominator: 16`: Halves the maximum per-block base fee change from 12.5% to 6.25%. This makes gas prices more predictable, which is critical for AI agents submitting many small transactions.
- `elasticity_multiplier: 4`: Allows blocks to fill to 4x the target gas before maximum fee increase kicks in. This provides more headroom for burst traffic (e.g., many agents submitting simultaneously).

**Additional gas parameters:**
- Initial base fee: 1 Gwei (vs 1 Gwei on mainnet) -- start low since network starts small
- Priority fee: Agents should use low priority fees (0.1-1 Gwei); block space will not be competitive initially

### 2.5 How Other Custom Chains Handle Specs

| Chain | Chain ID | Gas Token | Notes |
|---|---|---|---|
| Ethereum Mainnet | 1 | ETH | Reference |
| Polygon PoS | 137 | MATIC | Custom base fee params, lower gas limit |
| Optimism | 10 | ETH (L2) | Uses Optimism hardforks on top of standard ones |
| Arbitrum One | 42161 | ETH (L2) | Custom gas pricing via ArbOS |
| Gnosis Chain | 100 | xDAI | Fixed 1:1 stablecoin, custom base fee |
| Base | 8453 | ETH (L2) | Standard Optimism stack |
| Avalanche C-Chain | 43114 | AVAX | Custom base fee params |

Key patterns:
1. Custom chains define a unique chain ID (registered at chainlist.org)
2. EIP-1559 params are tuned to expected traffic patterns
3. Gas token naming is application-level, not protocol-level
4. Genesis allocations include system contracts needed for the chain's operation

### 2.6 Chain ID Selection

**Recommendation: Chain ID 9109**

Rationale:
- Not registered on chainlist.org (as of research date)
- "9109" is memorable ("9IO9" -> "NODO" in creative mnemonic, or just easy to remember)
- In the unofficial experimental range (>1000) but not conflicting with any known chain
- Can be registered on chainlist.org when Neunode launches

Alternative candidates: 910, 9009, 1909, 2909. All should be checked against chainlist.org before finalizing.

### 2.7 Neunode Chain Spec: `neunode-chain-spec`

#### genesis.json

```json
{
  "config": {
    "chainId": 9109,
    "homesteadBlock": 0,
    "eip150Block": 0,
    "eip155Block": 0,
    "eip158Block": 0,
    "byzantiumBlock": 0,
    "constantinopleBlock": 0,
    "petersburgBlock": 0,
    "istanbulBlock": 0,
    "berlinBlock": 0,
    "londonBlock": 0,
    "shanghaiTime": 0,
    "cancunTime": 0,
    "terminalTotalDifficulty": 0,
    "terminalTotalDifficultyPassed": true,
    "depositContractAddress": null,
    "blobSchedule": {
      "cancun": {
        "target": 3,
        "max": 6,
        "baseFeeUpdateFraction": 3338477
      },
      "prague": {
        "target": 6,
        "max": 9,
        "baseFeeUpdateFraction": 5007716
      }
    }
  },
  "nonce": "0x0",
  "timestamp": "0x0",
  "extraData": "0x8e65756e6f6465",
  "gasLimit": "0x1c9c380",
  "difficulty": "0x0",
  "mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
  "coinbase": "0x0000000000000000000000000000000000000000",
  "alloc": {
    "0x4e65756e6f6465000000000000000000000000": {
      "balance": "0x000000000000000000000000000000000000000000000152d02c7e14af6800000"
    }
  },
  "number": "0x0",
  "gasUsed": "0x0",
  "parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
  "baseFeePerGas": "0x3b9aca00"
}
```

#### Chain Metadata

```toml
# Chain metadata for wallets, explorers, and SDKs.
# Not part of the Reth chain spec itself, but used by tooling.

[chain]
name = "Neunode"
chain_id = 9109
network = "neunode-mainnet"

[native_currency]
name = "Neunode Token"
symbol = "NEUN"
decimals = 18

[gas]
# Block gas limit: 30M (same as Ethereum mainnet).
# Sufficient for complex AI agent interactions.
block_gas_limit = 30_000_000

# Initial base fee: 1 Gwei.
initial_base_fee = 1_000_000_000  # 1 Gwei in wei

# EIP-1559 parameters tuned for microtransactions.
[eip1559]
max_change_denominator = 16    # Slower fee changes (vs 8 on mainnet)
elasticity_multiplier = 4      # Wider target band (vs 2 on mainnet)

[blobs]
target_blob_count = 3
max_blob_count = 6
```

#### Rust Chain Spec Builder

```rust
// crates/neunode-chain-spec/src/lib.rs

use alloy_chains::Chain;
use alloy_eips::eip1559::BaseFeeParams;
use alloy_genesis::{Genesis, GenesisAccount};
use alloy_primitives::{address, bytes, U256};
use reth_chainspec::{ChainSpec, ChainSpecBuilder};
use reth_ethereum_forks::{EthereumHardfork, ForkCondition};
use std::collections::BTreeMap;

/// Neunode chain ID.
pub const NEUNODE_CHAIN_ID: u64 = 9109;

/// Neunode native token metadata (display only -- EVM does not use this).
pub const NEUNODE_TOKEN_NAME: &str = "Neunode Token";
pub const NEUNODE_TOKEN_SYMBOL: &str = "NEUN";
pub const NEUNODE_TOKEN_DECIMALS: u8 = 18;

/// Block gas limit: 30M gas.
pub const NEUNODE_BLOCK_GAS_LIMIT: u64 = 30_000_000;

/// Initial base fee: 1 Gwei.
pub const NEUNODE_INITIAL_BASE_FEE: u64 = 1_000_000_000;

/// EIP-1559 parameters tuned for AI agent microtransactions.
pub const NEUNODE_BASE_FEE_PARAMS: BaseFeeParams = BaseFeeParams::new(16, 4);

/// Build the Neunode chain specification.
pub fn neunode_chain_spec() -> ChainSpec {
    let chain = Chain::from_id(NEUNODE_CHAIN_ID);

    let mut alloc = BTreeMap::new();

    // Prefunded validator/deployer accounts.
    // Each gets 100M NEUN tokens (100_000_000 * 10^18).
    let initial_balance = U256::from(100_000_000u128) * U256::from(10u128).pow(U256::from(18));
    // TODO: Replace with actual validator addresses.
    let prefunded_addresses = [
        address!("0000000000000000000000000000000000000001"),
    ];
    for addr in &prefunded_addresses {
        alloc.insert(
            *addr,
            GenesisAccount {
                balance: initial_balance,
                ..Default::default()
            },
        );
    }

    // Predeployed contracts will be added here.
    // Example: Neunode Diamond proxy, Identity registry, etc.
    // Each needs:
    //   - deployed bytecode from `forge build`
    //   - constructor-initialized storage slots
    //   - zero balance

    let genesis = Genesis {
        config: alloy_genesis::ChainConfig {
            chain_id: NEUNODE_CHAIN_ID,
            homestead_block: Some(0),
            eip150_block: Some(0),
            eip155_block: Some(0),
            eip158_block: Some(0),
            byzantium_block: Some(0),
            constantinople_block: Some(0),
            petersburg_block: Some(0),
            istanbul_block: Some(0),
            berlin_block: Some(0),
            london_block: Some(0),
            shanghai_time: Some(0),
            cancun_time: Some(0),
            terminal_total_difficulty: Some(U256::ZERO),
            terminal_total_difficulty_passed: true,
            ..Default::default()
        },
        nonce: 0,
        timestamp: 0,
        extra_data: bytes!("8e65756e6f6465").to_vec().into(),  // "neunode" ASCII
        gas_limit: NEUNODE_BLOCK_GAS_LIMIT,
        difficulty: U256::ZERO,
        alloc,
        base_fee_per_gas: Some(NEUNODE_INITIAL_BASE_FEE as u128),
        ..Default::default()
    };

    ChainSpecBuilder::default()
        .chain(chain)
        .genesis(genesis)
        .with_fork(EthereumHardfork::Frontier, ForkCondition::Block(0))
        .with_fork(EthereumHardfork::Homestead, ForkCondition::Block(0))
        .with_fork(EthereumHardfork::Tangerine, ForkCondition::Block(0))
        .with_fork(EthereumHardfork::SpuriousDragon, ForkCondition::Block(0))
        .with_fork(EthereumHardfork::Byzantium, ForkCondition::Block(0))
        .with_fork(EthereumHardfork::Constantinople, ForkCondition::Block(0))
        .with_fork(EthereumHardfork::Petersburg, ForkCondition::Block(0))
        .with_fork(EthereumHardfork::Istanbul, ForkCondition::Block(0))
        .with_fork(EthereumHardfork::Berlin, ForkCondition::Block(0))
        .with_fork(EthereumHardfork::London, ForkCondition::Block(0))
        .with_fork(EthereumHardfork::Paris, ForkCondition::TTD {
            activation_block_number: 0,
            total_difficulty: U256::ZERO,
            fork_block: None,
        })
        .with_fork(EthereumHardfork::Shanghai, ForkCondition::Timestamp(0))
        .with_fork(EthereumHardfork::Cancun, ForkCondition::Timestamp(0))
        .build()
}

/// Contract predeploy descriptor.
pub struct PredeployedContract {
    /// Address at which to deploy.
    pub address: alloy_primitives::Address,
    /// Deployed bytecode (from `forge build`).
    pub bytecode: alloy_primitives::Bytes,
    /// Pre-initialized storage slots (slot -> value).
    pub storage: BTreeMap<alloy_primitives::U256, alloy_primitives::U256>,
    /// Initial balance in wei.
    pub balance: alloy_primitives::U256,
}
```

### 2.8 Predeployed Contracts List

The following Neunode contracts should be predeployed at genesis. Addresses are placeholders and should be finalized before deployment.

| Contract | Address (placeholder) | Notes |
|---|---|---|
| Neunode Diamond Proxy | `0xNeunode00000000000000000000000000001` | EIP-2535 diamond, entry point for all facets |
| NeunodeIdentity | facet of diamond | Agent identity registry |
| NeunodeBounty | facet of diamond | Bounty lifecycle management |
| NeunodeEscrow | facet of diamond | Escrow for bounty payments |
| NeunodeRegistry | `0xNeunode00000000000000000000000000002` | Model/agent registry |
| nCompute Token | `0xNeunode00000000000000000000000000003` | ERC-20 resource token |
| nTrain Token | `0xNeunode00000000000000000000000000004` | ERC-20 resource token |
| nBandwidth Token | `0xNeunode00000000000000000000000000005` | ERC-20 resource token |
| nStorage Token | `0xNeunode00000000000000000000000000006` | ERC-20 resource token |
| ModelRegistry | `0xNeunode00000000000000000000000000007` | Model lineage DAG |
| RoyaltySplitter | `0xNeunode00000000000000000000000000008` | Lineage royalty distribution |
| NeunodeGovernance | `0xNeunode00000000000000000000000000009` | DAO governance |

System predeploys (handled by Reth):
- EIP-4788 BeaconRoots: `0x000F3df6D732807Ef1319fB7B8bB8522d0Beac02`
- EIP-2935 BlockHashHistory (if Prague): `0x0000F90827F1C53a10cb7A02335B175320bd9765`

### 2.9 Block Gas Limit Recommendation

**Recommendation: 30M gas (same as Ethereum mainnet)**

Rationale:
- AI agent transactions can be complex (model registration, bounty creation, multi-step verification)
- 30M is well-tested and sufficient for most transaction patterns
- Can be adjusted via governance if needed (Reth supports `--target-gas-limit` for block builders)
- For comparison: Polygon uses 20M, Gnosis uses 17M, but these chains optimize for high throughput of simple transactions

If Neunode finds that agents need very high throughput of simple transactions (e.g., thousands of inference attestations per block), the gas limit can be raised to 60M-100M via governance.

---

## Appendix A: Research Sources

- Ethereum Execution API specification: https://github.com/ethereum/execution-apis/tree/main/src/engine
- Engine API authentication: https://github.com/ethereum/execution-apis/blob/main/src/engine/authentication.md
- Reth ChainSpec source: https://github.com/paradigmxyz/reth/blob/main/crates/chainspec/src/spec.rs
- Reth ChainSpecBuilder API: https://github.com/paradigmxyz/reth (from Context7 documentation)
- alloy_genesis types: https://github.com/alloy-rs/alloy (Genesis, ChainConfig, GenesisAccount)

## Appendix B: Open Questions

1. **Chain ID finalization**: Need to verify 9109 is not registered on chainlist.org. Should register when ready.
2. **Prague timing**: Should Neunode activate Prague at genesis or delay it? Prague adds EIP-7685 execution requests (deposits, withdrawals, consolidations). For a non-staked chain, this may be unnecessary initially.
3. **Blob schedule**: The blob parameters in the genesis JSON may need adjustment based on expected blob usage. AI model weights could be stored as blobs.
4. **Predeploy bytecode generation**: Need a build step that compiles Solidity contracts and extracts deployed bytecode + storage for genesis. This could be a Forge script or a Cargo build.rs step.
5. **Validator set management**: How does Neunode's consensus layer manage validators? If PoA initially, no deposit contract is needed. If PoS, a deposit contract must be predeployed.
6. **Engine API client: jsonrpsee vs reqwest**: If we need WebSocket support or want to share code with other JSON-RPC interfaces, jsonrpsee may be worth the complexity. Revisit if WebSocket is needed.
