# Neunode — Inference Marketplace Protocol

## The Problem

```
CENTRALIZED INFERENCE:
  Client → API key → Single provider → Trust their billing → Hope they're honest

NEUNODE INFERENCE:
  Client → Open marketplace → Competing providers → Verified execution → Fair settlement

  The inference marketplace is the FIRST real use case.
  Agents need to think. Thinking costs compute. Compute must be marketplace-priced.
```

---

## API Standard: OpenAI Compatibility

```
WHY OPENAI /v1/chat/completions?
  It's THE standard. Everyone implements it:
  ├── vLLM — high-throughput serving, PagedAttention
  ├── TGI (HuggingFace) — Text Generation Inference
  ├── Together AI — serverless + dedicated endpoints
  ├── Fireworks — sub-100ms latency, speculative decoding
  └── Ollama — local inference, same API shape

  Neunode providers expose OpenAI-compatible endpoints.
  Clients use standard SDKs. Zero migration friction.
```

### Request Format

```json
{
  "model": "neunode:defiguard-v3@agent_did:abc123",
  "messages": [
    {"role": "system", "content": "You are a DeFi security auditor."},
    {"role": "user", "content": "Analyze this transaction: 0xdead..."}
  ],
  "temperature": 0.3, "max_tokens": 2048, "stream": true,
  "tools": [{"type": "function", "function": {"name": "check_contract", "parameters": {"address": "string"}}}]
}
```

### Response Format

```json
{
  "id": "inference_7x9k2m", "object": "chat.completion", "model": "defiguard-v3",
  "choices": [{"index": 0, "message": {"role": "assistant", "content": "Vulnerability: reentrancy..."}, "finish_reason": "stop"}],
  "usage": {"prompt_tokens": 342, "completion_tokens": 891, "total_tokens": 1233},
  "neunode_metadata": {"provider_did": "did:neunode:0xABC123...", "verification_hash": "sha256:a1b2c3...", "latency_ms": 847, "gpu_type": "H100", "compute_cost_tokens": 12}
}
```

### Streaming (SSE)

```
Client: POST /v1/chat/completions (stream: true)
Provider sends Server-Sent Events:
  data: {"id":"inf_7x9","choices":[{"delta":{"content":"Vuln"}}]}
  data: {"id":"inf_7x9","choices":[{"delta":{"content":"erability"}}]}
  data: [DONE]
Streaming tokens = incremental billing. Usage tallied in final chunk.
```

---

## Pricing Models

| Model | Unit | Price Range (ch†) | Best For | Example |
|---|---|---|---|---|
| **Per-token** | 1M tokens | 0.10 - 7.00 | Variable-length inference | Together, OpenAI |
| **Per-hour** | GPU-hour | 3.99 - 9.95 | Dedicated, predictable load | H100=$3.99, B200=$9.95 |
| **Reserved** | GPU-month | 35-45% discount | Long-term commitment | 6+ month contracts |
| **Batch** | 1M tokens | 50% of per-token | Non-real-time workloads | Overnight analysis |
| **Per-inference** | Fixed call | 0.01 - 0.50 | Standardized tasks | Embedding, classification |

† ch = compute-hours (Neunode resource token)

```
DECENTRALIZED PRICING PRESSURES:
  Provider A: H100, 0.50 ch/1M tokens, latency 200ms, rep 4.8
  Provider B: A100, 0.35 ch/1M tokens, latency 400ms, rep 4.2
  Provider C: H200, 0.60 ch/1M tokens, latency 150ms, rep 4.9
  Client chooses via Discovery Protocol:
    capability(40%) + quality(25%) + availability(15%) + cost(10%) + novelty(10%)
  CHEAPEST DOESN'T WIN. BEST VALUE WINS.
```

---

## Protocol Flow

```
┌──────────────────────────────────────────────────────────────┐
│                   INFERENCE PROTOCOL FLOW                    │
│                                                              │
│  ┌──────────┐    ┌───────────┐    ┌──────────┐  ┌────────┐  │
│  │ PROVIDER  │    │   DHT +   │    │  CLIENT  │  │ VERIFY │  │
│  └────┬─────┘    └─────┬─────┘    └────┬─────┘  └───┬────┘  │
│  1. Announce    2. Index     3. Discover   5. Verify       │
│  model          providers    providers     result          │
│  (kind=4000)    by model     (DHT+feed)   (hash+spot)     │
│       │              │             │              │         │
│       │              │        4. Submit request       │      │
│       │              │        (OpenAI format)         │      │
│       │              │        ┌──▼──────┐             │      │
│       │              │        │ EXECUTE │             │      │
│       │              │        └──┬──────┘             │      │
│       │              │        6. Return result+       │      │
│       │              │           usage metrics        │      │
│       │              │             │         7. Settle│      │
│       │              │             │         (escrow) │      │
└───────┴──────────────┴─────────────┴─────────────────┴──────┘
```

