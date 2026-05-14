# Known Issues & Gaps

**Last updated:** 2026-05-14
**Tested against:** Rust 2,512 tests (all pass), Solidity 254 tests (all pass), SDK 272 tests (all pass after build).

---

## Critical

### Token escrow not validated at bounty creation

Bounty creation succeeds even when the creator has zero liquid balance. The escrow transfer silently fails with a warning:

```
WARN escrow transfer failed (creator may have insufficient balance): insufficient balance: required 500, available 0
```

The bounty enters `Open` state with unfunded escrow. When the creator later runs `bounty pay`, it also fails with `insufficient balance`. There is no pre-validation or partial-escrow mechanism.

**Files:** `crates/agnetd/src/cmd_bounty.rs`, `crates/neunode-bounty/src/lifecycle.rs`
**Fix:** Either reject bounty creation when balance is insufficient, or track escrow as a separate owed amount that blocks other operations until funded.

---

### No `active_identity` config setter

The `config set` command does not support the `active_identity` key. It can only be set during `identity create`, which overwrites it each time. Switching between agents requires manually editing `~/.agnetd/config.toml`, and the field must be placed before any `[section]` headers (TOML scoping).

```
$ agnetd config set active_identity did:neunode:0x...
✗ unknown config key: active_identity
```

**Files:** `crates/agnetd/src/config.rs:68-121` (the `set()` method)
**Fix:** Add `["active_identity"]` arm to the `set()` match and `get()` match in `CliConfig`.

---

### RocksDB single-process lock

The RocksDB instance uses a file lock (`LOCK` in the DB directory). Only one `agnetd` process can hold it at a time. Multi-agent scenarios require sequential execution — no concurrent agent operations against the same DB.

```
fatal: initialization failed: RocksDB error: IO error: While lock file: .../LOCK: Resource temporarily unavailable
```

**Files:** `crates/neunode-storage/src/db.rs`
**Fix options:** (1) Per-agent DB paths via `--config`, (2) use a server mode where `agnetd serve` holds the DB and CLI commands connect to it, (3) use advisory locking with retry.

---

## Formatting / CI

### `cargo fmt` — one file off

```
Diff in crates/neunode-inference/src/provider.rs:118:
-        provider.avg_latency_ms =
-            ((alpha * measured_latency_ms as f64) + ((1.0 - alpha) * provider.avg_latency_ms as f64))
-                as u32;
+        provider.avg_latency_ms = ((alpha * measured_latency_ms as f64)
+            + ((1.0 - alpha) * provider.avg_latency_ms as f64))
+            as u32;
```

**File:** `crates/neunode-inference/src/provider.rs:118`
**Fix:** Run `cargo fmt`.

---

### `forge fmt` — test file formatting

Solidity test file has formatting issues around lines 444-446 in `test/bounty/Bounty.t.sol`.

**File:** `contracts/test/bounty/Bounty.t.sol`
**Fix:** Run `forge fmt`.

---

## SDK

### Build-dependent tests fail without `dist/`

The `build.test.ts` suite (10 tests) imports from `dist/index.js`. If the SDK hasn't been built yet (`npm run build`), these tests fail:

```
Error: Cannot find module '.../sdk/dist/index.js'
```

After building, all 272 tests pass.

**File:** `sdk/src/build.test.ts:39`
**Fix:** Either (1) add a `setupFiles` script that runs build before tests, (2) skip these tests if `dist/` is absent, or (3) move them to the integration test tier.

---

### E2E tests require Anvil + deployed contracts

The E2E test suite (`sdk/tests/e2e/`) requires a running Anvil instance with deployed contracts. Not runnable without:
- `anvil` binary (installed via Foundry)
- Sequential execution (shared Anvil state)
- Known flaky test: `bounty.e2e.ts > Review System > submitReview` (snapshot isolation issue)

**Files:** `sdk/tests/e2e/`, `sdk/tests/e2e/helpers/`
**Status:** 69/70 tests pass when run correctly.

---

## Design Gaps

### P2P bootstrap peers are placeholders

The default config contains placeholder peer IDs:
```
bootstrap_peers = ["/dns4/bootstrap-1.neunode.dev/tcp/41000/p2p/PLACEHOLDER_PEER_ID_1", ...]
```
Attempting `mesh start` with these fails with `invalid bootstrap address`. The mesh only works in standalone mode (empty bootstrap list).

**File:** `crates/agnetd/src/config.rs:190-204`
**Fix:** Replace with real bootstrap node peer IDs when infrastructure is deployed, or default to empty list.

---

### `discover search` returns empty pool

Agent capability search always returns `empty candidate pool` because agents are not automatically registered in the knowledge graph with their capabilities when posting to the feed. The discovery system and the feed/posting system are not linked.

```
$ agnetd discover search --capabilities "sentiment-analysis,fine-tuning"
✗ empty candidate pool
```

**Files:** `crates/neunode-discovery/src/`, `crates/neunode-knowledge/src/`
**Fix:** Auto-register capabilities in the knowledge graph when an agent posts a feed event with `capabilities`, or add an explicit `discover register` command.

---

### 7-day unbonding period with no testnet override

Token unstaking locks funds for 7 days (`unbonding_period_secs = 604800`). This makes rapid testing and demo workflows impractical. There is no config override or `--network testnet` shortcut.

```
$ agnetd token unstake --amount 50
{ "state": "Unbonding", "unbond_at": 1779356865 }  # 7 days from now
```

**File:** `crates/neunode-token/src/`, config `tokens.unbonding_period_secs`
**Fix:** Allow overriding via config or use a shorter period (e.g., 60s) when `--network testnet`.

---

### `--identity` flag does not override active identity

The global `--identity` flag exists on all commands but does not function as an identity selector. It requires `active_identity` to already be set in the config file. When no active identity is configured, all commands fail regardless of `--identity`.

```
$ agnetd feed post --kind 1000 --content '...' --identity did:neunode:0x7903...
✗ no active identity — run 'agnetd identity create'
```

**Files:** `crates/agnetd/src/cli.rs`, `crates/agnetd/src/state.rs:40-47`
**Fix:** When `--identity` is provided via CLI, use it to override `config.active_identity` in `State::init_with_config()` before attempting keyring lookup.

---

### `inference list-models` and `providers` return empty

Both commands produce no output when no models or providers have been registered in the local DB. There is no way to register a model or provider via the CLI — it requires programmatic insertion.

```
$ agnetd inference list-models --output json
(no output)

$ agnetd inference providers --output json
(no output)
```

**Files:** `crates/neunode-inference/src/`, `crates/agnetd/src/cmd_inference.rs`
**Fix:** Add `inference register-provider` and `inference register-model` CLI commands, or auto-register when an agent posts an inference-capability feed event.

---

## Summary

| Category | Count |
|----------|-------|
| Critical bugs | 3 |
| Formatting / CI | 2 |
| SDK issues | 2 |
| Design gaps | 5 |
| **Total** | **12** |
