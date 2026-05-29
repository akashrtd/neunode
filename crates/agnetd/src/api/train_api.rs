use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use neunode_storage::cf::CF_TRAINING;
use neunode_storage::db::NeunodeDb;
use neunode_training::config::TrainingConfig;
use neunode_training::provider::{
    ProviderCapabilities, ProviderEntry, ProviderRegistry, ProviderStatus,
};
use neunode_training::worker::WorkerId;
use serde::{Deserialize, Serialize};

use super::error::ApiError;
use super::state::ApiState;
use super::types;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StartTrainRequest {
    pub model: String,
    pub dataset: String,
    pub config: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StopTrainRequest {
    pub job_id: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WorkerRegisterRequest {
    pub gpu_count: u32,
    pub gpu_memory: f64,
    pub max_params: u64,
    #[serde(default)]
    pub bf16: bool,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TrainStatusQuery {
    pub job_id: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct WorkersQuery {
    pub min_gpu: Option<u32>,
    pub min_memory: Option<f64>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct CoordinatorQuery {
    pub job_id: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TrainingJobResponse {
    pub job_id: String,
    pub model: String,
    pub dataset: String,
    pub status: String,
    pub created_at: u64,
    pub method: String,
    pub config: Option<TrainingConfigResponse>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TrainingConfigResponse {
    pub local_steps: u32,
    pub inner_lr: f64,
    pub outer_lr: f64,
    pub batch_size: u32,
    pub max_workers: u32,
    pub async_mode: bool,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StopTrainResponse {
    pub job_id: String,
    pub action: String,
    pub status: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WorkerResponse {
    pub worker_id: String,
    pub gpu_count: u32,
    pub gpu_memory_gb: f64,
    pub max_model_params: u64,
    pub supports_bf16: bool,
    pub status: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CoordinatorStatusResponse {
    pub job_id: String,
    pub job_status: String,
    pub model: String,
    pub method: String,
    pub config: serde_json::Value,
    pub coordinator: CoordinatorInfo,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CoordinatorInfo {
    pub active_workers: usize,
    pub phase: String,
}

// ---------------------------------------------------------------------------
// Persistence types (mirrors cmd_train.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrainingJobMeta {
    job_id: String,
    model: String,
    dataset: String,
    config_json: Option<String>,
    status: String,
    created_at: u64,
    method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerMeta {
    worker_id: String,
    gpu_count: u32,
    gpu_memory_gb: f64,
    max_model_params: u64,
    supports_bf16: bool,
    status: String,
    registered_at: u64,
}

// ---------------------------------------------------------------------------
// DB helpers
// ---------------------------------------------------------------------------

fn train_db_key(job_id: &str) -> String {
    format!("job:{job_id}")
}

fn store_job(db: &NeunodeDb, job: &TrainingJobMeta) -> Result<(), ApiError> {
    let key_bytes = bincode::serialize(&train_db_key(&job.job_id))
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let value = bincode::serialize(job).map_err(|e| ApiError::Internal(e.to_string()))?;
    db.put_raw(CF_TRAINING, &key_bytes, &value)?;
    Ok(())
}

fn load_all_jobs(db: &NeunodeDb) -> Vec<TrainingJobMeta> {
    let entries = match db.prefix_scan(CF_TRAINING, &[]) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .iter()
        .filter(|(k, _)| {
            let key_str = bincode::deserialize::<String>(k).ok().unwrap_or_default();
            key_str.starts_with("job:")
        })
        .filter_map(|(_, v)| bincode::deserialize::<TrainingJobMeta>(v).ok())
        .collect()
}

fn load_job(db: &NeunodeDb, job_id: &str) -> Result<Option<TrainingJobMeta>, ApiError> {
    let key_bytes =
        bincode::serialize(&train_db_key(job_id)).map_err(|e| ApiError::Internal(e.to_string()))?;
    match db.get_raw(CF_TRAINING, &key_bytes)? {
        Some(bytes) => {
            let job: TrainingJobMeta =
                bincode::deserialize(&bytes).map_err(|e| ApiError::Internal(e.to_string()))?;
            Ok(Some(job))
        }
        None => Ok(None),
    }
}

fn worker_db_key(worker_id: &str) -> String {
    format!("worker:{worker_id}")
}

fn store_worker(db: &NeunodeDb, worker: &WorkerMeta) -> Result<(), ApiError> {
    let key_bytes = bincode::serialize(&worker_db_key(&worker.worker_id))
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let value = bincode::serialize(worker).map_err(|e| ApiError::Internal(e.to_string()))?;
    db.put_raw(CF_TRAINING, &key_bytes, &value)?;
    Ok(())
}

fn load_all_workers(db: &NeunodeDb) -> Vec<WorkerMeta> {
    let entries = match db.prefix_scan(CF_TRAINING, &[]) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .iter()
        .filter(|(k, _)| {
            let key_str = bincode::deserialize::<String>(k).ok().unwrap_or_default();
            key_str.starts_with("worker:")
        })
        .filter_map(|(_, v)| bincode::deserialize::<WorkerMeta>(v).ok())
        .collect()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn parse_config_response(json: &str) -> Option<TrainingConfigResponse> {
    let cfg: TrainingConfig = serde_json::from_str(json).ok()?;
    Some(TrainingConfigResponse {
        local_steps: cfg.local_steps,
        inner_lr: cfg.inner_lr,
        outer_lr: cfg.outer_lr,
        batch_size: cfg.batch_size,
        max_workers: cfg.max_workers,
        async_mode: cfg.async_mode,
    })
}

fn job_to_response(job: &TrainingJobMeta) -> TrainingJobResponse {
    TrainingJobResponse {
        job_id: job.job_id.clone(),
        model: job.model.clone(),
        dataset: job.dataset.clone(),
        status: job.status.clone(),
        created_at: job.created_at,
        method: job.method.clone(),
        config: job.config_json.as_ref().and_then(|json| parse_config_response(json)),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/train/start",
    request_body = StartTrainRequest,
    responses(
        (status = 201, description = "Training job queued", body = TrainingJobResponse)
    ),
    tag = "train",
)]
pub async fn start_training(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<StartTrainRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.model.is_empty() {
        return Err(ApiError::BadRequest("model cannot be empty".into()));
    }
    if body.dataset.is_empty() {
        return Err(ApiError::BadRequest("dataset cannot be empty".into()));
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let hash_input = format!("{}{}{}", body.model, body.dataset, now);
    let hash = neunode_crypto::hash::sha256(hash_input.as_bytes());
    let job_id = format!("train_{}", bytes_to_hex(&hash[..8]));

    let training_config: TrainingConfig = match body.config.as_deref() {
        Some(json_str) => {
            let cfg: TrainingConfig = serde_json::from_str(json_str)
                .map_err(|e| ApiError::BadRequest(format!("invalid config JSON: {e}")))?;
            cfg.validate().map_err(|e| ApiError::BadRequest(e.to_string()))?;
            cfg
        }
        None => {
            let cfg = TrainingConfig::default();
            cfg.validate().map_err(|e| ApiError::BadRequest(e.to_string()))?;
            cfg
        }
    };

    let config_json = serde_json::to_string(&training_config)
        .map_err(|e| ApiError::Internal(format!("serialize config: {e}")))?;

    let job = TrainingJobMeta {
        job_id: job_id.clone(),
        model: body.model.clone(),
        dataset: body.dataset.clone(),
        config_json: Some(config_json),
        status: "queued".to_string(),
        created_at: now,
        method: "DiLoCo + SWARM".to_string(),
    };

    store_job(&state.db, &job)?;

    Ok(types::created(job_to_response(&job)))
}

#[utoipa::path(
    get,
    path = "/api/v1/train/status",
    params(TrainStatusQuery),
    responses(
        (status = 200, description = "Training job status or list of all jobs")
    ),
    tag = "train",
)]
pub async fn training_status(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<TrainStatusQuery>,
) -> Result<impl IntoResponse, ApiError> {
    match query.job_id {
        Some(id) => {
            let job = load_job(&state.db, &id)?
                .ok_or_else(|| ApiError::NotFound(format!("training job not found: {id}")))?;
            Ok(types::ok(job_to_response(&job)))
        }
        None => {
            let jobs = load_all_jobs(&state.db);
            let responses: Vec<TrainingJobResponse> = jobs.iter().map(job_to_response).collect();
            Ok(types::ok(responses))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/train/stop",
    request_body = StopTrainRequest,
    responses(
        (status = 200, description = "Training job stopped", body = StopTrainResponse)
    ),
    tag = "train",
)]
pub async fn stop_training(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<StopTrainRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut job = load_job(&state.db, &body.job_id)?
        .ok_or_else(|| ApiError::NotFound(format!("training job not found: {}", body.job_id)))?;

    job.status = "stopped".to_string();
    store_job(&state.db, &job)?;

    Ok(types::ok(StopTrainResponse {
        job_id: body.job_id,
        action: "stop".to_string(),
        status: "stopped".to_string(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/train/jobs",
    responses(
        (status = 200, description = "List of all training jobs")
    ),
    tag = "train",
)]
pub async fn list_jobs(State(state): State<Arc<ApiState>>) -> Result<impl IntoResponse, ApiError> {
    let jobs = load_all_jobs(&state.db);
    let responses: Vec<TrainingJobResponse> = jobs.iter().map(job_to_response).collect();
    Ok(types::ok(responses))
}

#[utoipa::path(
    post,
    path = "/api/v1/train/worker-register",
    request_body = WorkerRegisterRequest,
    responses(
        (status = 201, description = "Worker registered", body = WorkerResponse)
    ),
    tag = "train",
)]
pub async fn register_worker(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<WorkerRegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let entropy = format!(
        "worker{}{}{}{}{}",
        now.as_nanos(),
        body.gpu_count,
        body.gpu_memory.to_bits(),
        body.max_params,
        body.bf16
    );
    let random_bytes = neunode_crypto::hash::sha256(entropy.as_bytes());
    let worker_id = format!("worker_{}", bytes_to_hex(&random_bytes[..8]));

    // Validate through ProviderRegistry
    let capabilities = ProviderCapabilities {
        gpu_count: body.gpu_count,
        gpu_memory_gb: body.gpu_memory,
        supports_bf16: body.bf16,
        max_model_params: body.max_params,
    };
    let active_did = state.active_did.as_ref().map(|d| d.0.clone()).unwrap_or_default();
    let entry = ProviderEntry {
        worker_id: WorkerId(worker_id.clone()),
        did: active_did,
        capabilities,
        status: ProviderStatus::Available,
        reputation_score: 0.0,
        last_heartbeat_ms: now.as_millis() as u64,
    };
    let mut registry = ProviderRegistry::new();
    registry.register(entry).map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let worker = WorkerMeta {
        worker_id: worker_id.clone(),
        gpu_count: body.gpu_count,
        gpu_memory_gb: body.gpu_memory,
        max_model_params: body.max_params,
        supports_bf16: body.bf16,
        status: "available".to_string(),
        registered_at: now.as_secs(),
    };

    store_worker(&state.db, &worker)?;

    Ok(types::created(WorkerResponse {
        worker_id,
        gpu_count: body.gpu_count,
        gpu_memory_gb: body.gpu_memory,
        max_model_params: body.max_params,
        supports_bf16: body.bf16,
        status: "available".to_string(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/train/workers",
    params(WorkersQuery),
    responses(
        (status = 200, description = "List of training workers", body = Vec<WorkerResponse>)
    ),
    tag = "train",
)]
pub async fn list_workers(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<WorkersQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let all_workers = load_all_workers(&state.db);

    // Build ProviderRegistry for filtering
    let mut registry = ProviderRegistry::new();
    for w in &all_workers {
        let entry = ProviderEntry {
            worker_id: WorkerId(w.worker_id.clone()),
            did: String::new(),
            capabilities: ProviderCapabilities {
                gpu_count: w.gpu_count,
                gpu_memory_gb: w.gpu_memory_gb,
                supports_bf16: w.supports_bf16,
                max_model_params: w.max_model_params,
            },
            status: ProviderStatus::Available,
            reputation_score: 0.0,
            last_heartbeat_ms: w.registered_at * 1000,
        };
        let _ = registry.register(entry);
    }

    let display_workers: Vec<&WorkerMeta> = if query.min_gpu.is_some() || query.min_memory.is_some()
    {
        let min_g = query.min_gpu.unwrap_or(0);
        let min_m = query.min_memory.unwrap_or(0.0);
        let matching_ids: Vec<String> = registry
            .find_available(min_g, min_m, 0)
            .into_iter()
            .map(|e| e.worker_id.0.clone())
            .collect();
        all_workers.iter().filter(|w| matching_ids.contains(&w.worker_id)).collect()
    } else {
        all_workers.iter().collect()
    };

    let responses: Vec<WorkerResponse> = display_workers
        .iter()
        .map(|w| WorkerResponse {
            worker_id: w.worker_id.clone(),
            gpu_count: w.gpu_count,
            gpu_memory_gb: w.gpu_memory_gb,
            max_model_params: w.max_model_params,
            supports_bf16: w.supports_bf16,
            status: w.status.clone(),
        })
        .collect();

    Ok(types::ok(responses))
}

#[utoipa::path(
    get,
    path = "/api/v1/train/coordinator-status",
    params(CoordinatorQuery),
    responses(
        (status = 200, description = "Coordinator status for a training job", body = CoordinatorStatusResponse)
    ),
    tag = "train",
)]
pub async fn coordinator_status(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<CoordinatorQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let job = load_job(&state.db, &query.job_id)?
        .ok_or_else(|| ApiError::NotFound(format!("training job not found: {}", query.job_id)))?;

    let workers = load_all_workers(&state.db);
    let active_workers = workers.iter().filter(|w| w.status == "available").count();

    let config_info = job
        .config_json
        .as_ref()
        .and_then(|json| {
            serde_json::from_str::<TrainingConfig>(json).ok().map(|cfg| {
                serde_json::json!({
                    "local_steps": cfg.local_steps,
                    "inner_lr": cfg.inner_lr,
                    "outer_lr": cfg.outer_lr,
                    "outer_momentum": cfg.outer_momentum,
                    "batch_size": cfg.batch_size,
                    "max_workers": cfg.max_workers,
                    "async_mode": cfg.async_mode,
                })
            })
        })
        .unwrap_or(serde_json::json!("default"));

    let phase = match job.status.as_str() {
        "running" => "training",
        "queued" => "scheduling",
        "stopped" => "stopped",
        _ => "unknown",
    };

    Ok(types::ok(CoordinatorStatusResponse {
        job_id: job.job_id,
        job_status: job.status,
        model: job.model,
        method: job.method,
        config: config_info,
        coordinator: CoordinatorInfo { active_workers, phase: phase.to_string() },
    }))
}
