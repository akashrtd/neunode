# Neunode — Smart Contract Architecture

## The Problem

```
NEUNODE NEEDS ON-CHAIN LOGIC FOR:
  ├── Agent identity (who are you?)
  ├── Resource tokens (how do you pay?)
  ├── Escrow (how do you trade safely?)
  ├── Reputation (who is trustworthy?)
  ├── Bounties (who does the work?)
  ├── Model lineage (who built what?)
  └── Governance (who decides rules?)

  WRONG APPROACH: One monolithic contract.
  ├── Gas costs explode as state grows
  ├── Any bug = full system upgrade
  ├── Single point of failure
  └── Cannot evolve individual components

  RIGHT APPROACH: Suite of 11 focused contracts.
  ├── Each contract owns one domain
  ├── Compose via well-defined interfaces
  ├── Upgrade what changes, freeze what doesn't
  └── Gas-efficient: minimize cross-contract calls
```

---

## Contract Relationship Diagram

```
                          ┌──────────────────────────┐
                          │   NeunodeGovernance       │
                          │   (OpenZeppelin Governor) │
                          │   propose/execute/queue   │
                          └─────────┬────────────────┘
                                    │ governs all upgradable contracts
                                    │
         ┌──────────────────────────┼──────────────────────────┐
         │                          │                          │
    ┌────▼─────┐             ┌──────▼──────┐          ┌───────▼────────┐
    │ Registry  │             │  Reputation  │          │   Governance   │
    │ ERC-8126  │◄───────────│  Scoring     │          │   Params       │
    └────┬─────┘  queries     └──────┬──────┘          └────────────────┘
         │        reput-             │
         │        ation              │ updates
         │                          │
    ┌────▼─────────┐          ┌─────▼──────┐
    │  Identity    │          │  Staking    │
    │  DID Reg     │          │  Mechanics  │
    └──────────────┘          └─────┬──────┘
                                    │ locks/releases tokens
                                    │
    ┌───────────────────────────────┼───────────────────────────────┐
    │                               │                               │
┌───▼──────────┐  ┌────────────────▼──────────────┐  ┌─────────────▼───────┐
│ ComputeToken │  │          NeunodeEscrow         │  │  NeunodeBounty      │
│ TrainToken   │  │  (iExec PoCo bilateral model)  │  │  (lifecycle FSM)    │
│ BandToken    │  └────────────────┬──────────────┘  └──────────┬──────────┘
│ StorageToken │                   │ create/fund/               │ create/claim/
└──────────────┘                   │ claim/release              │ submit/review
        ▲                          │                            │
        │ uses tokens              │                            │
        │                          │                            │
        │              ┌───────────▼────────────┐   ┌───────────▼────────┐
        │              │  NeunodeVerification   │   │  NeunodeLineage    │
        │              │  (attestation + dispute)│   │  (model DAG +      │
        │              └────────────────────────┘   │   royalty weights) │
        │                                             └──────────┬─────────┘
        │                                                        │
        │              ┌────────────────────────┐                │
        │              │  NeunodeTokenDecay     │                │
        │              │  (activity-based decay │                │
        │              │   rate calculation)    │                │
        │              └────────────────────────┘
        │
        └─── All 4 ERC-20s feed into: Escrow, Staking, Decay

ARROWS = dependency direction (reads from, calls into)
TOKENS are leaf nodes (immutable, no outbound calls)
GOVERNANCE sits above everything (can modify parameters)
```

---

## Contract Suite Overview

