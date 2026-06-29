// SPDX-License-Identifier: AGPL-3.0-or-later
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

    // Removed ACTIVITY TRACKING and DECAY tests as Native Decay was decoupled into StakingEscrow.

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

    // DECAY CONFIG tests removed
}
