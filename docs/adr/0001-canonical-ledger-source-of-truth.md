# ADR-0001 — Canonical ledger and source of truth

- **Status:** Accepted
- **Date:** 2026-06-24
- **Supersedes:** none

## Context

The protocol currently has **two independent implementations of its core state**
(token balances, bounty lifecycle) with **no bridging code** between them:

1. **Off-chain Rust ledger** — `agnetd` reads and writes state through the
   `neunode-*` crates to a local RocksDB. This is what actually runs today.
   Token amounts are `TokenAmount(pub u64)` (`crates/neunode-core/src/types.rs`).
2. **On-chain Solidity contracts** — `NeunodeToken` / `NeunodeBounty` etc., using
   `uint256`. These compile and pass 333+ tests on Anvil but are **not deployed**
   and **not consulted** by `agnetd`.

Evidence (verified during the 2026-06-24 architecture review):

- `crates/agnetd/src/cmd_token.rs` and `cmd_bounty.rs` operate entirely on
  `neunode_storage::{token_store, bounty_store}` (local RocksDB). They make **no**
  contract calls.
- The only contract integration in the identity path is optional DID registration
  (`crates/neunode-identity/src/contracts.rs`), off by default.
- `TokenAmount(u64)` vs Solidity `uint256`: different width, no conversion layer.
- No event listener, sync job, or mirror code connects the two.

Consequences of leaving this ambiguous: the two implementations **silently drift**,
neither is clearly authoritative, and "decentralized" claims rest on a ledger
(local RocksDB) that is, by definition, centralized. Every new feature adds state
to *both* sides, doubling maintenance and the drift surface.

## Decision

**The off-chain Rust ledger is the canonical source of truth today.** The
Solidity contracts are the **on-chain migration target / security spec**, not a
parallel runtime. Convergence is one-directional: the Rust path evolves toward
issuing through the contracts when a chain exists — never the reverse.

Concrete implications:

1. **One canonical type per concept.** `TokenAmount` is the authoritative
   token-magnitude type off-chain. Its width must be sufficient for the contract
   range; the `u64` ↔ `uint256` mismatch is a known gap to close (see Open
   questions), not something to paper over with ad-hoc casts.
2. **Contracts are gated behind a feature/flag, not assumed live.** Code that
   needs on-chain finality must explicitly opt into the contract path (as
   `cmd_identity` already does for optional DID registration). Nothing reads from
   contracts by default.
3. **No new parallel state.** When adding stateful features, add them to the
   canonical (Rust) ledger first. Mirror to contracts only as part of a deliberate
   migration step, with a test that the two agree at the boundary.
4. **The "decentralized" claim is scoped.** Until a consensus layer ships
   (see ADR-TBD: consensus strategy) and the contract path becomes live, Neunode
   is operationally a **local application with an on-chain spec**, not a deployed
   decentralized network. Docs and marketing should reflect this.

## Consequences

- **Positive:** removes the silent-drift hazard; gives contributors a single place
  to add state; makes the path to real decentralization (contracts become live)
  explicit and one-directional.
- **Negative:** the contracts carry maintenance cost while not yet authoritative.
  This is accepted: they are the security spec and migration target, and slashing
  / reputation / identity logic is most safely expressed and tested on-chain first
  (as in the Sybil-resistance work, commits `8e6827c`, `e0bae0f`).
- **Migration cost:** when a chain ships, `agnetd` command paths flip from
  local-store reads/writes to contract calls. This is the single largest remaining
  integration task and should be planned, not improvised.

## Open questions

- **`TokenAmount` width.** `u64` caps at ~1.84e19 (≈18.4B units at 18 decimals).
  Widening to `u128` removes the practical cap and aligns intent with `uint256`'s
  range without a full 256-bit dependency. To be addressed in a follow-up.
- **Consensus strategy** (build Reth+Malachite vs deploy on an existing L2) is the
  gating decision for *when* the contract path becomes live. Tracked separately.
