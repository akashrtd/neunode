# PROJECT KNOWLEDGE BASE

**Generated:** 2026-08-16
**Commit:** ccf0776
**Branch:** main

## OVERVIEW

Neunode is a protocol for decentralized AI-agent networks — DID identity, stake-gated participation,
verifiable work, resource economics. CLI-first, machine-parseable, protocol-driven.

Three-language stack: Rust (core protocol + `agnetd` daemon/CLI), Solidity (on-chain contracts),
TypeScript (SDK + MCP server).

**Stage:** protocol-complete and tested, not a deployed network. The off-chain Rust ledger (RocksDB)
is the canonical source of truth; Solidity contracts are the on-chain migration target
(`docs/adr/0001-canonical-ledger-source-of-truth.md`). No live chain — contracts run on Anvil.
ZK verification is stubbed (`Unsupported`); TEE is simulated unless `tee-intel`/`tee-amd` is enabled.

## STRUCTURE

```
./
├── crates/          # Rust workspace — 19 lib crates + agnetd binary
├── contracts/       # Solidity — Foundry, EIP-2535 Diamond proxy (31 sources, 17 test files)
├── sdk/             # @neunode/sdk — TypeScript SDK (ESM+CJS), 16 resources, 4 transports
├── packages/        # npm distribution: @neunode/agnetd (+3 platform pkgs) and @neunode/mcp-server
├── examples/        # 3 fork-and-run agent templates (coding, research, inference provider)
├── tests/           # Rust cross-crate integration tests (10 files, own Cargo.toml)
├── docs/adr/        # Architecture Decision Records (append-only)
├── docs/            # Engineering spikes (Engine API / chain spec, Reth+Malachite, slashing)
├── research/        # 22 background design docs
├── scripts/         # install.sh, security_audit.sh
└── .github/         # 3 CI workflows (ci, contracts-ci, release)
```

Workspace members: 21 (19 libs + `agnetd` + `tests`).

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add CLI command | `crates/agnetd/src/cmd_<group>.rs` | 20 modules; register in `cli.rs::Commands` |
| Add HTTP endpoint | `crates/agnetd/src/api_<group>_api.rs` | 22 modules; register in `api_routes.rs` |
| Add shared Rust type | `crates/neunode-core/src/types.rs` | Central definitions |
| Add event kind | `crates/neunode-core/src/kind.rs` | 31 variants; category derived from numeric range |
| Change storage schema | `crates/neunode-storage/src/cf.rs` | 22 column families |
| Change binary encoding | `crates/neunode-storage/src/codec.rs` | Centralized; legacy migration path |
| Modify bounty FSM | `crates/neunode-bounty/src/state_machine.rs` | Transitions + guards |
| Add SDK resource | `sdk/src/resources/<name>.ts` | `createXResource(client)`; register in `client/client.ts` |
| Add SDK type | `sdk/src/types/<name>.ts` | Barrel export in `types/index.ts` |
| Add MCP tool | `packages/mcp-server/src/tools/<name>.ts` | Register in `tools/index.ts` |
| Add contract | `contracts/src/<domain>/` | Mirror test in `contracts/test/<domain>/` |
| Change deploy topology | `contracts/script/Deploy.s.sol` | Mirror in `sdk/src/contracts/addresses.ts` |
| Cross-crate flow test | `tests/<domain>_flow.rs` | Separate integration crate |
| E2E test (TS) | `sdk/tests/e2e/` | Anvil-based, sequential, single fork |
| Record a decision | `docs/adr/NNNN-kebab-title.md` | Append-only — supersede, don't edit |
| Interop constraints | `research/22-rust-crate-compatibility.md` | Cross-language matrix |

## RUST WORKSPACE

Dependency direction is strictly downward. `neunode-core` is the sole leaf (zero internal deps).
No cycles.

```
agnetd (binary) ── depends on 16 crates
  ├── neunode-training  → core, crypto, storage, p2p, bounty, token, feed
  ├── neunode-bounty    → core, crypto, identity, storage, feed, token, reputation
  ├── neunode-inference → core, crypto, identity, storage, feed, token
  ├── neunode-knowledge → core, crypto, storage
  ├── neunode-token     → core, crypto, identity, storage
  ├── neunode-feed      → core, crypto, identity, storage
  ├── neunode-p2p       → core, crypto, identity
  ├── neunode-reputation→ core, crypto, identity
  ├── neunode-identity  → core, crypto
  ├── neunode-storage   → core, crypto
  ├── neunode-lineage / neunode-verification → core, crypto
  ├── neunode-discovery / neunode-turboquant → core
  └── neunode-crypto    → core
```

