//! EIP-2535 Diamond proxy pattern bindings.
//!
//! Diamond proxy routes function calls to appropriate facets via delegatecall.
//! IDiamondCut defines adding/replacing/removing selectors.
//! IDiamondLoupe provides introspection for registered facets.

use alloy::sol;

sol! {
    // ═══════════════════════════════════════════════════════════════════════
    //  IDiamondCut
    // ═══════════════════════════════════════════════════════════════════════

    #[derive(Debug, PartialEq, Eq)]
    enum FacetCutAction {
        Add,
        Replace,
        Remove
    }

    #[derive(Debug, PartialEq)]
    struct FacetCut {
        address facetAddress;
        FacetCutAction action;
        bytes4[] functionSelectors;
    }

    #[derive(Debug)]
    event DiamondCut(FacetCut[] _diamondCut, address _init, bytes _calldata);

    function diamondCut(FacetCut[] calldata _diamondCut, address _init, bytes calldata _calldata) external;

    // ═══════════════════════════════════════════════════════════════════════
    //  IDiamondLoupe
    // ═══════════════════════════════════════════════════════════════════════

    #[derive(Debug, PartialEq)]
    struct Facet {
        address facetAddress;
        bytes4[] functionSelectors;
    }

    function facets() external view returns (Facet[] memory);
    function facetFunctionSelectors(address _facet) external view returns (bytes4[] memory);
    function facetAddresses() external view returns (address[] memory);
    function facetAddress(bytes4 _functionSelector) external view returns (address);

    // ═══════════════════════════════════════════════════════════════════════
    //  LibDiamond errors
    // ═══════════════════════════════════════════════════════════════════════

    error NotContractOwner(address caller, address owner);
    error NoSelectorsProvided();
    error FacetAddressZeroForAdd();
    error FacetAddressNotZeroForRemove();
    error SelectorAlreadyExists(bytes4 selector);
    error SelectorNotFound(bytes4 selector);
    error SameFacetForReplace(bytes4 selector);
    error InitReverted();

    // ═══════════════════════════════════════════════════════════════════════
    //  LibDiamond events
    // ═══════════════════════════════════════════════════════════════════════

    #[derive(Debug)]
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
}
