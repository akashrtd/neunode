// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/interfaces/IERC2981.sol";
import "@openzeppelin/contracts/interfaces/IERC165.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";
import "./IModelRegistry.sol";
import "./IRoyaltySplitter.sol";

/// @title RoyaltySplitter — ERC-2981 royalty distribution with BFS lineage traversal
/// @notice Distributes royalties to model ancestors using BFS traversal with weights
///         based on contribution type and recency decay. Each ancestor's share is
///         proportional to: shapley_score × contribution_type_weight × recency_decay.
contract RoyaltySplitter is IRoyaltySplitter, IERC2981, AccessControl {
    using SafeERC20 for IERC20;

    // ─── Storage ──────────────────────────────────────────────────────────

    IModelRegistry public immutable registry;

    /// @notice Protocol royalty cap in basis points (default 10% = 1000 bps)
    uint256 public protocolRoyaltyBps;

    /// @notice Default royalty receiver (for ERC-2981 compatibility)
    address public defaultReceiver;

    /// @dev Contribution type weights scaled by 100 (100 = 1.0)
    uint256[6] public contributionTypeWeights = [100, 80, 70, 60, 50, 30];
    // PreTraining=100, FineTune=80, RL=70, Data=60, Compute=50, Serving=30

    /// @dev Recency decay numerator per depth level (0.9 = 9/10)
    uint256 public constant DECAY_NUMERATOR = 9;
    uint256 public constant DECAY_DENOMINATOR = 10;

    /// @dev Maximum lineage depth for BFS traversal (prevents gas exhaustion)
    uint256 public maxLineageDepth;

    /// @dev Shapley score scale (100 = 1.0, stored for future per-model config)
    uint256 public constant DEFAULT_SHAPLEY_SCORE = 100;

    /// @dev Accumulated royalties per model per token (model CID → token → amount)
    mapping(bytes32 => mapping(address => uint256)) public accumulatedRoyalties;

    bytes32 public constant ADMIN_ROLE = keccak256("ADMIN_ROLE");

    // ─── Errors ───────────────────────────────────────────────────────────

    error ZeroAddress();
    error BpsExceedsMax(uint256 bps, uint256 max);
    error InvalidContributionType(uint8 contributionType);
    error LineageTooDeep(uint256 nodes, uint256 max);

    // ─── Constructor ──────────────────────────────────────────────────────

    constructor(address registry_) {
        if (registry_ == address(0)) revert ZeroAddress();
        registry = IModelRegistry(registry_);

        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(ADMIN_ROLE, msg.sender);

        protocolRoyaltyBps = 1000; // 10%
        defaultReceiver = msg.sender;
        maxLineageDepth = 512;
    }

    // ─── ERC-165 ──────────────────────────────────────────────────────────

    function supportsInterface(bytes4 interfaceId)
        public
        view
        virtual
        override(AccessControl, IERC165)
        returns (bool)
    {
        return interfaceId == type(IERC2981).interfaceId || super.supportsInterface(interfaceId);
    }

    // ─── Admin Functions ──────────────────────────────────────────────────

    /// @notice Update protocol royalty cap in basis points
    function setProtocolRoyaltyBps(uint256 newBps) external onlyRole(ADMIN_ROLE) {
        if (newBps > 5000) revert BpsExceedsMax(newBps, 5000);
        uint256 oldBps = protocolRoyaltyBps;
        protocolRoyaltyBps = newBps;
        emit ProtocolRoyaltyBpsUpdated(oldBps, newBps);
    }

    /// @notice Update default receiver for ERC-2981
    function setDefaultReceiver(address receiver) external onlyRole(ADMIN_ROLE) {
        if (receiver == address(0)) revert ZeroAddress();
        defaultReceiver = receiver;
    }

    /// @notice Update max lineage depth for BFS traversal
    function setMaxLineageDepth(uint256 newMax) external onlyRole(ADMIN_ROLE) {
        require(newMax > 0, "max lineage depth must be > 0");
        maxLineageDepth = newMax;
    }

    // ─── ERC-2981 ─────────────────────────────────────────────────────────

    /// @notice ERC-2981 royalty info. Uses tokenId as a model CID hash.
    function royaltyInfo(uint256 tokenId, uint256 salePrice)
        public
        view
        virtual
        override(IERC2981, IRoyaltySplitter)
        returns (address receiver, uint256 royaltyAmount)
    {
        // Convert tokenId to bytes32 CID
        bytes32 modelCid = bytes32(tokenId);

        // If model doesn't exist, return zero
        if (!registry.modelExists(modelCid)) {
            return (address(0), 0);
        }

        receiver = defaultReceiver;
        royaltyAmount = (salePrice * protocolRoyaltyBps) / 10_000;
    }

    // ─── Distribution ─────────────────────────────────────────────────────

    /// @notice Distribute royalties to all ancestors of a model via BFS
    /// @param modelCid The model CID to start from (typically a Serving model)
    /// @param amount Total royalty amount to distribute
    /// @param token ERC-20 token address for payment
    function distributeRoyalties(bytes32 modelCid, uint256 amount, address token)
        external
        override
    {
        if (amount == 0) revert ZeroAmount();
        if (!registry.modelExists(modelCid)) revert ModelNotFound(modelCid);

        // Get all recipients via BFS traversal (ancestors only)
        RecipientInfo[] memory recipients = _getLineageRecipients(modelCid);

        if (recipients.length == 0) revert NoLineage(modelCid);

        // Pull tokens from caller to this contract for distribution
        IERC20(token).safeTransferFrom(msg.sender, address(this), amount);

        // Calculate total weight for proportional split
        uint256 totalWeight = 0;
        for (uint256 i = 0; i < recipients.length; i++) {
            totalWeight += recipients[i].weight;
        }

        if (totalWeight == 0) revert NoLineage(modelCid);

        // Track amounts sent for event
        uint256 totalDistributed = 0;

        // Distribute proportionally
        for (uint256 i = 0; i < recipients.length; i++) {
            uint256 share = (amount * recipients[i].weight) / totalWeight;
            if (share == 0) continue;

            IERC20(token).safeTransfer(recipients[i].contributor, share);

            accumulatedRoyalties[modelCid][token] += share;
            totalDistributed += share;

            emit RecipientPaid(modelCid, recipients[i].contributor, share, recipients[i].depth);
        }

        emit RoyaltyDistributed(modelCid, token, totalDistributed, recipients.length);
    }

    // ─── View Functions ───────────────────────────────────────────────────

    /// @notice Get all royalty recipients for a model with their weights
    function getRecipients(bytes32 modelCid)
        external
        view
        override
        returns (RecipientInfo[] memory)
    {
        if (!registry.modelExists(modelCid)) {
            revert ModelNotFound(modelCid);
        }
        return _getLineageRecipients(modelCid);
    }

    /// @notice Get the weight for a contribution type (scaled by 100)
    function getContributionTypeWeight(uint8 contributionType)
        external
        pure
        override
        returns (uint256)
    {
        if (contributionType > 5) revert InvalidContributionType(contributionType);
        uint256[6] memory weights = [uint256(100), 80, 70, 60, 50, 30];
        return weights[contributionType];
    }

    // ─── Internal ─────────────────────────────────────────────────────────

    /// @dev BFS traversal from model upward through parent lineage.
    ///      Only ancestors (depth > 0) are included as recipients.
    ///      The starting model is the one being used/served — it earns no royalty.
    ///      Reverts if lineage exceeds maxLineageDepth to prevent silent truncation.
    function _getLineageRecipients(bytes32 modelCid)
        internal
        view
        returns (RecipientInfo[] memory)
    {
        // Dynamic BFS using memory arrays that grow as needed
        // We use a mapping-in-memory pattern via a fixed-size visited bitmap
        uint256 maxNodes = maxLineageDepth;
        bytes32[] memory queue = new bytes32[](maxNodes);
        uint256[] memory depths = new uint256[](maxNodes);
        // Track visited nodes using a flat array for O(1) lookup
        // Use linear scan visited set (acceptable for on-chain with maxNodes cap)
        bytes32[] memory visited = new bytes32[](maxNodes);
        uint256 visitedCount = 0;
        uint256 queueSize = 1;
        uint256 head = 0;

        queue[0] = modelCid;
        depths[0] = 0;

        // First pass: count unique ancestors
        uint256 count = 0;
        while (head < queueSize) {
            bytes32 current = queue[head];
            uint256 currentDepth = depths[head];
            head++;

            // Skip visited (linear scan — acceptable with maxNodes cap)
            bool isVisited = false;
            for (uint256 j = 0; j < visitedCount; j++) {
                if (visited[j] == current) {
                    isVisited = true;
                    break;
                }
            }
            if (isVisited) continue;

            visited[visitedCount] = current;
            visitedCount++;

            if (currentDepth > 0) {
                count++;
            }

            // Enqueue parents, respecting maxLineageDepth
            bytes32[] memory parents = registry.getParents(current);
            for (uint256 i = 0; i < parents.length; i++) {
                if (queueSize >= maxNodes) {
                    revert LineageTooDeep(queueSize, maxNodes);
                }
                queue[queueSize] = parents[i];
                depths[queueSize] = currentDepth + 1;
                queueSize++;
            }
        }

        if (count == 0) {
            return new RecipientInfo[](0);
        }

        // Second pass: collect recipients with weights
        RecipientInfo[] memory result = new RecipientInfo[](count);
        uint256 resultIndex = 0;

        // Reset BFS
        queueSize = 1;
        head = 0;
        visitedCount = 0;
        queue[0] = modelCid;
        depths[0] = 0;

        while (head < queueSize) {
            bytes32 current = queue[head];
            uint256 currentDepth = depths[head];
            head++;

            bool isVisited = false;
            for (uint256 j = 0; j < visitedCount; j++) {
                if (visited[j] == current) {
                    isVisited = true;
                    break;
                }
            }
            if (isVisited) continue;

            visited[visitedCount] = current;
            visitedCount++;

            if (currentDepth > 0) {
                IModelRegistry.ModelInfo memory info = registry.getModel(current);
                uint256 typeWeight = contributionTypeWeights[uint8(info.contribution)];
                uint256 decay = _computeDecay(currentDepth);
                uint256 weight = DEFAULT_SHAPLEY_SCORE * typeWeight * decay;

                result[resultIndex] = RecipientInfo({
                    contributor: info.contributor, weight: weight, depth: currentDepth
                });
                resultIndex++;
            }

            bytes32[] memory parents = registry.getParents(current);
            for (uint256 i = 0; i < parents.length; i++) {
                // No overflow check needed — already validated in first pass
                queue[queueSize] = parents[i];
                depths[queueSize] = currentDepth + 1;
                queueSize++;
            }
        }

        return result;
    }

    /// @dev Compute recency decay: (9/10)^depth, scaled by 100
    function _computeDecay(uint256 depth) internal pure returns (uint256) {
        if (depth == 0) return 100;
        uint256 result = 100;
        for (uint256 i = 0; i < depth; i++) {
            result = (result * DECAY_NUMERATOR) / DECAY_DENOMINATOR;
        }
        return result;
    }
}
