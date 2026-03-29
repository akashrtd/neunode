# Neunode — TurboQuant Compression Layer

> Based on: "TurboQuant: Online Vector Quantization with Near-optimal Distortion Rate" (arxiv 2504.19874v1)
> Authors: Amir Zandieh, Majid Daliri (Google Research / NYU)

## Paper Summary

Google Research proved that **random rotation + simple scalar quantization** achieves near-optimal compression (within 2.7× of the theoretical limit) for **any vector, any bit-width, without ever seeing the data** — and it's **5-6 orders of magnitude faster** than existing methods.

## Why This Matters for Neunode

The #1 bottleneck in decentralized training is inter-node communication bandwidth:

```
Centralized:  NVLink = 900 GB/s between GPUs
Decentralized: Home internet = 80 MB/s (11,000x slower)
```

Everything nodes send each other in distributed training is **vectors**:
- Gradients
- Activations
- KV cache
- Model weights
- Knowledge embeddings

TurboQuant compresses vectors near-optimally, in microseconds, with zero coordination.

---

## Key Properties for Neunode

### 1. Data-Oblivious = Zero Coordination

```
Traditional quantization (GPTQ, AWQ, QuIP#):
  • Collect calibration data
  • Analyze distributions
  • Compute optimal parameters
  → Requires ALL nodes to share data
  → Can't be done on-the-fly
  → TERRIBLE for decentralized training

TurboQuant:
  • Agree on a random seed (one integer)
  • Quantize. Done.
  → No data sharing needed
  → No preprocessing needed
  → Works in real-time
  → PERFECT for decentralized training
```

**Neunode implementation:**
```
Agent A and Agent B have never communicated.
They need to exchange gradient updates.

1. Agree on seed: block_height + round_number (public info)
2. Both compute same random rotation matrix locally
3. Both use same precomputed codebooks (determined by d and b)
4. Agent A quantizes → sends compressed bits
5. Agent B dequantizes → uses for training

ZERO coordination overhead.
Works same whether agents are on same LAN or different continents.
```

### 2. Unbiased Inner Product Estimation

```
In distributed SGD, nodes compute:
  g_avg = (g₁ + g₂ + ... + gₙ) / n

If quantization is BIASED:
  E[Q(g)] ≠ g  (systematic error)
  → g_avg converges to WRONG answer
  → Model trains incorrectly

TurboQuant_prod is UNBIASED:
  E[⟨y, Q⁻¹(Q(x))⟩] = ⟨y, x⟩  (exactly, provably)

  → Quantized gradient averaging converges to CORRECT answer
  → Can be used for TRAINING, not just inference
  → ONLY method that's both unbiased AND data-oblivious at arbitrary bit-widths
```

### 3. Near-Zero Computational Overhead

```
Quantization speed for 100K vectors (d=1536):

  Product Quantization:    239.75 seconds
  RabitQ:                2,267.59 seconds
  TurboQuant:                0.0013 seconds

  That's 5-6 ORDERS OF MAGNITUDE faster.

For Neunode: Quantizing a gradient vector (d=4096) takes ~0.0001 seconds on GPU.
Practically free. Can be applied to EVERY communication.
```

### 4. Quantified Compression-Accuracy Tradeoff

| Bit-width | Compression | MSE Distortion | Use Case |
|---|---|---|---|
| 1-bit | 16× | 0.36 | Extreme: gradient compression |
| 2-bit | 8× | 0.117 | Aggressive: inter-node gradients |
| 3-bit | 5.3× | 0.03 | Moderate: activation sharing |
| 4-bit | 4× | 0.009 | Standard: KV cache, weights |
| 3.5-bit | 4.5× | ~0.015 | Sweet spot: KV cache (quality-neutral) |

All within 2.7× of the **information-theoretic optimum**. No method can ever do more than 2.7× better.

---

## How TurboQuant Works

### Algorithm 1: TurboQuant_mse (MSE-optimized)

