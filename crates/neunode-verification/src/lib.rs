pub mod bisection;
pub mod error;
pub mod gauntlet;
pub mod hash_util;
pub mod repops;
pub mod spot_check;
pub mod tee;
pub mod types;
#[cfg(feature = "zk")]
pub mod zk;

pub use bisection::{BisectionResult, BisectionSolver};
pub use error::{Result, VerificationError};
pub use gauntlet::{Gauntlet, GauntletConfig, GauntletTest};
pub use repops::{DeterministicExecutor, RepOpsConfig, RepOpsResult};
pub use spot_check::{SpotCheckConfig, SpotCheckResult, SpotChecker};
pub use tee::{TeeAttestation, TeeQuote, TeeVerifier};
pub use types::{
    ArtifactHash, VerificationLayer, VerificationRequest, VerificationResult, VerificationTier,
};
#[cfg(feature = "zk")]
pub use zk::{ZkProofResult, ZkProofSystem, ZkVerifier};
