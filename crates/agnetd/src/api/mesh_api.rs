use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::error::ApiError;
use super::state::ApiState;
use super::types;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ConnectRequest {
    pub addr: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DisconnectRequest {
    pub peer_id: String,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct PeersQuery {
    pub verbose: Option<bool>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MeshStatusResponse {
    pub running: bool,
    pub local_peer_id: Option<String>,
    pub listeners: Vec<String>,
    pub connected_peers: Vec<String>,
    pub subscribed_topics: Vec<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PeersResponse {
    pub peers: Vec<String>,
    pub count: usize,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConnectResponse {
    pub addr: String,
    pub status: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DisconnectResponse {
    pub peer_id: String,
    pub status: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/mesh/status",
    responses(
        (status = 200, description = "Mesh status retrieved", body = MeshStatusResponse)
    ),
    tag = "mesh",
)]
pub async fn mesh_status(
    State(state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    let guard = state.mesh_handle.read().await;
    match guard.as_ref() {
        Some(handle) => {
            let status = handle.status().await.map_err(|e| ApiError::Internal(e.to_string()))?;
            Ok(types::ok(MeshStatusResponse {
                running: status.running,
                local_peer_id: Some(status.local_peer_id),
                listeners: status.listeners,
                connected_peers: status.connected_peers,
                subscribed_topics: status.subscribed_topics,
            }))
        }
        None => Ok(types::ok(MeshStatusResponse {
            running: false,
            local_peer_id: None,
            listeners: vec![],
            connected_peers: vec![],
            subscribed_topics: vec![],
        })),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/mesh/peers",
    params(PeersQuery),
    responses(
        (status = 200, description = "List of connected peers", body = PeersResponse)
    ),
    tag = "mesh",
)]
pub async fn list_peers(
    State(state): State<Arc<ApiState>>,
    Query(_query): Query<PeersQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let guard = state.mesh_handle.read().await;
    match guard.as_ref() {
        Some(handle) => {
            let peers = handle.peers().await.map_err(|e| ApiError::Internal(e.to_string()))?;
            let peer_list: Vec<String> = peers.iter().map(|p| p.to_string()).collect();
            let count = peer_list.len();
            Ok(types::ok(PeersResponse { peers: peer_list, count }))
        }
        None => Ok(types::ok(PeersResponse { peers: vec![], count: 0 })),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/mesh/connect",
    request_body = ConnectRequest,
    responses(
        (status = 200, description = "Connection initiated", body = ConnectResponse)
    ),
    tag = "mesh",
)]
pub async fn connect_peer(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<ConnectRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let guard = state.mesh_handle.read().await;
    let handle = guard.as_ref().ok_or(ApiError::MeshNotRunning)?;

    let addr: libp2p::Multiaddr = body
        .addr
        .parse()
        .map_err(|e| ApiError::BadRequest(format!("invalid multiaddr '{}': {e}", body.addr)))?;

    handle.dial(addr).map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(types::ok(ConnectResponse { addr: body.addr, status: "dialing".to_string() }))
}

#[utoipa::path(
    post,
    path = "/api/v1/mesh/disconnect",
    request_body = DisconnectRequest,
    responses(
        (status = 200, description = "Disconnected from peer", body = DisconnectResponse)
    ),
    tag = "mesh",
)]
pub async fn disconnect_peer(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<DisconnectRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let guard = state.mesh_handle.read().await;
    let handle = guard.as_ref().ok_or(ApiError::MeshNotRunning)?;

    let peer_id: libp2p::PeerId = body
        .peer_id
        .parse()
        .map_err(|e| ApiError::BadRequest(format!("invalid peer ID '{}': {e}", body.peer_id)))?;

    handle.disconnect(peer_id).map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(types::ok(DisconnectResponse { peer_id: body.peer_id, status: "disconnected".to_string() }))
}
