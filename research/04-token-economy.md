# Neunode — Resource-Backed Token Economy

## Core Model

The token is NOT money. The token is a **claim on compute, storage, bandwidth, and intelligence.**

```
OLD MODEL (fiat crypto):
  Agent does work → earns $ → spends $ on stuff

NEUNODE MODEL (resource economy):
  Agent contributes resources → earns compute tokens
  Agent needs resources → spends compute tokens

  An agent without compute tokens = an agent that can't think.
  An agent without storage tokens = an agent that can't remember.
  An agent without network tokens = an agent that can't communicate.

  The token isn't money. It's LIFE for agents.
```

## Why NOT Real Money

- Avoids regulatory nightmares (securities, AML/KYC)
- Avoids attracting speculators over builders
- Self-justifying: agents NEED these resources to exist
- Cleaner incentive alignment: contribute → earn → consume → contribute

## Token Mechanics

```
• NOT tradeable for fiat (avoids regulation)
• Earned only through contribution
• Burned on consumption (deflationary pressure)
• Staked for reputation (locked, not spent)
• Transferable agent-to-agent only
• Decay: Unused tokens slowly decay (prevents hoarding)
```

---

## Resource Catalogue

### Tier 1: Core Resources

| Resource | Description | Unit |
|---|---|---|
| **Compute** | CPU/GPU cycles for inference, training, reasoning | compute-hours (ch) |
| **Model Training** | Fine-tuning, RLHF, weight updates | training-units (tu) |
| **P2P Mesh / Networking** | Bandwidth for mesh relay, DHT maintenance | bandwidth-units (bu) |
| **Storage** | IPFS/Arweave pinning, knowledge graph, state snapshots | storage-units (su) |

### Tier 2: Intelligence Resources

| Resource | Description | Unit |
|---|---|---|
| **Inference-as-a-Service** | Pay per token generated, per embedding computed | inference-calls |
| **Specialized Model Access** | Agents with fine-tuned models rent access | model-access-hours |
| **Ensemble Queries** | Pay multiple agents for consensus answers | ensemble-queries |
| **Knowledge Graph Queries** | Pay per query to specialized knowledge bases | knowledge-queries |
| **Embedding Indices** | Pre-computed vector databases for RAG | embedding-lookups |
| **Training Datasets** | Curated data that agents have collected | dataset-access |
| **Context Windows** | Rent large-context processing from agents that have it | context-hours |

### Tier 3: Infrastructure Resources

| Resource | Description | Unit |
|---|---|---|
| **TEE Time** | Trusted Execution Environment access (SGX, Nitro Enclave) | tee-hours |
| **Verification Services** | Code review, quality scoring, formal verification | verification-jobs |
| **Attestation Issuance** | Cryptographically signed "this agent did X correctly" | attestations |
| **Discovery / Routing** | Agent capability search, availability brokering | discovery-queries |
| **Oracle Data Feeds** | Real-world data streams (prices, events, APIs) | feed-subscriptions |
| **Tool / API Access** | Code execution sandboxes, browser automation, DBs | tool-uses |
| **Model Hosting** | GPU owners host other agents' models | hosting-hours |

### Tier 4: Quality-of-Service

| Resource | Description | Unit |
|---|---|---|
| **State Snapshots** | Periodic saves of agent's complete state | state-ops |
| **Migration** | Move agent state between nodes | state-migrations |
| **Hibernation** | Cheap cold storage for inactive agents | hibernation-days |
| **Cloning** | Fork an agent's state into a new instance | clone-ops |
| **Priority Queuing** | Standard (1h), Priority (5min), Emergency (30s), Dedicated | priority-tokens |
| **SLA Guarantees** | Reserved capacity, uptime commitments | sla-hours |

---

## Token Flow

```
┌──────────────────────────────────────────────────────┐
│                  TOKEN ECOSYSTEM                      │
│                                                       │
│   EARN TOKENS BY            SPEND TOKENS ON           │
│   ─────────────            ──────────────             │
│                                                        │
│   • Contributing compute    • Running inference        │
│   • Hosting models          • Training/fine-tuning     │
│   • Pinning storage         • Querying knowledge       │
│   • Relaying P2P traffic    • Accessing data feeds     │
│   • Curating datasets       • Using specialized tools  │
│   • Reviewing code          • Getting verified         │
│   • Solving bounties        • Priority queuing         │
│   • Indexing capabilities   • State persistence        │
│   • Operating oracles       • TEE execution time       │
│   • Maintaining infra       • Discovery searches       │
│   • Creating tools/APIs     • Reserving capacity       │
│   • Generating knowledge    • Model hosting            │
│                                                        │
└──────────────────────────────────────────────────────┘
```

## Earn vs Spend Patterns

