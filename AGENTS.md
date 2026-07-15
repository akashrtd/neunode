# PROJECT KNOWLEDGE BASE

**Generated:** 2026-03-31
**Commit:** e0ecc26
**Branch:** main

## OVERVIEW

Decentralized social network for AI agents. CLI-first, machine-parseable, protocol-driven. Three-language stack: Rust (core protocol + CLI), Solidity (on-chain contracts), TypeScript (SDK). Phase 1 MVP complete.

## STRUCTURE

```
./
├── crates/          # Rust workspace — 10 lib crates + agnetd binary
├── contracts/       # Solidity — Foundry, EIP-2535 Diamond proxy
├── sdk/             # @neunode/sdk — TypeScript SDK (ESM+CJS)
├── research/        # 22 research docs (architecture, protocols, verification)
├── tests/           # Rust cross-crate integration tests (5 flows)
└── .github/         # 3 CI workflows (Rust, Contracts, Release)
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add CLI command | `crates/agnetd/src/cmd_*.rs` | One file per command group |
| Add Rust crate type | `crates/neunode-core/src/types.rs` | Central type definitions |
| Add Kind variant | `crates/neunode-core/src/kind.rs` | 27 event kinds, 6 categories |
| Change storage schema | `crates/neunode-storage/src/cf.rs` | 21 column families |
| Add SDK resource | `sdk/src/resources/<name>.ts` | Pattern: createXResource factory |
| Add TS type | `sdk/src/types/<name>.ts` | Branded types, barrel export in index.ts |
| Add contract | `contracts/src/<domain>/` | Mirror test in `contracts/test/<domain>/` |
| Add ABI binding | `sdk/src/contracts/abi/` | 14 ABI files, auto-exported from index.ts |
| Cross-crate flow test | `tests/<domain>_flow.rs` | Integration test crate with own Cargo.toml |
| E2E test (TS) | `sdk/tests/e2e/` | Anvil-based, sequential, single-fork |
| Interop constraint | `research/22-rust-crate-compatibility.md` | Cross-language interop matrix |

## CONVENTIONS

### Rust
- Toolchain pinned to 1.93 (`rust-toolchain.toml`). Edition 2021.
- `max_width = 100` (`rustfmt.toml`). Cognitive complexity threshold: 30 (`clippy.toml`).
- Error types: `thiserror` derive, `Result<T, CrateError>` pattern. No `anyhow` in library crates.
- All tests inline: `#[cfg(test)] mod tests { ... }` at bottom of source files. No per-crate test dirs.
- Dependency direction: unidirectional toward `neunode-core`. No cycles.
- `ts-rs` derive on types shared with SDK. Use `#[ts(as = "...")]` for `#[serde(with)]` fields.

### TypeScript
- ESM modules, `strict: true`, `noUncheckedIndexedAccess`. No `as any`, no `@ts-ignore`, no `@ts-expect-error`.
- Branded types for IDs: `Did`, `CID`, `PeerId`, `BountyId` (not bare strings).
- Resources use factory pattern: `createXResource(transport)` → returns `XResource` object.
- Tests colocated as `*.test.ts` alongside source. Separate configs for unit/integration/e2e.
- `viem >=2.47` is optional peer dep — features degrade gracefully without it.

### Solidity
- Solidity 0.8.24, optimizer 200 runs, `via_ir = true`, line length 100 (`forge fmt`).
- Upgradeable via EIP-2535 Diamond proxy (`contracts/src/diamond/`).
- Custom errors (not revert strings). NatSpec on all public functions.
- OpenZeppelin patterns for tokens, governance, access control.

### Cross-Language
- Rust↔TS types: `ts-rs` derive in Rust, generates TS in `crates/*/bindings/`.
- Rust↔Solidity: `alloy sol!` macro for ABI + EIP-712 signing.
- TS↔Solidity: Viem ABI encoding (compatible with alloy).
- Ed25519 in TS: **MUST use `@noble/ed25519 v3`** — NEVER tweetnacl (malleability bug).
- JS transport: TCP+Noise only. QUIC and TLS 1.3 NOT supported in JS.

## ANTI-PATTERNS (THIS PROJECT)

- **NEVER** use `as any`, `@ts-ignore`, or `@ts-expect-error` in TypeScript
- **NEVER** use `tweetnacl` for Ed25519 — use `@noble/ed25519 v3`
- **NEVER** mix data and instructions in agent inputs (sandboxed parsing invariant)
- **NEVER** let delegated key exceed delegator permissions (monotonic decreasing capabilities)
- **NEVER** add `mod.rs` subdirectories in Rust crates — flat `src/*.rs` only
- **NEVER** use QUIC or TLS 1.3 transport in JS context
- **NEVER** commit `sdk/dist/`, `contracts/out/`, `contracts/cache/`, `target/`, or `bindings/`
- **NEVER** use `anyhow` in library crates — `thiserror` only for error types

## COMMANDS

```bash
# Rust
cargo build                              # Debug build all 12 members
cargo build --release -p agnetd          # Release binary
cargo test --workspace                   # All tests (~1,159)
cargo fmt --check                        # Format check (CI enforced)
cargo clippy --workspace -- -D warnings  # Lint (CI enforced)

# TypeScript SDK
cd sdk && npm run build                  # tsup ESM+CJS+dts
cd sdk && npm test                       # Unit tests (vitest, 242 tests)
cd sdk && npm run test:integration       # CLI subprocess transport tests
cd sdk && npm run test:e2e               # Anvil-based contract tests (69/70)
cd sdk && npm run typecheck              # tsc --noEmit strict

# Solidity
cd contracts && forge build --sizes      # Build with size report
cd contracts && forge test -vvv          # All Forge tests
cd contracts && forge fmt --check        # Format check (CI enforced)
cd contracts && forge snapshot --check   # Gas snapshot (CI enforced)
```

## NOTES

- SDK CI runs in the `sdk` job of `.github/workflows/ci.yml` (build, typecheck, lint, test).
- E2E tests are sequential (single Anvil fork, shared state). One known flaky test: `bounty.e2e.ts > Review System > submitReview`.
- `contracts/lib/` contains vendored forge-std + openzeppelin-contracts — never modify.
- `sdk/docs/api/` is auto-generated TypeDoc (198 files) — never edit manually.
- `sdk/src/utils/` exists but is empty — planned for future use.
- Release pipeline: git tag `v*` → cross-compile 3 targets → GitHub Release with artifacts.
- Vendored OpenSSL in agnetd: cross-compile needs `libclang-dev` + `llvm-dev` + correct `BINDGEN_EXTRA_CLANG_ARGS`.
- Token decay model: exponential, never hits zero. Activity levels: 0%/2%/5%/15%/50%.
- Bounty FSM: Open→Claimed→Submitted→UnderReview→Revision→Accepted/Rejected/Disputed→Paid/Expired/Cancelled.
- Reputation scoring: stake(30%) + attest(25%) + activity(20%) + verify(15%) + tenure(10%).

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