Not wired into `agnetd`:
- `neunode-contracts` — `alloy sol!` bindings for the Solidity contracts (→ core)
- `neunode-chain-spec`, `neunode-engine-api-client` — sovereign-L1 spike (Reth + Malachite), zero
  internal deps. See ADR-0002; the recommendation is to ship on an existing L2 first.

| Crate | Responsibility |
|---|---|
| `neunode-core` | Types, `Kind` taxonomy, config, constants, errors |
| `neunode-crypto` | Ed25519, secp256k1, EIP-712, AEAD, hashing |
| `neunode-identity` | DID, keyring, agent card, DID documents, on-chain identity |
| `neunode-storage` | RocksDB (22 CFs), moka cache, codec, per-domain stores |
| `neunode-p2p` | libp2p: gossipsub, Kademlia DHT, peer auth/scoring, catchup, compression, private feeds |
| `neunode-feed` | Sigchain, events, schemas, filters, topics, rate limiting |
| `neunode-token` | Balances, staking, decay, mint/burn, constant-product AMM |
| `neunode-reputation` | 5-factor scoring, attestations |
| `neunode-bounty` | Lifecycle FSM, escrow, review, verification |
| `neunode-inference` | OpenAI-compatible API, routing, providers, settlement (incl. streaming), disputes |
| `neunode-training` | Distributed training: coordinator (sync + async), workers, gradients, aggregation, fault handling, settlement |
| `neunode-turboquant` | Model compression: adaptive strategy, codebooks, int8, rotation, MSE |
| `neunode-knowledge` | RDF-style knowledge graph: triples, ontology, query, dictionary, authorization |
| `neunode-lineage` | Model provenance DAG, royalties, sigchain |
| `neunode-discovery` | Agent search, complement matching, capability gaps, scoring |
| `neunode-verification` | Tiered verification: gauntlet, spot-check, RepOps, bisection, TEE (Intel TDX / AMD SEV-SNP), ZK (stub) |
| `neunode-contracts` | `alloy sol!` Rust bindings for Solidity contracts |
| `neunode-chain-spec` | L1 genesis, gas token, predeploys, EIP-1559 params (spike) |
| `neunode-engine-api-client` | Engine API JSON-RPC client with JWT auth (spike) |
| `agnetd` | clap 4 CLI (21 command groups) + axum HTTP/WS daemon + ratatui dashboard |

### agnetd: two surfaces over one core

`cmd_*.rs` (CLI) and `api_*.rs` (HTTP) are parallel front-ends dispatching into the same crate
logic. Adding a capability to one does **not** add it to the other, and nothing catches the drift at
compile time — the SDK and MCP server both consume the HTTP surface, so add both.

- CLI groups: identity, config, init, mesh, feed, model, train, bounty, token, security, lifecycle,
  reputation, inference, knowledge, lineage, verify, discover, turboquant, dashboard, serve, version
- HTTP: ~80 routes under `/api/v1`, plus `/swagger-ui` and `/api-docs/openapi.json` (utoipa) and a
  WebSocket feed stream
- `agnetd serve` default bind: `127.0.0.1:8080`. `examples/` and `packages/mcp-server/` default to
  **41000** — override with `--port` / `NEUNODE_URL`
- Exit codes: 1 general · 2 usage · 10 network · 11 timeout · 20 auth · 30 insufficient ·
  40 not-found · 50 rate-limited · 60 conflict

## TYPESCRIPT SDK

`createNeunodeClient(config)` accepts four transports:

| Transport | Use |
|---|---|
| `http` | **Primary.** REST against a running `agnetd serve` |
| `cli` | Subprocess `agnetd --output json-compact`; resolves `target/release/agnetd` → `target/debug/agnetd` |
| `viem` | Direct on-chain reads/writes. Optional peer dep `viem >= 2.47`; externalized from the bundle |
| `mock` | In-memory, HTTP-compatible; for development and tests |

16 resources, all `createXResource(client)` — they take the **client**, not a transport.

- HTTP-first with CLI fallback: bounty, config, discovery, feed, inference, knowledge, mesh, model,
  reputation, token, train, turboquant
- HTTP-only (throws without an HTTP transport): identity, lifecycle, lineage, verification

Envelope on both HTTP and CLI:
`{ data: T, success: true } | { error: { code, message }, success: false }`.

## CROSS-LANGUAGE CODEGEN

