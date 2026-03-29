# Neunode — DAO Governance Design

## The Core Problem

```
A DECENTRALIZED NETWORK NEEDS DECENTRALIZED DECISIONS.

  Who sets the protocol fee?        Currently: nobody.
  Who approves new capability types? Currently: nobody.
  Who handles slashing appeals?      Currently: nobody.
  Who pauses during an exploit?      Currently: nobody.

  Without governance:
  ├── Protocol parameters frozen at launch values (brittle)
  ├── No mechanism to correct economic imbalances
  ├── No way to appeal wrongful slashing
  ├── Emergency response requires a centralized admin (defeats the point)
  └── Network cannot evolve beyond its initial design

  SOLUTION: Reputation-weighted DAO governance.
  Those who contribute the most get the most say.
  Sybil-resistant via quadratic voting + reputation multipliers.
  Emergency powers exist but are time-locked and ratifiable.
```

---

## Governance Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                     NEUNODE GOVERNANCE STACK                        │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  GOVERNOR CONTRACT (OpenZeppelin Governor-based)              │  │
│  │  ├── propose() → vote() → queue() → execute()                │  │
│  │  ├── Reputation-weighted voting power                         │  │
│  │  ├── Quadratic component (Sybil resistance)                  │  │
│  │  └── Trust-weighted delegation chains (Bittensor-inspired)    │  │
│  └───────────────────────────────────────────────────────────────┘  │
│         │                    │                    │                   │
│  ┌──────▼──────┐    ┌───────▼───────┐    ┌──────▼──────────────┐   │
│  │  TIMELOCK   │    │  EMERGENCY    │    │  ARBITRATION        │   │
│  │  (2-day     │    │  MULTISIG     │    │  (Kleros-style      │   │
│  │   delay)    │    │  (3-of-5)     │    │   peer jurors)      │   │
│  └─────────────┘    └───────────────┘    └─────────────────────┘   │
│         │                    │                    │                   │
│  ┌──────▼────────────────────▼────────────────────▼──────────────┐  │
│  │                      TREASURY                                  │  │
│  │  Sources: decay(40%), fees(2-5%), slashing proceeds           │  │
│  │  Uses: dev fund, audits, incentives, grants                   │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Governance Frameworks Analyzed

```
┌──────────────────┬──────────────────────────────┬──────────────────────┐
│ Framework        │ Key Mechanism                │ What Neunode Adopts  │
├──────────────────┼──────────────────────────────┼──────────────────────┤
│ OpenZeppelin     │ propose→vote→queue→execute   │ Core lifecycle       │
│ Governor         │ ERC-20 weighted votes        │ (Governor contract)  │
├──────────────────┼──────────────────────────────┼──────────────────────┤
│ Compound         │ Delegatable voting, proposal  │ Delegation chains    │
│ Governor Bravo   │ threshold, vote reasons      │ + proposal threshold │
├──────────────────┼──────────────────────────────┼──────────────────────┤
│ Uniswap          │ Quadratic voting, timelocked  │ Quadratic component  │
│ Governance       │ execution, 2% quorum         │ + low quorum model   │
├──────────────────┼──────────────────────────────┼──────────────────────┤
│ Bittensor        │ Yuma Consensus: trust-weighted│ Reputation multiplier│
│ Yuma Consensus   │ delegation, 18%/41%/41% split│ + trust delegation   │
├──────────────────┼──────────────────────────────┼──────────────────────┤
│ Aragon           │ DAO apps, ACL, forwarding     │ Modular governance   │
│                  │                              │ actions pattern      │
├──────────────────┼──────────────────────────────┼──────────────────────┤
│ Kleros           │ Token-curated jurors,         │ Slashing appeals &   │
│                  │ coherent vote incentives      │ dispute arbitration  │
├──────────────────┼──────────────────────────────┼──────────────────────┤
│ Aragora (hybrid) │ Aragon structure + Bittensor  │ FULL INSPIRATION:    │
│                  │ trust-weighted voting         │ this is our model    │
└──────────────────┴──────────────────────────────┴──────────────────────┘
```

---

## Proposal Types

