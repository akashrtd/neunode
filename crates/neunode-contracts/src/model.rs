//! ModelRegistry + RoyaltySplitter contract bindings.
//!
//! ModelRegistry tracks model provenance as a DAG. Each model has a content hash (CID),
//! optional parent models, a contributor address, and a contribution type.
//!
//! RoyaltySplitter distributes royalties to model ancestors using BFS traversal
//! with weights based on contribution type and recency decay.

use alloy::sol;

sol! {
    // ═══════════════════════════════════════════════════════════════════════
    //  ModelRegistry
    // ═══════════════════════════════════════════════════════════════════════

    // ─── Enums ────────────────────────────────────────────────────────────

    #[derive(Debug, PartialEq, Eq)]
    enum ContributionType {
        PreTraining,
        FineTune,
        RL,
        Data,
        Compute,
        Serving
    }

    // ─── Structs ──────────────────────────────────────────────────────────

    #[derive(Debug, PartialEq)]
    struct ModelInfo {
        bytes32 cid;
        address contributor;
        ContributionType contribution;
        string metadataURI;
        uint256 registeredAt;
        bool exists;
    }

    // ─── Events ───────────────────────────────────────────────────────────

    #[derive(Debug)]
    event ModelRegistered(
        bytes32 indexed cid,
        address indexed contributor,
        ContributionType contribution,
        bytes32[] parentCids
    );

    #[derive(Debug)]
    event LineageExtended(
        bytes32 indexed parentCid,
        bytes32 indexed childCid,
        address indexed contributor
    );

    // ─── Errors ───────────────────────────────────────────────────────────

    error ModelNotFound(bytes32 cid);
    error ModelAlreadyExists(bytes32 cid);
    error InvalidCid(bytes32 cid);
    error ParentNotFound(bytes32 cid);
    error DerivationProofRequired(bytes32 parentCid);

    // ─── Functions ────────────────────────────────────────────────────────

    function registerModel(
        bytes32 cid,
        bytes32[] calldata parentCids,
        ContributionType contribution,
        string calldata metadataURI,
        bytes32 derivationProofHash
    ) external;

    function getModel(bytes32 cid) external view returns (ModelInfo memory);
    function getParents(bytes32 cid) external view returns (bytes32[] memory);
    function getChildren(bytes32 cid) external view returns (bytes32[] memory);
    function getLineageDepth(bytes32 cid) external view returns (uint256);
    function modelExists(bytes32 cid) external view returns (bool);
    function getModelCount() external view returns (uint256);

    // ═══════════════════════════════════════════════════════════════════════
    //  RoyaltySplitter
    // ═══════════════════════════════════════════════════════════════════════

    // ─── Structs ──────────────────────────────────────────────────────────

    #[derive(Debug, PartialEq)]
    struct RecipientInfo {
        address contributor;
        uint256 weight;
        uint256 depth;
    }

    // ─── Events ───────────────────────────────────────────────────────────

    #[derive(Debug)]
    event RoyaltyDistributed(
        bytes32 indexed modelCid,
        address indexed token,
        uint256 totalAmount,
        uint256 recipientCount
    );

    #[derive(Debug)]
    event RecipientPaid(
        bytes32 indexed modelCid,
        address indexed recipient,
        uint256 amount,
        uint256 depth
    );

    #[derive(Debug)]
    event ProtocolRoyaltyBpsUpdated(uint256 oldBps, uint256 newBps);

    // ─── Errors ───────────────────────────────────────────────────────────

    error NoLineage(bytes32 cid);
    error ZeroAmount();
    error DistributionFailed(address recipient, uint256 amount);
    error ZeroAddress();
    error BpsExceedsMax(uint256 bps, uint256 max);
    error InvalidContributionType(uint8 contributionType);
    error LineageTooDeep(uint256 nodes, uint256 max);

    // ─── Functions ────────────────────────────────────────────────────────

    // Admin
    function setProtocolRoyaltyBps(uint256 newBps) external;
    function setDefaultReceiver(address receiver) external;
    function setMaxLineageDepth(uint256 newMax) external;

    // ERC-2981
    function royaltyInfo(uint256 tokenId, uint256 salePrice) external view returns (address receiver, uint256 royaltyAmount);

    // Distribution
    function distributeRoyalties(bytes32 modelCid, uint256 amount, address token) external;

    // View
    function getRecipients(bytes32 modelCid) external view returns (RecipientInfo[] memory);
    function getContributionTypeWeight(uint8 contributionType) external view returns (uint256);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, FixedBytes, U256};
    use alloy::primitives::{address, fixed_bytes};
    use alloy::sol_types::{SolError, SolEvent};

    // ─── ContributionType enum tests ────────────────────────────────────────

    #[test]
    fn contribution_type_all_variants() {
        let _pre_training = ContributionType::PreTraining;
        let _fine_tune = ContributionType::FineTune;
        let _rl = ContributionType::RL;
        let _data = ContributionType::Data;
        let _compute = ContributionType::Compute;
        let _serving = ContributionType::Serving;
    }

    #[test]
    fn contribution_type_equality() {
        assert_eq!(ContributionType::PreTraining, ContributionType::PreTraining);
        assert_ne!(ContributionType::PreTraining, ContributionType::FineTune);
        assert_ne!(ContributionType::RL, ContributionType::Data);
        assert_ne!(ContributionType::Compute, ContributionType::Serving);
    }

    #[test]
    fn contribution_type_distinct_values() {
        let variants = [
            ContributionType::PreTraining,
            ContributionType::FineTune,
            ContributionType::RL,
            ContributionType::Data,
            ContributionType::Compute,
            ContributionType::Serving,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i] as u8, variants[j] as u8,
                    "ContributionType variants must have distinct discriminants");
            }
        }
    }

    // ─── ModelInfo struct tests ─────────────────────────────────────────────

    #[test]
    fn model_info_construction() {
        let info = ModelInfo {
            cid: fixed_bytes!("0000000000000000000000000000000000000000000000000000000000000001"),
            contributor: address!("0000000000000000000000000000000000000001"),
            contribution: ContributionType::FineTune,
            metadataURI: "ipfs://QmExample".to_string(),
            registeredAt: U256::from(1000),
            exists: true,
        };
        assert!(info.exists);
        assert_eq!(info.contribution, ContributionType::FineTune);
        assert_eq!(info.metadataURI, "ipfs://QmExample");
    }

    #[test]
    fn model_info_non_existent() {
        let info = ModelInfo {
            cid: FixedBytes::<32>::ZERO,
            contributor: Address::ZERO,
            contribution: ContributionType::PreTraining,
            metadataURI: String::new(),
            registeredAt: U256::ZERO,
            exists: false,
        };
        assert!(!info.exists);
        assert!(info.metadataURI.is_empty());
    }

    #[test]
    fn model_info_debug_format() {
        let info = ModelInfo {
            cid: FixedBytes::<32>::ZERO,
            contributor: Address::ZERO,
            contribution: ContributionType::Compute,
            metadataURI: String::new(),
            registeredAt: U256::ZERO,
            exists: false,
        };
        let debug_str = format!("{info:?}");
        assert!(debug_str.contains("ModelInfo"));
    }

    // ─── RecipientInfo struct tests ─────────────────────────────────────────

    #[test]
    fn recipient_info_construction() {
        let info = RecipientInfo {
            contributor: address!("0000000000000000000000000000000000000001"),
            weight: U256::from(4000),
            depth: U256::from(2),
        };
        assert_eq!(info.contributor, address!("0000000000000000000000000000000000000001"));
        assert_eq!(info.weight, U256::from(4000));
        assert_eq!(info.depth, U256::from(2));
    }

    #[test]
    fn recipient_info_zero_fields() {
        let info = RecipientInfo {
            contributor: Address::ZERO,
            weight: U256::ZERO,
            depth: U256::ZERO,
        };
        assert_eq!(info.contributor, Address::ZERO);
        assert_eq!(info.weight, U256::ZERO);
        assert_eq!(info.depth, U256::ZERO);
    }

    // ─── Event signature tests ──────────────────────────────────────────────

    #[test]
    fn model_event_signatures_non_empty() {
        assert!(!ModelRegistered::SIGNATURE.is_empty());
        assert!(!LineageExtended::SIGNATURE.is_empty());
    }

    #[test]
    fn model_event_signatures_expected_format() {
        assert!(ModelRegistered::SIGNATURE.starts_with("ModelRegistered("));
        assert!(LineageExtended::SIGNATURE.starts_with("LineageExtended("));
    }

    #[test]
    fn royalty_event_signatures_non_empty() {
        assert!(!RoyaltyDistributed::SIGNATURE.is_empty());
        assert!(!RecipientPaid::SIGNATURE.is_empty());
        assert!(!ProtocolRoyaltyBpsUpdated::SIGNATURE.is_empty());
    }

    #[test]
    fn royalty_event_signatures_expected_format() {
        assert!(RoyaltyDistributed::SIGNATURE.starts_with("RoyaltyDistributed("));
        assert!(RecipientPaid::SIGNATURE.starts_with("RecipientPaid("));
        assert!(ProtocolRoyaltyBpsUpdated::SIGNATURE.starts_with("ProtocolRoyaltyBpsUpdated("));
    }

    #[test]
    fn all_event_selectors_are_32_bytes() {
        assert_eq!(ModelRegistered::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(LineageExtended::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(RoyaltyDistributed::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(RecipientPaid::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(ProtocolRoyaltyBpsUpdated::SIGNATURE_HASH.as_slice().len(), 32);
    }

    #[test]
    fn all_event_selectors_unique() {
        let selectors = [
            ModelRegistered::SIGNATURE_HASH,
            LineageExtended::SIGNATURE_HASH,
            RoyaltyDistributed::SIGNATURE_HASH,
            RecipientPaid::SIGNATURE_HASH,
            ProtocolRoyaltyBpsUpdated::SIGNATURE_HASH,
        ];
        for i in 0..selectors.len() {
            for j in (i + 1)..selectors.len() {
                assert_ne!(
                    selectors[i], selectors[j],
                    "Model event selectors must be unique"
                );
            }
        }
    }

    // ─── Error construction tests ───────────────────────────────────────────

    #[test]
    fn model_error_types_constructible() {
        let cid = fixed_bytes!("0000000000000000000000000000000000000000000000000000000000000001");
        let _addr = address!("0000000000000000000000000000000000000001");

        let _ = ModelNotFound { cid };
        let _ = ModelAlreadyExists { cid };
        let _ = InvalidCid { cid };
        let _ = ParentNotFound { cid };
        let _ = DerivationProofRequired { parentCid: cid };
    }

    #[test]
    fn royalty_error_types_constructible() {
        let cid = fixed_bytes!("0000000000000000000000000000000000000000000000000000000000000001");
        let addr = address!("0000000000000000000000000000000000000001");

        let _ = NoLineage { cid };
        let _ = ZeroAmount {};
        let _ = DistributionFailed {
            recipient: addr,
            amount: U256::from(100),
        };
        let _ = ZeroAddress {};
        let _ = BpsExceedsMax {
            bps: U256::from(10001),
            max: U256::from(10000),
        };
        let _ = InvalidContributionType {
            contributionType: 6,
        };
        let _ = LineageTooDeep {
            nodes: U256::from(100),
            max: U256::from(50),
        };
    }

    #[test]
    fn model_error_selectors_are_4_bytes() {
        assert_eq!(ModelNotFound::SELECTOR.len(), 4);
        assert_eq!(ModelAlreadyExists::SELECTOR.len(), 4);
        assert_eq!(InvalidCid::SELECTOR.len(), 4);
        assert_eq!(ParentNotFound::SELECTOR.len(), 4);
        assert_eq!(DerivationProofRequired::SELECTOR.len(), 4);
    }

    #[test]
    fn royalty_error_selectors_are_4_bytes() {
        assert_eq!(NoLineage::SELECTOR.len(), 4);
        assert_eq!(ZeroAmount::SELECTOR.len(), 4);
        assert_eq!(DistributionFailed::SELECTOR.len(), 4);
        assert_eq!(ZeroAddress::SELECTOR.len(), 4);
        assert_eq!(BpsExceedsMax::SELECTOR.len(), 4);
        assert_eq!(InvalidContributionType::SELECTOR.len(), 4);
        assert_eq!(LineageTooDeep::SELECTOR.len(), 4);
    }

    // ─── Royalty split calculation tests ────────────────────────────────────

    #[test]
    fn royalty_split_bps_calculation() {
        // Protocol takes X bps, remainder split among ancestors by weight
        let total_amount = U256::from(10000);
        let protocol_bps = U256::from(1000); // 10%
        let protocol_share = total_amount * protocol_bps / U256::from(10000);
        assert_eq!(protocol_share, U256::from(1000));
        assert_eq!(total_amount - protocol_share, U256::from(9000));
    }

    #[test]
    fn royalty_depth_decay() {
        // Deeper ancestors get smaller shares
        let total_pool = U256::from(1000);
        // Depth 0: weight 500, Depth 1: weight 300, Depth 2: weight 200
        let weights = [U256::from(500), U256::from(300), U256::from(200)];
        let total_weight: U256 = weights.iter().fold(U256::ZERO, |acc, w| acc + *w);
        assert_eq!(total_weight, U256::from(1000));

        for (depth, weight) in weights.iter().enumerate() {
            let share = total_pool * *weight / total_weight;
            assert_eq!(share, weights[depth]);
        }
    }
}