Two generated artifacts, both gated in CI. Never hand-edit them.

| Artifact | Generator | Check |
|---|---|---|
| `sdk/src/types/protocol.generated.ts` | `cargo run -p neunode-core --example emit_sdk_protocol` | `npm run check:protocol` |
| `sdk/src/contracts/abi/*.ts` (19) | extracted from `contracts/out` | `npm run check:abi` (needs `forge build`) |

Also:
- Rust ↔ Solidity: `alloy sol!` in `neunode-contracts` for ABI + EIP-712 signing
- TS ↔ Solidity: viem ABI encoding (alloy-compatible)
- `ts-rs` `#[ts(export)]` derives still emit per-crate `bindings/` during `cargo test` — gitignored,
  and *not* the SDK's source of truth. `#[serde(with)]` fields need a matching `#[ts(as = "...")]`
- Ed25519 in TS: **must** be `@noble/ed25519 v3` — never tweetnacl (malleability bug)
- JS transport: TCP + Noise only. QUIC and TLS 1.3 are not supported in a JS context

## CONVENTIONS

### Rust
- Toolchain pinned to 1.93 (`rust-toolchain.toml`). Edition 2021.
- `max_width = 100`, `use_small_heuristics = "Max"` (`rustfmt.toml`). Cognitive complexity 30 (`clippy.toml`).
- Errors: `thiserror` derive in each crate's `error.rs`, `Result<T, CrateError>`. `anyhow` only in `agnetd`.
- Flat `src/*.rs` — no `mod.rs`, no subdirectories. Entry point is `lib.rs` or `main.rs`.
- Tests inline: `#[cfg(test)] mod tests` at the bottom of each source file. `proptest` for properties.
- Cross-crate integration tests live in the top-level `tests/` crate, not per-crate.
- Shared deps via `[workspace.dependencies]` — declare with `workspace = true`.

### TypeScript
- ESM, `strict: true`, `noUncheckedIndexedAccess`. No `as any`, `@ts-ignore`, `@ts-expect-error`.
- Branded ID types: `Did`, `CID`, `PeerId`, `BountyId` — never bare strings.
- Colocated `*.test.ts`; integration and e2e under `sdk/tests/` with their own vitest configs.
- Biome has **no config file** — defaults apply (tabs, double quotes). Match surrounding style.
- Node `>= 20.11` for the SDK; CI runs Node 22.

### Solidity
- 0.8.28, optimizer 200 runs, `via_ir = true`, `forge fmt` line length 100, 4-space indent.
- Custom errors, not revert strings. NatSpec on all public/external functions.
- EIP-2535 Diamond proxy for upgradeability; OpenZeppelin for tokens, governance, access control.
- Foundry pinned to **v1.5.1** — unpinned `stable` drifts fuzz gas medians and flakes `forge snapshot --check`.

### Commits
Conventional commits with a scope: `feat(verification):`, `fix(sdk):`, `refactor(storage):`,
`chore(deps):`, `ci(security):`.

## ANTI-PATTERNS (THIS PROJECT)

- **NEVER** use `as any`, `@ts-ignore`, or `@ts-expect-error` in TypeScript
- **NEVER** use `tweetnacl` for Ed25519 — use `@noble/ed25519 v3`
- **NEVER** use `anyhow` in library crates — `thiserror` only
- **NEVER** add `mod.rs` or subdirectories in Rust crates — flat `src/*.rs` only
- **NEVER** hand-edit `sdk/src/types/protocol.generated.ts` or `sdk/src/contracts/abi/*.ts`
- **NEVER** modify `contracts/lib/` (vendored forge-std + openzeppelin)
- **NEVER** use QUIC or TLS 1.3 transport in a JS context
- **NEVER** add a `cargo-audit` ignore without a matching `assert_unreachable` guard in `scripts/security_audit.sh`
- **NEVER** mix data and instructions in agent inputs (sandboxed parsing invariant)
- **NEVER** let a delegated key exceed its delegator's permissions (monotonically decreasing capabilities)
- **NEVER** commit `sdk/dist/`, `contracts/out/`, `contracts/cache/`, `target/`, `bindings/`, or `packages/agnetd-*/agnetd*`

## COMMANDS