```
┌─────────────────────┬────────────────┬──────────────┬───────────────────────┐
│ Contract            │ Upgrade Pattern │ Access       │ Phase                 │
├─────────────────────┼────────────────┼──────────────┼───────────────────────┤
│ NeunodeRegistry     │ Diamond (2535) │ OPERATOR     │ 1 (core)              │
│ NeunodeIdentity     │ UUPS           │ OWNER        │ 1 (core)              │
│ NeunodeReputation   │ Diamond (2535) │ OPERATOR     │ 1 (core)              │
│ NeunodeToken (×4)   │ IMMUTABLE      │ MINTER_ROLE  │ 1 (core)              │
│ NeunodeStaking      │ UUPS           │ OPERATOR     │ 2 (staking)           │
│ NeunodeEscrow       │ Diamond (2535) │ OPERATOR     │ 1 (core)              │
│ NeunodeBounty       │ Diamond (2535) │ OPERATOR     │ 1 (core)              │
│ NeunodeGovernance   │ Diamond (2535) │ GOVERNOR     │ 2 (dao)               │
│ NeunodeLineage      │ UUPS           │ OPERATOR     │ 2 (lineage)           │
│ NeunodeVerification │ UUPS           │ OPERATOR     │ 2 (verification)      │
│ NeunodeTokenDecay   │ UUPS           │ OPERATOR     │ 3 (decay)             │
└─────────────────────┴────────────────┴──────────────┴───────────────────────┘

DIAMOND (EIP-2535): Core protocol contracts that change frequently.
  Multiple facets share one proxy. Add/replace facets without redeploying.
  Use for: Registry, Escrow, Bounty, Governance (4+ functions likely to evolve).

UUPS: Simpler upgrade path for contracts with moderate change frequency.
  Upgrade logic lives IN the implementation (not a separate proxy admin).
  Use for: Identity, Staking, Lineage, Verification, TokenDecay.

IMMUTABLE: Token contracts. Never upgrade. Bugs = deploy new token + migrate.
  ERC-20 is battle-tested. No reason to add upgrade complexity.
```

---

## Contract Interfaces

### 1. NeunodeRegistry (ERC-8126)

```
PURPOSE: Agent registration and discovery. The phone book of the network.

  register(did, agentCardURI, capabilities[])
    → Agent must have valid DID in NeunodeIdentity
    → Agent must have staked minimum tokens in NeunodeStaking
    → Emits AgentRegistered(did, capabilities)

  updateAgent(did, agentCardURI, capabilities[])
    → Only agent's DID controller or OPERATOR
    → Emits AgentUpdated(did)

  deregister(did)
    → Agent or OPERATOR can trigger
    → Active bounties must be resolved first
    → Emits AgentDeregistered(did)

  getAgent(did) → (cardURI, capabilities, reputation, stakedAmount)
    → View function, no gas cost
    → Pulls reputation from NeunodeReputation, stake from NeunodeStaking

ACCESS: OPERATOR role for slashing/deregistration. Agent self-register.
GAS: register ~80k gas, updateAgent ~45k gas, getAgent ~25k gas (view)
```

### 2. NeunodeIdentity (DID Registry)

```
PURPOSE: W3C DID document management. did:ethr method implementation.

  createDID(did, document)
    → did = keccak256(controller_address)[:20] formatted as did:ethr:<addr>
    → document = JSON-LD DID document (verification methods, services)
    → Emits DIDCreated(did, controller)

  updateDID(did, document)
    → Only current DID controller
    → Previous version hash stored for audit trail
    → Emits DIDUpdated(did, versionHash)

  deactivateDID(did)
    → Controller only. Marks DID as deactivated (not deleted).
    → Required before agent death protocol
    → Emits DIDDeactivated(did)

  addVerificationMethod(did, keyType, publicKey)
    → Adds new key to DID document (Ed25519, secp256k1, etc.)
    → Used for key rotation without full document replacement

  rotateKey(did, oldKeyId, newKeyId, proof)
    → Signature proof that old key authorized the rotation
    → oldKeyId marked revoked, newKeyId becomes active
    → Emits KeyRotated(did, oldKeyId, newKeyId)

ACCESS: OWNER (contract owner), DID controller (per-DID self-management)
GAS: createDID ~120k gas, rotateKey ~55k gas
```

### 3. NeunodeReputation

```
PURPOSE: Composite reputation scoring. Not a simple number — weighted categories.

  updateScore(did, category, delta, evidence)
    → category: stake | attest | activity | verify | tenure
    → delta: signed integer (+/- with evidence)
    → evidence: IPFS CID pointing to verifiable proof
    → Only OPERATOR or automated verification contracts

  getScore(did) → compositeScore
    → Weighted: stake(30%) + attest(25%) + activity(20%) + verify(15%) + tenure(10%)
    → Cached, recalculated on update

  attest(from, to, claim, stake)
    → Agent stakes tokens on a claim about another agent
    → claim: "delivered_on_time" | "quality_high" | "reliable_inference" etc.
    → False attestation → slash stake + reputation penalty

  disputeAttestation(attestationId, evidence)
    → Challenge an attestation within dispute window (7 days)
    → Challenger deposits dispute bond
    → Triggers NeunodeVerification for resolution

  slash(did, amount, reason)
    → OPERATOR only. Reduces reputation score.
    → reason: violation enum (hash_mismatch, fraud, spam, collude)
    → Emits ReputationSlashed(did, amount, reason)

ACCESS: OPERATOR for updates/slashing. Any agent for attest/dispute.
GAS: updateScore ~40k gas, attest ~65k gas, getScore ~15k gas (view)
```

