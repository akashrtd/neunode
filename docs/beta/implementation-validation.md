# Beta remediation: implementation and validation

Date: 2026-10-04. Branch: `codex/beta-readiness-audit`.
Starting implementation revision: `ac26675`; initial runtime assessment: `cbfa58d`.
Tracking: Beads `neunode-zva`, its remediation children, and the existing L1 epic `neunode-77j`.

## Release judgment

The local agent runtime now executes provider inference, requires authorization for mutations,
persists usable identities, publishes verifiable feeds, and exchanges them between independent
nodes. The original acceptance baseline was 3 passing and 14 failing scenarios. The expanded
21-scenario real-daemon suite now passes. This is a substantial improvement in the operational
surface, with additional browser and packed-distribution checks.

A public sovereign-network beta remains unqualified. Local RocksDB remains the operational
ledger, training submissions remain metadata without a running executor, and real Reth/Malachite
multi-validator finality, chain replay/reorgs, chain-derived balances, resource-backed minting,
hardware TEE, and sustained fault/load testing have not been demonstrated. No live chain or
npm release was deployed. No claim of an exhaustive line-by-line semantic audit is made.

The initial [assessment](readiness-assessment.md) is historical evidence. Current results are in
[remediation-results.json](remediation-results.json). Remaining work is tracked in Beads rather
than treated as completed because component tests pass.

## Implemented behavior

| Boundary | Change and observable consequence |
| --- | --- |
| Local caller authority | Daemon mutations require its bearer token. Inference WebSockets accept an authenticated subprotocol. SDK, MCP, examples and dashboard supply credentials. MCP HTTP retains Host/Origin checks and checks its own token. Public reads remain public. This is an operator authority boundary; separate least-privilege credentials for every client are not implemented. |
| Identities | HTTP creation generates fresh keys, persists encrypted material, and activates the first identity. Active identity selection loads the corresponding owned keys. Changing the network identity while the mesh runs is rejected with restart guidance. Key-loading failures propagate. |
| Secret storage | Versioned AEAD storage replaces predictable hostname/username keys. An independent random master secret is created atomically with private permissions, or supplied externally. Legacy material migrates durably; plaintext is removed afterward. Missing master secrets produce backup/restore guidance. Secret files and protected directories reject symlinks. Filesystem access to both the ciphertext and master secret still grants access; this is an unattended secret-file design, not OS-vault integration. |
| Signed feed | CLI, HTTP and dashboard share canonical IDs, author signatures, timestamps, tags and previous hashes. Peer envelopes cryptographically bind Ed25519 keys to the DID's secp256k1 identity. Tampering, invalid IDs, forks and sequence gaps cannot overwrite stored history. Exact replay is idempotent. Post/reply kinds 9001/9002 are now canonical and generated into SDK types. |
| Mesh | `serve` starts and shares a real P2P actor. Topics include every protocol category. One-second heartbeats replace the five-minute default. Bounded command/event queues, bounded catchup messages and peer rate limits are enforced. Periodic head announcements recover missed events. Tests use separate processes, homes, databases and peer identities. |
| Inference | HTTP, CLI and authenticated WebSocket requests dispatch an actual bounded OpenAI-compatible provider call. Responses and usage are validated. Budget reservations, provider payout, fee and refund are atomic with a durable operation record. Idempotency keys prevent repeated execution/charging; restart recovery refunds interrupted reservations without replaying ambiguous provider calls. Receipts explicitly identify the local ledger. The provider call currently requests a complete response; incremental upstream SSE streaming remains follow-up work. |
| Money | Self-transfer no longer writes a duplicate account update that can mint value. Bootstrap grants are atomic and one-time per identity, including conservatively recognized legacy grants. Costs and fees use checked integer arithmetic across the full amount width. Decay and redistribution no longer convert `u128` to floating point. Each decay epoch rounds down, preserving a positive minimum of one; this changes the previous multi-epoch floating rounding example (1000 at 2% for three epochs: 940). This is local arithmetic hardening, not proof-backed minting or proven Rust/Solidity equivalence. |
| Safety stops | Persisted manual stops now reject token, bounty, reputation and inference operations. Authenticated HTTP/SDK/MCP controls allow inspection, trip and reset without restarting. Corrupt stop state fails closed. These controls are explicitly manual; automated anomaly monitoring and a complete agent execution sandbox remain separate work. |
| Knowledge/discovery/reputation | Caller-authorized keys and DID must match before graph mutation; CLI also rejects unowned identities. Capability names and ontology URIs normalize consistently. Discovery no longer invents stake, online status, latency or prices from array order. Reputation uses actual local stake, cryptographically verified attestations/feed history, and recorded bounty outcomes. Missing measurements are represented conservatively. These inputs are local, not chain-derived or independently measured remote uptime. |
| Training | Cryptographically random job IDs prevent concurrent identical submissions from overwriting one job. Health explicitly reports the training executor as unavailable. A queued record is not evidence that a model trained. |
| Identity rotation | Rotation verification checks old-key authorization, new-key possession, DID/key derivation, and all signatures over a versioned domain. Legacy unprovable messages fail closed. Applying rotations persistently and replay/time policies remain integration concerns. |
| Consensus library | Voting power/set configuration is validated, quorum and height arithmetic are checked, and certificates bind chain ID, genesis, epoch and validator set. Cross-domain replay and invalid totals are rejected. This does not prove a real multi-validator runtime or automatic governance/epoch integration. |
| Browser | Tab-scoped unlock/lock authorizes real mutations. Signed posts and tags persist. Dashboard history now explicitly lists all locally stored authors instead of querying the hash of an empty DID; author/mine filters intersect correctly. The token chart uses real available/staked balances rather than event counts. Global dashboard history still materializes a snapshot; scale/pagination and truthful live operational metrics remain in the operations issue. |
| Packages/releases | HTTP ESM/CJS imports work with optional `viem` absent. Contract factories/paymaster helpers move to `@neunode/sdk/contracts`; pure ABIs/addresses remain at the root. SDK and MCP use port 8080 consistently. Release jobs depend on functional/security/contract gates, use a shared six-package version, publish SDK/MCP too, distinguish beta tags, and configure native artifact tests on Linux x64, Linux ARM64 and macOS ARM64. Version preparation was tested against isolated copies of all six manifests, including invalid-tag rejection. Those hosted jobs have not been executed in this session. |
| Dependencies | Reachable `rustls` is updated to 0.23.45 and the reachable yanked `chacha20` resolution is repaired. The existing security audit passes without new ignores. Existing narrowly guarded exceptions/unmaintained-dependency warnings remain visible. |

