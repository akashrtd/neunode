// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

/// @title IDiamondLoupe — EIP-2535 Diamond Loupe interface
/// @notice Provides introspection functions to query registered facets and selectors.
interface IDiamondLoupe {
    struct Facet {
        address facetAddress;
        bytes4[] functionSelectors;
    }

    /// @notice Returns all registered facets and their function selectors
    function facets() external view returns (Facet[] memory);

    /// @notice Returns all function selectors for a given facet address
    function facetFunctionSelectors(address _facet) external view returns (bytes4[] memory);

    /// @notice Returns all registered facet addresses
    function facetAddresses() external view returns (address[] memory);

    /// @notice Returns the facet address that handles a given function selector
    function facetAddress(bytes4 _functionSelector) external view returns (address);
}
