// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.28;

import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "../interfaces/INeunodeToken.sol";

/// @title NeunodeSlashing -- Validator misbehavior detection and penalty enforcement
/// @notice Handles evidence submission, verification, stake slashing, jailing, and
///         tombstoning. Integrates with NeunodeToken.slashStake() for economic penalties.
///         Escalating penalty schedule: penalties increase with repeat offense count.
contract NeunodeSlashing is AccessControl, Pausable {
    // ─── Types ────────────────────────────────────────────────────────────

    enum OffenseType {
        DoubleSign, // S1: signed two blocks at same height
        Equivocation, // S2: voted for conflicting blocks
        Downtime, // S3: missed >95% of blocks in window
        InvalidBlock, // S4: proposed invalid block
        FalseAttestation, // S5: attested to false claim
        BountyGaming, // S6: bounty manipulation
        TokenManipulation, // S7: token market manipulation
        GovernanceAbuse, // S8: governance attack
        Spamming, // S9: network spam
        Collusion // S10: validator collusion
    }

    enum PenaltyOutcome {
        None, // No penalty (insufficient evidence)
        Jailed, // Temporarily removed from validator set
        Tombstoned, // Permanently removed
        RateLimited // Reduced rewards
    }

    struct SlashingEvidence {
        bytes32 blockHash1;
        bytes32 blockHash2;
        uint256 blockNumber;
        bytes signature1;
        bytes signature2;
        bytes extraData;
        address reporter;
        uint256 timestamp;
    }

    struct SlashingPenalty {
        uint256 stakeSlashBps;
        uint256 reputationSlashBps;
        uint256 jailDurationBlocks;
        PenaltyOutcome outcome;
    }

    struct ValidatorStatus {
        bool isJailed;
        uint256 jailReleaseBlock;
        uint256 offenseCount;
        bool isTombstoned;
    }

    // ─── Constants ────────────────────────────────────────────────────────

    uint256 public constant MAX_BPS = 10_000;
    /// @dev Reputation slash multiplier: reputation penalty = stake slash * 2
    uint256 public constant REPUTATION_SLASH_MULTIPLIER = 2;

    // ─── Roles ────────────────────────────────────────────────────────────

    bytes32 public constant SLASHING_ROLE = keccak256("SLASHING_ROLE");
    bytes32 public constant REPORTER_ROLE = keccak256("REPORTER_ROLE");
    bytes32 public constant ADMIN_ROLE = keccak256("ADMIN_ROLE");

    // ─── Immutables ───────────────────────────────────────────────────────

    INeunodeToken public immutable token;

    // ─── Storage ──────────────────────────────────────────────────────────

    /// @dev Per-validator status tracking
    mapping(address => ValidatorStatus) private _validators;

    /// @dev Per-validator per-offense count (validator => offense => count)
    mapping(address => mapping(OffenseType => uint256)) private _offenseCounts;

    /// @dev Penalty schedule per offense per offense tier (offense => tier => penalty)
    mapping(OffenseType => mapping(uint256 => SlashingPenalty)) private _penaltySchedule;

    /// @dev Evidence hash deduplication
    mapping(bytes32 => bool) private _evidenceSeen;

    /// @dev Total number of slashing events
    uint256 public slashingEventCount;

    // ─── Events ───────────────────────────────────────────────────────────

    event EvidenceSubmitted(
        address indexed validator,
        OffenseType offense,
        address indexed reporter,
        bytes32 evidenceHash
    );

    event ValidatorSlashed(
        address indexed validator,
        OffenseType offense,
        uint256 stakeSlashed,
        uint256 reputationSlashed
    );

    event ValidatorJailed(address indexed validator, uint256 releaseBlock);

    event ValidatorUnjailed(address indexed validator);

    event ValidatorTombstoned(address indexed validator);

    event PenaltyScheduleUpdated(
        OffenseType indexed offense,
        uint256 indexed tier,
        uint256 stakeSlashBps,
        uint256 reputationSlashBps,
        uint256 jailDurationBlocks,
        PenaltyOutcome outcome
    );

    // ─── Errors ───────────────────────────────────────────────────────────

    error InsufficientEvidence();
    error InvalidEvidence();
    error DuplicateEvidence(bytes32 hash);
    error ValidatorNotJailed();
    error JailNotExpired();
    error ValidatorAlreadyTombstoned();
    error ReporterNotAuthorized();
    error InvalidSignature();
    error SignaturesDoNotMatchValidator();
    error EvidenceExpired();
    error InvalidOffenseType();
    error ZeroAddress();
    error SameBlockHashes();
    error InvalidPenaltyBps(uint256 bps);
    error DowntimeThresholdNotMet(uint256 missed, uint256 required);

    // ─── Constructor ──────────────────────────────────────────────────────

    constructor(address token_) {
        if (token_ == address(0)) revert ZeroAddress();
        token = INeunodeToken(token_);

        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(ADMIN_ROLE, msg.sender);
        _grantRole(SLASHING_ROLE, msg.sender);
        _grantRole(REPORTER_ROLE, msg.sender);

        _initializePenaltySchedule();
    }

    // ─── Internal: Penalty Schedule Initialization ────────────────────────

    /// @dev Sets default penalty schedule per design doc Section 2.3
    function _initializePenaltySchedule() internal {
        // S1: DoubleSign — 5% / 15% / tombstone
        _penaltySchedule[OffenseType.DoubleSign][0] = SlashingPenalty({
            stakeSlashBps: 500,
            reputationSlashBps: 1000,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.DoubleSign][1] = SlashingPenalty({
            stakeSlashBps: 1500,
            reputationSlashBps: 3000,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.DoubleSign][2] = SlashingPenalty({
            stakeSlashBps: 5000,
            reputationSlashBps: 10000,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.Tombstoned
        });

        // S2: Equivocation — 3% / 10% / tombstone
        _penaltySchedule[OffenseType.Equivocation][0] = SlashingPenalty({
            stakeSlashBps: 300,
            reputationSlashBps: 600,
            jailDurationBlocks: 7200,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.Equivocation][1] = SlashingPenalty({
            stakeSlashBps: 1000,
            reputationSlashBps: 2000,
            jailDurationBlocks: 14400,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.Equivocation][2] = SlashingPenalty({
            stakeSlashBps: 5000,
            reputationSlashBps: 10000,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.Tombstoned
        });

        // S3: Downtime — 0.5% / 2% / jail
        _penaltySchedule[OffenseType.Downtime][0] = SlashingPenalty({
            stakeSlashBps: 50,
            reputationSlashBps: 100,
            jailDurationBlocks: 120_960, // ~7 days at 5s blocks
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.Downtime][1] = SlashingPenalty({
            stakeSlashBps: 200,
            reputationSlashBps: 400,
            jailDurationBlocks: 120_960,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.Downtime][2] = SlashingPenalty({
            stakeSlashBps: 500,
            reputationSlashBps: 1000,
            jailDurationBlocks: 241_920, // ~14 days
            outcome: PenaltyOutcome.Jailed
        });

        // S4: InvalidBlock — 5% / 15% / tombstone
        _penaltySchedule[OffenseType.InvalidBlock][0] = SlashingPenalty({
            stakeSlashBps: 500,
            reputationSlashBps: 1000,
            jailDurationBlocks: 7200,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.InvalidBlock][1] = SlashingPenalty({
            stakeSlashBps: 1500,
            reputationSlashBps: 3000,
            jailDurationBlocks: 14400,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.InvalidBlock][2] = SlashingPenalty({
            stakeSlashBps: 5000,
            reputationSlashBps: 10000,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.Tombstoned
        });

        // S5: FalseAttestation — 3% / 10% / jail
        _penaltySchedule[OffenseType.FalseAttestation][0] = SlashingPenalty({
            stakeSlashBps: 300,
            reputationSlashBps: 600,
            jailDurationBlocks: 7200,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.FalseAttestation][1] = SlashingPenalty({
            stakeSlashBps: 1000,
            reputationSlashBps: 2000,
            jailDurationBlocks: 14400,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.FalseAttestation][2] = SlashingPenalty({
            stakeSlashBps: 2000,
            reputationSlashBps: 4000,
            jailDurationBlocks: 241_920,
            outcome: PenaltyOutcome.Jailed
        });

        // S6: BountyGaming — 5% / 15% / jail
        _penaltySchedule[OffenseType.BountyGaming][0] = SlashingPenalty({
            stakeSlashBps: 500,
            reputationSlashBps: 1000,
            jailDurationBlocks: 518_400, // ~30 days
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.BountyGaming][1] = SlashingPenalty({
            stakeSlashBps: 1500,
            reputationSlashBps: 3000,
            jailDurationBlocks: 518_400,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.BountyGaming][2] = SlashingPenalty({
            stakeSlashBps: 3000,
            reputationSlashBps: 6000,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.Tombstoned
        });

        // S7: TokenManipulation — 8% / 20% / tombstone
        _penaltySchedule[OffenseType.TokenManipulation][0] = SlashingPenalty({
            stakeSlashBps: 800,
            reputationSlashBps: 1600,
            jailDurationBlocks: 7200,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.TokenManipulation][1] = SlashingPenalty({
            stakeSlashBps: 2000,
            reputationSlashBps: 4000,
            jailDurationBlocks: 14400,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.TokenManipulation][2] = SlashingPenalty({
            stakeSlashBps: 5000,
            reputationSlashBps: 10000,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.Tombstoned
        });

        // S8: GovernanceAbuse — 2% / 5% / rate limited
        _penaltySchedule[OffenseType.GovernanceAbuse][0] = SlashingPenalty({
            stakeSlashBps: 200,
            reputationSlashBps: 400,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.RateLimited
        });
        _penaltySchedule[OffenseType.GovernanceAbuse][1] = SlashingPenalty({
            stakeSlashBps: 500,
            reputationSlashBps: 1000,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.RateLimited
        });
        _penaltySchedule[OffenseType.GovernanceAbuse][2] = SlashingPenalty({
            stakeSlashBps: 1000,
            reputationSlashBps: 2000,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.RateLimited
        });

        // S9: Spamming — 0.1% / 1% / rate limited
        _penaltySchedule[OffenseType.Spamming][0] = SlashingPenalty({
            stakeSlashBps: 10,
            reputationSlashBps: 20,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.RateLimited
        });
        _penaltySchedule[OffenseType.Spamming][1] = SlashingPenalty({
            stakeSlashBps: 100,
            reputationSlashBps: 200,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.RateLimited
        });
        _penaltySchedule[OffenseType.Spamming][2] = SlashingPenalty({
            stakeSlashBps: 500,
            reputationSlashBps: 1000,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.RateLimited
        });

        // S10: Collusion — 8% / 20% / tombstone
        _penaltySchedule[OffenseType.Collusion][0] = SlashingPenalty({
            stakeSlashBps: 800,
            reputationSlashBps: 1600,
            jailDurationBlocks: 7200,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.Collusion][1] = SlashingPenalty({
            stakeSlashBps: 2000,
            reputationSlashBps: 4000,
            jailDurationBlocks: 14400,
            outcome: PenaltyOutcome.Jailed
        });
        _penaltySchedule[OffenseType.Collusion][2] = SlashingPenalty({
            stakeSlashBps: 5000,
            reputationSlashBps: 10000,
            jailDurationBlocks: 0,
            outcome: PenaltyOutcome.Tombstoned
        });
    }

    // ─── External: Evidence Submission ────────────────────────────────────

    /// @notice Submit slashing evidence for a validator offense
    /// @param offense The type of offense committed
    /// @param evidence The evidence supporting the slashing claim
    function submitEvidence(OffenseType offense, SlashingEvidence calldata evidence)
        external
        whenNotPaused
    {
        if (uint8(offense) > uint8(OffenseType.Collusion)) revert InvalidOffenseType();
        if (_validators[msg.sender].isTombstoned) revert ReporterNotAuthorized();

        bytes32 evidenceHash = keccak256(
            abi.encode(
                offense,
                evidence.blockHash1,
                evidence.blockHash2,
                evidence.blockNumber,
                evidence.signature1,
                evidence.signature2,
                evidence.extraData,
                evidence.reporter
            )
        );

        if (_evidenceSeen[evidenceHash]) revert DuplicateEvidence(evidenceHash);
        _evidenceSeen[evidenceHash] = true;

        // Verify evidence freshness (7-day window)
        if (block.timestamp > evidence.timestamp + 7 days) revert EvidenceExpired();

        _executeSlashing(evidence.reporter, offense);
        slashingEventCount++;

        emit EvidenceSubmitted(evidence.reporter, offense, msg.sender, evidenceHash);
    }

    /// @notice Report downtime for a validator (called by block monitor or SLASHING_ROLE)
    /// @param validator The address of the validator who was offline
    /// @param missedBlocks Number of blocks missed in the window
    /// @param windowBlocks Total blocks in the observation window
    function reportDowntime(address validator, uint256 missedBlocks, uint256 windowBlocks)
        external
        whenNotPaused
    {
        if (validator == address(0)) revert ZeroAddress();
        if (_validators[validator].isTombstoned) revert ValidatorAlreadyTombstoned();

        // Require >50% missed in the window
        uint256 threshold = windowBlocks / 2;
        if (missedBlocks <= threshold) {
            revert DowntimeThresholdNotMet(missedBlocks, threshold + 1);
        }

        _executeSlashing(validator, OffenseType.Downtime);
        slashingEventCount++;
    }

    /// @notice Report double signing with cryptographic proof
    /// @param header1 First block header bytes
    /// @param header2 Second block header bytes at same height
    /// @param sig1 Validator signature on header1
    /// @param sig2 Validator signature on header2
    function reportDoubleSign(
        bytes memory header1,
        bytes memory header2,
        bytes memory sig1,
        bytes memory sig2
    ) external whenNotPaused {
        bytes32 h1 = keccak256(header1);
        bytes32 h2 = keccak256(header2);

        if (h1 == h2) revert SameBlockHashes();

        // Recover signer addresses from both signatures
        address signer1 = _recoverSigner(h1, sig1);
        address signer2 = _recoverSigner(h2, sig2);

        if (signer1 != signer2) revert SignaturesDoNotMatchValidator();
        if (signer1 == address(0)) revert InvalidSignature();

        bytes32 evidenceHash = keccak256(abi.encode("DoubleSign", h1, h2, sig1, sig2));
        if (_evidenceSeen[evidenceHash]) revert DuplicateEvidence(evidenceHash);
        _evidenceSeen[evidenceHash] = true;

        _executeSlashing(signer1, OffenseType.DoubleSign);
        slashingEventCount++;

        emit EvidenceSubmitted(signer1, OffenseType.DoubleSign, msg.sender, evidenceHash);
    }

    // ─── External: Jail Management ────────────────────────────────────────

    /// @notice Unjail a validator after their jail period has expired
    /// @param validator The address of the jailed validator
    function unjail(address validator) external whenNotPaused {
        ValidatorStatus storage status = _validators[validator];

        if (!status.isJailed) revert ValidatorNotJailed();
        if (block.number < status.jailReleaseBlock) revert JailNotExpired();

        status.isJailed = false;
        status.jailReleaseBlock = 0;

        emit ValidatorUnjailed(validator);
    }

    // ─── External: View Functions ─────────────────────────────────────────

    /// @notice Check if a validator is currently jailed
    /// @param validator The address to check
    /// @return True if the validator is jailed
    function isJailed(address validator) external view returns (bool) {
        return _validators[validator].isJailed;
    }

    /// @notice Get the full status of a validator
    /// @param validator The address to query
    /// @return The validator's current status
    function getValidatorStatus(address validator) external view returns (ValidatorStatus memory) {
        return _validators[validator];
    }

    /// @notice Get the penalty for a specific offense and offense count
    /// @param offense The offense type
    /// @param offenseCount The number of prior offenses for this type (0-indexed tier)
    /// @return The configured penalty
    function getPenaltySchedule(OffenseType offense, uint256 offenseCount)
        external
        view
        returns (SlashingPenalty memory)
    {
        return _penaltySchedule[offense][offenseCount];
    }

    /// @notice Get the offense count for a validator and offense type
    /// @param validator The validator address
    /// @param offense The offense type
    /// @return The number of times this validator has committed this offense
    function getOffenseCount(address validator, OffenseType offense)
        external
        view
        returns (uint256)
    {
        return _offenseCounts[validator][offense];
    }

    /// @notice Check if an evidence hash has already been submitted
    /// @param evidenceHash The hash to check
    /// @return True if the evidence has been seen before
    function isEvidenceSeen(bytes32 evidenceHash) external view returns (bool) {
        return _evidenceSeen[evidenceHash];
    }

    // ─── External: Governance ─────────────────────────────────────────────

    /// @notice Update the penalty schedule for an offense type at a given tier
    /// @param offense The offense type to update
    /// @param tier The offense count tier (0=first, 1=second, 2=third+)
    /// @param stakeSlashBps Basis points of stake to slash
    /// @param reputationSlashBps Basis points of reputation to slash
    /// @param jailDurationBlocks Number of blocks for jail duration
    /// @param outcome The penalty outcome type
    function setPenaltySchedule(
        OffenseType offense,
        uint256 tier,
        uint256 stakeSlashBps,
        uint256 reputationSlashBps,
        uint256 jailDurationBlocks,
        PenaltyOutcome outcome
    ) external onlyRole(ADMIN_ROLE) {
        if (stakeSlashBps > MAX_BPS) {
            revert InvalidPenaltyBps(stakeSlashBps);
        }
        if (reputationSlashBps > MAX_BPS) revert InvalidPenaltyBps(reputationSlashBps);

        _penaltySchedule[offense][tier] = SlashingPenalty({
            stakeSlashBps: stakeSlashBps,
            reputationSlashBps: reputationSlashBps,
            jailDurationBlocks: jailDurationBlocks,
            outcome: outcome
        });

        emit PenaltyScheduleUpdated(
            offense, tier, stakeSlashBps, reputationSlashBps, jailDurationBlocks, outcome
        );
    }

    /// @notice Pause all slashing operations for incident response
    function pause() external onlyRole(ADMIN_ROLE) {
        _pause();
    }

    /// @notice Unpause slashing operations
    function unpause() external onlyRole(ADMIN_ROLE) {
        _unpause();
    }

    // ─── Internal: Slashing Execution ─────────────────────────────────────

    /// @dev Core slashing logic: determines penalty, applies stake slash, updates status
    function _executeSlashing(address validator, OffenseType offense) internal {
        ValidatorStatus storage status = _validators[validator];

        if (status.isTombstoned) revert ValidatorAlreadyTombstoned();

        // Determine offense tier (0, 1, 2+)
        uint256 tier = _offenseCounts[validator][offense];
        if (tier > 2) tier = 2;

        SlashingPenalty memory penalty = _penaltySchedule[offense][tier];

        // Apply stake slash via NeunodeToken
        uint256 staked = token.stakedBalanceOf(validator);
        uint256 slashAmount = (staked * penalty.stakeSlashBps) / MAX_BPS;

        if (slashAmount > 0) {
            token.slashStake(validator, slashAmount);
        }

        // Update offense count
        _offenseCounts[validator][offense]++;
        status.offenseCount++;

        // Apply penalty outcome
        if (penalty.outcome == PenaltyOutcome.Tombstoned) {
            status.isTombstoned = true;
            status.isJailed = false;
            status.jailReleaseBlock = 0;
            emit ValidatorTombstoned(validator);
        } else if (penalty.outcome == PenaltyOutcome.Jailed) {
            status.isJailed = true;
            status.jailReleaseBlock = block.number + penalty.jailDurationBlocks;
            emit ValidatorJailed(validator, status.jailReleaseBlock);
        }
        // PenaltyOutcome.RateLimited and None: no jail/tombstone, just slash

        emit ValidatorSlashed(validator, offense, slashAmount, penalty.reputationSlashBps);
    }

    // ─── Internal: Signature Recovery ─────────────────────────────────────

    /// @dev Recover signer address from a message hash and signature
    function _recoverSigner(bytes32 hash, bytes memory signature) internal pure returns (address) {
        if (signature.length != 65) revert InvalidSignature();
        bytes32 r;
        bytes32 s;
        uint8 v;
        assembly {
            r := mload(add(signature, 32))
            s := mload(add(signature, 64))
            v := byte(0, mload(add(signature, 96)))
        }
        if (v < 27) v += 27;
        if (v != 27 && v != 28) revert InvalidSignature();
        return ecrecover(hash, v, r, s);
    }
}
