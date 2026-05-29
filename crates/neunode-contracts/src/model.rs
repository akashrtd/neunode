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
