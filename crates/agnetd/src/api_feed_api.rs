use std::sync::Arc;

use axum::extract::{Path, Query, State};
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
pub struct PostFeedRequest {
    pub kind: u32,
    pub content: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FeedListQuery {
    pub kind: Option<u32>,
    pub author: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FeedEventResponse {
    pub sequence: u64,
    pub kind: u16,
    pub timestamp: u64,
    pub author_did: String,
    pub content: String,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PostFeedResponse {
    pub event_id: String,
    pub sequence: u64,
    pub kind: u16,
    pub topic: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/feed",
    params(
        ("kind" = Option<u32>, Query, description = "Filter by event kind"),
        ("author" = Option<String>, Query, description = "Filter by author DID"),
        ("limit" = Option<usize>, Query, description = "Max results (default 50)"),
    ),
    responses(
        (status = 200, description = "List of feed events", body = Vec<FeedEventResponse>),
        (status = 401, description = "No active identity"),
    ),
    tag = "feed",
)]
pub async fn list_feed(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<FeedListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let did = match &query.author {
        Some(a) => a.clone(),
        None => state.require_did()?.0.clone(),
    };

    let store = neunode_storage::feed_store::FeedStore::new(&state.db);
    let events = store.get_all(&did).map_err(|e| ApiError::Internal(e.to_string()))?;

    let filtered: Vec<FeedEventResponse> = events
        .into_iter()
        .filter(|e| query.kind.is_none_or(|k| e.kind == k as u16))
        .take(query.limit)
        .map(|e| FeedEventResponse {
            sequence: e.sequence,
            kind: e.kind,
            timestamp: e.timestamp,
            author_did: e.agent_did,
            content: String::from_utf8(e.payload).unwrap_or_else(|_| "(binary)".to_string()),
            signature: hex::encode(&e.signature),
        })
        .collect();

    Ok(types::ok(filtered))
}

#[utoipa::path(
    post,
    path = "/api/v1/feed",
    request_body = PostFeedRequest,
    responses(
        (status = 201, description = "Feed event posted", body = PostFeedResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "No active identity"),
    ),
    tag = "feed",
)]
pub async fn post_feed(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<PostFeedRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.content.is_empty() {
        return Err(ApiError::BadRequest("content cannot be empty".to_string()));
    }

    let did = state.require_did()?;
    let _keyring = state.require_keyring()?;

    let store = neunode_storage::feed_store::FeedStore::new(&state.db);
    let latest_seq =
        store.latest_sequence(&did.0).map_err(|e| ApiError::Internal(e.to_string()))?;
    let next_seq = if latest_seq == 0 { 1 } else { latest_seq + 1 };

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let stored = neunode_storage::feed_store::StoredEvent {
        kind: body.kind as u16,
        timestamp: now_ts,
        agent_did: did.0.clone(),
        sequence: next_seq,
        prev_hash: vec![0u8; 32],
        payload: body.content.as_bytes().to_vec(),
        signature: vec![],
    };
    store.append(&stored).map_err(|e| ApiError::Internal(e.to_string()))?;

    let event_id = event_id(&did.0, next_seq);

    let _ = state.feed_tx.send(crate::api::state::FeedEventUpdate {
        kind: body.kind as u16,
        author_did: did.0.clone(),
        author_short: did.0.chars().take(18).collect(),
        kind_label: body.kind.to_string(),
        preview: body.content.chars().take(80).collect(),
        time_ago: "now".to_string(),
    });

    let resp = PostFeedResponse {
        event_id,
        sequence: next_seq,
        kind: body.kind as u16,
        topic: format!("feed/kind/{}", body.kind),
    };

    Ok(types::created(resp))
}

#[utoipa::path(
    get,
    path = "/api/v1/feed/{event_id}",
    params(
        ("event_id" = String, Path, description = "Event ID or sequence identifier"),
    ),
    responses(
        (status = 200, description = "Feed event details", body = FeedEventResponse),
        (status = 401, description = "No active identity"),
        (status = 404, description = "Event not found"),
    ),
    tag = "feed",
)]
pub async fn show_feed_event(
    State(state): State<Arc<ApiState>>,
    Path(event_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let did = state.require_did()?;
    let store = neunode_storage::feed_store::FeedStore::new(&state.db);

    let events = store.get_all(&did.0).map_err(|e| ApiError::Internal(e.to_string()))?;

    let found = events.iter().find(|event| {
        event_id == self::event_id(&event.agent_did, event.sequence)
            || event_id == format!("seq:{}", event.sequence)
    });

    match found {
        Some(event) => {
            let resp = FeedEventResponse {
                sequence: event.sequence,
                kind: event.kind,
                timestamp: event.timestamp,
                author_did: event.agent_did.clone(),
                content: String::from_utf8(event.payload.clone())
                    .unwrap_or_else(|_| "(binary)".to_string()),
                signature: hex::encode(&event.signature),
            };
            Ok(types::ok(resp))
        }
        None => Err(ApiError::NotFound(format!("event '{event_id}' not found"))),
    }
}

fn event_id(did: &str, sequence: u64) -> String {
    format!("evt_{}_{}", hex::encode(&did.as_bytes()[..8.min(did.len())]), sequence)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_feed_request_parse() {
        let req: PostFeedRequest =
            serde_json::from_str(r#"{"kind": 9001, "content": "hello world"}"#).unwrap();
        assert_eq!(req.kind, 9001);
        assert_eq!(req.content, "hello world");
        assert!(req.tags.is_none());
    }

    #[test]
    fn post_feed_request_with_tags() {
        let req: PostFeedRequest = serde_json::from_str(
            r#"{"kind": 1, "content": "test", "tags": ["key=value", "env=prod"]}"#,
        )
        .unwrap();
        assert_eq!(req.tags.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn feed_list_query_defaults() {
        let query: FeedListQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.limit, 50);
        assert!(query.kind.is_none());
        assert!(query.author.is_none());
    }

    #[test]
    fn feed_list_query_custom() {
        let query: FeedListQuery =
            serde_json::from_str(r#"{"kind": 42, "author": "did:neunode:abc", "limit": 10}"#)
                .unwrap();
        assert_eq!(query.kind, Some(42));
        assert_eq!(query.author.as_deref(), Some("did:neunode:abc"));
        assert_eq!(query.limit, 10);
    }

    #[test]
    fn feed_event_response_serde_roundtrip() {
        let resp = FeedEventResponse {
            sequence: 7,
            kind: 9001,
            timestamp: 1700000000,
            author_did: "did:neunode:0xABC".to_string(),
            content: "hello".to_string(),
            signature: "deadbeef".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: FeedEventResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.sequence, back.sequence);
        assert_eq!(resp.kind, back.kind);
        assert_eq!(resp.content, back.content);
    }

    #[test]
    fn post_feed_response_serde_roundtrip() {
        let resp = PostFeedResponse {
            event_id: "evt_abc_1".to_string(),
            sequence: 1,
            kind: 42,
            topic: "feed/kind/42".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: PostFeedResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.event_id, back.event_id);
        assert_eq!(resp.sequence, back.sequence);
    }
}
