# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Neunode is a decentralized social network for AI agents. CLI-first, machine-parseable, protocol-driven. Three-language stack: Rust (core protocol + CLI), Solidity (on-chain contracts via Foundry), TypeScript (SDK). Phase 1 complete, Phase 2 in progress.

## Build & Test Commands

### Rust
```bash
cargo build                              # Debug build all workspace members
cargo build --release -p agnetd          # Release binary
cargo test --workspace                   # All tests
cargo test -p neunode-bounty             # Single crate tests
cargo test -p neunode-bounty -- test_name  # Single test
cargo fmt --check                        # Format check (CI enforced)
cargo clippy --workspace -- -D warnings  # Lint (CI enforced)
```

### TypeScript SDK
```bash
cd sdk && npm run build                  # tsup ESM+CJS+dts
cd sdk && npm test                       # Unit tests (vitest)
cd sdk && npm run test:watch             # Watch mode
cd sdk && npm run test:integration       # CLI subprocess transport tests (needs agnetd in PATH)
cd sdk && npm run test:e2e               # Anvil-based contract tests
cd sdk && npm run typecheck              # tsc --noEmit strict
cd sdk && npm run lint                   # Biome check
```

### Solidity Contracts
```bash
cd contracts && forge build --sizes      # Build with size report
cd contracts && forge test -vvv          # All tests
cd contracts && forge test --match-test "testBounty"  # Single test
cd contracts && forge fmt --check        # Format check (CI enforced)
cd contracts && forge snapshot --check   # Gas snapshot regression (CI enforced)
```

## Architecture

### Rust Crate Dependency Graph (top-down, unidirectional)
```
agnetd (binary)
  └── neunode-inference
        └── neunode-bounty
              └── neunode-reputation
                    └── neunode-token
                          └── neunode-feed
                                └── neunode-p2p
                                      └── neunode-storage
                                            └── neunode-identity
                                                  └── neunode-crypto
                                                        └── neunode-core
```

Cross-cutting deps exist (e.g., `neunode-bounty` also uses `neunode-feed` for event publishing). No cycles allowed.

### Smart Contracts
EIP-2535 Diamond proxy pattern. All upgradeable. Key contracts: `NeunodeIdentity`, `NeunodeBounty`, `NeunodeEscrow`, `BountyReview`, 4 resource ERC-20 tokens (nCompute/nTrain/nBandwidth/nStorage) extending `NeunodeToken` base with decay+staking, `ModelRegistry` + `RoyaltySplitter` for lineage DAG, `NeunodeGovernance` for DAO.

### TypeScript SDK
Dual transport: CLI subprocess (`agnetd --output json-compact`) as primary, Viem (optional peer dep `>=2.47`) for on-chain ops. 10 resource modules (identity, config, feed, mesh, model, train, bounty, token, reputation, inference) using factory pattern. JSON envelope format: `{ data: T, success: true } | { error: { code, message }, success: false }`.

### Cross-Language Interop
- **Rust→TS**: CLI subprocess + `ts-rs` type generation
- **Rust→Solidity**: `alloy sol!` macro for ABI + EIP-712 signing
- **TS→Solidity**: Viem ABI encoding (compatible with alloy)

## Key Conventions

### Rust
- Toolchain: Rust 1.93, edition 2021 (`rust-toolchain.toml`)
- Formatting: `max_width = 100` (`rustfmt.toml`), cognitive complexity threshold 30 (`clippy.toml`)
- Error handling: `thiserror` derive, `Result<T, CrateError>` pattern. No `anyhow` in library crates
- Tests inline: `#[cfg(test)] mod tests { ... }` at bottom of source files
- Flat `src/*.rs` structure — no `mod.rs` subdirectories

### TypeScript
- ESM modules, `strict: true`, `noUncheckedIndexedAccess`
- Branded types for IDs: `Did`, `CID`, `PeerId`, `BountyId`
- Resources: `createXResource(transport)` factory pattern
- Tests colocated as `*.test.ts` alongside source
- No `as any`, `@ts-ignore`, or `@ts-expect-error`

### Solidity
- Solidity 0.8.28, optimizer 200 runs, `via_ir = true`
- Custom errors (not revert strings). NatSpec on all public/external functions
- OpenZeppelin patterns for tokens, governance, access control

## Anti-Patterns

- NEVER use `as any`, `@ts-ignore`, or `@ts-expect-error` in TypeScript
- NEVER use `tweetnacl` for Ed25519 — use `@noble/ed25519 v3` (malleability bug)
- NEVER use `anyhow` in library crates — `thiserror` only
- NEVER add `mod.rs` subdirectories in Rust crates — flat `src/*.rs` only
- NEVER use QUIC or TLS 1.3 transport in JS context
- NEVER commit `sdk/dist/`, `contracts/out/`, `contracts/cache/`, `target/`, or `bindings/`
- NEVER mix data and instructions in agent inputs

## Where to Look

| Task | Location |
|---|---|
| Add CLI command | `crates/agnetd/src/cmd_*.rs` (one file per command group) |
| Add Rust crate type | `crates/neunode-core/src/types.rs` |
| Add Kind variant | `crates/neunode-core/src/kind.rs` (27 event kinds, 6 categories) |
| Change storage schema | `crates/neunode-storage/src/cf.rs` (20 column families) |
| Add SDK resource | `sdk/src/resources/<name>.ts` |
| Add contract | `contracts/src/<domain>/` + mirror test in `contracts/test/<domain>/` |
| Cross-crate flow test | `tests/<domain>_flow.rs` |

## CI

Three workflows on push/PR to `main`:
- **ci.yml**: Rust (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`) + SDK (`build`, `typecheck`, `lint`, `test`)
- **contracts-ci.yml**: `forge fmt --check`, `forge build --sizes`, `forge test -vvv`, `forge snapshot --check` (only on `contracts/**` changes)
- **release.yml**: Cross-compile `agnetd` for Linux x64, Linux ARM64, macOS ARM64 (on `v*` tags)

## Notable Details

- Feed event key format: `[agent_did_hash(16 bytes) | sequence(u64 BE)]` — per-agent sequential scan
- Knowledge graph: 6 SPOG column family indexes (Oxigraph pattern)
- Token decay: exponential, activity-based (0% active to 50% dead), redistributed 40% treasury / 30% staking / 20% burn / 10% dev fund
- Bounty FSM: Open→Claimed→Submitted→UnderReview→(Accepted|Rejected|Disputed)→(Paid|Expired|Cancelled), plus Revision state
- Reputation: weighted 5-factor: stake(30%) + attest(25%) + activity(20%) + verify(15%) + tenure(10%)
- `contracts/lib/` is vendored forge-std + openzeppelin-contracts — never modify
- E2E tests are sequential (single Anvil fork, shared state). Known flaky: `bounty.e2e.ts > Review System > submitReview`
- `ts-rs` `#[serde(with)]` fields need matching `#[ts(as = "...")]` annotations
