//! NeunodeGovernance contract bindings.
//!
//! On-chain governance with staked token voting. Full proposal lifecycle:
//! Pending -> Active -> Succeeded/Defeated -> Queued -> Executed/Expired.
//! Cancelled from Pending. Voting power via checkpointed staked balances.

use alloy::sol;

sol! {
    // ─── Enums ────────────────────────────────────────────────────────────

    #[derive(Debug, PartialEq, Eq)]
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

    #[derive(Debug, PartialEq, Eq)]
    enum VoteType {
        Against,
        For,
        Abstain
    }

    // ─── Structs ──────────────────────────────────────────────────────────

    #[derive(Debug, PartialEq)]
    struct Proposal {
        uint256 id;
        address proposer;
        address[] targets;
        uint256[] values;
        bytes[] calldatas;
        bytes32 descriptionHash;
        uint256 voteStart;
        uint256 voteEnd;
        uint256 forVotes;
        uint256 againstVotes;
        uint256 abstainVotes;
        uint256 snapshotBlock;
        bool executed;
        bool cancelled;
        uint256 queuedAt;
    }

    #[derive(Debug, PartialEq)]
    struct Checkpoint {
        uint256 fromBlock;
        uint256 power;
    }

    // ─── Events ───────────────────────────────────────────────────────────

    #[derive(Debug)]
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

    #[derive(Debug)]
    event VoteCast(
        uint256 indexed proposalId,
        address indexed voter,
        uint8 support,
        uint256 weight,
        string reason
    );

    #[derive(Debug)]
    event ProposalQueued(uint256 indexed proposalId, uint256 eta);

    #[derive(Debug)]
    event ProposalExecuted(uint256 indexed proposalId);

    #[derive(Debug)]
    event ProposalCancelled(uint256 indexed proposalId);

    #[derive(Debug)]
    event GovernanceParametersUpdated(address indexed updater);

    #[derive(Debug)]
    event AllowedTargetUpdated(address indexed target, bool allowed);

    // ─── Errors ───────────────────────────────────────────────────────────

    error ProposalNotFound(uint256 proposalId);
    error ProposalNotActive(uint256 proposalId);
    error AlreadyVoted(uint256 proposalId, address voter);
    error VotingPowerZero(address voter);
    error BelowProposalThreshold(address proposer, uint256 threshold, uint256 actual);
    error QuorumNotReached(uint256 proposalId);
    error ProposalNotSucceeded(uint256 proposalId);
    error ProposalNotQueued(uint256 proposalId);
    error ProposalNotReady(uint256 proposalId);
    error ProposalAlreadyExecuted(uint256 proposalId);
    error ProposalAlreadyCancelled(uint256 proposalId);
    error ProposalNotCancellable(uint256 proposalId);
    error ArrayLengthMismatch();
    error EmptyProposal();
    error ZeroAddress();
    error ExecutionFailed(uint256 proposalId);
    error NotAuthorized(address caller);
    error TargetNotAllowed(address target);

    // ─── Functions ────────────────────────────────────────────────────────

    // Checkpoint
    function checkpoint() external;
    function getVotes(address account, uint256 blockNumber) external view returns (uint256);

    // Propose
    function propose(
        address[] calldata targets,
        uint256[] calldata values,
        bytes[] calldata calldatas,
        string calldata description
    ) external returns (uint256 proposalId);

    // Vote
    function castVote(uint256 proposalId, uint8 support) external returns (uint256);
    function castVoteWithReason(uint256 proposalId, uint8 support, string calldata reason) external returns (uint256);

    // Lifecycle
    function queue(uint256 proposalId) external;
    function execute(uint256 proposalId) external payable;
    function cancel(uint256 proposalId) external;

    // State
    function state(uint256 proposalId) external view returns (ProposalState);

    // Emergency
    function pause() external;
    function unpause() external;

    // Parameter updates
    function setVotingDelay(uint256 newVotingDelay) external;
    function setVotingPeriod(uint256 newVotingPeriod) external;
    function setProposalThreshold(uint256 newThreshold) external;
    function setQuorumBps(uint256 newQuorumBps) external;
    function setTimelock(uint256 newTimelock) external;
    function setExecutionWindow(uint256 newWindow) external;
    function setAllowedTarget(address target, bool allowed) external;

    // View
    function getProposal(uint256 proposalId) external view returns (
        address proposer_,
        uint256 voteStart,
        uint256 voteEnd,
        uint256 forVotes,
        uint256 againstVotes,
        uint256 abstainVotes,
        uint256 snapshotBlock_,
        bool executed_,
        bool cancelled_,
        uint256 queuedAt
    );

    function hasVoted(uint256 proposalId, address account) external view returns (bool);
    function getProposalActions(uint256 proposalId) external view returns (
        address[] memory targets,
        uint256[] memory values,
        bytes[] memory calldatas
    );
}
