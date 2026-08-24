//! # neunode-consensus-bridge
//!
//! Bridge between the Neunode consensus layer and a Reth execution layer via the Engine API.
//!
//! ## Phase 2: Single-Node Mode
//!
//! In Phase 2, a single node produces blocks without BFT consensus. The
//! [`SingleNodeDriver`] drives Reth's Engine API to build, validate, and
//! finalize blocks in a loop. Every block is immediately finalized.
//!
//! ## Phase 3+: Multi-Validator Mode
//!
//! Phase 3 will introduce Malachite BFT consensus. The [`ConsensusDriver`] trait
//! provides a stable interface that the Malachite-backed implementation will satisfy,
//! allowing `agnetd` to switch between modes via configuration.
//!
//! ## Architecture
//!
//! ```text
//! Malachite CL (Phase 3)          Single-Node Driver (Phase 2)
//!        |                                |
//!        v                                v
//!   ConsensusDriver trait (stable interface)
//!                 |
//!                 v
//!        EngineApiClient (HTTP + JWT)
//!                 |
//!                 v
//!          Reth EL (port 8551)
//! ```
//!
//! ## Block Production Flow
//!
//! Each block cycle:
//! 1. `engine_forkchoiceUpdatedV3` with payload attributes → trigger block building
//! 2. `engine_getPayloadV3` → retrieve the built block
//! 3. `engine_newPayloadV3` → submit for validation
//! 4. `engine_forkchoiceUpdatedV3` → advance head + safe + finalized

pub mod bft;
pub mod error;
pub mod malachite_handler;
pub mod single_node;
pub mod types;

pub use bft::{CommitCertificate, DoubleSignEvidence, SignedVote, VoteCollector, VoteStep};
pub use error::{BridgeError, Result};
pub use malachite_handler::{MalachiteEvent, MalachiteHandler, MalachiteResponse};
pub use single_node::{SingleNodeConfig, SingleNodeDriver};
pub use types::{BlockProduced, BridgeState, ValidatorInfo, ValidatorSet};
