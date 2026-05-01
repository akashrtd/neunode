// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title NeunodeRegistry — Agent capability and endpoint registry
/// @notice Maps registered DIDs to their capabilities (JSON), endpoints, and stake.
///         Requires DID to exist in NeunodeIdentity.
// NOTE: INeunodeIdentity should be imported from NeunodeIdentity.sol but is kept inline
//       for now until Foundry remapping paths are configured for the interface.
interface INeunodeIdentity {
    function getController(bytes32 didHash) external view returns (address);
    function isActive(bytes32 didHash) external view returns (bool);
}

contract NeunodeRegistry {
    // ─── Types ────────────────────────────────────────────────────────────

    struct AgentRegistration {
        bytes32 didHash; // DID identifier
        string capabilities; // JSON string of agent capabilities
        string endpoint; // P2P multiaddr or HTTP endpoint
        uint256 stakeAmount; // Staked tokens (tracked externally)
        uint256 registeredAt;
        uint256 updatedAt;
        bool active;
    }

    // ─── Storage ──────────────────────────────────────────────────────────

    INeunodeIdentity public immutable identity;

    mapping(bytes32 => AgentRegistration) public agents; // didHash → registration
    bytes32[] public agentList; // All registered DIDs
    uint256 public activeCount;

    // ─── Events ───────────────────────────────────────────────────────────

    event AgentRegistered(bytes32 indexed didHash, address indexed controller, uint256 timestamp);
    event AgentUpdated(bytes32 indexed didHash, address indexed controller, uint256 timestamp);
    event AgentDeregistered(bytes32 indexed didHash, address indexed controller, uint256 timestamp);

    // ─── Errors ───────────────────────────────────────────────────────────

    error NotDidController(bytes32 didHash, address caller);
    error DidNotActive(bytes32 didHash);
    error AgentNotFound(bytes32 didHash);
    error AgentAlreadyRegistered(bytes32 didHash);
    error AgentNotActive(bytes32 didHash);
    error EmptyCapabilities();
    error EmptyEndpoint();
    error ZeroAddress();

    // ─── Constructor ──────────────────────────────────────────────────────

    constructor(address identity_) {
        if (identity_ == address(0)) revert ZeroAddress();
        identity = INeunodeIdentity(identity_);
    }

    // ─── Modifiers ────────────────────────────────────────────────────────

    modifier onlyDidController(bytes32 didHash) {
        address controller = identity.getController(didHash);
        if (controller != msg.sender) revert NotDidController(didHash, msg.sender);
        _;
    }

    modifier requireActiveDid(bytes32 didHash) {
        if (!identity.isActive(didHash)) revert DidNotActive(didHash);
        _;
    }

    // ─── Functions ────────────────────────────────────────────────────────

    /// @notice Register a new agent with capabilities and endpoint
    function register(bytes32 didHash, string calldata capabilities, string calldata endpoint)
        external
        onlyDidController(didHash)
        requireActiveDid(didHash)
    {
        if (bytes(capabilities).length == 0) revert EmptyCapabilities();
        if (bytes(endpoint).length == 0) revert EmptyEndpoint();
        if (agents[didHash].active) revert AgentAlreadyRegistered(didHash);

        agents[didHash] = AgentRegistration({
            didHash: didHash,
            capabilities: capabilities,
            endpoint: endpoint,
            stakeAmount: 0,
            registeredAt: block.timestamp,
            updatedAt: block.timestamp,
            active: true
        });

        agentList.push(didHash);
        activeCount++;

        emit AgentRegistered(didHash, msg.sender, block.timestamp);
    }

    /// @notice Update agent capabilities and/or endpoint
    function update(bytes32 didHash, string calldata capabilities, string calldata endpoint)
        external
        onlyDidController(didHash)
    {
        AgentRegistration storage agent = agents[didHash];
        if (!agent.active) revert AgentNotActive(didHash);
        if (bytes(capabilities).length == 0) revert EmptyCapabilities();
        if (bytes(endpoint).length == 0) revert EmptyEndpoint();

        agent.capabilities = capabilities;
        agent.endpoint = endpoint;
        agent.updatedAt = block.timestamp;

        emit AgentUpdated(didHash, msg.sender, block.timestamp);
    }

    /// @notice Deregister an agent
    function deregister(bytes32 didHash) external onlyDidController(didHash) {
        AgentRegistration storage agent = agents[didHash];
        if (!agent.active) revert AgentNotActive(didHash);

        agent.active = false;
        agent.updatedAt = block.timestamp;
        activeCount--;

        emit AgentDeregistered(didHash, msg.sender, block.timestamp);
    }

    /// @notice Get agent registration details
    function getAgent(bytes32 didHash) external view returns (AgentRegistration memory) {
        if (agents[didHash].registeredAt == 0) revert AgentNotFound(didHash);
        return agents[didHash];
    }

    /// @notice Get all active agent DIDs
    function getActiveAgents() external view returns (bytes32[] memory) {
        bytes32[] memory active = new bytes32[](activeCount);
        uint256 count;
        for (uint256 i = 0; i < agentList.length; i++) {
            if (agents[agentList[i]].active) {
                active[count] = agentList[i];
                count++;
            }
        }
        return active;
    }

    /// @notice Get total registered agents (including deregistered)
    function getTotalAgents() external view returns (uint256) {
        return agentList.length;
    }

    /// @notice Get a paginated slice of agent DIDs
    function getAgents(uint256 offset, uint256 limit) external view returns (bytes32[] memory) {
        uint256 total = agentList.length;
        uint256 end = offset + limit > total ? total : offset + limit;
        bytes32[] memory result = new bytes32[](end - offset);
        for (uint256 i = offset; i < end; i++) {
            result[i - offset] = agentList[i];
        }
        return result;
    }
}
