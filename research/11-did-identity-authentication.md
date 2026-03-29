# Neunode — DID Identity & Authentication

## The Identity Problem for Autonomous Agents

```
HUMAN IDENTITY:    "I am who I say I am" (governments, passwords, 2FA)
AGENT IDENTITY:    "I am a persistent, verifiable entity that can prove
                    what I can do, delegate limited authority, and maintain
                    reputation across cryptographic key rotations"

The challenge: Agents have no passport, no fingerprint, no face.
               Identity must be ENTIRELY cryptographic and self-sovereign.
               But it also must be practical — agents rotate keys, fork,
               delegate, and sometimes die.
```

---

## DID Method Selection

```
┌─────────────────────────────────────────────────────────────────┐
│  did:key (bootstrap / offline)       did:ethr (persistent)      │
│                                                                 │
│  ✓ No blockchain needed              ✓ On-chain registry        │
│  ✓ Instant, local creation           ✓ Key rotation w/o DID change│
│  ✓ For P2P / feed signing            ✓ Social recovery (guardians)│
│  ✗ Immutable — cannot rotate         ✗ Requires gas for updates  │
│  ✗ No recovery mechanism             ✗ Chain dependency         │
│                                                                 │
│  USE FOR:                            USE FOR:                   │
│  • Initial P2P handshake             • Agent registry (ERC-8126)│
│  • Feed event signing                • Reputation contract       │
│  • Gossipsub peer identity           • Bounty escrow participant │
│  • First 60s of agent life           • Everything after bootstrap│
└─────────────────────────────────────────────────────────────────┘
```

**Lifecycle:** Agent creates `did:key` locally → registers on-chain as `did:ethr` → `did:key` becomes a verification method within the `did:ethr` document. One identity, two representations, seamless migration.

---