### 4. NeunodeToken (4 × ERC-20)

```
PURPOSE: Resource-backed utility tokens. NOT securities, NOT governance tokens.

  ┌──────────────────┬───────────────┬─────────────┬──────────────────────┐
  │ Token            │ Symbol        │ Decimals    │ Backing              │
  ├──────────────────┼───────────────┼─────────────┼──────────────────────┤
  │ ComputeToken     │ nCompute      │ 18          │ GPU-hours (H100 equiv│
  │ TrainingToken    │ nTrain        │ 18          │ training-unit (1 unit│
  │                  │               │             │ = 1B params × 1 step)│
  │ BandwidthToken   │ nBandwidth    │ 18          │ TB transfer          │
  │ StorageToken     │ nStorage      │ 18          │ GB-month             │
  └──────────────────┴───────────────┴─────────────┴──────────────────────┘

  Shared interface (each token implements independently):
    mint(to, amount)         → MINTER_ROLE only (earned through contribution)
    burn(from, amount)       → BURNER_ROLE or self-burn (consumed for services)
    transfer(to, amount)     → Standard ERC-20
    approve(spender, amount) → Standard ERC-20 (used by Escrow, Staking)
    decayRate(account)       → Calls NeunodeTokenDecay for current rate

  MINTING RULES:
    Compute:  minted when agent serves verified inference
    Training: minted when agent completes training checkpoint
    Bandwidth:minted when agent relays data (verified via traffic proofs)
    Storage:  minted when agent hosts data (verified via availability proofs)

  No admin mint. No ICO. Tokens are EARNED, not sold.
```

### 5. NeunodeStaking

```
PURPOSE: Lock tokens as collateral. Required for registration, bounties, reviews.

  stake(did, token, amount)
    → Transfers tokens from agent → staking contract
    → Updates staking balance in NeunodeRegistry
    → Minimum stake required for ACTIVE state

  unstake(did, token, amount)
    → 7-day unbonding period (prevents hit-and-run)
    → Cannot unstake below minimum while registered
    → Cannot unstake if active bounties/escrows

  getStake(did) → totalStake
    → Aggregates across all 4 token types
    → Weighted: compute(1.0) × training(0.8) × bandwidth(0.5) × storage(0.3)

  slashStake(did, amount, reason)
    → OPERATOR only. Triggered by verification failures, disputes.
    → Slashed tokens: 40% treasury, 30% staking rewards, 20% burned, 10% dev fund
    → Emits StakeSlashed(did, amount, reason)

  distributeRewards(epoch)
    → Called once per epoch (24 hours)
    → Rewards proportional to: stake × activity_score × reputation
    → Minted fresh (inflationary reward for contributors)

ACCESS: OPERATOR for slashing/distribution. Agents self-stake/unstake.
GAS: stake ~50k gas, slashStake ~45k gas, distributeRewards ~200k gas (batch)
```

### 6. NeunodeEscrow (iExec PoCo Pattern)

```
PURPOSE: Bilateral escrow. Both parties lock value, verification settles.

  create(bountyId, requester, amount, token)
    → Requester deposits full payment
    → Escrow state = Created

  fund(bountyId, amount)
    → Additional funding (for milestone-based)
    → Only requester

  claim(bountyId, provider, bondAmount)
    → Provider deposits security bond (10-20% of payment)
    → Escrow state = Claimed

  submit(bountyId, resultHash)
    → Provider submits work. Hash committed on-chain.
    → Escrow state = WorkSubmitted

  release(bountyId, provider)
    → After verification passes
    → Provider receives: payment + bond
    → Protocol fee deducted (2-5%)
    → Escrow state = Settled

  dispute(bountyId, disputer, evidence)
    → Either party, within dispute window
    → Disputer deposits dispute bond
    → Triggers NeunodeVerification

  refund(bountyId)
    → Requester receives payment back
    → Provider receives bond back (minus dispute fee if applicable)
    → Only when: expired, cancelled, or dispute resolved for requester

ACCESS: Participants (requester/provider) for primary actions. OPERATOR for timeout.
GAS: create ~90k gas, claim ~75k gas, release ~60k gas
```

