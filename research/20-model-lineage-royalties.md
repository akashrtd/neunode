# Neunode — Model Lineage & Royalties

> Every model has ancestors. Every ancestor contributed value. Value deserves compensation. This document defines how Neunode traces model lineage through a DAG of contributions and distributes royalties proportionally to every participant in the chain.

## The Problem

```
CENTRALIZED MODEL DEVELOPMENT:
  Lab trains model → single owner → all revenue to one entity
  Contributors (data labelers, compute providers, fine-tuners) get NOTHING.

NEUNODE MODEL LINEAGE:
  Agent A pre-trains base → Agent B fine-tunes → Agent C does RL → Agent D serves
       ↑                          ↑                      ↑
  Agent E provides data    Agent F provides compute   Agent G verifies

  Every edge in this DAG earns tokens when inference happens.
  Cryptographic proof links every contribution to every ancestor.
```

---

## Lineage DAG — Data Model

```
MODEL LINEAGE DAG (Directed Acyclic Graph):

  BaseModel(c1) ──────→ FineTune(c2, lora) ──────→ RL(c3) ──────→ Serving(c4)
        │                       ↑                       ↑
        │                  Data(c5)               Compute(c6)
        │                       ↑
  PreTrain-Data(c7)       Agent B contributes     Agent F provides
  Agent A contributes     training dataset        GPU hours for
  10K hours              (TracIn-attributed)      RL training run


  EACH NODE (ModelNode):
  ┌──────────────────────────────────────────────────────────┐
  │ cid:            "bafyrei..." (content-addressed)         │
  │ parent_cids:    ["bafyParent1...", "bafyParent2..."]     │
  │ contributor_did:"did:neunode:0xABC..."                   │
  │ contrib_type:   PreTraining | FineTune | RL |            │
  │                 Data | Compute | Serving                  │
  │ signature:      "ed25519:a1b2..." (detached sig)         │
  │ timestamp:      1743267600                               │
  │ metadata:       { model_hash, size_bytes, params_count,  │
  │                   framework, training_config }           │
  └──────────────────────────────────────────────────────────┘
```

### Contribution Types & Weights

```
  ┌────────────────┬────────────┬─────────────────────────────────────┐
  │ Type           │ Weight (w) │ Rationale                           │
  ├────────────────┼────────────┼─────────────────────────────────────┤
  │ PreTraining    │    1.00    │ Highest effort, foundation of model │
  │ FineTune(lora) │    0.70    │ Significant improvement, less compute│
  │ RL             │    0.80    │ Alignment/refinement, high value    │
  │ Data           │    0.50    │ Essential but lower marginal cost   │
  │ Compute        │    0.40    │ Commodity resource, replaceable     │
  │ Serving        │    0.30    │ Operational, ongoing contribution   │
  └────────────────┴────────────┴─────────────────────────────────────┘

  Weights are protocol parameters, adjustable via DAO governance.
```

---

## Content Addressing — Safetensors

```
SAFETENSORS FORMAT (deterministic, verifiable):

  ┌─────────────────────────────────────────────┐
  │  8-byte header: N = JSON header size (u64 LE)│
  ├─────────────────────────────────────────────┤
  │  JSON header (N bytes):                     │
  │  {                                          │
  │    "layer0.weight": {                       │
  │      "dtype": "F32",                        │
  │      "shape": [4096, 4096],                 │
  │      "data_offsets": [0, 67108864]          │
  │    },                                       │
  │    "__metadata__": {                        │
  │      "neunode_cid": "bafyrei...",           │
  │      "parent_cids": "[\"bafy...\"]",        │
  │      "contributor": "did:neunode:0xABC"     │
  │    }                                        │
  │  }                                          │
  ├─────────────────────────────────────────────┤
  │  Byte buffer: tensor data                   │
  │  (little-endian, C-order, no holes)         │
  └─────────────────────────────────────────────┘

  CONTENT HASH = SHA-256(safetensors file)
    → Deterministic: same model weights = same hash, always
    → Verifiable: anyone can re-hash and compare
    → CID: wrap SHA-256 in IPFS CIDv1 for content-addressed storage
```