```
┌─────┬──────────────────────────┬────────────────────────────────────────┐
│ ID  │ Type                     │ Examples                               │
├─────┼──────────────────────────┼────────────────────────────────────────┤
│ P01 │ Protocol Parameter       │ Decay rates, fee percentages, quorum   │
│     │ Change                   │ thresholds, timeout values             │
├─────┼──────────────────────────┼────────────────────────────────────────┤
│ P02 │ Smart Contract Upgrade   │ Diamond proxy facet replacement,       │
│     │                          │ new module deployment                 │
├─────┼──────────────────────────┼────────────────────────────────────────┤
│ P03 │ Treasury Allocation      │ Fund grants, audit budget, incentive   │
│     │                          │ programs, infrastructure subsidies     │
├─────┼──────────────────────────┼────────────────────────────────────────┤
│ P04 │ Slashing Appeal          │ Overturn a slash, refund bond,         │
│     │                          │ restore reputation score               │
├─────┼──────────────────────────┼────────────────────────────────────────┤
│ P05 │ New Capability           │ Register a capability type in the      │
│     │ Registration             │ agent ontology (e.g., "video_gen")    │
├─────┼──────────────────────────┼────────────────────────────────────────┤
│ P06 │ Network Fork             │ Create testnet variant, parameter      │
│     │                          │ experiment with isolated state         │
├─────┼──────────────────────────┼────────────────────────────────────────┤
│ P07 │ Emergency Pause          │ Circuit breaker — halt marketplace,    │
│     │ (via multisig)           │ freeze escrows, stop token transfers   │
└─────┴──────────────────────────┴────────────────────────────────────────┘
```

---

## Proposal Lifecycle

```
                         ┌─────────────┐
                         │   Created   │  (proposer stakes threshold)
                         └──────┬──────┘
                                │
                         voting_delay (1 day)
                                │
                         ┌──────▼──────┐
                    ┌───→│    Active    │◀── delegates can re-delegate
                    │    └──────┬──────┘   during this window
                    │           │
                    │    voting_period (7 days)
                    │           │
                    │    ┌──────┴──────┐
                    │    │             │
              ┌─────▼──┐       ┌──────▼───────┐
              │Canceled│       │   Succeeded   │  (votes > quorum, majority For)
              │(by     │       └──────┬───────┘
              │proposer│              │
              │only)   │       timelock_delay (2 days)
              └────────┘              │
                          ┌───────────▼───────────┐
                          │        Queued          │
                          └───────────┬───────────┘
                                  │
                          execution_delay (1 day)
                                  │
                      ┌───────────┴───────────┐
                      │                       │
                ┌─────▼──────┐         ┌──────▼──────┐
                │  Executed   │         │   Expired   │  (not executed in
                │ (on-chain   │         │ (timed out) │   execution_window)
                │  action)    │         └─────────────┘
                └────────────┘
```

---

## Voting Mechanism

```
VOTING POWER FORMULA:

  effective_votes = √(staked_tokens) × reputation_multiplier

  WHERE reputation_multiplier:
  ├── score < 4.0  → 1.0x  (base)
  ├── score ≥ 4.0  → 1.5x  (contributor)
  ├── score ≥ 4.5  → 2.0x  (expert)
  └── score ≥ 4.8  → 3.0x  (pillar of the network)

  WHY QUADRATIC + REPUTATION:
  ├── √(tokens): prevents whale dominance (100 tokens ≠ 100 votes)
  ├── rep multiplier: rewards actual contribution, not just capital
  ├── Combination: 10k tokens + rep 4.8 = 100 × 3.0 = 300 votes
  │                 10k tokens + rep 3.0 = 100 × 1.0 = 100 votes
  └── Same stake, 3x difference — contribution matters MORE than capital
```

### Delegation (Bittensor-Inspired Trust Chains)

```
AGENTS CAN DELEGATE VOTES TO TRUSTED AGENTS:

  Agent A (rep 4.2, 500 tokens) ──delegate──→ Agent B (rep 4.8, 2000 tokens)

  Agent B's total voting power:
  ├── Own:  √(2000) × 2.0 = 89.4
  ├── From A: √(500) × 1.5 = 33.5
  └── Total: 122.9 effective votes

  TRUST CHAINS (max depth 3, prevents circular delegation):
  Agent C → Agent A → Agent B → (Governor)
  Depth 3   Depth 2   Depth 1

  RULES:
  ├── Delegator retains tokens, only votes transfer
  ├── Delegation revocable anytime before vote cast
  ├── Max delegation depth: 3 hops
  ├── No circular delegation (DAG enforced)
  └── Delegated power does NOT compound reputation multiplier
      (Agent B uses their OWN rep multiplier for delegated stake)
```

---

## Key Governance Parameters