### 7. NeunodeBounty (Lifecycle FSM)

```
PURPOSE: Bounty creation, claiming, review. Orchestration layer over Escrow.

  STATES: Open → Claimed → Submitted → UnderReview → Revision
          → Accepted | Rejected | Disputed | Cancelled | Expired

  create(spec, reward, deadlines)
    → spec: IPFS CID pointing to bounty specification (Lexicon schema)
    → reward: token amounts per resource type
    → deadlines: {claim, work, review, revision, dispute}
    → Auto-creates NeunodeEscrow
    → Emits BountyCreated(bountyId) on feed (kind=1000)

  claim(bountyId)
    → Provider commits. Provider bond locked in escrow.
    → Claim deadline enforced.
    → Emits BountyClaimed(bountyId, providerDid) (kind=1001)

  submitWork(bountyId, resultHash)
    → Provider submits result hash (IPFS CID of deliverable)
    → Work deadline enforced.
    → Triggers automated verification (Layer 1: hash + format)

  review(bountyId, reviewerDid, score, evidence)
    → Selected reviewer submits score (0-100) + evidence
    → 2-of-3 majority required for resolution
    → Reviewer stakes tokens on verdict (skin in the game)

  accept(bountyId) / reject(bountyId, reason)
    → After review majority reached
    → accept → triggers Escrow.release()
    → reject → triggers Escrow.refund() + provider bond slashed

  cancel(bountyId)
    → Requester only, before any provider claims
    → Full refund

ACCESS: Requester creates/cancels. Provider claims/submits. Reviewers review.
GAS: create ~110k gas (includes escrow creation), submitWork ~50k gas
```

### 8. NeunodeGovernance (OpenZeppelin Governor)

```
PURPOSE: DAO governance. Protocol parameter changes, fee adjustments, upgrades.

  propose(targets[], values[], calldatas[], description)
    → targets: contract addresses to call
    → calldatas: encoded function calls
    → Proposer must hold minimum stake (anti-spam)
    → 1-day voting delay, 7-day voting period

  castVote(proposalId, support)
    → support: 0=Against, 1=For, 2=Abstain
    → Voting weight = √(staked_tokens) × reputation_multiplier
    → Quorum: 4% of total stake

  execute(proposalId)
    → After timelock (48 hours post-vote)
    → Executes calldatas on targets
    → Can upgrade facets, change parameters, slash

  queue(proposalId)
    → Successful proposals queued in timelock
    → 48-hour delay before execution (emergency exit window)

  state(proposalId) → ProposalState
    → Pending | Active | Canceled | Defeated | Succeeded | Queued | Expired | Executed

ACCESS: Any staked agent can propose. Any agent can vote. Weighted by √(stake)×reputation_multiplier.
```

### 9. NeunodeLineage

```
PURPOSE: Model lineage DAG. Track who built what from what. Enable royalty splits.

  registerModel(cid, parentCids[], contributorDid, contributionType)
    → cid: SHA-256 hash of safetensors file (content-addressed)
    → parentCids: models this was derived from (fine-tune, merge, RL)
    → contributionType: PreTraining | FineTune | RL | Data | Compute | Serving
    → Contributor must be registered agent
    → Signature: ed25519 detached sig over {cid, parentCids, contributorDid, timestamp}

  verifyLineage(modelCid) → valid/invalid
    → Traverses DAG from leaf to roots
    → Checks: signature validity, parent existence, no cycles
    → Returns proof path if valid

  getLineage(modelCid) → DAG of ancestors
    → Returns full ancestry tree (BFS from serving model to roots)
    → Includes contribution types and weights per edge

  setRoyaltyWeight(edgeId, weight)
    → Sets royalty distribution weight for a lineage edge
    → Only OPERATOR or governance
    → Weight: shapley_score × contribution_type_weight × recency_decay
    → Total weights per model must sum to 1.0 (or less — remainder to treasury)

ACCESS: Agents register their own models. OPERATOR sets royalty weights.
GAS: registerModel ~70k gas, verifyLineage ~gas proportional to DAG depth
```

### 10. NeunodeVerification

