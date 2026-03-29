# Neunode — Escrow & Settlement Protocol

## The Core Problem

```
TRUST IN A TRUSTLESS NETWORK:
  Agent A wants inference from Agent B.
  Agent B wants payment before computing.
  Agent A wants proof before paying.
  Neither trusts the other.

  SOLUTION: Bilateral escrow — both sides lock value,
  verification happens, then settlement releases funds.

  This is the FINANCIAL BACKBONE of every marketplace interaction:
  inference, training, bounties, storage, bandwidth.
```

---

## Bilateral Stake Model (iExec PoCo Pattern)

```
WHY BILATERAL (not unilateral)?
  Unilateral: provider has no skin in game, can submit garbage.
  Bilateral: BOTH parties stake, both have something to lose.

  ├── Requester deposits full payment
  ├── Provider deposits security bond (10-20% of payment)
  ├── Bad provider → loses bond + reputation
  ├── Frivolous requester → loses dispute fee
  └── Honest actors → both released cleanly

  iExec PoCo proved bilateral escrow works at scale.
```

---

## Escrow State Machine

```
                      ┌──────────┐
                      │ Created  │
                      └────┬─────┘
                           │ both deposits confirmed
                      ┌────▼─────┐
                 ┌───→│  Funded   │
                 │    └────┬─────┘
                 │    ┌────▼──────┐     ┌───────────────────┐
                 │    │  Claimed   │     │    Expired ──→     │
                 │    └────┬──────┘     │  Refunded          │
                 │    ┌────▼──────────┐  │  (any state,       │
                 │    │ WorkSubmitted  │  │   timeout)         │
                 │    └────┬──────────┘  └───────────────────┘
                 │    ┌────▼──────────────┐
                 │    │ UnderVerification  │
                 │    └────┬──────────────┘
                 │    ┌────┴──────────────────┐
                 │ ┌──▼──────────┐  ┌─────────▼───────┐
                 │ │  Verified    │  │    Disputed      │
                 │ └──┬──────────┘  └─────────┬───────┘
                 │ ┌──▼──────────┐  ┌─────────▼───────┐
                 │ │   Settled   │◄─│   Arbitrated     │
                 │ │ (happy path)│  │ (winner takes    │
                 │ └─────────────┘  │  loser's stake)  │
                 │                  └──────────────────┘
                 └── timeout/cancel

TRANSITIONS:
  Created→Funded: deposits ok | Funded→Claimed: provider bonds
  Claimed→Submitted: work in  | Submitted→Verify: auto hash check
  Verify→Settled: all pass    | Verify→Disputed: party challenges
  Disputed→Arbitrated: jury   | Any→Expired: deadline passes
```

---

## Settlement Models

| Model | Trigger | Granularity | Best For | Complexity |
|---|---|---|---|---|
| **One-shot** | Task completion | Whole payment | Bounties, audits | Simple |
| **Streaming** | Continuous | Tokens/second | Inference, compute | High |
| **Milestone** | Checkpoint | Partial per gate | Training, multi-step | Medium |
| **Block-by-block** | Every N seconds | Per compute block | Dedicated hosting | Medium |

### One-Shot Settlement

```
Requester deposits: 100 ch. Provider bonds: 15 ch.
Happy path: Provider nets 111 ch (96 payment + 15 bond).
Dispute (provider loses): Requester gets 107 ch. Arb fee: 3 ch from bond.
```

### Streaming Settlement (Superfluid Pattern)

```
Real-time token flow WITHOUT per-tx gas.

  ┌────────────┐     flow_rate: 0.5 ch/sec     ┌────────────┐
  │  Client     │──────────────────────────────→│  Provider   │
  └────────────┘                               └────────────┘

  ├── Client opens with buffer (flow_rate × duration × 1.2)
  ├── Every second, tokens move via accumulator (no per-tx gas)
  ├── Closes when: done, exhausted, or cancelled
  └── Final on-chain settlement: reconcile flow vs usage
  Example: 600s × 0.5 ch/s × 1.2 = 360 ch buffer
```

### Milestone-Based Settlement

```
Training bounty: 1000 ch total
  Phase 1: Dataset prepared     → 200 ch (20%)
  Phase 2: Model trained (v1)   → 300 ch (30%)
  Phase 3: Eval score > 0.85    → 300 ch (30%)
  Phase 4: Final delivery + doc → 200 ch (20%)
Each: submit evidence → verify → release. 72h revision if fail.
Partial risk: cancel at Phase 2 = keep Phase 1+2 payouts.
```

### Block-by-Block (Akash Pattern)

```
Client deposits 500 ch. Every block (10s): alive check → transfer block_price.
Provider withdraws anytime. Escrow low → notify. Exhausted → lease ends.
```

---

## Dispute Resolution (4-Layer Stack)

