// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "./IDiamondCut.sol";
import "./LibDiamond.sol";

/// @title DiamondCutFacet — EIP-2535 Diamond Cut facet
/// @notice Allows the diamond owner to add, replace, or remove facet function selectors.
contract DiamondCutFacet is IDiamondCut {
    /// @notice Add, replace, or remove facet function selectors
    /// @param _diamondCut Array of facet cut actions
    /// @param _init Address of contract to delegatecall for initialization
    /// @param _calldata Calldata for initialization function
    function diamondCut(
        FacetCut[] calldata _diamondCut,
        address _init,
        bytes calldata _calldata
    ) external override {
        LibDiamond.enforceIsContractOwner();
        LibDiamond.diamondCut(_diamondCut, _init, _calldata);
    }
}
