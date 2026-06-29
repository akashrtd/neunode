// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

/// @title IDiamondCut — EIP-2535 Diamond Cut interface
/// @notice Defines the standard for adding, replacing, and removing facet function selectors.
interface IDiamondCut {
    enum FacetCutAction {
        Add, // Add new function selectors
        Replace, // Replace existing function selectors
        Remove // Remove function selectors
    }

    struct FacetCut {
        address facetAddress;
        FacetCutAction action;
        bytes4[] functionSelectors;
    }

    /// @notice Add, replace, or remove facet function selectors
    /// @param _diamondCut Array of facet cut actions
    /// @param _init Address of contract to delegatecall for initialization (address(0) = none)
    /// @param _calldata Calldata for initialization function
    function diamondCut(FacetCut[] calldata _diamondCut, address _init, bytes calldata _calldata)
        external;

    event DiamondCut(FacetCut[] _diamondCut, address _init, bytes _calldata);
}
