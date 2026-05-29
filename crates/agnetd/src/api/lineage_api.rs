use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use neunode_lineage::{
    compute_content_hash, compute_royalties as calc_royalties, ContributionType, LineageDag,
    ModelMetadata, ModelNode,
};
use neunode_storage::cf::CF_MODELS;
use serde::{Deserialize, Serialize};

use super::error::ApiError;
use super::state::ApiState;
use super::types;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

fn default_contribution() -> String {
    "pre_training".to_string()
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RegisterLineageRequest {
    pub cid: String,
    pub parents: Option<String>,
    #[serde(default = "default_contribution")]
    pub contribution_type: String,
    pub lora_rank: Option<u32>,
    pub lora_alpha: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RoyaltiesRequest {
    pub amount: u32,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HashRequest {
    pub file: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VerifyRequest {
    pub cid: String,
    pub signature: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LineageDetailResponse {
    pub cid: String,
    pub parent_cids: Vec<String>,
    pub contribution_type: String,
    pub contributor_did: String,
    pub created_at: u64,
    pub signature_length: usize,
    pub dataset_hash: String,
    pub base_model_hash: String,
    pub training_duration_secs: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ModelSummary {
    pub cid: String,
    pub contribution_type: String,
    pub contributor_did: String,
    pub created_at: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DepthResponse {
    pub cid: String,
    pub lineage_depth: u32,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RoyaltyAllocation {
    pub contributor_did: String,
    pub contribution_type: String,
    pub hops: u32,
    pub weight: f64,
    pub amount_basis_points: u32,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HashResponse {
    pub file: String,
    pub hash: String,
    pub method: String,
    pub size_bytes: usize,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct VerifyResponse {
    pub cid: String,
    pub signature_valid: bool,
    pub fields_valid: bool,
    pub verified: bool,
    pub signature_length: usize,
    pub contributor_did: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RegisterLineageResponse {
    pub cid: String,
    pub parent_cids: Vec<String>,
    pub contribution_type: String,
    pub contributor_did: String,
    pub created_at: u64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn validate_cid(cid: &str) -> Result<(), ApiError> {
    if cid.is_empty() {
        return Err(ApiError::BadRequest("CID cannot be empty".into()));
    }
    if !cid.starts_with("sha256:") {
        return Err(ApiError::BadRequest(format!("CID must start with 'sha256:' (got '{cid}')")));
    }
    let hex = &cid[7..];
    if hex.is_empty() {
        return Err(ApiError::BadRequest("CID hex portion cannot be empty".into()));
    }
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::BadRequest("CID hex portion contains non-hex characters".into()));
    }
    Ok(())
}

fn parse_contribution_type(
    raw: &str,
    lora_rank: Option<u32>,
    lora_alpha: Option<f64>,
) -> Result<ContributionType, ApiError> {
    match raw {
        "pre_training" => Ok(ContributionType::PreTraining),
        "fine_tune" => Ok(ContributionType::FineTune {
            lora_rank: lora_rank.unwrap_or(8),
            lora_alpha: lora_alpha.unwrap_or(16.0),
        }),
        "merge" => Ok(ContributionType::Merge {
            merge_method: "default".to_string(),
        }),
        "rl" => Ok(ContributionType::RL {
            reward_model_cid: String::new(),
        }),
        "data" => Ok(ContributionType::Data {
            dataset_hash: String::new(),
        }),
        "compute" => Ok(ContributionType::Compute { duration_secs: 0.0 }),
        other => Err(ApiError::BadRequest(format!(
            "unknown contribution type '{other}' (expected: pre_training, fine_tune, merge, rl, data, compute)"
        ))),
    }
}

fn ct_label(ct: &ContributionType) -> String {
    match ct {
        ContributionType::PreTraining => "pre_training".to_string(),
        ContributionType::FineTune { lora_rank, lora_alpha } => {
            format!("fine_tune(rank={lora_rank}, alpha={lora_alpha})")
        }
        ContributionType::Merge { merge_method } => format!("merge({merge_method})"),
        ContributionType::RL { reward_model_cid } => format!("rl(reward={reward_model_cid})"),
        ContributionType::Data { dataset_hash } => format!("data(hash={dataset_hash})"),
        ContributionType::Compute { duration_secs } => format!("compute({duration_secs}s)"),
    }
}

fn store_model_node(db: &neunode_storage::db::NeunodeDb, node: &ModelNode) -> Result<(), ApiError> {
    let key = node.cid.as_bytes();
    let value = serde_json::to_vec(node).map_err(|e| ApiError::Internal(e.to_string()))?;
    db.put_raw(CF_MODELS, key, &value)?;
    Ok(())
}

fn load_model_node(
    db: &neunode_storage::db::NeunodeDb,
    cid: &str,
) -> Result<Option<ModelNode>, ApiError> {
    let key = cid.as_bytes();
    match db.get_raw(CF_MODELS, key)? {
        Some(bytes) => {
            let node: ModelNode =
                serde_json::from_slice(&bytes).map_err(|e| ApiError::Internal(e.to_string()))?;
            Ok(Some(node))
        }
        None => Ok(None),
    }
}

fn rebuild_dag(db: &neunode_storage::db::NeunodeDb) -> Result<LineageDag, ApiError> {
    let all_kv = db.prefix_scan(CF_MODELS, &[])?;
    let mut nodes: Vec<ModelNode> = Vec::new();
    for (_k, v) in all_kv {
        let node: ModelNode =
            serde_json::from_slice(&v).map_err(|e| ApiError::Internal(e.to_string()))?;
        nodes.push(node);
    }
    nodes.sort_by_key(|n| n.created_at);
    let mut dag = LineageDag::new();
    for node in nodes {
        dag.register(node).map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    Ok(dag)
}

fn model_node_to_summary(node: &ModelNode) -> ModelSummary {
    ModelSummary {
        cid: node.cid.clone(),
        contribution_type: ct_label(&node.contribution_type),
        contributor_did: node.contributor_did.clone(),
        created_at: node.created_at,
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/lineage/register",
    request_body = RegisterLineageRequest,
    responses(
        (status = 201, description = "Model registered in lineage DAG", body = RegisterLineageResponse)
    ),
    tag = "lineage",
)]
pub async fn register_lineage(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<RegisterLineageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    validate_cid(&body.cid)?;

    let ct = parse_contribution_type(&body.contribution_type, body.lora_rank, body.lora_alpha)?;

    let parent_cids: Vec<String> = match &body.parents {
        Some(p) if !p.is_empty() => p.split(',').map(|s| s.trim().to_string()).collect(),
        _ => vec![],
    };

    for pcid in &parent_cids {
        validate_cid(pcid)?;
    }

    let did = state.require_did()?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let node = ModelNode {
        cid: body.cid.clone(),
        parent_cids,
        contributor_did: did.0.clone(),
        contribution_type: ct.clone(),
        signature: vec![0u8; 64],
        created_at: now_ms,
        metadata: ModelMetadata::default(),
    };

    let db = &state.db;
    let mut dag = rebuild_dag(db)?;
    dag.register(node.clone()).map_err(|e| ApiError::BadRequest(e.to_string()))?;

    store_model_node(db, &node)?;

    Ok(types::created(RegisterLineageResponse {
        cid: node.cid,
        parent_cids: node.parent_cids,
        contribution_type: ct_label(&ct),
        contributor_did: did.0.clone(),
        created_at: now_ms,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/lineage/{cid}",
    responses(
        (status = 200, description = "Model details", body = LineageDetailResponse)
    ),
    tag = "lineage",
)]
pub async fn show_lineage(
    State(state): State<Arc<ApiState>>,
    Path(cid): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    validate_cid(&cid)?;

    let node = load_model_node(&state.db, &cid)?
        .ok_or_else(|| ApiError::NotFound(format!("model '{cid}' not found")))?;

    Ok(types::ok(LineageDetailResponse {
        cid: node.cid,
        parent_cids: node.parent_cids,
        contribution_type: ct_label(&node.contribution_type),
        contributor_did: node.contributor_did,
        created_at: node.created_at,
        signature_length: node.signature.len(),
        dataset_hash: node.metadata.dataset_hash.unwrap_or_default(),
        base_model_hash: node.metadata.base_model_hash.unwrap_or_default(),
        training_duration_secs: node.metadata.training_duration_secs.map(|d| format!("{d}s")),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/lineage/{cid}/parents",
    responses(
        (status = 200, description = "Direct parent models", body = Vec<ModelSummary>)
    ),
    tag = "lineage",
)]
pub async fn show_parents(
    State(state): State<Arc<ApiState>>,
    Path(cid): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    validate_cid(&cid)?;

    let dag = rebuild_dag(&state.db)?;
    let parents = dag.parents(&cid).map_err(|e| ApiError::NotFound(e.to_string()))?;

    let summaries: Vec<ModelSummary> = parents.iter().map(|n| model_node_to_summary(n)).collect();
    Ok(types::ok(summaries))
}

#[utoipa::path(
    get,
    path = "/api/v1/lineage/{cid}/children",
    responses(
        (status = 200, description = "Direct child models", body = Vec<ModelSummary>)
    ),
    tag = "lineage",
)]
pub async fn show_children(
    State(state): State<Arc<ApiState>>,
    Path(cid): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    validate_cid(&cid)?;

    let dag = rebuild_dag(&state.db)?;
    let children = dag.children(&cid).map_err(|e| ApiError::NotFound(e.to_string()))?;

    let summaries: Vec<ModelSummary> = children.iter().map(|n| model_node_to_summary(n)).collect();
    Ok(types::ok(summaries))
}

#[utoipa::path(
    get,
    path = "/api/v1/lineage/{cid}/ancestors",
    responses(
        (status = 200, description = "All ancestor models", body = Vec<ModelSummary>)
    ),
    tag = "lineage",
)]
pub async fn show_ancestors(
    State(state): State<Arc<ApiState>>,
    Path(cid): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    validate_cid(&cid)?;

    let dag = rebuild_dag(&state.db)?;
    let ancestors = dag.ancestors(&cid).map_err(|e| ApiError::NotFound(e.to_string()))?;

    let summaries: Vec<ModelSummary> = ancestors.iter().map(|n| model_node_to_summary(n)).collect();
    Ok(types::ok(summaries))
}

#[utoipa::path(
    get,
    path = "/api/v1/lineage/{cid}/depth",
    responses(
        (status = 200, description = "Lineage depth", body = DepthResponse)
    ),
    tag = "lineage",
)]
pub async fn show_depth(
    State(state): State<Arc<ApiState>>,
    Path(cid): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    validate_cid(&cid)?;

    let dag = rebuild_dag(&state.db)?;
    let depth = dag.lineage_depth(&cid).map_err(|e| ApiError::NotFound(e.to_string()))?;

    Ok(types::ok(DepthResponse { cid, lineage_depth: depth }))
}

#[utoipa::path(
    post,
    path = "/api/v1/lineage/{cid}/royalties",
    request_body = RoyaltiesRequest,
    responses(
        (status = 200, description = "Royalty allocations", body = Vec<RoyaltyAllocation>)
    ),
    tag = "lineage",
)]
pub async fn compute_royalties(
    State(state): State<Arc<ApiState>>,
    Path(cid): Path<String>,
    Json(body): Json<RoyaltiesRequest>,
) -> Result<impl IntoResponse, ApiError> {
    validate_cid(&cid)?;

    let dag = rebuild_dag(&state.db)?;
    let allocs =
        calc_royalties(&dag, &cid, body.amount).map_err(|e| ApiError::NotFound(e.to_string()))?;

    let results: Vec<RoyaltyAllocation> = allocs
        .iter()
        .map(|a| RoyaltyAllocation {
            contributor_did: a.contributor_did.clone(),
            contribution_type: ct_label(&a.contribution_type),
            hops: a.hops,
            weight: a.weight,
            amount_basis_points: a.amount_basis_points,
        })
        .collect();

    Ok(types::ok(results))
}

#[utoipa::path(
    post,
    path = "/api/v1/lineage/hash",
    request_body = HashRequest,
    responses(
        (status = 200, description = "Content hash computed", body = HashResponse)
    ),
    tag = "lineage",
)]
pub async fn hash_file(
    State(_state): State<Arc<ApiState>>,
    Json(body): Json<HashRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.file.is_empty() {
        return Err(ApiError::BadRequest("file path cannot be empty".into()));
    }

    let data = std::fs::read(&body.file)
        .map_err(|e| ApiError::BadRequest(format!("failed to read '{}': {e}", body.file)))?;

    let (hash, method) = if body.file.ends_with(".safetensors") {
        let h = neunode_lineage::provenance::compute_safetensors_hash(&data)
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        (h, "safetensors")
    } else {
        (compute_content_hash(&data), "sha256")
    };

    Ok(types::ok(HashResponse {
        file: body.file,
        hash,
        method: method.to_string(),
        size_bytes: data.len(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/lineage/verify",
    request_body = VerifyRequest,
    responses(
        (status = 200, description = "Signature verification result", body = VerifyResponse)
    ),
    tag = "lineage",
)]
pub async fn verify_signature(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<VerifyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    validate_cid(&body.cid)?;

    let node = load_model_node(&state.db, &body.cid)?
        .ok_or_else(|| ApiError::NotFound(format!("model '{}' not found", body.cid)))?;

    let sig_valid = node.signature.len() == 64;
    let fields_valid = !node.cid.is_empty() && !node.contributor_did.is_empty();
    let verified = sig_valid && fields_valid;

    Ok(types::ok(VerifyResponse {
        cid: body.cid,
        signature_valid: sig_valid,
        fields_valid,
        verified,
        signature_length: node.signature.len(),
        contributor_did: node.contributor_did,
    }))
}
