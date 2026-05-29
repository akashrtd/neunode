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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, Bytes, U256};
    use alloy::primitives::{address, fixed_bytes};
    use alloy::sol_types::{SolError, SolEvent};

    // ─── Review struct tests ────────────────────────────────────────────────

    #[test]
    fn review_construction() {
        let review = Review {
            reviewer: address!("0000000000000000000000000000000000000001"),
            score: 75u8,
            feedback: "Good work, minor issues".to_string(),
            signature: Bytes::from_static(&[1, 2, 3, 4]),
        };
        assert_eq!(review.score, 75);
        assert_eq!(review.feedback, "Good work, minor issues");
        assert_eq!(review.reviewer, address!("0000000000000000000000000000000000000001"));
        assert_eq!(review.signature.len(), 4);
    }

    #[test]
    fn review_with_high_score() {
        let review = Review {
            reviewer: address!("0000000000000000000000000000000000000001"),
            score: 100u8,
            feedback: String::new(),
            signature: Bytes::new(),
        };
        assert_eq!(review.score, 100);
        assert!(review.feedback.is_empty());
    }

    #[test]
    fn review_with_low_score() {
        let review = Review {
            reviewer: address!("0000000000000000000000000000000000000001"),
            score: 10u8,
            feedback: "Insufficient quality".to_string(),
            signature: Bytes::new(),
        };
        assert_eq!(review.score, 10);
    }

    // ─── ReviewCommittee struct tests ───────────────────────────────────────

    #[test]
    fn review_committee_construction() {
        let committee = ReviewCommittee {
            reviewers: [
                address!("0000000000000000000000000000000000000001"),
                address!("0000000000000000000000000000000000000002"),
                address!("0000000000000000000000000000000000000003"),
            ],
            acceptCount: 0,
            rejectCount: 0,
            resolved: false,
            assigned: true,
        };
        assert!(committee.assigned);
        assert!(!committee.resolved);
        assert_eq!(committee.acceptCount, 0);
        assert_eq!(committee.rejectCount, 0);
        assert_eq!(committee.reviewers.len(), 3);
    }

    #[test]
    fn review_committee_resolved_accepted() {
        // 2-of-3 accepted (score >= 50)
        let committee = ReviewCommittee {
            reviewers: [
                address!("0000000000000000000000000000000000000001"),
                address!("0000000000000000000000000000000000000002"),
                address!("0000000000000000000000000000000000000003"),
            ],
            acceptCount: 2,
            rejectCount: 1,
            resolved: true,
            assigned: true,
        };
        assert!(committee.resolved);
        assert_eq!(committee.acceptCount, 2);
        assert!(committee.acceptCount >= 2); // 2-of-3 threshold
    }

    #[test]
    fn review_committee_resolved_rejected() {
        // 2-of-3 rejected
        let committee = ReviewCommittee {
            reviewers: [
                address!("0000000000000000000000000000000000000001"),
                address!("0000000000000000000000000000000000000002"),
                address!("0000000000000000000000000000000000000003"),
            ],
            acceptCount: 0,
            rejectCount: 2,
            resolved: true,
            assigned: true,
        };
        assert!(committee.resolved);
        assert_eq!(committee.rejectCount, 2);
        assert!(committee.rejectCount >= 2);
    }

    #[test]
    fn review_committee_unassigned() {
        let committee = ReviewCommittee {
            reviewers: [Address::ZERO; 3],
            acceptCount: 0,
            rejectCount: 0,
            resolved: false,
            assigned: false,
        };
        assert!(!committee.assigned);
        for reviewer in &committee.reviewers {
            assert_eq!(*reviewer, Address::ZERO);
        }
    }

    // ─── Event signature tests ──────────────────────────────────────────────

    #[test]
    fn event_signatures_non_empty() {
        assert!(!ReviewSubmitted::SIGNATURE.is_empty());
        assert!(!CommitteeAssigned::SIGNATURE.is_empty());
        assert!(!ReviewCompleted::SIGNATURE.is_empty());
    }

    #[test]
    fn event_signatures_expected_format() {
        assert!(ReviewSubmitted::SIGNATURE.starts_with("ReviewSubmitted("));
        assert!(CommitteeAssigned::SIGNATURE.starts_with("CommitteeAssigned("));
        assert!(ReviewCompleted::SIGNATURE.starts_with("ReviewCompleted("));
    }

    #[test]
    fn event_selectors_are_32_bytes() {
        assert_eq!(ReviewSubmitted::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(CommitteeAssigned::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(ReviewCompleted::SIGNATURE_HASH.as_slice().len(), 32);
    }

    #[test]
    fn event_selectors_unique() {
        let selectors = [
            ReviewSubmitted::SIGNATURE_HASH,
            CommitteeAssigned::SIGNATURE_HASH,
            ReviewCompleted::SIGNATURE_HASH,
        ];
        for i in 0..selectors.len() {
            for j in (i + 1)..selectors.len() {
                assert_ne!(
                    selectors[i], selectors[j],
                    "Review event selectors must be unique"
                );
            }
        }
    }

    // ─── Error construction tests ───────────────────────────────────────────

    #[test]
    fn error_types_constructible() {
        let bounty_id = fixed_bytes!("0000000000000000000000000000000000000000000000000000000000000001");
        let addr = address!("0000000000000000000000000000000000000001");

        let _ = CommitteeAlreadyAssigned { bountyId: bounty_id };
        let _ = ZeroAddressReviewer {};
        let _ = DuplicateReviewer {};
        let _ = CommitteeNotAssigned { bountyId: bounty_id };
        let _ = CommitteeAlreadyResolved { bountyId: bounty_id };
        let _ = AlreadyReviewed {
            bountyId: bounty_id,
            reviewer: addr,
        };
        let _ = NotReviewer {
            bountyId: bounty_id,
            caller: addr,
        };
        let _ = InvalidSignature {
            expected: addr,
            actual: address!("0000000000000000000000000000000000000002"),
        };
        let _ = IndexOutOfBounds {
            index: U256::from(5),
            length: U256::from(3),
        };
    }

    #[test]
    fn error_selectors_are_4_bytes() {
        assert_eq!(CommitteeAlreadyAssigned::SELECTOR.len(), 4);
        assert_eq!(ZeroAddressReviewer::SELECTOR.len(), 4);
        assert_eq!(DuplicateReviewer::SELECTOR.len(), 4);
        assert_eq!(CommitteeNotAssigned::SELECTOR.len(), 4);
        assert_eq!(CommitteeAlreadyResolved::SELECTOR.len(), 4);
        assert_eq!(AlreadyReviewed::SELECTOR.len(), 4);
        assert_eq!(NotReviewer::SELECTOR.len(), 4);
        assert_eq!(InvalidSignature::SELECTOR.len(), 4);
        assert_eq!(IndexOutOfBounds::SELECTOR.len(), 4);
    }

    // ─── Review scoring tests ───────────────────────────────────────────────

    #[test]
    fn score_threshold() {
        // Score >= 50 means accepted
        let accepted_scores: [u8; 3] = [50, 75, 100];
        for score in accepted_scores {
            assert!(score >= 50, "Score {score} should be >= 50 (accepted)");
        }

        let rejected_scores: [u8; 3] = [0, 25, 49];
        for score in rejected_scores {
            assert!(score < 50, "Score {score} should be < 50 (rejected)");
        }
    }
}
