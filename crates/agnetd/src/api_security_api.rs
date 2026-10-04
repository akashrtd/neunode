//! Authenticated operator controls for a running daemon.
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use neunode_storage::breaker_store::{self, BreakerState};
use serde::{Deserialize, Serialize};

use super::{error::ApiError, state::ApiState, types};

#[derive(Serialize, utoipa::ToSchema)]
pub struct BreakerResponse {
    pub name: String,
    pub open: bool,
    pub trip_count: u64,
    pub tripped_at: Option<u64>,
    pub mode: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct BreakerRequest {
    pub open: bool,
}

#[utoipa::path(get, path = "/api/v1/security/breakers", responses((status = 200, body = [BreakerResponse])), tag = "security")]
pub async fn list(State(state): State<Arc<ApiState>>) -> Result<impl IntoResponse, ApiError> {
    let rows = breaker_store::NAMES
        .into_iter()
        .map(|name| {
            let record = breaker_store::load(&state.db, name)?;
            Ok(BreakerResponse {
                name: name.into(),
                open: record.state == BreakerState::Open,
                trip_count: record.trip_count,
                tripped_at: record.tripped_at,
                mode: "manual".into(),
            })
        })
        .collect::<Result<Vec<_>, neunode_storage::error::StorageError>>()?;
    Ok(types::ok(rows))
}

#[utoipa::path(post, path = "/api/v1/security/breakers/{name}", params(("name" = String, Path)), request_body = BreakerRequest, responses((status = 200, body = BreakerResponse)), tag = "security")]
pub async fn set(
    State(state): State<Arc<ApiState>>,
    Path(name): Path<String>,
    Json(body): Json<BreakerRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if !breaker_store::NAMES.contains(&name.as_str()) {
        return Err(ApiError::BadRequest("unknown circuit breaker".into()));
    }
    let record = state.db.with_ledger_write(|| {
        let mut record = breaker_store::load(&state.db, &name)?;
        if body.open && record.state != BreakerState::Open {
            record.trip_count = record.trip_count.checked_add(1).ok_or_else(|| {
                neunode_storage::error::StorageError::Serialization("trip count overflow".into())
            })?;
            record.tripped_at = Some(chrono::Utc::now().timestamp().max(0) as u64);
        } else if !body.open {
            record.tripped_at = None;
        }
        record.state = if body.open { BreakerState::Open } else { BreakerState::Closed };
        breaker_store::save(&state.db, &name, &record)?;
        Ok::<_, neunode_storage::error::StorageError>(record)
    })?;
    Ok(types::ok(BreakerResponse {
        name,
        open: record.state == BreakerState::Open,
        trip_count: record.trip_count,
        tripped_at: record.tripped_at,
        mode: "manual".into(),
    }))
}
