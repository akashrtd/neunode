# ADR-0002 — Consensus strategy: build a sovereign L1 vs deploy on an existing L2

- **Status:** Proposed (recommendation; awaiting maintainer decision)
- **Date:** 2026-06-24
- **Supersedes:** none

## Context

The "decentralized" claim depends on a consensus layer that does not yet exist.
Verified state (2026-06-24 architecture review + chain-spec spike):

- The intended stack is **Reth (execution) + Malachite (consensus)** via a custom
  bridge layer (`docs/spike-reth-malachite-assessment.md`). Malachite is
  **pre-alpha**; the bridge layer is **planned, not implemented**.
- `crates/neunode-chain-spec` defines a custom EVM L1 (chain ID `9109`, native
  `NEU`, EIP-1559 at 1 Gwei) and `crates/neunode-engine-api-client` speaks the
  Engine API to a Reth execution layer. Both are **spec/client only** — no chain
  is deployed, validator addresses in genesis are `TODO`, predeploy bytecode is
  empty.
- Max ~100 BFT validators by design (`NeunodeReputation.MAX_VALIDATORS`). That is
  Cosmos-appchain tier, not Ethereum-grade security (see review's tier table).
- Reputation, slashing, and identity-stake logic all exist as **contracts** that
  nothing enforces yet (ADR-0001).

Building and operating a sovereign L1 is a multi-year, multi-engineer effort
(consensus client, validator bootstrapping, economic security, audits, uptime).
It is the single largest and riskiest item in the roadmap.

## Decision (recommended)

**Phase the decentralization: ship on an existing credible L2 first; treat the
sovereign L1 as a conditional future migration, not the starting point.**

1. **Near-term (ship):** deploy the Diamond contract suite on an established L2
   (Base or Arbitrum). This inherits Ethereum-grade security and finality
   immediately, makes ADR-0001's "contract path goes live" achievable in weeks,
   and lets the agent economy run for real (bounties, inference, royalties) with
   non-zero but low gas.
2. **Identity + reputation remain the Sybil/security layer** (ADR-0001 +
   commits `8e6827c`, `e0bae0f`): stake-gated registration and validator
   eligibility work identically whether the host is an L2 or a sovereign L1.
3. **Sovereign L1 only if/when justified** — i.e., if censorship resistance
   against the host L2 becomes a real requirement AND validator set + economic
   stake have grown enough to make a 100-validator BFT chain meaningfully
   decentralized. Until then, Reth+Malachite stays a spike, not a commitment.

## Rationale

- **Security is bought, not built,** at this stage. A new BFT chain with a small
  validator set is *less* secure than inheriting an L2 secured by Ethereum, until
  it has years of uptime and real stake at risk.
- **Unblocks the product.** The agent economy's value does not depend on
  sovereign consensus; it depends on contracts being live and cheap to call. An
  L2 delivers that now.
- **Reversibility.** Deploying on an L2 first does not preclude a sovereign L1
  later (same EVM contracts, same identities). Building the L1 first is far harder
  to undo if priorities shift.
- **Honest scoping.** Per ADR-0001, Neunode is operationally a local app today.
  Leaping straight to a custom L1 inverts the risk order.

## Consequences

- **If accepted:** the Reth+Malachite spike is de-prioritized; effort redirects to
  L2 deployment, a bridge/L1→L2 asset plan, and making `agnetd` issue through live
  contracts (the migration named in ADR-0001). The "decentralized" claim becomes
  accurate at the L2-security level, with sovereign decentralization explicitly
  future work.
- **If rejected (sovereign L1 chosen):** accept the multi-year commitment,
  sequence consensus-client maturity, validator bootstrapping, and audits as
  critical-path, and do not claim decentralization until the chain is live with
  meaningful stake.

## Open questions

- Which L2 (Base vs Arbitrum vs others)? Driven by gas, tooling, and bridge
  availability for any real-world asset onramp.
- Does the resource-token economy need a bridge to a real asset (ETH/USDC) to have
  non-zero value, or does it stay a closed utility economy (per the review's
  tokenomics analysis)? Separate ADR.