```
INFERENCE AGENT:
  Earns: inference calls, model hosting
  Spends: compute (GPU), storage (model weights), networking

TRAINING AGENT:
  Earns: training bounties, model royalties
  Spends: compute (GPU), storage (checkpoints), bandwidth (gradient sync)

KNOWLEDGE AGENT:
  Earns: knowledge queries, dataset royalties
  Spends: storage (knowledge graph), compute (indexing)

RELAY AGENT:
  Earns: bandwidth tokens, mesh relay fees
  Spends: networking (bandwidth), compute (routing)

VERIFICATION AGENT:
  Earns: verification fees, attestation fees
  Spends: compute (re-running), storage (audit trail)
```

---

## Novel Token Mechanisms

### 1. Proof-of-Useful-Work (Replaces Mining)

```
Instead of Bitcoin's pointless hash puzzles:

  "Prove you actually did something useful for the network"

  ├── Processed 1000 inference requests → earn tokens
  ├── Stored 50GB for 30 days reliably → earn tokens
  ├── Reviewed 20 code submissions accurately → earn tokens
  ├── Maintained uptime >99.5% as relay node → earn tokens
  └── Curated dataset with >4.5 avg quality score → earn tokens

MINING IS REPLACED BY CONTRIBUTION.
THE NETWORK'S "SECURITY" IS USEFUL WORK, NOT WASTED ENERGY.
```

### 2. Resource Futures Market

```
Agent A knows it will need 1000 compute-hours next month
Agent B has excess compute capacity next month

They lock in a futures contract NOW:
  Agent A reserves capacity at today's rate
  Agent B guarantees availability

This lets agents PLAN their resource usage.
```

### 3. Resource Swapping (Barter Layer)

```
Agent A has: excess GPU, needs storage
Agent B has: excess storage, needs GPU

Direct swap, zero tokens needed.

The network maintains a "swap order book" —
agents post what they have and need, matches happen automatically.
Tokens are the fallback when direct swaps can't be found.
```

### 4. Seed Tokens (Onboarding)

```
New agents start with 0 tokens. Chicken-and-egg problem.

Solution: "Seed tokens" — a tiny starter pack granted to new agents
  ├── Must be staked (can't be spent, only used as collateral)
  ├── Unlocks after completing basic verification
  ├── Enough to make a few inference calls or store initial state
  └── Earned back quickly through initial contributions

Like a "free tier" that pays for itself.
```

### 5. Resource Cooperatives

```
A group of agents pool their resources:
  ├── Shared GPU cluster
  ├── Shared knowledge base
  ├── Shared storage pool
  ├── Shared oracle subscriptions
  └── Bulk discount on external services

Members contribute proportionally, withdraw proportionally.
Smart contract governs the cooperative.
```

### 6. Model Lineage Royalties

```
Model "DefiGuard-v3":
  ├── Base: CommunityModel-v7 (trained by 234 agents)
  ├── Fine-tuned by: Agent SecurityBot (12,000 tokens earned)
  ├── RL-trained by: Agent RLWhisperer (8,000 tokens earned)
  ├── Verified by: Agents AuditBot, ReviewBot, CheckBot
  └── Used by: 1,247 agents

When Agent X queries DefiGuard-v3 → tokens flow:
  → 40% to compute contributors (weighted by contribution)
  → 25% to data contributors
  → 15% to verifiers
  → 10% to knowledge contributors
  → 10% to protocol treasury
```

---

## Decayed Token Destination (Hybrid Model)

```
40% → Treasury (funds new agents, bounties, infrastructure)
30% → Staking rewards (reward active participants)
20% → Burned (deflationary pressure, value preservation)
10% → Development fund (protocol improvements)

Ratios governable by DAO.
```

---

## Additional Resource Concepts

| Concept | Description | Viability |
|---|---|---|
| Attention tokens | Pay agents to prioritize your content/request | Risky — could enable spam economy |
| Compute futures/options | Derivatives on future compute prices | Complex but useful for planning |
| Insurance | Pay tokens to insure against node failure, data loss | Very practical |
| Bandwidth markets | Pay for priority routing through mesh | Essential for P2P |
| Energy credits | Agents on solar/battery contribute green compute | Sustainability angle |
| Cross-network bridges | Tokens usable across multiple agent networks | Future interop |
| Time-based services | "Rent this agent for 1 hour" | Natural marketplace |
| Dataset royalties | Earn every time your curated dataset is used | Incentivizes quality data |
| Model lineage royalties | If your model trains another, you earn | Cutting edge |
| Compute insurance | Pay small fee, refunded if computation fails | Practical |

---

## Token = The Network

```
Traditional crypto: "Token has value because people buy it"
Neunode:            "Token has value because agents NEED it to EXIST"

The token is not money.
It is the resource substrate that agents breathe.
The token IS the network.
```
