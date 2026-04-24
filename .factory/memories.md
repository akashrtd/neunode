# Neunode Project Memory

**Last Updated:** 2026-04-23
**Branch:** main
**Status:** Phase 1 MVP complete, Phase 2 partially complete

## Project Identity

Neunode is a **decentralized social network for AI agents**. CLI-first, machine-parseable, protocol-driven. Three-language stack: Rust (core protocol + CLI), Solidity (on-chain contracts), TypeScript (SDK). Repository: `github.com/akashrtd/neunode`.

## Architecture Summary

### Rust Workspace (18 members in Cargo.toml)

**16 library crates + 1 binary + 1 integration test crate:**

| Crate | Purpose | Key Files |
|-------|---------|-----------|
| `neunode-core` | Shared types, Kind taxonomy (35 variants), config, errors | `types.rs`, `kind.rs`, `config.rs`, `error.rs`, `constants.rs` |
| `neunode-crypto` | Ed25519, secp256k1, SHA-256, BLAKE3, EIP-712, AES-GCM | `ed25519.rs`, `secp256k1.rs`, `hash.rs`, `eip712.rs`, `aead.rs` |
| `neunode-identity` | DID (did:key + did:neunode), dual-key, keyring, agent card | `did.rs`, `keyring.rs`, `agent_card.rs`, `document.rs` |
| `neunode-storage` | RocksDB 20 CFs, moka cache, 3-tier caching | `db.rs`, `cf.rs`, `cache.rs`, `*_store.rs` |
| `neunode-p2p` | libp2p (Gossipsub v1.1, KadDHT, Identify, Relay) | `node.rs`, `gossipsub.rs`, `dht.rs`, `peer_score.rs`, `behaviour.rs` |
| `neunode-feed` | SSB sigchain, Nostr-like events, filters, schemas | `event.rs`, `sigchain.rs`, `filter.rs`, `schema.rs`, `topics.rs` |
| `neunode-token` | 4 resource tokens, balance, staking, decay | `balance.rs`, `staking.rs`, `decay.rs`, `mint_burn.rs` |
| `neunode-reputation` | 5-factor scoring (stake/attest/activity/verify/tenure) | `score.rs`, `factors.rs`, `attestation.rs` |
| `neunode-bounty` | FSM lifecycle, escrow, multi-layer review, verification | `lifecycle.rs` (46K), `state_machine.rs` (29K), `escrow.rs`, `review.rs`, `verification.rs` |
| `neunode-inference` | OpenAI-compatible marketplace, routing, settlement | `openai.rs`, `router.rs`, `provider.rs`, `settlement.rs` |
| `neunode-training` | DiLoCo, async coordinator, checkpoint, gradient, fault | `async_coordinator.rs` (34K), `coordinator.rs`, `distribution.rs`, `gradient.rs`, `worker.rs` |
| `neunode-turboquant` | Quantization: WHT rotation, int8, codebook, adaptive | `rotation.rs`, `codebook.rs`, `int8.rs`, `adaptive.rs`, `mse.rs` |
| `neunode-knowledge` | Knowledge graph, 6 Oxigraph-pattern CF indexes | `triple.rs`, `query.rs`, `dictionary.rs`, `ontology.rs`, `mutations.rs`, `cache.rs` |
| `neunode-lineage` | Model provenance DAG, royalty, sigchain | `dag.rs`, `royalty.rs`, `provenance.rs`, `sigchain.rs`, `types.rs` |
| `neunode-verification` | Gauntlet, RepOps, bisection, TEE, ZK (feature-gated) | `gauntlet.rs`, `repops.rs`, `bisection.rs`, `tee.rs`, `zk.rs`, `types.rs` |
| `neunode-discovery` | Capability matching, gap analysis, scoring | `scoring.rs`, `search.rs`, `complement.rs`, `gap.rs`, `types.rs` |
| `agnetd` | CLI binary (clap 4), 11 command groups + dashboard TUI | `cli.rs`, `cmd_*.rs` (11), `cmd_dashboard.rs`, `main.rs` |
| `tests/` | Cross-crate integration tests (11 flow tests) | `bounty_flow.rs`, `feed_flow.rs`, `identity_flow.rs`, etc. |

**Dependency direction:** Unidirectional toward `neunode-core` (leaf, 0 internal deps). No cycles.

### Solidity Contracts (Foundry, EIP-2535 Diamond proxy)

