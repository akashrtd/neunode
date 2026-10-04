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
    #[serde(default = "first_sequence")]
    pub from_sequence: u64,
}

fn first_sequence() -> u64 {
    1
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
    /// Full signed body, including tags and references, for independent verification.
    pub event: Option<serde_json::Value>,
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

    if query.kind.is_some_and(|kind| u16::try_from(kind).is_err()) || query.limit > 1000 {
        return Err(ApiError::BadRequest("invalid kind or limit exceeds 1000".into()));
    }
    let store = neunode_storage::feed_store::FeedStore::new(&state.db);
    let events = store
        .get_range(&did, query.from_sequence, query.limit)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let filtered: Vec<FeedEventResponse> = events
        .into_iter()
        .filter(|e| query.kind.is_none_or(|k| e.kind == k as u16))
        .take(query.limit)
        .map(|e| FeedEventResponse {
            sequence: e.sequence,
            kind: e.kind,
            timestamp: e.timestamp,
            author_did: e.agent_did.clone(),
            content: crate::feed_wire::stored_to_event(&e)
                .map(|event| event.content)
                .unwrap_or_else(|_| String::from_utf8_lossy(&e.payload).into_owned()),
            signature: String::from_utf8_lossy(&e.signature).into_owned(),
            event: crate::feed_wire::stored_to_event(&e)
                .ok()
                .and_then(|event| serde_json::to_value(event).ok()),
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

    let event = {
        let keyring = state.require_keyring()?;
        crate::feed_wire::create_event(
            &state.db,
            &keyring,
            body.kind,
            body.content.clone(),
            &body.tags.unwrap_or_default(),
        )
        .map_err(|error| ApiError::BadRequest(error.to_string()))?
    };
    let did = &event.author;
    let next_seq = event.sequence;
    let event_id = event.id.0.clone();
    let wire = {
        let keyring = state.require_keyring()?;
        crate::feed_wire::serialize_authenticated_event(&event, &keyring)?
    };
    if let Some(mesh) = state.mesh_handle.read().await.as_ref() {
        mesh.publish(event.kind.gossipsub_topic(), &wire)?;
    }

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
    let store = neunode_storage::feed_store::FeedStore::new(&state.db);
    let stored = if let Some(sequence) = event_id.strip_prefix("seq:") {
        let did = state.require_did()?;
        let sequence =
            sequence.parse().map_err(|_| ApiError::BadRequest("invalid sequence".into()))?;
        store.get(&did.0, sequence)?
    } else if let Some(wire) =
        state.db.get_raw(neunode_storage::cf::CF_FEED_INDEX, event_id.as_bytes())?
    {
        let (did, sequence) = crate::feed_wire::authenticated_position(&wire)?;
        store.get(&did, sequence)?
    } else {
        None
    }
    .ok_or_else(|| ApiError::NotFound(format!("event '{event_id}' not found")))?;
    let event = crate::feed_wire::stored_to_event(&stored)?;
    Ok(types::ok(FeedEventResponse {
        sequence: stored.sequence,
        kind: stored.kind,
        timestamp: stored.timestamp,
        author_did: stored.agent_did,
        content: event.content.clone(),
        signature: event
            .signature
            .as_ref()
            .map(|signature| signature.0.clone())
            .unwrap_or_default(),
        event: Some(
            serde_json::to_value(event).map_err(|error| ApiError::Internal(error.to_string()))?,
        ),
    }))
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
            event: None,
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
