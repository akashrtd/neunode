# Known issues and release gates

Updated: 2026-10-04. Current evidence is in
[the beta implementation report](docs/beta/implementation-validation.md) and
[its validation record](docs/beta/remediation-results.json).
The earlier 2026-05 snapshot had obsolete formatting, onboarding and escrow findings;
it is preserved in Git history rather than presented as current status.

## Remaining beta blockers

| Area | Current limitation | Tracking |
| --- | --- | --- |
| Canonical ledger | HTTP/CLI economics use local RocksDB. Chain supervision and direct SDK contract calls do not establish one shared authoritative ledger. Finalized receipts, event replay, reorg rollback and chain-derived mirrors remain. | Beads `neunode-zva.13`, `neunode-77j`; GitHub #37, #39, #40 |
| Sovereign consensus | Library vote-domain and validator-set checks pass. A real multi-validator Reth/Malachite network, reputation weights, epoch changes, outage/catchup and Byzantine tests have not run. | GitHub #30, #32, #35 |
| Economic policy | Local arithmetic is checked/integer and bootstrap grants are one-time per identity. Proof-backed issuance, sybil resistance, membrane enforcement, on-chain decay/redistribution and Rust/Solidity equivalence remain. | Beads `neunode-zva.17`; GitHub #41 |
| Training | Job IDs are unique and durable metadata can be queued. The daemon has no running model executor/scheduler; health reports `training_executor: unavailable`. Queuing does not train a model or produce a checkpoint. | Beads `neunode-zva.10` |
| Incremental inference | Real provider execution and local reservation/settlement work. WebSocket completion is authenticated, but upstream calls currently request a whole response rather than incremental SSE tokens. Further disconnect/crash/fault testing is needed. | Beads `neunode-zva.9` |
| Operations | Manual stops are enforced. Automatic anomaly monitoring, complete capability/budget sandboxing, decay/lifecycle scheduling, bounded global dashboard snapshots, pagination/load limits, backup/restore and sustained fault tests are not qualified. | Beads `neunode-zva.14`, `.17`; GitHub #42 |
| Distribution | Fresh SDK/MCP tarballs and installed `npx` tools pass locally. Three-platform native artifact gates and publication are configured but have not run in hosted CI. No release has been published during this work. External coding/research/provider reference workloads remain to qualify. | Beads `neunode-zva.15`; GitHub #3, #4 |
| Security review | Existing suites and new adversarial cases do not establish a complete per-line audit. Hardware TEE, real ML workloads, wider network adversaries and an independent review remain. | Beads `neunode-zva.16` |

## Operational constraints

RocksDB deliberately holds a single-process database lock. Use one daemon per data directory
and drive it through HTTP/SDK/MCP; do not open another CLI process against the same database.
Use separate homes/configurations/data directories for independent nodes.

The unattended keystore uses a private independent master-secret file, or `NEUNODE_KEYSTORE_KEY`.
Back up the key material and its secret together. Loss of the modern master secret cannot be
repaired by regenerating it. Compromise of the same operating-system account can expose both.

Daemon mutations require the operator access token. This grants the active daemon authority;
per-client least-privilege credentials are still an extension. Identity selection loads owned
keys; changing the running mesh identity requires restarting the daemon.

Default unstaking remains seven days. The configurable `tokens.unbonding_period_secs` supports
isolated testing; acceptance fixtures set it to zero. A locked unbond is not a liquid balance.

Build the SDK before running its distribution tests. Anvil tests require Foundry and run
sequentially against deployed test contracts. Current full runs pass; historical snapshot
isolation failures are not treated as evidence of present chain readiness.

Contract factory/paymaster helpers now import from `@neunode/sdk/contracts`. Installing `viem`
is necessary only when using those helpers; HTTP ESM/CJS imports work without it.

Public peer bootstrap infrastructure is not qualified. Local peer tests use explicit loopback
addresses and independent identities; they do not establish reachable public bootstrap nodes.
