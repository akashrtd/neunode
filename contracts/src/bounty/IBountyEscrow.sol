// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

/// @title IBountyEscrow — Interface for escrow integration with bounty lifecycle
/// @notice Defines the hooks that NeunodeBounty calls into NeunodeEscrow.
interface IBountyEscrow {
    /// @notice Create an escrow deposit tied to a bounty
    function createBountyEscrow(
        bytes32 bountyId,
        address requester,
        address token,
        uint256 amount,
        uint256 workDeadline
    ) external;

    /// @notice Provider bonds when claiming a bounty
    function bondProvider(bytes32 bountyId, address provider, uint256 bondAmount) external;

    /// @notice Release payment with fee splitting on bounty acceptance
    function releaseWithFees(
        bytes32 bountyId,
        address provider,
        uint256 protocolFeeBps,
        uint256 reviewerFeeBps,
        uint256 verificationFeeBps,
        address protocolFeeRecipient,
        address reviewerFeeRecipient,
        address verificationFeeRecipient
    ) external;

    /// @notice Refund requester on bounty rejection
    function refundRequester(bytes32 bountyId) external;

    /// @notice Check if escrow exists and is funded for a bounty
    function isEscrowFunded(bytes32 bountyId) external view returns (bool);
}
