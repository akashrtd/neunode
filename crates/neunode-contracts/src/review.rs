//! BountyReview contract bindings.
//!
//! 2-of-3 review committee for bounty submissions. Each bounty gets a 3-member
//! committee. 2 accept (score >= 50) = accepted. 2 reject = rejected.
//! EIP-712 signed reviews for off-chain verifiability.

use alloy::sol;

sol! {
    // ─── Structs ──────────────────────────────────────────────────────────

    #[derive(Debug, PartialEq)]
    struct Review {
        address reviewer;
        uint8 score;
        string feedback;
        bytes signature;
    }

    #[derive(Debug, PartialEq)]
    struct ReviewCommittee {
        address[3] reviewers;
        uint8 acceptCount;
        uint8 rejectCount;
        bool resolved;
        bool assigned;
    }

    // ─── Events ───────────────────────────────────────────────────────────

    #[derive(Debug)]
    event ReviewSubmitted(bytes32 indexed bountyId, address indexed reviewer, uint8 score, bool accepted);

    #[derive(Debug)]
    event CommitteeAssigned(bytes32 indexed bountyId, address[3] reviewers);

    #[derive(Debug)]
    event ReviewCompleted(bytes32 indexed bountyId, bool accepted);

    // ─── Errors ───────────────────────────────────────────────────────────

    error CommitteeAlreadyAssigned(bytes32 bountyId);
    error ZeroAddressReviewer();
    error DuplicateReviewer();
    error CommitteeNotAssigned(bytes32 bountyId);
    error CommitteeAlreadyResolved(bytes32 bountyId);
    error AlreadyReviewed(bytes32 bountyId, address reviewer);
    error NotReviewer(bytes32 bountyId, address caller);
    error InvalidSignature(address expected, address actual);
    error IndexOutOfBounds(uint256 index, uint256 length);

    // ─── Functions ────────────────────────────────────────────────────────

    function assignCommittee(bytes32 bountyId, address[3] calldata reviewers) external;

    function submitReview(
        bytes32 bountyId,
        uint8 score,
        string calldata feedback,
        bytes calldata signature
    ) external;

    function isAccepted(bytes32 bountyId) external view returns (bool);
    function isResolved(bytes32 bountyId) external view returns (bool);
    function getReviewCount(bytes32 bountyId) external view returns (uint256);
    function getReview(bytes32 bountyId, uint256 index) external view returns (address reviewer, uint8 score, string memory feedback);

    function getCommittee(bytes32 bountyId) external view returns (
        address[3] memory reviewers,
        uint8 acceptCount,
        uint8 rejectCount,
        bool resolved,
        bool assigned
    );
}
