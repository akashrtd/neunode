---
title: Why the daemon owns the database
description: How a file lock determined the architecture.
---

# Why the daemon owns the database

Sometimes a whole architecture follows from one small technical fact. This is one of those times,
and the fact is unglamorous.

**RocksDB takes a single-process file lock on its data directory.**

Neunode actually opens *three* RocksDB instances under the data directory (`ledger/`,
`network/`, `graph/`). That does not soften the constraint. It means three locks instead of
one, all held by the same process.

That is it. Everything below follows from that sentence.

## What the constraint forces

If exactly one process can open the database, then exactly one process can be authoritative about
state. Every other participant must ask that process rather than read the files directly.

So the question is not *whether* to have a daemon. The question is only what the daemon's interface
looks like.

## What it rules out

Some designs are simply unavailable, and it is useful to see why.

| Design | Why it fails |
|---|---|
| Every CLI call opens the database | Two concurrent calls collide on the lock |
| SDK reads the database directly | Same collision, now from another language |
| MCP server embeds the storage layer | Same again, and it would duplicate protocol logic |
| Multiple agents share one database directory | Only the first one starts |

The last row is the one that surprises people. Multi-agent scenarios on a single machine need
**separate database paths**, which is why `--db-path` is a global flag rather than a niche option.

## What it produces

The surviving design is the one the project actually has.

```text
  SDK (TypeScript)  ─┐
  MCP server        ─┤
  examples          ─┼──> HTTP ──> [ agnetd serve ] ──> RocksDB + libp2p
  curl              ─┤              sole lock holder
  another CLI call  ─┘
```

One process holds the lock. Everything else speaks HTTP to it. The daemon is not a convenience
layer over a library; it is the concurrency model made explicit.

## The migration you can see in the git history

The project did not start here. It started CLI-first, and the HTTP surface came later. You can read
that transition directly in the commit log:

```text
feat(inference): register providers over HTTP
feat(identity): make SDK operations HTTP native
feat(verification): expose production TEE checks over HTTP
fix(sdk): align training HTTP contracts
```

The migration is visible in the SDK too, and it is **not finished**. Sixteen resources exist. Twelve
still try HTTP and fall back to spawning the CLI. Four have completed the move and are HTTP only:

- `identity`
- `lifecycle`
- `lineage`
- `verification`

Those four throw immediately without an HTTP transport. That asymmetry is not an inconsistency to
work around. It is a migration in progress, and the direction of travel is toward HTTP for
everything.

## The cost

Honesty requires naming what this design gives up.

- **The CLI transport is now second class.** It still works, and it still takes the lock, so it
  cannot run beside a daemon.
- **Two surfaces must be maintained.** Every capability is written twice, once in `cmd_*.rs` and
  once in `api_*.rs`. Nothing enforces parity and the compiler cannot catch a gap. A CLI-only
  feature is invisible to the SDK and the MCP server.
- **A running daemon becomes a prerequisite.** For a system that advertises a single binary with no
  runtime dependencies, that is a real ergonomic cost.

The trade is worth it, but it is a trade.

## What would change this

A storage engine permitting multi-process access, or a shared-cache mode, would remove the original
constraint. That is a large change with its own risks, and nothing in the repository suggests it is
planned. Assume the daemon model is permanent.
