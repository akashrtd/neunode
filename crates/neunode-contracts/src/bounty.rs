//! NeunodeBounty contract bindings.
//!
//! Bounty state machine for agent work coordination.
//! Full lifecycle: Open -> Claimed -> Submitted -> UnderReview -> Accepted/Rejected/Disputed
//! -> Paid/Expired/Cancelled.

use alloy::sol;

sol! {
    // ─── Enums ────────────────────────────────────────────────────────────

    #[derive(Debug, PartialEq, Eq)]
    enum BountyState {
        Open,
        Claimed,
        Submitted,
        UnderReview,
        Revision,
        Accepted,
        Rejected,
        Disputed,
        Paid,
        Expired,
        Cancelled
    }

    // ─── Structs ──────────────────────────────────────────────────────────

    #[derive(Debug, PartialEq)]
    struct FeeConfig {
        uint256 protocolBps;
        uint256 reviewerBps;
        uint256 verificationBps;
        address protocolFeeRecipient;
        address reviewerFeeRecipient;
        address verificationFeeRecipient;
    }

    #[derive(Debug, PartialEq)]
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

    // ─── Events ───────────────────────────────────────────────────────────

    #[derive(Debug)]
    event BountyCreated(bytes32 indexed id, address indexed requester, uint256 reward, address rewardToken);

    #[derive(Debug)]
    event BountyClaimed(bytes32 indexed id, address indexed provider);

    #[derive(Debug)]
    event BountySubmitted(bytes32 indexed id, bytes32 commitment);

    #[derive(Debug)]
    event WorkRevealed(bytes32 indexed id, bytes32 submissionHash);

    #[derive(Debug)]
    event BountyReviewStarted(bytes32 indexed id);

    #[derive(Debug)]
    event BountyRevisionRequested(bytes32 indexed id);

    #[derive(Debug)]
    event BountyAccepted(bytes32 indexed id);

    #[derive(Debug)]
    event BountyRejected(bytes32 indexed id);

    #[derive(Debug)]
    event BountyDisputed(bytes32 indexed id);

    #[derive(Debug)]
    event BountyPaid(bytes32 indexed id, address indexed provider, uint256 amount);

    #[derive(Debug)]
    event BountyCancelled(bytes32 indexed id);

    #[derive(Debug)]
    event BountyExpired(bytes32 indexed id);

    #[derive(Debug)]
    event FeeConfigUpdated(address indexed admin);

    #[derive(Debug)]
    event FeeConfigProposed(address indexed admin, uint256 executesAt);

    #[derive(Debug)]
    event FeeConfigCancelled(address indexed admin);

    #[derive(Debug)]
    event EscrowUpdated(address indexed escrow);

    #[derive(Debug)]
    event ReviewContractUpdated(address indexed reviewContract);

    #[derive(Debug)]
    event FeesCollected(
        bytes32 indexed bountyId,
        uint256 protocolFee,
        uint256 reviewerFee,
        uint256 verificationFee,
        uint256 providerPayout
    );

    #[derive(Debug)]
    event DisputeResolved(bytes32 indexed bountyId, bool accepted);

    #[derive(Debug)]
    event ClaimCommitted(address indexed claimer, bytes32 indexed bountyId);

    #[derive(Debug)]
    event ClaimRevealed(address indexed claimer, bytes32 indexed bountyId);

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
    error NoPendingFeeChange();
    error FeeChangeTimelockNotExpired(uint256 expiresAt);
    error AlreadyCommitted(bytes32 bountyId);
    error NotCommitted(bytes32 bountyId);
    error InvalidReveal(bytes32 bountyId);
    error AlreadyRevealed(bytes32 bountyId);
    error SubmissionNotRevealed(bytes32 bountyId);
    error NotSubmitter(bytes32 bountyId, address caller);

    // ─── Functions ────────────────────────────────────────────────────────

    // Admin
    function proposeFeeConfig(
        uint256 protocolBps,
        uint256 reviewerBps,
        uint256 verificationBps,
        address protocolFeeRecipient,
        address reviewerFeeRecipient,
        address verificationFeeRecipient
    ) external;

    function executeFeeConfig() external;
    function cancelFeeConfigProposal() external;
    function setEscrow(address escrow_) external;
    function setReviewContract(address review_) external;

    // Create
    function createBounty(
        bytes32 id,
        uint256 reward,
        address rewardToken,
        uint256 claimDeadline,
        uint256 workDeadline
    ) external;

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
    ) external;

    // Claim
    function claimBounty(bytes32 id) external;
    function claimBountyWithBond(bytes32 id, uint256 bondAmount) external;
    function commitClaim(bytes32 bountyId, bytes32 commitment) external;
    function expireCommitment(address claimer, bytes32 bountyId) external;
    function revealClaim(bytes32 bountyId, uint256 bondAmount, bytes32 nonce) external;

    // Submit
    function submitWork(bytes32 id, bytes32 commitment) external;
    function revealWork(bytes32 id, bytes32 artifactHash, bytes32 salt) external;

    // Accept / Reject / Dispute
    function acceptSubmission(bytes32 id) external;
    function rejectSubmission(bytes32 id) external;
    function disputeBounty(bytes32 id) external;
    function resolveDispute(bytes32 id, bool accept) external;

    // Cancel / Expiry / Revision
    function cancelBounty(bytes32 id) external;
    function checkExpiry(bytes32 id) external;
    function requestRevision(bytes32 id) external;

    // Pay
    function payBounty(bytes32 id) external;
    function payBountyWithFees(bytes32 id) external;

    // Review integration
    function startReview(bytes32 id, address[3] calldata reviewers) external;
    function processReviewResult(bytes32 id) external;

    // View
    function getBountyState(bytes32 id) external view returns (BountyState);
    function getTotalBounties() external view returns (uint256);
    function getBountyFull(bytes32 id) external view returns (
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
    );
}
