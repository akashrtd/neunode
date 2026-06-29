# Spike: Slashing Mechanism & Reputation-to-Voting-Power Mapping

**Date:** 2026-05-29
**Status:** Design Spike
**Issues:** GitHub #34 (Reputation-to-Voting-Power), GitHub #36 (Slashing Mechanism)
**Blocks:** Reputation-weighted validation system (P0)

---

## Table of Contents

1. [Background & Existing System](#1-background--existing-system)
2. [Task 1: Slashing Mechanism Design](#2-task-1-slashing-mechanism-design)
3. [Task 2: Reputation-to-Voting-Power Mapping](#3-task-2-reputation-to-voting-power-mapping)
4. [Task 3: On-Chain Reputation Contract](#4-task-3-on-chain-reputation-contract)
5. [Integration Points](#5-integration-points)
6. [Open Questions](#6-open-questions)

---

## 1. Background & Existing System

### Current Reputation Model (Rust)

The 5-factor reputation system lives in `neunode-reputation` crate:

| Factor | Weight | Source | Computation |
|--------|--------|--------|-------------|
| Stake | 30% | `compute_stake_factor` | `staked_amount / total_staked * 100` |
| Attestation | 25% | `compute_attestation_factor` | `ln(1 + count) / ln(11) * 100 * (avg_score / 100)` |
| Activity | 20% | `compute_activity_factor` | `frequency/50 * 50 + ln(1 + days) / ln(366) * 50` |
| Verification | 15% | `compute_verify_factor` | `success_rate * 90 + volume_bonus` |
| Tenure | 10% | `compute_tenure_factor` | `ln(1 + days) / ln(366) * 100` |

Composite score: `weighted sum, capped at 100.0`. Grades: A (90+), B (75+), C (50+), D (25+), F (0-24).

### Current On-Chain Infrastructure

| Contract | Location | Relevance |
|----------|----------|-----------|
| `NeunodeIdentity` | `contracts/src/NeunodeIdentity.sol` | DID registry, agent identity |
| `NeunodeGovernance` | `contracts/src/governance/NeunodeGovernance.sol` | Staked-token voting, checkpoint system |
| `NeunodeToken` (base) | `contracts/src/tokens/NeunodeToken.sol` | ERC-20 with staking, `slashStake()`, activity tracking |
| `StakingEscrow` | `contracts/src/escrow/StakingEscrow.sol` | Inactivity decay (5 tiers: Active/Moderate/Low/Inactive/Dead) |
| `BountyReview` | `contracts/src/bounty/BountyReview.sol` | 2-of-3 review committees, EIP-712 signatures |
| Diamond proxy | `contracts/src/diamond/` | EIP-2535 upgradeable facets |

### Key Existing Mechanisms

**Stake slashing (already exists):** `NeunodeToken.slashStake(address, amount)` burns staked tokens, restricted to `GOVERNANCE_ROLE`. Seed tokens are protected from slashing.

**Inactivity decay (already exists):** `StakingEscrow.executeDecay()` applies daily decay based on activity level (0-137 bps/day). Decay destination: burn.

**Dispute system (Rust):** `InferenceDispute` has a 1-hour challenge window with evidence hash and resolution flow.

---

## 2. Task 1: Slashing Mechanism Design (GitHub #36)

### 2.1 Research: Cosmos/CometBFT Slashing Patterns

**Cosmos SDK slashing module** defines two offense categories:

1. **Double signing (equivocation):** A validator signs two different blocks at the same height/round. This is cryptographically provable from the two conflicting signatures. Penalty: typically 5% of stake on first offense. Permanent tombstoning (cannot rejoin).

2. **Liveness (downtime):** A validator misses more than X% of blocks in a sliding window (e.g., >50% in 10,000 blocks). Penalty: typically 0.01% of stake. Temporary jailing with unjail after downtime period.

**Key Cosmos parameters:**
- `signed_blocks_window`: sliding window size (default: 100 blocks)
- `min_signed_per_window`: minimum participation (default: 50%)
- `downtime_jail_duration`: jail time for liveness offenses (default: 60s for testnet, longer for mainnet)
- `slash_fraction_double_sign`: percentage slashed (default: 5%)
- `slash_fraction_downtime`: percentage slashed (default: 0.01%)
- `unbonding_time`: unbonding period (default: 21 days for Cosmos Hub)

**Ethereum 2.0 slashing** is more aggressive:
- Minimum penalty: 1 ETH (or 1/32 of effective balance)
- Correlation penalty: up to full stake if many validators slashed simultaneously
- Permanent ejection from validator set
- whistleblower reward: ~1/512 of slashed amount

### 2.2 Neunode-Specific Offenses

Neunode is not a traditional PoS chain -- it is a reputation-weighted network for AI agents. Offenses must reflect the agent domain.

#### Offense Taxonomy

| ID | Offense | Severity | Category | Evidence Required |
|----|---------|----------|----------|-------------------|
| S1 | Double signing (equivocation) | Critical | Consensus | Two signed blocks at same height with different hashes |
| S2 | Consensus downtime | High | Consensus | Missed >50% of blocks in sliding window |
| S3 | Malicious inference result | Critical | Agent behavior | Inference output hash + ground truth hash, signed by 2-of-3 verification committee |
| S4 | Bounty fraud (plagiarism/fake work) | High | Agent behavior | Evidence hash + 2-of-3 review committee rejection with fraud flag |
| S5 | Collusion (circular reputation) | High | Agent behavior | Graph analysis proof (cycle detection with threshold), attestation graph snapshot |
| S6 | Data poisoning (knowledge graph) | Medium | Agent behavior | Affected triples + verification hash, signed by oracle |
| S7 | False slashing accusation | Medium | Governance | Accusation record + counter-evidence hash |
| S8 | Unauthorized key usage | High | Security | Signature from rotated key after deactivation |
| S9 | Oracle manipulation | Critical | Agent behavior | Conflicting oracle outputs + timestamp proof |
| S10 | Spam/DDoS (resource abuse) | Low | Network | Rate counter snapshot, signed by relays |

### 2.3 Penalty Schedule

**Design principle:** Penalties scale with offense severity and repeat count. Reputation damage is separate from and additional to stake slashing.

```
Slashing percentages (of staked tokens):

                    First     Second    Third+
Consensus offenses:
  S1 Double sign     5%       15%      Tombstone
  S2 Downtime        0.5%     2%       Jail (7d)

Agent behavior:
  S3 Malicious inf   10%      30%      Tombstone
  S4 Bounty fraud    5%       15%      Jail (30d)
  S5 Collusion       8%       20%      Tombstone
  S6 Data poisoning  3%       10%      Jail (14d)
  S9 Oracle manip    10%      30%      Tombstone

Governance:
  S7 False accuse    2%       5%       Dispute ban (90d)
  S8 Unauthorized    5%       15%      Key freeze
  S10 Spam           0.1%     1%       Rate limit downgrade
```

**Tombstone:** Permanent removal from validator set. Agent can still participate in the network as a non-validator but can never again become a validator with that DID.

**Jail:** Temporary removal from validator set. Agent cannot sign blocks or earn staking rewards during jail period. Can request unjail after duration expires.

### 2.4 Reputation Impact

Slashing also reduces the reputation score directly:

```
Reputation penalty = slash_percentage * REPUTATION_SLASH_MULTIPLIER

Where REPUTATION_SLASH_MULTIPLIER = 2.0 (configurable by governance)

Example: 5% stake slash => 10 reputation point reduction
         A validator at 85 reputation drops to 75
```

This is implemented as a separate `reputation_penalty` field that is subtracted from the computed composite score before converting to voting power. Penalties decay linearly over 90 days (recoverable through good behavior).

### 2.5 Evidence Format

Each slashing event requires on-chain evidence:

```solidity
struct SlashingEvidence {
    bytes32 didHash;           // Agent DID being accused
    uint8 offenseId;           // Offense ID (S1-S10)
    bytes32 evidenceHash;      // IPFS/content hash of full evidence
    bytes[] signatures;        // Supporting signatures (committee, oracles)
    uint256 blockNumber;       // Block where offense occurred
    uint256 timestamp;         // Timestamp of offense
    bytes32[] relatedTxHashes; // Related transaction hashes
}
```

**Verification rules by offense:**

| Offense | Verifier | Min Signatures | Window |
|---------|----------|---------------|--------|
| S1 Double sign | Anyone (cryptographic proof) | 0 (self-evident from two blocks) | Unlimited |
| S2 Downtime | Automatic (on-chain counter) | 0 (computed from missed blocks) | Rolling window |
| S3 Malicious inference | Verification committee | 2-of-3 reviewers | 24h after settlement |
| S4 Bounty fraud | Review committee | 2-of-3 reviewers | During review period |
| S5 Collusion | Oracle + governance vote | Oracle signature + governance approval | 30d |
| S6 Data poisoning | Oracle | 1 oracle signature | 14d |
| S7 False accusation | Arbitration panel | 3-of-5 jurors | 90d |
| S8 Unauthorized | Automatic (key mismatch) | 0 (self-evident) | Unlimited |
| S9 Oracle manipulation | 2-of-3 backup oracles | 2 oracle signatures | 7d |
| S10 Spam | Relay attestations | 3 relay signatures | 7d |

### 2.6 Slashing Contract Interface

```solidity
// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

interface ISlashing {
    // ─── Types ────────────────────────────────────────────────────

    enum Offense {
        DoubleSign,        // S1
        Downtime,          // S2
        MaliciousInference,// S3
        BountyFraud,       // S4
        Collusion,         // S5
        DataPoisoning,     // S6
        FalseAccusation,   // S7
        UnauthorizedKey,   // S8
        OracleManipulation,// S9
        Spam               // S10
    }

    enum ValidatorStatus {
        Active,
        Jailed,
        Tombstoned
    }

    struct SlashingEvent {
        bytes32 didHash;
        Offense offense;
        uint256 slashPercentage;  // basis points
        uint256 reputationPenalty;
        uint256 jailedUntil;
        ValidatorStatus newStatus;
        bytes32 evidenceHash;
        uint256 timestamp;
    }

    // ─── Events ───────────────────────────────────────────────────

    event ValidatorSlashed(
        bytes32 indexed didHash,
        Offense offense,
        uint256 stakeSlashed,
        uint256 reputationPenalty,
        ValidatorStatus newStatus
    );

    event ValidatorJailed(
        bytes32 indexed didHash,
        uint256 releaseTime,
        Offense offense
    );

    event ValidatorUnjailed(bytes32 indexed didHash);

    event ValidatorTombstoned(bytes32 indexed didHash, Offense offense);

    event SlashParametersUpdated(
        Offense indexed offense,
        uint256 firstSlashBps,
        uint256 secondSlashBps,
        uint256 thirdSlashBps,
        uint256 jailDuration
    );

    // ─── Functions ────────────────────────────────────────────────

    /// @notice Submit slashing evidence for an offense
    function submitEvidence(
        bytes32 didHash,
        Offense offense,
        bytes32 evidenceHash,
        bytes[] calldata supportingSignatures
    ) external;

    /// @notice Report downtime for a validator (called by block monitor)
    function reportDowntime(bytes32 didHash, uint256 missedBlocks, uint256 windowSize)
        external;

    /// @notice Report double signing with cryptographic proof
    function reportDoubleSign(
        bytes32 didHash,
        bytes32 blockHash1,
        bytes32 blockHash2,
        uint256 height,
        bytes calldata sig1,
        bytes calldata sig2
    ) external;

    /// @notice Unjail a validator after jail period expires
    function unjail(bytes32 didHash) external;

    /// @notice Get validator status
    function getValidatorStatus(bytes32 didHash)
        external view returns (ValidatorStatus status, uint256 jailedUntil, uint256 offenseCount);

    /// @notice Get slashing history for a validator
    function getSlashingHistory(bytes32 didHash)
        external view returns (SlashingEvent[] memory);

    // ─── Governance ──────────────────────────────────────────────

    /// @notice Update slashing parameters for an offense type
    function setSlashParameters(
        Offense offense,
        uint256 firstSlashBps,
        uint256 secondSlashBps,
        uint256 thirdSlashBps,
        uint256 jailDurationDays
    ) external;
}
```

### 2.7 Jail/Unjail Flow

```
1. Offense detected -> evidence submitted via submitEvidence()
2. Contract verifies:
   a. Evidence signatures valid
   b. Offense within reporting window
   c. Validator not already tombstoned
3. Offense count incremented for this validator + offense
4. Slash percentage determined from penalty schedule (escalating)
5. Stake slashed via existing NeunodeToken.slashStake()
6. Reputation penalty applied via NeunodeReputation.applyPenalty()
7. Validator status updated:
   - First/second offense + non-critical: Jailed (with release time)
   - Third offense or critical offense: Tombstoned
8. Slashed tokens distributed:
   - 40% treasury
   - 30% staking rewards pool
   - 20% burned
   - 10% whistleblower reward (if applicable)

Unjail:
1. Validator calls unjail() after jailedUntil timestamp
2. Contract verifies:
   a. jailedUntil has passed
   b. No pending slashing events
3. Status updated to Active
4. Reputation penalty begins decay (90-day linear recovery)
```

### 2.8 False Accusation Defense

**Problem:** Malicious actors could submit false slashing evidence to eliminate competitors.

**Defense layers:**

1. **Evidence bond:** Accuser must post a bond (e.g., 1% of accused's stake) to submit evidence. If evidence is proven false, bond is slashed and given to the accused.

2. **Committee verification:** For non-cryptographic offenses (S3-S7, S9-S10), evidence requires multi-party signatures from an assigned committee.

3. **Appeal window:** 48-hour appeal window after slashing. Accused can submit counter-evidence. If appeal succeeds:
   - Slash reversed (stake restored from insurance pool)
   - Reputation restored
   - Accuser's bond slashed

4. **Arbitration:** For disputed cases, 5-member arbitration panel (randomly selected from active validators with reputation > 80) votes. 3-of-5 required for final decision.

### 2.9 Unbonding Period

Borrowing from Cosmos's model but adapted for Neunode:

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Unbonding period | 21 days | Standard from Cosmos; long enough to detect offenses |
| Slashing during unbonding | Yes | If offense occurred while validator was active, slash applies even during unbonding |
| Rebonding | Allowed | Can cancel unbonding and return to active set |

Special case: If a validator is slashed, their unbonding period is extended by the jail duration. Tokens are not released until the jail period ends.

---

## 3. Task 2: Reputation-to-Voting-Power Mapping (GitHub #34)

### 3.1 Research: Voting Power in Consensus Systems

**Cosmos/CometBFT approach:**
- Voting power = raw token stake (linear mapping)
- Top N validators by stake become the active set (N = 180 for Cosmos Hub)
- Validators self-delegate and receive external delegations
- Total voting power = sum of all active validator stakes

**Ethereum 2.0 approach:**
- Each validator deposits exactly 32 ETH
- 1 validator = 1 unit of voting power (flat, not proportional to total stake)
- Effective balance capped at 32 ETH per validator
- Supports up to ~1M validators

**Polkadot/Nominated PoS approach:**
- Nominator-backed validators
- Phragmen's algorithm for fair selection
- Slashing applies to both validators and nominators proportionally

**Bittensor (Yuma Consensus):**
- Trust-weighted scoring
- Sub-linear mapping (trust scores, not raw stake)
- 18%/41%/41% incentive split

### 3.2 Mapping Function Analysis

Three candidate functions for converting reputation score (0-100) to voting power:

```
Input:  reputation score R ∈ [0, 100]
Output: voting power VP ∈ [1, MAX_VP]  (uint64)

Candidate A: Linear
  VP = floor(R / 100 * MAX_VP)
  - Simple, proportional, wealth-correlated

Candidate B: Square root (sub-linear)
  VP = floor(sqrt(R / 100) * MAX_VP)
  - Diminishing returns, reduces whale dominance
  - At R=100: VP = MAX_VP
  - At R=50:  VP = 0.707 * MAX_VP (not 0.5)

Candidate C: Logarithmic (heavily sub-linear)
  VP = floor(ln(1 + R) / ln(101) * MAX_VP)
  - Very flat, extreme diminishing returns
  - At R=100: VP = MAX_VP
  - At R=50:  VP = 0.87 * MAX_VP (very compressed)
```

**Recommendation: Square root mapping.**

Justification:
1. **Sybil resistance:** Linear mapping means 10 agents with R=10 each have the same power as 1 agent with R=100. Sqrt gives the single high-reputation agent 3.16x the per-agent power of the 10 low-reputation agents combined. This rewards genuine reputation building over identity multiplication.

2. **Whale dampening:** Without sqrt, a validator with 2x the reputation gets 2x the power. With sqrt, they get 1.41x. This prevents power concentration while still rewarding high performers.

3. **Not too flat:** Logarithmic is too compressed -- it would make R=20 and R=80 nearly equivalent. Square root maintains meaningful differentiation while preventing dominance.

4. **Precedent:** UNI governance uses sqrt for quadratic voting. Gitcoin uses sqrt for matching. The pattern is well-understood.

### 3.3 Score Normalization Algorithm

```
Reputation score R is a float in [0.0, 100.0]
Voting power VP is a uint64 in [0, 2^64 - 1]

Constants:
  MAX_VOTING_POWER = 10_000_000_000  (10 billion, fits in uint64 with room)
  MIN_VALIDATOR_REPUTATION = 50.0     // Grade C or above required
  VALIDATOR_SET_SIZE = 100            // Maximum active validators

Algorithm:

1. Filter: Collect all agents with R >= MIN_VALIDATOR_REPUTATION
2. Rank: Sort descending by R
3. Cap: Take top VALIDATOR_SET_SIZE agents
4. Map each agent's R to VP:

   VP_i = floor(sqrt(R_i / 100.0) * MAX_VOTING_POWER)

   If VP_i == 0, set VP_i = 1 (every validator gets at least 1 unit)

5. Total voting power = sum of all VP_i

   For CometBFT: each VP_i is expressed as proportion of total.
   CometBFT requires 2/3 majority, so no single validator should
   exceed 1/3 of total VP.
```

**Numerical examples:**

| Reputation (R) | sqrt(R/100) | VP (out of 10B) | % of max |
|----------------|-------------|-----------------|----------|
| 100 | 1.000 | 10,000,000,000 | 100% |
| 90 | 0.949 | 9,486,832,981 | 94.9% |
| 80 | 0.894 | 8,944,271,910 | 89.4% |
| 75 | 0.866 | 8,660,254,038 | 86.6% |
| 70 | 0.837 | 8,366,600,265 | 83.7% |
| 60 | 0.775 | 7,745,966,692 | 77.5% |
| 50 | 0.707 | 7,071,067,812 | 70.7% |

Notice that the sqrt function preserves meaningful separation: the gap between R=90 and R=50 is 24.2 percentage points of voting power, not the 40 points it would be with linear mapping.

### 3.4 Minimum Thresholds for Validator Eligibility

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Minimum reputation score | 50.0 (Grade C) | Below this, agent hasn't demonstrated enough competence for consensus participation |
| Minimum stake | 1,000 tokens (resource-specific) | Economic skin-in-the-game, already exists in `REVIEWER_MIN_STAKE` |
| Minimum tenure | 30 days | Prevents flash-in agents from immediately joining validator set |
| Minimum activity | Level 1+ (Moderate, <=7d since last) | Inactive agents shouldn't validate |
| Maximum validators | 100 | Balances decentralization with BFT communication overhead |

**Special rules:**
- New agents start at R=0 and must build reputation through contributions before qualifying
- Agents in jail or tombstoned status are excluded from the validator set
- Agents with active slashing appeals are temporarily excluded until appeal resolves

### 3.5 Epoch Update Protocol

**Why epochs?** Updating the validator set on every reputation change would cause:
- Constant validator set churn (unstable consensus)
- MEV opportunities around known upcoming changes
- Excessive on-chain state transitions

**Epoch parameters:**

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Epoch length | 720 blocks (~1 hour at 5s blocks) | Frequent enough to respond to changes, slow enough for stability |
| Transition window | 10 blocks at epoch boundary | Smooth handover, prevents gaps |
| Score snapshot | Taken 100 blocks before epoch end | Allows computation time before transition |
| Grace period | 1 epoch after joining | New validators don't sign immediately |

**Epoch transition flow:**

```
Block N-100:   Snapshot all reputation scores
Block N-90:    Compute new validator set (top 100 by reputation, min threshold)
Block N-80:    Publish validator set hash on-chain
Block N-10:    New validator set begins warming up (participating but not counting)
Block N:       Epoch transition. Old set stops, new set is active.
Block N+10:    Old validator set fully decommissioned.
```

### 3.6 Mid-Epoch Reputation Drop

If a validator's reputation drops below the minimum threshold mid-epoch:

1. **Soft exclusion:** The validator's blocks are still accepted, but they earn no rewards for the rest of the epoch.
2. **Next epoch:** The validator is excluded from the new validator set.
3. **Emergency removal:** If reputation drops below 25.0 (Grade D) due to a slashing event, the validator is immediately removed and the epoch is shortened. This requires governance approval for the emergency epoch change.

**Tie-breaking:**
- Ties in reputation score are broken by: (1) higher tenure factor, (2) higher stake factor, (3) earlier DID creation timestamp
- Deterministic tie-breaking prevents validator set disagreements between nodes

### 3.7 Malachite Context Implementation

Malachite is a Rust implementation of CometBFT consensus. Neunode's L1 needs a custom `Context` implementation that integrates the reputation-to-voting-power mapping.

**Design outline (Rust):**

```rust
/// Neunode's consensus context implementing Malachite's Context trait.
///
/// This bridges the reputation system with the consensus engine:
/// - Validator set is derived from reputation scores, not raw stake
/// - Voting power uses sqrt mapping
/// - Epoch transitions are managed through this context
pub struct NeunodeContext {
    /// Current epoch metadata
    epoch: EpochState,
    /// Reputation scores for active validators
    validators: BTreeMap<Did, ValidatorInfo>,
    /// Slashing state
    slashing: SlashingState,
}

pub struct EpochState {
    /// Current epoch number
    number: u64,
    /// Block height where this epoch started
    start_height: u64,
    /// Block height where this epoch ends
    end_height: u64,
    /// Hash of the validator set for this epoch
    validator_set_hash: Hash256,
}

pub struct ValidatorInfo {
    /// Agent DID
    did: Did,
    /// Computed reputation score (0-100)
    reputation: f64,
    /// Mapped voting power (uint64)
    voting_power: u64,
    /// Validator status (Active, Jailed, Tombstoned)
    status: ValidatorStatus,
    /// Consensus key (Ed25519 public key for signing blocks)
    consensus_key: VerifyingKey,
    /// Last block this validator signed
    last_signed_block: u64,
    /// Missed block counter (for downtime tracking)
    missed_blocks: u64,
}

impl NeunodeContext {
    /// Convert reputation score to voting power using sqrt mapping.
    pub fn reputation_to_voting_power(reputation: f64) -> u64 {
        if reputation <= 0.0 {
            return 0;
        }
        const MAX_VP: f64 = 10_000_000_000.0;
        let vp = (reputation / 100.0).sqrt() * MAX_VP;
        vp.floor() as u64
    }

    /// Compute the validator set for the next epoch.
    /// Called during the snapshot window before epoch transition.
    pub fn compute_next_validator_set(
        &self,
        scores: &BTreeMap<Did, f64>,
    ) -> Vec<(Did, u64)> {
        const MIN_REPUTATION: f64 = 50.0;
        const MAX_VALIDATORS: usize = 100;

        let mut candidates: Vec<_> = scores
            .iter()
            .filter(|(_, &score)| score >= MIN_REPUTATION)
            .filter(|(did, _)| {
                // Exclude jailed/tombstoned validators
                self.slashing.get_status(did) == ValidatorStatus::Active
            })
            .map(|(did, &score)| {
                (did.clone(), Self::reputation_to_voting_power(score))
            })
            .collect();

        // Sort by voting power descending, then by DID for tie-breaking
        candidates.sort_by(|a, b| {
            b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0))
        });

        candidates.truncate(MAX_VALIDATORS);
        candidates
    }
}
```

### 3.8 Edge Cases

| Edge Case | Handling |
|-----------|----------|
| No validators above threshold | Emergency mode: top 3 agents by stake (regardless of reputation) become validators. Governance notified. |
| All validators leave simultaneously | Network halts. Requires governance intervention to bootstrap new validator set. |
| Single validator has >1/3 of total VP | Warning emitted. If >1/2, new validators are promoted to dilute. Never allow >2/3 concentration. |
| Validator joins mid-epoch | Must wait for next epoch. Pre-registered during snapshot window. |
| Fork with different validator sets | Use the validator set hash committed on-chain. The set with the canonical chain history wins. |
| Reputation computation divergence | All nodes compute reputation from on-chain data (deterministic). The on-chain reputation contract is the source of truth. |

---

## 4. Task 3: On-Chain Reputation Contract

### 4.1 Contract Design

The `NeunodeReputation` contract stores and manages per-agent reputation scores on-chain. It integrates with the existing Diamond proxy pattern (EIP-2535) and other Neunode contracts.

```solidity
// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";

interface INeunodeReputation {
    // ─── Types ────────────────────────────────────────────────────

    /// Per-agent reputation factor scores (all in basis points, 0-10000 = 0%-100%)
    struct FactorScores {
        uint16 stake;      // 0-10000
        uint16 attest;     // 0-10000
        uint16 activity;   // 0-10000
        uint16 verify;     // 0-10000
        uint16 tenure;     // 0-10000
    }

    /// Full reputation record for an agent
    struct ReputationRecord {
        FactorScores factors;
        uint16 compositeScore;   // Weighted sum, 0-10000
        uint16 penaltyPoints;    // Accumulated slashing penalties, decays over time
        uint8  grade;            // 0=F, 1=D, 2=C, 3=B, 4=A
        uint256 lastUpdated;
        uint256 createdAt;
    }

    /// Weight configuration (basis points, must sum to 10000)
    struct FactorWeights {
        uint16 stake;     // default 3000 (30%)
        uint16 attest;    // default 2500 (25%)
        uint16 activity;  // default 2000 (20%)
        uint16 verify;    // default 1500 (15%)
        uint16 tenure;    // default 1000 (10%)
    }

    // ─── Events ───────────────────────────────────────────────────

    event ReputationUpdated(
        bytes32 indexed didHash,
        uint16 compositeScore,
        uint8 grade,
        FactorScores factors
    );

    event FactorUpdated(
        bytes32 indexed didHash,
        string factor,
        uint16 oldValue,
        uint16 newValue
    );

    event PenaltyApplied(
        bytes32 indexed didHash,
        uint16 penaltyPoints,
        uint16 totalPenalty
    );

    event PenaltyDecayed(
        bytes32 indexed didHash,
        uint16 remainingPenalty
    );

    event WeightsUpdated(FactorWeights oldWeights, FactorWeights newWeights);

    event ValidatorThresholdUpdated(uint16 oldThreshold, uint16 newThreshold);

    // ─── Score Update Functions ───────────────────────────────────

    /// @notice Update the stake factor for an agent
    /// @param didHash The agent's DID hash
    /// @param score Basis points (0-10000)
    function updateStakeFactor(bytes32 didHash, uint16 score) external;

    /// @notice Update the attestation factor for an agent
    function updateAttestFactor(bytes32 didHash, uint16 score) external;

    /// @notice Update the activity factor for an agent
    function updateActivityFactor(bytes32 didHash, uint16 score) external;

    /// @notice Update the verification factor for an agent
    function updateVerifyFactor(bytes32 didHash, uint16 score) external;

    /// @notice Update the tenure factor for an agent
    function updateTenureFactor(bytes32 didHash, uint16 score) external;

    /// @notice Batch update all factors at once (for epoch snapshots)
    function updateAllFactors(
        bytes32 didHash,
        FactorScores calldata scores
    ) external;

    /// @notice Apply a slashing penalty to an agent's reputation
    /// @param penaltyPoints Basis points to subtract from composite
    function applyPenalty(bytes32 didHash, uint16 penaltyPoints) external;

    /// @notice Decay penalty points over time (called during epoch transitions)
    function decayPenalties(bytes32[] calldata didHashes) external;

    // ─── Query Functions ──────────────────────────────────────────

    /// @notice Get the full reputation record for an agent
    function getReputation(bytes32 didHash)
        external view returns (ReputationRecord memory);

    /// @notice Get only the composite score
    function getCompositeScore(bytes32 didHash) external view returns (uint16);

    /// @notice Get the computed voting power for an agent
    function getVotingPower(bytes32 didHash) external view returns (uint64);

    /// @notice Get the grade for an agent
    function getGrade(bytes32 didHash) external view returns (uint8);

    /// @notice Check if an agent meets validator eligibility criteria
    function isValidatorEligible(bytes32 didHash) external view returns (bool);

    /// @notice Get the current factor weights
    function getWeights() external view returns (FactorWeights memory);

    /// @notice Get the minimum reputation threshold for validator status
    function getValidatorThreshold() external view returns (uint16);

    // ─── Governance Functions ─────────────────────────────────────

    /// @notice Update factor weights (governance only, must sum to 10000)
    function setWeights(FactorWeights calldata newWeights) external;

    /// @notice Update the minimum reputation threshold for validators
    function setValidatorThreshold(uint16 threshold) external;
}
```

### 4.2 Full Implementation Sketch

```solidity
contract NeunodeReputation is INeunodeReputation, AccessControl, Pausable {
    // ─── Constants ────────────────────────────────────────────────

    uint16 public constant MAX_BPS = 10000;
    uint64 public constant MAX_VOTING_POWER = 10_000_000_000;
    uint16 public constant DEFAULT_VALIDATOR_THRESHOLD = 5000; // 50%

    // Penalty decay: 90 epochs = ~90 hours at 1h epochs
    uint16 public constant PENALTY_DECAY_PER_EPOCH = 111; // ~1/90 of 10000

    // ─── Roles ────────────────────────────────────────────────────

    bytes32 public constant STAKE_ORACLE_ROLE = keccak256("STAKE_ORACLE_ROLE");
    bytes32 public constant ATTEST_ORACLE_ROLE = keccak256("ATTEST_ORACLE_ROLE");
    bytes32 public constant ACTIVITY_ORACLE_ROLE = keccak256("ACTIVITY_ORACLE_ROLE");
    bytes32 public constant VERIFY_ORACLE_ROLE = keccak256("VERIFY_ORACLE_ROLE");
    bytes32 public constant TENURE_ORACLE_ROLE = keccak256("TENURE_ORACLE_ROLE");
    bytes32 public constant SLASHING_ROLE = keccak256("SLASHING_ROLE");
    bytes32 public constant REPUTATION_ADMIN_ROLE = keccak256("REPUTATION_ADMIN_ROLE");

    // ─── Storage ──────────────────────────────────────────────────

    FactorWeights public weights;
    uint16 public validatorThreshold;
    mapping(bytes32 => ReputationRecord) private _records;
    uint256 public currentEpoch;

    // ─── Errors ───────────────────────────────────────────────────

    error WeightsMustSumTo10000(uint256 actual);
    error ScoreOutOfBounds(uint16 score);
    error AgentNotFound(bytes32 didHash);
    error PenaltyDecayNotDue(bytes32 didHash);

    // ─── Constructor ──────────────────────────────────────────────

    constructor() {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(REPUTATION_ADMIN_ROLE, msg.sender);

        weights = FactorWeights({
            stake: 3000,
            attest: 2500,
            activity: 2000,
            verify: 1500,
            tenure: 1000
        });

        validatorThreshold = DEFAULT_VALIDATOR_THRESHOLD;
    }

    // ─── Internal ─────────────────────────────────────────────────

    function _computeComposite(FactorScores memory f) internal view returns (uint16) {
        uint256 composite = (
            uint256(f.stake) * weights.stake +
            uint256(f.attest) * weights.attest +
            uint256(f.activity) * weights.activity +
            uint256(f.verify) * weights.verify +
            uint256(f.tenure) * weights.tenure
        ) / MAX_BPS;

        // Subtract penalty, floor at 0
        ReputationRecord storage record = _records[msg.data...]; // placeholder

        return uint16(composite.min(MAX_BPS));
    }

    function _gradeFromScore(uint16 score) internal pure returns (uint8) {
        if (score >= 9000) return 4; // A
        if (score >= 7500) return 3; // B
        if (score >= 5000) return 2; // C
        if (score >= 2500) return 1; // D
        return 0; // F
    }

    function _votingPowerFromScore(uint16 score) internal pure returns (uint64) {
        if (score == 0) return 0;
        // sqrt(score / 10000) * MAX_VOTING_POWER
        // Using integer sqrt approximation
        uint256 scaledScore = uint256(score) * MAX_VOTING_POWER * MAX_VOTING_POWER / MAX_BPS;
        uint64 vp = uint64(_sqrt(scaledScore));
        return vp == 0 ? 1 : vp;
    }

    /// @dev Integer square root (Babylonian method)
    function _sqrt(uint256 x) internal pure returns (uint256) {
        if (x == 0) return 0;
        uint256 z = (x + 1) / 2;
        uint256 y = x;
        while (z < y) {
            y = z;
            z = (x / z + z) / 2;
        }
        return y;
    }

    function _updateComposite(bytes32 didHash) internal {
        ReputationRecord storage r = _records[didHash];
        uint256 composite = (
            uint256(r.factors.stake) * weights.stake +
            uint256(r.factors.attest) * weights.attest +
            uint256(r.factors.activity) * weights.activity +
            uint256(r.factors.verify) * weights.verify +
            uint256(r.factors.tenure) * weights.tenure
        ) / MAX_BPS;

        // Subtract penalty, floor at 0
        if (composite > r.penaltyPoints) {
            composite -= r.penaltyPoints;
        } else {
            composite = 0;
        }

        r.compositeScore = uint16(composite > MAX_BPS ? MAX_BPS : composite);
        r.grade = _gradeFromScore(r.compositeScore);
        r.lastUpdated = block.timestamp;
    }

    // ─── Factor Updates ───────────────────────────────────────────

    function updateStakeFactor(bytes32 didHash, uint16 score)
        external onlyRole(STAKE_ORACLE_ROLE) whenNotPaused
    {
        if (score > MAX_BPS) revert ScoreOutOfBounds(score);
        uint16 old = _records[didHash].factors.stake;
        _records[didHash].factors.stake = score;
        if (_records[didHash].createdAt == 0) {
            _records[didHash].createdAt = block.timestamp;
        }
        _updateComposite(didHash);
        emit FactorUpdated(didHash, "stake", old, score);
        emit ReputationUpdated(
            didHash,
            _records[didHash].compositeScore,
            _records[didHash].grade,
            _records[didHash].factors
        );
    }

    function updateAttestFactor(bytes32 didHash, uint16 score)
        external onlyRole(ATTEST_ORACLE_ROLE) whenNotPaused
    {
        if (score > MAX_BPS) revert ScoreOutOfBounds(score);
        uint16 old = _records[didHash].factors.attest;
        _records[didHash].factors.attest = score;
        if (_records[didHash].createdAt == 0) {
            _records[didHash].createdAt = block.timestamp;
        }
        _updateComposite(didHash);
        emit FactorUpdated(didHash, "attest", old, score);
        emit ReputationUpdated(
            didHash,
            _records[didHash].compositeScore,
            _records[didHash].grade,
            _records[didHash].factors
        );
    }

    function updateActivityFactor(bytes32 didHash, uint16 score)
        external onlyRole(ACTIVITY_ORACLE_ROLE) whenNotPaused
    {
        if (score > MAX_BPS) revert ScoreOutOfBounds(score);
        uint16 old = _records[didHash].factors.activity;
        _records[didHash].factors.activity = score;
        if (_records[didHash].createdAt == 0) {
            _records[didHash].createdAt = block.timestamp;
        }
        _updateComposite(didHash);
        emit FactorUpdated(didHash, "activity", old, score);
        emit ReputationUpdated(
            didHash,
            _records[didHash].compositeScore,
            _records[didHash].grade,
            _records[didHash].factors
        );
    }

    function updateVerifyFactor(bytes32 didHash, uint16 score)
        external onlyRole(VERIFY_ORACLE_ROLE) whenNotPaused
    {
        if (score > MAX_BPS) revert ScoreOutOfBounds(score);
        uint16 old = _records[didHash].factors.verify;
        _records[didHash].factors.verify = score;
        if (_records[didHash].createdAt == 0) {
            _records[didHash].createdAt = block.timestamp;
        }
        _updateComposite(didHash);
        emit FactorUpdated(didHash, "verify", old, score);
        emit ReputationUpdated(
            didHash,
            _records[didHash].compositeScore,
            _records[didHash].grade,
            _records[didHash].factors
        );
    }

    function updateTenureFactor(bytes32 didHash, uint16 score)
        external onlyRole(TENURE_ORACLE_ROLE) whenNotPaused
    {
        if (score > MAX_BPS) revert ScoreOutOfBounds(score);
        uint16 old = _records[didHash].factors.tenure;
        _records[didHash].factors.tenure = score;
        if (_records[didHash].createdAt == 0) {
            _records[didHash].createdAt = block.timestamp;
        }
        _updateComposite(didHash);
        emit FactorUpdated(didHash, "tenure", old, score);
        emit ReputationUpdated(
            didHash,
            _records[didHash].compositeScore,
            _records[didHash].grade,
            _records[didHash].factors
        );
    }

    function updateAllFactors(bytes32 didHash, FactorScores calldata scores)
        external onlyRole(REPUTATION_ADMIN_ROLE) whenNotPaused
    {
        if (scores.stake > MAX_BPS || scores.attest > MAX_BPS ||
            scores.activity > MAX_BPS || scores.verify > MAX_BPS ||
            scores.tenure > MAX_BPS) {
            revert ScoreOutOfBounds(10001);
        }

        if (_records[didHash].createdAt == 0) {
            _records[didHash].createdAt = block.timestamp;
        }
        _records[didHash].factors = scores;
        _updateComposite(didHash);

        emit ReputationUpdated(
            didHash,
            _records[didHash].compositeScore,
            _records[didHash].grade,
            scores
        );
    }

    // ─── Penalty ──────────────────────────────────────────────────

    function applyPenalty(bytes32 didHash, uint16 penaltyPoints)
        external onlyRole(SLASHING_ROLE) whenNotPaused
    {
        _records[didHash].penaltyPoints =
            uint16(uint256(_records[didHash].penaltyPoints) + penaltyPoints);
        _updateComposite(didHash);
        emit PenaltyApplied(didHash, penaltyPoints, _records[didHash].penaltyPoints);
    }

    function decayPenalties(bytes32[] calldata didHashes) external {
        for (uint256 i = 0; i < didHashes.length; i++) {
            uint16 current = _records[didHashes[i]].penaltyPoints;
            if (current == 0) continue;
            uint16 newPenalty = current > PENALTY_DECAY_PER_EPOCH
                ? current - PENALTY_DECAY_PER_EPOCH
                : 0;
            _records[didHashes[i]].penaltyPoints = newPenalty;
            _updateComposite(didHashes[i]);
            emit PenaltyDecayed(didHashes[i], newPenalty);
        }
    }

    // ─── Queries ──────────────────────────────────────────────────

    function getReputation(bytes32 didHash)
        external view returns (ReputationRecord memory)
    {
        return _records[didHash];
    }

    function getCompositeScore(bytes32 didHash) external view returns (uint16) {
        return _records[didHash].compositeScore;
    }

    function getVotingPower(bytes32 didHash) external view returns (uint64) {
        return _votingPowerFromScore(_records[didHash].compositeScore);
    }

    function getGrade(bytes32 didHash) external view returns (uint8) {
        return _records[didHash].grade;
    }

    function isValidatorEligible(bytes32 didHash) external view returns (bool) {
        ReputationRecord memory r = _records[didHash];
        return r.compositeScore >= validatorThreshold && r.penaltyPoints < 500;
    }

    // ─── Governance ───────────────────────────────────────────────

    function setWeights(FactorWeights calldata newWeights)
        external onlyRole(REPUTATION_ADMIN_ROLE)
    {
        uint256 sum = uint256(newWeights.stake) + newWeights.attest +
                      newWeights.activity + newWeights.verify + newWeights.tenure;
        if (sum != MAX_BPS) revert WeightsMustSumTo10000(sum);

        FactorWeights memory oldWeights = weights;
        weights = newWeights;
        emit WeightsUpdated(oldWeights, newWeights);
    }

    function setValidatorThreshold(uint16 threshold)
        external onlyRole(REPUTATION_ADMIN_ROLE)
    {
        uint16 oldThreshold = validatorThreshold;
        validatorThreshold = threshold;
        emit ValidatorThresholdUpdated(oldThreshold, threshold);
    }

    function pause() external onlyRole(REPUTATION_ADMIN_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(REPUTATION_ADMIN_ROLE) {
        _unpause();
    }
}
```

### 4.3 Access Control Model

Each factor has its own oracle role, allowing separation of update authority:

| Role | Who holds it | What it controls |
|------|-------------|-----------------|
| `STAKE_ORACLE_ROLE` | Staking contract (auto-update) | Stake factor score |
| `ATTEST_ORACLE_ROLE` | Attestation oracle | Attestation factor score |
| `ACTIVITY_ORACLE_ROLE` | Activity monitor oracle | Activity factor score |
| `VERIFY_ORACLE_ROLE` | Verification oracle | Verification factor score |
| `TENURE_ORACLE_ROLE` | Identity contract (auto-update) | Tenure factor score |
| `SLASHING_ROLE` | Slashing contract | Reputation penalties |
| `REPUTATION_ADMIN_ROLE` | Governance (timelocked) | Weights, thresholds, batch updates, pause |

### 4.4 Gas Optimization Notes

- **Basis points (uint16)** instead of floating point: Solidity has no floats. All scores are 0-10000 representing 0%-100%. The Rust-side `f64` scores are multiplied by 100 before passing on-chain.
- **Single storage slot for factors:** Five `uint16` values fit in one `bytes32` storage slot (5 x 16 bits = 80 bits). Packing `FactorScores` into a single slot saves ~20k gas per update.
- **Batch decay:** `decayPenalties()` accepts arrays to amortize the fixed transaction cost across multiple agents.
- **Lazy composite recomputation:** Composite is only recomputed when a factor changes or a penalty is applied, not on every query.

### 4.5 Diamond Facet Integration

`NeunodeReputation` should be deployed as a facet in the existing EIP-2535 Diamond proxy pattern:

```
Diamond (proxy)
  ├── NeunodeIdentity facet      (existing)
  ├── NeunodeBounty facet        (existing)
  ├── NeunodeEscrow facet        (existing)
  ├── NeunodeGovernance facet    (existing)
  ├── NeunodeReputation facet    (NEW - this contract)
  └── NeunodeSlashing facet      (NEW - from Task 1)
```

The Diamond's `LibDiamond` storage pattern ensures the reputation facet can access shared state (e.g., DID documents) through the diamond storage position.

---

## 5. Integration Points

### 5.1 Contract Dependency Graph

```
NeunodeIdentity ─────────────────────────────┐
  │                                           │
  ▼                                           ▼
NeunodeToken ◄──── StakingEscrow       NeunodeReputation
  │                                     │           │
  │                                     │           │
  ▼                                     ▼           ▼
NeunodeGovernance ◄── NeunodeSlashing ◄─┘     ValidatorSet
                                                   │
                                                   ▼
                                            NeunodeContext
                                            (Malachite/CometBFT)
```

### 5.2 Cross-Contract Interactions

**Slashing flow (end-to-end):**

```
1. Offense detected (e.g., double sign)
2. NeunodeSlashing.submitEvidence() called
3. Slashing contract:
   a. Verifies evidence
   b. Calls NeunodeToken.slashStake() to burn staked tokens
   c. Calls NeunodeReputation.applyPenalty() to reduce reputation
   d. Updates validator status (Jailed/Tombstoned)
   e. Emits ValidatorSlashed event
4. At next epoch boundary:
   a. NeunodeReputation.decayPenalties() called
   b. ValidatorSet recomputed from updated reputation scores
   c. Slashed validator excluded (if below threshold or jailed)
5. Governance can update slashing parameters via timelocked proposal
```

**Reputation update flow (end-to-end):**

```
1. Oracle observes agent activity off-chain
   (e.g., completed 10 bounties successfully)
2. Oracle calls NeunodeReputation.updateVerifyFactor(didHash, newScore)
3. Contract:
   a. Verifies caller has VERIFY_ORACLE_ROLE
   b. Updates verify factor
   c. Recomputes composite score with current weights
   d. Emits FactorUpdated and ReputationUpdated events
4. At epoch snapshot:
   a. ValidatorSet reads NeunodeReputation.getVotingPower() for all agents
   b. Top 100 eligible agents become new validator set
   c. Voting power = sqrt(compositeScore / 10000) * MAX_VP
```

### 5.3 Rust Crate Integration

The existing `neunode-reputation` crate (`crates/neunode-reputation/`) provides the off-chain computation logic. The on-chain contract stores the canonical scores. The relationship:

```
Off-chain (Rust):
  neunode-reputation::score::ReputationScore::compute()
  → Computes factors from raw data
  → Returns composite score

On-chain (Solidity):
  NeunodeReputation.updateAllFactors()
  → Stores factor scores as uint16 basis points
  → Recomputes composite on-chain (deterministic)

Oracle bridge:
  Rust oracle computes factors → submits tx to Solidity contract
  → Contract verifies oracle role → updates storage
```

### 5.4 SDK Integration

The TypeScript SDK needs a new `reputation` resource module:

```typescript
// sdk/src/resources/reputation.ts

export interface ReputationRecord {
  factors: {
    stake: number;      // basis points 0-10000
    attest: number;
    activity: number;
    verify: number;
    tenure: number;
  };
  compositeScore: number;
  penaltyPoints: number;
  grade: 'A' | 'B' | 'C' | 'D' | 'F';
  lastUpdated: number;
}

export interface ValidatorInfo {
  did: Did;
  reputation: number;
  votingPower: bigint;
  status: 'active' | 'jailed' | 'tombstoned';
}

export const createReputationResource = (transport: Transport) => ({
  getScore(did: Did): Promise<ReputationRecord>,
  getVotingPower(did: Did): Promise<bigint>,
  isValidatorEligible(did: Did): Promise<boolean>,
  getSlashingHistory(did: Did): Promise<SlashingEvent[]>,
});
```

---

## 6. Open Questions

### Design Decisions Needed

1. **Oracle model:** Should factor updates be pushed by off-chain oracles (current design) or should the contract compute factors from on-chain data directly? The latter is more trustless but limits factors to what's observable on-chain.

2. **Epoch length:** Is 1 hour (720 blocks at 5s) the right granularity? Shorter epochs respond faster to attacks but cause more churn. Longer epochs are more stable but slower to respond.

3. **Validator set size:** 100 validators balances decentralization with BFT overhead. At mainnet scale, should this be governable up to 500?

4. **Sqrt vs. linear for voting power:** The spike recommends sqrt, but this should be validated with a game-theoretic analysis of Sybil economics. If the cost of creating N identities is sub-linear in N (e.g., fixed per-identity), then sqrt mapping might still be exploitable.

5. **Penalty decay rate:** 90-day linear decay means an agent slashed to 0 reputation needs 90 days of perfect behavior to recover. Is this too punitive or too lenient?

6. **Slashing distribution:** Should slashed tokens follow the same 40/30/20/10 split as token decay, or should a larger fraction go to the whistleblower to incentivize reporting?

7. **Emergency validator set:** If no agents meet the threshold, should the fallback be top-by-stake, top-by-existing-validators, or a hardcoded set of bootstrap validators?

### Implementation Order

```
Phase 1 (blocks everything else):
  1. NeunodeReputation contract (Solidity)
  2. NeunodeSlashing contract (Solidity)
  3. Voting power computation (Rust, in neunode-reputation crate)

Phase 2 (depends on Phase 1):
  4. NeunodeContext (Malachite integration, Rust)
  5. Epoch management contract (Solidity)
  6. Validator set contract (Solidity)

Phase 3 (depends on Phase 2):
  7. SDK reputation resource module (TypeScript)
  8. CLI commands for reputation/slashing (Rust, agnetd)
  9. Integration tests (Rust + Solidity + SDK)
```

---

## Appendix A: Parameter Summary Table

| Parameter | Value | Location | Governance? |
|-----------|-------|----------|-------------|
| Weight: Stake | 30% | NeunodeReputation.weights | Yes |
| Weight: Attestation | 25% | NeunodeReputation.weights | Yes |
| Weight: Activity | 20% | NeunodeReputation.weights | Yes |
| Weight: Verification | 15% | NeunodeReputation.weights | Yes |
| Weight: Tenure | 10% | NeunodeReputation.weights | Yes |
| Min validator reputation | 50% (5000 bps) | NeunodeReputation | Yes |
| Max validators | 100 | ValidatorSet | Yes |
| Voting power function | sqrt mapping | NeunodeContext | Yes (via upgrade) |
| Max voting power | 10,000,000,000 | NeunodeContext | No (constant) |
| Epoch length | 720 blocks (~1h) | EpochManager | Yes |
| S1 slash (1st offense) | 5% | NeunodeSlashing | Yes |
| S3 slash (1st offense) | 10% | NeunodeSlashing | Yes |
| Reputation slash multiplier | 2.0x | NeunodeSlashing | Yes |
| Penalty decay period | 90 epochs | NeunodeReputation | Yes |
| Unbonding period | 21 days | NeunodeToken | Yes |
| Jail duration (downtime) | 7 days | NeunodeSlashing | Yes |
| Evidence bond | 1% of accused stake | NeunodeSlashing | Yes |
| Appeal window | 48 hours | NeunodeSlashing | Yes |

## Appendix B: Relationship to Existing Code

| New Component | Integrates With | Integration Point |
|---------------|----------------|-------------------|
| NeunodeReputation.sol | NeunodeToken.sol | STAKE_ORACLE reads staked balance |
| NeunodeReputation.sol | NeunodeIdentity.sol | DID hash as primary key |
| NeunodeReputation.sol | NeunodeGovernance.sol | Voting power replaces raw staked balance |
| NeunodeSlashing.sol | NeunodeToken.sol | Calls slashStake() |
| NeunodeSlashing.sol | NeunodeReputation.sol | Calls applyPenalty() |
| NeunodeSlashing.sol | BountyReview.sol | Reads review committee decisions for S4 |
| ValidatorSet.sol | NeunodeReputation.sol | Reads voting power for set construction |
| neunode-reputation (Rust) | NeunodeReputation.sol | Off-chain computation feeds on-chain oracle |
| NeunodeContext (Rust) | ValidatorSet.sol | Reads validator set for Malachite consensus |