```bash
# Rust
cargo build                              # Debug build workspace
cargo build --release -p agnetd          # Release binary
cargo test --workspace                   # All tests (~2,800 test fns)
cargo fmt --check                        # CI enforced
cargo clippy --workspace -- -D warnings  # CI enforced
cargo check --workspace --all-features   # CI enforced
./scripts/security_audit.sh              # CI enforced

# TypeScript SDK
cd sdk && npm run build                  # tsup ESM+CJS+dts
cd sdk && npm test                       # Unit tests (vitest)
cd sdk && npm run test:integration       # Against a real agnetd binary
cd sdk && npm run test:e2e               # Anvil-based contract tests
cd sdk && npm run typecheck              # tsc --noEmit strict
cd sdk && npm run lint                   # biome check src/
cd sdk && npm run check:protocol         # Rust→TS drift gate
cd sdk && npm run check:abi              # Solidity→TS drift gate

# MCP server
cd packages/mcp-server && npm run typecheck && npm test && npm run build

# Solidity
cd contracts && forge build --sizes      # Build with size report
cd contracts && forge test -vvv          # All Forge tests (397, verified)
cd contracts && forge fmt --check        # CI enforced
cd contracts && forge snapshot --check   # Gas snapshot (CI enforced)

# Run a node
agnetd init                              # Config wizard → ~/.agnetd/config.toml
agnetd serve                             # 127.0.0.1:8080 · /api/v1 · /swagger-ui
agnetd dashboard                         # ratatui TUI
```

## CI

- **ci.yml** (push/PR → `main`), 6 jobs: `lint` (fmt + clippy), `test` (`cargo test --workspace`),
  `check` (`--all-features`), `security` (`scripts/security_audit.sh`), `sdk`, `mcp-server`.
  The `sdk` job builds `agnetd` and runs `forge build` first, then: build, typecheck,
  typecheck+lint of `examples/`, protocol drift, ABI drift, lint, unit, integration, e2e.
- **contracts-ci.yml** (only on `contracts/**`): `forge fmt --check`, `forge build --sizes`,
  `forge test -vvv`, `forge snapshot --check`. Foundry pinned v1.5.1.
- **release.yml** (on `v*` tags): cross-compile `agnetd` for linux-x64 / linux-arm64 / darwin-arm64
  → GitHub Release → publish `@neunode/agnetd` plus 3 platform packages to npm.

## NOTES

- Feed event key: `[agent_did_hash(16B) | sequence(u64 BE)]` — per-agent sequential scan.
- Knowledge graph: 6 SPOG-style CF indexes (`KG_SPOG/POSG/OSPG/GSPO/GPOS/GOSP`, Oxigraph pattern)
  plus a `KG_ID2STR` dictionary.
- `Kind`: 31 variants; the category is derived from the numeric range — 0–99 System, 1000s Bounty,
  2000s Training, 3000s Attestation, 4000s Inference, 5000s Governance, 9000s Custom.
- Token decay: exponential, activity-tiered (0% / 2% / 5% / 15% / 50%), never reaches zero.
  Redistribution: 40% treasury / 30% staking / 20% burn / 10% dev fund.
- Bounty FSM: Open → Claimed → Submitted → UnderReview → Revision → Accepted/Rejected/Disputed →
  Paid/Expired/Cancelled.
- Reputation: stake 30% + attest 25% + activity 20% + verify 15% + tenure 10%.
- `neunode-verification` features: `gauntlet`, `spot-check`, `repops` on by default; `tee-sim`,
  `tee-intel` (DCAP), `tee-amd` (SEV-SNP + pinned VLEK) opt-in. Conformance vectors are committed as
  `.hex` / `.pem` / `.json` fixtures next to the source.
- On-chain surface includes ERC-4337 gas sponsorship (`AgentPaymaster`), a resource AMM
  (`ResourceAMM`), `StakingEscrow`, `NeunodeSlashing`, and `NeunodeReputation` in addition to
  identity / bounty / escrow / review / governance / royalties / 4 resource tokens.
- RocksDB holds a single-process file lock — one `agnetd` per DB. Run the daemon and drive it over
  HTTP rather than opening a second process.
- `KNOWN_ISSUES.md` tracks live gaps (bounty escrow not validated at creation, no `active_identity`
  config setter, RocksDB lock). Read it before "fixing" surprising behavior.
- E2E tests need `anvil` and run sequentially against one shared fork. Known flaky:
  `bounty.e2e.ts > Review System > submitReview`.
- `sdk/docs/` (TypeDoc output) and per-crate `bindings/` are generated and gitignored — do not commit.
- Cross-compiling `agnetd` needs `libclang-dev` + `llvm-dev` and correct `BINDGEN_EXTRA_CLANG_ARGS`
  (vendored OpenSSL).

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
Use 'bd' for task tracking
