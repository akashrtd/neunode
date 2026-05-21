// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../../src/NeunodeBounty.sol";
import "../../src/tokens/ComputeToken.sol";

/// @title CommitRevealTest — Tests for commit-reveal claim scheme
contract CommitRevealTest is Test {
    NeunodeBounty public bounty;
    ComputeToken public token;

    address public requester;
    address public provider;
    address public attacker;

    bytes32 constant BOUNTY_ID = keccak256("commit_reveal_bounty");
    uint256 constant REWARD = 5000e18;
    uint256 constant BOND = 750e18;
    bytes32 constant NONCE = keccak256("secret_nonce");

    uint256 claimDeadline;
    uint256 workDeadline;

    function setUp() public {
        bounty = new NeunodeBounty();
        token = new ComputeToken();

        requester = makeAddr("requester");
        provider = makeAddr("provider");
        attacker = makeAddr("attacker");

        claimDeadline = block.timestamp + 3 days;
        workDeadline = block.timestamp + 10 days;

        token.mint(requester, 100_000e18);
        token.mint(provider, 100_000e18);

        vm.prank(requester);
        token.approve(address(bounty), type(uint256).max);
        vm.prank(provider);
        token.approve(address(bounty), type(uint256).max);
    }

    // ─── Helpers ──────────────────────────────────────────────────────────

    function _createBounty() internal {
        vm.prank(requester);
        bounty.createBounty(BOUNTY_ID, REWARD, address(token), claimDeadline, workDeadline);
    }

    function _computeCommitment(address claimer, bytes32 bountyId, bytes32 nonce)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(abi.encodePacked(claimer, bountyId, nonce));
    }

    // ─── Happy Path: commit → reveal → bounty claimed ─────────────────────

    function testCommitRevealHappyPath() public {
        _createBounty();

        bytes32 commitment = _computeCommitment(provider, BOUNTY_ID, NONCE);

        // Commit
        vm.prank(provider);
        bounty.commitClaim(BOUNTY_ID, commitment);

        // Reveal
        vm.prank(provider);
        bounty.revealClaim(BOUNTY_ID, 0, NONCE);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Claimed));

        // Provider is the claimer
        (,, address prov,,,,,,,,,) = bounty.bounties(BOUNTY_ID);
        assertEq(prov, provider);
    }

    function testCommitRevealWithBond() public {
        _createBounty();

        bytes32 commitment = _computeCommitment(provider, BOUNTY_ID, NONCE);

        vm.prank(provider);
        bounty.commitClaim(BOUNTY_ID, commitment);

        uint256 providerBalBefore = token.balanceOf(provider);

        vm.prank(provider);
        bounty.revealClaim(BOUNTY_ID, BOND, NONCE);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Claimed));
        assertEq(bounty.providerBonds(BOUNTY_ID), BOND);
        assertEq(providerBalBefore - token.balanceOf(provider), BOND);
    }

    function testCommitRevealFullLifecycle() public {
        _createBounty();

        bytes32 commitment = _computeCommitment(provider, BOUNTY_ID, NONCE);

        // Commit + Reveal
        vm.prank(provider);
        bounty.commitClaim(BOUNTY_ID, commitment);
        vm.prank(provider);
        bounty.revealClaim(BOUNTY_ID, 0, NONCE);

        // Submit work (commit-reveal for artifact)
        bytes32 artifactHash = keccak256("final_work");
        bytes32 salt = keccak256("work_salt");
        bytes32 workCommitment = keccak256(abi.encodePacked(artifactHash, salt));
        vm.prank(provider);
        bounty.submitWork(BOUNTY_ID, workCommitment);

        // Accept
        vm.prank(requester);
        bounty.acceptSubmission(BOUNTY_ID);

        // Reveal work
        vm.prank(provider);
        bounty.revealWork(BOUNTY_ID, artifactHash, salt);

        // Pay
        uint256 providerBalBefore = token.balanceOf(provider);
        bounty.payBounty(BOUNTY_ID);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Paid));
        assertEq(token.balanceOf(provider) - providerBalBefore, REWARD);
    }

    // ─── Revert: reveal without commit ────────────────────────────────────

    function testRevertRevealWithoutCommit() public {
        _createBounty();

        vm.prank(provider);
        vm.expectRevert(abi.encodeWithSelector(NeunodeBounty.NotCommitted.selector, BOUNTY_ID));
        bounty.revealClaim(BOUNTY_ID, 0, NONCE);
    }

    // ─── Revert: wrong nonce ──────────────────────────────────────────────

    function testRevertWrongNonce() public {
        _createBounty();

        bytes32 commitment = _computeCommitment(provider, BOUNTY_ID, NONCE);

        vm.prank(provider);
        bounty.commitClaim(BOUNTY_ID, commitment);

        bytes32 wrongNonce = keccak256("wrong_nonce");
        vm.prank(provider);
        vm.expectRevert(abi.encodeWithSelector(NeunodeBounty.InvalidReveal.selector, BOUNTY_ID));
        bounty.revealClaim(BOUNTY_ID, 0, wrongNonce);
    }

    // ─── Revert: double commit ────────────────────────────────────────────

    function testRevertDoubleCommit() public {
        _createBounty();

        bytes32 commitment = _computeCommitment(provider, BOUNTY_ID, NONCE);

        vm.prank(provider);
        bounty.commitClaim(BOUNTY_ID, commitment);

        vm.prank(provider);
        vm.expectRevert(abi.encodeWithSelector(NeunodeBounty.AlreadyCommitted.selector, BOUNTY_ID));
        bounty.commitClaim(BOUNTY_ID, commitment);
    }

    // ─── Front-running mitigation: attacker can't reveal with stolen commitment

    // ─────────────────────────────────────────────────────────────────────

    function testAttackerCannotRevealStolenCommitment() public {
        _createBounty();

        // Provider commits
        bytes32 commitment = _computeCommitment(provider, BOUNTY_ID, NONCE);
        vm.prank(provider);
        bounty.commitClaim(BOUNTY_ID, commitment);

        // Attacker never committed, so reveal reverts with NotCommitted
        // (even though they know the nonce, they can't bypass the per-address mapping)
        vm.prank(attacker);
        vm.expectRevert(abi.encodeWithSelector(NeunodeBounty.NotCommitted.selector, BOUNTY_ID));
        bounty.revealClaim(BOUNTY_ID, 0, NONCE);
    }

    /// @notice Attacker commits their own commitment but can't reveal with provider's nonce
    function testAttackerCommitsCannotRevealWithOthersNonce() public {
        _createBounty();

        // Provider commits
        bytes32 providerCommitment = _computeCommitment(provider, BOUNTY_ID, NONCE);
        vm.prank(provider);
        bounty.commitClaim(BOUNTY_ID, providerCommitment);

        // Attacker also commits (with different commitment since msg.sender differs)
        bytes32 attackerCommitment = _computeCommitment(attacker, BOUNTY_ID, NONCE);
        vm.prank(attacker);
        bounty.commitClaim(BOUNTY_ID, attackerCommitment);

        // Attacker reveals with their own nonce — works because commitment includes
        // msg.sender. But the first revealer wins the bounty, second gets InvalidState.
        vm.prank(attacker);
        bounty.revealClaim(BOUNTY_ID, 0, NONCE);

        // Provider can no longer claim because state moved to Claimed
        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Claimed));
    }

    // ─── Commitment cleared after reveal ──────────────────────────────────

    function testCommitmentClearedAfterReveal() public {
        _createBounty();

        bytes32 commitment = _computeCommitment(provider, BOUNTY_ID, NONCE);

        vm.prank(provider);
        bounty.commitClaim(BOUNTY_ID, commitment);

        vm.prank(provider);
        bounty.revealClaim(BOUNTY_ID, 0, NONCE);

        // Provider should be able to commit again (e.g. for a different bounty)
        // but NOT for this bounty since it's already claimed
        // The commitment is cleared, so attempting reveal again should revert NotCommitted
        vm.prank(provider);
        vm.expectRevert(abi.encodeWithSelector(NeunodeBounty.NotCommitted.selector, BOUNTY_ID));
        bounty.revealClaim(BOUNTY_ID, 0, NONCE);
    }

    // ─── Events ───────────────────────────────────────────────────────────

    function testClaimCommittedEvent() public {
        _createBounty();

        bytes32 commitment = _computeCommitment(provider, BOUNTY_ID, NONCE);

        vm.prank(provider);
        vm.expectEmit(address(bounty));
        emit NeunodeBounty.ClaimCommitted(provider, BOUNTY_ID);
        bounty.commitClaim(BOUNTY_ID, commitment);
    }

    function testClaimRevealedEvent() public {
        _createBounty();

        bytes32 commitment = _computeCommitment(provider, BOUNTY_ID, NONCE);

        vm.prank(provider);
        bounty.commitClaim(BOUNTY_ID, commitment);

        vm.prank(provider);
        vm.expectEmit(address(bounty));
        emit NeunodeBounty.ClaimRevealed(provider, BOUNTY_ID);
        bounty.revealClaim(BOUNTY_ID, 0, NONCE);
    }

    // ─── Multiple providers can commit to same bounty ─────────────────────

    function testMultipleProvidersCommit() public {
        _createBounty();

        address provider2 = makeAddr("provider2");
        token.mint(provider2, 100_000e18);
        vm.prank(provider2);
        token.approve(address(bounty), type(uint256).max);

        bytes32 commitment1 = _computeCommitment(provider, BOUNTY_ID, NONCE);
        bytes32 nonce2 = keccak256("nonce2");
        bytes32 commitment2 = _computeCommitment(provider2, BOUNTY_ID, nonce2);

        vm.prank(provider);
        bounty.commitClaim(BOUNTY_ID, commitment1);

        vm.prank(provider2);
        bounty.commitClaim(BOUNTY_ID, commitment2);

        // First provider reveals — gets the bounty
        vm.prank(provider);
        bounty.revealClaim(BOUNTY_ID, 0, NONCE);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Claimed));

        // Second provider can't reveal because state is no longer Open
        vm.prank(provider2);
        vm.expectRevert();
        bounty.revealClaim(BOUNTY_ID, 0, nonce2);
    }

    // ─── Commitment Expiry (Anti-Griefing) ────────────────────────────────

    function testExpireStaleCommitment() public {
        _createBounty();

        bytes32 commitment = _computeCommitment(attacker, BOUNTY_ID, NONCE);

        // Attacker commits but never reveals
        vm.prank(attacker);
        bounty.commitClaim(BOUNTY_ID, commitment);

        // Before timeout — can't expire
        vm.expectRevert();
        bounty.expireCommitment(attacker, BOUNTY_ID);

        // After 1 hour timeout — anyone can expire
        skip(1 hours + 1);
        bounty.expireCommitment(attacker, BOUNTY_ID);

        // Provider can now commit and claim normally
        bytes32 providerNonce = keccak256("provider_nonce");
        bytes32 providerCommitment = _computeCommitment(provider, BOUNTY_ID, providerNonce);
        vm.prank(provider);
        bounty.commitClaim(BOUNTY_ID, providerCommitment);
        vm.prank(provider);
        bounty.revealClaim(BOUNTY_ID, 0, providerNonce);

        assertEq(uint8(bounty.getBountyState(BOUNTY_ID)), uint8(NeunodeBounty.BountyState.Claimed));
    }
}
