// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/NeunodeBounty.sol";
import "../src/tokens/ComputeToken.sol";

/// @title NeunodeBountyTest — Tests for bounty state machine
contract NeunodeBountyTest is Test {
    NeunodeBounty public bounty;
    ComputeToken public token;

    address public requester;
    address public provider;
    address public attacker;

    bytes32 constant BOUNTY_ID = keccak256("bounty_1");
    uint256 constant REWARD = 5000e18;
    uint256 constant CLAIM_DEADLINE_OFFSET = 3 days;
    uint256 constant WORK_DEADLINE_OFFSET = 10 days;

    uint256 claimDeadline;
    uint256 workDeadline;

    function setUp() public {
        bounty = new NeunodeBounty();
        token = new ComputeToken();

        requester = makeAddr("requester");
        provider = makeAddr("provider");
        attacker = makeAddr("attacker");

        claimDeadline = block.timestamp + CLAIM_DEADLINE_OFFSET;
        workDeadline = block.timestamp + WORK_DEADLINE_OFFSET;

        token.mint(requester, 100_000e18);

        vm.prank(requester);
        token.approve(address(bounty), type(uint256).max);
    }

    // ─── Create Bounty ────────────────────────────────────────────────────

    function testCreateBounty() public {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);

        assertEq(token.balanceOf(address(bounty)), REWARD);
        assertEq(bounty.activeCount(), 1);

        (
            bytes32 id,
            address req,
            address prov,
            NeunodeBounty.BountyState state,
            uint256 reward,
            address rewardToken,
            uint256 cDeadline,
            uint256 wDeadline,,,,
        ) = bounty.bounties(BOUNTY_ID);

        assertEq(id, BOUNTY_ID);
        assertEq(req, requester);
        assertEq(prov, address(0));
        assertEq(uint8(state), uint8(NeunodeBounty.BountyState.Open));
        assertEq(reward, REWARD);
        assertEq(rewardToken, address(token));
    }

    function testRevertCreateBountyZeroReward() public {
        vm.prank(requester);
        vm.expectRevert(NeunodeBounty.InvalidReward.selector);
        bounty.createBounty(BOUNTY_ID, 0, address(token), claimDeadline, workDeadline);
    }

    function testRevertCreateBountyPastDeadline() public {
        vm.prank(requester);
        vm.expectRevert(NeunodeBounty.InvalidDeadline.selector);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), block.timestamp - 1, workDeadline);
    }

    function testRevertCreateBountyWorkBeforeClaim() public {
        vm.prank(requester);
        vm.expectRevert(NeunodeBounty.InvalidDeadline.selector);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), workDeadline, claimDeadline);
    }

    // ─── Claim Bounty ─────────────────────────────────────────────────────

    function testClaimBounty() public {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);

        vm.prank(provider);
        bounty.claimBounty(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Claimed));
    }

    function testRevertClaimNotOpen() public {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);

        vm.prank(provider);
        bounty.claimBounty(BOUNTY_ID);

        vm.prank(attacker);
        vm.expectRevert();
        bounty.claimBounty(BOUNTY_ID);
    }

    function testRevertClaimAfterDeadline() public {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);

        vm.warp(claimDeadline + 1);

        vm.prank(provider);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeBounty.DeadlinePassed.selector, claimDeadline)
        );
        bounty.claimBounty(BOUNTY_ID);
    }

    // ─── Submit Work ──────────────────────────────────────────────────────

    function testSubmitWork() public {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);

        vm.prank(provider);
        bounty.claimBounty(BOUNTY_ID);

        bytes32 subHash = keccak256("submission_data");
        vm.prank(provider);
        bounty.submitWork(BOUNTY_ID, subHash);

        assertEq(
            uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Submitted)
        );
    }

    function testRevertSubmitNotProvider() public {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);

        vm.prank(provider);
        bounty.claimBounty(BOUNTY_ID);

        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeBounty.NotProvider.selector, BOUNTY_ID, attacker)
        );
        bounty.submitWork(BOUNTY_ID, keccak256("fake"));
    }

    function testRevertSubmitAfterWorkDeadline() public {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);

        vm.prank(provider);
        bounty.claimBounty(BOUNTY_ID);

        vm.warp(workDeadline + 1);

        vm.prank(provider);
        vm.expectRevert(abi.encodeWithSelector(NeunodeBounty.DeadlinePassed.selector, workDeadline));
        bounty.submitWork(BOUNTY_ID, keccak256("late"));
    }

    // ─── Accept Submission ────────────────────────────────────────────────

    function testAcceptSubmission() public {
        _createClaimSubmit();

        vm.prank(requester);
        bounty.acceptSubmission(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Accepted));
    }

    function testRevertAcceptNotRequester() public {
        _createClaimSubmit();

        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeBounty.NotRequester.selector, BOUNTY_ID, attacker)
        );
        bounty.acceptSubmission(BOUNTY_ID);
    }

    // ─── Reject Submission ────────────────────────────────────────────────

    function testRejectSubmission() public {
        _createClaimSubmit();

        uint256 requesterBalBefore = token.balanceOf(requester);

        vm.prank(requester);
        bounty.rejectSubmission(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Rejected));
        assertEq(token.balanceOf(requester), requesterBalBefore + REWARD);
        assertEq(token.balanceOf(address(bounty)), 0);
    }

    // ─── Dispute ──────────────────────────────────────────────────────────

    function testDisputeByRequester() public {
        _createClaimSubmit();

        vm.prank(requester);
        bounty.disputeBounty(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Disputed));
    }

    function testDisputeByProvider() public {
        _createClaimSubmit();

        vm.prank(provider);
        bounty.disputeBounty(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Disputed));
    }

    function testRevertDisputeByAttacker() public {
        _createClaimSubmit();

        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeBounty.NotClaimer.selector, BOUNTY_ID, attacker)
        );
        bounty.disputeBounty(BOUNTY_ID);
    }

    // ─── Cancel ───────────────────────────────────────────────────────────

    function testCancelBounty() public {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);

        uint256 requesterBalBefore = token.balanceOf(requester);

        vm.prank(requester);
        bounty.cancelBounty(BOUNTY_ID);

        assertEq(
            uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Cancelled)
        );
        assertEq(token.balanceOf(requester), requesterBalBefore + REWARD);
    }

    function testRevertCancelNotRequester() public {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);

        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeBounty.NotRequester.selector, BOUNTY_ID, attacker)
        );
        bounty.cancelBounty(BOUNTY_ID);
    }

    function testRevertCancelClaimedBounty() public {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);

        vm.prank(provider);
        bounty.claimBounty(BOUNTY_ID);

        vm.prank(requester);
        vm.expectRevert();
        bounty.cancelBounty(BOUNTY_ID);
    }

    // ─── Revision ─────────────────────────────────────────────────────────

    function testRequestRevision() public {
        _createClaimSubmit();

        vm.prank(requester);
        bounty.requestRevision(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Revision));
    }

    function testResubmitAfterRevision() public {
        _createClaimSubmit();

        vm.prank(requester);
        bounty.requestRevision(BOUNTY_ID);

        vm.prank(provider);
        bounty.submitWork(BOUNTY_ID, keccak256("revised_submission"));

        assertEq(
            uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Submitted)
        );
    }

    function testRevertMaxRevisions() public {
        _createClaimSubmit();

        for (uint256 i = 0; i < 3; i++) {
            vm.prank(requester);
            bounty.requestRevision(BOUNTY_ID);

            vm.prank(provider);
            bounty.submitWork(BOUNTY_ID, keccak256(abi.encode("revision", i)));
        }

        // 4th revision should fail
        vm.prank(requester);
        vm.expectRevert(NeunodeBounty.MaxRevisionsReached.selector);
        bounty.requestRevision(BOUNTY_ID);
    }

    // ─── Pay Bounty ───────────────────────────────────────────────────────

    function testPayBounty() public {
        _createClaimSubmit();

        vm.prank(requester);
        bounty.acceptSubmission(BOUNTY_ID);

        uint256 providerBalBefore = token.balanceOf(provider);

        bounty.payBounty(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Paid));
        assertEq(token.balanceOf(provider) - providerBalBefore, REWARD);
        assertEq(token.balanceOf(address(bounty)), 0);
    }

    // ─── Expiry ───────────────────────────────────────────────────────────

    function testExpiryOpenBounty() public {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);

        vm.warp(claimDeadline + 1);

        uint256 requesterBalBefore = token.balanceOf(requester);
        bounty.checkExpiry(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Expired));
        assertEq(token.balanceOf(requester), requesterBalBefore + REWARD);
    }

    function testExpiryClaimedBounty() public {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);

        vm.prank(provider);
        bounty.claimBounty(BOUNTY_ID);

        vm.warp(workDeadline + 1);

        bounty.checkExpiry(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Expired));
    }

    // ─── Full Lifecycle ───────────────────────────────────────────────────

    function testFullLifecycle() public {
        uint256 providerBalBefore = token.balanceOf(provider);

        // Create
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);
        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Open));

        // Claim
        vm.prank(provider);
        bounty.claimBounty(BOUNTY_ID);
        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Claimed));

        // Submit
        vm.prank(provider);
        bounty.submitWork(BOUNTY_ID, keccak256("final_work"));
        assertEq(
            uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Submitted)
        );

        // Accept
        vm.prank(requester);
        bounty.acceptSubmission(BOUNTY_ID);
        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Accepted));

        // Pay
        bounty.payBounty(BOUNTY_ID);
        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Paid));

        // Provider got paid
        assertEq(token.balanceOf(provider) - providerBalBefore, REWARD);
    }

    // ─── Helpers ──────────────────────────────────────────────────────────

    function _createClaimSubmit() internal {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);

        vm.prank(provider);
        bounty.claimBounty(BOUNTY_ID);

        vm.prank(provider);
        bounty.submitWork(BOUNTY_ID, keccak256("test_submission"));
    }
}
