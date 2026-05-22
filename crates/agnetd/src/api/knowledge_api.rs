use std::sync::Arc;

use axum::extract::{Query, State};
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

fn default_limit() -> usize {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct KnowledgeQuery {
    pub subject: Option<String>,
    pub predicate: Option<String>,
    pub object: Option<String>,
    pub graph: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterAgentRequest {
    pub did: String,
    pub capabilities: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterModelRequest {
    pub did: String,
    pub cid: String,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterBountyRequest {
    pub id: String,
    pub capabilities: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct KnowledgeQueryResult {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub graph: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentRegistrationResponse {
    pub did: String,
    pub capabilities: Vec<String>,
    pub triples_inserted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelRegistrationResponse {
    pub owner: String,
    pub cid: String,
    pub parent: Option<String>,
    pub triples_inserted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BountyRegistrationResponse {
    pub id: String,
    pub required_capabilities: Vec<String>,
    pub triples_inserted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OntologyEntry {
    pub name: String,
    pub uri: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/knowledge/query",
    params(
        ("subject" = Option<String>, Query, description = "Subject URI filter"),
        ("predicate" = Option<String>, Query, description = "Predicate URI filter"),
        ("object" = Option<String>, Query, description = "Object URI filter"),
        ("graph" = Option<String>, Query, description = "Named graph filter"),
        ("limit" = Option<usize>, Query, description = "Max results (default 20)"),
    ),
    responses(
        (status = 200, description = "Query results", body = Vec<KnowledgeQueryResult>),
        (status = 400, description = "Bad request"),
    ),
    tag = "knowledge",
)]
pub async fn query_knowledge(
    State(state): State<Arc<ApiState>>,
    Query(params): Query<KnowledgeQuery>,
) -> Result<impl IntoResponse, ApiError> {
    if params.subject.is_none()
        && params.predicate.is_none()
        && params.object.is_none()
        && params.graph.is_none()
    {
        return Ok(types::ok(Vec::<KnowledgeQueryResult>::new()));
    }

    let db = &state.db;
    let dict = neunode_knowledge::StringDictionary::new(db);
    let engine = neunode_knowledge::QueryEngine::new(db, &dict);

    let pattern = build_query_pattern(
        &dict,
        params.subject.as_deref(),
        params.predicate.as_deref(),
        params.object.as_deref(),
        params.graph.as_deref(),
    )?;

    let results = engine.query(&pattern)?;
    let limited: Vec<KnowledgeQueryResult> = results
        .into_iter()
        .take(params.limit)
        .map(|r| KnowledgeQueryResult {
            subject: r.subject,
            predicate: r.predicate,
            object: r.object,
            graph: r.graph,
        })
        .collect();

    Ok(types::ok(limited))
}

#[utoipa::path(
    post,
    path = "/api/v1/knowledge/register-agent",
    request_body = RegisterAgentRequest,
    responses(
        (status = 201, description = "Agent registered", body = AgentRegistrationResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "No active identity"),
    ),
    tag = "knowledge",
)]
pub async fn register_agent(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<RegisterAgentRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.did.is_empty() {
        return Err(ApiError::BadRequest("agent DID cannot be empty".into()));
    }

    let keyring = state.require_keyring()?;
    let caps = parse_capabilities(&body.capabilities);
    let cap_refs: Vec<&str> = caps.iter().map(|s| s.as_str()).collect();

    let payload =
        neunode_knowledge::authorization::canonical_register_agent(&body.did, &cap_refs);
    let auth = sign_mutation(&*keyring, &body.did, &payload)?;

    let db = &state.db;
    let dict = neunode_knowledge::StringDictionary::new(db);
    let batch = neunode_knowledge::register_agent(&dict, &body.did, &cap_refs)?;
    neunode_knowledge::apply_authorized(&batch, db, &auth, &payload)?;

    Ok(types::created(AgentRegistrationResponse {
        did: body.did,
        capabilities: caps,
        triples_inserted: batch.len(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/knowledge/register-model",
    request_body = RegisterModelRequest,
    responses(
        (status = 201, description = "Model registered", body = ModelRegistrationResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "No active identity"),
    ),
    tag = "knowledge",
)]
pub async fn register_model(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<RegisterModelRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.did.is_empty() {
        return Err(ApiError::BadRequest("owner DID cannot be empty".into()));
    }
    if body.cid.is_empty() {
        return Err(ApiError::BadRequest("model CID cannot be empty".into()));
    }

    let keyring = state.require_keyring()?;
    let payload = neunode_knowledge::authorization::canonical_register_model(
        &body.did,
        &body.cid,
        body.parent.as_deref(),
    );
    let auth = sign_mutation(&*keyring, &body.did, &payload)?;

    let db = &state.db;
    let dict = neunode_knowledge::StringDictionary::new(db);
    let batch =
        neunode_knowledge::register_model(&dict, &body.did, &body.cid, body.parent.as_deref())?;
    neunode_knowledge::apply_authorized(&batch, db, &auth, &payload)?;

    Ok(types::created(ModelRegistrationResponse {
        owner: body.did,
        cid: body.cid,
        parent: body.parent,
        triples_inserted: batch.len(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/knowledge/register-bounty",
    request_body = RegisterBountyRequest,
    responses(
        (status = 201, description = "Bounty registered", body = BountyRegistrationResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "No active identity"),
    ),
    tag = "knowledge",
)]
pub async fn register_bounty(
    State(state): State<Arc<ApiState>>,
    Json(body): Json<RegisterBountyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.id.is_empty() {
        return Err(ApiError::BadRequest("bounty ID cannot be empty".into()));
    }

    let keyring = state.require_keyring()?;
    let active_did = state.require_did()?;
    let caps = parse_capabilities(&body.capabilities);
    let cap_refs: Vec<&str> = caps.iter().map(|s| s.as_str()).collect();

    let payload =
        neunode_knowledge::authorization::canonical_register_bounty(&body.id, &cap_refs);
    let auth = sign_mutation(&*keyring, &active_did.0, &payload)?;

    let db = &state.db;
    let dict = neunode_knowledge::StringDictionary::new(db);
    let batch = neunode_knowledge::register_bounty(&dict, &body.id, &cap_refs)?;
    neunode_knowledge::apply_authorized(&batch, db, &auth, &payload)?;

    Ok(types::created(BountyRegistrationResponse {
        id: body.id,
        required_capabilities: caps,
        triples_inserted: batch.len(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/knowledge/classes",
    responses(
        (status = 200, description = "Ontology classes", body = Vec<OntologyEntry>),
    ),
    tag = "knowledge",
)]
pub async fn list_classes(
    State(_state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    let classes = neunode_knowledge::all_classes();
    let entries: Vec<OntologyEntry> = classes
        .iter()
        .map(|c| OntologyEntry { name: c.to_string(), uri: neunode_knowledge::nn(c) })
        .collect();
    Ok(types::ok(entries))
}

#[utoipa::path(
    get,
    path = "/api/v1/knowledge/predicates",
    responses(
        (status = 200, description = "Ontology predicates", body = Vec<OntologyEntry>),
    ),
    tag = "knowledge",
)]
pub async fn list_predicates(
    State(_state): State<Arc<ApiState>>,
) -> Result<impl IntoResponse, ApiError> {
    let predicates = neunode_knowledge::all_predicates();
    let entries: Vec<OntologyEntry> = predicates
        .iter()
        .map(|p| OntologyEntry { name: p.to_string(), uri: neunode_knowledge::nn(p) })
        .collect();
    Ok(types::ok(entries))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_capabilities(input: &str) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }
    input.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

fn build_query_pattern(
    dict: &neunode_knowledge::StringDictionary,
    subject: Option<&str>,
    predicate: Option<&str>,
    object: Option<&str>,
    graph: Option<&str>,
) -> Result<neunode_knowledge::QueryPattern, ApiError> {
    let mut strings = Vec::new();
    if let Some(s) = subject {
        strings.push(s.to_string());
    }
    if let Some(p) = predicate {
        strings.push(p.to_string());
    }
    if let Some(o) = object {
        strings.push(o.to_string());
    }
    if let Some(g) = graph {
        strings.push(g.to_string());
    }

    let refs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
    let hashes = dict
        .batch_insert(&refs)
        .map_err(|e| ApiError::Internal(format!("dictionary batch insert: {e}")))?;

    let mut idx = 0usize;
    let sub_hash = subject.map(|_| {
        let h = hashes[idx];
        idx += 1;
        h
    });
    let pred_hash = predicate.map(|_| {
        let h = hashes[idx];
        idx += 1;
        h
    });
    let obj_hash = object.map(|_| {
        let h = hashes[idx];
        idx += 1;
        h
    });
    let graph_hash = graph.map(|_| {
        let h = hashes[idx];
        idx += 1;
        h
    });

    Ok(neunode_knowledge::QueryPattern {
        subject: sub_hash,
        predicate: pred_hash,
        object: obj_hash,
        graph: graph_hash,
    })
}

fn sign_mutation(
    keyring: &neunode_identity::keyring::Keyring,
    signer_did: &str,
    payload: &[u8],
) -> Result<neunode_knowledge::MutationAuthorization, ApiError> {
    let (ed_bytes, _) = keyring.to_bytes();
    let mut ed_signing = [0u8; 32];
    ed_signing.copy_from_slice(&ed_bytes[..32]);
    let vk_bytes = neunode_crypto::ed25519::verifying_key_to_bytes(&keyring.ed25519_public_key());
    neunode_knowledge::MutationAuthorization::sign(
        signer_did.to_string(),
        vk_bytes,
        &ed_signing,
        payload,
    )
    .map_err(|e| ApiError::Internal(format!("failed to sign mutation: {e}")))
}