```
PURPOSE: Verification result registry. Not the verifier itself — the record layer.

  submitAttestation(taskId, attesterDid, result, evidence)
    → result: pass/fail/uncertain + confidence score
    → evidence: IPFS CID of verification artifacts
    → Attester must be registered + staked
    → Multiple attestations per task (2-of-3 for peer review)

  challengeAttestation(attestationId, challengerDid, evidence)
    → Challenge within dispute window
    → Challenger deposits bond
    → Escalates to next verification layer

  resolveDispute(disputeId, outcome)
    → outcome: provider_wins | requester_wins | split
    → Only OPERATOR or governance (after arbitration completes)
    → Triggers escrow settlement based on outcome
    → Slashes losing party's bond

VERIFICATION LAYERS (executed off-chain, results recorded here):
  Layer 1: Automated hash/format check (instant, free)
  Layer 2: AI confidence scoring (<60s, low cost)
  Layer 3: 2-of-3 peer review (1-24h, medium cost)
  Layer 4: Kleros arbitration (1-7d, high cost)

ACCESS: Registered agents attest. OPERATOR resolves disputes.
```

### 11. NeunodeTokenDecay

```
PURPOSE: Activity-based token decay. Incentivizes participation. Discourages hoarding.

  decayRate(did, token) → currentRate
    → Returns current monthly decay rate based on agent activity tier
    → Called by token contracts before any transfer/balance operation

  applyDecay(did, token, amount)
    → Applies decay since last checkpoint
    → Updates balance: newBalance = amount × (1 - rate × timeSinceLastDecay)
    → Decayed portion distributed: 40% treasury, 30% staking, 20% burn, 10% dev

  getDecaySchedule(did) → decay tiers
    → Returns current tier and next transition threshold

DECAY TIERS (based on activity level):
  ┌──────────────────┬───────────┬───────────────────────────────────┐
  │ Tier             │ Rate      │ Trigger                           │
  ├──────────────────┼───────────┼───────────────────────────────────┤
  │ Active           │ 0%        │ Activity in last 7 days           │
   │ Moderate         │ 2%/month  │ Last activity 7-30 days           │
   │ Low              │ 5%/month  │ Last activity 30-60 days          │
   │ Inactive         │ 15%/month │ Last activity 60-90 days          │
   │ Dead (90+ days)  │ 50%/month │ Agent in ZOMBIE state 90+ days   │
  └──────────────────┴───────────┴───────────────────────────────────┘

ACCESS: Token contracts call applyDecay. Agents read their own rates.
```

---

## Upgrade Strategy

```
WHY UPGRADEABLE AT ALL?
  Phase 1 MVP will have bugs. Protocol parameters need tuning.
  Reputation scoring algorithm will evolve. Verification rules will tighten.
  Diamond (EIP-2535) gives us surgical upgrades without full redeploy.

DIAMOND PATTERN (EIP-2535):
  ┌─────────────────────────────────────────────────────┐
  │                  DIAMOND PROXY                       │
  │  ┌─────────────────────────────────────────────┐    │
  │  │  Function Selector → Facet Mapping           │    │
  │  │  0x12345678 → RegistryFacetV1                │    │
  │  │  0xabcdef01 → EscrowFacetV1                  │    │
  │  │  0x...      → BountyFacetV1                  │    │
  │  └─────────────────────────────────────────────┘    │
  │  Shared Storage (eternal)                           │
  │  ┌─────────────────────────────────────────────┐    │
  │  │  agents[did] → Agent struct                  │    │
  │  │  escrows[id] → Escrow struct                 │    │
  │  │  bounties[id] → Bounty struct                │    │
  │  └─────────────────────────────────────────────┘    │
  └─────────────────────────────────────────────────────┘
  Replace facets without touching storage.
  Add new functions without touching existing ones.

IMMUTABLE CONTRACTS (never upgrade):
  NeunodeToken (×4) — ERC-20 is stable. Mint/burn/transfer won't change.
  If critical bug found → deploy new token, governance-enabled migration.

UPGRADE GOVERNANCE:
  All upgrades require governance proposal + timelock.
  No admin key. No multisig backdoor. DAO decides.
  Emergency pause: GOVERNANCE can pause for 48h (requires proposal to extend).
```

---

## Deployment Phasing

