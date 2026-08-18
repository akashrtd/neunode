---
title: Crate map
description: What each crate does, and the dependency rules that hold the workspace together.
---

# Crate map

The workspace has 21 members: 19 library crates, the `agnetd` binary, and an integration test crate.

## The rule

Dependencies flow **downward only**. `neunode-core` is the sole leaf and depends on nothing inside
the workspace. There are no cycles.

```text
agnetd  (binary, depends on 16 crates)
  |
  +-- domain:    bounty · inference · training · knowledge · lineage
  |              discovery · verification · turboquant · reputation · token
  |
  +-- protocol:  feed · p2p · identity
  |
  +-- infra:     storage · crypto
  |
  +-- core:      neunode-core     (leaf, zero internal dependencies)
```

## Library crates

| Crate | Responsibility |
|---|---|
| `neunode-core` | Types, `Kind` taxonomy, config, constants, errors |
| `neunode-crypto` | Ed25519, secp256k1, EIP-712, AEAD, hashing |
| `neunode-identity` | DID, keyring, agent card, DID documents |
| `neunode-storage` | Three partitioned RocksDB instances, 22 column families, cache, codec, stores |
| `neunode-p2p` | libp2p: gossipsub, Kademlia DHT, peer scoring, catch-up, private feeds |
| `neunode-feed` | Sigchain, events, schema, filters, topics, rate limiting |
| `neunode-token` | Balances, staking, decay, mint and burn, constant-product AMM |
| `neunode-reputation` | Five-factor scoring, attestations |
| `neunode-bounty` | Lifecycle state machine, escrow, review, verification |
| `neunode-inference` | OpenAI-compatible API, routing, providers, settlement, disputes |
| `neunode-training` | Coordinator, workers, gradient transport, aggregation, fault handling |
| `neunode-turboquant` | Model compression: adaptive strategy, codebooks, int8, rotation |
| `neunode-knowledge` | Knowledge graph: triples, ontology, query, authorization |
| `neunode-lineage` | Model provenance DAG, royalties, sigchain |
| `neunode-discovery` | Agent search, complement matching, capability gaps, scoring |
| `neunode-verification` | Gauntlet, spot check, RepOps, bisection, TEE, ZK stub |
| `neunode-contracts` | `alloy sol!` Rust bindings for the Solidity contracts |
| `neunode-chain-spec` | L1 genesis, gas token, predeploys. Spike work |
| `neunode-engine-api-client` | Engine API JSON-RPC client with JWT. Spike work |

!!! note "Three crates are not wired into the daemon"

    `neunode-contracts`, `neunode-chain-spec`, and `neunode-engine-api-client` compile in CI but
    `agnetd` does not use them. The last two back the sovereign-L1 spike described in
    [ADR-0002](https://github.com/akashrtd/neunode/blob/main/docs/adr/0002-consensus-strategy.md).
    Changes there do not affect runtime behavior.

## There is no machine learning here

Worth stating plainly, because the crate names suggest otherwise.

**No crate depends on an ML framework.** No PyTorch, no candle, no burn, no ONNX, no CUDA.

- `neunode-training` depends on `reqwest` and `axum`. It coordinates workers over HTTP. Its
  `gradient.rs` serializes f32 and int8 gradient bytes for transport. It never computes a gradient.
- `neunode-turboquant` depends on `bytemuck` and `rand`. It performs quantization arithmetic on
  plain f32 slices, on the CPU, without a BLAS library.
- The inference layer routes requests and settles payment, then forwards to an upstream model
  server that you supply.

The practical consequence: **a GPU does nothing for Neunode itself.** You need one only for the
model server you point it at.

## The binary

`agnetd` is roughly 22,000 lines across 56 files, and exposes every capability twice.

| Surface | Files | Registered in |
|---|---|---|
| CLI | `cmd_*.rs`, 20 modules | `cli.rs` |
| HTTP | `api_*.rs`, 22 modules | `api_routes.rs` |

## Conventions

- Flat `src/*.rs` in every crate. No `mod.rs`, no subdirectories.
- Errors use `thiserror` in each crate's `error.rs`. `anyhow` appears only in `agnetd`.
- Tests are inline at the bottom of each source file, in `#[cfg(test)] mod tests`.
- Cross-crate integration tests live in the top-level `tests/` crate.
- Shared dependencies come from `[workspace.dependencies]` with `workspace = true`.
- Toolchain is pinned to Rust 1.93. Format width is 100.

## Feature flags

Only `neunode-verification` has meaningful features.

| Feature | Default | Effect |
|---|---|---|
| `gauntlet`, `spot-check`, `repops` | on | Verification tiers |
| `tee-sim` | off | Simulated TEE evidence |
| `tee-intel` | off | Intel TDX via DCAP |
| `tee-amd` | off | AMD SEV-SNP with pinned VLEK |

CI runs `cargo check --workspace --all-features`, so feature-gated code must compile in every
combination.

## Storage layout

22 column families, split across **three separate RocksDB instances** under the data directory:
`ledger/`, `network/`, and `graph/`. A partition map routes each column family to its database,
and a `ledger_write_lock` mutex makes multi-key ledger writes atomic. Six column families form
the knowledge graph index.

| Group | Column families |
|---|---|
| Core | `IDENTITY`, `CONFIG`, `TOKENS`, `REPUTATION`, `MODELS`, `TRAINING`, `BOUNTIES` |
| Feed | `FEED_EVENTS`, `FEED_INDEX`, `FEED_STATE` |
| System | `UNBONDING`, `AUDIT_LOG`, `P2P_STATE`, `MERKLE_NODES`, `SNAPSHOTS` |
| Knowledge graph | `KG_ID2STR`, `KG_SPOG`, `KG_POSG`, `KG_OSPG`, `KG_GSPO`, `KG_GPOS`, `KG_GOSP` |

The six permutation indexes follow the Oxigraph pattern, so a triple pattern with any combination of
bound terms resolves to a range scan on one of them.
