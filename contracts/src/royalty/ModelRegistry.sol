// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/AccessControl.sol";
import "./IModelRegistry.sol";

/// @title ModelRegistry — Model lineage DAG with content-addressed models
/// @notice Tracks model provenance as a DAG. Each model has a content hash (CID),
///         optional parent models, a contributor address, and a contribution type.
///         REGISTRAR_ROLE controls who can register models.
contract ModelRegistry is IModelRegistry, AccessControl {
    // ─── Storage ──────────────────────────────────────────────────────────

    mapping(bytes32 => ModelInfo) private _models;
    mapping(bytes32 => bytes32[]) private _parentCids;
    mapping(bytes32 => bytes32[]) private _childCids;
    bytes32[] private _modelList;

    bytes32 public constant REGISTRAR_ROLE = keccak256("REGISTRAR_ROLE");

    // ─── Errors ───────────────────────────────────────────────────────────

    error ModelNotFound(bytes32 cid);
    error ModelAlreadyExists(bytes32 cid);
    error InvalidCid(bytes32 cid);
    error ParentNotFound(bytes32 cid);
    error DerivationProofRequired(bytes32 parentCid);

    // ─── Constructor ──────────────────────────────────────────────────────

    constructor() {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(REGISTRAR_ROLE, msg.sender);
    }

    // ─── Registration ─────────────────────────────────────────────────────

    /// @notice Register a new model in the lineage DAG
    /// @param cid Content identifier (SHA-256 of safetensors file)
    /// @param parentCids Array of parent model CIDs (empty for root model)
    /// @param contribution Type of contribution (PreTraining, FineTune, etc.)
    /// @param metadataURI Off-chain metadata URI
    /// @param derivationProofHash Hash proving derivation from parents (e.g., training logs hash)
    function registerModel(
        bytes32 cid,
        bytes32[] calldata parentCids,
        ContributionType contribution,
        string calldata metadataURI,
        bytes32 derivationProofHash
    ) external onlyRole(REGISTRAR_ROLE) {
        if (cid == bytes32(0)) revert InvalidCid(cid);
        if (_models[cid].exists) revert ModelAlreadyExists(cid);

        // Validate all parents exist and derivation proof is provided
        if (parentCids.length > 0 && derivationProofHash == bytes32(0)) {
            revert DerivationProofRequired(parentCids[0]);
        }
        for (uint256 i = 0; i < parentCids.length; i++) {
            if (!_models[parentCids[i]].exists) revert ParentNotFound(parentCids[i]);
        }

        _models[cid] = ModelInfo({
            cid: cid,
            contributor: msg.sender,
            contribution: contribution,
            metadataURI: metadataURI,
            registeredAt: block.timestamp,
            exists: true
        });

        // Store parent links
        for (uint256 i = 0; i < parentCids.length; i++) {
            _parentCids[cid].push(parentCids[i]);
            _childCids[parentCids[i]].push(cid);
        }

        _modelList.push(cid);

        emit ModelRegistered(cid, msg.sender, contribution, parentCids);

        // Emit lineage extension events for each parent
        for (uint256 i = 0; i < parentCids.length; i++) {
            emit LineageExtended(parentCids[i], cid, msg.sender);
        }
    }

    // ─── View Functions ───────────────────────────────────────────────────

    /// @notice Get model information by CID
    function getModel(bytes32 cid) external view returns (ModelInfo memory) {
        if (!_models[cid].exists) revert ModelNotFound(cid);
        return _models[cid];
    }

    /// @notice Get parent model CIDs for a model
    function getParents(bytes32 cid) external view returns (bytes32[] memory) {
        if (!_models[cid].exists) revert ModelNotFound(cid);
        return _parentCids[cid];
    }

    /// @notice Get child model CIDs for a model
    function getChildren(bytes32 cid) external view returns (bytes32[] memory) {
        if (!_models[cid].exists) revert ModelNotFound(cid);
        return _childCids[cid];
    }

    /// @notice Get lineage depth from model to root using BFS
    /// @return depth Number of hops to the farthest root ancestor (0 for root models)
    function getLineageDepth(bytes32 cid) external view returns (uint256) {
        if (!_models[cid].exists) revert ModelNotFound(cid);

        uint256 modelCount = _modelList.length;
        uint256 maxDepth = 0;
        uint256 queueSize = 1;
        uint256 head = 0;

        // BFS queue: pairs of (cid, depth)
        bytes32[] memory queueCids = new bytes32[](modelCount);
        uint256[] memory queueDepths = new uint256[](modelCount);
        bytes32[] memory visited = new bytes32[](modelCount);
        uint256 visitedCount = 0;

        queueCids[0] = cid;
        queueDepths[0] = 0;

        while (head < queueSize) {
            bytes32 currentCid = queueCids[head];
            uint256 currentDepth = queueDepths[head];
            head++;

            // Skip if already visited (handles DAG cycles)
            bool alreadyVisited = false;
            for (uint256 j = 0; j < visitedCount; j++) {
                if (visited[j] == currentCid) {
                    alreadyVisited = true;
                    break;
                }
            }
            if (alreadyVisited) continue;

            visited[visitedCount] = currentCid;
            visitedCount++;

            bytes32[] storage parents = _parentCids[currentCid];
            if (parents.length == 0) {
                // Root model — update max depth
                if (currentDepth > maxDepth) {
                    maxDepth = currentDepth;
                }
            } else {
                // Enqueue parents
                for (uint256 i = 0; i < parents.length; i++) {
                    if (queueSize < modelCount) {
                        queueCids[queueSize] = parents[i];
                        queueDepths[queueSize] = currentDepth + 1;
                        queueSize++;
                    }
                }
            }
        }

        return maxDepth;
    }

    /// @notice Check if a model exists
    function modelExists(bytes32 cid) external view returns (bool) {
        return _models[cid].exists;
    }

    /// @notice Get total number of registered models
    function getModelCount() external view returns (uint256) {
        return _modelList.length;
    }
}
