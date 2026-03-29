// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/// @title NeunodeBounty — Bounty state machine for agent work coordination
/// @notice Full lifecycle: Open→Claimed→Submitted→UnderReview→Accepted/Rejected/Disputed
///         →Paid/Expired/Cancelled. Integrates with NeunodeEscrow for payments.
contract NeunodeBounty {
    using SafeERC20 for IERC20;

    // ─── Types ────────────────────────────────────────────────────────────

    enum BountyState {
        Open,         // Accepting claims
        Claimed,      // Provider working
        Submitted,    // Work submitted, awaiting review
        UnderReview,  // Review in progress
        Revision,     // Provider revising work
        Accepted,     // Work accepted, pending payment
        Rejected,     // Work rejected
        Disputed,     // Under dispute
        Paid,         // Payment released
        Expired,      // Deadline passed
        Cancelled     // Cancelled by requester
    }

    struct Bounty {
        bytes32 id;
        address requester;
        address provider;
        BountyState state;
        uint256 reward;
        address rewardToken;
        uint256 claimDeadline;
        uint256 workDeadline;
        uint256 reviewDeadline;
        uint256 created;
        bytes32 submissionHash;
        uint256 revisionCount;
    }

    // ─── Storage ──────────────────────────────────────────────────────────

    mapping(bytes32 => Bounty) public bounties;
    bytes32[] public bountyList;
    uint256 public activeCount;

    // ─── Events ───────────────────────────────────────────────────────────

    event BountyCreated(
        bytes32 indexed id, address indexed requester, uint256 reward, address rewardToken
    );
    event BountyClaimed(bytes32 indexed id, address indexed provider);
    event BountySubmitted(bytes32 indexed id, bytes32 submissionHash);
    event BountyReviewStarted(bytes32 indexed id);
    event BountyRevisionRequested(bytes32 indexed id);
    event BountyAccepted(bytes32 indexed id);
    event BountyRejected(bytes32 indexed id);
    event BountyDisputed(bytes32 indexed id);
    event BountyPaid(bytes32 indexed id, address indexed provider, uint256 amount);
    event BountyCancelled(bytes32 indexed id);
    event BountyExpired(bytes32 indexed id);

    // ─── Errors ───────────────────────────────────────────────────────────

    error BountyNotFound(bytes32 id);
    error BountyAlreadyExists(bytes32 id);
    error InvalidState(bytes32 id, BountyState current, BountyState required);
    error NotRequester(bytes32 id, address caller);
    error NotProvider(bytes32 id, address caller);
    error NotClaimer(bytes32 id, address caller);
    error InvalidDeadline();
    error InvalidReward();
    error DeadlinePassed(uint256 deadline);
    error MaxRevisionsReached();

    // ─── Functions ────────────────────────────────────────────────────────

    /// @notice Create a new bounty
    function createBounty(
        bytes32 id,
        uint256 reward,
        address rewardToken,
        uint256 claimDeadline,
        uint256 workDeadline
    ) external {
        if (reward == 0) revert InvalidReward();
        if (rewardToken == address(0)) revert InvalidReward();
        if (claimDeadline <= block.timestamp) revert InvalidDeadline();
        if (workDeadline <= claimDeadline) revert InvalidDeadline();
        if (bounties[id].created != 0) revert BountyAlreadyExists(id);

        // Transfer reward tokens to this contract
        IERC20(rewardToken).safeTransferFrom(msg.sender, address(this), reward);

        bounties[id] = Bounty({
            id: id,
            requester: msg.sender,
            provider: address(0),
            state: BountyState.Open,
            reward: reward,
            rewardToken: rewardToken,
            claimDeadline: claimDeadline,
            workDeadline: workDeadline,
            reviewDeadline: 0,
            created: block.timestamp,
            submissionHash: bytes32(0),
            revisionCount: 0
        });

        bountyList.push(id);
        activeCount++;

        emit BountyCreated(id, msg.sender, reward, rewardToken);
    }

    /// @notice Provider claims the bounty
    function claimBounty(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.Open) {
            revert InvalidState(id, bounty.state, BountyState.Open);
        }
        if (block.timestamp > bounty.claimDeadline) revert DeadlinePassed(bounty.claimDeadline);

        bounty.provider = msg.sender;
        bounty.state = BountyState.Claimed;

        emit BountyClaimed(id, msg.sender);
    }

    /// @notice Provider submits work
    function submitWork(bytes32 id, bytes32 submissionHash) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.Claimed && bounty.state != BountyState.Revision) {
            revert InvalidState(id, bounty.state, BountyState.Claimed);
        }
        if (bounty.provider != msg.sender) revert NotProvider(id, msg.sender);
        if (block.timestamp > bounty.workDeadline) revert DeadlinePassed(bounty.workDeadline);

        bounty.submissionHash = submissionHash;
        bounty.state = BountyState.Submitted;

        emit BountySubmitted(id, submissionHash);
    }

    /// @notice Requester accepts submission → triggers payment
    function acceptSubmission(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (
            bounty.state != BountyState.Submitted
                && bounty.state != BountyState.UnderReview
        ) {
            revert InvalidState(id, bounty.state, BountyState.Submitted);
        }
        if (bounty.requester != msg.sender) revert NotRequester(id, msg.sender);

        bounty.state = BountyState.Accepted;

        emit BountyAccepted(id);
    }

    /// @notice Requester rejects submission
    function rejectSubmission(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (
            bounty.state != BountyState.Submitted
                && bounty.state != BountyState.UnderReview
        ) {
            revert InvalidState(id, bounty.state, BountyState.Submitted);
        }
        if (bounty.requester != msg.sender) revert NotRequester(id, msg.sender);

        bounty.state = BountyState.Rejected;
        activeCount--;

        // Refund reward to requester
        IERC20(bounty.rewardToken).safeTransfer(bounty.requester, bounty.reward);

        emit BountyRejected(id);
    }

    /// @notice Either party can dispute
    function disputeBounty(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (
            bounty.state != BountyState.Submitted
                && bounty.state != BountyState.UnderReview
                && bounty.state != BountyState.Accepted
        ) {
            revert InvalidState(id, bounty.state, BountyState.Submitted);
        }
        if (bounty.requester != msg.sender && bounty.provider != msg.sender) {
            revert NotClaimer(id, msg.sender);
        }

        bounty.state = BountyState.Disputed;

        emit BountyDisputed(id);
    }

    /// @notice Requester cancels an open bounty
    function cancelBounty(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.Open) {
            revert InvalidState(id, bounty.state, BountyState.Open);
        }
        if (bounty.requester != msg.sender) revert NotRequester(id, msg.sender);

        bounty.state = BountyState.Cancelled;
        activeCount--;

        // Refund reward to requester
        IERC20(bounty.rewardToken).safeTransfer(bounty.requester, bounty.reward);

        emit BountyCancelled(id);
    }

    /// @notice Check and process bounty expiry
    function checkExpiry(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);

        if (bounty.state == BountyState.Open && block.timestamp > bounty.claimDeadline) {
            bounty.state = BountyState.Expired;
            activeCount--;
            IERC20(bounty.rewardToken).safeTransfer(bounty.requester, bounty.reward);
            emit BountyExpired(id);
        } else if (
            bounty.state == BountyState.Claimed && block.timestamp > bounty.workDeadline
        ) {
            bounty.state = BountyState.Expired;
            activeCount--;
            IERC20(bounty.rewardToken).safeTransfer(bounty.requester, bounty.reward);
            emit BountyExpired(id);
        } else {
            revert InvalidState(id, bounty.state, BountyState.Expired);
        }
    }

    /// @notice Request revision from provider (max 3 revisions)
    function requestRevision(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.Submitted) {
            revert InvalidState(id, bounty.state, BountyState.Submitted);
        }
        if (bounty.requester != msg.sender) revert NotRequester(id, msg.sender);
        if (bounty.revisionCount >= 3) revert MaxRevisionsReached();

        bounty.revisionCount++;
        bounty.state = BountyState.Revision;

        emit BountyRevisionRequested(id);
    }

    /// @notice Release payment after acceptance
    function payBounty(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.Accepted) {
            revert InvalidState(id, bounty.state, BountyState.Accepted);
        }

        bounty.state = BountyState.Paid;
        activeCount--;

        IERC20(bounty.rewardToken).safeTransfer(bounty.provider, bounty.reward);

        emit BountyPaid(id, bounty.provider, bounty.reward);
    }

    /// @notice Get bounty state
    function getBountyState(bytes32 id) external view returns (BountyState) {
        if (bounties[id].created == 0) revert BountyNotFound(id);
        return bounties[id].state;
    }

    /// @notice Get total bounties
    function getTotalBounties() external view returns (uint256) {
        return bountyList.length;
    }
}