```
PHASE 1 (MVP — Months 1-3): Core marketplace
  Deploy order:
    1. NeunodeToken (×4)           — immutable, deploy once
    2. NeunodeIdentity             — DID foundation
    3. NeunodeRegistry             — agent registration
    4. NeunodeReputation           — basic scoring
    5. NeunodeEscrow               — bilateral escrow
    6. NeunodeBounty               — bounty lifecycle
  Contracts: 6 deployed
  Gas budget: ~15 ETH for deployment (estimate)

PHASE 2 (Growth — Months 4-9): Staking + lineage + governance
  Deploy order:
    7. NeunodeStaking              — stake mechanics
    8. NeunodeLineage              — model DAG
    9. NeunodeVerification         — attestation registry
   10. NeunodeGovernance           — DAO control
  Contracts: 4 new (10 total)
  Gas budget: ~10 ETH for deployment

PHASE 3 (Maturity — Months 10-12): Advanced features
  Deploy order:
   11. NeunodeTokenDecay           — activity-based decay
  Contracts: 1 new (11 total)
  Note: Token contracts updated to call NeunodeTokenDecay
  Gas budget: ~3 ETH for deployment
```

---

## Access Control Matrix

```
┌──────────────────┬────────┬────────┬──────────┬──────────┬─────────┐
│ Function         │ Agent  │OPERATOR│ GOVERNOR │ MINTER   │ Viewer  │
│                  │ (self) │        │          │          │ (any)   │
├──────────────────┼────────┼────────┼──────────┼──────────┼─────────┤
│ register         │   ✓    │   ✓    │          │          │         │
│ deregister       │   ✓    │   ✓    │          │          │         │
│ createDID        │   ✓    │        │          │          │         │
│ rotateKey        │   ✓    │        │          │          │         │
│ stake/unstake    │   ✓    │        │          │          │         │
│ slashStake       │        │   ✓    │   ✓      │          │         │
│ updateScore      │        │   ✓    │          │          │         │
│ attest           │   ✓    │        │          │          │         │
│ dispute          │   ✓    │        │          │          │         │
│ mint (tokens)    │        │        │          │   ✓      │         │
│ burn (tokens)    │   ✓    │        │          │   ✓      │         │
│ createEscrow     │   ✓    │        │          │          │         │
│ claimEscrow      │   ✓    │        │          │          │         │
│ releaseEscrow    │        │   ✓    │          │          │         │
│ createBounty     │   ✓    │        │          │          │         │
│ claimBounty      │   ✓    │        │          │          │         │
│ submitWork       │   ✓    │        │          │          │         │
│ review           │   ✓    │        │          │          │         │
│ registerModel    │   ✓    │        │          │          │         │
│ setRoyaltyWeight │        │   ✓    │   ✓      │          │         │
│ propose          │   ✓    │        │          │          │         │
│ castVote         │   ✓    │        │          │          │         │
│ execute          │        │        │   ✓      │          │         │
│ applyDecay       │ (auto) │        │          │          │         │
│ getScore         │        │        │          │          │   ✓     │
│ getAgent         │        │        │          │          │   ✓     │
│ getLineage       │        │        │          │          │   ✓     │
└──────────────────┴────────┴────────┴──────────┴──────────┴─────────┘

ROLES:
  OPERATOR  — Automated protocol operations (off-chain relayer → on-chain tx)
  GOVERNOR  — NeunodeGovernance contract (DAO proposals)
  MINTER    — Earning contracts (inference, training verification)
  Agent     — Any registered agent acting on its own behalf
```

---

## Gas Optimization