```
Setup Phase (once per d,b combination):
1. Generate random rotation matrix Π ∈ ℝ^{d×d}
2. Solve continuous 1D k-means for Beta distribution on coordinates
3. Precompute and store codebooks (tiny: 2^b entries)

Quant(x):
1. y ← Π · x                    (random rotation)
2. idx_j ← argmin_k |y_j - c_k|  (nearest centroid per coordinate)
3. Output: idx (b-bit integers)

DeQuant(idx):
1. ỹ_j ← c_{idx_j}               (lookup centroids)
2. x̃ ← Πᵀ · ỹ                   (inverse rotation)
3. Output: x̃
```

### Algorithm 2: TurboQuant_prod (Inner Product-optimized)

```
Two-stage approach for unbiased inner product estimation:
  Stage 1: MSE quantizer with (b-1) bits
  Stage 2: 1-bit QJL transform on residual

Quant_prod(x):
1. idx ← Quant_mse(x)            (MSE quantize with b-1 bits)
2. r ← x - DeQuant_mse(idx)      (compute residual)
3. qjl ← sign(S · r)             (1-bit QJL on residual)
4. Output: (idx, qjl, ‖r‖₂)     (total: b bits + 1 scalar)

DeQuant_prod(idx, qjl, γ):
1. x̃_mse ← DeQuant_mse(idx)
2. x̃_qjl ← (√(π/2)/d) · γ · Sᵀ · qjl
3. Output: x̃_mse + x̃_qjl        (unbiased estimate)
```

### The Mathematical Insight

```
1. Random rotation erases input structure
   → Πx is uniform on sphere regardless of what x was

2. Each coordinate follows a KNOWN Beta distribution
   → Doesn't depend on input data at all

3. In high dimensions, coordinates are approximately independent
   → Joint distribution factorizes

4. Optimal vector quantizer reduces to d independent scalar quantizers
   → Exponential-complexity d-dimensional problem → d simple 1D problems

5. Panter-Dite formula: MSE decays as 1/4^b (exponential in bit-width)
   → This is the information-theoretic optimum
```

---

## Performance Results

### KV Cache Quality (Llama-3.1-8B-Instruct)

| Method | Bits | Needle-In-Haystack | LongBench Avg |
|---|---|---|---|
| Full Precision | 16 | 0.997 | 50.06 |
| KIVI | 3 | 0.981 | 48.50 |
| KIVI | 5 | 0.997 | 50.16 |
| PolarQuant | 3.9 | 0.995 | 49.78 |
| **TurboQuant** | **2.5** | **0.997** | **49.44** |
| **TurboQuant** | **3.5** | **0.997** | **50.06** |

**At 3.5 bits, TurboQuant matches full precision. At 2.5 bits, marginal degradation.**

### Nearest Neighbor Search — Indexing Time

| Method | d=200 | d=1536 | d=3072 |
|---|---|---|---|
| Product Quantization | 37.04s | 239.75s | 494.42s |
| RabitQ | 597.25s | 2,267.59s | 3,957.59s |
| **TurboQuant** | **0.0007s** | **0.0013s** | **0.0021s** |

---

## Neunode Integration

### Communication Stack

```
BEFORE TURBOQUANT:
  Agent A ──→ FP16 gradient (8 KB/vector) ──→ Agent B

AFTER TURBOQUANT:
  Agent A ──→ TQ 2-bit gradient (1 KB/vector) ──→ Agent B
  8× less bandwidth, unbiased estimate, microseconds to compute
```

### Mode Selection by Data Type

| Data Type | TurboQuant Mode | Bit-width | Why |
|---|---|---|---|
| Gradients | TQ_prod | 1-2 bit | Unbiased = safe for SGD convergence |
| Activations | TQ_mse | 3-4 bit | Minimize distortion for downstream layers |
| KV cache | TQ_mse | 3.5 bit | Quality-neutral, 4.5× memory savings |
| Knowledge vectors | TQ_mse | 4 bit | High fidelity, query accuracy matters |