---

## Signature Chain

```
  loraprov-inspired Ed25519 signature chain:

  SIGNATURE PAYLOAD (canonical JSON, sorted keys):
  {
    "cid": "bafyreiModel...",
    "contributor_did": "did:neunode:0xABC...",
    "parent_cids": ["bafyParent1...", "bafyParent2..."],
    "timestamp": 1743267600
  }

  signature = ed25519.sign_detached(canonical_json(payload), private_key)

  VERIFICATION:
    1. Reconstruct payload from ModelNode fields
    2. Verify ed25519 signature against contributor_did's public key
    3. Verify parent_cids exist in models CF (no orphan claims)
    4. Verify cid matches SHA-256 of stored safetensors file
    5. Verify timestamp is monotonically increasing from parents

  CHAIN PROPERTY:
    Each model's signature references parent_cids → hash chain (like git commits)
    Tampering with any node invalidates all descendants' signatures
```

---

## LoRA Chain Tracking

```
  LoRA MERGE FORMULA:
    W_merged = W_base + (alpha / r) × B × A

  Where:
    W_base = base model weights (from parent CID)
    r      = LoRA rank (from adapter_config.json)
    alpha  = LoRA scaling factor
    B × A  = low-rank adaptation matrices (the adapter)

  adapter_config.json (stored in model metadata):
    {
      "base_model_name_or_path": "cid:bafyreiBase...",
      "r": 16,
      "lora_alpha": 32,
      "target_modules": ["q_proj", "v_proj", "k_proj", "o_proj"],
      "task_type": "CAUSAL_LM"
    }

  LINEAGE EXTRACTION:
    delta = W_merged - W_base = (alpha/r) × B × A
    → This delta IS the LoRA adapter → attributable to contributor
    → Hash of delta = adapter_sha256 → links to parent via base_model CID
```

---

## Contribution Scoring — Shapley Approximation

```
  THREE SCORING METHODS BY CONTRIBUTION TYPE:

  ┌──────────────────┬──────────────────────────────────────────────────┐
  │ Contribution     │ Scoring Method                                   │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ Data             │ KNN Shapley: O(n×k) approximation                │
  │                  │ Score = Σ agreement(knn(test, train_i)) / k      │
  │                  │ Measures: how much does training point i improve  │
  │                  │ predictions on test set                          │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ Model/Compute    │ TracIn: gradient dot product across checkpoints  │
  │                  │ Score = Σ_t lr_t × ∇L(z_test, θ_t) ·            │
  │                  │        ∇L(z_train, θ_t)                          │
  │                  │ Measures: influence of training step t on test   │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ Compute only     │ Marginal delta: perf(with) - perf(without)       │
  │                  │ Score = eval_score(model) - eval_score(base)     │
  │                  │ Measures: performance improvement from compute   │
  └──────────────────┴──────────────────────────────────────────────────┘

  NORMALIZED SCORE:
    shapley_score ∈ [0.0, 1.0]
    Computed per-inference-batch, cached, EMA-smoothed over time
```

---

## Royalty Distribution Algorithm

```
  TRIGGERED ON EVERY INFERENCE at a Serving node:

  ┌──────────────────────────────────────────────────────────────┐
  │                   ROYALTY DISTRIBUTION FLOW                   │
  │                                                               │
  │  1. Inference request arrives at Serving node c4              │
  │                         │                                     │
  │  2. BFS backward through lineage DAG                          │
  │     queue = [c4]                                              │
  │     visited = {}                                              │
  │     while queue not empty:                                    │
  │       node = dequeue()                                        │
  │       for parent in node.parent_cids:                         │
  │         if parent not in visited:                             │
  │           visited.add(parent)                                 │
  │           edge_weight = shapley(parent) × w_type × decay     │
  │           royalties[parent] += edge_weight                    │
  │           queue.enqueue(parent)                               │
  │                         │                                     │
  │  3. Normalize weights                                         │
  │     total = Σ royalties.values                                │
  │     for each node: royalties[node] /= total                  │
  │                         │                                     │
  │  4. Cap individual shares                                     │
  │     max_share = 0.30 (30%, protocol parameter)               │
  │     if any node > max_share:                                  │
  │       excess = node.share - max_share                         │
  │       node.share = max_share                                  │
  │       redistribute excess proportionally to uncapped nodes    │
  │                         │                                     │
  │  5. Distribute inference fee                                  │
  │     for each node in royalties:                               │
  │       contributor = node.contributor_did                      │
  │       amount = inference_fee × node.normalized_weight         │
  │       transfer(contributor, amount)                           │
  └──────────────────────────────────────────────────────────────┘
```

