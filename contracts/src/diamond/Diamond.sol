// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "./IDiamondCut.sol";
import "./IDiamondLoupe.sol";
import "./LibDiamond.sol";

/// @title Diamond — EIP-2535 Diamond proxy contract
/// @notice Routes function calls to appropriate facets via delegatecall.
///         Supports upgradeable facets through the DiamondCut pattern.
contract Diamond {
    // ─── Errors ───────────────────────────────────────────────────────────

    error FunctionDoesNotExist(bytes4 selector);

    // ─── Constructor ──────────────────────────────────────────────────────

    /// @notice Deploy the diamond with initial facet configuration
    /// @param _diamondCut Initial facet cuts to apply
    /// @param _init Address of initializer contract (address(0) for none)
    /// @param _calldata Calldata for initialization
    /// @param _owner Address of the diamond owner
    constructor(
        IDiamondCut.FacetCut[] memory _diamondCut,
        address _init,
        bytes memory _calldata,
        address _owner
    ) {
        if (_owner != address(0)) {
            LibDiamond.setContractOwner(_owner);
        } else {
            LibDiamond.setContractOwner(msg.sender);
        }

        LibDiamond.diamondCut(_diamondCut, _init, _calldata);
    }

    // ─── Fallback ─────────────────────────────────────────────────────────

    /// @notice Route function calls to the appropriate facet via delegatecall
    fallback() external payable {
        LibDiamond.DiamondStorage storage ds;
        bytes32 position = LibDiamond.STORAGE_POSITION;
        assembly {
            ds.slot := position
        }
        address facet = ds.selectorToFacet[msg.sig].facetAddress;
        if (facet == address(0)) revert FunctionDoesNotExist(msg.sig);

        assembly {
            calldatacopy(0, 0, calldatasize())
            let result := delegatecall(gas(), facet, 0, calldatasize(), 0, 0)
            returndatacopy(0, 0, returndatasize())
            switch result
            case 0 {
                revert(0, returndatasize())
            }
            default {
                return(0, returndatasize())
            }
        }
    }

    /// @notice Accept ETH transfers
    receive() external payable {}
}