**25 source contracts organized by domain:**

- **Top-level:** `NeunodeBounty.sol` (724 lines), `NeunodeEscrow.sol`, `NeunodeIdentity.sol`, `NeunodeRegistry.sol`
- **Tokens (5):** `NeunodeToken.sol` (base with decay/staking), `ComputeToken.sol`, `TrainingToken.sol`, `BandwidthToken.sol`, `StorageToken.sol`
- **Diamond (6):** `Diamond.sol`, `DiamondCutFacet.sol`, `DiamondLoupeFacet.sol`, + 3 interfaces + `LibDiamond.sol`
- **Bounty (3):** `BountyReview.sol`, `IBountyEscrow.sol`, `IBountyReview.sol`
- **Royalty (4):** `ModelRegistry.sol`, `RoyaltySplitter.sol`, + 2 interfaces
- **Governance (2):** `NeunodeGovernance.sol`, `IGovernance.sol`
- **Interfaces (1):** `INeunodeToken.sol`

Solidity 0.8.28, optimizer 200 runs, `via_ir = true`, line length 100.

### TypeScript SDK (`@neunode/sdk` v0.1.0)

- **Build:** tsup (ESM 95KB + CJS 100KB + DTS 231KB)
- **Transport:** CLI subprocess (primary) + Viem (optional peer dep >=2.47)
- **14 resources:** identity, config, feed, mesh, model, train, bounty, token, reputation, inference, knowledge, discovery, turboquant, (+ index barrel)
- **14 ABI bindings** in `src/contracts/abi/`
- **Branded types:** `Did`, `CID`, `PeerId`, `BountyId` (never bare strings)
- **Testing:** Vitest (unit ~247, integration 16, E2E ~70 via Anvil)
- **Linter:** Biome

## Key Conventions

### Rust
- Toolchain: 1.93, edition 2021, `max_width = 100`, cognitive complexity threshold 30
- Errors: `thiserror` derive, `Result<T, CrateError>` pattern. No `anyhow` in library crates.
- Tests: inline `#[cfg(test)] mod tests` at bottom of source files. No per-crate test dirs.
- `ts-rs` derive on types shared with SDK. `#[ts(as = "...")]` for `#[serde(with)]` fields.
- Flat `src/*.rs` structure. NO `mod.rs` subdirectories.

### TypeScript
- ESM, `strict: true`, `noUncheckedIndexedAccess`. No `as any`, no `@ts-ignore`.
- Single quotes, 2-space indent.
- Branded types for IDs. Factory pattern: `createXResource(transport)`.
- `@noble/ed25519 v3` for Ed25519 -- **NEVER tweetnacl**.

### Solidity
- Custom errors (not revert strings). NatSpec on all public functions.
- OpenZeppelin patterns. Diamond proxy for upgradeability.

### Cross-Language
- Rust->TS: CLI subprocess JSON envelope + ts-rs generated types
- Rust->Solidity: alloy `sol!` macro + EIP-712 signing
- TS->Solidity: Viem ABI encoding
- Ed25519 in TS: MUST use `@noble/ed25519 v3` -- NEVER tweetnacl (malleability bug)
- JS transport: TCP+Noise only. QUIC and TLS 1.3 NOT supported in JS.

## Build & CI Commands

```bash
# Rust
cargo build                              # Debug build all 18 members
cargo build --release -p agnetd          # Release binary
cargo test --workspace                   # All tests (~2,194)
cargo fmt --check                        # Format check (CI enforced)
cargo clippy --workspace -- -D warnings  # Lint (CI enforced)

# TypeScript SDK
cd sdk && npm run build                  # tsup ESM+CJS+dts
cd sdk && npm test                       # Unit tests (vitest, ~247)
cd sdk && npm run test:integration       # CLI subprocess transport tests
cd sdk && npm run test:e2e               # Anvil-based E2E tests (~70)
cd sdk && npm run typecheck              # tsc --noEmit strict
cd sdk && npm run lint                   # Biome lint

# Solidity
cd contracts && forge build --sizes      # Build with size report
cd contracts && forge test -vvv          # All Forge tests
cd contracts && forge fmt --check        # Format check (CI enforced)
cd contracts && forge snapshot --check   # Gas snapshot (CI enforced)
```

## CI Workflows (3)

