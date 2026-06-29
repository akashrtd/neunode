// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "./IBountyReview.sol";

/// @title BountyReview — 2-of-3 review committee for bounty submissions
/// @notice Each bounty gets a 3-member committee. 2 accept (score >= 50) = accepted.
///         2 reject = rejected. EIP-712 signed reviews for off-chain verifiability.
contract BountyReview is IBountyReview, AccessControl, EIP712 {
    using ECDSA for bytes32;

    // ─── Types ────────────────────────────────────────────────────────────

    struct Review {
        address reviewer;
        uint8 score; // 0-100
        string feedback;
        bytes signature;
    }

    struct ReviewCommittee {
        address[3] reviewers;
        uint8 acceptCount;
        uint8 rejectCount;
        bool resolved;
        bool assigned;
    }

    // ─── Storage ──────────────────────────────────────────────────────────

    mapping(bytes32 => ReviewCommittee) public committees;
    mapping(bytes32 => mapping(address => bool)) public hasReviewed;
    mapping(bytes32 => Review[]) public reviews;

    bytes32 public constant REVIEWER_ROLE = keccak256("REVIEWER_ROLE");

    /// @notice EIP-712 type hash for review attestation
    bytes32 public constant REVIEW_TYPEHASH =
        keccak256("BountyReview(bytes32 bountyId,uint8 score,string feedback,uint256 nonce)");

    mapping(address => uint256) public nonces;

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

    // ─── Events ───────────────────────────────────────────────────────────

    event ReviewSubmitted(
        bytes32 indexed bountyId, address indexed reviewer, uint8 score, bool accepted
    );
    event CommitteeAssigned(bytes32 indexed bountyId, address[3] reviewers);
    event ReviewCompleted(bytes32 indexed bountyId, bool accepted);

    // ─── Constructor ──────────────────────────────────────────────────────

    constructor() EIP712("NeunodeBountyReview", "1") {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
    }

    // ─── Functions ────────────────────────────────────────────────────────

    /// @notice Assign a 3-member review committee to a bounty
    function assignCommittee(bytes32 bountyId, address[3] calldata reviewers)
        external
        onlyRole(DEFAULT_ADMIN_ROLE)
    {
        if (committees[bountyId].assigned) revert CommitteeAlreadyAssigned(bountyId);
        if (reviewers[0] == address(0) || reviewers[1] == address(0) || reviewers[2] == address(0)) revert ZeroAddressReviewer();
        if (
            reviewers[0] == reviewers[1] || reviewers[1] == reviewers[2]
                || reviewers[0] == reviewers[2]
        ) revert DuplicateReviewer();

        committees[bountyId] = ReviewCommittee({
            reviewers: reviewers, acceptCount: 0, rejectCount: 0, resolved: false, assigned: true
        });

        emit CommitteeAssigned(bountyId, reviewers);
    }

    /// @notice Submit a review with EIP-712 signature
    function submitReview(
        bytes32 bountyId,
        uint8 score,
        string calldata feedback,
        bytes calldata signature
    ) external {
        ReviewCommittee storage committee = committees[bountyId];
        if (!committee.assigned) revert CommitteeNotAssigned(bountyId);
        if (committee.resolved) revert CommitteeAlreadyResolved(bountyId);
        if (hasReviewed[bountyId][msg.sender]) revert AlreadyReviewed(bountyId, msg.sender);

        // Verify reviewer is on the committee
        bool isReviewer = false;
        for (uint256 i = 0; i < 3; i++) {
            if (committee.reviewers[i] == msg.sender) {
                isReviewer = true;
                break;
            }
        }
        if (!isReviewer) revert NotReviewer(bountyId, msg.sender);

        // Verify EIP-712 signature
        bytes32 structHash = keccak256(
            abi.encode(
                REVIEW_TYPEHASH, bountyId, score, keccak256(bytes(feedback)), nonces[msg.sender]
            )
        );
        bytes32 hash = _hashTypedDataV4(structHash);
        address signer = hash.recover(signature);
        if (signer != msg.sender) revert InvalidSignature(msg.sender, signer);

        nonces[msg.sender]++;
        hasReviewed[bountyId][msg.sender] = true;

        bool accepted = score >= 50;
        if (accepted) {
            committee.acceptCount++;
        } else {
            committee.rejectCount++;
        }

        reviews[bountyId].push(
            Review({reviewer: msg.sender, score: score, feedback: feedback, signature: signature})
        );

        emit ReviewSubmitted(bountyId, msg.sender, score, accepted);

        // Check if 2-of-3 reached
        if (committee.acceptCount >= 2 || committee.rejectCount >= 2) {
            committee.resolved = true;
            emit ReviewCompleted(bountyId, committee.acceptCount >= 2);
        }
    }

    /// @notice Check if the committee accepted the bounty
    function isAccepted(bytes32 bountyId) external view returns (bool) {
        return committees[bountyId].acceptCount >= 2;
    }

    /// @notice Check if the committee has resolved (2-of-3 reached)
    function isResolved(bytes32 bountyId) external view returns (bool) {
        return committees[bountyId].resolved;
    }

    /// @notice Get review count for a bounty
    function getReviewCount(bytes32 bountyId) external view returns (uint256) {
        return reviews[bountyId].length;
    }

    /// @notice Get a specific review
    function getReview(bytes32 bountyId, uint256 index)
        external
        view
        returns (address reviewer, uint8 score, string memory feedback)
    {
        if (index >= reviews[bountyId].length) {
            revert IndexOutOfBounds(index, reviews[bountyId].length);
        }
        Review storage r = reviews[bountyId][index];
        return (r.reviewer, r.score, r.feedback);
    }

    /// @notice Get committee info for a bounty
    function getCommittee(bytes32 bountyId)
        external
        view
        returns (
            address[3] memory reviewers,
            uint8 acceptCount,
            uint8 rejectCount,
            bool resolved,
            bool assigned
        )
    {
        ReviewCommittee storage c = committees[bountyId];
        return (c.reviewers, c.acceptCount, c.rejectCount, c.resolved, c.assigned);
    }
}