## Validation that actually ran

The implementation was built and exercised on macOS with Rust 1.93.1, Node 26.7.0 and Foundry
1.5.1. The repository CI targets Rust 1.93 and Node 22; local success is not a substitute for
those hosted runs.

| Gate | Result |
| --- | --- |
| Rust workspace | 2,944 passed; 0 failed; 1 ignored documentation example |
| SDK built distribution/unit suite | 142 passed |
| Real daemon HTTP integration | 16 passed |
| Real daemon beta acceptance | 21 passed |
| SDK against deployed Anvil contracts | 68 passed |
| MCP unit/server suite | 58 passed |
| Solidity Foundry | 397 passed; size build, formatting and gas snapshot checks pass |
| Real Chromium dashboard | 6 checks: locked rejection, authorized mutation, signed/tagged persistence, rendered history after reload, retained tab access, lock |
| Fresh tarball installation | 6 checks: install, ESM/CJS without `viem`, authenticated SDK feed, safety controls, `npx` MCP initialize/discovery (36 tools), authenticated MCP feed |
| Static/cross-language checks | Rust formatting, clippy with denied warnings, all features; SDK/MCP strict types and builds; SDK/example lint; generated protocol and ABI drift checks pass |
| Dependency security gate | Pass, retaining 7 existing allowed warnings |

The provider fixture is a real local HTTP server, not a transport mock. It proves network dispatch,
response handling and accounting, but does not establish model quality or hardware performance.
The peer acceptance scenario proves bidirectional signed posts/replies and recovery of a 40-event
backlog exceeding a single catchup batch. It is not Byzantine cluster or wide-area-network proof.
The browser test found defects beyond JSON/API assertions. The tarball test found a mandatory
optional-dependency import that normal development installs concealed.

Reproduce the real-use checks after building the daemon:

```sh
cargo build -p agnetd
cd sdk
npm run build
npm run test:beta
npm run test:integration
npm run test:e2e
```

For browser checks, install Python Playwright/Chromium in a disposable environment, then run
`python scripts/beta_dashboard_test.py --binary target/debug/agnetd` from the repository root.
For package checks, build and pack SDK/MCP into a temporary directory, then run
`node scripts/beta_package_smoke.mjs target/debug/agnetd /tmp/neunode-sdk-0.1.0.tgz /tmp/neunode-mcp-server-0.1.0.tgz`.
Both scripts isolate the daemon home and clean up their processes. The package test downloads
runtime dependencies and runs the installed MCP executable through `npx --no-install`.

## Existing installations and compatibility

