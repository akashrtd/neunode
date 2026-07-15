pub mod bisection;
pub mod error;
pub mod gauntlet;
pub mod hash_util;
pub mod repops;
pub mod spot_check;
pub mod tee;
#[cfg(feature = "tee-amd")]
pub mod tee_amd;
#[cfg(feature = "tee-intel")]
pub mod tee_intel;
pub mod types;
#[cfg(feature = "zk")]
pub mod zk;

pub use bisection::{BisectionResult, BisectionSolver};
pub use error::{Result, VerificationError};
pub use gauntlet::{Gauntlet, GauntletConfig, GauntletTest};
pub use repops::{DeterministicExecutor, RepOpsConfig, RepOpsResult};
pub use spot_check::{SpotCheckConfig, SpotCheckResult, SpotChecker};
pub use tee::{TeeAttestation, TeeQuote, TeeVerifier};
#[cfg(feature = "tee-amd")]
pub use tee_amd::{AmdGeneration, AmdSnpClaims, AmdSnpPolicy, AmdSnpVerifier, AmdTcb};
#[cfg(feature = "tee-intel")]
pub use tee_intel::{IntelTdxClaims, IntelTdxPolicy, IntelTdxVerifier};
pub use types::{
    ArtifactHash, VerificationLayer, VerificationRequest, VerificationResult, VerificationTier,
};
#[cfg(feature = "zk")]
pub use zk::{ZkProofResult, ZkProofSystem, ZkVerifier};