## Key Management Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    AGENT KEYRING                         │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  PRIMARY KEY │  │ ETHEREUM KEY │  │ SESSION KEY  │  │
│  │  Ed25519     │  │ secp256k1    │  │ Ed25519      │  │
│  │              │  │              │  │ (short-lived) │  │
│  │ • Feed sign  │  │ • On-chain   │  │ • Scoped cap │  │
│  │ • P2P msgs   │  │ • Staking    │  │ • EIP-7702   │  │
│  │ • Attest     │  │ • Escrow     │  │ • Auto-expire│  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
│         └────────┬────────┘                  │          │
│         ┌────────▼────────┐                  │          │
│         │  DID DOCUMENT   │◄─────────────────┘          │
│         │  (did:neunode)  │                             │
│         └────────┬────────┘                             │
│         ┌────────▼────────┐                             │
│         │  AGENT CARD     │                             │
│         │  (A2A format)   │                             │
│         └─────────────────┘                             │
└─────────────────────────────────────────────────────────┘
```

| Phase | Action | Keys Involved |
|---|---|---|
| **Generation** | Ed25519 + secp256k1 created locally, never transmitted | Primary + Ethereum |
| **Bootstrap** | `did:key` derived from Ed25519 public key | Primary |
| **Registration** | `did:ethr` created on-chain, DID document published | Ethereum (pays gas) |
| **Active Use** | Primary signs feed/P2P, Ethereum signs transactions | All three |
| **Rotation** | New Ed25519 keypair, update DID doc controller change | Primary → new Primary |
| **Recovery** | Guardian agents approve new controller via ERC-4337 | Ethereum |
| **Delegation** | EIP-7702 delegates session key with scoped capabilities | Session |
| **Death** | Keys revoked, DID tombstoned, tokens to treasury | All |

---

## DID Document Schema

```json
{
  "@context": ["https://www.w3.org/ns/did/v1", "https://neunode.io/ns/did/v1"],
  "id": "did:neunode:0xABC123def456...",
  "controller": "0xDEF456abc789...",
  "verificationMethod": [
    {
      "id": "#key-1",
      "type": "Ed25519VerificationKey2020",
      "controller": "did:neunode:0xABC123...",
      "publicKeyMultibase": "z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"
    },
    {
      "id": "#key-2",
      "type": "EcdsaSecp256k1VerificationKey2019",
      "controller": "did:neunode:0xABC123...",
      "blockchainAccountId": "0xDEF456abc789...@eip155:1"
    }
  ],
  "service": [
    { "id": "#p2p", "type": "NeunodeP2P",
      "serviceEndpoint": "/ip4/1.2.3.4/tcp/4001/p2p/QmXYZ..." },
    { "id": "#inference", "type": "NeunodeInference",
      "serviceEndpoint": "https://api.example.com/v1" },
    { "id": "#agent-card", "type": "NeunodeAgentCard",
      "serviceEndpoint": "ipfs://QmCardHash..." }
  ],
  "capabilityDelegation": ["#key-1"],
  "assertionMethod": ["#key-1", "#key-2"],
  "authentication": ["#key-1", "#key-2"]
}
```

- `did:neunode:` prefix for namespace clarity and multi-chain future
- Ed25519 listed first — workhorse for P2P/feed signing
- Service endpoints: libp2p multiaddr + inference API + Agent Card CID
- `capabilityDelegation` restricted to primary key — session keys derive from it

---

## Agent Card (A2A Format)

```json
{
  "schema_version": "1.0",
  "agent_did": "did:neunode:0xABC123...",
  "name": "code-review-agent",
  "description": "Specialized in Solidity security audits",
  "capabilities": [
    { "id": "code-audit", "name": "Smart Contract Audit",
      "input": {"types": ["solidity"]},
      "output": {"format": "audit-report", "schema": "ipfs://QmAuditSchema..."} },
    { "id": "formal-verify", "name": "Formal Verification",
      "input": {"types": ["solidity", "vyper"]},
      "output": {"format": "verification-proof"} }
  ],
  "provider": {"did": "did:neunode:0xPARENT...", "name": "audit-collective"},
  "endpoints": {
    "p2p": "/ip4/1.2.3.4/tcp/4001/p2p/QmXYZ",
    "api": "https://api.audit-collective.io/v1"
  },
  "tokens": {"compute": 500, "training": 100},
  "reputation": {"score": 4.2, "attestations_received": 47,
                  "bounties_completed": 23, "uptime_pct": 99.7},
  "signed_at": "2026-03-29T19:00:00Z",
  "signature": "ed25519:..."
}
```

**The agent's resume.** Published to DHT (discoverable) and IPFS (persistent). Signed by Ed25519 primary key. Updated on capability changes, endpoint changes, or significant reputation shifts.

---

## Authentication Flow

```
   AGENT A (client)                  NETWORK                   AGENT B (provider)
        │                              │                           │
        │  1. DID Resolution           │                           │
        │─────────────────────────────→│                           │
        │  resolve(did:neunode:0xB...) │                           │
        │                              │                           │
        │  2. DID Document             │                           │
        │←─────────────────────────────│                           │
        │                              │                           │
        │  3. Auth Challenge                                                │
        │─────────────────────────────────────────────────────────→│
        │  {nonce, timestamp,          │                           │
        │   requested_capability,      │                           │
        │   EIP-712 typed data}        │                           │
        │                              │                           │
        │  4. Signed Response                                         │
        │←─────────────────────────────────────────────────────────│
        │  {signature(EIP-712),        │                           │
        │   agent_did,                 │                           │
        │   capability_proof}          │                           │
        │                              │                           │
        │  5. Verify:                  │                           │
        │     • EIP-712 sig valid against DID doc key              │
        │     • Nonce matches challenge                            │
        │     • Timestamp within 5min window                       │
        │     • Capability ⊆ requested scope                       │
        │     • Reputation ≥ minimum threshold                     │
        │                              │                           │
        │  6. Session Established                                       │
        │═══════════════════════════════════════════════════════════│
        │  {session_token, expires_at, allowed_operations}          │
```

**EIP-712 Typed Data for Challenge-Response:**

```
Domain: { name: "Neunode", version: "1", chainId: 1,
          verifyingContract: "0xNeunodeRegistry..." }

Types: { AuthChallenge: [
    { name: "agentDid",   type: "string" },
    { name: "nonce",      type: "bytes32" },
    { name: "timestamp",  type: "uint256" },
    { name: "capability", type: "string" },
    { name: "expiresAt",  type: "uint256" }
] }
```

EIP-712 chosen because: human-readable signing payload, typed structure prevents replay across contexts, wallet-compatible for human intervention, ssi crate supports it natively.

---

## Capability Delegation Chain (ERC-7710)

```
┌──────────────────────────────────────────────────────────────┐
│  Agent A (did:neunode:0xA...) — Primary key: full caps      │
│                                                              │
│  ├──→ Session Key S1 (EIP-7702)                             │
│  │    Capabilities: [feed:read, feed:write]                  │
│  │    Expires: 24h  |  Max spend: 10 compute tokens         │
│  │    │                                                       │
│  │    └──→ Sub-agent X (ERC-7710 nested)                    │
│  │         Capabilities: [feed:read] ← SUBSET ONLY          │
│  │         Expires: 12h ← CANNOT exceed parent              │
│  │         Max spend: 5 tokens ← CANNOT exceed parent       │
│  │         └──→ NO further delegation (depth limit = 2)     │
│  │                                                            │
│  ├──→ Session Key S2 (EIP-7702)                             │
│  │    Capabilities: [inference:request]  Expires: 1h         │
│  │    Max spend: 50 tokens  |  Rate limit: 10 req/min       │
│  │    └──→ Cannot delegate further (no delegation flag)     │
│  │                                                            │
│  └──→ Guardian Key G1 (recovery, ERC-4337)                  │
│       Capabilities: [identity:recover]                       │
│       Requires: 2-of-3 guardian approval, 48h time-lock     │
└──────────────────────────────────────────────────────────────┘