### Adaptive Bit-Width by Network Condition

```
Agent with fiber (1 Gbps):
  → TQ 4-bit (high fidelity, 4× savings)

Agent with home internet (80 Mbps):
  → TQ 2-bit (aggressive, 8× savings)

Agent on mobile hotspot (10 Mbps):
  → TQ 1-bit (extreme, 16× savings)

ALL THREE TRAIN TOGETHER.
Each agent adapts quantization to its bandwidth.
Unbiased property means averaged gradient is still correct.
```

### Practical Bandwidth Impact

```
DiLoCo-style training, 7B model:

WITHOUT TURBOQUANT:
  ~14 GB per gradient sync
  ~3 minutes upload on 80 MB/s home internet

WITH TURBOQUANT (2-bit):
  ~1.75 GB per sync
  ~22 seconds upload
  → 8× faster sync → agents sync more often → faster convergence

WITH TURBOQUANT (1-bit):
  ~875 MB per sync
  ~11 seconds upload
  → Agents can sync every few minutes instead of every few hours
  → Fundamentally changes training dynamics
```

---

## Theoretical Guarantees

### Theorem 1 (MSE Upper Bound)
For any b≥1 and unit-norm x:
```
D_mse ≤ (√3·π/2) · 1/4^b ≈ 2.72/4^b
```

### Theorem 2 (Inner Product Upper Bound)
For any b≥1, unbiased with:
```
D_prod ≤ (√3·π²·‖y‖²/d) · 1/4^b
```

### Theorem 3 (Information-Theoretic Lower Bound)
For ANY randomized quantizer with b bits per coordinate:
```
D_mse ≥ 1/4^b
```

**Gap: TurboQuant is within factor √3·π/2 ≈ 2.7 of optimal. No method can ever do more than 2.7× better.**

---

## Limitations

1. **Unit norm assumption** — general vectors need separate norm storage (minor overhead)
2. **O(d²) rotation cost** — structured rotations could reduce this (future work)
3. **Entropy encoding not implemented** — could save ~5% additional bits
4. **Only tested on decoder transformers** (Llama, Ministral) — no encoder/diffusion results
5. **No weight-only quantization experiments** — focuses on KV cache and embeddings

---

## Comparison with Existing Methods

| Method | Online? | GPU-friendly? | Unbiased? | Optimal bound? |
|---|---|---|---|---|
| GPTQ | ❌ Offline | Moderate | ❌ | No guarantees |
| AWQ | ❌ Offline | Moderate | ❌ | No guarantees |
| QuIP# | ❌ Offline | Moderate | ❌ | Some guarantees |
| Product Quantization | ❌ Offline | Moderate | ❌ | No guarantees |
| RabitQ | ✅ Online | ❌ Non-vectorizable | ❌ | Loose bounds |
| QJL | ✅ Online | ✅ | ✅ (1-bit only) | 1-bit optimal |
| **TurboQuant** | **✅ Online** | **✅ Vectorizable** | **✅ All bit-widths** | **Near-optimal (2.7×)** |

---

## Bottom Line for Neunode

```
TurboQuant solves the LAST major technical objection to decentralized training:

  "But bandwidth is too expensive / slow / limited"

Combined with:
  • DiLoCo (sync every 500 steps)
  • Pipeline parallelism (small batches)
  • SWARM (dynamic rewiring on failure)
  • TurboQuant (8× compression, unbiased, microseconds)

Neunode agents can train LLMs across consumer hardware on home internet.
```

---

## References

- [TurboQuant: Online Vector Quantization with Near-optimal Distortion Rate](https://arxiv.org/html/2504.19874v1)
- [Google Research Blog: TurboQuant](https://research.google/blog/turboquant-redefining-ai-efficiency-with-extreme-compression/)
- [QJL: 1-Bit Quantization](https://arxiv.org/abs/2405.14553) (foundation for TurboQuant_prod)
