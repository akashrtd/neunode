// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../../src/tokens/ComputeToken.sol";
import "../../src/tokens/TrainingToken.sol";
import "../../src/tokens/BandwidthToken.sol";
import "../../src/tokens/StorageToken.sol";
import "../../src/interfaces/INeunodeToken.sol";

/// @title NeunodeTokenEnhancedTest — Tests for AccessControl, staking, activity, decay, seed tokens
contract NeunodeTokenEnhancedTest is Test {
    ComputeToken public token;
    address public owner;
    address public alice;
    address public bob;
    address public treasury;
    address public stakingRewards;
    address public devFund;
    address public minter;

    bytes32 constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 constant BURNER_ROLE = keccak256("BURNER_ROLE");
    bytes32 constant GOVERNANCE_ROLE = keccak256("GOVERNANCE_ROLE");
    bytes32 constant DEFAULT_ADMIN_ROLE = 0x00;

    function setUp() public {
        owner = address(this);
        alice = makeAddr("alice");
        bob = makeAddr("bob");
        treasury = makeAddr("treasury");
        stakingRewards = makeAddr("stakingRewards");
        devFund = makeAddr("devFund");
        minter = makeAddr("minter");

        token = new ComputeToken();
        token.setDecayConfig(treasury, stakingRewards, devFund);
    }

    // ─── Helpers ──────────────────────────────────────────────────────────

    function _mintTo(address to, uint256 amount) internal {
        token.mint(to, amount);
    }

    // ═══════════════════════════════════════════════════════════════════════
    //   ROLES (6 tests)
    // ═══════════════════════════════════════════════════════════════════════

    function testDeployerHasAllRoles() public view {
        assertTrue(token.hasRole(DEFAULT_ADMIN_ROLE, owner));
        assertTrue(token.hasRole(MINTER_ROLE, owner));
        assertTrue(token.hasRole(BURNER_ROLE, owner));
        assertTrue(token.hasRole(GOVERNANCE_ROLE, owner));
    }

    function testRoleConstants() public view {
        assertEq(token.MINTER_ROLE(), MINTER_ROLE);
        assertEq(token.BURNER_ROLE(), BURNER_ROLE);
        assertEq(token.GOVERNANCE_ROLE(), GOVERNANCE_ROLE);
    }

    function testGrantAndUseMinterRole() public {
        token.grantRole(MINTER_ROLE, minter);
        assertTrue(token.hasRole(MINTER_ROLE, minter));

        vm.prank(minter);
        token.mint(alice, 1000e18);
        assertEq(token.balanceOf(alice), 1000e18);
    }

    function testGrantAndUseBurnerRole() public {
        _mintTo(alice, 1000e18);
        address burner = makeAddr("burner");
        token.grantRole(BURNER_ROLE, burner);

        vm.prank(burner);
        token.burn(alice, 400e18);
        assertEq(token.balanceOf(alice), 600e18);
    }

    function testRevertMintWithoutRole() public {
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, alice));
        token.mint(alice, 100e18);
    }

    function testRevertBurnWithoutRole() public {
        _mintTo(alice, 1000e18);
        vm.prank(bob);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, bob));
        token.burn(alice, 100e18);
    }

    // ═══════════════════════════════════════════════════════════════════════
    //   STAKING (6 tests)
    // ═══════════════════════════════════════════════════════════════════════

    function testStake() public {
        _mintTo(alice, 1000e18);
        vm.prank(alice);
        token.stake(500e18);

        assertEq(token.stakedBalanceOf(alice), 500e18);
        assertEq(token.balanceOf(alice), 500e18);
        assertEq(token.balanceOf(address(token)), 500e18);
    }

    function testUnstake() public {
        _mintTo(alice, 1000e18);
        vm.prank(alice);
        token.stake(500e18);

        vm.prank(alice);
        token.unstake(500e18);

        assertEq(token.stakedBalanceOf(alice), 0);
        assertEq(token.balanceOf(alice), 1000e18);
    }

    function testStakeMultipleTimes() public {
        _mintTo(alice, 1000e18);
        vm.startPrank(alice);
        token.stake(200e18);
        token.stake(300e18);
        vm.stopPrank();

        assertEq(token.stakedBalanceOf(alice), 500e18);
        assertEq(token.balanceOf(alice), 500e18);
    }

    function testRevertStakeInsufficientBalance() public {
        _mintTo(alice, 100e18);
        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeToken.InsufficientBalance.selector, alice, 200e18)
        );
        token.stake(200e18);
    }

    function testRevertUnstakeMoreThanStaked() public {
        _mintTo(alice, 1000e18);
        vm.prank(alice);
        token.stake(500e18);

        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeToken.InsufficientStake.selector, alice, 600e18)
        );
        token.unstake(600e18);
    }

    function testPartialUnstake() public {
        _mintTo(alice, 1000e18);
        vm.prank(alice);
        token.stake(500e18);

        vm.prank(alice);
        token.unstake(200e18);

        assertEq(token.stakedBalanceOf(alice), 300e18);
        assertEq(token.balanceOf(alice), 700e18);
    }

    // ═══════════════════════════════════════════════════════════════════════
    //   ACTIVITY TRACKING (6 tests)
    // ═══════════════════════════════════════════════════════════════════════

    function testActivityUpdatedOnTransfer() public {
        _mintTo(alice, 1000e18);
        uint256 before = block.timestamp;

        vm.prank(alice);
        token.transfer(bob, 100e18);

        assertEq(token.lastActivity(alice), before);
        assertEq(token.lastActivity(bob), before);
    }

    function testActivityLevelActive() public {
        _mintTo(alice, 100e18); // triggers _transfer from address(0) — no activity
        vm.prank(alice);
        token.transfer(bob, 10e18); // now alice and bob are active

        assertEq(token.getActivityLevel(alice), 0); // Active
    }

    function testActivityLevelModerate() public {
        _mintTo(alice, 100e18);
        vm.prank(alice);
        token.transfer(bob, 10e18); // sets activity

        vm.warp(block.timestamp + 3 days);
        assertEq(token.getActivityLevel(alice), 1); // Moderate (>1 day)
    }

    function testActivityLevelLow() public {
        _mintTo(alice, 100e18);
        vm.prank(alice);
        token.transfer(bob, 10e18);

        vm.warp(block.timestamp + 15 days);
        assertEq(token.getActivityLevel(alice), 2); // Low (>7 days)
    }

    function testActivityLevelInactive() public {
        _mintTo(alice, 100e18);
        vm.prank(alice);
        token.transfer(bob, 10e18);

        vm.warp(block.timestamp + 45 days);
        assertEq(token.getActivityLevel(alice), 3); // Inactive (>30 days)
    }

    function testActivityLevelDead() public {
        // Alice has never interacted (no activity timestamp)
        assertEq(token.getActivityLevel(alice), 4); // Dead (never active)
    }

    // ─── Activity Access Control ──────────────────────────────────────────

    function testRevertUpdateActivityOther() public {
        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeToken.UnauthorizedActivityUpdate.selector, alice, bob)
        );
        token.updateActivity(bob);
    }

    function testUpdateActivitySelf() public {
        vm.prank(alice);
        token.updateActivity(alice);
        assertEq(token.lastActivity(alice), block.timestamp);
    }

    function testRevertExecuteDecayOther() public {
        _mintTo(alice, 10000e18);
        vm.prank(alice);
        token.transfer(bob, 1e18);

        vm.warp(block.timestamp + 3 days);

        vm.prank(bob);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeToken.UnauthorizedActivityUpdate.selector, bob, alice)
        );
        token.executeDecay(alice);
    }

    // ═══════════════════════════════════════════════════════════════════════
    //   DECAY (7 tests)
    // ═══════════════════════════════════════════════════════════════════════

    function testComputeDecayActive() public {
        _mintTo(alice, 10000e18);
        vm.prank(alice);
        token.transfer(bob, 1e18); // makes alice Active

        assertEq(token.computeDecay(alice), 0);
    }

    function testComputeDecayModerate() public {
        _mintTo(alice, 10000e18);
        vm.prank(alice);
        token.transfer(bob, 1e18);

        vm.warp(block.timestamp + 3 days);
        // Alice balance = 9999e18 (transferred 1e18 to bob)
        // Rate = 5 bps = 0.05%
        // 9999e18 * 5 / 10000 = 4999500000000000000
        assertEq(token.computeDecay(alice), 9999e18 * 5 / 10000);
    }

    function testComputeDecayLow() public {
        _mintTo(alice, 10000e18);
        vm.prank(alice);
        token.transfer(bob, 1e18);

        vm.warp(block.timestamp + 15 days);
        // Rate = 14 bps = 0.14%
        // 9999e18 * 14 / 10000
        assertEq(token.computeDecay(alice), 9999e18 * 14 / 10000);
    }

    function testComputeDecayInactive() public {
        _mintTo(alice, 10000e18);
        vm.prank(alice);
        token.transfer(bob, 1e18);

        vm.warp(block.timestamp + 45 days);
        // Rate = 41 bps = 0.41%
        // 9999e18 * 41 / 10000
        assertEq(token.computeDecay(alice), 9999e18 * 41 / 10000);
    }

    function testComputeDecayDead() public {
        _mintTo(alice, 10000e18);
        vm.prank(alice);
        token.transfer(bob, 1e18);

        vm.warp(block.timestamp + 100 days);
        // Rate = 137 bps = 1.37%
        // 9999e18 * 137 / 10000
        assertEq(token.computeDecay(alice), 9999e18 * 137 / 10000);
    }

    function testExecuteDecayDistribution() public {
        _mintTo(alice, 10000e18);
        vm.prank(alice);
        token.transfer(bob, 1e18); // set activity

        vm.warp(block.timestamp + 3 days); // Moderate → decay

        uint256 aliceBalBefore = token.balanceOf(alice);
        uint256 decayAmount = token.computeDecay(alice);
        assertGt(decayAmount, 0);

        vm.prank(alice);
        token.executeDecay(alice);

        // Distribution: 40% treasury, 30% staking, 20% burn, 10% dev
        uint256 expectedTreasury = (decayAmount * 40) / 100;
        uint256 expectedStaking = (decayAmount * 30) / 100;
        uint256 expectedBurn = (decayAmount * 20) / 100;
        uint256 expectedDev = decayAmount - expectedTreasury - expectedStaking - expectedBurn;

        assertEq(token.balanceOf(treasury), expectedTreasury);
        assertEq(token.balanceOf(stakingRewards), expectedStaking);
        assertEq(token.balanceOf(devFund), expectedDev);
        assertEq(token.balanceOf(alice), aliceBalBefore - decayAmount);
    }

    function testRevertDecayTooSoon() public {
        _mintTo(alice, 10000e18);
        vm.prank(alice);
        token.transfer(bob, 1e18);

        vm.warp(block.timestamp + 3 days);
        vm.prank(alice);
        token.executeDecay(alice); // first decay OK

        // Try again immediately — should fail
        vm.warp(block.timestamp + 12 hours);
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(NeunodeToken.DecayTooSoon.selector, alice));
        token.executeDecay(alice);
    }

    // ═══════════════════════════════════════════════════════════════════════
    //   SEED TOKENS (6 tests)
    // ═══════════════════════════════════════════════════════════════════════

    function testMintSeed() public {
        token.mintSeed(alice, 500e18);

        assertEq(token.stakedBalanceOf(alice), 500e18);
        assertEq(token.seedBalanceOf(alice), 500e18);
        // Tokens are minted to the contract and staked
        assertEq(token.balanceOf(address(token)), 500e18);
    }

    function testSeedLocksUnstake() public {
        token.mintSeed(alice, 500e18);
        _mintTo(alice, 300e18); // extra spendable tokens
        vm.prank(alice);
        token.stake(300e18); // stake extra

        // Total staked: 800 (500 seed + 300 normal)
        // Trying to unstake 600 would go below seed lock of 500
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(NeunodeToken.CannotUnstakeSeed.selector));
        token.unstake(600e18);
    }

    function testActivateSeedAllowsUnstake() public {
        token.mintSeed(alice, 500e18);
        _mintTo(alice, 300e18);
        vm.prank(alice);
        token.stake(300e18);

        // Activate seed — removes lock
        token.activateSeed(alice);
        assertEq(token.seedBalanceOf(alice), 0);

        // Now can unstake everything
        vm.prank(alice);
        token.unstake(800e18);
        assertEq(token.stakedBalanceOf(alice), 0);
        assertEq(token.balanceOf(alice), 800e18);
    }

    function testRevertMintSeedNotMinter() public {
        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, alice, MINTER_ROLE
            )
        );
        token.mintSeed(alice, 100e18);
    }

    function testRevertActivateSeedNotGovernance() public {
        token.mintSeed(alice, 500e18);

        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, alice, GOVERNANCE_ROLE
            )
        );
        token.activateSeed(alice);
    }

    function testSeedPartialUnstakeAfterActivation() public {
        token.mintSeed(alice, 500e18);
        token.activateSeed(alice);

        // Seed is activated, so can unstake partially
        vm.prank(alice);
        token.unstake(200e18);

        assertEq(token.stakedBalanceOf(alice), 300e18);
        assertEq(token.balanceOf(alice), 200e18);
    }

    // ═══════════════════════════════════════════════════════════════════════
    //   DECAY CONFIG (2 tests)
    // ═══════════════════════════════════════════════════════════════════════

    function testSetDecayConfig() public {
        address newTreasury = makeAddr("newTreasury");
        address newStaking = makeAddr("newStaking");
        address newDev = makeAddr("newDev");

        token.setDecayConfig(newTreasury, newStaking, newDev);

        (address t, address s, address d) = token.decayConfig();
        assertEq(t, newTreasury);
        assertEq(s, newStaking);
        assertEq(d, newDev);
    }

    function testRevertSetDecayConfigNotGovernance() public {
        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, alice, GOVERNANCE_ROLE
            )
        );
        token.setDecayConfig(treasury, stakingRewards, devFund);
    }
}

/// @dev Imports for error selectors
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/access/IAccessControl.sol";
import "../../src/tokens/NeunodeToken.sol";
