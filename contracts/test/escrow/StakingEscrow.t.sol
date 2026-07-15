// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";
import {StakingEscrow} from "../../src/escrow/StakingEscrow.sol";
import {ComputeToken} from "../../src/tokens/ComputeToken.sol";

contract StakingEscrowTest is Test {
    ComputeToken private token;
    StakingEscrow private escrow;

    address private alice;
    address private keeper;

    uint256 private constant STAKE = 10_000e18;

    function setUp() public {
        token = new ComputeToken();
        escrow = new StakingEscrow(address(token));
        alice = makeAddr("alice");
        keeper = makeAddr("keeper");

        token.mint(alice, STAKE);
        vm.prank(alice);
        token.stake(STAKE);
        token.grantRole(token.GOVERNANCE_ROLE(), address(escrow));
    }

    function testRevertConstructorZeroToken() public {
        vm.expectRevert(StakingEscrow.InvalidTokenAddress.selector);
        new StakingEscrow(address(0));
    }

    function testComputeDecayForEveryActivityLevel() public {
        assertEq(escrow.computeDecay(alice), 0);

        vm.warp(block.timestamp + 2 days);
        assertEq(escrow.computeDecay(alice), (STAKE * 5) / 10_000);

        vm.warp(block.timestamp + 6 days);
        assertEq(escrow.computeDecay(alice), (STAKE * 14) / 10_000);

        vm.warp(block.timestamp + 23 days);
        assertEq(escrow.computeDecay(alice), (STAKE * 41) / 10_000);

        vm.warp(block.timestamp + 60 days);
        assertEq(escrow.computeDecay(alice), (STAKE * 137) / 10_000);
    }

    function testAnyoneCanExecuteDecayAndAccountingMatchesStakeDelta() public {
        vm.warp(block.timestamp + 2 days);
        uint256 expected = (STAKE * 5) / 10_000;

        vm.expectEmit(true, false, false, true, address(escrow));
        emit StakingEscrow.DecayExecuted(alice, expected);
        vm.prank(keeper);
        escrow.executeDecay(alice);

        assertEq(token.stakedBalanceOf(alice), STAKE - expected);
        assertEq(escrow.lastDecayTimestamp(alice), block.timestamp);
    }

    function testRevertDecayMoreThanOncePerDay() public {
        vm.warp(block.timestamp + 2 days);
        escrow.executeDecay(alice);

        vm.warp(block.timestamp + 1 days - 1);
        vm.expectRevert(abi.encodeWithSelector(StakingEscrow.DecayTooSoon.selector, alice));
        escrow.executeDecay(alice);

        vm.warp(block.timestamp + 1);
        escrow.executeDecay(alice);
    }

    function testZeroDecayStillAdvancesCadenceWithoutSlashing() public {
        vm.warp(block.timestamp + 1 days);
        uint256 before = token.stakedBalanceOf(alice);

        escrow.executeDecay(alice);

        assertEq(token.stakedBalanceOf(alice), before);
        assertEq(escrow.lastDecayTimestamp(alice), block.timestamp);
        vm.expectRevert(abi.encodeWithSelector(StakingEscrow.DecayTooSoon.selector, alice));
        escrow.executeDecay(alice);
    }

    function testMissingSlashRoleRevertsWithoutAdvancingCadence() public {
        token.revokeRole(token.GOVERNANCE_ROLE(), address(escrow));
        vm.warp(block.timestamp + 2 days);

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                address(escrow),
                token.GOVERNANCE_ROLE()
            )
        );
        escrow.executeDecay(alice);

        assertEq(escrow.lastDecayTimestamp(alice), 0);
        assertEq(token.stakedBalanceOf(alice), STAKE);
    }

    function testSeedProtectionReportsActualZeroSlash() public {
        address seeded = makeAddr("seeded");
        token.mintSeed(seeded, STAKE);
        vm.warp(block.timestamp + 1 days);

        vm.expectEmit(true, false, false, true, address(escrow));
        emit StakingEscrow.DecayExecuted(seeded, 0);
        escrow.executeDecay(seeded);

        assertEq(token.stakedBalanceOf(seeded), STAKE);
        assertEq(escrow.lastDecayTimestamp(seeded), block.timestamp);
    }
}
