// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title IModelRegistry — Interface for model lineage DAG with content-addressed models
interface IModelRegistry {
    // ─── Types ────────────────────────────────────────────────────────────

    enum ContributionType {
        PreTraining, // 0
        FineTune, // 1
        RL, // 2
        Data, // 3
        Compute, // 4
        Serving // 5
    }

    struct ModelInfo {
        bytes32 cid; // Content identifier (SHA-256 of safetensors)
        address contributor; // Agent DID controller who registered
        ContributionType contribution; // Type of contribution
        string metadataURI; // Off-chain metadata URI
        uint256 registeredAt; // Timestamp of registration
        bool exists; // Whether model has been registered
    }

    // ─── Events ───────────────────────────────────────────────────────────

    event ModelRegistered(
        bytes32 indexed cid,
        address indexed contributor,
        ContributionType contribution,
        bytes32[] parentCids
    );

    event LineageExtended(
        bytes32 indexed parentCid, bytes32 indexed childCid, address indexed contributor
    );

    // ─── Functions ────────────────────────────────────────────────────────

    function registerModel(
        bytes32 cid,
        bytes32[] calldata parentCids,
        ContributionType contribution,
        string calldata metadataURI,
        bytes32 derivationProofHash
    ) external;

    function getModel(bytes32 cid) external view returns (ModelInfo memory);
    function getParents(bytes32 cid) external view returns (bytes32[] memory);
    function getChildren(bytes32 cid) external view returns (bytes32[] memory);
    function getLineageDepth(bytes32 cid) external view returns (uint256);
    function modelExists(bytes32 cid) external view returns (bool);
}
