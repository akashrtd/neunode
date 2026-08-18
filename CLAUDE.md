# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Neunode is a protocol for decentralized AI-agent networks: DID identity, stake-gated participation, verifiable work, and resource economics. CLI-first, machine-parseable, protocol-driven.

Three-language stack: **Rust** (core protocol, `agnetd` daemon/CLI), **Solidity** (on-chain contracts via Foundry), **TypeScript** (SDK + MCP server).

**Status:** protocol-complete and tested, *not* a deployed network. The off-chain Rust ledger (RocksDB) is the canonical source of truth today; the Solidity contracts are the on-chain migration target (see `docs/adr/0001-canonical-ledger-source-of-truth.md`). There is no live chain — contracts run against Anvil. ZK verification is stubbed (`Unsupported`); TEE is simulated unless the `tee-intel`/`tee-amd` features are enabled.

## Build & Test Commands

### Rust (21 workspace members: 19 libs + `agnetd` + `tests`)
```bash
cargo build                                # Debug build workspace
cargo build --release -p agnetd            # Release binary → target/release/agnetd
cargo test --workspace                     # All tests (~2,800 test fns)
cargo test -p neunode-bounty               # Single crate
cargo test -p neunode-bounty -- test_name  # Single test
cargo fmt --check                          # CI enforced
cargo clippy --workspace -- -D warnings    # CI enforced
cargo check --workspace --all-features     # CI enforced (feature-gated code)
./scripts/security_audit.sh                # CI enforced (cargo-audit + reachability allowlist)
```

### TypeScript SDK (`sdk/`)
```bash
cd sdk && npm run build              # tsup → ESM + CJS + dts
cd sdk && npm test                   # Unit tests (vitest, colocated *.test.ts)
cd sdk && npm run test:integration   # Against a real agnetd binary (CLI + HTTP routes)
cd sdk && npm run test:e2e           # Anvil-based contract tests
cd sdk && npm run typecheck          # tsc --noEmit, strict
cd sdk && npm run lint               # biome check src/
cd sdk && npm run check:protocol     # Rust→TS type drift gate (CI enforced)
cd sdk && npm run check:abi          # Solidity→TS ABI drift gate (CI enforced)
cd sdk && npm run generate:protocol  # Regenerate src/types/protocol.generated.ts
cd sdk && npm run generate:abi       # Regenerate src/contracts/abi/*.ts (needs contracts/out)
```

### MCP server (`packages/mcp-server/`)
```bash
cd packages/mcp-server && npm run typecheck && npm test && npm run build
```

### Solidity (`contracts/`)
```bash
cd contracts && forge build --sizes                   # Build + size report
cd contracts && forge test -vvv                       # All tests (397, verified)
cd contracts && forge test --match-test "testBounty"  # Single test
cd contracts && forge fmt --check                     # CI enforced
cd contracts && forge snapshot --check                # Gas regression (CI enforced)
```
Foundry is **pinned to v1.5.1** in CI — unpinned `stable` drifts fuzz-test gas medians and flakes `forge snapshot --check`. Match it locally.

### Running a node
```bash
agnetd init                     # First-run config wizard (~/.agnetd/config.toml)
agnetd serve                    # Default 127.0.0.1:8080 — /api/v1, /swagger-ui, /api-docs/openapi.json, WS feed
agnetd dashboard                # ratatui TUI
```
Note: `examples/` and `packages/mcp-server/` default to port **41000**, not 8080. Pass `--port 41000` or set `NEUNODE_URL` when running them.

## Architecture

### Rust workspace layers (dependencies flow downward only; `neunode-core` is the sole leaf)

| Layer | Crates |
|---|---|
| Binary | `agnetd` — clap 4 CLI + axum HTTP/WS daemon |
| Domain | `neunode-bounty`, `neunode-inference`, `neunode-training`, `neunode-knowledge`, `neunode-lineage`, `neunode-discovery`, `neunode-verification`, `neunode-turboquant`, `neunode-reputation`, `neunode-token` |
| Protocol | `neunode-feed`, `neunode-p2p`, `neunode-identity` |
| Infra | `neunode-storage` (RocksDB), `neunode-crypto` |
| Foundation | `neunode-core` (types, `Kind` taxonomy, config, errors — zero internal deps) |
| Standalone | `neunode-chain-spec`, `neunode-engine-api-client` (no internal deps); `neunode-contracts` (→ core only) |

