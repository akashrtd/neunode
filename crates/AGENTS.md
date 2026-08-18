# CRATES — Rust Workspace

## OVERVIEW

19 library crates + the `agnetd` binary. `neunode-core` is the foundation and the sole leaf (zero
internal deps). Flat `src/*.rs` structure — no `mod.rs`, no subdirectories, in any crate.

Cross-crate integration tests live in the top-level `/tests/` crate, not here.

## LAYERS

```
crates/
├── neunode-core/               # Types, Kind taxonomy, config, constants, errors      (0 internal deps)
├── neunode-crypto/             # Ed25519, secp256k1, EIP-712, AEAD, hashing           → core
├── neunode-identity/           # DID, keyring, agent card, documents, on-chain ident  → core, crypto
├── neunode-storage/            # RocksDB (22 CFs), moka cache, codec, domain stores   → core, crypto
├── neunode-p2p/                # libp2p: gossipsub, Kad DHT, peer auth/score,
│                               #   catchup, compression, private feeds                → core, crypto, identity
├── neunode-feed/               # Sigchain, events, schema, filter, topics, ratelimit  → core, crypto, identity, storage
├── neunode-token/              # Balance, staking, decay, mint/burn, AMM              → core, crypto, identity, storage
├── neunode-reputation/         # 5-factor scoring, attestations                       → core, crypto, identity
├── neunode-bounty/             # Lifecycle FSM, escrow, review, verification          → +feed, token, reputation
├── neunode-inference/          # OpenAI compat, router, provider, settlement,
│                               #   streaming settlement, disputes                     → core, crypto, identity, storage, feed, token
├── neunode-training/           # Coordinator (sync/async), worker, gradient,
│                               #   aggregator, distribution, fault, settlement        → +p2p, bounty
├── neunode-turboquant/         # Compression: adaptive, codebook, int8, rotation, mse → core
├── neunode-knowledge/          # Triples, ontology, query, dictionary, authorization  → core, crypto, storage
├── neunode-lineage/            # Provenance DAG, royalty, sigchain                    → core, crypto
├── neunode-discovery/          # Search, complement, gap, scoring                     → core
├── neunode-verification/       # Gauntlet, spot-check, repops, bisection, TEE, ZK     → core, crypto
├── neunode-contracts/          # alloy sol! bindings for Solidity contracts           → core        (not used by agnetd)
├── neunode-chain-spec/         # L1 genesis, gas, predeploys (spike)                  → (none)      (not used by agnetd)
├── neunode-engine-api-client/  # Engine API JSON-RPC + JWT (spike)                    → (none)      (not used by agnetd)
└── agnetd/                     # CLI + HTTP/WS daemon + TUI dashboard                 → 16 crates
```

Dependencies flow downward only. No cycles.

`neunode-chain-spec` and `neunode-engine-api-client` back the sovereign-L1 spike (Reth + Malachite)
described in `docs/adr/0002-consensus-strategy.md`. They compile in CI but are not wired into the
daemon — don't assume changes there affect runtime behavior.

## AGNETD — TWO SURFACES, ONE CORE

`agnetd` exposes every capability twice:

| Surface | Files | Registry |
|---|---|---|
| CLI | `src/cmd_<group>.rs` (20) | `src/cli.rs::Commands` (21 groups) |
| HTTP | `src/api_<group>_api.rs` (22) | `src/api_routes.rs` (~80 routes under `/api/v1`) |

Both dispatch into the same crate logic. **Adding one does not add the other, and nothing catches
the drift at compile time.** The TypeScript SDK and the MCP server consume the HTTP surface, so a
CLI-only addition is invisible to them.

Supporting modules: `api.rs` (utoipa `OpenApi` doc), `api_state.rs` / `api_types.rs` / `api_error.rs`
(shared envelope + error mapping), `state.rs`, `config.rs`, `output.rs` (human/JSON rendering),
`feed_wire.rs` / `token_wire.rs` (wire types), `bounty_service.rs` / `turboquant_service.rs`
(service layer), `mesh_handle.rs`, `testutil.rs`.

