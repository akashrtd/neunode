// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {MessageHashUtils} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

/// @notice Minimal stake-read interface (interface segregation).
///         NeunodeToken satisfies this via its public stakedBalanceOf.
interface IStakeSource {
    function stakedBalanceOf(address account) external view returns (uint256);
}

/// @title NeunodeIdentity — DID Registry for AI agents
/// @notice Maps did:neunode:<hash> → controller address with key rotation support.
///         Dual-key model: Ed25519 for P2P signing, secp256k1 (Ethereum) for on-chain ops.
contract NeunodeIdentity {
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;

    // ─── Access Control ───────────────────────────────────────────────────

    address public owner;

    constructor() {
        owner = msg.sender;
    }

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner(msg.sender);
        _;
    }

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

    // Sybil resistance: network participation requires a slashable stake.
    IStakeSource public stakeSource;
    uint256 public minRegistrationStake;
    mapping(bytes32 => bool) private _registered; // didHash → reputation/validator eligible

    // ─── Events ───────────────────────────────────────────────────────────

    event DidCreated(bytes32 indexed didHash, address indexed controller, uint256 timestamp);
    event DidUpdated(bytes32 indexed didHash, address indexed newController, uint256 timestamp);
    event DidKeyRotated(bytes32 indexed didHash, bytes32 newPubKeyHash, uint256 timestamp);
    event DidDeactivated(bytes32 indexed didHash, uint256 timestamp);
    event NetworkRegistered(bytes32 indexed didHash, address indexed controller, uint256 stake);
    event NetworkDeregistered(bytes32 indexed didHash, address indexed controller);
    event MinRegistrationStakeUpdated(uint256 oldMin, uint256 newMin);
    event StakeSourceUpdated(address stakeSource);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    // ─── Errors ───────────────────────────────────────────────────────────

    error DidAlreadyExists(bytes32 didHash);
    error DidNotFound(bytes32 didHash);
    error DidNotActive(bytes32 didHash);
    error NotController(bytes32 didHash, address caller);
    error AddressAlreadyHasDid(address addr);
    error InvalidPublicKeyHash();
    error NotOwner(address caller);
    error NotRegistered(bytes32 didHash);
    error InsufficientRegistrationStake(address controller, uint256 staked, uint256 required);
    error InvalidOwner();

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

    /// @notice Rotate the Ed25519 public key (P2P session key)
    function updateEd25519Key(bytes32 didHash, bytes32 newPubKeyHash) external {
        DidDocument storage doc = documents[didHash];
        if (doc.created == 0) revert DidNotFound(didHash);
        if (!doc.active) revert DidNotActive(didHash);
        if (doc.controller != msg.sender) revert NotController(didHash, msg.sender);
        if (newPubKeyHash == bytes32(0)) revert InvalidPublicKeyHash();

        doc.ed25519PublicKeyHash = newPubKeyHash;
        doc.updated = block.timestamp;

        emit DidKeyRotated(didHash, newPubKeyHash, block.timestamp);
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

    // ─── Sybil Resistance: Network Registration ──────────────────────────
    // DID creation is free (key generation). Participating in reputation and
    // validator eligibility requires a slashable stake ≥ minRegistrationStake,
    // so an attacker cannot cheaply mint identities to game reputation.

    /// @notice Register a DID for network participation (reputation/validator eligibility).
    /// @dev Requires the controller to currently stake ≥ minRegistrationStake.
    ///      Ongoing eligibility (e.g. auto-deregister on slash) hangs off this seam.
    function registerForNetwork(bytes32 didHash) external {
        DidDocument storage doc = documents[didHash];
        if (doc.created == 0) revert DidNotFound(didHash);
        if (!doc.active) revert DidNotActive(didHash);
        if (doc.controller != msg.sender) revert NotController(didHash, msg.sender);

        uint256 staked = 0;
        if (minRegistrationStake != 0) {
            staked = stakeSource.stakedBalanceOf(msg.sender);
            if (staked < minRegistrationStake) {
                revert InsufficientRegistrationStake(msg.sender, staked, minRegistrationStake);
            }
        }
        _registered[didHash] = true;
        emit NetworkRegistered(didHash, msg.sender, staked);
    }

    /// @notice Voluntarily remove a DID from network participation.
    function deregisterFromNetwork(bytes32 didHash) external {
        DidDocument storage doc = documents[didHash];
        if (doc.created == 0) revert DidNotFound(didHash);
        if (doc.controller != msg.sender) revert NotController(didHash, msg.sender);
        if (!_registered[didHash]) revert NotRegistered(didHash);

        _registered[didHash] = false;
        emit NetworkDeregistered(didHash, msg.sender);
    }

    /// @notice Whether a DID is registered for reputation/validator eligibility.
    function isRegistered(bytes32 didHash) external view returns (bool) {
        return _registered[didHash];
    }

    /// @notice Set the stake source (any contract exposing stakedBalanceOf, e.g. NeunodeToken).
    function setStakeSource(address source) external onlyOwner {
        stakeSource = IStakeSource(source);
        emit StakeSourceUpdated(source);
    }

    /// @notice Set the minimum stake required to register for the network.
    function setMinRegistrationStake(uint256 newMin) external onlyOwner {
        emit MinRegistrationStakeUpdated(minRegistrationStake, newMin);
        minRegistrationStake = newMin;
    }

    /// @notice Transfer protocol administration to a new owner (normally governance).
    function transferOwnership(address newOwner) external onlyOwner {
        if (newOwner == address(0)) revert InvalidOwner();
        address previousOwner = owner;
        owner = newOwner;
        emit OwnershipTransferred(previousOwner, newOwner);
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
    /// @param signature The 65-byte ECDSA signature (EIP-2 compliant)
    function verifySignature(bytes32 didHash, bytes32 messageHash, bytes calldata signature)
        external
        view
        returns (bool)
    {
        DidDocument storage doc = documents[didHash];
        if (doc.created == 0 || !doc.active) return false;

        address signer = messageHash.toEthSignedMessageHash().recover(signature);
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