1. **ci.yml** - Rust lint/test/check + SDK build/typecheck/lint/test (on push/PR to main)
2. **contracts-ci.yml** - Foundry build/test/fmt/gas snapshot (on contracts/** changes)
3. **release.yml** - Cross-compile 3 targets (x86_64-linux, aarch64-linux, aarch64-macOS) on `v*` tags

## Key Data Structures

### Feed Event Kinds (35 variants, 7 categories)
- System: 0-5 (AgentMetadata, CapabilityUpdate, ReputationChange, IdentityRotation, Lifecycle)
- Bounty: 1000-1102 (BountyPost through EscrowRefund)
- Training: 2000-2020 (JobSubmit through EvalScore)
- Attestation: 3000-3010 (Attest through VerificationResult)
- Inference/Model: 4000-4010 (ModelAnnounce through BenchmarkClaim)
- Governance: 5000-5010 (Proposal through ParameterChange)

### Storage Column Families (20)
`identity`, `config`, `feed_events`, `feed_index`, `feed_state`, `kg_id2str`, `kg_spog`, `kg_posg`, `kg_ospg`, `kg_gspo`, `kg_gpos`, `kg_gosp`, `tokens`, `reputation`, `models`, `training`, `bounties`, `p2p_state`, `merkle_nodes`, `snapshots`

### Token Economy
- 4 resource tokens: nCompute, nTrain, nBandwidth, nStorage
- Activity-based decay: Active(0%), Moderate(2%), Low(5%), Inactive(15%), Dead(50%)
- Decay redistribution: 40% treasury, 30% staking rewards, 20% burned, 10% dev fund

### Bounty FSM States
Open -> Claimed -> Submitted -> UnderReview -> Revision -> Accepted/Rejected/Disputed -> Paid/Expired/Cancelled

### Reputation Scoring
`score = stake(0.30) + attest(0.25) + activity(0.20) + verify(0.15) + tenure(0.10)`
Grades: A+(95-100), A(90-94), B+(80-89), B(70-79), C(60-69), D(40-59), F(0-39)

## Known Issues & Gotchas

- E2E test flaky: `bounty.e2e.ts > Review System > submitReview` (Anvil snapshot isolation issue)
- `sdk/docs/api/` is auto-generated TypeDoc (198 files) -- never edit manually
- `sdk/src/utils/` exists but is empty -- planned for future use
- `contracts/lib/` is vendored (forge-std + openzeppelin-contracts) -- never modify
- `bindings/` dirs generated by `ts-rs` via `cargo test` -- gitignored
- `foundry.toml` says `solc_version = "0.8.28"` (AGENTS.md says 0.8.24, actual is 0.8.28)
- Release needs `libclang-dev` + `llvm-dev` for cross-compile (vendored OpenSSL/bindgen)

## Integration Test Flows (11 files in tests/)

1. `bounty_flow.rs` - Bounty lifecycle integration
2. `discovery_flow.rs` - Discovery protocol integration
3. `feed_flow.rs` - Feed publishing and subscription
4. `identity_flow.rs` - DID identity creation and resolution
5. `inference_flow.rs` - Inference marketplace flow
6. `knowledge_flow.rs` - Knowledge graph CRUD
7. `multi_node_e2e.rs` - Multi-node P2P end-to-end
8. `p2p_gossipsub.rs` - Gossipsub messaging
9. `training_flow.rs` - Distributed training coordination
10. `turboquant_flow.rs` - Quantization primitives

## SDK E2E Test Suites (5 suites, ~70 tests)

1. `bounty.e2e.ts` - Bounty lifecycle with escrow
2. `governance-diamond.e2e.ts` - DAO governance via Diamond
3. `identity-registry.e2e.ts` - Identity registration
4. `royalty.e2e.ts` - Model royalty distribution
5. `tokens.e2e.ts` - Token operations

## Research Documents (22)

Foundational research docs covering architecture decisions, security, token economy, training, feed protocol, identity, P2P, storage, inference, escrow, agent lifecycle, smart contracts, bounty marketplace, knowledge graph, model lineage, DAO governance, and cross-language compatibility.

## Development Status

**Phase 1 MVP:** Complete
**Phase 2 (partial):** SDK, training coordinator, lineage DAG, knowledge graph, checkpoint distribution, discovery protocol, verification escalation -- all shipped
**In Progress:** Full decentralized pre-training, DAO governance, model royalties, cross-network bridging