`cmd_serve.rs` also serves `/swagger-ui`, `/api-docs/openapi.json`, and a WebSocket feed stream.
Default bind is `127.0.0.1:8080`.

Exit codes (`src/error.rs`): 1 general · 2 usage · 10 network · 11 timeout · 20 auth ·
30 insufficient · 40 not-found · 50 rate-limited · 60 conflict.

## WHERE TO LOOK

| Task | File | Notes |
|------|------|-------|
| Add shared type | `neunode-core/src/types.rs` | Central. `ts-rs` derives for SDK export. |
| Add event kind | `neunode-core/src/kind.rs` | 31 variants; category derived from numeric range |
| Add crypto primitive | `neunode-crypto/src/<name>.rs` | ed25519, secp256k1, eip712, aead, hash |
| Change storage layout | `neunode-storage/src/cf.rs` | 22 column families |
| Change binary encoding | `neunode-storage/src/codec.rs` | Centralized; has a legacy migration path |
| Add feed event field | `neunode-feed/src/event.rs` | Schema validation in `schema.rs` |
| Modify bounty FSM | `neunode-bounty/src/state_machine.rs` | Transitions + guards; lifecycle in `lifecycle.rs` |
| Add a TEE backend | `neunode-verification/src/tee_*.rs` | Feature-gated; commit a conformance vector |
| Add CLI command | `agnetd/src/cmd_<name>.rs` | Register in `cli.rs::Commands` |
| Add HTTP endpoint | `agnetd/src/api_<name>_api.rs` | Register in `api_routes.rs`; annotate for utoipa |
| Add CLI error variant | `agnetd/src/error.rs` | Map to an existing exit code |
| TS type bindings | `crates/*/bindings/` | Generated by `ts-rs` during `cargo test`; gitignored |

## CONVENTIONS

- Entry point: `lib.rs` (libraries) or `main.rs` (`agnetd`). No `mod.rs`, no subdirectories.
- Errors: `thiserror` derive in each crate's `error.rs`, `Result<T, CrateError>`. `anyhow` is
  permitted **only** in `agnetd` — it is the sole crate that depends on it.
- Tests: inline `#[cfg(test)] mod tests` at the bottom of each source file. `proptest` for properties.
- Shared deps come from `[workspace.dependencies]` in the root `Cargo.toml` — declare `workspace = true`.
- Toolchain pinned 1.93; `max_width = 100`; cognitive complexity threshold 30.
- `ts-rs` derive (`#[ts(export)]`) on types shared with the SDK. `#[serde(with)]` fields need a
  matching `#[ts(as = "...")]`.

## FEATURE FLAGS

`neunode-verification` is the only crate with meaningful features:

| Feature | Default | Effect |
|---|---|---|
| `gauntlet`, `spot-check`, `repops` | on | Verification tiers |
| `tee-sim` | off | Simulated TEE evidence |
| `tee-intel` | off | Intel TDX via DCAP (`dcap-qvl`) |
| `tee-amd` | off | AMD SEV-SNP with pinned VLEK (`sev`, `x509-cert`, `der`, `openssl`) |

CI runs `cargo check --workspace --all-features`, so feature-gated code must compile in every
combination. `scripts/security_audit.sh` asserts that the RSA backend pulled in by `tee-intel` stays
unreachable — keep that guard honest when touching TEE deps.

## NOTES

- The SDK's protocol types are generated by `cargo run -p neunode-core --example emit_sdk_protocol`,
  not by the `ts-rs` `bindings/` output. Changing exported core types requires
  `cd sdk && npm run generate:protocol` or CI fails on drift.
- `neunode-training` is the largest library crate (~6.3k LOC); `agnetd` is ~22k LOC across 56 files.
- RocksDB takes a single-process file lock — tests and tools that open a DB cannot run concurrently
  against the same path.
