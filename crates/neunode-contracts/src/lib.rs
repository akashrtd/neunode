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

    // ─── BountyState conversion tests ──────────────────────────────────────

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

    #[test]
    fn bounty_state_core_to_contract_individual() {
        assert_eq!(
            bounty::BountyState::Open,
            neunode_core::BountyState::Open.into()
        );
        assert_eq!(
            bounty::BountyState::Claimed,
            neunode_core::BountyState::Claimed.into()
        );
        assert_eq!(
            bounty::BountyState::Submitted,
            neunode_core::BountyState::Submitted.into()
        );
        assert_eq!(
            bounty::BountyState::UnderReview,
            neunode_core::BountyState::UnderReview.into()
        );
        assert_eq!(
            bounty::BountyState::Revision,
            neunode_core::BountyState::Revision.into()
        );
        assert_eq!(
            bounty::BountyState::Accepted,
            neunode_core::BountyState::Accepted.into()
        );
        assert_eq!(
            bounty::BountyState::Rejected,
            neunode_core::BountyState::Rejected.into()
        );
        assert_eq!(
            bounty::BountyState::Disputed,
            neunode_core::BountyState::Disputed.into()
        );
        assert_eq!(
            bounty::BountyState::Paid,
            neunode_core::BountyState::Paid.into()
        );
        assert_eq!(
            bounty::BountyState::Expired,
            neunode_core::BountyState::Expired.into()
        );
        assert_eq!(
            bounty::BountyState::Cancelled,
            neunode_core::BountyState::Cancelled.into()
        );
    }

    #[test]
    fn bounty_state_contract_to_core_individual() {
        // Verify each variant converts back correctly
        let pairs: Vec<(bounty::BountyState, neunode_core::BountyState)> = vec![
            (bounty::BountyState::Open, neunode_core::BountyState::Open),
            (
                bounty::BountyState::Claimed,
                neunode_core::BountyState::Claimed,
            ),
            (
                bounty::BountyState::Submitted,
                neunode_core::BountyState::Submitted,
            ),
            (
                bounty::BountyState::UnderReview,
                neunode_core::BountyState::UnderReview,
            ),
            (
                bounty::BountyState::Revision,
                neunode_core::BountyState::Revision,
            ),
            (
                bounty::BountyState::Accepted,
                neunode_core::BountyState::Accepted,
            ),
            (
                bounty::BountyState::Rejected,
                neunode_core::BountyState::Rejected,
            ),
            (
                bounty::BountyState::Disputed,
                neunode_core::BountyState::Disputed,
            ),
            (bounty::BountyState::Paid, neunode_core::BountyState::Paid),
            (
                bounty::BountyState::Expired,
                neunode_core::BountyState::Expired,
            ),
            (
                bounty::BountyState::Cancelled,
                neunode_core::BountyState::Cancelled,
            ),
        ];
        for (contract, expected_core) in pairs {
            let converted: neunode_core::BountyState = contract.try_into().unwrap();
            assert_eq!(converted, expected_core);
        }
    }

    // ─── ContributionType / TokenType conversion tests ─────────────────────

    #[test]
    fn token_type_to_contribution_type() {
        assert_eq!(
            model::ContributionType::Compute,
            neunode_core::TokenType::Compute.into()
        );
        assert_eq!(
            model::ContributionType::FineTune,
            neunode_core::TokenType::Train.into()
        );
        assert_eq!(
            model::ContributionType::Serving,
            neunode_core::TokenType::Bandwidth.into()
        );
        assert_eq!(
            model::ContributionType::Data,
            neunode_core::TokenType::Storage.into()
        );
    }

    #[test]
    fn contribution_type_to_token_type_valid() {
        // Compute maps directly
        let ct = model::ContributionType::Compute;
        let tt: neunode_core::TokenType = ct.try_into().unwrap();
        assert_eq!(tt, neunode_core::TokenType::Compute);

        // Data maps to Storage
        let ct = model::ContributionType::Data;
        let tt: neunode_core::TokenType = ct.try_into().unwrap();
        assert_eq!(tt, neunode_core::TokenType::Storage);
    }

    #[test]
    fn contribution_type_to_token_type_invalid() {
        // PreTraining, FineTune, RL, Serving have no mapping
        let invalid_variants = [
            model::ContributionType::PreTraining,
            model::ContributionType::FineTune,
            model::ContributionType::RL,
            model::ContributionType::Serving,
        ];
        for variant in invalid_variants {
            let result: Result<neunode_core::TokenType, ConversionError> = variant.try_into();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(
                err,
                ConversionError::UnknownContributionType(_)
            ));
        }
    }

    #[test]
    fn contribution_type_roundtrip_compute() {
        // Compute -> Compute -> Compute is a valid roundtrip
        let core = neunode_core::TokenType::Compute;
        let contract: model::ContributionType = core.into();
        let back: neunode_core::TokenType = contract.try_into().unwrap();
        assert_eq!(core, back);
    }

    #[test]
    fn contribution_type_roundtrip_storage() {
        // Storage -> Data -> Storage is a valid roundtrip
        let core = neunode_core::TokenType::Storage;
        let contract: model::ContributionType = core.into();
        let back: neunode_core::TokenType = contract.try_into().unwrap();
        assert_eq!(core, back);
    }

    // ─── ConversionError display tests ─────────────────────────────────────

    #[test]
    fn conversion_error_messages() {
        let err = ConversionError::InvalidBytes32Length(16);
        assert_eq!(
            err.to_string(),
            "invalid bytes32 length: expected 32 bytes, got 16"
        );

        let err = ConversionError::InvalidAddress("bad format".to_string());
        assert_eq!(err.to_string(), "invalid address format: bad format");

        let err = ConversionError::UnknownBountyState(99);
        assert_eq!(err.to_string(), "unknown bounty state: 99");

        let err = ConversionError::UnknownProposalState(42);
        assert_eq!(err.to_string(), "unknown proposal state: 42");

        let err = ConversionError::UnknownContributionType(7);
        assert_eq!(err.to_string(), "unknown contribution type: 7");
    }

    // ─── Re-export smoke tests ─────────────────────────────────────────────

    #[test]
    fn re_exports_accessible() {
        // Verify that the convenience re-exports are usable
        let _: U256 = U256::from(100);
        let _: I256 = I256::try_from(-50i64).unwrap();
        let _: Address = Address::ZERO;
        let bytes = FixedBytes::<32>::default();
        assert_eq!(bytes.as_slice().len(), 32);
        let _: Bytes = Bytes::new();
    }
}
