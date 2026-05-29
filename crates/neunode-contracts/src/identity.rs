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
