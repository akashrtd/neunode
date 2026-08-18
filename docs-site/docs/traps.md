---
title: Traps
description: The failures that look like your mistake but are known behavior.
---

# Traps

Every one of these has cost somebody an afternoon. Read the list now and save yourself the time.

## The port mismatch

**Symptom.** The SDK, the MCP server, or an example cannot reach the daemon, and nothing in the logs
explains why.

**Cause.** `agnetd serve` defaults to port **8080**. The MCP server and the bundled examples default
to **41000**. They disagree out of the box.

**Fix.** Pick one and force it everywhere.

```bash
agnetd serve --port 41000
# or point the client at 8080 via AGNETD_URL / NEUNODE_URL
```

---

## The database lock

**Symptom.**

```text
IO error: While lock file: .../LOCK: Resource temporarily unavailable
```

**Cause.** RocksDB permits one process per database directory. You are running a second `agnetd`
against a database that the first one already holds.

**Fix.** Give each process its own database, or stop using a second process.

```bash
agnetd --db-path /tmp/neunode-b <command>
```

Better: run one daemon and talk to it over HTTP. That is the intended pattern. See
[Why the daemon owns the database](explanation/http-native.md).

---

## Seeded tokens that you cannot spend

**Symptom.** You ran `agnetd token seed`, `token balance` shows tokens, and `bounty create` still
fails with an insufficient-balance error.

**Cause.** Seeded tokens arrive **staked**, not liquid. Bounty escrow draws from the **liquid**
balance, which is still zero.

**Fix.** Unstake, wait for unbonding, then claim. Shorten the period first on a local node.

```bash
agnetd config set tokens.unbonding_period_secs 5
agnetd token unstake --amount 100
agnetd token claim-unbonded
agnetd token balance
```

Full explanation in [The economy](guide/economy.md).

---

## `HTTP transport required`

**Symptom.** An SDK call throws `HTTP transport required for ... operations`, even though you
configured a transport.

**Cause.** Four SDK resources are HTTP only: `identity`, `lifecycle`, `lineage`, and `verification`.
They have no CLI fallback. You configured only the `cli` transport.

**Fix.** Add an `http` transport to the client.

```typescript
const client = createNeunodeClient({
  http: { baseUrl: "http://127.0.0.1:8080" },
});
```

---

## `forge build` cannot find `forge-std`

**Symptom.** Solidity imports fail on a fresh clone.

**Cause.** `contracts/lib/` holds git submodules, and you cloned without `--recursive`.

**Fix.**

```bash
git submodule update --init --recursive
```

---

## Gas snapshot fails on an untouched checkout

**Symptom.** `forge snapshot --check` fails even though you changed no Solidity.

**Cause.** Your Foundry version is not the pinned one. Fuzz-test gas medians shift between releases.

**Fix.**

```bash
foundryup --version v1.5.1
```

---

## The build gets killed

**Symptom.** The cold build dies, or the machine locks up.

**Cause.** Memory, not CPU. RocksDB and OpenSSL both compile from C and C++ source. Parallel codegen
plus linking can exceed available RAM.

**Fix.** Cap parallelism.

```bash
cargo build --release -p agnetd -j4
```

---

## A CLI feature the SDK cannot see

**Symptom.** A command works in the CLI, but there is no SDK or MCP equivalent.

**Cause.** `agnetd` implements every capability twice, in `cmd_*.rs` for the CLI and `api_*.rs` for
HTTP. Nothing enforces parity, and the compiler will not catch the gap. The SDK and MCP server only
consume the HTTP surface.

**Fix.** Add the missing HTTP handler and register it in `api_routes.rs`. When contributing, add
both surfaces in the same change.

---

## Stale entries in `KNOWN_ISSUES.md`

**Symptom.** The repository's `KNOWN_ISSUES.md` describes bugs that you cannot reproduce.

**Cause.** That file is dated 2026-05-14 and has not kept up. At least two entries are fixed:

- **Bounty escrow is now validated.** Creation checks the liquid balance, then commits the balance
  change, the bounty record, and the audit entry in one atomic batch under a ledger write lock. It
  no longer creates unfunded bounties.
- **`active_identity` is now settable.** `agnetd config set active_identity <DID>` works. There are
  17 settable configuration keys.

**Fix.** Trust the code over that file. Verified against
`crates/neunode-storage/src/bounty_store.rs` and `crates/agnetd/src/config.rs` at commit `ccf0776`.
