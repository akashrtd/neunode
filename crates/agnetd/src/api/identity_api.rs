use std::sync::Arc;

use axum::extract::State;
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
pub struct CreateIdentityRequest {
    pub name: String,
    #[serde(default = "default_method")]
    pub method: String,
}

fn default_method() -> String {
    "key".to_string()
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IdentityResponse {
    pub did: String,
    pub method: String,
    pub name: String,
    pub ethereum: String,
    pub peer_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IdentityListItem {
    pub did: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OnChainRegistrationResponse {
    pub tx_hash: String,
    pub block_number: u64,
    pub did_hash: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/identity",
    responses(
        (status = 200, description = "Active identity details", body = IdentityResponse),
        (status = 401, description = "No active identity"),
    ),
    tag = "identity",
)]
pub async fn show_identity(
    State(state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    let did = state.require_did()?;

    let store = neunode_storage::identity_store::IdentityStore::new(&state.db);
    let doc_json: Option<String> =
        store.get(&did.0).map_err(|e| ApiError::Internal(e.to_string()))?;

    match doc_json {
        Some(json_str) => {
            let doc = neunode_identity::document::DidDocument::from_json(&json_str)
                .map_err(|e| ApiError::Internal(format!("failed to parse DID document: {e}")))?;
            let resp = IdentityResponse {
                did: doc.id.clone(),
                method: "persisted".to_string(),
                name: did.0.split(':').next_back().unwrap_or(&did.0).to_string(),
                ethereum: doc
                    .verification_method
                    .first()
                    .map(|_| "see document".to_string())
                    .unwrap_or_default(),
                peer_id: "see document".to_string(),
            };
            Ok(types::ok(resp))
        }
        None => Err(ApiError::NotFound(format!("identity '{}' not found in local store", did.0))),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/identity",
    request_body = CreateIdentityRequest,
    responses(
        (status = 201, description = "Identity created", body = IdentityResponse),
        (status = 400, description = "Invalid parameters"),
        (status = 401, description = "No active identity (keyring required)"),
    ),
    tag = "identity",
)]
pub async fn create_identity(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<CreateIdentityRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.name.is_empty() {
        return Err(ApiError::BadRequest("name cannot be empty".to_string()));
    }
    if !matches!(body.method.as_str(), "key" | "neunode") {
        return Err(ApiError::BadRequest(format!(
            "unsupported DID method: '{}'. Use 'key' or 'neunode'.",
            body.method
        )));
    }

    let keyring = state.require_keyring()?;
    let did = keyring.to_did();
    let did_key = keyring.to_did_key();
    let peer_id = neunode_identity::did::did_to_peer_id(&did_key)
        .map(|p| p.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let eth_addr = keyring.ethereum_address();

    let mut capabilities = Vec::new();
    if body.method == "neunode" {
        capabilities.push("inference".to_string());
        capabilities.push("training".to_string());
    }

    let card = neunode_identity::agent_card::AgentCard::new(
        &body.name,
        &keyring,
        capabilities,
        std::collections::HashMap::new(),
    )
    .map_err(|e| ApiError::Internal(format!("failed to create agent card: {e}")))?;

    let doc = neunode_identity::document::DidDocument::from_keyring(&keyring)
        .map_err(|e| ApiError::Internal(format!("failed to create DID document: {e}")))?;

    let store = neunode_storage::identity_store::IdentityStore::new(&state.db);
    let doc_json = doc
        .to_json()
        .map_err(|e| ApiError::Internal(format!("failed to serialize DID document: {e}")))?;
    store.put(&did.to_string(), &doc_json).map_err(|e| ApiError::Internal(e.to_string()))?;

    let card_cid = card.to_cid();

    let resp = IdentityResponse {
        did: did.to_string(),
        method: body.method.clone(),
        name: body.name.clone(),
        ethereum: eth_addr,
        peer_id,
    };

    Ok(types::created(serde_json::json!({
        "identity": resp,
        "card_cid": card_cid.to_string(),
    })))
}

#[utoipa::path(
    get,
    path = "/api/v1/identity/list",
    responses(
        (status = 200, description = "List of all identities", body = Vec<IdentityListItem>),
    ),
    tag = "identity",
)]
pub async fn list_identities(
    State(state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    let entries = state
        .db
        .prefix_scan(neunode_storage::cf::CF_IDENTITY, &[])
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let mut identities: Vec<IdentityListItem> = Vec::new();
    for (_, value_bytes) in &entries {
        if let Ok(doc_json) = bincode::deserialize::<String>(value_bytes) {
            if let Ok(doc) = neunode_identity::document::DidDocument::from_json(&doc_json) {
                identities
                    .push(IdentityListItem { did: doc.id.clone(), status: "stored".to_string() });
            }
        }
    }

    Ok(types::ok(identities))
}

#[utoipa::path(
    post,
    path = "/api/v1/identity/register-onchain",
    responses(
        (status = 200, description = "Identity registered on-chain", body = OnChainRegistrationResponse),
        (status = 401, description = "No active identity"),
        (status = 400, description = "On-chain registration not configured"),
    ),
    tag = "identity",
)]
pub async fn register_onchain(
    State(state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    let contracts = &state.config.app_config.contracts;
    let rpc_url = match &contracts.eth_rpc_url {
        Some(url) => url.clone(),
        None => {
            return Err(ApiError::BadRequest(
                "on-chain registration not configured — set contracts.eth_rpc_url".to_string(),
            ));
        }
    };
    let contract_addr = match &contracts.identity_contract_address {
        Some(addr) => addr.clone(),
        None => {
            return Err(ApiError::BadRequest(
                "on-chain registration not configured — set contracts.identity_contract_address"
                    .to_string(),
            ));
        }
    };
    let onchain_config = neunode_identity::contracts::OnChainConfig {
        eth_rpc_url: rpc_url,
        identity_contract_address: contract_addr,
    };

    // Extract key bytes while holding the lock, then drop it before .await
    let (ed_bytes, secp_bytes) = {
        let keyring = state.require_keyring()?;
        let (ed, secp) = keyring.to_bytes();
        (ed, secp)
    };
    let ed_arr: [u8; 32] =
        ed_bytes.try_into().map_err(|_| ApiError::Internal("ed25519 key size mismatch".into()))?;
    let secp_arr: [u8; 32] = secp_bytes
        .try_into()
        .map_err(|_| ApiError::Internal("secp256k1 key size mismatch".into()))?;
    let kr = neunode_identity::keyring::Keyring::from_bytes(&ed_arr, &secp_arr)
        .map_err(|e| ApiError::Internal(format!("key reconstruction failed: {e}")))?;

    let result = neunode_identity::contracts::register_did_onchain(&onchain_config, &kr)
        .await
        .map_err(|e| ApiError::Internal(format!("on-chain registration failed: {e}")))?;

    let resp = OnChainRegistrationResponse {
        tx_hash: result.tx_hash.clone(),
        block_number: result.block_number,
        did_hash: format!("0x{}", hex::encode(result.did_hash)),
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
    fn create_identity_request_default_method() {
        let req: CreateIdentityRequest = serde_json::from_str(r#"{"name": "test"}"#).unwrap();
        assert_eq!(req.method, "key");
        assert_eq!(req.name, "test");
    }

    #[test]
    fn create_identity_request_custom_method() {
        let req: CreateIdentityRequest =
            serde_json::from_str(r#"{"name": "agent", "method": "neunode"}"#).unwrap();
        assert_eq!(req.method, "neunode");
    }

    #[test]
    fn identity_response_serde_roundtrip() {
        let resp = IdentityResponse {
            did: "did:neunode:0xABC".to_string(),
            method: "key".to_string(),
            name: "test".to_string(),
            ethereum: "0x1234".to_string(),
            peer_id: "12D3Koo...".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: IdentityResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.did, back.did);
        assert_eq!(resp.name, back.name);
    }

    #[test]
    fn identity_list_item_serde_roundtrip() {
        let item =
            IdentityListItem { did: "did:neunode:0xDEF".to_string(), status: "stored".to_string() };
        let json = serde_json::to_string(&item).unwrap();
        let back: IdentityListItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item.did, back.did);
        assert_eq!(item.status, back.status);
    }

    #[test]
    fn onchain_response_serde_roundtrip() {
        let resp = OnChainRegistrationResponse {
            tx_hash: "0xabc".to_string(),
            block_number: 42,
            did_hash: "0xdead".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: OnChainRegistrationResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.tx_hash, back.tx_hash);
        assert_eq!(resp.block_number, back.block_number);
    }
}