`neunode-chain-spec` and `neunode-engine-api-client` are L1 spike work (Reth/Malachite, see ADR-0002) and are **not** wired into `agnetd`.

### agnetd: two surfaces, one core
- `cmd_*.rs` (20 files) — CLI subcommands, registered in `cli.rs::Commands`
- `api_*.rs` (22 files) — axum handlers, registered in `api_routes.rs` (~80 routes under `/api/v1`)
- Both dispatch into the same crate logic. **When you add a capability, add both** — the SDK and MCP server drive the HTTP surface, and the drift is not caught by the compiler.

### TypeScript SDK
`createNeunodeClient(config)` with four transports: `http` (primary), `cli` (subprocess `agnetd --output json-compact`), `viem` (direct on-chain, optional peer dep `viem >= 2.47`), `mock` (in-memory, for tests).

16 resources built via `createXResource(client)` — note they take the **client**, not a transport. Most are HTTP-first with CLI fallback; `identity`, `lifecycle`, `lineage`, and `verification` are **HTTP-only** and throw without an HTTP transport.

JSON envelope on both HTTP and CLI: `{ data: T, success: true } | { error: { code, message }, success: false }`.

### Cross-language codegen — generated, never hand-edited
- **Rust → TS**: `sdk/src/types/protocol.generated.ts`, emitted by `cargo run -p neunode-core --example emit_sdk_protocol`. CI fails on drift.
- **Solidity → TS**: `sdk/src/contracts/abi/*.ts` (19 files), extracted from `contracts/out`. CI fails on drift.
- **Rust ↔ Solidity**: `alloy sol!` macro in `neunode-contracts` for ABI + EIP-712 signing.
- `ts-rs` derives (`#[ts(export)]`) still emit per-crate `bindings/` during `cargo test` — gitignored, not the SDK's source of truth.

## Key Conventions

### Rust
- Toolchain pinned 1.93, edition 2021 (`rust-toolchain.toml`)
- `max_width = 100`, `use_small_heuristics = "Max"` (`rustfmt.toml`); cognitive complexity 30 (`clippy.toml`)
- Errors: `thiserror` + `Result<T, CrateError>` per crate `error.rs`. `anyhow` only in `agnetd`
- Flat `src/*.rs` — **no `mod.rs`, no subdirectories** (verified true across all 20 crates)
- Tests inline: `#[cfg(test)] mod tests { … }` at the bottom of each source file; `proptest` for property tests
- Cross-crate integration tests live in the top-level `tests/` crate (10 `*_flow.rs` / e2e files)
- Shared deps come from `[workspace.dependencies]` — use `workspace = true`

### TypeScript
- ESM, `strict: true`, `noUncheckedIndexedAccess`
- Branded types for IDs (`Did`, `CID`, `PeerId`, `BountyId`) — never bare strings
- Tests colocated `*.test.ts`; integration/e2e under `sdk/tests/` with their own vitest configs
- Biome runs with **no config file** — defaults (tabs, double quotes). Match surrounding style

### Solidity
- 0.8.28, optimizer 200 runs, `via_ir = true`, `forge fmt` line length 100
- Custom errors (not revert strings); NatSpec on all public/external functions
- EIP-2535 Diamond proxy for upgradeability; OpenZeppelin for tokens/governance/access control

### Commits
Conventional commits with a scope: `feat(verification):`, `fix(sdk):`, `refactor(storage):`, `chore(deps):`, `ci(security):`.

## Anti-Patterns

- NEVER use `as any`, `@ts-ignore`, or `@ts-expect-error`
- NEVER use `tweetnacl` for Ed25519 — use `@noble/ed25519 v3` (malleability bug)
- NEVER use `anyhow` in library crates — `thiserror` only
- NEVER add `mod.rs` or subdirectories in Rust crates — flat `src/*.rs` only
- NEVER hand-edit `protocol.generated.ts` or `sdk/src/contracts/abi/*.ts` — regenerate them
- NEVER modify `contracts/lib/` (vendored forge-std + openzeppelin)
- NEVER use QUIC or TLS 1.3 transport in a JS context (TCP + Noise only)
- NEVER add a `cargo-audit` ignore without a matching `assert_unreachable` guard in `scripts/security_audit.sh`
- NEVER commit `sdk/dist/`, `contracts/out/`, `contracts/cache/`, `target/`, `bindings/`, or `packages/agnetd-*/agnetd*`
- NEVER mix data and instructions in agent inputs (sandboxed-parsing invariant)
- NEVER let a delegated key exceed its delegator's permissions (capabilities decrease monotonically)

