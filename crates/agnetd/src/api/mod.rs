pub mod audit_api;
pub mod bounty_api;
pub mod config_api;
pub mod discovery_api;
pub mod error;
pub mod feed_api;
pub mod health_api;
pub mod identity_api;
pub mod inference_api;
pub mod knowledge_api;
pub mod lifecycle_api;
pub mod lineage_api;
pub mod mesh_api;
pub mod model_api;
pub mod reputation_api;
pub mod routes;
pub mod state;
pub mod token_api;
pub mod train_api;
pub mod turboquant_api;
pub mod types;

pub use health_api::health_handler;
pub use routes::build_api_router;

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
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
