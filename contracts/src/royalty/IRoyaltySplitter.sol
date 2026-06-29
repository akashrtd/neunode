// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

/// @title IRoyaltySplitter — Interface for ERC-2981 royalty distribution with BFS traversal
interface IRoyaltySplitter {
    // ─── Types ────────────────────────────────────────────────────────────

    /// @notice Details of a royalty recipient in the lineage
    struct RecipientInfo {
        address contributor;
        uint256 weight;
        uint256 depth;
    }

    // ─── Events ───────────────────────────────────────────────────────────

    event RoyaltyDistributed(
        bytes32 indexed modelCid, address indexed token, uint256 totalAmount, uint256 recipientCount
    );

    event RecipientPaid(
        bytes32 indexed modelCid, address indexed recipient, uint256 amount, uint256 depth
    );

    event ProtocolRoyaltyBpsUpdated(uint256 oldBps, uint256 newBps);

    // ─── Errors ───────────────────────────────────────────────────────────

    error NoLineage(bytes32 cid);
    error ZeroAmount();
    error DistributionFailed(address recipient, uint256 amount);
    error ModelNotFound(bytes32 cid);

    // ─── Functions ────────────────────────────────────────────────────────

    function distributeRoyalties(bytes32 modelCid, uint256 amount, address token) external;
    function royaltyInfo(uint256 tokenId, uint256 salePrice)
        external
        view
        returns (address receiver, uint256 royaltyAmount);
    function getRecipients(bytes32 modelCid) external view returns (RecipientInfo[] memory);
    function getContributionTypeWeight(uint8 contributionType) external view returns (uint256);
}
