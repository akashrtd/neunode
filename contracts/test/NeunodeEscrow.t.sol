// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/NeunodeEscrow.sol";
import "../src/tokens/ComputeToken.sol";

/// @title NeunodeEscrowTest — Tests for bilateral escrow
contract NeunodeEscrowTest is Test {
    NeunodeEscrow public escrow;
    ComputeToken public token;

    address public requester;
    address public provider;
    address public attacker;

    bytes32 constant BOUNTY_ID = keccak256("test_bounty_1");
    uint256 constant AMOUNT = 1000e18;
    uint256 constant BOND = 150e18; // 15% of 1000

    function setUp() public {
        escrow = new NeunodeEscrow();
        token = new ComputeToken();

        requester = makeAddr("requester");
        provider = makeAddr("provider");
        attacker = makeAddr("attacker");

        // Mint tokens to participants
        token.mint(requester, 10_000e18);
        token.mint(provider, 10_000e18);

        // Approve escrow contract
        vm.prank(requester);
        token.approve(address(escrow), type(uint256).max);
        vm.prank(provider);
        token.approve(address(escrow), type(uint256).max);
    }

    // ─── Create Escrow ────────────────────────────────────────────────────

    function testCreateEscrow() public {
        vm.prank(requester);
        escrow.createEscrow(BOUNTY_ID, address(token), AMOUNT, block.timestamp + 7 days);

        assertEq(token.balanceOf(address(escrow)), AMOUNT);

        (
            bytes32 bountyId,
            address req,
            address prov,
            address tkn,
            uint256 amount,
            uint256 bond,
            uint256 created,
            uint256 deadline,
            NeunodeEscrow.EscrowState state
        ) = escrow.escrows(BOUNTY_ID);

        assertEq(bountyId, BOUNTY_ID);
        assertEq(req, requester);
        assertEq(prov, address(0));
        assertEq(tkn, address(token));
        assertEq(amount, AMOUNT);
        assertEq(bond, 0);
        assertEq(uint8(state), uint8(NeunodeEscrow.EscrowState.Created));
    }

    function testRevertCreateEscrowZeroAmount() public {
        vm.prank(requester);
        vm.expectRevert(NeunodeEscrow.InvalidAmount.selector);
        escrow.createEscrow(BOUNTY_ID, address(token), 0, block.timestamp + 7 days);
    }

    function testRevertCreateEscrowZeroToken() public {
        vm.prank(requester);
        vm.expectRevert(NeunodeEscrow.InvalidToken.selector);
        escrow.createEscrow(BOUNTY_ID, address(0), AMOUNT, block.timestamp + 7 days);
    }

    // ─── Fund Escrow (Provider bonds) ─────────────────────────────────────

    function testFundEscrow() public {
        vm.prank(requester);
        escrow.createEscrow(BOUNTY_ID, address(token), AMOUNT, block.timestamp + 7 days);

        vm.prank(provider);
        escrow.fundEscrow(BOUNTY_ID, BOND);

        assertEq(token.balanceOf(address(escrow)), AMOUNT + BOND);

        (,, address prov,,,,,, NeunodeEscrow.EscrowState state) = escrow.escrows(BOUNTY_ID);
        assertEq(prov, provider);
        assertEq(uint8(state), uint8(NeunodeEscrow.EscrowState.Funded));
    }

    function testRevertFundLowBond() public {
        vm.prank(requester);
        escrow.createEscrow(BOUNTY_ID, address(token), AMOUNT, block.timestamp + 7 days);

        vm.prank(provider);
        vm.expectRevert(NeunodeEscrow.InvalidAmount.selector);
        escrow.fundEscrow(BOUNTY_ID, 10e18); // Too low
    }

    // ─── Release (Happy Path) ─────────────────────────────────────────────

    function testRelease() public {
        vm.prank(requester);
        escrow.createEscrow(BOUNTY_ID, address(token), AMOUNT, block.timestamp + 7 days);

        vm.prank(provider);
        escrow.fundEscrow(BOUNTY_ID, BOND);

        vm.prank(requester);
        escrow.release(BOUNTY_ID);

        // Provider gets payment + bond back
        assertEq(token.balanceOf(provider), 10_000e18 - BOND + AMOUNT + BOND);
        assertEq(token.balanceOf(address(escrow)), 0);
    }

    function testRevertReleaseNotRequester() public {
        vm.prank(requester);
        escrow.createEscrow(BOUNTY_ID, address(token), AMOUNT, block.timestamp + 7 days);

        vm.prank(provider);
        escrow.fundEscrow(BOUNTY_ID, BOND);

        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeEscrow.NotRequester.selector, BOUNTY_ID, attacker)
        );
        escrow.release(BOUNTY_ID);
    }

    // ─── Refund ───────────────────────────────────────────────────────────

    function testRefund() public {
        vm.prank(requester);
        escrow.createEscrow(BOUNTY_ID, address(token), AMOUNT, block.timestamp + 7 days);

        vm.prank(provider);
        escrow.fundEscrow(BOUNTY_ID, BOND);

        uint256 requesterBalBefore = token.balanceOf(requester);

        vm.prank(requester);
        escrow.refund(BOUNTY_ID);

        // Requester gets payment + slashed bond
        assertEq(token.balanceOf(requester), requesterBalBefore + AMOUNT + BOND);
        assertEq(token.balanceOf(address(escrow)), 0);
    }

    // ─── Dispute ──────────────────────────────────────────────────────────

    function testDisputeByRequester() public {
        vm.prank(requester);
        escrow.createEscrow(BOUNTY_ID, address(token), AMOUNT, block.timestamp + 7 days);

        vm.prank(provider);
        escrow.fundEscrow(BOUNTY_ID, BOND);

        vm.prank(requester);
        escrow.dispute(BOUNTY_ID);

        assertEq(uint8(escrow.getEscrowState(BOUNTY_ID)), uint8(NeunodeEscrow.EscrowState.Disputed));
    }

    function testDisputeByProvider() public {
        vm.prank(requester);
        escrow.createEscrow(BOUNTY_ID, address(token), AMOUNT, block.timestamp + 7 days);

        vm.prank(provider);
        escrow.fundEscrow(BOUNTY_ID, BOND);

        vm.prank(provider);
        escrow.dispute(BOUNTY_ID);

        assertEq(uint8(escrow.getEscrowState(BOUNTY_ID)), uint8(NeunodeEscrow.EscrowState.Disputed));
    }

    function testRevertDisputeByAttacker() public {
        vm.prank(requester);
        escrow.createEscrow(BOUNTY_ID, address(token), AMOUNT, block.timestamp + 7 days);

        vm.prank(provider);
        escrow.fundEscrow(BOUNTY_ID, BOND);

        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeEscrow.NotProvider.selector, BOUNTY_ID, attacker)
        );
        escrow.dispute(BOUNTY_ID);
    }

    // ─── Full Lifecycle ───────────────────────────────────────────────────

    function testFullLifecycleCreateFundRelease() public {
        uint256 providerBalBefore = token.balanceOf(provider);

        vm.prank(requester);
        escrow.createEscrow(BOUNTY_ID, address(token), AMOUNT, block.timestamp + 7 days);

        vm.prank(provider);
        escrow.fundEscrow(BOUNTY_ID, BOND);

        vm.prank(requester);
        escrow.release(BOUNTY_ID);

        // Provider gained AMOUNT (bond returned + payment)
        assertEq(token.balanceOf(provider) - providerBalBefore, AMOUNT);
    }

    // ─── Zero Address Fee Recipient Prevention ────────────────────────────

    function testRevertZeroAddressProtocolFeeRecipient() public {
        _createAndFund();
        escrow.grantRole(escrow.BOUNTY_CONTRACT_ROLE(), address(this));

        vm.expectRevert(NeunodeEscrow.ZeroAddressFeeRecipient.selector);
        escrow.releaseWithFees(
            BOUNTY_ID,
            provider,
            200, // 2% protocol
            0,
            0,
            address(0), // zero address — should revert
            makeAddr("rev"),
            makeAddr("ver")
        );
    }

    // ─── Auto-Refund After Inactivity ─────────────────────────────────────

    function testAutoRefundAfterTimeout() public {
        _createAndFund();

        uint256 requesterBalBefore = token.balanceOf(requester);
        uint256 providerBalBefore = token.balanceOf(provider);

        // Warp past deadline + 7 day timeout
        vm.warp(block.timestamp + 7 days + 8 days);

        escrow.autoRefund(BOUNTY_ID, 7 days);

        assertEq(uint8(escrow.getEscrowState(BOUNTY_ID)), uint8(NeunodeEscrow.EscrowState.Refunded));
        assertEq(token.balanceOf(requester) - requesterBalBefore, AMOUNT);
        assertEq(token.balanceOf(provider) - providerBalBefore, BOND);
    }

    function testRevertAutoRefundBeforeTimeout() public {
        _createAndFund();

        vm.expectRevert();
        escrow.autoRefund(BOUNTY_ID, 30 days);
    }

    function _createAndFund() internal {
        vm.prank(requester);
        escrow.createEscrow(BOUNTY_ID, address(token), AMOUNT, block.timestamp + 7 days);

        vm.prank(provider);
        escrow.fundEscrow(BOUNTY_ID, BOND);
    }
}
