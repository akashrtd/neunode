//! # neunode-contracts
//!
//! Type-safe Rust bindings for Neunode Solidity contracts using `alloy::sol!`.
//!
//! This crate provides ABI-level bindings for all Neunode L1 contracts:
//!
//! - **NeunodeIdentity** — DID registry for AI agents
//! - **NeunodeBounty** — Bounty state machine for agent work coordination
//! - **NeunodeEscrow** — Bilateral escrow for bounty payments
//! - **NeunodeToken** — Base ERC-20 with staking, decay, and seed tokens
//! - **NeunodeGovernance** — On-chain governance with staked token voting
//! - **ModelRegistry** — Model lineage DAG with content-addressed models
//! - **RoyaltySplitter** — ERC-2981 royalty distribution with BFS traversal
//! - **BountyReview** — 2-of-3 review committee for bounty submissions
//! - **Diamond** — EIP-2535 diamond proxy pattern bindings

pub mod bounty;
pub mod diamond;
pub mod escrow;
pub mod governance;
pub mod identity;
pub mod model;
pub mod review;
pub mod token;

// Re-export commonly used alloy primitives for convenience
pub use alloy::primitives::{Address, Bytes, FixedBytes, I256, U256};

// Re-export sol_types for ABI encoding/decoding
pub use alloy::sol_types::SolCall;

use thiserror::Error;

/// Errors that can occur when converting between neunode-core types and contract types.
#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("invalid bytes32 length: expected 32 bytes, got {0}")]
    InvalidBytes32Length(usize),

    #[error("invalid address format: {0}")]
    InvalidAddress(String),

    #[error("unknown bounty state: {0}")]
    UnknownBountyState(u8),

    #[error("unknown proposal state: {0}")]
    UnknownProposalState(u8),

    #[error("unknown contribution type: {0}")]
    UnknownContributionType(u8),
}

// ─── BountyState bridge ──────────────────────────────────────────────────

impl TryFrom<bounty::BountyState> for neunode_core::BountyState {
    type Error = ConversionError;

    fn try_from(value: bounty::BountyState) -> Result<Self, Self::Error> {
        match value {
            bounty::BountyState::Open => Ok(neunode_core::BountyState::Open),
            bounty::BountyState::Claimed => Ok(neunode_core::BountyState::Claimed),
            bounty::BountyState::Submitted => Ok(neunode_core::BountyState::Submitted),
            bounty::BountyState::UnderReview => Ok(neunode_core::BountyState::UnderReview),
            bounty::BountyState::Revision => Ok(neunode_core::BountyState::Revision),
            bounty::BountyState::Accepted => Ok(neunode_core::BountyState::Accepted),
            bounty::BountyState::Rejected => Ok(neunode_core::BountyState::Rejected),
            bounty::BountyState::Disputed => Ok(neunode_core::BountyState::Disputed),
            bounty::BountyState::Paid => Ok(neunode_core::BountyState::Paid),
            bounty::BountyState::Expired => Ok(neunode_core::BountyState::Expired),
            bounty::BountyState::Cancelled => Ok(neunode_core::BountyState::Cancelled),
            _ => Err(ConversionError::UnknownBountyState(value as u8)),
        }
    }
}

impl From<neunode_core::BountyState> for bounty::BountyState {
    fn from(value: neunode_core::BountyState) -> Self {
        match value {
            neunode_core::BountyState::Open => bounty::BountyState::Open,
            neunode_core::BountyState::Claimed => bounty::BountyState::Claimed,
            neunode_core::BountyState::Submitted => bounty::BountyState::Submitted,
            neunode_core::BountyState::UnderReview => bounty::BountyState::UnderReview,
            neunode_core::BountyState::Revision => bounty::BountyState::Revision,
            neunode_core::BountyState::Accepted => bounty::BountyState::Accepted,
            neunode_core::BountyState::Rejected => bounty::BountyState::Rejected,
            neunode_core::BountyState::Disputed => bounty::BountyState::Disputed,
            neunode_core::BountyState::Paid => bounty::BountyState::Paid,
            neunode_core::BountyState::Expired => bounty::BountyState::Expired,
            neunode_core::BountyState::Cancelled => bounty::BountyState::Cancelled,
        }
    }
}

// ─── ContributionType bridge ─────────────────────────────────────────────

impl TryFrom<model::ContributionType> for neunode_core::TokenType {
    type Error = ConversionError;

    fn try_from(value: model::ContributionType) -> Result<Self, Self::Error> {
        // ContributionType does not directly map to TokenType.
        // This is a placeholder for potential future mapping logic.
        match value {
            model::ContributionType::Compute => Ok(neunode_core::TokenType::Compute),
            model::ContributionType::Data => Ok(neunode_core::TokenType::Storage),
            _ => Err(ConversionError::UnknownContributionType(value as u8)),
        }
    }
}

impl From<neunode_core::TokenType> for model::ContributionType {
    fn from(value: neunode_core::TokenType) -> Self {
        match value {
            neunode_core::TokenType::Compute => model::ContributionType::Compute,
            neunode_core::TokenType::Train => model::ContributionType::FineTune,
            neunode_core::TokenType::Bandwidth => model::ContributionType::Serving,
            neunode_core::TokenType::Storage => model::ContributionType::Data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounty_state_roundtrip() {
        let states = [
            neunode_core::BountyState::Open,
            neunode_core::BountyState::Claimed,
            neunode_core::BountyState::Submitted,
            neunode_core::BountyState::UnderReview,
            neunode_core::BountyState::Revision,
            neunode_core::BountyState::Accepted,
            neunode_core::BountyState::Rejected,
            neunode_core::BountyState::Disputed,
            neunode_core::BountyState::Paid,
            neunode_core::BountyState::Expired,
            neunode_core::BountyState::Cancelled,
        ];

        for state in states {
            let contract_state: bounty::BountyState = state.into();
            let back: neunode_core::BountyState = contract_state.try_into().unwrap();
            assert_eq!(state, back);
        }
    }
}
