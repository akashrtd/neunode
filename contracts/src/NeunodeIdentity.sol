// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title NeunodeIdentity — DID Registry for AI agents
/// @notice Maps did:neunode:<hash> → controller address with key rotation support.
///         Dual-key model: Ed25519 for P2P signing, secp256k1 (Ethereum) for on-chain ops.
contract NeunodeIdentity {
    // ─── Types ────────────────────────────────────────────────────────────

    struct DidDocument {
        address controller; // Ethereum address that controls this DID
        bytes32 ed25519PublicKeyHash; // keccak256 hash of Ed25519 public key (32 bytes)
        uint256 created;
        uint256 updated;
        bool active;
    }

    // ─── Storage ──────────────────────────────────────────────────────────

    mapping(bytes32 => DidDocument) public documents; // didHash → document
    mapping(address => bytes32) public addressToDid; // addr → didHash

    // ─── Events ───────────────────────────────────────────────────────────

    event DidCreated(bytes32 indexed didHash, address indexed controller, uint256 timestamp);
    event DidUpdated(bytes32 indexed didHash, address indexed newController, uint256 timestamp);
    event DidDeactivated(bytes32 indexed didHash, uint256 timestamp);

    // ─── Errors ───────────────────────────────────────────────────────────

    error DidAlreadyExists(bytes32 didHash);
    error DidNotFound(bytes32 didHash);
    error DidNotActive(bytes32 didHash);
    error NotController(bytes32 didHash, address caller);
    error AddressAlreadyHasDid(address addr);
    error InvalidPublicKeyHash();

    // ─── Functions ────────────────────────────────────────────────────────

    /// @notice Create a new DID for the caller.
    /// @param ed25519PubKeyHash keccak256 hash of the Ed25519 public key
    /// @return didHash The DID identifier (keccak256 of controller + pubKeyHash + salt)
    function createDid(bytes32 ed25519PubKeyHash) external returns (bytes32) {
        if (ed25519PubKeyHash == bytes32(0)) revert InvalidPublicKeyHash();
        if (addressToDid[msg.sender] != bytes32(0)) revert AddressAlreadyHasDid(msg.sender);

        // Deterministic DID hash from controller + pubKeyHash + address (as salt)
        bytes32 didHash =
            keccak256(abi.encodePacked(msg.sender, ed25519PubKeyHash, block.timestamp));

        if (documents[didHash].created != 0) revert DidAlreadyExists(didHash);

        documents[didHash] = DidDocument({
            controller: msg.sender,
            ed25519PublicKeyHash: ed25519PubKeyHash,
            created: block.timestamp,
            updated: block.timestamp,
            active: true
        });
        addressToDid[msg.sender] = didHash;

        emit DidCreated(didHash, msg.sender, block.timestamp);
        return didHash;
    }

    /// @notice Transfer DID control to a new address (key rotation)
    function updateController(bytes32 didHash, address newController) external {
        DidDocument storage doc = documents[didHash];
        if (doc.created == 0) revert DidNotFound(didHash);
        if (!doc.active) revert DidNotActive(didHash);
        if (doc.controller != msg.sender) revert NotController(didHash, msg.sender);
        if (newController == address(0)) revert InvalidPublicKeyHash();
        if (addressToDid[newController] != bytes32(0)) revert AddressAlreadyHasDid(newController);

        // Clear old mapping, set new
        addressToDid[doc.controller] = bytes32(0);
        addressToDid[newController] = didHash;

        doc.controller = newController;
        doc.updated = block.timestamp;

        emit DidUpdated(didHash, newController, block.timestamp);
    }

    /// @notice Permanently deactivate a DID (irreversible)
    function deactivateDid(bytes32 didHash) external {
        DidDocument storage doc = documents[didHash];
        if (doc.created == 0) revert DidNotFound(didHash);
        if (!doc.active) revert DidNotActive(didHash);
        if (doc.controller != msg.sender) revert NotController(didHash, msg.sender);

        doc.active = false;
        doc.updated = block.timestamp;
        addressToDid[msg.sender] = bytes32(0);

        emit DidDeactivated(didHash, block.timestamp);
    }

    /// @notice Get the controller address for a DID
    function getController(bytes32 didHash) external view returns (address) {
        if (documents[didHash].created == 0) revert DidNotFound(didHash);
        return documents[didHash].controller;
    }

    /// @notice Check if a DID is active
    function isActive(bytes32 didHash) external view returns (bool) {
        if (documents[didHash].created == 0) return false;
        return documents[didHash].active;
    }

    /// @notice Verify an ECDSA signature from the DID's controller (secp256k1)
    /// @param didHash The DID to verify against
    /// @param messageHash The keccak256 hash of the message
    /// @param signature The 65-byte ECDSA signature
    function verifySignature(bytes32 didHash, bytes32 messageHash, bytes calldata signature)
        external
        view
        returns (bool)
    {
        DidDocument storage doc = documents[didHash];
        if (doc.created == 0 || !doc.active) return false;

        bytes32 ethSignedHash =
            keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash));

        // Recover signer from signature
        if (signature.length != 65) return false;
        bytes32 r;
        bytes32 s;
        uint8 v;
        assembly {
            r := calldataload(signature.offset)
            s := calldataload(add(signature.offset, 32))
            v := byte(0, calldataload(add(signature.offset, 64)))
        }

        address signer = ecrecover(ethSignedHash, v, r, s);
        return signer != address(0) && signer == doc.controller;
    }

    /// @notice Get the DID hash for an address
    function getDidForAddress(address addr) external view returns (bytes32) {
        return addressToDid[addr];
    }

    /// @notice Get full DID document
    function getDocument(bytes32 didHash) external view returns (DidDocument memory) {
        if (documents[didHash].created == 0) revert DidNotFound(didHash);
        return documents[didHash];
    }
}