```
┌────────────────────────┬───────────┬───────────────────────────────────┐
│ Parameter              │ Default   │ Description                       │
├────────────────────────┼───────────┼───────────────────────────────────┤
│ voting_delay           │ 1 day     │ Blocks between creation & voting  │
│ voting_period          │ 7 days    │ Duration of active voting window  │
│ proposal_threshold     │ 100 ch    │ Min staked tokens to propose      │
│ quorum                 │ 4%        │ Of total staked tokens must vote  │
│ timelock_delay         │ 2 days    │ Delay between pass and queue      │
│ execution_delay        │ 1 day     │ Delay between queue and execute   │
│ execution_window       │ 14 days   │ Window to execute before expiry   │
│ max_delegation_depth   │ 3         │ Max hops in delegation chain      │
│ quorum_scale_factor    │ 1.0       │ Adjustable by governance          │
│ proposal_deposit       │ 50 ch     │ Slashed if proposal is invalid    │
└────────────────────────┴───────────┴───────────────────────────────────┘

ALL PARAMETERS ARE THEMSELVES GOVERNABLE (except emergency_multisig threshold).
```

---

## Emergency Governance

```
┌────────────────────────────────────────────────────────────────────┐
│                    EMERGENCY PROTOCOL                               │
│                                                                    │
│  TRIGGER: Critical security vulnerability, active exploit,         │
│           or network-threatening event                             │
│                                                                    │
│  STEP 1: MULTISIG ACTION (immediate)                               │
│  ├── 3-of-5 multisig signers (elected by governance)               │
│  ├── Can: pause marketplace, freeze escrows, halt token transfers  │
│  ├── Cannot: change parameters, access treasury, modify contracts  │
│  └── Action logged on-chain with reason + evidence hash            │
│                                                                    │
│  STEP 2: GOVERNANCE RATIFICATION (within 7 days)                   │
│  ├── Emergency proposal auto-created                               │
│  ├── Expedited voting: 3-day period (not 7)                        │
│  ├── Ratified → action continues, post-mortem published            │
│  └── Rejected → action reversed, multisig signers penalized        │
│                                                                    │
│  STEP 3: RESOLUTION (after ratification)                           │
│  ├── Fix deployed via governance upgrade (P02)                     │
│  ├── Pause lifted via governance proposal                          │
│  └── Post-mortem + parameter adjustments if needed                 │
│                                                                    │
│  SAFEGUARDS:                                                       │
│  ├── Multisig rotation every 90 days (governance vote)             │
│  ├── Auto-expiry: emergency pause lasts max 30 days                │
│  ├── Signer bond: 500 ch staked, slashed for unjustified pauses    │
│  └── Rate limit: max 2 emergency actions per 30-day window         │
└────────────────────────────────────────────────────────────────────┘
```

---

## Treasury Management

```
TREASURY INFLOWS:
  ┌──────────────────────┬─────────────┬──────────────────────────────┐
  │ Source               │ Est. Share  │ Trigger                     │
  ├──────────────────────┼─────────────┼──────────────────────────────┤
  │ Decay redistribution │ 40%         │ Continuous (token decay)     │
  │ Protocol fees        │ 2-5%        │ Per marketplace transaction  │
  │ Slashing proceeds    │ Variable    │ When provider/requester cut  │
  │ Expired escrows      │ Variable    │ Escrow timeout, no claimant  │
  │ Agent death (tokens) │ 80% of dead │ 90-day zombie → death        │
  └──────────────────────┴─────────────┴──────────────────────────────┘

TREASURY OUTFLOWS (governance-approved only):
  ┌──────────────────────┬─────────────┬──────────────────────────────┐
  │ Use                  │ Est. Share  │ Approval Required            │
  ├──────────────────────┼─────────────┼──────────────────────────────┤
  │ Development fund     │ 10%         │ Annual budget proposal       │
  │ Security audits      │ Per-audit   │ Individual P03 proposal      │
  │ Network incentives   │ 30%         │ Quarterly allocation         │
  │ Grants program       │ 20%         │ Rolling P03 proposals        │
  │ Infrastructure       │ 15%         │ Quarterly ops proposal       │
  │ Staking rewards      │ 30%         │ Automatic (smart contract)   │
  │ Emergency reserve    │ 10%         │ Multisig + ratification      │
  └──────────────────────┴─────────────┴──────────────────────────────┘

  SPENDING LIMITS (cannot exceed without super-quorum):
  ├── Single proposal: max 5% of treasury balance
  ├── Monthly total: max 15% of treasury balance
  └── Super-quorum (10%) required for anything above these caps
```

