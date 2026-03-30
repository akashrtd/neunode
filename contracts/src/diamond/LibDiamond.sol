// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "./IDiamondCut.sol";

/// @title LibDiamond — Shared storage library for EIP-2535 Diamond pattern
/// @notice Provides diamond storage, ownership, and selector management functions.
library LibDiamond {
    // ─── Storage ──────────────────────────────────────────────────────────

    bytes32 constant STORAGE_POSITION = keccak256("diamond.neunode.storage");

    struct SelectorToFacet {
        address facetAddress;
        uint96 selectorIndex; // Index in facetToSelectorList[facetAddress].selectors
    }

    struct FacetSelectorSet {
        bytes4[] selectors;
        uint256 facetIndex; // Index in ds.facetAddresses
    }

    struct DiamondStorage {
        address contractOwner;
        mapping(bytes4 => SelectorToFacet) selectorToFacet;
        mapping(address => FacetSelectorSet) facetToSelectorList;
        address[] facetAddresses;
    }

    // ─── Events ───────────────────────────────────────────────────────────

    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    // ─── Errors ───────────────────────────────────────────────────────────

    error NotContractOwner(address caller, address owner);
    error NoSelectorsProvided();
    error FacetAddressZeroForAdd();
    error FacetAddressNotZeroForRemove();
    error SelectorAlreadyExists(bytes4 selector);
    error SelectorNotFound(bytes4 selector);
    error SameFacetForReplace(bytes4 selector);
    error InitReverted();

    // ─── Storage Access ───────────────────────────────────────────────────

    function diamondStorage() internal pure returns (DiamondStorage storage ds) {
        bytes32 position = STORAGE_POSITION;
        assembly {
            ds.slot := position
        }
    }

    // ─── Ownership ────────────────────────────────────────────────────────

    function setContractOwner(address _newOwner) internal {
        DiamondStorage storage ds = diamondStorage();
        address previousOwner = ds.contractOwner;
        ds.contractOwner = _newOwner;
        emit OwnershipTransferred(previousOwner, _newOwner);
    }

    function contractOwner() internal view returns (address) {
        return diamondStorage().contractOwner;
    }

    function enforceIsContractOwner() internal view {
        DiamondStorage storage ds = diamondStorage();
        if (msg.sender != ds.contractOwner) {
            revert NotContractOwner(msg.sender, ds.contractOwner);
        }
    }

    // ─── Diamond Cut Internal ─────────────────────────────────────────────

    function diamondCut(
        IDiamondCut.FacetCut[] memory _diamondCut,
        address _init,
        bytes memory _calldata
    ) internal {
        for (uint256 i; i < _diamondCut.length; i++) {
            IDiamondCut.FacetCutAction action = _diamondCut[i].action;
            if (action == IDiamondCut.FacetCutAction.Add) {
                _addSelectors(_diamondCut[i].facetAddress, _diamondCut[i].functionSelectors);
            } else if (action == IDiamondCut.FacetCutAction.Replace) {
                _replaceSelectors(_diamondCut[i].facetAddress, _diamondCut[i].functionSelectors);
            } else if (action == IDiamondCut.FacetCutAction.Remove) {
                _removeSelectors(_diamondCut[i].facetAddress, _diamondCut[i].functionSelectors);
            }
        }
        emit IDiamondCut.DiamondCut(_diamondCut, _init, _calldata);
        _initializeDiamondCut(_init, _calldata);
    }

    // ─── Add Selectors ────────────────────────────────────────────────────

    function _addSelectors(address _facetAddress, bytes4[] memory _selectors) internal {
        if (_selectors.length == 0) revert NoSelectorsProvided();
        if (_facetAddress == address(0)) revert FacetAddressZeroForAdd();

        DiamondStorage storage ds = diamondStorage();
        // Enforce that this is a new facet or existing one
        uint96 selectorCount = uint96(ds.facetToSelectorList[_facetAddress].selectors.length);

        // Register facet address if new
        if (selectorCount == 0) {
            ds.facetToSelectorList[_facetAddress].facetIndex = ds.facetAddresses.length;
            ds.facetAddresses.push(_facetAddress);
        }

        for (uint256 i; i < _selectors.length; i++) {
            bytes4 selector = _selectors[i];
            if (ds.selectorToFacet[selector].facetAddress != address(0)) {
                revert SelectorAlreadyExists(selector);
            }
            ds.selectorToFacet[selector] = SelectorToFacet(_facetAddress, selectorCount);
            ds.facetToSelectorList[_facetAddress].selectors.push(selector);
            selectorCount++;
        }
    }

    // ─── Replace Selectors ────────────────────────────────────────────────

    function _replaceSelectors(address _facetAddress, bytes4[] memory _selectors) internal {
        if (_selectors.length == 0) revert NoSelectorsProvided();

        DiamondStorage storage ds = diamondStorage();

        for (uint256 i; i < _selectors.length; i++) {
            bytes4 selector = _selectors[i];
            address oldFacet = ds.selectorToFacet[selector].facetAddress;
            if (oldFacet == address(0)) revert SelectorNotFound(selector);
            if (oldFacet == _facetAddress) revert SameFacetForReplace(selector);

            // Remove from old facet's selector list (swap-and-pop)
            uint96 oldSelectorIndex = ds.selectorToFacet[selector].selectorIndex;
            uint256 oldSelectorCount = ds.facetToSelectorList[oldFacet].selectors.length;
            if (oldSelectorIndex < oldSelectorCount - 1) {
                bytes4 lastSelector =
                    ds.facetToSelectorList[oldFacet].selectors[oldSelectorCount - 1];
                ds.facetToSelectorList[oldFacet].selectors[oldSelectorIndex] = lastSelector;
                ds.selectorToFacet[lastSelector].selectorIndex = oldSelectorIndex;
            }
            ds.facetToSelectorList[oldFacet].selectors.pop();

            // If old facet has no selectors left, remove it from facetAddresses
            if (ds.facetToSelectorList[oldFacet].selectors.length == 0) {
                _removeFacetAddress(oldFacet);
            }

            // Add to new facet's selector list
            uint96 newSelectorCount = uint96(ds.facetToSelectorList[_facetAddress].selectors.length);
            if (newSelectorCount == 0) {
                // New facet not yet registered
                ds.facetToSelectorList[_facetAddress].facetIndex = ds.facetAddresses.length;
                ds.facetAddresses.push(_facetAddress);
            }
            ds.selectorToFacet[selector] = SelectorToFacet(_facetAddress, newSelectorCount);
            ds.facetToSelectorList[_facetAddress].selectors.push(selector);
        }
    }

    // ─── Remove Selectors ─────────────────────────────────────────────────

    function _removeSelectors(address _facetAddress, bytes4[] memory _selectors) internal {
        if (_selectors.length == 0) revert NoSelectorsProvided();
        if (_facetAddress != address(0)) revert FacetAddressNotZeroForRemove();

        DiamondStorage storage ds = diamondStorage();

        for (uint256 i; i < _selectors.length; i++) {
            bytes4 selector = _selectors[i];
            address oldFacet = ds.selectorToFacet[selector].facetAddress;
            if (oldFacet == address(0)) revert SelectorNotFound(selector);

            // Remove from facet's selector list (swap-and-pop)
            uint96 selectorIndex = ds.selectorToFacet[selector].selectorIndex;
            uint256 selectorCount = ds.facetToSelectorList[oldFacet].selectors.length;
            if (selectorIndex < selectorCount - 1) {
                bytes4 lastSelector = ds.facetToSelectorList[oldFacet].selectors[selectorCount - 1];
                ds.facetToSelectorList[oldFacet].selectors[selectorIndex] = lastSelector;
                ds.selectorToFacet[lastSelector].selectorIndex = selectorIndex;
            }
            ds.facetToSelectorList[oldFacet].selectors.pop();
            delete ds.selectorToFacet[selector];

            // If facet has no selectors left, remove it
            if (ds.facetToSelectorList[oldFacet].selectors.length == 0) {
                _removeFacetAddress(oldFacet);
            }
        }
    }

    // ─── Remove Facet Address ─────────────────────────────────────────────

    function _removeFacetAddress(address _facetAddress) internal {
        DiamondStorage storage ds = diamondStorage();
        uint256 facetIndex = ds.facetToSelectorList[_facetAddress].facetIndex;
        uint256 lastFacetIndex = ds.facetAddresses.length - 1;

        if (facetIndex != lastFacetIndex) {
            address lastFacetAddress = ds.facetAddresses[lastFacetIndex];
            ds.facetAddresses[facetIndex] = lastFacetAddress;
            ds.facetToSelectorList[lastFacetAddress].facetIndex = facetIndex;
        }
        ds.facetAddresses.pop();
        delete ds.facetToSelectorList[_facetAddress];
    }

    // ─── Initialization ───────────────────────────────────────────────────

    function _initializeDiamondCut(address _init, bytes memory _calldata) internal {
        if (_init == address(0)) {
            return;
        }
        (bool success, bytes memory error) = _init.delegatecall(_calldata);
        if (!success) {
            if (error.length > 0) {
                assembly {
                    revert(add(32, error), mload(error))
                }
            } else {
                revert InitReverted();
            }
        }
    }
}
