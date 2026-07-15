#[path = "api_audit_api.rs"]
pub mod audit_api;
#[path = "api_bounty_api.rs"]
pub mod bounty_api;
#[path = "api_config_api.rs"]
pub mod config_api;
#[path = "api_discovery_api.rs"]
pub mod discovery_api;
#[path = "api_error.rs"]
pub mod error;
#[path = "api_feed_api.rs"]
pub mod feed_api;
#[path = "api_health_api.rs"]
pub mod health_api;
#[path = "api_identity_api.rs"]
pub mod identity_api;
#[path = "api_inference_api.rs"]
pub mod inference_api;
#[path = "api_knowledge_api.rs"]
pub mod knowledge_api;
#[path = "api_lifecycle_api.rs"]
pub mod lifecycle_api;
#[path = "api_lineage_api.rs"]
pub mod lineage_api;
#[path = "api_mesh_api.rs"]
pub mod mesh_api;
#[path = "api_model_api.rs"]
pub mod model_api;
#[path = "api_reputation_api.rs"]
pub mod reputation_api;
#[path = "api_routes.rs"]
pub mod routes;
#[path = "api_state.rs"]
pub mod state;
#[path = "api_token_api.rs"]
pub mod token_api;
#[path = "api_train_api.rs"]
pub mod train_api;
#[path = "api_turboquant_api.rs"]
pub mod turboquant_api;
#[path = "api_types.rs"]
pub mod types;
#[path = "api_verification_api.rs"]
pub mod verification_api;

pub use health_api::health_handler;
pub use routes::build_api_router;

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        identity_api::show_identity,
        identity_api::export_identity,
        verification_api::verify_intel_tdx,
        verification_api::verify_amd_snp,
        verification_api::verify_amd_vlek,
        inference_api::register_provider,
        lifecycle_api::lifecycle_status,
        lifecycle_api::activate,
        lifecycle_api::hibernate,
        lifecycle_api::reactivate,
        lifecycle_api::list_states,
        lifecycle_api::reap,
        lineage_api::register_lineage,
        lineage_api::show_lineage,
        lineage_api::show_parents,
        lineage_api::show_children,
        lineage_api::show_ancestors,
        lineage_api::show_depth,
        lineage_api::compute_royalties,
        lineage_api::hash_file,
        lineage_api::verify_signature,
    ),
    components(schemas(
        identity_api::IdentityQuery,
        identity_api::IdentityDetailResponse,
        identity_api::IdentityExportResponse,
        verification_api::IntelTdxVerifyRequest,
        verification_api::AmdPolicyRequest,
        verification_api::AmdTcbRequest,
        verification_api::AmdGenerationRequest,
        verification_api::AmdSnpVerifyRequest,
        verification_api::AmdVlekVerifyRequest,
        verification_api::IntelTdxVerifyResponse,
        verification_api::IntelTdxClaimsResponse,
        verification_api::AmdSnpVerifyResponse,
        verification_api::AmdVlekVerifyResponse,
        verification_api::AmdSnpClaimsResponse,
        verification_api::AmdTcbClaimsResponse,
        inference_api::RegisterProviderRequest,
        inference_api::RegisterProviderResponse,
        lifecycle_api::LifecycleStatusResponse,
        lifecycle_api::LifecycleStatusBody,
        lifecycle_api::NoRecordResponse,
        lifecycle_api::AgentSummary,
        lifecycle_api::ReapTransition,
        lifecycle_api::ReapResult,
        lineage_api::RegisterLineageRequest,
        lineage_api::RoyaltiesRequest,
        lineage_api::HashRequest,
        lineage_api::VerifyRequest,
        lineage_api::LineageDetailResponse,
        lineage_api::ModelSummary,
        lineage_api::DepthResponse,
        lineage_api::RoyaltyAllocation,
        lineage_api::HashResponse,
        lineage_api::VerifyResponse,
        lineage_api::RegisterLineageResponse,
        types::Ack,
    )),
    tags(
        (name = "lifecycle", description = "Agent lifecycle operations"),
        (name = "lineage", description = "Model provenance and royalty operations"),
    ),
    info(title = "Neunode REST API", version = "0.1.0"),
)]
pub struct ApiDoc;