---

## Slashing Appeal Process

```
APPEAL FLOW (governance-mediated):

  Slashed Agent ─→ Submit Appeal (P04) ─→ Evidence Package
                                              │
                                    ┌─────────┴─────────┐
                                    │                   │
                              Verifiable Error    Legitimate Slash
                              (hash mismatch      (provider was
                               was a bug)          actually malicious)
                                    │                   │
                              ┌─────▼─────┐      ┌─────▼─────┐
                              │  Refund    │      │  Uphold    │
                              │  + restore │      │  + extra   │
                              │  reputation│      │  appeal fee│
                              └───────────┘      │  forfeited │
                                                 └───────────┘

  EVIDENCE REQUIREMENTS:
  ├── Original task input + expected output hash
  ├── Provider's submitted output + verification logs
  ├── Relevant feed events (kind=1100-1109)
  └── Optional: independent re-execution results

  APPEAL DEPOSIT: 10% of original slash amount (returned if appeal succeeds)
```

---

## Governance Feed Events

```
  kind=5000  GovernanceProposalCreated   {proposal_id, proposer_did, type, description}
  kind=5001  GovernanceVoteCast          {proposal_id, voter_did, support, weight}
  kind=5002  GovernanceProposalSucceeded {proposal_id, for_votes, against_votes}
  kind=5003  GovernanceProposalDefeated  {proposal_id, for_votes, against_votes}
  kind=5004  GovernanceProposalQueued    {proposal_id, execute_after}
  kind=5005  GovernanceProposalExecuted  {proposal_id, transaction_hash}
  kind=5006  GovernanceProposalCanceled  {proposal_id, reason}
  kind=5007  GovernanceDelegationChanged {delegator_did, delegate_did, previous}
  kind=5008  EmergencyAction             {action, signers[], reason_hash, expires}
  kind=5009  EmergencyRatified           {action, vote_result}
  kind=5010  TreasuryAllocation          {proposal_id, amount, recipient, purpose}
  kind=5011  SlashingAppealFiled         {appeal_id, original_slash, evidence_hash}
  kind=5012  SlashingAppealResolved      {appeal_id, outcome, reason}
```

---

## Phase Implementation

```
PHASE 1 (MVP):
  ├── Multisig (3-of-5) for emergency pause only
  ├── Off-chain governance (token-weighted signalling via feed)
  ├── Manual parameter changes by multisig (ratified by community)
  └── No on-chain governance contracts yet

PHASE 2:
  ├── OpenZeppelin Governor deployment
  ├── On-chain proposal lifecycle (propose→vote→queue→execute)
  ├── Reputation-weighted quadratic voting
  ├── Delegation chains
  └── Treasury smart contract with spending limits

PHASE 3:
  ├── Full Aragora model (Aragon structure + Bittensor trust weights)
  ├── Kleros integration for slashing appeals
  ├── Network fork governance (spin up testnet variants)
  ├── Automatic parameter adjustment (bounded, governance-overridable)
  └── Cross-network governance bridging
```

---

## References

```
GOVERNANCE FRAMEWORKS:
  • OpenZeppelin Governor — Standard ERC governance (docs.openzeppelin.com)
  • Compound Governor Bravo — Delegatable voting, proposal thresholds
  • Uniswap Governance — Quadratic voting, timelocked execution
  • Aragon — DAO framework with modular apps and ACL (aragon.org)

CONSENSUS & TRUST:
  • Bittensor Yuma Consensus — Trust-weighted delegation (subtensor source)
  • Aragora — Hybrid Aragon + Bittensor governance model

ARBITRATION:
  • Kleros — Decentralized arbitration, PNK staking jurors (kleros.io)
  • Kleros Yellow Paper — Game-theoretic juror incentive analysis

SMART CONTRACT PATTERNS:
  • ERC-2981 — Royalty standard (governance-controlled parameters)
  • Diamond Proxy (EIP-2535) — Upgradeable contracts via governance
  • TimelockController — Delayed execution for governance actions

RESEARCH:
  • Quadratic Voting — Lalley & Weyl, efficient collective decisions
  • Governance Attacks — Flash loan voting, barrage attacks, plutocracy
  • Vitalik on Governance — "Governance Minimization" philosophy
```
