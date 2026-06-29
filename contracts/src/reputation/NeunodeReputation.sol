// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.28;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import {IStakeSource} from "../NeunodeIdentity.sol";

/// @notice Minimal identity-registry read interface (NeunodeIdentity satisfies it).
interface IIdentityRegistry {
    function getDidForAddress(address addr) external view returns (bytes32);
    function isRegistered(bytes32 didHash) external view returns (bool);
}

/// @title NeunodeReputation — On-chain reputation scores and voting power for AI agents
/// @notice Manages per-agent 5-factor reputation scores (stake, attest, activity,
///         verify, tenure), composite score computation, sqrt-mapped voting power,
///         validator set management, epoch finalization, and penalty decay.
///         Each factor has its own oracle role for access control.
contract NeunodeReputation is AccessControl {
    // ─── Types ────────────────────────────────────────────────────────────

    /// @notice Per-factor weight configuration in basis points. Must sum to 10000.
    struct FactorWeights {
        uint16 stake; // default 3000 (30%)
        uint16 attest; // default 2500 (25%)
        uint16 activity; // default 2000 (20%)
        uint16 verify; // default 1500 (15%)
        uint16 tenure; // default 1000 (10%)
    }

    /// @notice Per-agent factor scores, all in basis points 0-10000 (0%-100%).
    ///         Packed into a single storage slot (5 x uint16 = 80 bits).
    struct FactorScores {
        uint16 stake;
        uint16 attest;
        uint16 activity;
        uint16 verify;
        uint16 tenure;
    }

    /// @notice Full reputation record for a registered agent.
    struct ReputationEntry {
        FactorScores scores;
        uint256 compositeScore; // weighted sum minus penalty, 0-10000
        uint256 votingPower; // sqrt(compositeScore) * VOTING_POWER_SCALE / 100
        uint256 lastUpdateEpoch;
        uint256 penaltyEpoch; // 0 = no penalty, >0 = epoch when penalty started
        uint256 penaltyBps; // total penalty basis points (decays over 90 epochs)
        bool isValidator;
    }

    /// @notice Metadata for a finalized epoch.
    struct EpochInfo {
        uint256 startBlock;
        uint256 endBlock;
        bool isFinalized;
    }

    // ─── Constants ────────────────────────────────────────────────────────

    uint256 public constant EPOCH_SIZE = 720; // blocks per epoch (~1h at 5s blocks)
    uint256 public constant SNAPSHOT_WINDOW = 100; // blocks before epoch end for snapshot
    uint256 public constant TRANSITION_BLOCKS = 10; // blocks for validator set transition
    uint256 public constant MAX_VALIDATORS = 100;
    uint256 public constant MIN_REPUTATION_BPS = 5000; // 50% minimum to be validator
    uint256 public constant VOTING_POWER_SCALE = 1e12; // scale factor for voting power
    uint256 public constant MAX_BPS = 10000; // 100% in basis points
    uint256 public constant PENALTY_DECAY_EPOCHS = 90; // penalty decays linearly over 90 epochs

    // ─── Roles ────────────────────────────────────────────────────────────

    bytes32 public constant STAKE_ORACLE_ROLE = keccak256("STAKE_ORACLE_ROLE");
    bytes32 public constant ATTEST_ORACLE_ROLE = keccak256("ATTEST_ORACLE_ROLE");
    bytes32 public constant ACTIVITY_ORACLE_ROLE = keccak256("ACTIVITY_ORACLE_ROLE");
    bytes32 public constant VERIFY_ORACLE_ROLE = keccak256("VERIFY_ORACLE_ROLE");
    bytes32 public constant TENURE_ORACLE_ROLE = keccak256("TENURE_ORACLE_ROLE");
    bytes32 public constant SLASHING_ROLE = keccak256("SLASHING_ROLE");
    bytes32 public constant REPUTATION_ADMIN_ROLE = keccak256("REPUTATION_ADMIN_ROLE");

    // ─── Storage ──────────────────────────────────────────────────────────

    FactorWeights public weights;
    uint256 public minReputationBps;
    uint256 public maxValidators;

    mapping(address => ReputationEntry) private _entries;
    address[] private _validators;
    mapping(address => bool) private _isValidatorActive;

    uint256 public currentEpoch;
    mapping(uint256 => EpochInfo) private _epochs;
    mapping(uint256 => address[]) private _epochValidators;
    mapping(uint256 => uint256[]) private _epochVotingPowers;

    // Sybil resistance: when set, only network-registered (staked) DIDs may validate.
    IIdentityRegistry public identityRegistry;

    // Decentralization: when set, the stake factor (0) is derived from real staked balance.
    IStakeSource public stakeSource;
    uint256 public stakeFactorTarget; // staked balance at which stake factor = 100%

    // ─── Events ───────────────────────────────────────────────────────────

    event FactorScoreUpdated(address indexed agent, uint8 indexed factor, uint16 scoreBps);
    event CompositeScoreUpdated(address indexed agent, uint256 compositeScore);
    event VotingPowerUpdated(address indexed agent, uint256 votingPower);
    event ValidatorRegistered(address indexed agent);
    event ValidatorDeregistered(address indexed agent);
    event EpochFinalized(uint256 indexed epoch, uint256 validatorCount);
    event PenaltyApplied(address indexed validator, uint256 reputationSlashBps);
    event WeightsUpdated(FactorWeights newWeights);
    event IdentityRegistryUpdated(address registry);
    event StakeSourceUpdated(address source);
    event StakeFactorTargetUpdated(uint256 target);
    event StakeFactorRecomputed(address indexed agent, uint16 factorBps);

    // ─── Errors ───────────────────────────────────────────────────────────

    error AgentNotFound(address agent);
    error InsufficientReputation(address agent, uint256 score, uint256 minimum);
    error MaxValidatorsReached(uint256 max);
    error NotAValidator(address agent);
    error AlreadyValidator(address agent);
    error EpochNotFinalized(uint256 epoch);
    error EpochAlreadyFinalized(uint256 epoch);
    error InvalidFactorIndex(uint8 index);
    error InvalidWeightSum(uint256 sum);
    error ScoreOutOfBounds(uint16 score);
    error PenaltyNotDecayed();
    error ArrayLengthMismatch();
    error NotNetworkRegistered(address agent);
    error StakeFactorDerived(); // stake factor is auto-derived; oracle cannot set it
    error StakeDerivationNotConfigured();

    // ─── Constructor ──────────────────────────────────────────────────────

    constructor() {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(REPUTATION_ADMIN_ROLE, msg.sender);

        weights =
            FactorWeights({stake: 3000, attest: 2500, activity: 2000, verify: 1500, tenure: 1000});

        minReputationBps = MIN_REPUTATION_BPS;
        maxValidators = MAX_VALIDATORS;
        currentEpoch = 1;

        // Genesis epoch
        _epochs[1] = EpochInfo({
            startBlock: block.number, endBlock: block.number + EPOCH_SIZE, isFinalized: false
        });
    }

    // ─── Modifiers ────────────────────────────────────────────────────────

    modifier onlyFactorOracle(uint8 factorIndex) {
        if (factorIndex > 4) revert InvalidFactorIndex(factorIndex);
        bytes32 role;
        if (factorIndex == 0) {
            role = STAKE_ORACLE_ROLE;
        } else if (factorIndex == 1) {
            role = ATTEST_ORACLE_ROLE;
        } else if (factorIndex == 2) {
            role = ACTIVITY_ORACLE_ROLE;
        } else if (factorIndex == 3) {
            role = VERIFY_ORACLE_ROLE;
        } else {
            role = TENURE_ORACLE_ROLE;
        }
        if (!hasRole(role, msg.sender)) {
            revert AccessControlUnauthorizedAccount(msg.sender, role);
        }
        _;
    }

    // ─── Score Management ─────────────────────────────────────────────────

    /// @notice Update a single factor score for an agent
    /// @param agent The agent's address
    /// @param factorIndex 0=stake, 1=attest, 2=activity, 3=verify, 4=tenure
    /// @param scoreBps Score in basis points (0-10000)
    function updateFactorScore(address agent, uint8 factorIndex, uint16 scoreBps)
        external
        onlyFactorOracle(factorIndex)
    {
        if (scoreBps > uint16(MAX_BPS)) revert ScoreOutOfBounds(scoreBps);
        if (factorIndex == 0 && address(stakeSource) != address(0)) revert StakeFactorDerived();
        _setFactorScore(agent, factorIndex, scoreBps);
        _recompute(agent);

        emit FactorScoreUpdated(agent, factorIndex, scoreBps);
        emit CompositeScoreUpdated(agent, _entries[agent].compositeScore);
        emit VotingPowerUpdated(agent, _entries[agent].votingPower);
    }

    /// @notice Batch update a single factor across multiple agents
    /// @param agents Array of agent addresses
    /// @param factorIndex 0=stake, 1=attest, 2=activity, 3=verify, 4=tenure
    /// @param scoresBps Array of scores in basis points (0-10000)
    function batchUpdateScores(
        address[] calldata agents,
        uint8 factorIndex,
        uint16[] calldata scoresBps
    ) external onlyFactorOracle(factorIndex) {
        if (agents.length != scoresBps.length) revert ArrayLengthMismatch();
        if (factorIndex == 0 && address(stakeSource) != address(0)) revert StakeFactorDerived();
        for (uint256 i = 0; i < agents.length; i++) {
            if (scoresBps[i] > uint16(MAX_BPS)) revert ScoreOutOfBounds(scoresBps[i]);
            _setFactorScore(agents[i], factorIndex, scoresBps[i]);
            _recompute(agents[i]);

            emit FactorScoreUpdated(agents[i], factorIndex, scoresBps[i]);
            emit CompositeScoreUpdated(agents[i], _entries[agents[i]].compositeScore);
            emit VotingPowerUpdated(agents[i], _entries[agents[i]].votingPower);
        }
    }

    /// @notice Get the composite score for an agent (weighted sum minus penalty)
    /// @param agent The agent's address
    /// @return Composite score in basis points (0-10000)
    function getCompositeScore(address agent) external view returns (uint256) {
        return _entries[agent].compositeScore;
    }

    /// @notice Get the voting power for an agent (sqrt of composite score, scaled)
    /// @param agent The agent's address
    /// @return Voting power (sqrt(compositeScore) * VOTING_POWER_SCALE / 100)
    function getVotingPower(address agent) external view returns (uint256) {
        return _entries[agent].votingPower;
    }

    /// @notice Get all factor scores for an agent
    /// @param agent The agent's address
    /// @return FactorScores struct with all five scores
    function getFactorScores(address agent) external view returns (FactorScores memory) {
        return _entries[agent].scores;
    }

    // ─── Validator Management ─────────────────────────────────────────────

    /// @notice Register the caller as a validator if they have sufficient reputation
    function registerValidator() external {
        ReputationEntry storage entry = _entries[msg.sender];
        if (entry.isValidator) revert AlreadyValidator(msg.sender);
        if (entry.compositeScore < minReputationBps) {
            revert InsufficientReputation(msg.sender, entry.compositeScore, minReputationBps);
        }
        if (_validators.length >= maxValidators) revert MaxValidatorsReached(maxValidators);
        // Sybil resistance: a validator must control a network-registered (staked) DID.
        if (address(identityRegistry) != address(0)) {
            bytes32 didHash = identityRegistry.getDidForAddress(msg.sender);
            if (didHash == bytes32(0) || !identityRegistry.isRegistered(didHash)) {
                revert NotNetworkRegistered(msg.sender);
            }
        }

        entry.isValidator = true;
        _validators.push(msg.sender);
        _isValidatorActive[msg.sender] = true;

        emit ValidatorRegistered(msg.sender);
    }

    /// @notice Deregister the caller from the validator set
    function deregisterValidator() external {
        ReputationEntry storage entry = _entries[msg.sender];
        if (!entry.isValidator) revert NotAValidator(msg.sender);

        entry.isValidator = false;
        _isValidatorActive[msg.sender] = false;

        // Remove from validators array (swap-and-pop)
        for (uint256 i = 0; i < _validators.length; i++) {
            if (_validators[i] == msg.sender) {
                _validators[i] = _validators[_validators.length - 1];
                _validators.pop();
                break;
            }
        }

        emit ValidatorDeregistered(msg.sender);
    }

    /// @notice Get the list of currently active validators
    /// @return Array of validator addresses
    function getActiveValidators() external view returns (address[] memory) {
        return _validators;
    }

    /// @notice Get the total voting power across all active validators
    /// @return Sum of all validator voting powers
    function getTotalVotingPower() external view returns (uint256) {
        uint256 total = 0;
        for (uint256 i = 0; i < _validators.length; i++) {
            total += _entries[_validators[i]].votingPower;
        }
        return total;
    }

    /// @notice Check if an agent is eligible to be a validator
    /// @param agent The agent's address
    /// @return True if composite score >= minimum threshold
    function isEligibleValidator(address agent) external view returns (bool) {
        return _entries[agent].compositeScore >= minReputationBps;
    }

    // ─── Epoch Management ─────────────────────────────────────────────────

    /// @notice Finalize the current epoch and start the next one
    /// @dev Snapshots current validator set, marks epoch finalized, creates next epoch
    function finalizeEpoch() external {
        EpochInfo storage epoch = _epochs[currentEpoch];
        if (epoch.isFinalized) revert EpochAlreadyFinalized(currentEpoch);

        epoch.isFinalized = true;
        epoch.endBlock = block.number;

        // Snapshot validator set and voting powers for this epoch
        address[] storage currentValidators = _validators;
        uint256[] storage powers = _epochVotingPowers[currentEpoch];
        for (uint256 i = 0; i < currentValidators.length; i++) {
            _epochValidators[currentEpoch].push(currentValidators[i]);
            powers.push(_entries[currentValidators[i]].votingPower);
        }

        emit EpochFinalized(currentEpoch, currentValidators.length);

        // Start next epoch
        uint256 nextEpoch = currentEpoch + 1;
        currentEpoch = nextEpoch;
        _epochs[nextEpoch] = EpochInfo({
            startBlock: block.number, endBlock: block.number + EPOCH_SIZE, isFinalized: false
        });
    }

    /// @notice Get the current epoch number
    /// @return Current epoch number (1-indexed)
    function getCurrentEpoch() external view returns (uint256) {
        return currentEpoch;
    }

    /// @notice Get epoch metadata
    /// @param epoch The epoch number to query
    /// @return EpochInfo struct with start/end blocks and finalized status
    function getEpochInfo(uint256 epoch) external view returns (EpochInfo memory) {
        return _epochs[epoch];
    }

    /// @notice Get the validator set snapshot for a finalized epoch
    /// @param epoch The epoch number to query
    /// @return validators Array of validator addresses
    /// @return votingPowers Array of corresponding voting powers
    function getValidatorSetForEpoch(uint256 epoch)
        external
        view
        returns (address[] memory validators, uint256[] memory votingPowers)
    {
        if (!_epochs[epoch].isFinalized) revert EpochNotFinalized(epoch);
        return (_epochValidators[epoch], _epochVotingPowers[epoch]);
    }

    // ─── Governance ───────────────────────────────────────────────────────

    /// @notice Update factor weights (must sum to 10000)
    /// @param newWeights The new weight configuration
    function setFactorWeights(FactorWeights calldata newWeights)
        external
        onlyRole(REPUTATION_ADMIN_ROLE)
    {
        uint256 sum = uint256(newWeights.stake) + newWeights.attest + newWeights.activity
            + newWeights.verify + newWeights.tenure;
        if (sum != MAX_BPS) revert InvalidWeightSum(sum);

        weights = newWeights;
        emit WeightsUpdated(newWeights);
    }

    /// @notice Update the minimum reputation threshold for validator eligibility
    /// @param bps New threshold in basis points
    function setMinReputationThreshold(uint256 bps) external onlyRole(REPUTATION_ADMIN_ROLE) {
        minReputationBps = bps;
    }

    /// @notice Update the maximum number of validators
    /// @param max New maximum validator count
    function setMaxValidators(uint256 max) external onlyRole(REPUTATION_ADMIN_ROLE) {
        maxValidators = max;
    }

    /// @notice Set the identity registry used to gate validator registration (Sybil resistance).
    /// @dev address(0) disables the gate (backward compatible).
    function setIdentityRegistry(address registry) external onlyRole(REPUTATION_ADMIN_ROLE) {
        identityRegistry = IIdentityRegistry(registry);
        emit IdentityRegistryUpdated(registry);
    }

    // ─── Stake-factor on-chain derivation (decentralization) ───────────────
    // When a stake source is configured, factor 0 (stake) is derived from real
    // staked balance deterministically, and the stake oracle can no longer set it.

    /// @notice Recompute an agent's stake factor from its on-chain staked balance.
    /// @dev Callable by anyone (deterministic). factor = min(staked * MAX_BPS / target, MAX_BPS).
    function deriveStakeFactor(address agent) external {
        if (address(stakeSource) == address(0)) revert StakeDerivationNotConfigured();

        uint256 balance = stakeSource.stakedBalanceOf(agent);
        uint256 factor = stakeFactorTarget == 0
            ? (balance == 0 ? 0 : MAX_BPS)
            : (balance * MAX_BPS) / stakeFactorTarget;
        if (factor > MAX_BPS) factor = MAX_BPS;

        _setFactorScore(agent, 0, uint16(factor));
        _recompute(agent);

        emit FactorScoreUpdated(agent, 0, uint16(factor));
        emit CompositeScoreUpdated(agent, _entries[agent].compositeScore);
        emit VotingPowerUpdated(agent, _entries[agent].votingPower);
        emit StakeFactorRecomputed(agent, uint16(factor));
    }

    /// @notice Set the stake source used to derive the stake factor (NeunodeToken satisfies IStakeSource).
    function setStakeSource(address source) external onlyRole(REPUTATION_ADMIN_ROLE) {
        stakeSource = IStakeSource(source);
        emit StakeSourceUpdated(source);
    }

    /// @notice Set the staked balance at which the stake factor reaches 100%.
    function setStakeFactorTarget(uint256 target) external onlyRole(REPUTATION_ADMIN_ROLE) {
        stakeFactorTarget = target;
        emit StakeFactorTargetUpdated(target);
    }

    // ─── Slashing Integration ─────────────────────────────────────────────

    /// @notice Apply a slashing penalty to a validator's reputation
    /// @param validator The validator address to penalize
    /// @param reputationSlashBps Basis points to subtract from composite score
    /// @param stakeSlashBps_ Basis points of stake to slash (reserved for future integration)
    function applyPenalty(address validator, uint256 reputationSlashBps, uint256 stakeSlashBps_)
        external
        onlyRole(SLASHING_ROLE)
    {
        // stakeSlashBps_ reserved for future cross-contract integration with NeunodeToken.slashStake()
        if (stakeSlashBps_ > MAX_BPS) stakeSlashBps_ = 0;

        ReputationEntry storage entry = _entries[validator];

        entry.penaltyBps += reputationSlashBps;
        if (entry.penaltyBps > MAX_BPS) {
            entry.penaltyBps = MAX_BPS;
        }
        entry.penaltyEpoch = currentEpoch;

        _recompute(validator);

        emit PenaltyApplied(validator, reputationSlashBps);
        emit CompositeScoreUpdated(validator, entry.compositeScore);
        emit VotingPowerUpdated(validator, entry.votingPower);
    }

    /// @notice Get the effective penalty for a validator after decay
    /// @param validator The validator address
    /// @return Effective penalty in basis points after linear decay
    function getPenaltyDecay(address validator) external view returns (uint256) {
        ReputationEntry storage entry = _entries[validator];
        if (entry.penaltyEpoch == 0 || entry.penaltyBps == 0) return 0;

        uint256 elapsed = currentEpoch - entry.penaltyEpoch;
        if (elapsed >= PENALTY_DECAY_EPOCHS) return 0;

        // Linear decay: effectivePenalty = penalty * (90 - elapsed) / 90
        return (entry.penaltyBps * (PENALTY_DECAY_EPOCHS - elapsed)) / PENALTY_DECAY_EPOCHS;
    }

    // ─── Internal ─────────────────────────────────────────────────────────

    /// @notice Set a single factor score for an agent, creating the entry if needed
    function _setFactorScore(address agent, uint8 factorIndex, uint16 scoreBps) internal {
        ReputationEntry storage entry = _entries[agent];

        if (factorIndex == 0) {
            entry.scores.stake = scoreBps;
        } else if (factorIndex == 1) {
            entry.scores.attest = scoreBps;
        } else if (factorIndex == 2) {
            entry.scores.activity = scoreBps;
        } else if (factorIndex == 3) {
            entry.scores.verify = scoreBps;
        } else {
            entry.scores.tenure = scoreBps;
        }
    }

    /// @notice Recompute composite score and voting power for an agent
    function _recompute(address agent) internal {
        ReputationEntry storage entry = _entries[agent];

        // Weighted sum of factor scores
        uint256 composite =
            (uint256(entry.scores.stake)
                    * weights.stake
                    + uint256(entry.scores.attest)
                    * weights.attest
                    + uint256(entry.scores.activity)
                    * weights.activity
                    + uint256(entry.scores.verify)
                    * weights.verify
                    + uint256(entry.scores.tenure)
                    * weights.tenure) / MAX_BPS;

        // Subtract effective penalty with decay
        uint256 effectivePenalty = _effectivePenalty(entry);
        if (composite > effectivePenalty) {
            composite -= effectivePenalty;
        } else {
            composite = 0;
        }

        // Cap at MAX_BPS
        if (composite > MAX_BPS) {
            composite = MAX_BPS;
        }

        entry.compositeScore = composite;
        entry.votingPower = _computeVotingPower(composite);
        entry.lastUpdateEpoch = currentEpoch;
    }

    /// @notice Compute the effective penalty with linear decay
    function _effectivePenalty(ReputationEntry storage entry) internal view returns (uint256) {
        if (entry.penaltyEpoch == 0 || entry.penaltyBps == 0) return 0;

        uint256 elapsed = currentEpoch - entry.penaltyEpoch;
        if (elapsed >= PENALTY_DECAY_EPOCHS) return 0;

        return (entry.penaltyBps * (PENALTY_DECAY_EPOCHS - elapsed)) / PENALTY_DECAY_EPOCHS;
    }

    /// @notice Convert composite score to voting power using sqrt mapping
    /// @dev VP = sqrt(compositeScore) * VOTING_POWER_SCALE / 100
    function _computeVotingPower(uint256 compositeScore) internal pure returns (uint256) {
        if (compositeScore == 0) return 0;
        uint256 sqrtScore = _sqrt(compositeScore);
        return (sqrtScore * VOTING_POWER_SCALE) / 100;
    }

    /// @dev Integer square root using the Babylonian method
    function _sqrt(uint256 x) internal pure returns (uint256 y) {
        if (x == 0) return 0;
        if (x <= 3) return 1;
        uint256 z = x;
        y = x / 2 + 1;
        while (y < z) {
            z = y;
            y = (x / z + z) / 2;
        }
    }
}
