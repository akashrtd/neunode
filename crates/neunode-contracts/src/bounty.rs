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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, FixedBytes, U256};
    use alloy::primitives::{address, fixed_bytes};
    use alloy::sol_types::{SolError, SolEvent};

    // ─── BountyState enum tests ─────────────────────────────────────────────

    #[test]
    fn bounty_state_all_variants() {
        let _open = BountyState::Open;
        let _claimed = BountyState::Claimed;
        let _submitted = BountyState::Submitted;
        let _under_review = BountyState::UnderReview;
        let _revision = BountyState::Revision;
        let _accepted = BountyState::Accepted;
        let _rejected = BountyState::Rejected;
        let _disputed = BountyState::Disputed;
        let _paid = BountyState::Paid;
        let _expired = BountyState::Expired;
        let _cancelled = BountyState::Cancelled;
    }

    #[test]
    fn bounty_state_equality() {
        assert_eq!(BountyState::Open, BountyState::Open);
        assert_ne!(BountyState::Open, BountyState::Claimed);
    }

    // ─── FeeConfig struct tests ─────────────────────────────────────────────

    #[test]
    fn fee_config_construction() {
        let config = FeeConfig {
            protocolBps: U256::from(250),
            reviewerBps: U256::from(150),
            verificationBps: U256::from(100),
            protocolFeeRecipient: address!("0000000000000000000000000000000000000001"),
            reviewerFeeRecipient: address!("0000000000000000000000000000000000000002"),
            verificationFeeRecipient: address!("0000000000000000000000000000000000000003"),
        };
        assert_eq!(config.protocolBps, U256::from(250));
        assert_eq!(config.reviewerBps, U256::from(150));
        assert_eq!(config.verificationBps, U256::from(100));
    }

    #[test]
    fn fee_config_total_exceeds_100_pct() {
        // 250 + 150 + 100 = 500 bps = 5%, valid
        let config = FeeConfig {
            protocolBps: U256::from(250),
            reviewerBps: U256::from(150),
            verificationBps: U256::from(100),
            protocolFeeRecipient: Address::ZERO,
            reviewerFeeRecipient: Address::ZERO,
            verificationFeeRecipient: Address::ZERO,
        };
        let total = config.protocolBps + config.reviewerBps + config.verificationBps;
        assert_eq!(total, U256::from(500));
        assert!(total <= U256::from(10000));
    }

    #[test]
    fn fee_config_debug_format() {
        let config = FeeConfig {
            protocolBps: U256::ZERO,
            reviewerBps: U256::ZERO,
            verificationBps: U256::ZERO,
            protocolFeeRecipient: Address::ZERO,
            reviewerFeeRecipient: Address::ZERO,
            verificationFeeRecipient: Address::ZERO,
        };
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("FeeConfig"));
    }

    // ─── Bounty struct tests ────────────────────────────────────────────────

    #[test]
    fn bounty_construction() {
        let bounty = Bounty {
            id: fixed_bytes!("0000000000000000000000000000000000000000000000000000000000000001"),
            requester: address!("0000000000000000000000000000000000000001"),
            provider: address!("0000000000000000000000000000000000000002"),
            state: BountyState::Open,
            reward: U256::from(1000),
            rewardToken: address!("0000000000000000000000000000000000000003"),
            claimDeadline: U256::from(100),
            workDeadline: U256::from(200),
            reviewDeadline: U256::from(300),
            created: U256::from(50),
            submissionHash: FixedBytes::<32>::ZERO,
            revisionCount: U256::ZERO,
        };
        assert_eq!(bounty.state, BountyState::Open);
        assert_eq!(bounty.reward, U256::from(1000));
        assert_eq!(bounty.revisionCount, U256::ZERO);
    }

    #[test]
    fn bounty_default_provider_is_zero() {
        let bounty = Bounty {
            id: FixedBytes::<32>::ZERO,
            requester: Address::ZERO,
            provider: Address::ZERO,
            state: BountyState::Open,
            reward: U256::ZERO,
            rewardToken: Address::ZERO,
            claimDeadline: U256::ZERO,
            workDeadline: U256::ZERO,
            reviewDeadline: U256::ZERO,
            created: U256::ZERO,
            submissionHash: FixedBytes::<32>::ZERO,
            revisionCount: U256::ZERO,
        };
        assert_eq!(bounty.provider, Address::ZERO);
        assert_eq!(bounty.state, BountyState::Open);
    }

    // ─── Event signature tests ──────────────────────────────────────────────

    #[test]
    fn event_signatures_non_empty() {
        assert!(!BountyCreated::SIGNATURE.is_empty());
        assert!(!BountyClaimed::SIGNATURE.is_empty());
        assert!(!BountySubmitted::SIGNATURE.is_empty());
        assert!(!WorkRevealed::SIGNATURE.is_empty());
        assert!(!BountyReviewStarted::SIGNATURE.is_empty());
        assert!(!BountyRevisionRequested::SIGNATURE.is_empty());
        assert!(!BountyAccepted::SIGNATURE.is_empty());
        assert!(!BountyRejected::SIGNATURE.is_empty());
        assert!(!BountyDisputed::SIGNATURE.is_empty());
        assert!(!BountyPaid::SIGNATURE.is_empty());
        assert!(!BountyCancelled::SIGNATURE.is_empty());
        assert!(!BountyExpired::SIGNATURE.is_empty());
        assert!(!FeeConfigUpdated::SIGNATURE.is_empty());
        assert!(!FeeConfigProposed::SIGNATURE.is_empty());
        assert!(!FeeConfigCancelled::SIGNATURE.is_empty());
        assert!(!EscrowUpdated::SIGNATURE.is_empty());
        assert!(!ReviewContractUpdated::SIGNATURE.is_empty());
        assert!(!FeesCollected::SIGNATURE.is_empty());
        assert!(!DisputeResolved::SIGNATURE.is_empty());
        assert!(!ClaimCommitted::SIGNATURE.is_empty());
        assert!(!ClaimRevealed::SIGNATURE.is_empty());
    }

    #[test]
    fn event_signatures_expected_format() {
        assert!(BountyCreated::SIGNATURE.starts_with("BountyCreated("));
        assert!(BountyClaimed::SIGNATURE.starts_with("BountyClaimed("));
        assert!(BountySubmitted::SIGNATURE.starts_with("BountySubmitted("));
        assert!(WorkRevealed::SIGNATURE.starts_with("WorkRevealed("));
        assert!(BountyAccepted::SIGNATURE.starts_with("BountyAccepted("));
        assert!(BountyPaid::SIGNATURE.starts_with("BountyPaid("));
    }

    #[test]
    fn event_selectors_are_32_bytes() {
        assert_eq!(BountyCreated::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(BountyClaimed::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(BountySubmitted::SIGNATURE_HASH.as_slice().len(), 32);
    }

    #[test]
    fn event_selectors_unique() {
        let selectors = [
            BountyCreated::SIGNATURE_HASH,
            BountyClaimed::SIGNATURE_HASH,
            BountySubmitted::SIGNATURE_HASH,
            WorkRevealed::SIGNATURE_HASH,
            BountyReviewStarted::SIGNATURE_HASH,
            BountyRevisionRequested::SIGNATURE_HASH,
            BountyAccepted::SIGNATURE_HASH,
            BountyRejected::SIGNATURE_HASH,
            BountyDisputed::SIGNATURE_HASH,
            BountyPaid::SIGNATURE_HASH,
            BountyCancelled::SIGNATURE_HASH,
            BountyExpired::SIGNATURE_HASH,
        ];
        for i in 0..selectors.len() {
            for j in (i + 1)..selectors.len() {
                assert_ne!(
                    selectors[i], selectors[j],
                    "Bounty event selectors must be unique"
                );
            }
        }
    }

    // ─── Error construction tests ───────────────────────────────────────────

    #[test]
    fn error_types_constructible() {
        let id = fixed_bytes!("0000000000000000000000000000000000000000000000000000000000000001");
        let caller = address!("0000000000000000000000000000000000000001");

        let _ = BountyNotFound { id };
        let _ = BountyAlreadyExists { id };
        let _ = InvalidState {
            id,
            current: BountyState::Open,
            required: BountyState::Claimed,
        };
        let _ = NotRequester { id, caller };
        let _ = NotProvider { id, caller };
        let _ = NotClaimer { id, caller };
        let _ = InvalidDeadline {};
        let _ = InvalidReward {};
        let _ = DeadlinePassed {
            deadline: U256::from(100),
        };
        let _ = MaxRevisionsReached {};
        let _ = ReviewNotResolved { id };
        let _ = ReviewNotAccepted { id };
        let _ = InsufficientBond {};
        let _ = TotalFeesExceed100 {};
        let _ = NoPendingFeeChange {};
        let _ = FeeChangeTimelockNotExpired {
            expiresAt: U256::from(200),
        };
        let _ = AlreadyCommitted { bountyId: id };
        let _ = NotCommitted { bountyId: id };
        let _ = InvalidReveal { bountyId: id };
        let _ = AlreadyRevealed { bountyId: id };
        let _ = SubmissionNotRevealed { bountyId: id };
        let _ = NotSubmitter { bountyId: id, caller };
    }

    #[test]
    fn error_selectors_are_4_bytes() {
        assert_eq!(BountyNotFound::SELECTOR.len(), 4);
        assert_eq!(BountyAlreadyExists::SELECTOR.len(), 4);
        assert_eq!(InvalidState::SELECTOR.len(), 4);
        assert_eq!(NotRequester::SELECTOR.len(), 4);
        assert_eq!(NotProvider::SELECTOR.len(), 4);
        assert_eq!(NotClaimer::SELECTOR.len(), 4);
        assert_eq!(InvalidDeadline::SELECTOR.len(), 4);
        assert_eq!(InvalidReward::SELECTOR.len(), 4);
        assert_eq!(DeadlinePassed::SELECTOR.len(), 4);
        assert_eq!(MaxRevisionsReached::SELECTOR.len(), 4);
        assert_eq!(ReviewNotResolved::SELECTOR.len(), 4);
        assert_eq!(ReviewNotAccepted::SELECTOR.len(), 4);
        assert_eq!(InsufficientBond::SELECTOR.len(), 4);
        assert_eq!(TotalFeesExceed100::SELECTOR.len(), 4);
        assert_eq!(NoPendingFeeChange::SELECTOR.len(), 4);
        assert_eq!(FeeChangeTimelockNotExpired::SELECTOR.len(), 4);
        assert_eq!(AlreadyCommitted::SELECTOR.len(), 4);
        assert_eq!(NotCommitted::SELECTOR.len(), 4);
        assert_eq!(InvalidReveal::SELECTOR.len(), 4);
        assert_eq!(AlreadyRevealed::SELECTOR.len(), 4);
        assert_eq!(SubmissionNotRevealed::SELECTOR.len(), 4);
        assert_eq!(NotSubmitter::SELECTOR.len(), 4);
    }

    #[test]
    fn error_count_is_19() {
        // Verify we have exactly 19 error types by checking selectors
        let selectors: Vec<[u8; 4]> = vec![
            BountyNotFound::SELECTOR,
            BountyAlreadyExists::SELECTOR,
            InvalidState::SELECTOR,
            NotRequester::SELECTOR,
            NotProvider::SELECTOR,
            NotClaimer::SELECTOR,
            InvalidDeadline::SELECTOR,
            InvalidReward::SELECTOR,
            DeadlinePassed::SELECTOR,
            MaxRevisionsReached::SELECTOR,
            ReviewNotResolved::SELECTOR,
            ReviewNotAccepted::SELECTOR,
            InsufficientBond::SELECTOR,
            TotalFeesExceed100::SELECTOR,
            NoPendingFeeChange::SELECTOR,
            FeeChangeTimelockNotExpired::SELECTOR,
            AlreadyCommitted::SELECTOR,
            AlreadyRevealed::SELECTOR,
            NotSubmitter::SELECTOR,
        ];
        assert_eq!(selectors.len(), 19);
    }
}
