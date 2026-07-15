use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use neunode_storage::audit_store::{AuditEntry, AuditStore};

use super::error::ApiError;
use super::state::ApiState;
use super::types;

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 1000;

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    #[serde(default)]
    pub after: u64,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct AuditPage {
    pub entries: Vec<AuditEntry>,
    pub next_sequence: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct AuditVerification {
    pub valid: bool,
}

pub async fn list_audit(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<AuditQuery>,
) -> Result<impl IntoResponse, ApiError> {
    state.require_did()?;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT);
    if limit == 0 || limit > MAX_LIMIT {
        return Err(ApiError::BadRequest(format!("limit must be between 1 and {MAX_LIMIT}")));
    }
    let entries = AuditStore::new(&state.db).entries_from(query.after, limit)?;
    let next_sequence = entries.last().and_then(|entry| entry.sequence.checked_add(1));
    Ok(types::ok(AuditPage { entries, next_sequence }))
}

pub async fn verify_audit(
    State(state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    state.require_did()?;
    AuditStore::new(&state.db).verify_chain()?;
    Ok(types::ok(AuditVerification { valid: true }))
}
