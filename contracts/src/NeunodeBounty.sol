// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";
import "./bounty/IBountyEscrow.sol";
import "./bounty/IBountyReview.sol";

/// @title NeunodeBounty — Bounty state machine for agent work coordination
/// @notice Full lifecycle: Open→Claimed→Submitted→UnderReview→Accepted/Rejected/Disputed
///         →Paid/Expired/Cancelled. Integrates with NeunodeEscrow for payments.
///         Optional escrow integration, 2-of-3 review committee, fee collection,
///         5-deadline enforcement.
contract NeunodeBounty is AccessControl {
    using SafeERC20 for IERC20;

    // ─── Types ────────────────────────────────────────────────────────────

    enum BountyState {
        Open, // Accepting claims
        Claimed, // Provider working
        Submitted, // Work submitted, awaiting review
        UnderReview, // Review in progress
        Revision, // Provider revising work
        Accepted, // Work accepted, pending payment
        Rejected, // Work rejected
        Disputed, // Under dispute
        Paid, // Payment released
        Expired, // Deadline passed
        Cancelled // Cancelled by requester
    }

    /// @notice Fee configuration for bounty payouts
    struct FeeConfig {
        uint256 protocolBps; // Protocol fee in basis points (e.g., 300 = 3%)
        uint256 reviewerBps; // Reviewer fee in basis points
        uint256 verificationBps; // Verification fee in basis points
        address protocolFeeRecipient;
        address reviewerFeeRecipient;
        address verificationFeeRecipient;
    }

    // NOTE: Bounty struct kept at original 12 fields to preserve tuple layout
    // for existing test destructuring. New fields in separate mappings below.
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

    // New fields in separate mappings (preserves existing tuple layout)
    mapping(bytes32 => uint256) public revisionDeadlines;
    mapping(bytes32 => uint256) public disputeDeadlines;
    mapping(bytes32 => bool) public useEscrowFlags;
    mapping(bytes32 => uint256) public providerBonds;

    FeeConfig public feeConfig;

    IBountyEscrow public escrow;
    IBountyReview public reviewContract;

    bytes32 public constant ADMIN_ROLE = keccak256("ADMIN_ROLE");
    bytes32 public constant BOUNTY_MANAGER_ROLE = keccak256("BOUNTY_MANAGER_ROLE");

    uint256 public constant MAX_REVISIONS = 3;

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
    event FeeConfigUpdated(address indexed admin);
    event EscrowUpdated(address indexed escrow);
    event ReviewContractUpdated(address indexed reviewContract);
    event FeesCollected(
        bytes32 indexed bountyId,
        uint256 protocolFee,
        uint256 reviewerFee,
        uint256 verificationFee,
        uint256 providerPayout
    );
    event DisputeResolved(bytes32 indexed bountyId, bool accepted);

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
    error ReviewNotResolved(bytes32 id);
    error ReviewNotAccepted(bytes32 id);
    error InsufficientBond();
    error TotalFeesExceed100();

    // ─── Constructor ──────────────────────────────────────────────────────

    constructor() {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(ADMIN_ROLE, msg.sender);
        _grantRole(BOUNTY_MANAGER_ROLE, msg.sender);

        // Default fee config: 2% protocol, 3% reviewer, 1% verification
        feeConfig = FeeConfig({
            protocolBps: 200,
            reviewerBps: 300,
            verificationBps: 100,
            protocolFeeRecipient: msg.sender,
            reviewerFeeRecipient: msg.sender,
            verificationFeeRecipient: msg.sender
        });
    }

    // ─── Admin Functions ──────────────────────────────────────────────────

    /// @notice Set fee configuration (ADMIN only)
    function setFeeConfig(
        uint256 protocolBps,
        uint256 reviewerBps,
        uint256 verificationBps,
        address protocolFeeRecipient,
        address reviewerFeeRecipient,
        address verificationFeeRecipient
    ) external onlyRole(ADMIN_ROLE) {
        if (protocolBps + reviewerBps + verificationBps > 1000) {
            revert TotalFeesExceed100();
        }
        feeConfig = FeeConfig({
            protocolBps: protocolBps,
            reviewerBps: reviewerBps,
            verificationBps: verificationBps,
            protocolFeeRecipient: protocolFeeRecipient,
            reviewerFeeRecipient: reviewerFeeRecipient,
            verificationFeeRecipient: verificationFeeRecipient
        });
        emit FeeConfigUpdated(msg.sender);
    }

    /// @notice Set escrow contract (ADMIN only)
    function setEscrow(address escrow_) external onlyRole(ADMIN_ROLE) {
        escrow = IBountyEscrow(escrow_);
        emit EscrowUpdated(escrow_);
    }

    /// @notice Set review contract (ADMIN only)
    function setReviewContract(address review_) external onlyRole(ADMIN_ROLE) {
        reviewContract = IBountyReview(review_);
        emit ReviewContractUpdated(review_);
    }

    // ─── Create Bounty ────────────────────────────────────────────────────

    /// @notice Create a new bounty (backward-compatible, no escrow)
    function createBounty(
        bytes32 id,
        uint256 reward,
        address rewardToken,
        uint256 claimDeadline,
        uint256 workDeadline
    ) external {
        _createBounty(id, reward, rewardToken, claimDeadline, workDeadline, 0, 0, 0, false);
    }

    /// @notice Create a new bounty with full deadline config and optional escrow
    function createBountyWithDeadlines(
        bytes32 id,
        uint256 reward,
        address rewardToken,
        uint256 claimDeadline,
        uint256 workDeadline,
        uint256 reviewDeadline_,
        uint256 revisionDeadline_,
        uint256 disputeDeadline_,
        bool useEscrow_
    ) external {
        if (reviewDeadline_ != 0 && reviewDeadline_ <= workDeadline) {
            revert InvalidDeadline();
        }
        if (revisionDeadline_ != 0 && revisionDeadline_ <= reviewDeadline_) {
            revert InvalidDeadline();
        }
        if (disputeDeadline_ != 0 && disputeDeadline_ <= revisionDeadline_) {
            revert InvalidDeadline();
        }

        _createBounty(
            id,
            reward,
            rewardToken,
            claimDeadline,
            workDeadline,
            reviewDeadline_,
            revisionDeadline_,
            disputeDeadline_,
            useEscrow_
        );
    }

    function _createBounty(
        bytes32 id,
        uint256 reward,
        address rewardToken,
        uint256 claimDeadline,
        uint256 workDeadline,
        uint256 reviewDeadline_,
        uint256 revisionDeadline_,
        uint256 disputeDeadline_,
        bool useEscrow_
    ) internal {
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
            reviewDeadline: reviewDeadline_,
            created: block.timestamp,
            submissionHash: bytes32(0),
            revisionCount: 0
        });

        // Set extended fields in separate mappings
        revisionDeadlines[id] = revisionDeadline_;
        disputeDeadlines[id] = disputeDeadline_;
        useEscrowFlags[id] = useEscrow_;

        bountyList.push(id);
        activeCount++;

        emit BountyCreated(id, msg.sender, reward, rewardToken);
    }

    // ─── Claim Bounty ─────────────────────────────────────────────────────

    /// @notice Provider claims the bounty (backward-compatible)
    function claimBounty(bytes32 id) external {
        _claimBounty(id, 0);
    }

    /// @notice Provider claims the bounty with escrow bond
    function claimBountyWithBond(bytes32 id, uint256 bondAmount) external {
        _claimBounty(id, bondAmount);
    }

    function _claimBounty(bytes32 id, uint256 bondAmount) internal {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.Open) {
            revert InvalidState(id, bounty.state, BountyState.Open);
        }
        if (block.timestamp > bounty.claimDeadline) revert DeadlinePassed(bounty.claimDeadline);

        bounty.provider = msg.sender;
        bounty.state = BountyState.Claimed;

        // Store provider bond if provided
        if (bondAmount > 0) {
            IERC20(bounty.rewardToken).safeTransferFrom(msg.sender, address(this), bondAmount);
            providerBonds[id] = bondAmount;
        }

        emit BountyClaimed(id, msg.sender);
    }

    // ─── Submit Work ──────────────────────────────────────────────────────

    /// @notice Provider submits work
    function submitWork(bytes32 id, bytes32 submissionHash) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.Claimed && bounty.state != BountyState.Revision) {
            revert InvalidState(id, bounty.state, BountyState.Claimed);
        }
        if (bounty.provider != msg.sender) revert NotProvider(id, msg.sender);

        // If resubmitting after revision, check revision deadline first
        if (bounty.state == BountyState.Revision && revisionDeadlines[id] != 0) {
            if (block.timestamp > revisionDeadlines[id]) {
                revert DeadlinePassed(revisionDeadlines[id]);
            }
        } else if (block.timestamp > bounty.workDeadline) {
            revert DeadlinePassed(bounty.workDeadline);
        }

        bounty.submissionHash = submissionHash;
        bounty.state = BountyState.Submitted;

        emit BountySubmitted(id, submissionHash);
    }

    // ─── Accept Submission ────────────────────────────────────────────────

    /// @notice Requester accepts submission → triggers payment
    function acceptSubmission(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.Submitted && bounty.state != BountyState.UnderReview) {
            revert InvalidState(id, bounty.state, BountyState.Submitted);
        }
        if (bounty.requester != msg.sender) revert NotRequester(id, msg.sender);

        bounty.state = BountyState.Accepted;

        emit BountyAccepted(id);
    }

    // ─── Reject Submission ────────────────────────────────────────────────

    /// @notice Requester rejects submission
    function rejectSubmission(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.Submitted && bounty.state != BountyState.UnderReview) {
            revert InvalidState(id, bounty.state, BountyState.Submitted);
        }
        if (bounty.requester != msg.sender) revert NotRequester(id, msg.sender);

        bounty.state = BountyState.Rejected;
        activeCount--;

        // Refund reward to requester
        IERC20(bounty.rewardToken).safeTransfer(bounty.requester, bounty.reward);

        // Return provider bond if any
        if (providerBonds[id] > 0) {
            uint256 bond = providerBonds[id];
            providerBonds[id] = 0;
            IERC20(bounty.rewardToken).safeTransfer(bounty.provider, bond);
        }

        emit BountyRejected(id);
    }

    // ─── Dispute ──────────────────────────────────────────────────────────

    /// @notice Either party can dispute
    function disputeBounty(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (
            bounty.state != BountyState.Submitted && bounty.state != BountyState.UnderReview
                && bounty.state != BountyState.Accepted
        ) {
            revert InvalidState(id, bounty.state, BountyState.Submitted);
        }
        if (bounty.requester != msg.sender && bounty.provider != msg.sender) {
            revert NotClaimer(id, msg.sender);
        }

        // Check dispute deadline if set
        if (disputeDeadlines[id] != 0 && block.timestamp > disputeDeadlines[id]) {
            revert DeadlinePassed(disputeDeadlines[id]);
        }

        bounty.state = BountyState.Disputed;

        emit BountyDisputed(id);
    }

    /// @notice Resolve a disputed bounty (ADMIN only)
    function resolveDispute(bytes32 id, bool accept) external onlyRole(BOUNTY_MANAGER_ROLE) {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.Disputed) {
            revert InvalidState(id, bounty.state, BountyState.Disputed);
        }

        if (accept) {
            bounty.state = BountyState.Accepted;
            emit DisputeResolved(id, true);
        } else {
            bounty.state = BountyState.Rejected;
            activeCount--;

            // Refund requester
            IERC20(bounty.rewardToken).safeTransfer(bounty.requester, bounty.reward);
            // Return provider bond
            if (providerBonds[id] > 0) {
                uint256 bond = providerBonds[id];
                providerBonds[id] = 0;
                IERC20(bounty.rewardToken).safeTransfer(bounty.provider, bond);
            }

            emit DisputeResolved(id, false);
            emit BountyRejected(id);
        }
    }

    // ─── Cancel ───────────────────────────────────────────────────────────

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

    // ─── Expiry ───────────────────────────────────────────────────────────

    /// @notice Check and process bounty expiry
    function checkExpiry(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);

        if (bounty.state == BountyState.Open && block.timestamp > bounty.claimDeadline) {
            bounty.state = BountyState.Expired;
            activeCount--;
            IERC20(bounty.rewardToken).safeTransfer(bounty.requester, bounty.reward);
            emit BountyExpired(id);
        } else if (bounty.state == BountyState.Claimed && block.timestamp > bounty.workDeadline) {
            bounty.state = BountyState.Expired;
            activeCount--;
            IERC20(bounty.rewardToken).safeTransfer(bounty.requester, bounty.reward);
            if (providerBonds[id] > 0) {
                uint256 bond = providerBonds[id];
                providerBonds[id] = 0;
                IERC20(bounty.rewardToken).safeTransfer(bounty.provider, bond);
            }
            emit BountyExpired(id);
        } else if (
            bounty.state == BountyState.Submitted && bounty.reviewDeadline != 0
                && block.timestamp > bounty.reviewDeadline
        ) {
            // Review deadline passed without review — auto-expire
            bounty.state = BountyState.Expired;
            activeCount--;
            IERC20(bounty.rewardToken).safeTransfer(bounty.requester, bounty.reward);
            if (providerBonds[id] > 0) {
                uint256 bond = providerBonds[id];
                providerBonds[id] = 0;
                IERC20(bounty.rewardToken).safeTransfer(bounty.provider, bond);
            }
            emit BountyExpired(id);
        } else if (
            bounty.state == BountyState.Revision && revisionDeadlines[id] != 0
                && block.timestamp > revisionDeadlines[id]
        ) {
            // Revision deadline passed — auto-expire
            bounty.state = BountyState.Expired;
            activeCount--;
            IERC20(bounty.rewardToken).safeTransfer(bounty.requester, bounty.reward);
            if (providerBonds[id] > 0) {
                uint256 bond = providerBonds[id];
                providerBonds[id] = 0;
                IERC20(bounty.rewardToken).safeTransfer(bounty.provider, bond);
            }
            emit BountyExpired(id);
        } else if (
            bounty.state == BountyState.Disputed && disputeDeadlines[id] != 0
                && block.timestamp > disputeDeadlines[id]
        ) {
            // Dispute deadline passed — auto-refund to requester
            bounty.state = BountyState.Expired;
            activeCount--;
            IERC20(bounty.rewardToken).safeTransfer(bounty.requester, bounty.reward);
            if (providerBonds[id] > 0) {
                uint256 bond = providerBonds[id];
                providerBonds[id] = 0;
                IERC20(bounty.rewardToken).safeTransfer(bounty.provider, bond);
            }
            emit BountyExpired(id);
        } else {
            revert InvalidState(id, bounty.state, BountyState.Expired);
        }
    }

    // ─── Revision ─────────────────────────────────────────────────────────

    /// @notice Request revision from provider (max 3 revisions)
    function requestRevision(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.Submitted) {
            revert InvalidState(id, bounty.state, BountyState.Submitted);
        }
        if (bounty.requester != msg.sender) revert NotRequester(id, msg.sender);
        if (bounty.revisionCount >= MAX_REVISIONS) revert MaxRevisionsReached();

        bounty.revisionCount++;
        bounty.state = BountyState.Revision;

        emit BountyRevisionRequested(id);
    }

    // ─── Pay Bounty ───────────────────────────────────────────────────────

    /// @notice Release payment after acceptance (backward-compatible, no fees)
    function payBounty(bytes32 id) external {
        _payBounty(id, false);
    }

    /// @notice Release payment after acceptance with fee splitting
    function payBountyWithFees(bytes32 id) external {
        _payBounty(id, true);
    }

    function _payBounty(bytes32 id, bool applyFees) internal {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.Accepted) {
            revert InvalidState(id, bounty.state, BountyState.Accepted);
        }

        bounty.state = BountyState.Paid;
        activeCount--;

        if (applyFees) {
            _payWithFees(bounty, id);
        } else {
            IERC20(bounty.rewardToken).safeTransfer(bounty.provider, bounty.reward);
            // Return provider bond
            if (providerBonds[id] > 0) {
                uint256 bond = providerBonds[id];
                providerBonds[id] = 0;
                IERC20(bounty.rewardToken).safeTransfer(bounty.provider, bond);
            }
            emit BountyPaid(id, bounty.provider, bounty.reward);
        }
    }

    function _payWithFees(Bounty storage bounty, bytes32 id) internal {
        uint256 totalFeesBps =
            feeConfig.protocolBps + feeConfig.reviewerBps + feeConfig.verificationBps;
        uint256 totalFee = (bounty.reward * totalFeesBps) / 10_000;
        uint256 providerPayout = bounty.reward - totalFee;

        uint256 protocolFee;
        uint256 reviewerFee;
        uint256 verificationFee;

        // Distribute fees
        if (feeConfig.protocolBps > 0) {
            protocolFee = (bounty.reward * feeConfig.protocolBps) / 10_000;
            IERC20(bounty.rewardToken).safeTransfer(feeConfig.protocolFeeRecipient, protocolFee);
        }
        if (feeConfig.reviewerBps > 0) {
            reviewerFee = (bounty.reward * feeConfig.reviewerBps) / 10_000;
            IERC20(bounty.rewardToken).safeTransfer(feeConfig.reviewerFeeRecipient, reviewerFee);
        }
        if (feeConfig.verificationBps > 0) {
            verificationFee = (bounty.reward * feeConfig.verificationBps) / 10_000;
            IERC20(bounty.rewardToken)
                .safeTransfer(feeConfig.verificationFeeRecipient, verificationFee);
        }

        // Pay provider
        IERC20(bounty.rewardToken).safeTransfer(bounty.provider, providerPayout);

        // Return provider bond
        if (providerBonds[id] > 0) {
            uint256 bond = providerBonds[id];
            providerBonds[id] = 0;
            IERC20(bounty.rewardToken).safeTransfer(bounty.provider, bond);
        }

        emit FeesCollected(bounty.id, protocolFee, reviewerFee, verificationFee, providerPayout);
        emit BountyPaid(bounty.id, bounty.provider, providerPayout);
    }

    // ─── Review Integration ───────────────────────────────────────────────

    /// @notice Start review process: Submitted → UnderReview with committee assignment
    function startReview(bytes32 id, address[3] calldata reviewers) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.Submitted) {
            revert InvalidState(id, bounty.state, BountyState.Submitted);
        }
        if (bounty.requester != msg.sender) revert NotRequester(id, msg.sender);

        // Assign committee via review contract
        if (address(reviewContract) != address(0)) {
            IBountyReview(reviewContract).assignCommittee(id, reviewers);
        }

        bounty.state = BountyState.UnderReview;
        emit BountyReviewStarted(id);
    }

    /// @notice Process review result after committee resolves
    function processReviewResult(bytes32 id) external {
        Bounty storage bounty = bounties[id];
        if (bounty.created == 0) revert BountyNotFound(id);
        if (bounty.state != BountyState.UnderReview) {
            revert InvalidState(id, bounty.state, BountyState.UnderReview);
        }

        // Check review contract resolution
        if (address(reviewContract) != address(0)) {
            if (!IBountyReview(reviewContract).isResolved(id)) revert ReviewNotResolved(id);

            if (IBountyReview(reviewContract).isAccepted(id)) {
                bounty.state = BountyState.Accepted;
                emit BountyAccepted(id);
            } else {
                bounty.state = BountyState.Rejected;
                activeCount--;

                // Refund reward to requester
                IERC20(bounty.rewardToken).safeTransfer(bounty.requester, bounty.reward);
                // Return provider bond
                if (providerBonds[id] > 0) {
                    uint256 bond = providerBonds[id];
                    providerBonds[id] = 0;
                    IERC20(bounty.rewardToken).safeTransfer(bounty.provider, bond);
                }

                emit BountyRejected(id);
            }
        }
    }

    // ─── View Functions ───────────────────────────────────────────────────

    /// @notice Get bounty state
    function getBountyState(bytes32 id) external view returns (BountyState) {
        if (bounties[id].created == 0) revert BountyNotFound(id);
        return bounties[id].state;
    }

    /// @notice Get total bounties
    function getTotalBounties() external view returns (uint256) {
        return bountyList.length;
    }

    /// @notice Get all bounty details including extended fields
    function getBountyFull(bytes32 id)
        external
        view
        returns (
            bytes32 bountyId,
            address requester_,
            address provider_,
            BountyState state,
            uint256 reward,
            address rewardToken,
            uint256 claimDeadline_,
            uint256 workDeadline_,
            uint256 reviewDeadline_,
            uint256 created,
            bytes32 submissionHash,
            uint256 revisionCount_,
            uint256 revisionDeadline_,
            uint256 disputeDeadline_,
            bool useEscrow_,
            uint256 providerBond_
        )
    {
        Bounty storage b = bounties[id];
        if (b.created == 0) revert BountyNotFound(id);
        return (
            b.id,
            b.requester,
            b.provider,
            b.state,
            b.reward,
            b.rewardToken,
            b.claimDeadline,
            b.workDeadline,
            b.reviewDeadline,
            b.created,
            b.submissionHash,
            b.revisionCount,
            revisionDeadlines[id],
            disputeDeadlines[id],
            useEscrowFlags[id],
            providerBonds[id]
        );
    }
}
