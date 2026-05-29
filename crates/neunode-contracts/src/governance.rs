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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, Bytes, FixedBytes, U256};
    use alloy::primitives::{address, fixed_bytes};
    use alloy::sol_types::{SolError, SolEvent};

    // ─── ProposalState enum tests ───────────────────────────────────────────

    #[test]
    fn proposal_state_all_variants() {
        let _pending = ProposalState::Pending;
        let _active = ProposalState::Active;
        let _succeeded = ProposalState::Succeeded;
        let _defeated = ProposalState::Defeated;
        let _queued = ProposalState::Queued;
        let _executed = ProposalState::Executed;
        let _expired = ProposalState::Expired;
        let _cancelled = ProposalState::Cancelled;
    }

    #[test]
    fn proposal_state_equality() {
        assert_eq!(ProposalState::Pending, ProposalState::Pending);
        assert_ne!(ProposalState::Pending, ProposalState::Active);
        assert_ne!(ProposalState::Succeeded, ProposalState::Defeated);
        assert_ne!(ProposalState::Queued, ProposalState::Executed);
        assert_ne!(ProposalState::Expired, ProposalState::Cancelled);
    }

    // ─── VoteType enum tests ────────────────────────────────────────────────

    #[test]
    fn vote_type_all_variants() {
        let _against = VoteType::Against;
        let _for = VoteType::For;
        let _abstain = VoteType::Abstain;
    }

    #[test]
    fn vote_type_equality() {
        assert_eq!(VoteType::Against, VoteType::Against);
        assert_ne!(VoteType::Against, VoteType::For);
        assert_ne!(VoteType::For, VoteType::Abstain);
    }

    // ─── Proposal struct tests ──────────────────────────────────────────────

    #[test]
    fn proposal_construction() {
        let proposal = Proposal {
            id: U256::from(1),
            proposer: address!("0000000000000000000000000000000000000001"),
            targets: vec![address!("0000000000000000000000000000000000000002")],
            values: vec![U256::ZERO],
            calldatas: vec![Bytes::new()],
            descriptionHash: fixed_bytes!(
                "0000000000000000000000000000000000000000000000000000000000000001"
            ),
            voteStart: U256::from(100),
            voteEnd: U256::from(200),
            forVotes: U256::from(60),
            againstVotes: U256::from(30),
            abstainVotes: U256::from(10),
            snapshotBlock: U256::from(50),
            executed: false,
            cancelled: false,
            queuedAt: U256::ZERO,
        };
        assert_eq!(proposal.id, U256::from(1));
        assert!(!proposal.executed);
        assert!(!proposal.cancelled);
        assert_eq!(proposal.targets.len(), 1);
    }

    #[test]
    fn proposal_with_multiple_actions() {
        let proposal = Proposal {
            id: U256::from(2),
            proposer: Address::ZERO,
            targets: vec![
                address!("0000000000000000000000000000000000000001"),
                address!("0000000000000000000000000000000000000002"),
                address!("0000000000000000000000000000000000000003"),
            ],
            values: vec![U256::ZERO, U256::from(1), U256::ZERO],
            calldatas: vec![Bytes::new(), Bytes::new(), Bytes::new()],
            descriptionHash: FixedBytes::<32>::ZERO,
            voteStart: U256::ZERO,
            voteEnd: U256::ZERO,
            forVotes: U256::ZERO,
            againstVotes: U256::ZERO,
            abstainVotes: U256::ZERO,
            snapshotBlock: U256::ZERO,
            executed: false,
            cancelled: false,
            queuedAt: U256::ZERO,
        };
        assert_eq!(proposal.targets.len(), 3);
        assert_eq!(proposal.values.len(), 3);
        assert_eq!(proposal.calldatas.len(), 3);
    }

    // ─── Checkpoint struct tests ────────────────────────────────────────────

    #[test]
    fn checkpoint_construction() {
        let cp = Checkpoint { fromBlock: U256::from(1000), power: U256::from(500) };
        assert_eq!(cp.fromBlock, U256::from(1000));
        assert_eq!(cp.power, U256::from(500));
    }

    #[test]
    fn checkpoint_zero() {
        let cp = Checkpoint { fromBlock: U256::ZERO, power: U256::ZERO };
        assert_eq!(cp.fromBlock, U256::ZERO);
        assert_eq!(cp.power, U256::ZERO);
    }

    // ─── Event signature tests ──────────────────────────────────────────────

    #[test]
    fn event_signatures_non_empty() {
        assert!(!ProposalCreated::SIGNATURE.is_empty());
        assert!(!VoteCast::SIGNATURE.is_empty());
        assert!(!ProposalQueued::SIGNATURE.is_empty());
        assert!(!ProposalExecuted::SIGNATURE.is_empty());
        assert!(!ProposalCancelled::SIGNATURE.is_empty());
        assert!(!GovernanceParametersUpdated::SIGNATURE.is_empty());
        assert!(!AllowedTargetUpdated::SIGNATURE.is_empty());
    }

    #[test]
    fn event_signatures_expected_format() {
        assert!(ProposalCreated::SIGNATURE.starts_with("ProposalCreated("));
        assert!(VoteCast::SIGNATURE.starts_with("VoteCast("));
        assert!(ProposalQueued::SIGNATURE.starts_with("ProposalQueued("));
        assert!(ProposalExecuted::SIGNATURE.starts_with("ProposalExecuted("));
        assert!(ProposalCancelled::SIGNATURE.starts_with("ProposalCancelled("));
    }

    #[test]
    fn event_selectors_are_32_bytes() {
        assert_eq!(ProposalCreated::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(VoteCast::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(ProposalQueued::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(ProposalExecuted::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(ProposalCancelled::SIGNATURE_HASH.as_slice().len(), 32);
    }

    #[test]
    fn event_selectors_unique() {
        let selectors = [
            ProposalCreated::SIGNATURE_HASH,
            VoteCast::SIGNATURE_HASH,
            ProposalQueued::SIGNATURE_HASH,
            ProposalExecuted::SIGNATURE_HASH,
            ProposalCancelled::SIGNATURE_HASH,
            GovernanceParametersUpdated::SIGNATURE_HASH,
            AllowedTargetUpdated::SIGNATURE_HASH,
        ];
        for i in 0..selectors.len() {
            for j in (i + 1)..selectors.len() {
                assert_ne!(selectors[i], selectors[j], "Governance event selectors must be unique");
            }
        }
    }

    // ─── Error construction tests ───────────────────────────────────────────

    #[test]
    fn error_types_constructible() {
        let proposal_id = U256::from(1);
        let addr = address!("0000000000000000000000000000000000000001");

        let _ = ProposalNotFound { proposalId: proposal_id };
        let _ = ProposalNotActive { proposalId: proposal_id };
        let _ = AlreadyVoted { proposalId: proposal_id, voter: addr };
        let _ = VotingPowerZero { voter: addr };
        let _ = BelowProposalThreshold {
            proposer: addr,
            threshold: U256::from(100),
            actual: U256::from(50),
        };
        let _ = QuorumNotReached { proposalId: proposal_id };
        let _ = ProposalNotSucceeded { proposalId: proposal_id };
        let _ = ProposalNotQueued { proposalId: proposal_id };
        let _ = ProposalNotReady { proposalId: proposal_id };
        let _ = ProposalAlreadyExecuted { proposalId: proposal_id };
        let _ = ProposalAlreadyCancelled { proposalId: proposal_id };
        let _ = ProposalNotCancellable { proposalId: proposal_id };
        let _ = ArrayLengthMismatch {};
        let _ = EmptyProposal {};
        let _ = ZeroAddress {};
        let _ = ExecutionFailed { proposalId: proposal_id };
        let _ = NotAuthorized { caller: addr };
        let _ = TargetNotAllowed { target: addr };
    }

    #[test]
    fn error_selectors_are_4_bytes() {
        assert_eq!(ProposalNotFound::SELECTOR.len(), 4);
        assert_eq!(ProposalNotActive::SELECTOR.len(), 4);
        assert_eq!(AlreadyVoted::SELECTOR.len(), 4);
        assert_eq!(VotingPowerZero::SELECTOR.len(), 4);
        assert_eq!(BelowProposalThreshold::SELECTOR.len(), 4);
        assert_eq!(QuorumNotReached::SELECTOR.len(), 4);
        assert_eq!(ProposalNotSucceeded::SELECTOR.len(), 4);
        assert_eq!(ProposalNotQueued::SELECTOR.len(), 4);
        assert_eq!(ProposalNotReady::SELECTOR.len(), 4);
        assert_eq!(ProposalAlreadyExecuted::SELECTOR.len(), 4);
        assert_eq!(ProposalAlreadyCancelled::SELECTOR.len(), 4);
        assert_eq!(ProposalNotCancellable::SELECTOR.len(), 4);
        assert_eq!(ArrayLengthMismatch::SELECTOR.len(), 4);
        assert_eq!(EmptyProposal::SELECTOR.len(), 4);
        assert_eq!(ZeroAddress::SELECTOR.len(), 4);
        assert_eq!(ExecutionFailed::SELECTOR.len(), 4);
        assert_eq!(NotAuthorized::SELECTOR.len(), 4);
        assert_eq!(TargetNotAllowed::SELECTOR.len(), 4);
    }

    // ─── Vote casting parameter tests ───────────────────────────────────────

    #[test]
    fn vote_type_values() {
        // VoteType is used as u8 in castVote: 0=Against, 1=For, 2=Abstain
        assert_eq!(VoteType::Against as u8, 0);
        assert_eq!(VoteType::For as u8, 1);
        assert_eq!(VoteType::Abstain as u8, 2);
    }

    #[test]
    fn proposal_lifecycle_order() {
        // Pending -> Active -> Succeeded/Defeated -> Queued -> Executed/Expired
        // Cancelled from Pending
        let pending = ProposalState::Pending;
        let active = ProposalState::Active;
        assert_ne!(pending as u8, active as u8);
        assert!((pending as u8) < active as u8);

        let succeeded = ProposalState::Succeeded;
        let defeated = ProposalState::Defeated;
        assert_ne!(succeeded as u8, defeated as u8);

        let queued = ProposalState::Queued;
        let executed = ProposalState::Executed;
        let expired = ProposalState::Expired;
        assert!((queued as u8) < executed as u8);
        assert!((executed as u8) < expired as u8);
    }
}
