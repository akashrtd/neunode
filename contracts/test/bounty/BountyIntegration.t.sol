// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../../src/bounty/BountyReview.sol";
import "../../src/NeunodeBounty.sol";
import "../../src/NeunodeEscrow.sol";
import "../../src/tokens/ComputeToken.sol";

/// @title BountyIntegrationTest — Tests for BountyReview + NeunodeBounty + NeunodeEscrow integration
contract BountyIntegrationTest is Test {
    BountyReview public review;
    NeunodeBounty public bounty;
    NeunodeEscrow public escrow;
    ComputeToken public token;

    address public admin;
    address public requester;
    address public provider;
    address public reviewer1;
    address public reviewer2;
    address public reviewer3;
    address public outsider;

    uint256 public reviewer1Pk;
    uint256 public reviewer2Pk;
    uint256 public reviewer3Pk;

    bytes32 constant BOUNTY_ID = keccak256("integration_bounty_1");
    uint256 constant REWARD = 5000e18;
    uint256 constant BOND = 750e18; // 15% of 5000

    uint256 claimDeadline;
    uint256 workDeadline;
    uint256 reviewDeadline;
    uint256 revisionDeadline;
    uint256 disputeDeadline;

    function setUp() public {
        admin = address(this);
        requester = makeAddr("requester");
        provider = makeAddr("provider");
        (reviewer1, reviewer1Pk) = makeAddrAndKey("reviewer1");
        (reviewer2, reviewer2Pk) = makeAddrAndKey("reviewer2");
        (reviewer3, reviewer3Pk) = makeAddrAndKey("reviewer3");
        outsider = makeAddr("outsider");

        review = new BountyReview();
        bounty = new NeunodeBounty();
        escrow = new NeunodeEscrow();
        token = new ComputeToken();

        // Setup bounty contract
        bounty.setReviewContract(address(review));
        bounty.setEscrow(address(escrow));

        // Grant provider BOUNTY_MANAGER_ROLE for deprecated claimBounty calls
        bounty.grantRole(bounty.BOUNTY_MANAGER_ROLE(), provider);

        // Setup escrow — register bounty contract
        escrow.registerBountyContract(address(bounty));

        // Grant review admin role to bounty contract so it can assign committees
        review.grantRole(0x00, address(bounty));

        // Mint tokens
        token.mint(requester, 100_000e18);
        token.mint(provider, 100_000e18);

        // Approve
        vm.prank(requester);
        token.approve(address(bounty), type(uint256).max);
        vm.prank(provider);
        token.approve(address(bounty), type(uint256).max);

        // Set deadlines
        claimDeadline = block.timestamp + 3 days;
        workDeadline = block.timestamp + 10 days;
        reviewDeadline = block.timestamp + 15 days;
        revisionDeadline = block.timestamp + 20 days;
        disputeDeadline = block.timestamp + 25 days;
    }

    // ─── Committee Assignment ─────────────────────────────────────────────

    function testAssignCommittee() public {
        address[3] memory reviewers = [reviewer1, reviewer2, reviewer3];
        review.assignCommittee(BOUNTY_ID, reviewers);

        (address[3] memory members,, uint8 rejectCount, bool resolved, bool isAssigned) =
            review.getCommittee(BOUNTY_ID);

        assertTrue(isAssigned);
        assertEq(members[0], reviewer1);
        assertEq(members[1], reviewer2);
        assertEq(members[2], reviewer3);
        assertEq(rejectCount, 0);
        assertFalse(resolved);
    }

    function testRevertAssignCommitteeZeroAddress() public {
        address[3] memory reviewers = [reviewer1, address(0), reviewer3];
        vm.expectRevert(BountyReview.ZeroAddressReviewer.selector);
        review.assignCommittee(BOUNTY_ID, reviewers);
    }

    function testRevertAssignCommitteeDuplicate() public {
        address[3] memory reviewers = [reviewer1, reviewer1, reviewer3];
        vm.expectRevert(BountyReview.DuplicateReviewer.selector);
        review.assignCommittee(BOUNTY_ID, reviewers);
    }

    function testRevertAssignCommitteeTwice() public {
        address[3] memory reviewers = [reviewer1, reviewer2, reviewer3];
        review.assignCommittee(BOUNTY_ID, reviewers);
        vm.expectRevert(
            abi.encodeWithSelector(BountyReview.CommitteeAlreadyAssigned.selector, BOUNTY_ID)
        );
        review.assignCommittee(BOUNTY_ID, reviewers);
    }

    // ─── 2-of-3 Accept Scenarios ──────────────────────────────────────────

    function testTwoOfThreeAccept() public {
        _setupCommittee();

        _submitSignedReview(reviewer1, reviewer1Pk, 80);
        assertFalse(review.isResolved(BOUNTY_ID));

        _submitSignedReview(reviewer2, reviewer2Pk, 70);
        assertTrue(review.isResolved(BOUNTY_ID));
        assertTrue(review.isAccepted(BOUNTY_ID));
    }

    function testTwoAcceptOneRejectStillAccepted() public {
        _setupCommittee();

        _submitSignedReview(reviewer1, reviewer1Pk, 80);
        _submitSignedReview(reviewer2, reviewer2Pk, 30);
        assertFalse(review.isResolved(BOUNTY_ID));

        _submitSignedReview(reviewer3, reviewer3Pk, 60);
        assertTrue(review.isResolved(BOUNTY_ID));
        assertTrue(review.isAccepted(BOUNTY_ID));
    }

    function testAcceptBoundary50() public {
        _setupCommittee();

        _submitSignedReview(reviewer1, reviewer1Pk, 50);
        _submitSignedReview(reviewer2, reviewer2Pk, 50);
        assertTrue(review.isAccepted(BOUNTY_ID));
    }

    // ─── 2-of-3 Reject Scenarios ──────────────────────────────────────────

    function testTwoOfThreeReject() public {
        _setupCommittee();

        _submitSignedReview(reviewer1, reviewer1Pk, 20);
        assertFalse(review.isResolved(BOUNTY_ID));

        _submitSignedReview(reviewer2, reviewer2Pk, 30);
        assertTrue(review.isResolved(BOUNTY_ID));
        assertFalse(review.isAccepted(BOUNTY_ID));
    }

    function testTwoRejectOneAcceptStillRejected() public {
        _setupCommittee();

        _submitSignedReview(reviewer1, reviewer1Pk, 10);
        _submitSignedReview(reviewer2, reviewer2Pk, 80);
        assertFalse(review.isResolved(BOUNTY_ID));

        _submitSignedReview(reviewer3, reviewer3Pk, 20);
        assertTrue(review.isResolved(BOUNTY_ID));
        assertFalse(review.isAccepted(BOUNTY_ID));
    }

    // ─── Double Review Prevention ─────────────────────────────────────────

    function testRevertDoubleReview() public {
        _setupCommittee();

        // First review
        _submitSignedReview(reviewer1, reviewer1Pk, 80);

        // Second review with nonce=1 — inline to avoid vm.prank conflicts
        bytes32 structHash = keccak256(
            abi.encode(
                review.REVIEW_TYPEHASH(),
                BOUNTY_ID,
                uint8(90),
                keccak256(bytes("good work")),
                uint256(1)
            )
        );
        bytes32 digest = _getTypedDataHash(structHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(reviewer1Pk, digest);
        bytes memory sig = abi.encodePacked(r, s, v);

        vm.prank(reviewer1);
        vm.expectRevert(
            abi.encodeWithSelector(BountyReview.AlreadyReviewed.selector, BOUNTY_ID, reviewer1)
        );
        review.submitReview(BOUNTY_ID, 90, "good work", sig);
    }

    // ─── Non-Committee Reviewer Rejection ─────────────────────────────────

    function testRevertNonCommitteeReviewer() public {
        _setupCommittee();

        // Create a valid-looking signature but from an outsider address
        bytes32 structHash = keccak256(
            abi.encode(
                review.REVIEW_TYPEHASH(), BOUNTY_ID, uint8(80), keccak256(bytes("test")), uint256(0)
            )
        );
        bytes32 digest = _getTypedDataHash(structHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(reviewer1Pk, digest);
        bytes memory sig = abi.encodePacked(r, s, v);

        vm.prank(outsider);
        vm.expectRevert(
            abi.encodeWithSelector(BountyReview.NotReviewer.selector, BOUNTY_ID, outsider)
        );
        review.submitReview(BOUNTY_ID, 80, "test", sig);
    }

    // ─── Review After Resolution ──────────────────────────────────────────

    function testRevertReviewAfterResolved() public {
        _setupCommittee();

        _submitSignedReview(reviewer1, reviewer1Pk, 80);
        _submitSignedReview(reviewer2, reviewer2Pk, 70);

        // reviewer3 hasn't reviewed yet, nonce=0 — inline to ensure expectRevert works
        bytes32 structHash = keccak256(
            abi.encode(
                review.REVIEW_TYPEHASH(),
                BOUNTY_ID,
                uint8(90),
                keccak256(bytes("good work")),
                uint256(0)
            )
        );
        bytes32 digest = _getTypedDataHash(structHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(reviewer3Pk, digest);
        bytes memory sig = abi.encodePacked(r, s, v);

        vm.prank(reviewer3);
        vm.expectRevert(
            abi.encodeWithSelector(BountyReview.CommitteeAlreadyResolved.selector, BOUNTY_ID)
        );
        review.submitReview(BOUNTY_ID, 90, "good work", sig);
    }

    // ─── Review Count & Get ───────────────────────────────────────────────

    function testReviewCount() public {
        _setupCommittee();

        assertEq(review.getReviewCount(BOUNTY_ID), 0);

        _submitSignedReview(reviewer1, reviewer1Pk, 80);
        assertEq(review.getReviewCount(BOUNTY_ID), 1);

        _submitSignedReview(reviewer2, reviewer2Pk, 70);
        assertEq(review.getReviewCount(BOUNTY_ID), 2);
    }

    function testGetReview() public {
        _setupCommittee();

        _submitSignedReview(reviewer1, reviewer1Pk, 75);

        (address r, uint8 score, string memory feedback) = review.getReview(BOUNTY_ID, 0);
        assertEq(r, reviewer1);
        assertEq(score, 75);
        assertEq(feedback, "good work");
    }

    // ─── Full Lifecycle: Create → Claim → Submit → Review → Payout ────────

    function testFullLifecycleWithReviewAccept() public {
        _createBountyWithDeadlines();
        _claimAndSubmit();

        address[3] memory reviewers = [reviewer1, reviewer2, reviewer3];
        vm.prank(requester);
        bounty.startReview(BOUNTY_ID, reviewers);

        assertEq(
            uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.UnderReview)
        );

        _submitSignedReview(reviewer1, reviewer1Pk, 80);
        _submitSignedReview(reviewer2, reviewer2Pk, 70);

        bounty.processReviewResult(BOUNTY_ID);
        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Accepted));

        vm.prank(provider);
        bounty.revealWork(BOUNTY_ID, keccak256("test_submission"), keccak256("submission_salt"));

        uint256 providerBalBefore = token.balanceOf(provider);
        bounty.payBounty(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Paid));
        assertEq(token.balanceOf(provider) - providerBalBefore, REWARD);
    }

    function testFullLifecycleWithReviewReject() public {
        _createBountyWithDeadlines();
        _claimAndSubmit();

        address[3] memory reviewers = [reviewer1, reviewer2, reviewer3];
        vm.prank(requester);
        bounty.startReview(BOUNTY_ID, reviewers);

        _submitSignedReview(reviewer1, reviewer1Pk, 20);
        _submitSignedReview(reviewer2, reviewer2Pk, 30);

        uint256 requesterBalBefore = token.balanceOf(requester);
        bounty.processReviewResult(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Rejected));
        assertEq(token.balanceOf(requester) - requesterBalBefore, REWARD);
    }

    // ─── Fee Splitting ────────────────────────────────────────────────────

    function testFeeSplitting() public {
        _createBountyWithDeadlines();
        _claimAndSubmit();

        _revealAndAccept();

        uint256 providerBalBefore = token.balanceOf(provider);
        uint256 adminBalBefore = token.balanceOf(admin);

        bounty.payBountyWithFees(BOUNTY_ID);

        uint256 expectedFee = (REWARD * 600) / 10_000;
        uint256 expectedPayout = REWARD - expectedFee;

        assertEq(token.balanceOf(provider) - providerBalBefore, expectedPayout);
        assertEq(token.balanceOf(admin) - adminBalBefore, expectedFee);
    }

    function testFeeSplittingExactValues() public {
        _createBountyWithDeadlines();
        _claimAndSubmit();

        _revealAndAccept();

        bounty.payBountyWithFees(BOUNTY_ID);

        assertEq(token.balanceOf(provider), 100_000e18 + 4700e18);
    }

    function testRevertFeeConfigTooHigh() public {
        vm.expectRevert(NeunodeBounty.TotalFeesExceed100.selector);
        bounty.proposeFeeConfig(500, 400, 200, admin, admin, admin);
    }

    /// @notice Verify no rounding drift with separate fee recipients:
    ///         providerPayout + sum(fees) == reward exactly
    function testFeeRoundingNoDriftOddAmounts() public {
        // Use distinct recipients so individual balance checks are unambiguous
        address protoRecipient = makeAddr("protoFee");
        address revRecipient = makeAddr("revFee");
        address verRecipient = makeAddr("verFee");
        bounty.proposeFeeConfig(200, 300, 100, protoRecipient, revRecipient, verRecipient);
        skip(24 hours);
        bounty.executeFeeConfig();

        // Odd amounts that expose rounding: 10001 wei, 1 wei, 999 wei
        uint256[3] memory rewards = [uint256(10001), uint256(1), uint256(999)];

        for (uint256 i = 0; i < rewards.length; i++) {
            bytes32 bountyId = keccak256(abi.encode("rounding_test", i));
            uint256 reward = rewards[i];

            token.mint(requester, reward);

            vm.prank(requester);
            bounty.createBounty(
                bountyId,
                reward,
                address(token),
                block.timestamp + 3 days,
                block.timestamp + 10 days
            );

            vm.prank(provider);
            bounty.claimBounty(bountyId);

            vm.prank(provider);
            bounty.submitWork(
                bountyId, keccak256(abi.encodePacked(keccak256("work"), keccak256("salt")))
            );

            vm.prank(provider);
            bounty.revealWork(bountyId, keccak256("work"), keccak256("salt"));

            vm.prank(requester);
            bounty.acceptSubmission(bountyId);

            uint256 providerBalBefore = token.balanceOf(provider);
            uint256 protoBalBefore = token.balanceOf(protoRecipient);
            uint256 revBalBefore = token.balanceOf(revRecipient);
            uint256 verBalBefore = token.balanceOf(verRecipient);
            uint256 contractBalBefore = token.balanceOf(address(bounty));

            bounty.payBountyWithFees(bountyId);

            uint256 protocolFee = (reward * 200) / 10_000;
            uint256 reviewerFee = (reward * 300) / 10_000;
            uint256 verificationFee = (reward * 100) / 10_000;
            uint256 providerPayout = reward - protocolFee - reviewerFee - verificationFee;

            // CORE INVARIANT: no dust, no drift
            assertEq(providerPayout + protocolFee + reviewerFee + verificationFee, reward);

            // Verify individual transfers
            assertEq(token.balanceOf(provider) - providerBalBefore, providerPayout);
            assertEq(token.balanceOf(protoRecipient) - protoBalBefore, protocolFee);
            assertEq(token.balanceOf(revRecipient) - revBalBefore, reviewerFee);
            assertEq(token.balanceOf(verRecipient) - verBalBefore, verificationFee);

            // Contract must have released the full reward
            assertEq(contractBalBefore - token.balanceOf(address(bounty)), reward);
        }
    }

    function testSetFeeConfig() public {
        address feeRecipient1 = makeAddr("fee1");
        address feeRecipient2 = makeAddr("fee2");
        address feeRecipient3 = makeAddr("fee3");

        // Propose fee config
        bounty.proposeFeeConfig(100, 200, 50, feeRecipient1, feeRecipient2, feeRecipient3);

        // Cannot execute before timelock expires
        vm.expectRevert(
            abi.encodeWithSelector(
                NeunodeBounty.FeeChangeTimelockNotExpired.selector, block.timestamp + 24 hours
            )
        );
        bounty.executeFeeConfig();

        // Warp past timelock
        skip(24 hours + 1);
        bounty.executeFeeConfig();

        (
            uint256 protocolBps,
            uint256 reviewerBps,
            uint256 verificationBps,
            address protoRecipient,
            address revRecipient,
            address verRecipient
        ) = bounty.feeConfig();

        assertEq(protocolBps, 100);
        assertEq(reviewerBps, 200);
        assertEq(verificationBps, 50);
        assertEq(protoRecipient, feeRecipient1);
        assertEq(revRecipient, feeRecipient2);
        assertEq(verRecipient, feeRecipient3);
    }

    // ─── Bounty With Provider Bond ────────────────────────────────────────

    function testClaimWithBond() public {
        _createBountyWithDeadlines();

        vm.prank(provider);
        bounty.claimBountyWithBond(BOUNTY_ID, BOND);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Claimed));
        assertEq(bounty.providerBonds(BOUNTY_ID), BOND);
    }

    function testBondReturnedOnAccept() public {
        _createBountyWithDeadlines();

        vm.prank(provider);
        bounty.claimBountyWithBond(BOUNTY_ID, BOND);

        vm.prank(provider);
        bounty.submitWork(
            BOUNTY_ID, keccak256(abi.encodePacked(keccak256("work"), keccak256("salt")))
        );

        vm.prank(provider);
        bounty.revealWork(BOUNTY_ID, keccak256("work"), keccak256("salt"));

        vm.prank(requester);
        bounty.acceptSubmission(BOUNTY_ID);

        uint256 providerBalBefore = token.balanceOf(provider);
        bounty.payBounty(BOUNTY_ID);

        assertEq(token.balanceOf(provider) - providerBalBefore, REWARD + BOND);
    }

    function testBondReturnedOnReject() public {
        _createBountyWithDeadlines();

        vm.prank(provider);
        bounty.claimBountyWithBond(BOUNTY_ID, BOND);

        vm.prank(provider);
        bounty.submitWork(
            BOUNTY_ID, keccak256(abi.encodePacked(keccak256("work"), keccak256("salt")))
        );

        uint256 providerBalBefore = token.balanceOf(provider);
        uint256 requesterBalBefore = token.balanceOf(requester);

        vm.prank(requester);
        bounty.rejectSubmission(BOUNTY_ID);

        assertEq(token.balanceOf(provider) - providerBalBefore, BOND);
        assertEq(token.balanceOf(requester) - requesterBalBefore, REWARD);
    }

    // ─── Deadline Enforcement ─────────────────────────────────────────────

    function testReviewDeadlineExpiry() public {
        _createBountyWithDeadlines();
        _claimAndSubmit();

        vm.warp(reviewDeadline + 1);

        uint256 requesterBalBefore = token.balanceOf(requester);
        bounty.checkExpiry(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Expired));
        assertEq(token.balanceOf(requester) - requesterBalBefore, REWARD);
    }

    function testRevisionDeadlineExpiry() public {
        _createBountyWithDeadlines();
        _claimAndSubmit();

        vm.prank(requester);
        bounty.requestRevision(BOUNTY_ID);

        vm.warp(revisionDeadline + 1);

        bounty.checkExpiry(BOUNTY_ID);
        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Expired));
    }

    function testDisputeDeadlineExpiry() public {
        _createBountyWithDeadlines();
        _claimAndSubmit();

        vm.prank(provider);
        bounty.disputeBounty(BOUNTY_ID);

        vm.warp(disputeDeadline + 1);

        uint256 requesterBalBefore = token.balanceOf(requester);
        bounty.checkExpiry(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Expired));
        assertEq(token.balanceOf(requester) - requesterBalBefore, REWARD);
    }

    function testRevisionDeadlineEnforcement() public {
        _createBountyWithDeadlines();
        _claimAndSubmit();

        vm.prank(requester);
        bounty.requestRevision(BOUNTY_ID);

        vm.warp(revisionDeadline + 1);

        vm.prank(provider);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeBounty.DeadlinePassed.selector, revisionDeadline)
        );
        bounty.submitWork(
            BOUNTY_ID, keccak256(abi.encodePacked(keccak256("late_revision"), keccak256("salt")))
        );
    }

    function testDisputeDeadlineEnforcement() public {
        _createBountyWithDeadlines();
        _claimAndSubmit();

        vm.warp(disputeDeadline + 1);

        vm.prank(provider);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeBounty.DeadlinePassed.selector, disputeDeadline)
        );
        bounty.disputeBounty(BOUNTY_ID);
    }

    // ─── Dispute Resolution ───────────────────────────────────────────────

    function testResolveDisputeAccept() public {
        _createBountyWithDeadlines();
        _claimAndSubmit();

        vm.prank(provider);
        bounty.disputeBounty(BOUNTY_ID);

        bounty.resolveDispute(BOUNTY_ID, true);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Accepted));
    }

    function testResolveDisputeReject() public {
        _createBountyWithDeadlines();
        _claimAndSubmit();

        vm.prank(provider);
        bounty.disputeBounty(BOUNTY_ID);

        uint256 requesterBalBefore = token.balanceOf(requester);
        bounty.resolveDispute(BOUNTY_ID, false);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Rejected));
        assertEq(token.balanceOf(requester) - requesterBalBefore, REWARD);
    }

    // ─── View Functions ───────────────────────────────────────────────────

    function testGetBountyFull() public {
        _createBountyWithDeadlines();

        (
            bytes32 id,
            address req,
            address prov,
            NeunodeBounty.BountyState state,
            uint256 reward,
            address _rewardToken,
            uint256 _claimDl,
            uint256 _workDl,
            uint256 rDeadline,
            uint256 _created,
            bytes32 _subHash,
            uint256 _revCount,
            uint256 revDeadline,
            uint256 dDeadline,
            bool useEscrow_,
            uint256 provBond
        ) = bounty.getBountyFull(BOUNTY_ID);

        assertEq(id, BOUNTY_ID);
        assertEq(req, requester);
        assertEq(prov, address(0));
        assertEq(uint8(state), uint8(NeunodeBounty.BountyState.Open));
        assertEq(reward, REWARD);
        assertEq(rDeadline, reviewDeadline);
        assertEq(revDeadline, revisionDeadline);
        assertEq(dDeadline, disputeDeadline);
        assertFalse(useEscrow_);
        assertEq(provBond, 0);
    }

    // ─── Process Review Not Resolved ──────────────────────────────────────

    function testRevertProcessReviewNotResolved() public {
        _createBountyWithDeadlines();
        _claimAndSubmit();

        address[3] memory reviewers = [reviewer1, reviewer2, reviewer3];
        vm.prank(requester);
        bounty.startReview(BOUNTY_ID, reviewers);

        _submitSignedReview(reviewer1, reviewer1Pk, 80);

        vm.expectRevert(abi.encodeWithSelector(NeunodeBounty.ReviewNotResolved.selector, BOUNTY_ID));
        bounty.processReviewResult(BOUNTY_ID);
    }

    // ─── Escrow Integration ───────────────────────────────────────────────

    function testEscrowIsFundedAfterBountyFlow() public {
        escrow.registerBountyContract(address(this));

        vm.prank(requester);
        token.approve(address(escrow), type(uint256).max);
        vm.prank(provider);
        token.approve(address(escrow), type(uint256).max);

        escrow.createBountyEscrow(BOUNTY_ID, requester, address(token), REWARD, workDeadline);

        assertFalse(escrow.isEscrowFunded(BOUNTY_ID));

        escrow.bondProvider(BOUNTY_ID, provider, BOND);

        assertTrue(escrow.isEscrowFunded(BOUNTY_ID));
    }

    function testEscrowReleaseWithFees() public {
        escrow.registerBountyContract(address(this));

        vm.prank(requester);
        token.approve(address(escrow), type(uint256).max);
        vm.prank(provider);
        token.approve(address(escrow), type(uint256).max);

        escrow.createBountyEscrow(BOUNTY_ID, requester, address(token), REWARD, workDeadline);
        escrow.bondProvider(BOUNTY_ID, provider, BOND);

        uint256 providerBalBefore = token.balanceOf(provider);

        escrow.releaseWithFees(BOUNTY_ID, provider, 200, 300, 100, admin, admin, admin);

        uint256 expectedPayout = REWARD - (REWARD * 600) / 10_000;

        assertEq(token.balanceOf(provider) - providerBalBefore, expectedPayout + BOND);
    }

    function testEscrowRefundRequester() public {
        escrow.registerBountyContract(address(this));

        vm.prank(requester);
        token.approve(address(escrow), type(uint256).max);
        vm.prank(provider);
        token.approve(address(escrow), type(uint256).max);

        escrow.createBountyEscrow(BOUNTY_ID, requester, address(token), REWARD, workDeadline);
        escrow.bondProvider(BOUNTY_ID, provider, BOND);

        uint256 requesterBalBefore = token.balanceOf(requester);

        escrow.refundRequester(BOUNTY_ID);

        assertEq(token.balanceOf(requester) - requesterBalBefore, REWARD + BOND);
    }

    // ─── Helpers ──────────────────────────────────────────────────────────

    function _setupCommittee() internal {
        address[3] memory reviewers = [reviewer1, reviewer2, reviewer3];
        review.assignCommittee(BOUNTY_ID, reviewers);
    }

    function _submitSignedReview(address reviewer, uint256 pk, uint8 score) internal {
        _submitSignedReviewWithNonce(reviewer, pk, score, 0);
    }

    function _submitSignedReviewWithNonce(address reviewer, uint256 pk, uint8 score, uint256 nonce)
        internal
    {
        bytes32 structHash = keccak256(
            abi.encode(
                review.REVIEW_TYPEHASH(), BOUNTY_ID, score, keccak256(bytes("good work")), nonce
            )
        );

        bytes32 digest = _getTypedDataHash(structHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(pk, digest);
        bytes memory sig = abi.encodePacked(r, s, v);

        vm.prank(reviewer);
        review.submitReview(BOUNTY_ID, score, "good work", sig);
    }

    function _getTypedDataHash(bytes32 structHash) internal view returns (bytes32) {
        bytes32 domainSeparator = _hashDomainSeparator();
        return keccak256(abi.encodePacked("\x19\x01", domainSeparator, structHash));
    }

    function _hashDomainSeparator() internal view returns (bytes32) {
        bytes32 EIP712_DOMAIN_TYPEHASH = keccak256(
            "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
        );
        return keccak256(
            abi.encode(
                EIP712_DOMAIN_TYPEHASH,
                keccak256("NeunodeBountyReview"),
                keccak256("1"),
                block.chainid,
                address(review)
            )
        );
    }

    function _createBountyWithDeadlines() internal {
        vm.prank(requester);
        bounty.createBountyWithDeadlines(
            BOUNTY_ID,
            REWARD,
            address(token),
            claimDeadline,
            workDeadline,
            reviewDeadline,
            revisionDeadline,
            disputeDeadline,
            false
        );
    }

    function _claimAndSubmit() internal {
        vm.prank(provider);
        bounty.claimBounty(BOUNTY_ID);

        bytes32 artifactHash = keccak256("test_submission");
        bytes32 salt = keccak256("submission_salt");
        vm.prank(provider);
        bounty.submitWork(BOUNTY_ID, keccak256(abi.encodePacked(artifactHash, salt)));
    }

    function _revealAndAccept() internal {
        vm.prank(provider);
        bounty.revealWork(BOUNTY_ID, keccak256("test_submission"), keccak256("submission_salt"));

        vm.prank(requester);
        bounty.acceptSubmission(BOUNTY_ID);
    }

    // ─── Fee Config Timelock Tests ──────────────────────────────────────────

    function testRevertExecuteNoPendingFeeChange() public {
        vm.expectRevert(NeunodeBounty.NoPendingFeeChange.selector);
        bounty.executeFeeConfig();
    }

    function testRevertCancelNoPendingFeeChange() public {
        vm.expectRevert(NeunodeBounty.NoPendingFeeChange.selector);
        bounty.cancelFeeConfigProposal();
    }

    function testCancelFeeConfigProposal() public {
        bounty.proposeFeeConfig(100, 200, 50, admin, admin, admin);

        // Cancel before timelock expires
        bounty.cancelFeeConfigProposal();

        // Pending state should be cleared
        (uint256 pBps,,,,,) = bounty.pendingFeeConfig();
        assertEq(pBps, 0);
        assertEq(bounty.pendingFeeConfigTimestamp(), 0);

        // Execute should fail after cancel
        skip(24 hours + 1);
        vm.expectRevert(NeunodeBounty.NoPendingFeeChange.selector);
        bounty.executeFeeConfig();
    }

    function testProposeOverwritesPreviousPending() public {
        bounty.proposeFeeConfig(100, 100, 100, admin, admin, admin);
        // Advance partially but not past timelock
        skip(12 hours);
        // Overwrite with new proposal — timer resets from now
        bounty.proposeFeeConfig(50, 50, 50, admin, admin, admin);
        // Skip remaining 12h from first proposal — NOT enough for second
        skip(12 hours);
        // Should NOT be executable — second proposal's timelock hasn't expired
        uint256 expiresAt = block.timestamp + 12 hours;
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeBounty.FeeChangeTimelockNotExpired.selector, expiresAt)
        );
        bounty.executeFeeConfig();
        // Warp past the second proposal's timelock
        skip(12 hours + 1);
        bounty.executeFeeConfig();
        (uint256 pBps, uint256 rBps, uint256 vBps,,,) = bounty.feeConfig();
        assertEq(pBps, 50);
        assertEq(rBps, 50);
        assertEq(vBps, 50);
    }

    function testFeeTimelockExactBoundary() public {
        bounty.proposeFeeConfig(300, 300, 300, admin, admin, admin);
        // Exactly at timelock boundary — should succeed
        skip(24 hours);
        bounty.executeFeeConfig();
        (uint256 pBps, uint256 rBps, uint256 vBps,,,) = bounty.feeConfig();
        assertEq(pBps, 300);
    }

    function testNonAdminCannotProposeFeeConfig() public {
        vm.prank(outsider);
        vm.expectRevert();
        bounty.proposeFeeConfig(100, 100, 100, outsider, outsider, outsider);
    }

    function testNonAdminCannotExecuteFeeConfig() public {
        bounty.proposeFeeConfig(100, 100, 100, admin, admin, admin);
        skip(24 hours + 1);
        vm.prank(outsider);
        vm.expectRevert();
        bounty.executeFeeConfig();
    }
}
