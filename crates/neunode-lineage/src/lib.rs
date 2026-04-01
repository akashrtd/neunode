pub mod dag;
pub mod error;
pub mod provenance;
pub mod royalty;
pub mod sigchain;
pub mod types;

pub use dag::LineageDag;
pub use error::{LineageError, Result};
pub use provenance::compute_content_hash;
pub use royalty::{compute_royalties, RoyaltyAllocation};
pub use sigchain::{sign_model_node, verify_model_node};
pub use types::{ContributionType, LineageEdge, ModelMetadata, ModelNode, RoyaltyWeight};