Back up the identity directory, database, configuration, and `~/.neunode/keystore/master.key`
together before migration. With `NEUNODE_KEYSTORE_KEY`, preserve that external secret instead.
Losing the master secret loses access to encrypted identities. Existing machine-derived files
are migrated when loaded; corruption or a missing modern master secret is not silently ignored.

Set a printable `NEUNODE_API_KEY` of at least 32 characters for the daemon, or use the generated
private `api-token` file next to its configuration. Pass that value as SDK `http.apiKey`, or
`NEUNODE_API_KEY` to MCP/examples. The default daemon URL is `http://127.0.0.1:8080`;
`NEUNODE_URL` overrides it. Dashboard authorization is scoped to the current tab and can be locked.
A bearer token grants the operator's configured authority; do not publish it as an agent capability.

The clean-install check covers JavaScript runtime imports. Exported viem-transport types still
reference the optional peer; a strict packed TypeScript-consumer check without that peer remains
in the distribution issue.

SDK contract-helper imports must move from the package root to `@neunode/sdk/contracts`.
Inference prices/amounts cross the JSON boundary as decimal strings. In Rust, cost calculation
now returns `Result`, knowledge mutation application requires independently trusted DID/key
arguments, and vote signing/collection requires a consensus domain. Identity rotation proofs
have a new versioned format. These compatibility changes belong in beta release notes.

## GitHub issues: checked, mapped, and left open where acceptance is unmet

The GitHub query returned 11 open issues. None is closed merely because related safeguards or
local tests were added. They are broad milestone/epic issues with the following remaining gates:

| GitHub | Related work and evidence still required |
| --- | --- |
| [#2 L1 epic](https://github.com/akashrtd/neunode/issues/2) | Track child phases through `neunode-77j`; real cluster, canonical ledger, economics and migration remain. |
| [#3 MCP and examples](https://github.com/akashrtd/neunode/issues/3) | Authenticated installed MCP works, 36 tools discovered, defaults fixed; actual published release and externally executed coding/research/provider reference workloads remain. |
| [#4 adoption roadmap](https://github.com/akashrtd/neunode/issues/4) | Onboarding and real-use foundations improved; deployed bootstrap infrastructure, public distribution, operator experience and broad hardening remain. |
| [#30 multi-validator BFT](https://github.com/akashrtd/neunode/issues/30) | Certificate domains/configuration hardened; launch at least three real validators and exercise quorum, catchup, outage and finality. |
| [#32 reputation voting power](https://github.com/akashrtd/neunode/issues/32) | Local reputation is measured now; derive authenticated chain scores, epoch weights and adversarial slashing in the running consensus network. |
| [#35 epoch set updates](https://github.com/akashrtd/neunode/issues/35) | Prove governance-driven validator changes apply at epoch boundaries across validators. |
| [#37 canonical chain runtime](https://github.com/akashrtd/neunode/issues/37) | Implement transactions and pending/finalized/reverted receipts before replacing the local ledger. |
| [#39 event synchronization](https://github.com/akashrtd/neunode/issues/39) | Chain log indexing, durable cursor/replay, duplicate handling and reorg rollback remain. Feed catchup is a different protocol path. |
| [#40 chain-derived balances](https://github.com/akashrtd/neunode/issues/40) | Convert local balance/staking stores into chain-derived mirrors after authoritative transaction/event integration. |
| [#41 economic correctness](https://github.com/akashrtd/neunode/issues/41) | Full-width local arithmetic and one-time grants fixed; cross-language policy, membrane, proof-backed minting and on-chain redistribution remain. |
| [#42 operations/migration](https://github.com/akashrtd/neunode/issues/42) | Health reports local authority and missing training executor; sustained liveness/recovery, chain-default migration and removal of obsolete Ethereum paths remain. |

Neither Reth nor Malaketh was available on PATH. Existing user work on pinned Malaketh builds
was preserved. The sovereign path was not silently replaced with a different chain architecture.

## Order of the remaining implementation

Beads retains the concrete acceptance work: complete provider streaming and cancellation/recovery
(`neunode-zva.9`); wire actual training workers or restrict the beta surface (`.10`); qualify
operations, budgets/sandbox boundaries and Rust/Solidity economic equivalence (`.14`, `.17`);
run native artifact/release jobs and reference agents (`.15`); continue the per-file semantic
review and independent adversarial audit (`.16`). The L1 phases retain their existing dependency
order: real multi-validator finality, reputation/epoch sets, canonical transactions, replay/reorg
mirrors, chain-derived economics, then migration/operations (`.13` and `neunode-77j`).
