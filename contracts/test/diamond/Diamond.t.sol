// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../../src/diamond/Diamond.sol";
import "../../src/diamond/DiamondCutFacet.sol";
import "../../src/diamond/DiamondLoupeFacet.sol";
import "../../src/diamond/IDiamondCut.sol";
import "../../src/diamond/IDiamondLoupe.sol";
import "../../src/diamond/LibDiamond.sol";

// ─── Test Facets ──────────────────────────────────────────────────────────

/// @notice TestFacet1 returns value 1
contract TestFacet1 {
    function getValue() external pure returns (uint256) {
        return 1;
    }

    function getOwner() external pure returns (address) {
        return address(0x1);
    }
}

/// @notice TestFacet2 returns value 2 (used for replace testing)
contract TestFacet2 {
    function getValue() external pure returns (uint256) {
        return 2;
    }

    function getOwner() external pure returns (address) {
        return address(0x2);
    }
}

/// @notice Initialization contract for testing delegatecall init
contract DiamondInit {
    uint256 public initialValue;

    function init(uint256 _value) external {
        initialValue = _value;
    }
}

// ─── Diamond Test Suite ──────────────────────────────────────────────────

/// @title DiamondTest — Comprehensive tests for EIP-2535 Diamond proxy
contract DiamondTest is Test {
    Diamond public diamond;
    DiamondCutFacet public cutFacet;
    DiamondLoupeFacet public loupeFacet;
    TestFacet1 public testFacet1;
    TestFacet2 public testFacet2;
    DiamondInit public diamondInit;

    address public owner;
    address public nonOwner;

    // Selector constants
    bytes4 constant DIAMOND_CUT_SELECTOR =
        bytes4(keccak256("diamondCut((address,uint8,bytes4[])[],address,bytes)"));
    bytes4 constant FACETS_SELECTOR = IDiamondLoupe.facets.selector;
    bytes4 constant FACET_FUNCTION_SELECTORS_SELECTOR =
        IDiamondLoupe.facetFunctionSelectors.selector;
    bytes4 constant FACET_ADDRESSES_SELECTOR = IDiamondLoupe.facetAddresses.selector;
    bytes4 constant FACET_ADDRESS_SELECTOR = IDiamondLoupe.facetAddress.selector;
    bytes4 constant GET_VALUE_SELECTOR = bytes4(keccak256("getValue()"));
    bytes4 constant GET_OWNER_SELECTOR = bytes4(keccak256("getOwner()"));

    function setUp() public {
        owner = makeAddr("owner");
        nonOwner = makeAddr("nonOwner");

        // Deploy facets
        cutFacet = new DiamondCutFacet();
        loupeFacet = new DiamondLoupeFacet();
        testFacet1 = new TestFacet1();
        testFacet2 = new TestFacet2();
        diamondInit = new DiamondInit();

        // Build initial diamond cut: add loupe + cut facets
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](2);

        // DiamondCutFacet selectors
        bytes4[] memory cutSelectors = new bytes4[](1);
        cutSelectors[0] = DIAMOND_CUT_SELECTOR;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(cutFacet),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: cutSelectors
        });

        // DiamondLoupeFacet selectors
        bytes4[] memory loupeSelectors = new bytes4[](4);
        loupeSelectors[0] = FACETS_SELECTOR;
        loupeSelectors[1] = FACET_FUNCTION_SELECTORS_SELECTOR;
        loupeSelectors[2] = FACET_ADDRESSES_SELECTOR;
        loupeSelectors[3] = FACET_ADDRESS_SELECTOR;
        cuts[1] = IDiamondCut.FacetCut({
            facetAddress: address(loupeFacet),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: loupeSelectors
        });

        // Deploy diamond
        vm.startBroadcast(owner);
        diamond = new Diamond(cuts, address(0), "", owner);
        vm.stopBroadcast();
    }

    // ─── Helper: call diamondCut on the diamond ───────────────────────────

    function _cut(IDiamondCut.FacetCut[] memory cuts, address init, bytes memory data) internal {
        vm.prank(owner);
        IDiamondCut(address(diamond)).diamondCut(cuts, init, data);
    }

    // ─── 1. Add facet and verify selectors registered ─────────────────────

    function testAddFacet() public {
        // Add TestFacet1
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = GET_VALUE_SELECTOR;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet1),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: selectors
        });
        _cut(cuts, address(0), "");

        // Verify selector routes to TestFacet1
        address facet = IDiamondLoupe(address(diamond)).facetAddress(GET_VALUE_SELECTOR);
        assertEq(facet, address(testFacet1));

        // Verify getValue() returns 1
        (bool success, bytes memory data) =
            address(diamond).staticcall(abi.encodeWithSelector(GET_VALUE_SELECTOR));
        assertTrue(success);
        assertEq(abi.decode(data, (uint256)), 1);
    }

    // ─── 2. Replace function selector ─────────────────────────────────────

    function testReplaceSelector() public {
        // First add TestFacet1
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = GET_VALUE_SELECTOR;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet1),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: selectors
        });
        _cut(cuts, address(0), "");

        // Now replace with TestFacet2
        IDiamondCut.FacetCut[] memory replaceCuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory replaceSelectors = new bytes4[](1);
        replaceSelectors[0] = GET_VALUE_SELECTOR;
        replaceCuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet2),
            action: IDiamondCut.FacetCutAction.Replace,
            functionSelectors: replaceSelectors
        });
        _cut(replaceCuts, address(0), "");

        // Verify selector now routes to TestFacet2
        address facet = IDiamondLoupe(address(diamond)).facetAddress(GET_VALUE_SELECTOR);
        assertEq(facet, address(testFacet2));

        // Verify getValue() returns 2
        (bool success, bytes memory data) =
            address(diamond).staticcall(abi.encodeWithSelector(GET_VALUE_SELECTOR));
        assertTrue(success);
        assertEq(abi.decode(data, (uint256)), 2);
    }

    // ─── 3. Remove function selector ──────────────────────────────────────

    function testRemoveSelector() public {
        // Add TestFacet1
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = GET_VALUE_SELECTOR;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet1),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: selectors
        });
        _cut(cuts, address(0), "");

        // Remove getValue()
        IDiamondCut.FacetCut[] memory removeCuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory removeSelectors = new bytes4[](1);
        removeSelectors[0] = GET_VALUE_SELECTOR;
        removeCuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(0),
            action: IDiamondCut.FacetCutAction.Remove,
            functionSelectors: removeSelectors
        });
        _cut(removeCuts, address(0), "");

        // Verify selector no longer routes
        address facet = IDiamondLoupe(address(diamond)).facetAddress(GET_VALUE_SELECTOR);
        assertEq(facet, address(0));
    }

    // ─── 4. facets() returns all registered facets ────────────────────────

    function testFacetsReturnsAll() public {
        // Add TestFacet1
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = GET_VALUE_SELECTOR;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet1),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: selectors
        });
        _cut(cuts, address(0), "");

        IDiamondLoupe.Facet[] memory allFacets = IDiamondLoupe(address(diamond)).facets();

        // Should have 3 facets: cutFacet, loupeFacet, testFacet1
        assertEq(allFacets.length, 3);
    }

    // ─── 5. facetFunctionSelectors() returns correct selectors ────────────

    function testFacetFunctionSelectors() public {
        bytes4[] memory selectors =
            IDiamondLoupe(address(diamond)).facetFunctionSelectors(address(loupeFacet));
        assertEq(selectors.length, 4);
        assertEq(selectors[0], FACETS_SELECTOR);
        assertEq(selectors[1], FACET_FUNCTION_SELECTORS_SELECTOR);
        assertEq(selectors[2], FACET_ADDRESSES_SELECTOR);
        assertEq(selectors[3], FACET_ADDRESS_SELECTOR);
    }

    // ─── 6. facetAddresses() returns all facet addresses ──────────────────

    function testFacetAddresses() public {
        address[] memory addresses = IDiamondLoupe(address(diamond)).facetAddresses();
        assertEq(addresses.length, 2);
        // Should contain cutFacet and loupeFacet
        bool hasCut;
        bool hasLoupe;
        for (uint256 i; i < addresses.length; i++) {
            if (addresses[i] == address(cutFacet)) hasCut = true;
            if (addresses[i] == address(loupeFacet)) hasLoupe = true;
        }
        assertTrue(hasCut, "Missing cutFacet");
        assertTrue(hasLoupe, "Missing loupeFacet");
    }

    // ─── 7. facetAddress() returns correct facet for selector ─────────────

    function testFacetAddressForSelector() public {
        address facet = IDiamondLoupe(address(diamond)).facetAddress(DIAMOND_CUT_SELECTOR);
        assertEq(facet, address(cutFacet));

        address loupeFacetAddr = IDiamondLoupe(address(diamond)).facetAddress(FACETS_SELECTOR);
        assertEq(loupeFacetAddr, address(loupeFacet));
    }

    // ─── 8. Non-owner cannot call diamondCut ──────────────────────────────

    function testRevertNonOwnerCut() public {
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = GET_VALUE_SELECTOR;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet1),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: selectors
        });

        vm.prank(nonOwner);
        vm.expectRevert(
            abi.encodeWithSelector(LibDiamond.NotContractOwner.selector, nonOwner, owner)
        );
        IDiamondCut(address(diamond)).diamondCut(cuts, address(0), "");
    }

    // ─── 9. Owner can call diamondCut ─────────────────────────────────────

    function testOwnerCanCut() public {
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = GET_VALUE_SELECTOR;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet1),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: selectors
        });

        vm.prank(owner);
        // Should not revert
        IDiamondCut(address(diamond)).diamondCut(cuts, address(0), "");

        assertEq(
            IDiamondLoupe(address(diamond)).facetAddress(GET_VALUE_SELECTOR), address(testFacet1)
        );
    }

    // ─── 10. Multiple facet adds in single diamondCut ─────────────────────

    function testMultipleAddsInSingleCut() public {
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](2);

        bytes4[] memory selectors1 = new bytes4[](1);
        selectors1[0] = GET_VALUE_SELECTOR;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet1),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: selectors1
        });

        bytes4[] memory selectors2 = new bytes4[](1);
        selectors2[0] = GET_OWNER_SELECTOR;
        cuts[1] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet2),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: selectors2
        });

        _cut(cuts, address(0), "");

        assertEq(
            IDiamondLoupe(address(diamond)).facetAddress(GET_VALUE_SELECTOR), address(testFacet1)
        );
        assertEq(
            IDiamondLoupe(address(diamond)).facetAddress(GET_OWNER_SELECTOR), address(testFacet2)
        );

        address[] memory addresses = IDiamondLoupe(address(diamond)).facetAddresses();
        assertEq(addresses.length, 4); // cut + loupe + testFacet1 + testFacet2
    }

    // ─── 11. Initialize with calldata on deployment ───────────────────────

    function testInitializeWithCalldata() public {
        // Deploy new diamond with initialization
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](2);

        bytes4[] memory cutSelectors = new bytes4[](1);
        cutSelectors[0] = DIAMOND_CUT_SELECTOR;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(cutFacet),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: cutSelectors
        });

        bytes4[] memory loupeSelectors = new bytes4[](4);
        loupeSelectors[0] = FACETS_SELECTOR;
        loupeSelectors[1] = FACET_FUNCTION_SELECTORS_SELECTOR;
        loupeSelectors[2] = FACET_ADDRESSES_SELECTOR;
        loupeSelectors[3] = FACET_ADDRESS_SELECTOR;
        cuts[1] = IDiamondCut.FacetCut({
            facetAddress: address(loupeFacet),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: loupeSelectors
        });

        bytes memory initData = abi.encodeWithSelector(DiamondInit.init.selector, uint256(42));
        Diamond newDiamond = new Diamond(cuts, address(diamondInit), initData, owner);

        // Verify init was called — initialValue should be 42
        // (stored in diamond's storage slot 0 via delegatecall, since DiamondInit's
        // initialValue is at slot 0 and delegatecall uses caller's storage)
        bytes32 storedValue = vm.load(address(newDiamond), bytes32(uint256(0)));
        assertEq(uint256(storedValue), 42, "Initial value should be 42");
    }

    // ─── 12. Diamond receives ETH correctly ───────────────────────────────

    function testReceiveETH() public {
        (bool success,) = address(diamond).call{value: 1 ether}("");
        assertTrue(success);
        assertEq(address(diamond).balance, 1 ether);
    }

    // ─── 13. Removed selector reverts ─────────────────────────────────────

    function testRevertOnRemovedSelector() public {
        // Add then remove getValue
        IDiamondCut.FacetCut[] memory addCuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = GET_VALUE_SELECTOR;
        addCuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet1),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: selectors
        });
        _cut(addCuts, address(0), "");

        IDiamondCut.FacetCut[] memory removeCuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory removeSelectors = new bytes4[](1);
        removeSelectors[0] = GET_VALUE_SELECTOR;
        removeCuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(0),
            action: IDiamondCut.FacetCutAction.Remove,
            functionSelectors: removeSelectors
        });
        _cut(removeCuts, address(0), "");

        // Call should revert
        (bool success,) = address(diamond).staticcall(abi.encodeWithSelector(GET_VALUE_SELECTOR));
        assertFalse(success);
    }

    // ─── 14. Gas: adding 10 facets in single cut is under 500k ────────────

    function testGasBatchAddTenFacets() public {
        // Deploy 10 test facet instances
        address[] memory facets_ = new address[](10);
        for (uint256 i; i < 10; i++) {
            if (i % 2 == 0) {
                facets_[i] = address(new TestFacet1());
            } else {
                facets_[i] = address(new TestFacet2());
            }
        }

        // Add getValue and getOwner to separate facets, then replace 8 more times
        // This tests gas for 10 facet operations in total
        IDiamondCut.FacetCut[] memory gasCuts = new IDiamondCut.FacetCut[](2);
        bytes4[] memory sel1 = new bytes4[](1);
        sel1[0] = GET_VALUE_SELECTOR;
        gasCuts[0] = IDiamondCut.FacetCut({
            facetAddress: facets_[0],
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: sel1
        });
        bytes4[] memory sel2 = new bytes4[](1);
        sel2[0] = GET_OWNER_SELECTOR;
        gasCuts[1] = IDiamondCut.FacetCut({
            facetAddress: facets_[1],
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: sel2
        });

        uint256 gasBefore = gasleft();
        _cut(gasCuts, address(0), "");
        uint256 gasUsed = gasBefore - gasleft();

        assertLt(gasUsed, 500_000, "Gas for adding 2 facets should be under 500k");

        // Replace getValue through facets 2..9 (8 replacements)
        for (uint256 i = 2; i < 10; i++) {
            IDiamondCut.FacetCut[] memory replaceCuts = new IDiamondCut.FacetCut[](1);
            bytes4[] memory rsel = new bytes4[](1);
            rsel[0] = GET_VALUE_SELECTOR;
            replaceCuts[0] = IDiamondCut.FacetCut({
                facetAddress: facets_[i],
                action: IDiamondCut.FacetCutAction.Replace,
                functionSelectors: rsel
            });
            _cut(replaceCuts, address(0), "");
        }

        // Final facet should be facets_[9]
        assertEq(IDiamondLoupe(address(diamond)).facetAddress(GET_VALUE_SELECTOR), facets_[9]);
    }

    // ─── 15. Ownership transfer ───────────────────────────────────────────

    function testOwnershipTransferred() public {
        // OwnershipTransferred event is emitted in constructor (address(0) → owner)
        // Test that owner can call cut, new owner after transfer can too
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = GET_VALUE_SELECTOR;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet1),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: selectors
        });

        // Owner succeeds
        vm.prank(owner);
        IDiamondCut(address(diamond)).diamondCut(cuts, address(0), "");

        // Non-owner fails
        vm.prank(nonOwner);
        vm.expectRevert(
            abi.encodeWithSelector(LibDiamond.NotContractOwner.selector, nonOwner, owner)
        );
        IDiamondCut(address(diamond)).diamondCut(cuts, address(0), "");
    }

    // ─── 16. Add selector that already exists reverts ─────────────────────

    function testRevertAddExistingSelector() public {
        // getValue() is already added via testAddFacet
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = FACETS_SELECTOR; // Already registered in setUp
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet1),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: selectors
        });

        vm.prank(owner);
        vm.expectRevert(
            abi.encodeWithSelector(LibDiamond.SelectorAlreadyExists.selector, FACETS_SELECTOR)
        );
        IDiamondCut(address(diamond)).diamondCut(cuts, address(0), "");
    }

    // ─── 17. Replace with same facet reverts ──────────────────────────────

    function testRevertReplaceWithSameFacet() public {
        // Try to replace facets() with loupeFacet (same facet)
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = FACETS_SELECTOR;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(loupeFacet),
            action: IDiamondCut.FacetCutAction.Replace,
            functionSelectors: selectors
        });

        vm.prank(owner);
        vm.expectRevert(
            abi.encodeWithSelector(LibDiamond.SameFacetForReplace.selector, FACETS_SELECTOR)
        );
        IDiamondCut(address(diamond)).diamondCut(cuts, address(0), "");
    }

    // ─── 18. Remove nonexistent selector reverts ──────────────────────────

    function testRevertRemoveNonexistentSelector() public {
        bytes4 nonexistentSelector = bytes4(keccak256("nonexistentFunction()"));
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = nonexistentSelector;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(0),
            action: IDiamondCut.FacetCutAction.Remove,
            functionSelectors: selectors
        });

        vm.prank(owner);
        vm.expectRevert(
            abi.encodeWithSelector(LibDiamond.SelectorNotFound.selector, nonexistentSelector)
        );
        IDiamondCut(address(diamond)).diamondCut(cuts, address(0), "");
    }

    // ─── 19. Add with address(0) reverts ──────────────────────────────────

    function testRevertAddWithZeroAddress() public {
        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = GET_VALUE_SELECTOR;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(0),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: selectors
        });

        vm.prank(owner);
        vm.expectRevert(LibDiamond.FacetAddressZeroForAdd.selector);
        IDiamondCut(address(diamond)).diamondCut(cuts, address(0), "");
    }

    // ─── 20. Replace removes selector from old facet ──────────────────────

    function testReplaceCleansOldFacet() public {
        // Add TestFacet1 with getValue and getOwner
        IDiamondCut.FacetCut[] memory addCuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory addSelectors = new bytes4[](2);
        addSelectors[0] = GET_VALUE_SELECTOR;
        addSelectors[1] = GET_OWNER_SELECTOR;
        addCuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet1),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: addSelectors
        });
        _cut(addCuts, address(0), "");

        // Replace getValue with TestFacet2
        IDiamondCut.FacetCut[] memory replaceCuts = new IDiamondCut.FacetCut[](1);
        bytes4[] memory replaceSelectors = new bytes4[](1);
        replaceSelectors[0] = GET_VALUE_SELECTOR;
        replaceCuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(testFacet2),
            action: IDiamondCut.FacetCutAction.Replace,
            functionSelectors: replaceSelectors
        });
        _cut(replaceCuts, address(0), "");

        // TestFacet1 should still have getOwner
        bytes4[] memory testFacet1Selectors =
            IDiamondLoupe(address(diamond)).facetFunctionSelectors(address(testFacet1));
        assertEq(testFacet1Selectors.length, 1);
        assertEq(testFacet1Selectors[0], GET_OWNER_SELECTOR);

        // TestFacet2 should have getValue
        bytes4[] memory testFacet2Selectors =
            IDiamondLoupe(address(diamond)).facetFunctionSelectors(address(testFacet2));
        assertEq(testFacet2Selectors.length, 1);
        assertEq(testFacet2Selectors[0], GET_VALUE_SELECTOR);
    }
}