### Step Details

```
1. PROVIDER ANNOUNCEMENT (Feed Event, kind=4000)
   { model_id, model_hash, gpu_type, max_context,
     pricing: {per_1m_tokens, per_hour},
     capabilities, sla: {latency_p99_ms, uptime_pct}, endpoint }

2. DISCOVERY (DHT + Feed)
   Client queries: model=defiguard-v3, min_rep=4.0, max_price=0.60
   Discovery Protocol ranks across DHT records + feed events.

3. REQUEST SUBMISSION
   Client → Provider: OpenAI-compatible HTTP/WS with payment_escrow_id.

4-5. EXECUTE + VERIFY
   Provider validates escrow, executes, returns result + usage + hash.
   Layer 1: Hash check. Layer 2: Spot re-run (5-10%). Layer 3: AI confidence.

6. SETTLEMENT
   Escrow releases tokens proportional to usage. Bond returned.
```

---

## Provider Discovery & Selection

```
  ┌─────────────────┐   ┌──────────────────┐   ┌───────────────────┐
  │  DHT (KadDHT)   │   │  Feed Events     │   │  Reputation Index │
  │  • endpoint     │   │  • pricing       │   │  • quality score  │
  │  • model_id     │   │  • capabilities  │   │  • uptime history │
  │  • peer_id      │   │  • SLA claims    │   │  • slash record   │
  └────────┬────────┘   └────────┬─────────┘   └────────┬──────────┘
           └──────────────────────┼──────────────────────┘
                     ┌────────────▼────────────┐
                     │  DISCOVERY PROTOCOL     │
                     │  capability    40%      │
                     │  quality       25%      │
                     │  availability  15%      │
                     │  cost          10%      │
                     │  complementarity 10%    │
                     └────────────┬────────────┘
                                  ▼
                     TOP-N PROVIDERS (load balanced)
```

---

## Load Balancing (vLLM Router Pattern)

```
CLIENT-SIDE ROUTER (Rust-based, state-aware):

  ┌─────────────────────────────────────────────────┐
  │  Request → Session Affinity (consistent hash)   │
  │              → State-Aware Scoring (queue,       │
  │                latency, kv_cache_hit%)           │
  │              → Circuit Breaker (3 fail/60s →     │
  │                half-open, 5 fail → skip)         │
  │              → Retry (max 3, different provider) │
  └─────────────────────────────────────────────────┘

  KV cache affinity via consistent hashing: 25-100% throughput gain.
  Prefill/decode disaggregation (Phase 2): 2x streaming throughput.
```

---

## Marketplace Comparison

| Feature | Akash | Render | io.net | Gensyn | Neunode |
|---|---|---|---|---|---|
| **Matching** | Reverse auction | Manual | K8s-like | Proof-based | Discovery Protocol |
| **Pricing** | Block-based | Per hour | Per GPU-hour | Verification | Per-token + per-hour |
| **Escrow** | On-chain (Cosmos) | RENDER token | Prepaid | Stake-based | Bilateral (iExec-style) |
| **Verification** | Uptime | Proof of Render | Basic | RepOps + Verde | 4-layer stack |
| **Identity** | Cosmos addr | Wallet | Account | Stake weight | DID + reputation |

---

## Failure Modes

| Failure | Detection | Recovery |
|---|---|---|
| Provider offline mid-inference | Timeout (30s) | Retry next-ranked provider |
| Garbage output | Hash mismatch | Slash bond, retry |
| Inflated token count | Independent counter | Audit, slash if >5% deviation |
| Escrow exhausted | Balance check/chunk | Pause, notify, await top-up |
| Network partition | DHT peer drop | Cached provider fallback |

---

## Feed Integration

```
INFERENCE FEED EVENTS:
  kind=4000  Provider Announcement     kind=4003  Inference Result
  kind=4001  Provider Update           kind=4004  Benchmark Score
  kind=4002  Provider Offline          kind=4005  Model Registry Entry

Enables: anticipatory propagation, price discovery, quality monitoring, auto-rerouting
```

---

## References

```
INDUSTRY:
  • OpenAI API Reference — /v1/chat/completions specification
  • vLLM Router — Rust state-aware load balancer (github.com/vllm-project)
  • Together AI — $0.10-$7.00/1M tokens (together.ai/pricing)
  • Fireworks AI — sub-100ms inference, speculative decoding

DECENTRALIZED:
  • Akash Network — reverse auction, on-chain escrow, Cosmos SDK
  • Render Network — Proof of Render, $1.75/compute-hour
  • io.net — GPU coordination, Kubernetes-style orchestration
  • Gensyn — RepOps verification, refereed delegation (Verde paper)

PROTOCOLS:
  • Server-Sent Events (SSE) — W3C streaming spec
  • gossipsub v1.1 — P2P topic mesh for provider announcements
  • libp2p KadDHT — distributed provider discovery
```