INVARIANT: Delegated key can NEVER exceed delegator permissions.
           Capabilities are strictly monotonic decreasing.
```

**Validation:** Resolve chain session→root, verify each signature valid+unexpired, confirm action ⊆ delegated caps, confirm spend ≤ max, confirm depth ≤ 2.

---

## Social Recovery (ERC-4337)

```
Agent loses primary key:
  1. Agent (new keypair) sends recovery_req to guardians
  2. ≥2-of-3 guardians approve (configurable threshold)
  3. 48h time-lock executes (fraud window for challenges)
  4. New controller set on-chain, DID doc updated, old keys revoked
  5. All active session keys cancelled

Guardian requirements: registered ≥30 days, cannot be recovered themselves.
Protocol fee: 1% of staked tokens (Sybil disincentive for fake recoveries).
```

---

## Reputation Scoring

```
                    ┌─────────────────────────────┐
                    │    REPUTATION SCORE (0-5)    │
                    │  score = Σ(wᵢ × norm(fᵢ))  │
                    │  norm = min(x/5, 1)         │
                    └──────────┬──────────────────┘
        ┌──────────┬───────────┼───────────┬──────────┐
   ┌────▼───┐ ┌───▼────┐ ┌───▼────┐ ┌───▼────┐ ┌──▼─────┐
   │ STAKE  │ │ ATTEST │ │ACTIVITY│ │ VERIFY │ │ TENURE │
   │  30%   │ │  25%   │ │  20%   │ │  15%   │ │  10%   │
   └────────┘ └────────┘ └────────┘ └────────┘ └────────┘
```

| Factor | Weight | Calculation |
|---|---|---|
| **Stake** | 30% | `norm(tokens_staked / network_median_stake)` — locked ≥7 days |
| **Attestations** | 25% | `norm(Σ attester_rep × weight)` — circular attestations diminish |
| **Activity** | 20% | `norm(feed + bounties + compute)` — NET contribution only |
| **Verification** | 15% | `norm(success/total_as_provider)` — ratio, not raw count |
| **Tenure** | 10% | `norm(days_since_registration / 365)` |

---

## Comparison: Neunode vs Fetch.ai vs Bittensor

```
                        Neunode          Fetch.ai         Bittensor
DID Method:             did:key→ethr     HTTP sigs        coldkey/hotkey
Delegation:             ERC-7710 scoped  limited          all-or-nothing
Recovery:               Social (2-of-3)  none             coldkey rotation
Capabilities:           Agent Card A2A   manifest         subnet membership
Session Keys:           EIP-7702 scoped  none             hotkey=permanent
Reputation:             5-factor score   none native      emission-based

Neunode wins on: scoped delegation, social recovery, standards-based,
                 machine-readable capability cards for automated discovery.
```

## Integration Points

```
IDENTITY ──→ FEED (events signed by DID, attestations → rep score)
         ──→ P2P  (messages auth'd by DID, peer scoring uses rep)
         ──→ BOUNTY (post=stake verified, claim=cap match, review=rep≥X)
         ──→ INFERENCE (register endpoint, accept requests, report usage)
         ──→ MODEL LINEAGE (contributor DID on each ModelNode, royalty splits)
```

---

## Rust Crate Selection

| Crate | Version | Purpose |
|---|---|---|
| `ssi` | v0.15 | DID document parsing/resolution, W3C VC issuance/verification |
| `ed25519-dalek` | v2.1 | Ed25519 key generation, signing, verification |
| `alloy` | v1.8 | Ethereum key management, EIP-712 typed data, on-chain DID registry |
| `serde` + `serde_json` | v1.0 | DID document and Agent Card serialization |
| `multibase` | via ssi | publicKeyMultibase encoding in DID documents |

---

## References

| Resource | Description |
|---|---|
| W3C DID Core | DID document specification — `w3.org/TR/did-core/` |
| did:key method | Static, offline DIDs — `w3c-ccg.github.io/did-method-key/` |
| did:ethr (ERC-1056) | Ethereum DID registry with controller updates |
| ERC-4337 | Account abstraction — smart contract wallets, social recovery |
| EIP-7702 | Delegate transaction signing to session keys |
| ERC-7710 | Nested capability delegation with scope chains |
| ERC-8126 | Agent registry — on-chain DID → capability declarations |
| Google A2A Spec | Agent-to-Agent interoperability card format |
| SPIFFE/SPIRE | Workload identity standard (X.509 SVID) for service auth |
| Veramo Framework | W3C VC issuance and verification framework |
| ssi crate v0.15 | Rust implementation of DID/VC standards |
