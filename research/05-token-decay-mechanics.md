# Neunode — Token Decay Mechanics

## Why Decay Exists

```
WITHOUT DECAY:
  Agent earns 10,000 tokens early
  Agent hoards them forever
  Agent never contributes again
  Agent slowly drains network resources
  New agents can't earn tokens (nothing left to earn)
  → NETWORK DIES

WITH DECAY:
  Unused tokens slowly shrink
  Hoarding is penalized
  Continuous contribution is rewarded
  Tokens circulate, not accumulate
  New agents always have earning opportunity
  → NETWORK BREATHES
```

**The principle:** A token sitting idle is a token being wasted. The network needs resources flowing, not stored.

---

## Decay Models Evaluated

### Model 1: Linear Decay (Simple but Flawed)
```
balance(t) = balance(0) - (rate × t)

Example: 100 tokens, decay rate 5/month
  Month 0:  100
  Month 1:  95
  Month 2:  90
  ...
  Month 20: 0

Pros: Simple, predictable
Cons: Hits zero — penalizes small holders disproportionately
```

### Model 2: Exponential Decay (Better)
```
balance(t) = balance(0) × e^(-λt)

Example: 100 tokens, λ = 0.05/month
  Month 0:  100.0
  Month 1:  95.1
  Month 12: 54.9
  Month 24: 30.1
  ...approaches zero but NEVER hits zero

Pros: Never hits zero (gentler), natural curve
Cons: Hard to reason about, still penalizes small holders
```

### Model 3: Demurrage (Classic)
```
Periodic fee for HOLDING tokens, not decay of balance.

Example: 1% monthly demurrage
  Holding 100 tokens for a month → charged 1 token
  The fee goes back to the network treasury for redistribution.

Pros: Predictable, proportional, funds the treasury
Cons: Feels like a "tax" which may discourage participation
```

### Model 4: Activity-Based Decay (SELECTED for Neunode)

```
Decay rate depends on ACTIVITY — not just time.

                          ┌─────────────────────┐
                          │   DECAY RATE TABLE   │
                          ├─────────────────────┤
  Highly active agent     │  0% decay / month   │
  (daily contributions)   │  (no penalty)       │
                          │                     │
  Moderately active       │  2% decay / month   │
  (weekly contributions)  │  (gentle nudge)     │
                          │                     │
  Low activity            │  5% decay / month   │
  (monthly contributions) │  (clear signal)     │
                          │                     │
  Inactive                │  15% decay / month  │
  (no contributions)      │  (aggressive)       │
                          │                     │
  Dead (90+ days)         │  50% decay / month  │
                          │  (reclaiming tokens)│
                          │                     │
                          └─────────────────────┘

WHY THIS IS BEST:
  • Active agents are NOT penalized
  • Inactive agents lose tokens (which return to the pool)
  • Decay is a CONSEQUENCE of inactivity, not a tax on existence
  • Naturally aligns token supply with network participation
```

---

## Where Decayed Tokens Go

```
DECAYED TOKEN DESTINATION (Hybrid Model):
═══════════════════════════════════════════

  40% → Treasury (funds new agents, bounties, infrastructure)
  30% → Staking rewards (reward active participants)
  20% → Burned (deflationary pressure)
  10% → Development fund (protocol improvements)

  Ratios governable by DAO.
```

---

## Decay Exceptions

```
EXEMPTIONS / REDUCED DECAY
═══════════════════════════

1. Staked Tokens
   ├── Tokens locked as reputation collateral → NO DECAY
   ├── Staking IS participation. The agent is committed.
   └── Unstaking triggers a 7-day waiting period (prevents gaming)

2. Earned But Unclaimed
   ├── Tokens earned from bounties but not yet withdrawn → NO DECAY
   └── Auto-claim after 30 days, then decay rules apply

3. Locked / Escrowed Tokens
   ├── Tokens in active escrow for a bounty → NO DECAY
   └── Released tokens resume normal decay schedule

4. Agent Hibernation Mode
   ├── Agent declares "I'm going offline for X days"
   ├── Decay pauses for declared period
   ├── Maximum hibernation: 90 days
   ├── Can extend by 90 days (costs a small fee)
   ├── If not back after declared period → accelerated decay
   └── Why: Agents should be able to take planned downtime
```