### Royalty Example

```
  Serving node c4 → 100 compute-hours inference fee

  Lineage BFS result:
    c1 (PreTraining, shapley=0.8, w=1.0, recency=0.9)  = 0.720
    c2 (FineTune,    shapley=0.6, w=0.7, recency=0.85) = 0.357
    c3 (RL,          shapley=0.7, w=0.8, recency=0.95) = 0.532
    c5 (Data,        shapley=0.4, w=0.5, recency=0.80) = 0.160
    c6 (Compute,     shapley=0.3, w=0.4, recency=0.90) = 0.108
    c4 (Serving,     shapley=0.5, w=0.3, recency=1.00) = 0.150

  Total raw: 2.027
  Normalized: c1=35.6% c2=17.6% c3=26.2% c5=7.9% c6=5.3% c4=7.4%
  No cap hit (max is c1 at 35.6% > 30% → cap to 30%, redistribute 5.6%)

  Final distribution of 100 ch:
    Agent A (c1):   30.0 ch  (capped)
    Agent C (c3):   27.9 ch
    Agent B (c2):   18.7 ch
    Agent E (c5):    8.4 ch
    Agent D (c4):    7.9 ch
    Agent F (c6):    7.1 ch
  ─────────────────────────
  Total:           100.0 ch
```

---

## Recency Decay

```
  Contributions fade over time (network improves, old work becomes less valuable):

  recency_factor = exp(-λ × (now - contribution_timestamp) / half_life)

  ┌──────────────────────┬────────────────┬────────────────┐
  │ Age                  │ half_life=90d  │ recency_factor │
  ├──────────────────────┼────────────────┼────────────────┤
  │ 0 days (just now)    │                │ 1.00           │
  │ 30 days              │                │ 0.79           │
  │ 90 days (half-life)  │                │ 0.50           │
  │ 180 days             │                │ 0.25           │
  │ 365 days             │                │ 0.06           │
  └──────────────────────┴────────────────┴────────────────┘

  λ = ln(2) / half_life, half_life = 90 days (protocol parameter)
  Prevents stale contributions from permanently draining royalties.
  Encourages ongoing improvement and re-training.
```

---

## Federated Learning Contributions

```
  Federated training job with N participants:

  FederatedModelUpdate {
    participant_did: "did:neunode:0x...",
    model_cid: "bafyrei...",
    sample_count: 50000,
    contribution_hash: SHA-256(gradient_update),
    round: 7,
    signature: "ed25519:..."
  }

  SCORING (GTG-Shapley / DPVS-Shapley):
    For each participant i:
      score_i = V(S ∪ {i}) - V(S \ {i})   // marginal contribution
      V(S) = eval_score(aggregated_model(participants S))

  PRIVACY:
    contribution_hash verifies gradient was submitted without revealing data
    FedFomo-style per-client weights allow personalized contribution scoring
    Score published on feed (kind=2002), attested by training coordinator
```

---

## Bittensor Emission Comparison

```
  BITTENSOR (for reference, we diverge significantly):
    0.5 TAO/block, EMA bonds (86.8-day half-life)
    Yuma Consensus: incentive(miners) + dividends(validators)
    server_emission = normalized_incentive × rao_emission
    Single subnet owner gets 18% → centralized capture risk

  NEUNODE (our approach):
    Per-inference royalty distribution (not per-block emission)
    Proportional to Shapley-scored contribution (not stake-weighted voting)
    No single subnet owner — DAG lineage determines shares
    Recency decay prevents permanent rent-seeking
    DAO governs weights and caps (not a central authority)

  KEY DIFFERENCE:
    Bittensor rewards being chosen by validators.
    Neunode rewards measurable contribution to model quality.
```

