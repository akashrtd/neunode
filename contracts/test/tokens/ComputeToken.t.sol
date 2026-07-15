// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../../src/tokens/ComputeToken.sol";
import "../../src/tokens/TrainingToken.sol";
import "../../src/tokens/BandwidthToken.sol";
import "../../src/tokens/StorageToken.sol";

/// @title ComputeTokenTest — Tests for Neunode base token and ComputeToken
contract ComputeTokenTest is Test {
    ComputeToken public token;
    address public owner;
    address public alice;
    address public bob;

    function setUp() public {
        owner = address(this);
        alice = makeAddr("alice");
        bob = makeAddr("bob");
        token = new ComputeToken();
    }

    // ─── Constructor ──────────────────────────────────────────────────────

    function testConstructor() public view {
        assertEq(token.name(), "Neunode Compute");
        assertEq(token.symbol(), "nCompute");
        assertEq(token.decimals(), 18);
        assertEq(token.totalSupply(), 0);
        assertEq(token.supplyCap(), 1_000_000_000e18);
        assertEq(token.maxSupplyCap(), 10_000_000_000e18);
    }

    // ─── Mint ─────────────────────────────────────────────────────────────

    function testMint() public {
        token.mint(alice, 1000e18);
        assertEq(token.balanceOf(alice), 1000e18);
        assertEq(token.totalSupply(), 1000e18);
    }

    function testMintToSelf() public {
        token.mint(address(this), 500e18);
        assertEq(token.balanceOf(address(this)), 500e18);
    }

    function testRevertMintNotOwner() public {
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, alice));
        token.mint(alice, 100e18);
    }

    function testMintAtSupplyCap() public {
        token.mint(alice, token.supplyCap());
        assertEq(token.totalSupply(), token.supplyCap());
    }

    function testRevertMintAboveSupplyCap() public {
        uint256 cap = token.supplyCap();
        token.mint(alice, cap);

        vm.expectRevert(
            abi.encodeWithSelector(NeunodeToken.SupplyCapExceeded.selector, cap + 1, cap)
        );
        token.mint(alice, 1);
    }

    function testRevertSeedMintAboveSupplyCap() public {
        uint256 cap = token.supplyCap();
        token.mint(alice, cap);

        vm.expectRevert(
            abi.encodeWithSelector(NeunodeToken.SupplyCapExceeded.selector, cap + 1, cap)
        );
        token.mintSeed(bob, 1);
    }

    function testGovernanceCanRaiseSupplyCapWithinMaximum() public {
        uint256 oldCap = token.supplyCap();
        uint256 newCap = oldCap + 1_000_000e18;

        vm.expectEmit(false, false, false, true);
        emit INeunodeToken.SupplyCapUpdated(oldCap, newCap);
        token.setSupplyCap(newCap);

        token.mint(alice, newCap);
        assertEq(token.totalSupply(), newCap);
    }

    function testRevertSupplyCapAboveImmutableMaximum() public {
        uint256 maximum = token.maxSupplyCap();
        vm.expectRevert(
            abi.encodeWithSelector(
                NeunodeToken.SupplyCapAboveMaximum.selector, maximum + 1, maximum
            )
        );
        token.setSupplyCap(maximum + 1);
    }

    function testRevertSupplyCapBelowCurrentSupply() public {
        token.mint(alice, 100e18);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeToken.SupplyCapBelowCurrentSupply.selector, 99e18, 100e18)
        );
        token.setSupplyCap(99e18);
    }

    function testRevertSetSupplyCapWithoutGovernanceRole() public {
        bytes32 governanceRole = token.GOVERNANCE_ROLE();
        uint256 currentCap = token.supplyCap();
        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, alice, governanceRole
            )
        );
        token.setSupplyCap(currentCap);
    }

    function testBurnRestoresMintingCapacity() public {
        uint256 cap = token.supplyCap();
        token.mint(alice, cap);
        token.burn(alice, 100e18);
        token.mint(bob, 100e18);
        assertEq(token.totalSupply(), cap);
    }

    // ─── Burn ─────────────────────────────────────────────────────────────

    function testBurn() public {
        token.mint(alice, 1000e18);
        token.burn(alice, 400e18);
        assertEq(token.balanceOf(alice), 600e18);
        assertEq(token.totalSupply(), 600e18);
    }

    function testRevertBurnExceedsBalance() public {
        token.mint(alice, 100e18);
        vm.expectRevert();
        token.burn(alice, 200e18);
    }

    function testRevertBurnNotOwner() public {
        token.mint(alice, 100e18);
        vm.prank(bob);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, bob));
        token.burn(alice, 50e18);
    }

    // ─── Transfer ─────────────────────────────────────────────────────────

    function testTransfer() public {
        token.mint(alice, 1000e18);
        vm.prank(alice);
        token.transfer(bob, 300e18);
        assertEq(token.balanceOf(alice), 700e18);
        assertEq(token.balanceOf(bob), 300e18);
    }

    function testTransferFrom() public {
        token.mint(alice, 1000e18);
        vm.prank(alice);
        token.approve(bob, 500e18);
        vm.prank(bob);
        token.transferFrom(alice, bob, 200e18);
        assertEq(token.balanceOf(alice), 800e18);
        assertEq(token.balanceOf(bob), 200e18);
    }

    function testRevertTransferInsufficientBalance() public {
        token.mint(alice, 100e18);
        vm.prank(alice);
        vm.expectRevert();
        token.transfer(bob, 200e18);
    }

    // ─── Other tokens ─────────────────────────────────────────────────────

    function testTrainingToken() public {
        TrainingToken t = new TrainingToken();
        assertEq(t.name(), "Neunode Training");
        assertEq(t.symbol(), "nTrain");
    }

    function testBandwidthToken() public {
        BandwidthToken t = new BandwidthToken();
        assertEq(t.name(), "Neunode Bandwidth");
        assertEq(t.symbol(), "nBandwidth");
    }

    function testStorageToken() public {
        StorageToken t = new StorageToken();
        assertEq(t.name(), "Neunode Storage");
        assertEq(t.symbol(), "nStorage");
    }
}