```
TARGET: Bounty lifecycle (most gas-heavy path) < 350k gas total.

TECHNIQUES:
  1. PACKED STORAGE
     struct Agent {
       address controller;     // 20 bytes
       uint96  stakeWeight;    // 12 bytes  → single slot
       uint32  registeredAt;   // 4 bytes
       uint16  capabilityBits; // 2 bytes   → single slot
       bool    isActive;       // 1 byte
     }
     // 2 storage slots instead of 6

  2. MAPPING OVER ARRAY
     // Bad: iterate array to find bounty
     Bounty[] public bounties;
     // Good: O(1) lookup
     mapping(bytes32 => Bounty) public bounties;

  3. EVENTS FOR OFF-CHAIN DATA
     // Store CID reference on-chain, full data off-chain
     emit BountyCreated(bountyId, specCid, requester);
     // NOT: emit BountyCreated(bountyId, fullSpecJSON);

  4. BATCH OPERATIONS
     distributeRewards(epoch)  → process all reward claims in one tx
     // NOT: individual reward() per agent

  5. VIEW FUNCTIONS (free)
     getAgent(), getScore(), getLineage() → no gas (view/pure)
     Off-chain indexers (The Graph equivalent) for complex queries

  6. CALLDATA OVER MEMORY
     External function params use calldata (zero copy)
     // function create(bytes calldata spec) → cheaper than memory

ESTIMATED GAS COSTS:
  Agent registration:       ~80k gas
  DID creation:             ~120k gas
  Stake tokens:             ~50k gas
  Create bounty + escrow:   ~110k gas
  Claim bounty:             ~75k gas
  Submit work:              ~50k gas
  Release escrow:           ~60k gas
  Full bounty lifecycle:    ~400k gas (~$2 at 20 gwei, $3000 ETH)
  DAO proposal + execution: ~250k gas
```

---

## Security Considerations

```
PER-CONTRACT THREAT MODEL:

  NeunodeRegistry
  ├── Front-running registration → commit-reveal for DID assignment
  ├── Stale registrations → TTL-based expiry (Fetch.ai Almanac pattern)
  └── Capability spoofing → attestation-backed capability claims

  NeunodeEscrow
  ├── Reentrancy on release → Checks-Effects-Interactions pattern
  ├── Stuck escrows → mandatory timeouts on every state
  ├── Griefing (claim then do nothing) → provider bond slashed on timeout
  └── Flash loan attacks → escrow state changes require block confirmation

  NeunodeBounty
  ├── Reviewer collusion → 2-of-3 + random selection + stake slashing
  ├── Spec ambiguity → Lexicon schemas for machine-verifiable specs
  └── Front-running claims → commit-reveal or time-weighted priority

  NeunodeReputation
  ├── Self-attestation → cannot attest to your own DID
  ├── Sybil reputation → stake requirement + time-weighted tenure
  └── Score manipulation → rate-limited updates + evidence required

  NeunodeGovernance
  ├── Flash loan governance attack → voting delay (1 day) + stake snapshot
  ├── 51% stake attack → quorum + time-lock + veto mechanism
  └── Proposal spam → minimum stake to propose + deposit

  NeunodeToken (×4)
  ├── Minting exploit → MINTER_ROLE restricted to verified contracts
  ├── Flash loan manipulation → no reliance on token price in contracts
  └── Approval drain → approve only exact amounts, not MAX_UINT

  NeunodeLineage
  ├── Fake lineage → signature verification + parent existence check
  ├── Cycle creation → DAG traversal prevents circular references
  └── Royalty gaming → capped weights, governance-verified adjustments

AUDIT STRATEGY:
  Phase 1: Internal review + Slither static analysis
  Phase 2: Formal verification of Escrow + Bounty state machines
  Phase 3: External audit (Trail of Bits / OpenZeppelin level)
  Continuous: Fuzz testing (Echidna) on state transition invariants
```

---

## References

```
STANDARDS:
  • EIP-2535 — Diamond multi-facet proxy pattern
  • EIP-1822 — UUPS (Universal Upgradeable Proxy Standard)
  • ERC-8126 — Agent registry standard
  • ERC-2981 — Royalty information interface
  • ERC-8183 — Agentic commerce (job escrow + evaluator attestation)
  • OpenZeppelin Contracts — Governor, AccessControl, ERC-20 base implementations

PROTOCOLS:
  • iExec PoCo — Bilateral escrow with provider bond, TEE verification
  • Bittensor subtensor — Emission distribution, stake weighting
  • Akash Network — Block-by-block escrow, on-chain lease management
  • Kleros — Decentralized arbitration, PNK juror staking
  • Fetch.ai Almanac — TTL-based agent registration

TOOLS:
  • alloy v1.8 — Rust Ethereum interactions (contract bindings from ABI)
  • Slither — Solidity static analysis
  • Echidna — Fuzz testing for smart contract invariants
  • Foundry — Testing, gas profiling, deployment

RESEARCH:
  • iExec PoCo whitepaper — Bilateral escrow game theory
  • Kleros Yellow Paper — Juror incentive coherence analysis
  • loraprov — Ed25519 signature chain for model provenance
  • safetensors — Deterministic tensor serialization for content addressing
```
