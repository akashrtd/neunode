use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;

use super::state::ApiState;

pub fn build_api_router() -> Router<Arc<ApiState>> {
    Router::new()
        // Health
        .route("/api/v1/health", get(super::health_handler))
        // Forensic audit
        .route("/api/v1/audit", get(super::audit_api::list_audit))
        .route("/api/v1/audit/verify", get(super::audit_api::verify_audit))
        // Identity
        .route("/api/v1/identity", get(super::identity_api::show_identity))
        .route("/api/v1/identity/create", post(super::identity_api::create_identity))
        .route("/api/v1/identity/list", get(super::identity_api::list_identities))
        .route("/api/v1/identity/export", get(super::identity_api::export_identity))
        .route("/api/v1/identity/register-onchain", post(super::identity_api::register_onchain))
        // Feed
        .route("/api/v1/feed", get(super::feed_api::list_feed).post(super::feed_api::post_feed))
        .route("/api/v1/feed/{event_id}", get(super::feed_api::show_feed_event))
        // Bounty
        .route(
            "/api/v1/bounties",
            get(super::bounty_api::list_bounties).post(super::bounty_api::create_bounty),
        )
        .route("/api/v1/bounties/{id}", get(super::bounty_api::show_bounty))
        .route("/api/v1/bounties/{id}/claim", post(super::bounty_api::claim_bounty))
        .route("/api/v1/bounties/{id}/submit", post(super::bounty_api::submit_bounty))
        .route("/api/v1/bounties/{id}/review", post(super::bounty_api::review_bounty))
        .route("/api/v1/bounties/{id}/pay", post(super::bounty_api::pay_bounty))
        .route("/api/v1/bounties/{id}/cancel", post(super::bounty_api::cancel_bounty))
        // Token
        .route("/api/v1/tokens/balance", get(super::token_api::token_balance))
        .route("/api/v1/tokens/transfer", post(super::token_api::transfer))
        .route("/api/v1/tokens/stake", post(super::token_api::stake))
        .route("/api/v1/tokens/unstake", post(super::token_api::unstake))
        .route("/api/v1/tokens/claim-unbonded", post(super::token_api::claim_unbonded))
        .route("/api/v1/tokens/stake-status", get(super::token_api::stake_status))
        .route("/api/v1/tokens/decay-info", get(super::token_api::decay_info))
        // Inference
        .route("/api/v1/inference/request", post(super::inference_api::request_inference))
        .route("/api/v1/inference/models", get(super::inference_api::list_models))
        .route(
            "/api/v1/inference/providers",
            get(super::inference_api::list_providers).post(super::inference_api::register_provider),
        )
        .route("/api/v1/inference/route", get(super::inference_api::show_route))
        .route("/api/v1/inference/pricing", get(super::inference_api::show_pricing))
        // Discovery
        .route("/api/v1/discovery/search", get(super::discovery_api::search_agents))
        .route("/api/v1/discovery/complement", get(super::discovery_api::complement_agents))
        .route("/api/v1/discovery/gaps", get(super::discovery_api::capability_gaps))
        .route("/api/v1/discovery/score", get(super::discovery_api::score_agent))
        .route("/api/v1/discovery/weights", get(super::discovery_api::scoring_weights))
        // Mesh
        .route("/api/v1/mesh/status", get(super::mesh_api::mesh_status))
        .route("/api/v1/mesh/peers", get(super::mesh_api::list_peers))
        .route("/api/v1/mesh/connect", post(super::mesh_api::connect_peer))
        .route("/api/v1/mesh/disconnect", post(super::mesh_api::disconnect_peer))
        // Knowledge
        .route("/api/v1/knowledge/query", get(super::knowledge_api::query_knowledge))
        .route("/api/v1/knowledge/register-agent", post(super::knowledge_api::register_agent))
        .route("/api/v1/knowledge/register-model", post(super::knowledge_api::register_model))
        .route("/api/v1/knowledge/register-bounty", post(super::knowledge_api::register_bounty))
        .route("/api/v1/knowledge/join-job", post(super::knowledge_api::join_job))
        .route("/api/v1/knowledge/classes", get(super::knowledge_api::list_classes))
        .route("/api/v1/knowledge/predicates", get(super::knowledge_api::list_predicates))
        // Reputation
        .route("/api/v1/reputation", get(super::reputation_api::show_reputation))
        .route("/api/v1/reputation/attest", post(super::reputation_api::attest_agent))
        .route("/api/v1/reputation/leaderboard", get(super::reputation_api::leaderboard))
        .route("/api/v1/reputation/factors", get(super::reputation_api::show_factors))
        // Model
        .route(
            "/api/v1/models",
            get(super::model_api::list_models).post(super::model_api::push_model),
        )
        .route(
            "/api/v1/models/{model_id}",
            get(super::model_api::show_model).delete(super::model_api::remove_model),
        )
        // Lineage
        .route("/api/v1/lineage/register", post(super::lineage_api::register_lineage))
        .route("/api/v1/lineage/{cid}", get(super::lineage_api::show_lineage))
        .route("/api/v1/lineage/{cid}/parents", get(super::lineage_api::show_parents))
        .route("/api/v1/lineage/{cid}/children", get(super::lineage_api::show_children))
        .route("/api/v1/lineage/{cid}/ancestors", get(super::lineage_api::show_ancestors))
        .route("/api/v1/lineage/{cid}/depth", get(super::lineage_api::show_depth))
        .route("/api/v1/lineage/{cid}/royalties", post(super::lineage_api::compute_royalties))
        .route("/api/v1/lineage/hash", post(super::lineage_api::hash_file))
        .route("/api/v1/lineage/verify", post(super::lineage_api::verify_signature))
        // Train
        .route("/api/v1/train/start", post(super::train_api::start_training))
        .route("/api/v1/train/status", get(super::train_api::training_status))
        .route("/api/v1/train/stop", post(super::train_api::stop_training))
        .route("/api/v1/train/jobs", get(super::train_api::list_jobs))
        .route("/api/v1/train/worker-register", post(super::train_api::register_worker))
        .route("/api/v1/train/workers", get(super::train_api::list_workers))
        .route("/api/v1/train/coordinator-status", get(super::train_api::coordinator_status))
        // TurboQuant
        .route("/api/v1/turboquant/compress", post(super::turboquant_api::select_strategy))
        .route("/api/v1/turboquant/codebook", post(super::turboquant_api::generate_codebook))
        // Config
        .route(
            "/api/v1/config",
            get(super::config_api::get_config).put(super::config_api::set_config),
        )
        .route("/api/v1/config/path", get(super::config_api::config_path))
        // Lifecycle
        .route("/api/v1/lifecycle/status", get(super::lifecycle_api::lifecycle_status))
        .route("/api/v1/lifecycle/activate", post(super::lifecycle_api::activate))
        .route("/api/v1/lifecycle/hibernate", post(super::lifecycle_api::hibernate))
        .route("/api/v1/lifecycle/reactivate", post(super::lifecycle_api::reactivate))
        .route("/api/v1/lifecycle/list", get(super::lifecycle_api::list_states))
        .route("/api/v1/lifecycle/reap", post(super::lifecycle_api::reap))
        // Production TEE verification
        .route(
            "/api/v1/verification/tee/intel-tdx",
            post(super::verification_api::verify_intel_tdx),
        )
        .route("/api/v1/verification/tee/amd-snp", post(super::verification_api::verify_amd_snp))
        .route("/api/v1/verification/tee/amd-vlek", post(super::verification_api::verify_amd_vlek))
}