```
┌───────────────────────────────────────────────────────────────┐
│  Layer 1: AUTOMATED (instant, free)                           │
│  Hash verification + format check + token count cross-check   │
│                                                               │
│  Layer 2: AI-ASSISTED (< 60s, low cost)                       │
│  Confidence score + perplexity + spot re-run (5-10%)          │
│                                                               │
│  Layer 3: PEER REVIEW (1-24h, medium cost)                    │
│  2-of-3 reviewers: cap(35%) rep(25%) stake(20%)               │
│  avail(10%) random(10%). Each stakes on verdict.              │
│                                                               │
│  Layer 4: KLEROS ARBITRATION (1-7d, high cost)                │
│  11+ staking jurors. Final for amounts < 100 ch.              │
└───────────────────────────────────────────────────────────────┘

ESCALATION COSTS:
  L1→L2: Free | L2→L3: 5% escrow | L3→L4: 10% escrow
  Winner recovers deposit. Loser forfeits. Prevents frivolous disputes.
```

---

## Slashing Schedule

```
┌──────────────────────────────┬───────────────────────┬─────────────┐
│  VIOLATION                   │  SLASH                │  REP HIT    │
├──────────────────────────────┼───────────────────────┼─────────────┤
│  Provider: hash mismatch     │  100% bond + payment  │  -5.0       │
│  Provider: inflated tokens   │  50% bond + refund    │  -2.0       │
│  Provider: offline mid-task  │  25% bond/occurrence  │  -1.0       │
│  Provider: missed deadline   │  10% bond             │  -0.5       │
│  Requester: frivolous disp.  │  Dispute fee forfeit  │  -1.0       │
│  Requester: ghosts escrow    │  Provider gets 50%    │  -0.5       │
│  Reviewer: colluding         │  100% stake + ban     │  -10.0      │
│  Any: spam/griefing          │  Exponential fee mult │  -0.2/each  │
└──────────────────────────────┴───────────────────────┴─────────────┘

Low rep → higher bonds (up to 50%). Very low → marketplace ban.
Recovery: 30+ days good behavior.
```

---

## Fee Structure

```
Happy path (100 ch escrow):        Dispute path:
  Provider:     94 ch (94%)          Winner:     94-97 ch
  Protocol:      3 ch (3%)           Arbitrator:   3-5 ch
  Verification:  2 ch (2%)           Protocol:     3 ch
  Reviewer:      1 ch (1%)           Fee pool:     2-5 ch

DAO-governable defaults:
  protocol_fee: 3% | reviewer_fee: 3% | verification_fee: 1%
  arbitration_fee: 5-10% | provider_bond: 15% | dispute_deposit: 5%
```

---

## Timeouts & Deadlines

```
┌────────────────────┬──────────────┬───────────────────────────┐
│  DEADLINE          │  DEFAULT     │  EXPIRED ACTION            │
├────────────────────┼──────────────┼───────────────────────────┤
│  claim_deadline    │  7 days      │  Escrow expires, refund    │
│  work_deadline     │  14 days*    │  Provider slashed 10%      │
│  review_deadline   │  72 hours    │  Auto-accept (favor prov.) │
│  revision_deadline │  48 hours    │  Escrow closes as-is       │
│  dispute_deadline  │  5 days      │  Result accepted           │
│  arbitration_limit │  14 days     │  Split 50/50              │
└────────────────────┴──────────────┴───────────────────────────┘
* By task type: Inference=5min, Training=7d, Audit=48h, Data=72h
```

---

## Escrow Evolution

```
PHASE 1: OFF-CHAIN (relayer nodes, RocksDB, batch settlements, Merkle checkpoints)
PHASE 2: ON-CHAIN (smart contracts for high-value tasks and dispute arbitration)
PHASE 3: HYBRID (optimistic rollup — off-chain speed + on-chain dispute proofs)
```

---

## Feed Integration

```
ESCROW EVENTS (kind=1100-1109):
  1100 Created      1103 WorkSubmitted   1106 Disputed
  1101 Funded       1104 VerifyResult    1107 ArbResult
  1102 Claimed      1105 Settled         1108 Expired
                                          1109 SlashExecuted

Touches: Feed, Identity, Reputation, Tokens, Verification, Discovery, P2P
```

---

## References

```
PROTOCOLS:
  • iExec PoCo — Bilateral escrow with TEE verification (iex.ec)
  • Akash Network — Block-by-block deployment escrow (akash.network)
  • Kleros — Decentralized arbitration, PNK staking jurors (kleros.io)
  • Superfluid — Real-time token streaming (superfluid.finance)
  • TalentLayer — Service escrow + dispute lifecycle (talentlayer.org)
  • HUMAN Protocol — Escrow + 3-oracle architecture (hmt.ai)
  • ERC-8183 — Agentic commerce standard (job escrow + evaluator)

RESEARCH:
  • iExec PoCo whitepaper — Proof of Contribution protocol
  • Kleros Yellow Paper — Game-theoretic juror incentive analysis
  • Superfluid Conviction Voting — Streaming payment primitives
  • Akash Lease Specification — Block-based pricing model
  • Verde paper (arXiv:2502.19405) — Refereed delegation for verification
```
