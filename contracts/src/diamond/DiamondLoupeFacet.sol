// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "./IDiamondLoupe.sol";
import "./LibDiamond.sol";

/// @title DiamondLoupeFacet — EIP-2535 Diamond Loupe facet
/// @notice Provides introspection functions for querying registered facets and selectors.
contract DiamondLoupeFacet is IDiamondLoupe {
    /// @notice Returns all registered facets and their function selectors
    function facets() external view override returns (Facet[] memory) {
        LibDiamond.DiamondStorage storage ds = LibDiamond.diamondStorage();
        uint256 numFacets = ds.facetAddresses.length;
        Facet[] memory facets_ = new Facet[](numFacets);

        for (uint256 i; i < numFacets; i++) {
            address facetAddr = ds.facetAddresses[i];
            facets_[i].facetAddress = facetAddr;
            facets_[i].functionSelectors = ds.facetToSelectorList[facetAddr].selectors;
        }
        return facets_;
    }

    /// @notice Returns all function selectors for a given facet address
    function facetFunctionSelectors(address _facet)
        external
        view
        override
        returns (bytes4[] memory)
    {
        return LibDiamond.diamondStorage().facetToSelectorList[_facet].selectors;
    }

    /// @notice Returns all registered facet addresses
    function facetAddresses() external view override returns (address[] memory) {
        return LibDiamond.diamondStorage().facetAddresses;
    }

    /// @notice Returns the facet address that handles a given function selector
    function facetAddress(bytes4 _functionSelector)
        external
        view
        override
        returns (address)
    {
        return LibDiamond.diamondStorage().selectorToFacet[_functionSelector].facetAddress;
    }
}
