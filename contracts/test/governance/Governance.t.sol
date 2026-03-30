// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../../src/governance/NeunodeGovernance.sol";
import "../../src/tokens/ComputeToken.sol";

/// @title GovernanceTest — Comprehensive tests for NeunodeGovernance
contract GovernanceTest is Test {
    NeunodeGovernance public gov;
    ComputeToken public token;

    address public proposer;
    address public voter1;
    address public voter2;
    address public voter3;
    address public attacker;
    address public governanceAdmin;

    uint256 constant PROPOSAL_THRESHOLD = 100e18;
    uint256 constant VOTING_DELAY = 1 days;
    uint256 constant VOTING_PERIOD = 7 days;
    uint256 constant QUORUM_BPS = 400; // 4%
    uint256 constant TIMELOCK = 2 days;
    uint256 constant EXECUTION_WINDOW = 14 days;
    uint256 constant STAKE_AMOUNT = 10_000e18;

    TargetMock public target;

    function setUp() public {
        token = new ComputeToken();
        gov = new NeunodeGovernance(
            address(token),
            VOTING_DELAY,
            VOTING_PERIOD,
            PROPOSAL_THRESHOLD,
            QUORUM_BPS,
            TIMELOCK,
            EXECUTION_WINDOW
        );

        proposer = makeAddr("proposer");
        voter1 = makeAddr("voter1");
        voter2 = makeAddr("voter2");
        voter3 = makeAddr("voter3");
        attacker = makeAddr("attacker");
        governanceAdmin = makeAddr("governanceAdmin");

        target = new TargetMock();

        // Grant governanceAdmin the GOVERNANCE_ROLE
        gov.grantRole(gov.GOVERNANCE_ROLE(), governanceAdmin);

        // Mint tokens to test accounts
        token.mint(proposer, STAKE_AMOUNT);
        token.mint(voter1, STAKE_AMOUNT);
        token.mint(voter2, STAKE_AMOUNT);
        token.mint(voter3, STAKE_AMOUNT);

        // Stake tokens
        vm.prank(proposer);
        token.stake(STAKE_AMOUNT);
        vm.prank(voter1);
        token.stake(STAKE_AMOUNT);
        vm.prank(voter2);
        token.stake(STAKE_AMOUNT);
        vm.prank(voter3);
        token.stake(STAKE_AMOUNT);

        // Checkpoint voting power in governance contract
        vm.prank(proposer);
        gov.checkpoint();
        vm.prank(voter1);
        gov.checkpoint();
        vm.prank(voter2);
        gov.checkpoint();
        vm.prank(voter3);
        gov.checkpoint();
    }

    // ─── Helper: create a basic proposal ─────────────────────────────────

    function _createProposal() internal returns (uint256) {
        address[] memory targets = new address[](1);
        uint256[] memory values = new uint256[](1);
        bytes[] memory calldatas = new bytes[](1);
        targets[0] = address(target);
        values[0] = 0;
        calldatas[0] = abi.encodeCall(TargetMock.setValue, (42));

        vm.prank(proposer);
        return gov.propose(targets, values, calldatas, "Test Proposal");
    }

    function _createEmptyProposal() internal returns (uint256) {
        address[] memory targets = new address[](1);
        uint256[] memory values = new uint256[](1);
        bytes[] memory calldatas = new bytes[](1);
        targets[0] = address(target);
        values[0] = 0;
        calldatas[0] = "";

        vm.prank(proposer);
        return gov.propose(targets, values, calldatas, "Empty Call Proposal");
    }

    // ─── 1. Create Proposal with Valid Threshold ─────────────────────────

    function testCreateProposal() public {
        uint256 proposalId = _createProposal();

        assertEq(proposalId, 1);
        assertEq(gov.proposalCount(), 1);

        (
            address proposer_,
            uint256 voteStart,
            uint256 voteEnd,
            uint256 forVotes,
            uint256 againstVotes,
            uint256 abstainVotes,
            uint256 snapshotBlock,
            bool executed,
            bool cancelled,
            uint256 queuedAt
        ) = gov.getProposal(proposalId);

        assertEq(proposer_, proposer);
        assertEq(voteStart, block.timestamp + VOTING_DELAY);
        assertEq(voteEnd, block.timestamp + VOTING_DELAY + VOTING_PERIOD);
        assertEq(forVotes, 0);
        assertEq(againstVotes, 0);
        assertEq(abstainVotes, 0);
        assertEq(snapshotBlock, block.number);
        assertFalse(executed);
        assertFalse(cancelled);
        assertEq(queuedAt, 0);
    }

    // ─── 2. Cannot Create Proposal Below Threshold ───────────────────────

    function testRevertCreateProposalBelowThreshold() public {
        address poorUser = makeAddr("poorUser");
        token.mint(poorUser, 50e18);
        vm.prank(poorUser);
        token.stake(50e18);

        address[] memory targets = new address[](1);
        uint256[] memory values = new uint256[](1);
        bytes[] memory calldatas = new bytes[](1);
        targets[0] = address(target);
        values[0] = 0;
        calldatas[0] = "";

        vm.prank(poorUser);
        vm.expectRevert(
            abi.encodeWithSelector(
                NeunodeGovernance.BelowProposalThreshold.selector,
                poorUser,
                PROPOSAL_THRESHOLD,
                50e18
            )
        );
        gov.propose(targets, values, calldatas, "Below Threshold");
    }

    // ─── 3. Vote on Active Proposal — For ────────────────────────────────

    function testVoteFor() public {
        uint256 proposalId = _createProposal();

        // Advance past voting delay
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        vm.prank(voter1);
        uint256 weight = gov.castVote(proposalId, uint8(IGovernance.VoteType.For));

        assertEq(weight, STAKE_AMOUNT);

        (,,, uint256 forVotes, uint256 againstVotes, uint256 abstainVotes,,,,) =
            gov.getProposal(proposalId);

        assertEq(forVotes, STAKE_AMOUNT);
        assertEq(againstVotes, 0);
        assertEq(abstainVotes, 0);
        assertTrue(gov.hasVoted(proposalId, voter1));
    }

    // ─── 4. Vote on Active Proposal — Against ────────────────────────────

    function testVoteAgainst() public {
        uint256 proposalId = _createProposal();
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        vm.prank(voter1);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.Against));

        (,,, uint256 forVotes, uint256 againstVotes, uint256 abstainVotes,,,,) =
            gov.getProposal(proposalId);

        assertEq(forVotes, 0);
        assertEq(againstVotes, STAKE_AMOUNT);
        assertEq(abstainVotes, 0);
    }

    // ─── 5. Vote on Active Proposal — Abstain ────────────────────────────

    function testVoteAbstain() public {
        uint256 proposalId = _createProposal();
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        vm.prank(voter1);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.Abstain));

        (,,, uint256 forVotes, uint256 againstVotes, uint256 abstainVotes,,,,) =
            gov.getProposal(proposalId);

        assertEq(forVotes, 0);
        assertEq(againstVotes, 0);
        assertEq(abstainVotes, STAKE_AMOUNT);
    }

    // ─── 6. Cannot Vote Twice on Same Proposal ───────────────────────────

    function testRevertVoteTwice() public {
        uint256 proposalId = _createProposal();
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        vm.prank(voter1);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));

        vm.prank(voter1);
        vm.expectRevert(
            abi.encodeWithSelector(
                NeunodeGovernance.AlreadyVoted.selector, proposalId, voter1
            )
        );
        gov.castVote(proposalId, uint8(IGovernance.VoteType.Against));
    }

    // ─── 7. Cannot Vote on Pending Proposal ──────────────────────────────

    function testRevertVoteOnPendingProposal() public {
        uint256 proposalId = _createProposal();

        // Don't advance time — proposal is still Pending
        vm.prank(voter1);
        vm.expectRevert(
            abi.encodeWithSelector(
                NeunodeGovernance.ProposalNotActive.selector, proposalId
            )
        );
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));
    }

    // ─── 8. Proposal Succeeds with Enough For-Votes ──────────────────────

    function testProposalSucceeds() public {
        uint256 proposalId = _createProposal();
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        // voter1 and voter2 vote For
        vm.prank(voter1);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));
        vm.prank(voter2);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));

        // Advance past voting period
        vm.warp(block.timestamp + VOTING_PERIOD + 1);

        assertEq(
            uint8(gov.state(proposalId)),
            uint8(IGovernance.ProposalState.Succeeded)
        );
    }

    // ─── 9. Proposal Defeated with More Against-Votes ────────────────────

    function testProposalDefeated() public {
        uint256 proposalId = _createProposal();
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        // voter1 votes For, voter2 and voter3 vote Against
        vm.prank(voter1);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));
        vm.prank(voter2);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.Against));
        vm.prank(voter3);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.Against));

        vm.warp(block.timestamp + VOTING_PERIOD + 1);

        assertEq(
            uint8(gov.state(proposalId)),
            uint8(IGovernance.ProposalState.Defeated)
        );
    }

    // ─── 10. Quorum Check: Fails Even with Majority For ──────────────────

    function testQuorumNotReached() public {
        // Add a large staker to inflate total staked supply
        address bigStaker = makeAddr("bigStaker");
        token.mint(bigStaker, 1_000_000e18);
        vm.prank(bigStaker);
        token.stake(1_000_000e18);
        vm.prank(bigStaker);
        gov.checkpoint();

        // Total staked = 4 * 10,000 + 1,000,000 = 1,040,000
        // 4% quorum = 41,600
        // Only voter1 votes For = 10,000 < 41,600 → quorum fails

        uint256 proposalId = _createProposal();
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        vm.prank(voter1);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));

        vm.warp(block.timestamp + VOTING_PERIOD + 1);

        // Defeated because quorum not reached
        assertEq(
            uint8(gov.state(proposalId)),
            uint8(IGovernance.ProposalState.Defeated)
        );
    }

    // ─── 11. Execute Succeeded Proposal ──────────────────────────────────

    function testExecuteProposal() public {
        uint256 proposalId = _createProposal();
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        // Vote for
        vm.prank(voter1);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));
        vm.prank(voter2);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));

        // End voting
        vm.warp(block.timestamp + VOTING_PERIOD + 1);

        // Queue
        gov.queue(proposalId);

        // Wait for timelock
        vm.warp(block.timestamp + TIMELOCK + 1);

        // Execute
        gov.execute(proposalId);

        // Verify execution
        assertEq(
            uint8(gov.state(proposalId)),
            uint8(IGovernance.ProposalState.Executed)
        );
        // Verify target was called
        assertEq(target.value(), 42);
    }

    // ─── 12. Cannot Execute Non-Succeeded Proposal ───────────────────────

    function testRevertExecuteNonSucceeded() public {
        uint256 proposalId = _createProposal();

        // Still pending, not even active
        vm.expectRevert(
            abi.encodeWithSelector(
                NeunodeGovernance.ProposalNotQueued.selector, proposalId
            )
        );
        gov.execute(proposalId);
    }

    // ─── 13. Cancel Proposal Before Voting Starts ────────────────────────

    function testCancelProposal() public {
        uint256 proposalId = _createProposal();

        // Cancel while still Pending
        vm.prank(proposer);
        gov.cancel(proposalId);

        assertEq(
            uint8(gov.state(proposalId)),
            uint8(IGovernance.ProposalState.Cancelled)
        );
    }

    // ─── 14. Cannot Cancel After Voting Starts ───────────────────────────

    function testRevertCancelAfterVotingStarts() public {
        uint256 proposalId = _createProposal();

        // Advance past voting delay → Active
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        vm.prank(proposer);
        vm.expectRevert(
            abi.encodeWithSelector(
                NeunodeGovernance.ProposalNotCancellable.selector, proposalId
            )
        );
        gov.cancel(proposalId);
    }

    // ─── 15. Get State Returns Correct State at Each Lifecycle Phase ─────

    function testStateTransitions() public {
        uint256 proposalId = _createProposal();

        // Pending
        assertEq(
            uint8(gov.state(proposalId)),
            uint8(IGovernance.ProposalState.Pending)
        );

        // Active
        vm.warp(block.timestamp + VOTING_DELAY + 1);
        assertEq(
            uint8(gov.state(proposalId)),
            uint8(IGovernance.ProposalState.Active)
        );

        // Vote for
        vm.prank(voter1);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));

        // Succeeded
        vm.warp(block.timestamp + VOTING_PERIOD + 1);
        assertEq(
            uint8(gov.state(proposalId)),
            uint8(IGovernance.ProposalState.Succeeded)
        );

        // Queued
        gov.queue(proposalId);
        assertEq(
            uint8(gov.state(proposalId)),
            uint8(IGovernance.ProposalState.Queued)
        );

        // Wait for timelock
        vm.warp(block.timestamp + TIMELOCK + 1);

        // Executed
        gov.execute(proposalId);
        assertEq(
            uint8(gov.state(proposalId)),
            uint8(IGovernance.ProposalState.Executed)
        );
    }

    // ─── 16. Voting Power = Staked Balance at Snapshot Block ─────────────

    function testVotingPowerAtSnapshot() public {
        uint256 proposalId = _createProposal();
        (,,,,,, uint256 snapshotBlock,,,) = gov.getProposal(proposalId);

        // Voting power should equal staked balance at snapshot block
        assertEq(gov.getVotes(voter1, snapshotBlock), STAKE_AMOUNT);
        assertEq(gov.getVotes(voter2, snapshotBlock), STAKE_AMOUNT);
        assertEq(gov.getVotes(proposer, snapshotBlock), STAKE_AMOUNT);
    }

    // ─── 17. Multiple Voters — Verify Vote Tallying ──────────────────────

    function testMultipleVoters() public {
        uint256 proposalId = _createProposal();
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        vm.prank(voter1);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));
        vm.prank(voter2);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.Against));
        vm.prank(voter3);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.Abstain));

        (,,, uint256 forVotes, uint256 againstVotes, uint256 abstainVotes,,,,) =
            gov.getProposal(proposalId);

        assertEq(forVotes, STAKE_AMOUNT);
        assertEq(againstVotes, STAKE_AMOUNT);
        assertEq(abstainVotes, STAKE_AMOUNT);

        assertTrue(gov.hasVoted(proposalId, voter1));
        assertTrue(gov.hasVoted(proposalId, voter2));
        assertTrue(gov.hasVoted(proposalId, voter3));
        assertFalse(gov.hasVoted(proposalId, proposer));
    }

    // ─── 18. Abstain Votes Count Toward Quorum but Not For/Against ───────

    function testAbstainCountsTowardQuorum() public {
        // Setup: large staker to make quorum harder to reach
        address bigStaker = makeAddr("bigStaker2");
        token.mint(bigStaker, 500_000e18);
        vm.prank(bigStaker);
        token.stake(500_000e18);
        vm.prank(bigStaker);
        gov.checkpoint();

        // Total staked = 4 * 10,000 + 500,000 = 540,000
        // 4% quorum = 21,600
        // voter1 For = 10,000
        // voter2 Abstain = 10,000
        // Total votes = 20,000 < 21,600 → quorum fails without voter3
        // voter3 Abstain = 10,000
        // Total votes = 30,000 >= 21,600 → quorum met with abstain help

        uint256 proposalId = _createProposal();
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        vm.prank(voter1);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));
        vm.prank(voter2);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.Abstain));
        vm.prank(voter3);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.Abstain));

        vm.warp(block.timestamp + VOTING_PERIOD + 1);

        // Succeeded because for > against (10k > 0) and quorum met (30k >= 21.6k)
        assertEq(
            uint8(gov.state(proposalId)),
            uint8(IGovernance.ProposalState.Succeeded)
        );
    }

    // ─── 19. Proposal Expired If Not Executed Within Window ──────────────

    function testProposalExpired() public {
        uint256 proposalId = _createProposal();
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        vm.prank(voter1);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));
        vm.prank(voter2);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));

        vm.warp(block.timestamp + VOTING_PERIOD + 1);

        // Queue
        gov.queue(proposalId);

        // Wait past execution window
        vm.warp(block.timestamp + EXECUTION_WINDOW + 1);

        assertEq(
            uint8(gov.state(proposalId)),
            uint8(IGovernance.ProposalState.Expired)
        );
    }

    // ─── 20. Update Governance Parameters (GOVERNANCE_ROLE only) ─────────

    function testUpdateGovernanceParameters() public {
        vm.prank(governanceAdmin);
        gov.setVotingDelay(2 days);
        assertEq(gov.votingDelay(), 2 days);

        vm.prank(governanceAdmin);
        gov.setVotingPeriod(14 days);
        assertEq(gov.votingPeriod(), 14 days);

        vm.prank(governanceAdmin);
        gov.setProposalThreshold(200e18);
        assertEq(gov.proposalThreshold(), 200e18);

        vm.prank(governanceAdmin);
        gov.setQuorumBps(500);
        assertEq(gov.quorumBps(), 500);

        vm.prank(governanceAdmin);
        gov.setTimelock(3 days);
        assertEq(gov.timelock(), 3 days);

        vm.prank(governanceAdmin);
        gov.setExecutionWindow(21 days);
        assertEq(gov.executionWindow(), 21 days);
    }

    // ─── 21. Non-Governance Cannot Update Parameters ─────────────────────

    function testRevertNonGovernanceUpdateParams() public {
        // Attacker has no GOVERNANCE_ROLE — all setter calls should revert
        vm.prank(attacker);
        vm.expectRevert();
        gov.setVotingDelay(2 days);

        vm.prank(attacker);
        vm.expectRevert();
        gov.setVotingPeriod(14 days);

        vm.prank(attacker);
        vm.expectRevert();
        gov.setProposalThreshold(200e18);

        vm.prank(attacker);
        vm.expectRevert();
        gov.setQuorumBps(500);
    }

    // ─── 22. getVotes Returns Correct Power at Snapshot Block ────────────

    function testGetVotes() public {
        uint256 proposalId = _createProposal();
        (,,,,,, uint256 snapshotBlock,,,) = gov.getProposal(proposalId);

        // All voters staked STAKE_AMOUNT and checkpointed before proposal
        assertEq(gov.getVotes(voter1, snapshotBlock), STAKE_AMOUNT);
        assertEq(gov.getVotes(voter2, snapshotBlock), STAKE_AMOUNT);
        assertEq(gov.getVotes(voter3, snapshotBlock), STAKE_AMOUNT);
        assertEq(gov.getVotes(proposer, snapshotBlock), STAKE_AMOUNT);
    }

    // ─── 23. Cast Vote With Reason ───────────────────────────────────────

    function testCastVoteWithReason() public {
        uint256 proposalId = _createProposal();
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        vm.prank(voter1);
        gov.castVoteWithReason(
            proposalId, uint8(IGovernance.VoteType.For), "I support this"
        );

        assertTrue(gov.hasVoted(proposalId, voter1));

        (,,, uint256 forVotes,,,,,,) = gov.getProposal(proposalId);
        assertEq(forVotes, STAKE_AMOUNT);
    }

    // ─── 24. Cannot Vote with Zero Voting Power ──────────────────────────

    function testRevertVoteZeroPower() public {
        // Attacker has no staked tokens and no checkpoints
        uint256 proposalId = _createProposal();
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(
                NeunodeGovernance.VotingPowerZero.selector, attacker
            )
        );
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));
    }

    // ─── 25. Proposal Not Found ──────────────────────────────────────────

    function testRevertProposalNotFound() public {
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeGovernance.ProposalNotFound.selector, 999)
        );
        gov.state(999);
    }

    // ─── 26. Cannot Execute Before Timelock ──────────────────────────────

    function testRevertExecuteBeforeTimelock() public {
        uint256 proposalId = _createProposal();
        vm.warp(block.timestamp + VOTING_DELAY + 1);

        vm.prank(voter1);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));
        vm.prank(voter2);
        gov.castVote(proposalId, uint8(IGovernance.VoteType.For));

        vm.warp(block.timestamp + VOTING_PERIOD + 1);
        gov.queue(proposalId);

        // Try to execute immediately — timelock not passed
        vm.expectRevert(
            abi.encodeWithSelector(
                NeunodeGovernance.ProposalNotReady.selector, proposalId
            )
        );
        gov.execute(proposalId);
    }

    // ─── 27. Cannot Queue Non-Succeeded Proposal ─────────────────────────

    function testRevertQueueNonSucceeded() public {
        uint256 proposalId = _createProposal();

        // Still pending
        vm.expectRevert(
            abi.encodeWithSelector(
                NeunodeGovernance.ProposalNotSucceeded.selector, proposalId
            )
        );
        gov.queue(proposalId);
    }

    // ─── 28. Non-Proposer Cannot Cancel ──────────────────────────────────

    function testRevertCancelNonProposer() public {
        uint256 proposalId = _createProposal();

        vm.prank(attacker);
        vm.expectRevert("not authorized");
        gov.cancel(proposalId);
    }

    // ─── 29. Revert Empty Proposal ───────────────────────────────────────

    function testRevertEmptyProposal() public {
        address[] memory targets = new address[](0);
        uint256[] memory values = new uint256[](0);
        bytes[] memory calldatas = new bytes[](0);

        vm.prank(proposer);
        vm.expectRevert(NeunodeGovernance.EmptyProposal.selector);
        gov.propose(targets, values, calldatas, "Empty");
    }

    // ─── 30. Revert Array Length Mismatch ────────────────────────────────

    function testRevertArrayLengthMismatch() public {
        address[] memory targets = new address[](2);
        uint256[] memory values = new uint256[](1);
        bytes[] memory calldatas = new bytes[](2);

        vm.prank(proposer);
        vm.expectRevert(NeunodeGovernance.ArrayLengthMismatch.selector);
        gov.propose(targets, values, calldatas, "Mismatch");
    }
}

/// @title TargetMock — Simple target contract for execution testing
contract TargetMock {
    uint256 public value;

    function setValue(uint256 newValue) external {
        value = newValue;
    }
}