## Where to Look

| Task | Location |
|---|---|
| Add CLI command | `crates/agnetd/src/cmd_<group>.rs` + register in `cli.rs` |
| Add HTTP endpoint | `crates/agnetd/src/api_<group>_api.rs` + register in `api_routes.rs` |
| Add shared Rust type | `crates/neunode-core/src/types.rs` |
| Add event kind | `crates/neunode-core/src/kind.rs` (31 variants; category derived from the numeric range) |
| Change storage schema | `crates/neunode-storage/src/cf.rs` (22 column families) |
| Change binary encoding | `crates/neunode-storage/src/codec.rs` (centralized; has a legacy migration path) |
| Modify bounty FSM | `crates/neunode-bounty/src/state_machine.rs` |
| Add SDK resource | `sdk/src/resources/<name>.ts` + `<name>.test.ts` + register in `client/client.ts` |
| Add MCP tool | `packages/mcp-server/src/tools/<name>.ts` + `tools/index.ts` |
| Add contract | `contracts/src/<domain>/` + mirror test in `contracts/test/<domain>/` |
| Change deploy topology | `contracts/script/Deploy.s.sol`, `sdk/src/contracts/addresses.ts` |
| Cross-crate flow test | `tests/<domain>_flow.rs` |
| Architecture decisions | `docs/adr/` (append-only; supersede, don't edit) |
| Background research | `research/` (22 design docs) |

## CI

- **ci.yml** (push/PR to `main`): 6 jobs — `lint` (fmt + clippy), `test` (`cargo test --workspace`), `check` (`--all-features`), `security` (`scripts/security_audit.sh`), `sdk` (build, typecheck, typecheck+lint examples, protocol/ABI drift, lint, unit + integration + e2e), `mcp-server` (typecheck, test, build). The `sdk` job builds `agnetd` and runs `forge build` first.
- **contracts-ci.yml** (only on `contracts/**`): `forge fmt --check`, `forge build --sizes`, `forge test -vvv`, `forge snapshot --check`.
- **release.yml** (on `v*` tags): cross-compile `agnetd` for linux-x64 / linux-arm64 / darwin-arm64 → GitHub Release → publish `@neunode/agnetd` + 3 platform packages to npm.

## Notable Details

- Feed event key: `[agent_did_hash(16B) | sequence(u64 BE)]` — enables per-agent sequential scan
- Knowledge graph: 6 SPOG-style column-family indexes (`KG_SPOG/POSG/OSPG/GSPO/GPOS/GOSP`, Oxigraph pattern) + `KG_ID2STR` dictionary
- Token decay: exponential, activity-tiered (0% / 2% / 5% / 15% / 50%), never reaches zero; redistributed 40% treasury / 30% staking / 20% burn / 10% dev fund
- Bounty FSM: Open → Claimed → Submitted → UnderReview → (Accepted | Rejected | Disputed | Revision) → (Paid | Expired | Cancelled)
- Reputation: 5 weighted factors — stake 30%, attest 25%, activity 20%, verify 15%, tenure 10%
- `agnetd` exit codes: 1 general, 2 usage, 10 network, 11 timeout, 20 auth, 30 insufficient, 40 not-found, 50 rate-limited, 60 conflict
- `neunode-verification` features: `gauntlet`/`spot-check`/`repops` default on; `tee-sim`, `tee-intel` (DCAP), `tee-amd` (SEV-SNP + pinned VLEK) opt-in. Conformance vectors are committed as `.hex`/`.pem`/`.json` fixtures alongside the source
- RocksDB takes a single-process file lock — only one `agnetd` can hold a DB. Run the daemon and point CLI/SDK at HTTP rather than opening a second process. See `KNOWN_ISSUES.md`
- `KNOWN_ISSUES.md` documents live gaps (unvalidated bounty escrow, no `active_identity` config setter) — check it before "fixing" surprising behavior
- E2E tests are sequential (one shared Anvil fork). Known flaky: `bounty.e2e.ts > Review System > submitReview`
- Cross-compiling `agnetd` needs `libclang-dev` + `llvm-dev` + correct `BINDGEN_EXTRA_CLANG_ARGS` (vendored OpenSSL)
- This repo uses **beads (`bd`)** for issue tracking, not markdown TODOs — see the tracker section in `AGENTS.md`
