//! NeunodeIdentity contract bindings.
//!
//! DID Registry for AI agents. Maps `did:neunode:<hash>` to controller address
//! with key rotation support. Dual-key model: Ed25519 for P2P signing,
//! secp256k1 (Ethereum) for on-chain ops.

use alloy::sol;

sol! {
    // ─── Structs ──────────────────────────────────────────────────────────

    #[derive(Debug, PartialEq)]
    struct DidDocument {
        address controller;
        bytes32 ed25519PublicKeyHash;
        uint256 created;
        uint256 updated;
        bool active;
    }

    // ─── Events ───────────────────────────────────────────────────────────

    #[derive(Debug)]
    event DidCreated(bytes32 indexed didHash, address indexed controller, uint256 timestamp);

    #[derive(Debug)]
    event DidUpdated(bytes32 indexed didHash, address indexed newController, uint256 timestamp);

    #[derive(Debug)]
    event DidKeyRotated(bytes32 indexed didHash, bytes32 newPubKeyHash, uint256 timestamp);

    #[derive(Debug)]
    event DidDeactivated(bytes32 indexed didHash, uint256 timestamp);

    // ─── Errors ───────────────────────────────────────────────────────────

    error DidAlreadyExists(bytes32 didHash);
    error DidNotFound(bytes32 didHash);
    error DidNotActive(bytes32 didHash);
    error NotController(bytes32 didHash, address caller);
    error AddressAlreadyHasDid(address addr);
    error InvalidPublicKeyHash();

    // ─── Functions ────────────────────────────────────────────────────────

    function createDid(bytes32 ed25519PubKeyHash) external returns (bytes32);
    function updateController(bytes32 didHash, address newController) external;
    function updateEd25519Key(bytes32 didHash, bytes32 newPubKeyHash) external;
    function deactivateDid(bytes32 didHash) external;
    function getController(bytes32 didHash) external view returns (address);
    function isActive(bytes32 didHash) external view returns (bool);
    function verifySignature(bytes32 didHash, bytes32 messageHash, bytes calldata signature) external view returns (bool);
    function getDidForAddress(address addr) external view returns (bytes32);
    function getDocument(bytes32 didHash) external view returns (DidDocument memory);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, FixedBytes, U256};
    use alloy::primitives::{address, fixed_bytes};
    use alloy::sol_types::{SolError, SolEvent};

    // ─── DidDocument struct tests ───────────────────────────────────────────

    #[test]
    fn did_document_construction() {
        let doc = DidDocument {
            controller: address!("0000000000000000000000000000000000000001"),
            ed25519PublicKeyHash: fixed_bytes!(
                "0000000000000000000000000000000000000000000000000000000000000001"
            ),
            created: U256::from(1000),
            updated: U256::from(2000),
            active: true,
        };
        assert!(doc.active);
        assert_eq!(doc.controller, address!("0000000000000000000000000000000000000001"));
        assert_eq!(
            doc.ed25519PublicKeyHash,
            fixed_bytes!("0000000000000000000000000000000000000000000000000000000000000001")
        );
        assert_eq!(doc.created, U256::from(1000));
        assert_eq!(doc.updated, U256::from(2000));
    }

    #[test]
    fn did_document_default_fields() {
        let doc = DidDocument {
            controller: Address::ZERO,
            ed25519PublicKeyHash: FixedBytes::<32>::ZERO,
            created: U256::ZERO,
            updated: U256::ZERO,
            active: false,
        };
        assert!(!doc.active);
        assert_eq!(doc.controller, Address::ZERO);
        assert_eq!(doc.ed25519PublicKeyHash, FixedBytes::<32>::ZERO);
    }

    #[test]
    fn did_document_debug_format() {
        let doc = DidDocument {
            controller: Address::ZERO,
            ed25519PublicKeyHash: FixedBytes::<32>::ZERO,
            created: U256::ZERO,
            updated: U256::ZERO,
            active: false,
        };
        let debug_str = format!("{doc:?}");
        assert!(debug_str.contains("DidDocument"));
    }

    // ─── Event signature tests ──────────────────────────────────────────────

    #[test]
    fn event_signatures_non_empty() {
        assert!(!DidCreated::SIGNATURE.is_empty());
        assert!(!DidUpdated::SIGNATURE.is_empty());
        assert!(!DidKeyRotated::SIGNATURE.is_empty());
        assert!(!DidDeactivated::SIGNATURE.is_empty());
    }

    #[test]
    fn event_signatures_expected_format() {
        // Solidity event signatures follow the pattern: EventName(type1,type2,...)
        assert!(DidCreated::SIGNATURE.starts_with("DidCreated("));
        assert!(DidUpdated::SIGNATURE.starts_with("DidUpdated("));
        assert!(DidKeyRotated::SIGNATURE.starts_with("DidKeyRotated("));
        assert!(DidDeactivated::SIGNATURE.starts_with("DidDeactivated("));
    }

    #[test]
    fn event_selectors_are_32_bytes() {
        assert_eq!(DidCreated::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(DidUpdated::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(DidKeyRotated::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(DidDeactivated::SIGNATURE_HASH.as_slice().len(), 32);
    }

    #[test]
    fn event_selectors_unique() {
        let selectors = [
            DidCreated::SIGNATURE_HASH,
            DidUpdated::SIGNATURE_HASH,
            DidKeyRotated::SIGNATURE_HASH,
            DidDeactivated::SIGNATURE_HASH,
        ];
        for i in 0..selectors.len() {
            for j in (i + 1)..selectors.len() {
                assert_ne!(selectors[i], selectors[j], "Event selectors must be unique");
            }
        }
    }

    // ─── Error construction tests ───────────────────────────────────────────

    #[test]
    fn error_types_constructible() {
        let _ = DidAlreadyExists { didHash: FixedBytes::<32>::ZERO };
        let _ = DidNotFound { didHash: FixedBytes::<32>::ZERO };
        let _ = DidNotActive { didHash: FixedBytes::<32>::ZERO };
        let _ = NotController { didHash: FixedBytes::<32>::ZERO, caller: Address::ZERO };
        let _ = AddressAlreadyHasDid { addr: Address::ZERO };
        let _ = InvalidPublicKeyHash {};
    }

    #[test]
    fn error_selectors_non_empty() {
        assert!(!DidAlreadyExists::SELECTOR.is_empty());
        assert!(!DidNotFound::SELECTOR.is_empty());
        assert!(!DidNotActive::SELECTOR.is_empty());
        assert!(!NotController::SELECTOR.is_empty());
        assert!(!AddressAlreadyHasDid::SELECTOR.is_empty());
        assert!(!InvalidPublicKeyHash::SELECTOR.is_empty());
    }

    #[test]
    fn error_selectors_are_4_bytes() {
        assert_eq!(DidAlreadyExists::SELECTOR.len(), 4);
        assert_eq!(DidNotFound::SELECTOR.len(), 4);
        assert_eq!(DidNotActive::SELECTOR.len(), 4);
        assert_eq!(NotController::SELECTOR.len(), 4);
        assert_eq!(AddressAlreadyHasDid::SELECTOR.len(), 4);
        assert_eq!(InvalidPublicKeyHash::SELECTOR.len(), 4);
    }
}
