// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/AccessControl.sol";
import "../interfaces/INeunodeToken.sol";
import "./IGovernance.sol";

/// @title NeunodeGovernance — On-chain governance with staked token voting
/// @notice Full proposal lifecycle: Pending → Active → Succeeded/Defeated →
///         Queued → Executed/Expired. Cancelled from Pending. Voting power via
///         checkpointed staked balances. All parameters configurable by GOVERNANCE_ROLE.
contract NeunodeGovernance is AccessControl, IGovernance {
    // ─── Types ────────────────────────────────────────────────────────────

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

    struct Checkpoint {
        uint256 fromBlock;
        uint256 power;
    }

    // ─── Storage ──────────────────────────────────────────────────────────

    INeunodeToken public token;

    uint256 public proposalCount;
    mapping(uint256 => Proposal) private _proposals;

    // Voting power checkpoints per account
    mapping(address => Checkpoint[]) private _checkpoints;
    // Total checkpointed voting power
    Checkpoint[] private _totalPowerCheckpoints;
    // Vote receipt per proposal per account
    mapping(uint256 => mapping(address => bool)) private _hasVoted;

    // Governance parameters
    uint256 public votingDelay;
    uint256 public votingPeriod;
    uint256 public proposalThreshold;
    uint256 public quorumBps;
    uint256 public timelock;
    uint256 public executionWindow;

    // Target whitelist for execute() — only allowed addresses can be called
    mapping(address => bool) public allowedTargets;

    bytes32 public constant GOVERNANCE_ROLE = keccak256("GOVERNANCE_ROLE");

    // ─── Events ───────────────────────────────────────────────────────────

    event GovernanceParametersUpdated(address indexed updater);
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

    // ─── Constructor ──────────────────────────────────────────────────────

    constructor(
        address token_,
        uint256 votingDelay_,
        uint256 votingPeriod_,
        uint256 proposalThreshold_,
        uint256 quorumBps_,
        uint256 timelock_,
        uint256 executionWindow_
    ) {
        if (token_ == address(0)) revert ZeroAddress();

        token = INeunodeToken(token_);
        votingDelay = votingDelay_;
        votingPeriod = votingPeriod_;
        proposalThreshold = proposalThreshold_;
        quorumBps = quorumBps_;
        timelock = timelock_;
        executionWindow = executionWindow_;

        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(GOVERNANCE_ROLE, msg.sender);
    }

    // ─── Checkpoint System ───────────────────────────────────────────────

    /// @notice Register current staked balance as voting power checkpoint
    function checkpoint() external {
        uint256 power = token.stakedBalanceOf(msg.sender);
        _writeCheckpoint(msg.sender, power);
    }

    /// @notice Get voting power at a specific block number
    function getVotes(address account, uint256 blockNumber) public view returns (uint256) {
        return _getPastVotes(account, blockNumber);
    }

    // ─── Propose ─────────────────────────────────────────────────────────

    /// @notice Create a new governance proposal
    function propose(
        address[] calldata targets,
        uint256[] calldata values,
        bytes[] calldata calldatas,
        string calldata description
    ) external returns (uint256 proposalId) {
        if (targets.length == 0) revert EmptyProposal();
        if (targets.length != values.length || targets.length != calldatas.length) {
            revert ArrayLengthMismatch();
        }

        // Check proposer has enough staked tokens
        uint256 proposerPower = token.stakedBalanceOf(msg.sender);
        if (proposerPower < proposalThreshold) {
            revert BelowProposalThreshold(msg.sender, proposalThreshold, proposerPower);
        }

        proposalId = ++proposalCount;
        bytes32 descriptionHash = keccak256(bytes(description));

        Proposal storage p = _proposals[proposalId];
        p.id = proposalId;
        p.proposer = msg.sender;
        p.targets = targets;
        p.values = values;
        p.calldatas = calldatas;
        p.descriptionHash = descriptionHash;
        p.voteStart = block.timestamp + votingDelay;
        p.voteEnd = block.timestamp + votingDelay + votingPeriod;
        p.snapshotBlock = block.number;
        p.executed = false;
        p.cancelled = false;
        p.queuedAt = 0;

        emit ProposalCreated(
            proposalId,
            msg.sender,
            targets,
            values,
            calldatas,
            descriptionHash,
            p.voteStart,
            p.voteEnd
        );
    }

    // ─── Vote ────────────────────────────────────────────────────────────

    /// @notice Cast a vote on an active proposal
    function castVote(uint256 proposalId, uint8 support) external returns (uint256) {
        return _castVote(proposalId, support, "");
    }

    /// @notice Cast a vote with a reason on an active proposal
    function castVoteWithReason(uint256 proposalId, uint8 support, string calldata reason)
        external
        returns (uint256)
    {
        return _castVote(proposalId, support, reason);
    }

    function _castVote(uint256 proposalId, uint8 support, string memory reason)
        internal
        returns (uint256 weight)
    {
        Proposal storage p = _proposals[proposalId];
        if (p.id == 0) revert ProposalNotFound(proposalId);
        if (state(proposalId) != ProposalState.Active) {
            revert ProposalNotActive(proposalId);
        }
        if (_hasVoted[proposalId][msg.sender]) {
            revert AlreadyVoted(proposalId, msg.sender);
        }

        weight = getVotes(msg.sender, p.snapshotBlock);
        if (weight == 0) revert VotingPowerZero(msg.sender);

        _hasVoted[proposalId][msg.sender] = true;

        if (support == uint8(VoteType.Against)) {
            p.againstVotes += weight;
        } else if (support == uint8(VoteType.For)) {
            p.forVotes += weight;
        } else if (support == uint8(VoteType.Abstain)) {
            p.abstainVotes += weight;
        }

        emit VoteCast(proposalId, msg.sender, support, weight, reason);
    }

    // ─── Queue ───────────────────────────────────────────────────────────

    /// @notice Queue a succeeded proposal for execution after timelock
    function queue(uint256 proposalId) external {
        Proposal storage p = _proposals[proposalId];
        if (p.id == 0) revert ProposalNotFound(proposalId);
        if (state(proposalId) != ProposalState.Succeeded) {
            revert ProposalNotSucceeded(proposalId);
        }

        p.queuedAt = block.timestamp;

        emit ProposalQueued(proposalId, block.timestamp + timelock);
    }

    // ─── Execute ─────────────────────────────────────────────────────────

    /// @notice Execute a queued proposal after timelock has passed
    function execute(uint256 proposalId) external payable {
        Proposal storage p = _proposals[proposalId];
        if (p.id == 0) revert ProposalNotFound(proposalId);

        ProposalState currentState = state(proposalId);
        if (currentState == ProposalState.Executed) {
            revert ProposalAlreadyExecuted(proposalId);
        }
        if (currentState != ProposalState.Queued) {
            revert ProposalNotQueued(proposalId);
        }
        if (block.timestamp < p.queuedAt + timelock) {
            revert ProposalNotReady(proposalId);
        }

        p.executed = true;

        for (uint256 i = 0; i < p.targets.length; i++) {
            if (!allowedTargets[p.targets[i]]) revert TargetNotAllowed(p.targets[i]);
            (bool success,) = p.targets[i].call{value: p.values[i]}(p.calldatas[i]);
            if (!success) revert ExecutionFailed(proposalId);
        }

        emit ProposalExecuted(proposalId);
    }

    // ─── Cancel ──────────────────────────────────────────────────────────

    /// @notice Cancel a proposal before voting starts
    function cancel(uint256 proposalId) external {
        Proposal storage p = _proposals[proposalId];
        if (p.id == 0) revert ProposalNotFound(proposalId);
        if (p.executed) revert ProposalAlreadyExecuted(proposalId);
        if (p.cancelled) revert ProposalAlreadyCancelled(proposalId);
        if (state(proposalId) != ProposalState.Pending) {
            revert ProposalNotCancellable(proposalId);
        }
        if (msg.sender != p.proposer && !hasRole(GOVERNANCE_ROLE, msg.sender)) {
            revert NotAuthorized(msg.sender);
        }

        p.cancelled = true;

        emit ProposalCancelled(proposalId);
    }

    // ─── State ───────────────────────────────────────────────────────────

    /// @notice Get the current state of a proposal
    function state(uint256 proposalId) public view returns (ProposalState) {
        Proposal storage p = _proposals[proposalId];
        if (p.id == 0) revert ProposalNotFound(proposalId);

        if (p.cancelled) return ProposalState.Cancelled;
        if (p.executed) return ProposalState.Executed;

        // Before voting starts
        if (block.timestamp < p.voteStart) return ProposalState.Pending;

        // During voting period
        if (block.timestamp <= p.voteEnd) return ProposalState.Active;

        // Voting ended — check result
        if (p.forVotes <= p.againstVotes) return ProposalState.Defeated;

        // Check quorum: total votes must reach quorum threshold
        uint256 totalVotes = p.forVotes + p.abstainVotes + p.againstVotes;
        uint256 totalStaked = _getPastTotalPower(p.snapshotBlock);
        uint256 quorumRequired = (totalStaked * quorumBps) / 10_000;
        if (totalVotes < quorumRequired) return ProposalState.Defeated;

        // Not queued yet
        if (p.queuedAt == 0) return ProposalState.Succeeded;

        // Queued — check if execution window expired
        if (block.timestamp > p.queuedAt + executionWindow) {
            return ProposalState.Expired;
        }

        return ProposalState.Queued;
    }

    // ─── Parameter Updates ───────────────────────────────────────────────

    /// @notice Update voting delay (GOVERNANCE_ROLE only)
    function setVotingDelay(uint256 newVotingDelay) external onlyRole(GOVERNANCE_ROLE) {
        votingDelay = newVotingDelay;
        emit GovernanceParametersUpdated(msg.sender);
    }

    /// @notice Update voting period (GOVERNANCE_ROLE only)
    function setVotingPeriod(uint256 newVotingPeriod) external onlyRole(GOVERNANCE_ROLE) {
        votingPeriod = newVotingPeriod;
        emit GovernanceParametersUpdated(msg.sender);
    }

    /// @notice Update proposal threshold (GOVERNANCE_ROLE only)
    function setProposalThreshold(uint256 newThreshold) external onlyRole(GOVERNANCE_ROLE) {
        proposalThreshold = newThreshold;
        emit GovernanceParametersUpdated(msg.sender);
    }

    /// @notice Update quorum basis points (GOVERNANCE_ROLE only)
    function setQuorumBps(uint256 newQuorumBps) external onlyRole(GOVERNANCE_ROLE) {
        quorumBps = newQuorumBps;
        emit GovernanceParametersUpdated(msg.sender);
    }

    /// @notice Update timelock duration (GOVERNANCE_ROLE only)
    function setTimelock(uint256 newTimelock) external onlyRole(GOVERNANCE_ROLE) {
        timelock = newTimelock;
        emit GovernanceParametersUpdated(msg.sender);
    }

    /// @notice Update execution window (GOVERNANCE_ROLE only)
    function setExecutionWindow(uint256 newWindow) external onlyRole(GOVERNANCE_ROLE) {
        executionWindow = newWindow;
        emit GovernanceParametersUpdated(msg.sender);
    }

    /// @notice Add or remove an address from the allowed targets whitelist
    /// @param target The address to update
    /// @param allowed Whether the target is allowed for execution
    function setAllowedTarget(address target, bool allowed) external onlyRole(GOVERNANCE_ROLE) {
        allowedTargets[target] = allowed;
        emit AllowedTargetUpdated(target, allowed);
    }

    // ─── View Helpers ────────────────────────────────────────────────────

    /// @notice Get proposal details
    function getProposal(uint256 proposalId)
        external
        view
        returns (
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
        )
    {
        Proposal storage p = _proposals[proposalId];
        if (p.id == 0) revert ProposalNotFound(proposalId);
        return (
            p.proposer,
            p.voteStart,
            p.voteEnd,
            p.forVotes,
            p.againstVotes,
            p.abstainVotes,
            p.snapshotBlock,
            p.executed,
            p.cancelled,
            p.queuedAt
        );
    }

    /// @notice Check if an account has voted on a proposal
    function hasVoted(uint256 proposalId, address account) external view returns (bool) {
        return _hasVoted[proposalId][account];
    }

    /// @notice Get proposal targets and calldatas for execution verification
    function getProposalActions(uint256 proposalId)
        external
        view
        returns (address[] memory targets, uint256[] memory values, bytes[] memory calldatas)
    {
        Proposal storage p = _proposals[proposalId];
        if (p.id == 0) revert ProposalNotFound(proposalId);
        return (p.targets, p.values, p.calldatas);
    }

    // ─── Internal Checkpoint Logic ───────────────────────────────────────

    function _writeCheckpoint(address account, uint256 newPower) internal {
        uint256 oldPower = _checkpoints[account].length > 0
            ? _checkpoints[account][_checkpoints[account].length - 1].power
            : 0;

        _checkpoints[account].push(Checkpoint({fromBlock: block.number, power: newPower}));

        // Update total power checkpoints
        uint256 currentTotal = _totalPowerCheckpoints.length > 0
            ? _totalPowerCheckpoints[_totalPowerCheckpoints.length - 1].power
            : 0;

        uint256 newTotal;
        if (newPower >= oldPower) {
            newTotal = currentTotal + (newPower - oldPower);
        } else {
            newTotal = currentTotal - (oldPower - newPower);
        }

        _totalPowerCheckpoints.push(Checkpoint({fromBlock: block.number, power: newTotal}));
    }

    function _getPastVotes(address account, uint256 blockNumber) internal view returns (uint256) {
        Checkpoint[] storage cps = _checkpoints[account];
        if (cps.length == 0) return 0;

        // Binary search: find last checkpoint with fromBlock <= blockNumber
        uint256 low = 0;
        uint256 high = cps.length;

        while (low < high) {
            uint256 mid = (low + high) / 2;
            if (cps[mid].fromBlock > blockNumber) {
                high = mid;
            } else {
                low = mid + 1;
            }
        }

        if (low == 0) return 0;
        return cps[low - 1].power;
    }

    function _getPastTotalPower(uint256 blockNumber) internal view returns (uint256) {
        if (_totalPowerCheckpoints.length == 0) return 0;

        uint256 low = 0;
        uint256 high = _totalPowerCheckpoints.length;

        while (low < high) {
            uint256 mid = (low + high) / 2;
            if (_totalPowerCheckpoints[mid].fromBlock > blockNumber) {
                high = mid;
            } else {
                low = mid + 1;
            }
        }

        if (low == 0) return 0;
        return _totalPowerCheckpoints[low - 1].power;
    }
}
