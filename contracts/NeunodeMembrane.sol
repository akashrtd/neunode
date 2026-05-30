// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Context.sol";
import "@openzeppelin/contracts/utils/Address.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/draft-EIP712.sol";

/**
 * @title NeunodeMembrane
 * @notice Manages escrow, bounty creation, agent identities, and reputation attestations.
 *         Designed for Neunode L1 but EVM-compatible.
 */
contract NeunodeMembrane is Context, ReentrancyGuard, EIP712 {
    using Address for address payable;
    using ECDSA for bytes32;

    /* ========== STATE ========== */

    /// @notice Bounty identifier => Bounty structure
    mapping(bytes32 => Bounty) public bounties;

    /// @notice agent address => AgentInfo
    mapping(address => AgentInfo) public agents;

    /// @notice attester => attestee => Attestation
    mapping(address => mapping(address => Attestation)) public attestations;

    /// @notice Total number of registered agents
    uint256 public agentCount;

    /// @notice Total number of bounties created
    uint256 public bountyCount;

    /// @notice Contract owner/operator
    address public owner;

    /// @notice Reputation weight categories (5-factor: stake, attest, activity, verify, tenure)
    enum ReputationFactor { Stake, Attestation, Activity, Verification, Tenure }

    /* ========== STRUCTURES ========== */

    struct AgentInfo {
        string name;
        string metadataURI;     // off-chain identity data
        uint256 registrationTime;
        bool isActive;
    }

    struct Bounty {
        bytes32 id;
        address payable creator;
        address payable claimant;
        uint256 reward;                 // in neu wei
        uint256 expiration;             // block timestamp
        bool funded;
        bool claimed;
        bool refunded;
        bytes conditionHash;            // keccak256 of off-chain conditions
    }

    struct Attestation {
        ReputationFactor factor;
        uint256 score;                  // 0-100
        string evidenceURI;
        uint256 timestamp;
        bool revoked;
    }

    /* ========== EVENTS ========== */

    event AgentRegistered(address indexed agent, string name, string metadataURI);
    event AgentUpdated(address indexed agent, string name, string metadataURI, bool isActive);
    event BountyCreated(bytes32 indexed bountyId, address indexed creator, uint256 reward, uint256 expiration, bytes conditionHash);
    event BountyFunded(bytes32 indexed bountyId, address indexed funder, uint256 amount);
    event BountyClaimed(bytes32 indexed bountyId, address indexed claimant);
    event BountyRefunded(bytes32 indexed bountyId, address indexed recipient);
    event AttestationMade(address indexed attester, address indexed agent, ReputationFactor factor, uint256 score, string evidenceURI);
    event AttestationRevoked(address indexed attester, address indexed agent, ReputationFactor factor);

    /* ========== MODIFIERS ========== */

    modifier onlyOwner() {
        require(_msgSender() == owner, "NeunodeMembrane: not owner");
        _;
    }

    modifier onlyRegisteredAgent() {
        require(agents[_msgSender()].isActive, "NeunodeMembrane: agent not registered");
        _;
    }

    modifier onlyBountyCreator(bytes32 bountyId) {
        require(bounties[bountyId].creator == _msgSender(), "NeunodeMembrane: not bounty creator");
        _;
    }

    modifier validBounty(bytes32 bountyId) {
        require(bounties[bountyId].id != bytes32(0), "NeunodeMembrane: bounty does not exist");
        _;
    }

    /* ========== CONSTRUCTOR ========== */

    constructor() EIP712("NeunodeMembrane", "1") {
        owner = _msgSender();
    }

    /* ========== AGENT IDENTITY FUNCTIONS ========== */

    /// @notice Register a new agent identity
    /// @param name Agent's chosen name
    /// @param metadataURI URI to off-chain identity data
    function registerAgent(string calldata name, string calldata metadataURI) external {
        address agent = _msgSender();
        require(!agents[agent].isActive, "NeunodeMembrane: already registered");
        require(bytes(name).length > 0, "NeunodeMembrane: name required");

        agents[agent] = AgentInfo({
            name: name,
            metadataURI: metadataURI,
            registrationTime: block.timestamp,
            isActive: true
        });
        agentCount++;

        emit AgentRegistered(agent, name, metadataURI);
    }

    /// @notice Update agent identity details
    /// @param name New name
    /// @param metadataURI New metadata URI
    /// @param isActive New active status
    function updateAgent(string calldata name, string calldata metadataURI, bool isActive) external onlyRegisteredAgent {
        address agent = _msgSender();
        require(bytes(name).length > 0, "NeunodeMembrane: name required");

        agents[agent].name = name;
        agents[agent].metadataURI = metadataURI;
        agents[agent].isActive = isActive;

        emit AgentUpdated(agent, name, metadataURI, isActive);
    }

    /* ========== BOUNTY & ESCROW FUNCTIONS ========== */

    /// @notice Create a new bounty. Reward is specified in neu wei.
    /// @param reward Amount of neu to be locked in escrow (msg.value)
    /// @param expiration Unix timestamp after which bounty can be refunded
    /// @param conditionHash keccak256 hash of off-chain bounty conditions
    /// @return bountyId Generated identifier
    function createBounty(uint256 reward, uint256 expiration, bytes calldata conditionHash)
        external
        payable
        nonReentrant
        returns (bytes32 bountyId)
    {
        require(msg.value == reward, "NeunodeMembrane: reward mismatch");
        require(expiration > block.timestamp, "NeunodeMembrane: expiration in past");
        require(conditionHash.length == 32, "NeunodeMembrane: invalid condition hash");

        bountyId = keccak256(abi.encodePacked(_msgSender(), block.timestamp, reward, bountyCount));
        bountyCount++;

        bounties[bountyId] = Bounty({
            id: bountyId,
            creator: payable(_msgSender()),
            claimant: payable(address(0)),
            reward: reward,
            expiration: expiration,
            funded: true,
            claimed: false,
            refunded: false,
            conditionHash: conditionHash
        });

        emit BountyCreated(bountyId, _msgSender(), reward, expiration, conditionHash);
    }

    /// @notice Fund an existing bounty (add more neu to escrow)
    /// @param bountyId ID of the bounty
    function fundBounty(bytes32 bountyId) external payable validBounty(bountyId) nonReentrant {
        Bounty storage bounty = bounties[bountyId];
        require(!bounty.claimed, "NeunodeMembrane: already claimed");
        require(!bounty.refunded, "NeunodeMembrane: already refunded");
        require(bounty.expiration > block.timestamp, "NeunodeMembrane: expired");

        bounty.reward += msg.value;
        emit BountyFunded(bountyId, _msgSender(), msg.value);
    }

    /// @notice Claim a bounty as the designated claimant after satisfying conditions
    /// @param bountyId ID of the bounty
    /// @param signature Creator's signature authorizing claim (EIP-712)
    /// @dev The signature must be from the bounty creator over a message containing the bounty ID and claimant address.
    function claimBounty(bytes32 bountyId, bytes calldata signature)
        external
        nonReentrant
        validBounty(bountyId)
    {
        Bounty storage bounty = bounties[bountyId];
        require(!bounty.claimed, "NeunodeMembrane: already claimed");
        require(!bounty.refunded, "NeunodeMembrane: already refunded");
        require(bounty.expiration >= block.timestamp, "NeunodeMembrane: expired");

        // Verify EIP-712 signature from creator authorizing claimant
        address claimant = _msgSender();
        bytes32 structHash = keccak256(
            abi.encode(
                keccak256("ClaimBounty(bytes32 bountyId,address claimant)"),
                bountyId,
                claimant
            )
        );
        bytes32 typedHash = _hashTypedDataV4(structHash);
        address signer = typedHash.recover(signature);
        require(signer == bounty.creator, "NeunodeMembrane: invalid signature");

        // Mark as claimed and transfer reward
        bounty.claimed = true;
        bounty.claimant = payable(claimant);

        uint256 reward = bounty.reward;
        bounty.reward = 0; // prevent reentrancy
        payable(claimant).sendValue(reward);

        emit BountyClaimed(bountyId, claimant);
    }

    /// @notice Refund a bounty (only creator, after expiration or mutual agreement)
    /// @param bountyId ID of the bounty
    function refundBounty(bytes32 bountyId)
        external
        nonReentrant
        onlyBountyCreator(bountyId)
        validBounty(bountyId)
    {
        Bounty storage bounty = bounties[bountyId];
        require(!bounty.claimed, "NeunodeMembrane: already claimed");
        require(!bounty.refunded, "NeunodeMembrane: already refunded");
        require(
            block.timestamp > bounty.expiration,
            "NeunodeMembrane: bounty not expired"
        );

        bounty.refunded = true;
        uint256 reward = bounty.reward;
        bounty.reward = 0;
        payable(_msgSender()).sendValue(reward);

        emit BountyRefunded(bountyId, _msgSender());
    }

    /* ========== REPUTATION ATTESTATION FUNCTIONS ========== */

    /// @notice Attest to another agent's reputation factor.
    /// @param agent Address of the agent being attested
    /// @param factor Reputation factor category
    /// @param score Score 0-100
    /// @param evidenceURI URI to evidence supporting the attestation
    function attestReputation(
        address agent,
        ReputationFactor factor,
        uint256 score,
        string calldata evidenceURI
    ) external onlyRegisteredAgent {
        require(agents[agent].isActive, "NeunodeMembrane: target not active");
        require(score <= 100, "NeunodeMembrane: score out of range");

        address attester = _msgSender();
        require(attester != agent, "NeunodeMembrane: self-attest not allowed");

        Attestation storage att = attestations[attester][agent];
        require(
            !att.revoked && att.timestamp == 0,
            "NeunodeMembrane: existing attestation, revoke first"
        );

        attestations[attester][agent] = Attestation({
            factor: factor,
            score: score,
            evidenceURI: evidenceURI,
            timestamp: block.timestamp,
            revoked: false
        });

        emit AttestationMade(attester, agent, factor, score, evidenceURI);
    }

    /// @notice Revoke a previous attestation
    /// @param agent Address of the agent whose attestation is revoked
    function revokeAttestation(address agent) external onlyRegisteredAgent {
        address attester = _msgSender();
        Attestation storage att = attestations[attester][agent];
        require(att.timestamp != 0 && !att.revoked, "NeunodeMembrane: no active attestation");

        att.revoked = true;
        emit AttestationRevoked(attester, agent, att.factor);
    }

    /* ========== ADMIN FUNCTIONS ========== */

    /// @notice Transfer ownership
    /// @param newOwner Address of new owner
    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "NeunodeMembrane: zero address");
        owner = newOwner;
    }

    /// @notice Emergency withdraw of any accidental ETH sent to contract (only owner)
    function emergencyWithdraw() external onlyOwner nonReentrant {
        payable(owner).sendValue(address(this).balance);
    }

    /* ========== VIEW FUNCTIONS ========== */

    /// @notice Get agent info
    function getAgent(address agent) external view returns (AgentInfo memory) {
        return agents[agent];
    }

    /// @notice Get bounty details
    function getBounty(bytes32 bountyId) external view returns (Bounty memory) {
        return bounties[bountyId];
    }

    /// @notice Get attestation details
    function getAttestation(address attester, address agent) external view returns (Attestation memory) {
        return attestations[attester][agent];
    }

    /// @notice Check if an address is a registered active agent
    function isRegisteredAgent(address agent) external view returns (bool) {
        return agents[agent].isActive;
    }

    /// @notice Get the EIP-712 domain separator
    function domainSeparator() external view returns (bytes32) {
        return _domainSeparatorV4();
    }
}