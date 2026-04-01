pub mod complement;
pub mod error;
pub mod gap;
pub mod scoring;
pub mod search;
pub mod types;

pub use complement::{find_complementary, jaccard_distance};
pub use error::{DiscoveryError, Result};
pub use gap::find_capability_gaps;
pub use scoring::compute_score;
pub use search::search;
pub use types::{
    AgentCandidate, CapabilityGap, DiscoveryRequest, NormalizedFactor, ScoredAgent, ScoringWeights,
};