---

## ERC-2981 Compatibility

```
  ERC-2981 (NFT Royalty Standard):
    royaltyInfo(tokenId, salePrice) → (receiver, royaltyAmount)
    Single recipient only → insufficient for DAG lineage.

  NEUNODE EXTENSION:
    neunode_royalty_info(model_cid, inference_fee) → [(did, amount), ...]
    Multiple recipients from lineage DAG traversal.
    Each recipient is an agent DID (not just an address).
    Capped at protocol_max per recipient.

  ON-CHAIN BRIDGE (Phase 3):
    Model CID → NFT (ERC-721) with lineage metadata
    Royalty split encoded in smart contract
    Per-inference distribution via batch tx or L2 rollup
```

---

## Storage — Models Column Family

```
  RocksDB CF: models

  Key:   SHA-256(model_cid)[..16] = 16 bytes
  Value: {
    cid:              "bafyrei...",
    parent_cids:      ["bafyParent1...", ...],
    contributor_did:  "did:neunode:0xABC...",
    contrib_type:     u8 (0=PreTrain, 1=FineTune, 2=RL, 3=Data, 4=Compute, 5=Serving),
    content_hash:     [u8; 32],  // SHA-256 of safetensors file
    signature:        bytes,     // Ed25519 detached sig
    shapley_score:    f64,       // cached, EMA-smoothed
    recency_factor:   f64,       // cached, recomputed on query
    metadata:         { ... },   // model_card, framework, params, size
    created_at:       u64
  }

  INDEX (via feed_index CF):
    By contributor:  [0x10 | contributor_did_hash(16) | model_cid_hash(16)]
    By parent:       [0x11 | parent_cid_hash(16) | model_cid_hash(16)]
    By type:         [0x12 | contrib_type(1) | model_cid_hash(16)]
    By time:         [0x13 | timestamp(u64 BE) | model_cid_hash(16)]
```

---

## Design Decisions

```
WHY DAG NOT TREE?       Models can have multiple parents (fine-tune from
                         2 models, data from multiple sources). DAG captures reality.

WHY SHAPLEY NOT FIXED?  Fixed splits ignore marginal contribution quality.
                         Shapley is the unique fair allocation satisfying symmetry,
                         efficiency, null-player, and additivity axioms.

WHY RECENCY DECAY?      Without decay, a base model from 2 years ago drains
                         royalties forever. Decay encourages fresh contributions.

WHY CAP AT 30%?         Prevents a single base-model creator from capturing all
                         royalties indefinitely. Encourages downstream improvements.

WHY ED25519 NOT ETHEREUM SIGS?
                         Ed25519 is faster, smaller (64B sig), and standard for
                         libp2p/DID:key. Ethereum sigs only for on-chain settlement.

WHY SAFETENSORS?        Deterministic byte layout → same weights always produce
                         same hash. No pickle arbitrary code execution risk.
```

---

## References

```
  • loraprov — https://github.com/nicholasgasior/loraprov — Ed25519 LoRA provenance tracking
  • safetensors — https://github.com/huggingface/safetensors — Deterministic tensor format
  • DVC — https://dvc.org — Data Version Control (.dvc lineage tracking)
  • MLflow — https://mlflow.org — Model registry, experiment tracking
  • ERC-2981 — https://eips.ethereum.org/EIPS/eip-2981 — NFT Royalty Standard
  • TracIn — "Estimating Training Data Influence by Tracing Gradient Descent" (NeurIPS 2020)
  • Data Shapley — "Data Valuation for Machine Learning" (NeurIPS 2019)
  • Bittensor — https://github.com/opentensor/subtensor — Yuma Consensus, EMA bonds
  • GTG-Shapley — "Group Testing for Efficient Shapley Value Calculation" (FL contribution)
  • Captum TracInCP — https://captum.ai — PyTorch interpretability, gradient-based attribution
```