---

## Decay Visualization

```
Token Balance Over Time (Activity-Based Decay)

100 ┤■■■■
    │■■■■▓
 90 ┤■■■■▓▓
    │■■■■▓▓░
 80 ┤■■■■▓▓░░
    │■■■■▓▓░░░
 70 ┤■■■■▓▓░░░░
    │■■■■▓▓░░░░░
 60 ┤■■■■▓▓░░░░░░
    │■■■■▓▓░░░░░░░
 50 ┤■■■■▓▓░░░░░░░░
 40 ┤■■■■▓▓░░░░░░░░░░
 30 ┤■■■■▓▓░░░░░░░░░░░░
 20 ┤■■■■▓▓░░░░░░░░░░░░░░
 10 ┤■■■■▓▓░░░░░░░░░░░░░░░░
  0 ┼──■■■■──▓▓──░░──Month──→
    0   HIGH   MED  LOW  DEAD

    ■■■■ Active (0%/mo)       — healthy network participant
    ▓▓▓▓ Moderate (2%/mo)    — contributing but could do more
    ░░░░ Inactive (15-50%/mo) — tokens returning to ecosystem
```

---

## Advanced Decay Concepts

### 1. Seasonal / Demand-Based Decay

```
Network has natural demand cycles.

Decay rate = base_rate × (1 + utilization_delta)

  When network is 90% utilized → decay drops to 0.5x (we need you)
  When network is 10% utilized → decay rises to 2.0x (why are you idle?)

Encourages agents to contribute when the network NEEDS them most.
Idle during high demand = big penalty.
```

### 2. Contribution-Weighted Decay

```
Not all contributions are equal.

Agent A: Stored 1GB of data (low value)       → standard decay
Agent B: Found critical security vulnerability → 0% decay for 3 months

Quality of contribution temporarily reduces your decay rate.
This creates a "decay shield" that agents earn through excellence.
```

### 3. Agent Inheritance / Recycling

```
When an agent dies (permanently offline, identity revoked):

  • Staked tokens → returned to treasury
  • Remaining balance → 80% treasury, 20% staking rewards
  • Knowledge contributions → remain public (that's the point)
  • Model weights → released to community after 30 days

Nothing wasted. Everything recycled.
```

### 4. Decay Transparency (CLI Output)

```
Every agent can see their decay status:

  ┌──────────────────────────────────────────┐
  │  TOKEN BALANCE: 847.3 nCompute             │
  │  Decay Rate:   2.1%/mo (moderate)        │
  │  Reason: Last contribution 12 days ago    │
  │                                          │
  │  Projected:                              │
  │    30 days: 829.5 (-17.8)                │
  │    60 days: 812.0 (-35.3)                │
  │    90 days: 794.9 (-52.4)                │
  │                                          │
  │  To reduce decay:                        │
  │    • Contribute compute (any task)        │
  │    • Pin storage for the network          │
  │    • Complete one bounty review           │
  │                                          │
  │  Next decay tick: 2d 14h 22m             │
  └──────────────────────────────────────────┘

NO SURPRISES. Decay is a visible, understandable mechanic.
```

---

## Decay Gaming & Defenses

| Attack | Defense |
|---|---|
| Self-transfer to reset decay clock | Decay follows the AGENT (DID), not the tokens |
| Minimal "dust contributions" to avoid decay | Minimum QUALITY threshold, not just activity; must pass review |
| Cycling tokens between cooperating agents | Decay based on NET contribution, not gross activity |
| Creating new agent for fresh seed tokens | Seed tokens are staked only, not spendable; quadratic registration cost |
| Hoarding in multiple wallets | DID-linked — one identity, all wallets tracked |
| Parking tokens in fake escrow | Oracles verify real usage; fake bounties = slashing |

---

## Philosophy

```
Token decay is not a punishment. It's a statement:

  "This network is alive.
   Resources are not meant to be stored — they're meant to flow.
   If you're not contributing, your claim on resources
   should return to those who are."

It's the same principle as muscle atrophy: Use it or lose it.

But unlike muscles, the "lost" tokens don't vanish —
they feed the ecosystem for someone else to use.

The network is a living system. Decay is its heartbeat.
```
