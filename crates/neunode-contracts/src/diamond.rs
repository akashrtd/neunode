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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Address;
    use alloy::primitives::{address, fixed_bytes};
    use alloy::sol_types::{SolError, SolEvent};

    // ─── FacetCutAction enum tests ──────────────────────────────────────────

    #[test]
    fn facet_cut_action_all_variants() {
        let _add = FacetCutAction::Add;
        let _replace = FacetCutAction::Replace;
        let _remove = FacetCutAction::Remove;
    }

    #[test]
    fn facet_cut_action_equality() {
        assert_eq!(FacetCutAction::Add, FacetCutAction::Add);
        assert_ne!(FacetCutAction::Add, FacetCutAction::Replace);
        assert_ne!(FacetCutAction::Replace, FacetCutAction::Remove);
        assert_ne!(FacetCutAction::Add, FacetCutAction::Remove);
    }

    #[test]
    fn facet_cut_action_values() {
        // EIP-2535: Add=0, Replace=1, Remove=2
        assert_eq!(FacetCutAction::Add as u8, 0);
        assert_eq!(FacetCutAction::Replace as u8, 1);
        assert_eq!(FacetCutAction::Remove as u8, 2);
    }

    // ─── FacetCut struct tests ──────────────────────────────────────────────

    #[test]
    fn facet_cut_construction() {
        let cut = FacetCut {
            facetAddress: address!("0000000000000000000000000000000000000001"),
            action: FacetCutAction::Add,
            functionSelectors: vec![fixed_bytes!("12345678"), fixed_bytes!("abcdef01")],
        };
        assert_eq!(cut.action, FacetCutAction::Add);
        assert_eq!(cut.functionSelectors.len(), 2);
        assert_eq!(
            cut.facetAddress,
            address!("0000000000000000000000000000000000000001")
        );
    }

    #[test]
    fn facet_cut_with_remove_action() {
        let cut = FacetCut {
            facetAddress: Address::ZERO, // Must be zero for Remove
            action: FacetCutAction::Remove,
            functionSelectors: vec![fixed_bytes!("12345678")],
        };
        assert_eq!(cut.action, FacetCutAction::Remove);
        assert_eq!(cut.facetAddress, Address::ZERO);
    }

    #[test]
    fn facet_cut_empty_selectors() {
        let cut = FacetCut {
            facetAddress: address!("0000000000000000000000000000000000000001"),
            action: FacetCutAction::Add,
            functionSelectors: vec![],
        };
        assert!(cut.functionSelectors.is_empty());
    }

    #[test]
    fn facet_cut_debug_format() {
        let cut = FacetCut {
            facetAddress: Address::ZERO,
            action: FacetCutAction::Add,
            functionSelectors: vec![],
        };
        let debug_str = format!("{cut:?}");
        assert!(debug_str.contains("FacetCut"));
    }

    // ─── Facet struct tests ─────────────────────────────────────────────────

    #[test]
    fn facet_construction() {
        let facet = Facet {
            facetAddress: address!("0000000000000000000000000000000000000001"),
            functionSelectors: vec![
                fixed_bytes!("12345678"),
                fixed_bytes!("abcdef01"),
                fixed_bytes!("deadbeef"),
            ],
        };
        assert_eq!(
            facet.facetAddress,
            address!("0000000000000000000000000000000000000001")
        );
        assert_eq!(facet.functionSelectors.len(), 3);
    }

    #[test]
    fn facet_empty_selectors() {
        let facet = Facet {
            facetAddress: Address::ZERO,
            functionSelectors: vec![],
        };
        assert!(facet.functionSelectors.is_empty());
    }

    // ─── Event signature tests ──────────────────────────────────────────────

    #[test]
    fn event_signatures_non_empty() {
        assert!(!DiamondCut::SIGNATURE.is_empty());
        assert!(!OwnershipTransferred::SIGNATURE.is_empty());
    }

    #[test]
    fn event_signatures_expected_format() {
        assert!(DiamondCut::SIGNATURE.starts_with("DiamondCut("));
        assert!(OwnershipTransferred::SIGNATURE.starts_with("OwnershipTransferred("));
    }

    #[test]
    fn event_selectors_are_32_bytes() {
        assert_eq!(DiamondCut::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(OwnershipTransferred::SIGNATURE_HASH.as_slice().len(), 32);
    }

    #[test]
    fn event_selectors_unique() {
        assert_ne!(
            DiamondCut::SIGNATURE_HASH,
            OwnershipTransferred::SIGNATURE_HASH,
            "Diamond event selectors must be unique"
        );
    }

    // ─── Error construction tests ───────────────────────────────────────────

    #[test]
    fn error_types_constructible() {
        let caller = address!("0000000000000000000000000000000000000001");
        let owner = address!("0000000000000000000000000000000000000002");
        let selector = fixed_bytes!("12345678");

        let _ = NotContractOwner { caller, owner };
        let _ = NoSelectorsProvided {};
        let _ = FacetAddressZeroForAdd {};
        let _ = FacetAddressNotZeroForRemove {};
        let _ = SelectorAlreadyExists { selector };
        let _ = SelectorNotFound { selector };
        let _ = SameFacetForReplace { selector };
        let _ = InitReverted {};
    }

    #[test]
    fn error_selectors_are_4_bytes() {
        assert_eq!(NotContractOwner::SELECTOR.len(), 4);
        assert_eq!(NoSelectorsProvided::SELECTOR.len(), 4);
        assert_eq!(FacetAddressZeroForAdd::SELECTOR.len(), 4);
        assert_eq!(FacetAddressNotZeroForRemove::SELECTOR.len(), 4);
        assert_eq!(SelectorAlreadyExists::SELECTOR.len(), 4);
        assert_eq!(SelectorNotFound::SELECTOR.len(), 4);
        assert_eq!(SameFacetForReplace::SELECTOR.len(), 4);
        assert_eq!(InitReverted::SELECTOR.len(), 4);
    }

    #[test]
    fn error_selectors_unique() {
        let selectors = [
            NotContractOwner::SELECTOR,
            NoSelectorsProvided::SELECTOR,
            FacetAddressZeroForAdd::SELECTOR,
            FacetAddressNotZeroForRemove::SELECTOR,
            SelectorAlreadyExists::SELECTOR,
            SelectorNotFound::SELECTOR,
            SameFacetForReplace::SELECTOR,
            InitReverted::SELECTOR,
        ];
        for i in 0..selectors.len() {
            for j in (i + 1)..selectors.len() {
                assert_ne!(
                    selectors[i], selectors[j],
                    "Diamond error selectors must be unique"
                );
            }
        }
    }

    // ─── Diamond cut parameter tests ────────────────────────────────────────

    #[test]
    fn diamond_cut_with_multiple_facets() {
        let cuts = vec![
            FacetCut {
                facetAddress: address!("0000000000000000000000000000000000000001"),
                action: FacetCutAction::Add,
                functionSelectors: vec![fixed_bytes!("11111111"), fixed_bytes!("22222222")],
            },
            FacetCut {
                facetAddress: address!("0000000000000000000000000000000000000002"),
                action: FacetCutAction::Replace,
                functionSelectors: vec![fixed_bytes!("33333333")],
            },
            FacetCut {
                facetAddress: Address::ZERO,
                action: FacetCutAction::Remove,
                functionSelectors: vec![fixed_bytes!("44444444")],
            },
        ];
        assert_eq!(cuts.len(), 3);
        assert_eq!(cuts[0].action, FacetCutAction::Add);
        assert_eq!(cuts[1].action, FacetCutAction::Replace);
        assert_eq!(cuts[2].action, FacetCutAction::Remove);
    }

    #[test]
    fn function_selector_is_4_bytes() {
        let selector = fixed_bytes!("12345678");
        assert_eq!(selector.as_slice().len(), 4);
    }
}
