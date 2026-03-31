// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title IGovernance — Interface for Neunode on-chain governance
/// @notice Staked token voting with proposal lifecycle, timelock execution,
///         and configurable parameters.
interface IGovernance {
    // ─── Types ────────────────────────────────────────────────────────────

    enum ProposalState {
        Pending,
        Active,
        Succeeded,
        Defeated,
        Queued,
        Executed,
        Expired,
        Cancelled
    }

    enum VoteType {
        Against,
        For,
        Abstain
    }

    // ─── Events ───────────────────────────────────────────────────────────

    event ProposalCreated(
        uint256 indexed proposalId,
        address indexed proposer,
        address[] targets,
        uint256[] values,
        bytes[] calldatas,
        bytes32 descriptionHash,
        uint256 voteStart,
        uint256 voteEnd
    );

    event VoteCast(
        uint256 indexed proposalId,
        address indexed voter,
        uint8 support,
        uint256 weight,
        string reason
    );

    event ProposalQueued(uint256 indexed proposalId, uint256 eta);
    event ProposalExecuted(uint256 indexed proposalId);
    event ProposalCancelled(uint256 indexed proposalId);

    // ─── Functions ────────────────────────────────────────────────────────

    function propose(
        address[] calldata targets,
        uint256[] calldata values,
        bytes[] calldata calldatas,
        string calldata description
    ) external returns (uint256 proposalId);

    function castVote(uint256 proposalId, uint8 support) external returns (uint256 weight);

    function castVoteWithReason(uint256 proposalId, uint8 support, string calldata reason)
        external
        returns (uint256 weight);

    function queue(uint256 proposalId) external;
    function execute(uint256 proposalId) external payable;
    function cancel(uint256 proposalId) external;

    function state(uint256 proposalId) external view returns (ProposalState);
    function getVotes(address account, uint256 blockNumber) external view returns (uint256);
    function checkpoint() external;
}
