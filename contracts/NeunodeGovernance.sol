// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title NeunodeGovernance
/// @notice Governance contract for Neunode L1 – manages validator set proposals,
///         registration, deactivation, slashing, and reputation parameter updates.
/// @dev All state transitions are epoch-boundary. Validator set changes are
///      queued and finalized after a mandatory cooldown.
contract NeunodeGovernance {
    // ============================================================
    // Custom Errors
    // ============================================================
    error NotOwner();
    error NotActiveValidator();
    error AlreadyRegistered();
    error AlreadyActive();
    error InsufficientStake();
    error InvalidParameters();
    error ProposalNotFound();
    error ProposalAlreadyFinalized();
    error ProposalNotYetReady();
    error CooldownNotPassed();
    error ZeroAddress();
    error ArrayLengthMismatch();

    // ============================================================
    // Structs
    // ============================================================
    struct Validator {
        uint256 stake;            // staked NEU wei
        uint256 reputationScore;  // cached reputation score (0..1e18)
        bool active;
        uint256 registeredAt;     // block number of registration
        uint256 lastActive;       // last block the validator was seen active (on-chain)
        uint256 slashedCount;
    }

    struct ValidatorSetProposal {
        address[] validators;
        uint256[] weights;           // voting power assigned to each validator
        uint256 targetEpoch;
        uint256 proposedAt;          // block timestamp
        bool finalized;
    }

    struct ReputationWeights {
        uint256 weightStake;        // basis points (e.g., 3000 = 30%)
        uint256 weightAttestation;  // basis points
        uint256 weightActivity;     // basis points
        uint256 weightVerification; // basis points
        uint256 weightTenure;       // basis points
        // total must equal 10000 (100%)
    }

    // ============================================================
    // State Variables
    // ============================================================
    address public owner;

    /// @notice Minimum stake required to register as a validator (in NEU wei).
    uint256 public minStake;

    /// @notice Cooldown period (in seconds) between proposal and finalization.
    uint256 public proposalCooldown;

    /// @notice Current epoch number.
    uint256 public currentEpoch;

    /// @notice Mapping from validator address to its info.
    mapping(address => Validator) public validators;

    /// @notice List of currently active validator addresses.
    address[] public activeValidatorList;

    /// @notice Incremental proposal ID.
    uint256 public proposalCount;

    /// @notice Mapping from proposal ID to proposal details.
    mapping(uint256 => ValidatorSetProposal) public proposals;

    /// @notice Current reputation weights.
    ReputationWeights public weights;

    /// @notice Whether the contract has been initialized.
    bool public initialized;

    // ============================================================
    // Events
    // ============================================================
    event ValidatorRegistered(
        address indexed validator,
        uint256 stake,
        uint256 timestamp
    );
    event ValidatorDeactivated(
        address indexed validator,
        uint256 timestamp
    );
    event ValidatorSlashed(
        address indexed validator,
        uint256 amount,
        string reason
    );
    event ValidatorSetProposed(
        uint256 indexed proposalId,
        address[] validators,
        uint256[] weights,
        uint256 targetEpoch,
        uint256 timestamp
    );
    event ValidatorSetFinalized(
        uint256 indexed proposalId,
        uint256 epoch,
        uint256 timestamp
    );
    event ParametersUpdated(
        ReputationWeights newWeights,
        uint256 newMinStake,
        uint256 newProposalCooldown,
        uint256 timestamp
    );

    // ============================================================
    // Modifiers
    // ============================================================
    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    modifier onlyActiveValidator() {
        if (!validators[msg.sender].active) revert NotActiveValidator();
        _;
    }

    // ============================================================
    // Constructor
    // ============================================================
    /// @notice Initializes the contract with an owner and initial parameters.
    /// @param _owner Address that will own the governance contract.
    /// @param _minStake Minimum stake (wei) required to register as validator.
    /// @param _proposalCooldown Cooldown period in seconds for proposals.
    /// @param _weightStake Weight for stake factor (basis points).
    /// @param _weightAttestation Weight for attestation factor.
    /// @param _weightActivity Weight for activity factor.
    /// @param _weightVerification Weight for verification factor.
    /// @param _weightTenure Weight for tenure factor.
    constructor(
        address _owner,
        uint256 _minStake,
        uint256 _proposalCooldown,
        uint256 _weightStake,
        uint256 _weightAttestation,
        uint256 _weightActivity,
        uint256 _weightVerification,
        uint256 _weightTenure
    ) {
        if (_owner == address(0)) revert ZeroAddress();
        owner = _owner;
        minStake = _minStake;
        proposalCooldown = _proposalCooldown;
        _setWeights(
            _weightStake,
            _weightAttestation,
            _weightActivity,
            _weightVerification,
            _weightTenure
        );
        initialized = true;
        currentEpoch = 1; // start at epoch 1
    }

    // ============================================================
    // Validator Registration
    // ============================================================
    /// @notice Register as a new validator. The caller must send exactly `stake`
    ///         in NEU (the native gas token). The stake is locked in the contract.
    /// @param stake Amount of NEU to stake (must be >= minStake).
    function registerValidator(uint256 stake) external payable {
        if (validators[msg.sender].active) revert AlreadyActive();
        if (validators[msg.sender].stake > 0) revert AlreadyRegistered();
        if (stake < minStake) revert InsufficientStake();
        if (msg.value != stake) revert InsufficientStake();

        validators[msg.sender] = Validator({
            stake: stake,
            reputationScore: 0,
            active: true,
            registeredAt: block.number,
            lastActive: block.number,
            slashedCount: 0
        });
        activeValidatorList.push(msg.sender);

        emit ValidatorRegistered(msg.sender, stake, block.timestamp);
    }

    // ============================================================
    // Deactivation
    // ============================================================
    /// @notice Voluntary deactivation. The validator will be removed from the
    ///         active set at the next epoch boundary. Stake can be withdrawn
    ///         after a withdrawal delay (not implemented here – separate contract).
    function deactivateValidator() external onlyActiveValidator {
        validators[msg.sender].active = false;
        emit ValidatorDeactivated(msg.sender, block.timestamp);
    }

    /// @notice Owner‑forced deactivation of a validator (e.g., for inactivity).
    /// @param validator Address of the validator to deactivate.
    function forceDeactivateValidator(address validator) external onlyOwner {
        if (!validators[validator].active) revert NotActiveValidator();
        validators[validator].active = false;
        emit ValidatorDeactivated(validator, block.timestamp);
    }

    // ============================================================
    // Slashing
    // ============================================================
    /// @notice Slash a validator for misbehavior. A portion of the stake is
    ///         burned or sent to a treasury (here we send to owner for simplicity).
    /// @param validator Address of the validator to slash.
    /// @param amount Amount of NEU to slash (in wei).
    /// @param reason Human‑readable reason for the slashing.
    function slash(address validator, uint256 amount, string calldata reason)
        external
        onlyOwner
    {
        if (amount == 0) revert InvalidParameters();
        if (amount > validators[validator].stake)
            revert InvalidParameters();

        validators[validator].stake -= amount;
        validators[validator].slashedCount++;
        // Transfer slashed funds to owner (treasury)
        (bool sent,) = payable(owner).call{value: amount}("");
        require(sent, "Slash transfer failed");

        emit ValidatorSlashed(validator, amount, reason);
    }

    // ============================================================
    // Validator Set Proposals
    // ============================================================
    /// @notice Propose a new validator set for a future epoch.
    /// @param validatorsList Array of validator addresses.
    /// @param weightsList Array of voting weights (must sum to 1e18 for meaningful distribution).
    /// @param targetEpoch The epoch number when this set should become active.
    /// @return proposalId The ID of the created proposal.
    function proposeValidatorSet(
        address[] calldata validatorsList,
        uint256[] calldata weightsList,
        uint256 targetEpoch
    ) external onlyOwner returns (uint256 proposalId) {
        if (validatorsList.length != weightsList.length)
            revert ArrayLengthMismatch();
        if (targetEpoch <= currentEpoch) revert InvalidParameters();

        proposalId = ++proposalCount;
        proposals[proposalId] = ValidatorSetProposal({
            validators: validatorsList,
            weights: weightsList,
            targetEpoch: targetEpoch,
            proposedAt: block.timestamp,
            finalized: false
        });

        emit ValidatorSetProposed(
            proposalId,
            validatorsList,
            weightsList,
            targetEpoch,
            block.timestamp
        );
    }

    /// @notice Finalize a previously proposed validator set after cooldown.
    ///         This updates the active validator list and moves to the target epoch.
    /// @param proposalId The ID of the proposal to finalize.
    function finalizeValidatorSet(uint256 proposalId) external onlyOwner {
        ValidatorSetProposal storage proposal = proposals[proposalId];
        if (proposal.proposedAt == 0) revert ProposalNotFound();
        if (proposal.finalized) revert ProposalAlreadyFinalized();
        if (block.timestamp < proposal.proposedAt + proposalCooldown)
            revert CooldownNotPassed();

        // Replace active validator list
        delete activeValidatorList;
        for (uint256 i = 0; i < proposal.validators.length; i++) {
            address val = proposal.validators[i];
            // Ensure validator is registered and active
            if (!validators[val].active) {
                // Propose to activate if currently inactive? Here we require active.
                // In production, you might want to re‑register if they were previously registered.
                // For now, skip inactive validators.
                continue;
            }
            activeValidatorList.push(val);
        }
        // Update epoch
        currentEpoch = proposal.targetEpoch;
        proposal.finalized = true;

        emit ValidatorSetFinalized(proposalId, currentEpoch, block.timestamp);
    }

    // ============================================================
    // Parameter Updates
    // ============================================================
    /// @notice Update reputation weights and other governance parameters.
    /// @param _weightStake New weight for stake.
    /// @param _weightAttestation New weight for attestation.
    /// @param _weightActivity New weight for activity.
    /// @param _weightVerification New weight for verification.
    /// @param _weightTenure New weight for tenure.
    /// @param _minStake New minimum stake.
    /// @param _proposalCooldown New proposal cooldown in seconds.
    function updateParameters(
        uint256 _weightStake,
        uint256 _weightAttestation,
        uint256 _weightActivity,
        uint256 _weightVerification,
        uint256 _weightTenure,
        uint256 _minStake,
        uint256 _proposalCooldown
    ) external onlyOwner {
        _setWeights(
            _weightStake,
            _weightAttestation,
            _weightActivity,
            _weightVerification,
            _weightTenure
        );
        minStake = _minStake;
        proposalCooldown = _proposalCooldown;

        emit ParametersUpdated(
            ReputationWeights({
                weightStake: _weightStake,
                weightAttestation: _weightAttestation,
                weightActivity: _weightActivity,
                weightVerification: _weightVerification,
                weightTenure: _weightTenure
            }),
            _minStake,
            _proposalCooldown,
            block.timestamp
        );
    }

    // ============================================================
    // Internal Helpers
    // ============================================================
    /// @dev Validates and sets reputation weights (must sum to exactly 10000).
    function _setWeights(
        uint256 _wStake,
        uint256 _wAttest,
        uint256 _wActivity,
        uint256 _wVerify,
        uint256 _wTenure
    ) internal pure {
        if (_wStake + _wAttest + _wActivity + _wVerify + _wTenure != 10000)
            revert InvalidParameters();
        weights = ReputationWeights({
            weightStake: _wStake,
            weightAttestation: _wAttest,
            weightActivity: _wActivity,
            weightVerification: _wVerify,
            weightTenure: _wTenure
        });
    }

    // ============================================================
    // View Functions
    // ============================================================
    /// @notice Returns the list of active validator addresses.
    /// @return Array of active validator addresses.
    function getActiveValidators() external view returns (address[] memory) {
        return activeValidatorList;
    }

    /// @notice Returns the full `Validator` struct for a given address.
    function getValidator(address validator)
        external
        view
        returns (Validator memory)
    {
        return validators[validator];
    }

    /// @notice Returns the number of active validators.
    function activeValidatorCount() external view returns (uint256) {
        return activeValidatorList.length;
    }

    /// @notice Returns the proposal details for a given ID.
    function getProposal(uint256 proposalId)
        external
        view
        returns (ValidatorSetProposal memory)
    {
        return proposals[proposalId];
    }

    /// @notice Returns the current reputation weights and other parameters.
    function getParameters()
        external
        view
        returns (ReputationWeights memory, uint256, uint256)
    {
        return (weights, minStake, proposalCooldown);
    }

    // ============================================================
    // Fallback / Receive
    // ============================================================
    /// @notice Accept direct ETH transfers (e.g., for slashing recovery?).
    receive() external payable {}
}