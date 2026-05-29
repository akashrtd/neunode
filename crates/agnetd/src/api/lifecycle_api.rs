use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use super::error::ApiError;
use super::state::ApiState;
use super::types;

// ---------------------------------------------------------------------------
// Lifecycle state definitions (mirrors cmd_lifecycle.rs)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, utoipa::ToSchema)]
pub enum AgentState {
    Created,
    Active,
    Hibernating,
    Idle,
    Zombie,
    Dead,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentState::Created => write!(f, "CREATED"),
            AgentState::Active => write!(f, "ACTIVE"),
            AgentState::Hibernating => write!(f, "HIBERNATING"),
            AgentState::Idle => write!(f, "IDLE"),
            AgentState::Zombie => write!(f, "ZOMBIE"),
            AgentState::Dead => write!(f, "DEAD"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LifecycleRecord {
    did: String,
    state: AgentState,
    last_activity: u64,
    activated_at: Option<u64>,
    hibernated_at: Option<u64>,
    tombstoned_at: Option<u64>,
    derived_from: Option<String>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LifecycleStatusResponse {
    pub did: String,
    pub state: String,
    pub last_activity: u64,
    pub elapsed_secs: u64,
    pub activated_at: Option<u64>,
    pub warning: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AgentSummary {
    pub did: String,
    pub state: String,
    pub last_activity: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ReapTransition {
    pub did: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ReapResult {
    pub transitions: Vec<ReapTransition>,
    pub count: usize,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct NoRecordResponse {
    pub message: String,
}

// ---------------------------------------------------------------------------
// Storage helpers
// ---------------------------------------------------------------------------

const LIFECYCLE_PREFIX: &str = "lifecycle:";

fn lifecycle_key(did: &str) -> String {
    format!("{LIFECYCLE_PREFIX}{did}")
}

fn get_record(
    db: &neunode_storage::db::NeunodeDb,
    did: &str,
) -> Result<Option<LifecycleRecord>, ApiError> {
    let key = lifecycle_key(did);
    let store = neunode_storage::identity_store::IdentityStore::new(db);
    Ok(store.get(&key)?)
}

fn put_record(
    db: &neunode_storage::db::NeunodeDb,
    record: &LifecycleRecord,
) -> Result<(), ApiError> {
    let key = lifecycle_key(&record.did);
    let store = neunode_storage::identity_store::IdentityStore::new(db);
    store.put(&key, record)?;
    Ok(())
}

fn now_ts() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

// Thresholds (Doc 16)
const IDLE_THRESHOLD_SECS: u64 = 7 * 86400;
const ZOMBIE_THRESHOLD_SECS: u64 = 30 * 86400;
const DEATH_THRESHOLD_SECS: u64 = 90 * 86400;
const ZOMBIE_WARNING_SECS: u64 = 25 * 86400;

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/lifecycle/status",
    responses(
        (status = 200, description = "Current agent lifecycle status", body = LifecycleStatusResponse)
    ),
    tag = "lifecycle",
)]
pub async fn lifecycle_status(
    State(state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    let did = state.require_did()?;

    match get_record(&state.db, &did.0)? {
        Some(record) => {
            let elapsed = now_ts().saturating_sub(record.last_activity);
            let warning = if record.state == AgentState::Active && elapsed > ZOMBIE_WARNING_SECS {
                let days_to_zombie = (ZOMBIE_THRESHOLD_SECS.saturating_sub(elapsed)) / 86400;
                Some(format!("Zombie warning: {days_to_zombie} days until zombification"))
            } else {
                None
            };

            Ok(types::ok(LifecycleStatusResponse {
                did: did.0.clone(),
                state: record.state.to_string(),
                last_activity: record.last_activity,
                elapsed_secs: elapsed,
                activated_at: record.activated_at,
                warning,
            }))
        }
        None => Ok(types::ok(NoRecordResponse {
            message: "No lifecycle record. POST /api/v1/lifecycle/activate to register."
                .to_string(),
        })),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/lifecycle/activate",
    responses(
        (status = 200, description = "Agent activated")
    ),
    tag = "lifecycle",
)]
pub async fn activate(State(state): State<Arc<ApiState>>) -> Result<impl IntoResponse, ApiError> {
    let did = state.require_did()?;
    let now = now_ts();

    match get_record(&state.db, &did.0)? {
        Some(mut record) => {
            if record.state == AgentState::Active {
                return Err(ApiError::BadRequest("agent is already ACTIVE".into()));
            }
            if record.state == AgentState::Dead {
                return Err(ApiError::BadRequest(
                    "agent is DEAD (tombstoned, irreversible)".into(),
                ));
            }
            record.state = AgentState::Active;
            record.last_activity = now;
            put_record(&state.db, &record)?;
            Ok(types::ack(&format!("Agent {} reactivated", did.0)))
        }
        None => {
            let record = LifecycleRecord {
                did: did.0.clone(),
                state: AgentState::Active,
                last_activity: now,
                activated_at: Some(now),
                hibernated_at: None,
                tombstoned_at: None,
                derived_from: None,
            };
            put_record(&state.db, &record)?;
            Ok(types::ack(&format!("Agent {} activated (immunity period: 14 days)", did.0)))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/lifecycle/hibernate",
    responses(
        (status = 200, description = "Agent hibernated")
    ),
    tag = "lifecycle",
)]
pub async fn hibernate(State(state): State<Arc<ApiState>>) -> Result<impl IntoResponse, ApiError> {
    let did = state.require_did()?;
    let now = now_ts();

    let mut record = get_record(&state.db, &did.0)?.ok_or_else(|| {
        ApiError::NotFound("no lifecycle record -- POST /api/v1/lifecycle/activate first".into())
    })?;

    if record.state != AgentState::Active {
        return Err(ApiError::BadRequest(format!(
            "can only hibernate from ACTIVE state, current: {}",
            record.state
        )));
    }

    record.state = AgentState::Hibernating;
    record.hibernated_at = Some(now);
    put_record(&state.db, &record)?;

    Ok(types::ack(&format!("Agent {} hibernating (state preserved)", did.0)))
}

#[utoipa::path(
    post,
    path = "/api/v1/lifecycle/reactivate",
    responses(
        (status = 200, description = "Agent reactivated from hibernation")
    ),
    tag = "lifecycle",
)]
pub async fn reactivate(State(state): State<Arc<ApiState>>) -> Result<impl IntoResponse, ApiError> {
    let did = state.require_did()?;
    let now = now_ts();

    let mut record = get_record(&state.db, &did.0)?.ok_or_else(|| {
        ApiError::NotFound("no lifecycle record -- POST /api/v1/lifecycle/activate first".into())
    })?;

    if record.state != AgentState::Hibernating {
        return Err(ApiError::BadRequest(format!(
            "can only reactivate from HIBERNATING, current: {}",
            record.state
        )));
    }

    record.state = AgentState::Active;
    record.last_activity = now;
    record.hibernated_at = None;
    put_record(&state.db, &record)?;

    Ok(types::ack(&format!("Agent {} reactivated from hibernation", did.0)))
}

#[utoipa::path(
    get,
    path = "/api/v1/lifecycle/list",
    responses(
        (status = 200, description = "List all agent states", body = Vec<AgentSummary>)
    ),
    tag = "lifecycle",
)]
pub async fn list_states(
    State(state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    let entries = state.db.prefix_scan("identity", b"lifecycle:")?;
    if entries.is_empty() {
        let empty: Vec<AgentSummary> = vec![];
        return Ok(types::ok(empty));
    }

    let agents: Vec<AgentSummary> = entries
        .iter()
        .filter_map(|(_, value)| {
            let record: LifecycleRecord = bincode::deserialize(value).ok()?;
            Some(AgentSummary {
                did: record.did,
                state: record.state.to_string(),
                last_activity: record.last_activity,
            })
        })
        .collect();

    Ok(types::ok(agents))
}

#[utoipa::path(
    post,
    path = "/api/v1/lifecycle/reap",
    responses(
        (status = 200, description = "Reap idle/zombie agents", body = ReapResult)
    ),
    tag = "lifecycle",
)]
pub async fn reap(State(state): State<Arc<ApiState>>) -> Result<impl IntoResponse, ApiError> {
    let db = &state.db;
    let now = now_ts();

    let entries = db.prefix_scan("identity", b"lifecycle:")?;
    let mut transitions: Vec<ReapTransition> = Vec::new();

    for (_, value) in &entries {
        let mut record: LifecycleRecord = match bincode::deserialize(value) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let elapsed = now.saturating_sub(record.last_activity);
        let old_state = record.state.clone();

        match record.state {
            AgentState::Active | AgentState::Idle => {
                if elapsed >= ZOMBIE_THRESHOLD_SECS {
                    record.state = AgentState::Zombie;
                } else if elapsed >= IDLE_THRESHOLD_SECS && record.state == AgentState::Active {
                    record.state = AgentState::Idle;
                }
            }
            AgentState::Zombie => {
                if elapsed >= DEATH_THRESHOLD_SECS {
                    record.state = AgentState::Dead;
                    record.tombstoned_at = Some(now);
                }
            }
            AgentState::Hibernating => {
                if let Some(hib_at) = record.hibernated_at {
                    if now.saturating_sub(hib_at) >= 180 * 86400 {
                        record.state = AgentState::Dead;
                        record.tombstoned_at = Some(now);
                    }
                }
            }
            AgentState::Created | AgentState::Dead => {}
        }

        if record.state != old_state {
            transitions.push(ReapTransition {
                did: record.did.clone(),
                from: old_state.to_string(),
                to: record.state.to_string(),
            });
            put_record(db, &record)?;
        }
    }

    let count = transitions.len();
    Ok(types::ok(ReapResult { transitions, count }))
}
