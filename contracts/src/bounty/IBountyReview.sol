// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title IBountyReview — Interface for 2-of-3 review committee
interface IBountyReview {
    function assignCommittee(bytes32 bountyId, address[3] calldata reviewers) external;
    function submitReview(
        bytes32 bountyId,
        uint8 score,
        string calldata feedback,
        bytes calldata signature
    ) external;
    function isAccepted(bytes32 bountyId) external view returns (bool);
    function isResolved(bytes32 bountyId) external view returns (bool);
}
