---
title: About these docs
description: The rules this documentation follows, and why.
---

# About these docs

Documentation fails in predictable ways. These pages try to avoid the predictable ways. This page
records how, so that anyone adding to the site keeps it coherent.

## The framework

This site follows [Diátaxis](https://diataxis.fr/), which observes that readers arrive with four
different needs, and that a page serving two of them at once serves neither well.

| Type | Serves | Reader is | Answers |
|---|---|---|---|
| **Tutorial** | Learning | Studying | "Take me through this" |
| **How-to** | A task | Working | "How do I solve X?" |
| **Reference** | Facts | Working | "What are the parameters?" |
| **Explanation** | Understanding | Studying | "Why is it like this?" |

Mapped onto this site:

- [The guide](guide/index.md) is **tutorial**. It has a fixed order and a promised outcome.
- [Traps](traps.md) is **how-to**. Each entry is a symptom, a cause, and a fix.
- [Reference](reference/index.md) is **reference**. Facts only, no teaching.
- [Explanation](explanation/index.md) is **explanation**. Reasoning only, no instructions.

The failure this prevents is the common one: reference material that stops to teach, or a tutorial
that breaks its own flow to explain the design. Both leave the reader stranded.

## The rules

Applied to every page here.

### Structure

1. **One job per page.** If a page teaches and specifies, split it.
2. **State the outcome first.** The reader should know what they will have at the end before they
   start.
3. **Give checkpoints.** A tutorial without a way to confirm progress is a tutorial the reader
   abandons silently.
4. **Put the warning before the hazard**, never after. A caution below the command that destroys
   data is useless.

### Writing

5. **Use active voice and name the actor.** "The daemon holds the lock", not "the lock is held".
6. **One idea per sentence.** Keep instructions near 20 words and descriptions near 25.
7. **Start instructions with a verb.** "Run the build", not "the build should be run".
8. **One term per concept.** Never vary vocabulary for style. `liquid balance` stays
   `liquid balance` on every page.
9. **Distinguish requirement from advice.** `must` for requirements, `should` for recommendations,
   `can` for permissions.
10. **Avoid idioms and decorative phrasing.** They are the first thing to fail for a reader whose
    first language is not English.

These come from [ASD-STE100 Simplified Technical English](https://www.asd-ste100.org/), a controlled
language written for aerospace maintenance manuals, where an ambiguous instruction has consequences.

### Honesty

11. **Document what the code does, not what the README claims.** Where the two disagree, the code
    wins and the disagreement gets written down. Two entries in the repository's `KNOWN_ISSUES.md`
    are stale, and [Traps](traps.md) says so.
12. **Name the gaps.** The ZK verification tier is stubbed. There is no live chain. Tokens have no
    value. A document that hides these wastes the reader's time later.
13. **Do not present untested commands as verified output.** These pages show commands and describe
    results. They do not fabricate console transcripts.

## Verification status

Every factual claim here was checked against the source at commit `ccf0776`, specifically:

| Claim | Source |
|---|---|
| CLI flags, subcommands, defaults | `crates/agnetd/src/cli.rs` |
| HTTP routes (80, extracted programmatically) | `crates/agnetd/src/api_routes.rs` |
| Seed amounts, unbonding period, decay split | `crates/neunode-core/src/constants.rs` |
| Activity levels and decay rates | `crates/neunode-core/src/types.rs` |
| Reputation factor weights | `crates/neunode-core/src/constants.rs` |
| Bounty states | `crates/neunode-core/src/types.rs` |
| Escrow validation and atomicity | `crates/neunode-storage/src/bounty_store.rs` |
| Config keys (17) | `crates/agnetd/src/config.rs` |
| Column families (22), feed key layout | `crates/neunode-storage/src/cf.rs` |
| Event kinds (31) and category ranges | `crates/neunode-core/src/kind.rs` |
| ZK tier returns `Unsupported` | `crates/neunode-verification/src/zk.rs` |
| SDK transport split per resource | `sdk/src/resources/*.ts` |
| Absence of ML frameworks | every `crates/*/Cargo.toml` |

!!! success "The guide's commands were executed"

    On 2026-08-18 the toolchain was installed and the guide was run end to end against a real
    `agnetd` built from commit `ccf0776`. Verified by execution:

    - `init`, `identity create`, `identity list`, `feed post`, `feed list`
    - `token seed` grants 250 tokens, all staked, liquid balance zero
    - `bounty create` is **rejected** with `insufficient balance: required 50, available 0`
    - `config set tokens.unbonding_period_secs` then `unstake` then `claim-unbonded` yields liquid
      tokens, after which `bounty create` succeeds
    - `--db-path` plus `--config` gives true per-node isolation
    - Contracts: 397 Forge tests pass. SDK: 358 unit tests pass, plus build, typecheck, lint, and
      the ABI drift gate. MCP server: 53 tests pass

    This pass corrected five errors in earlier drafts, including a missing `--config` in the
    multi-node instructions and an incorrect description of the storage layout.

!!! warning "Still not verified"

    The multi-node mesh walkthrough in stage 5 was not run with two live gossiping nodes. The
    isolation flags were verified; peer connection and event propagation were not. The inference,
    training, knowledge, and lineage command groups are documented from source, not from execution.

## Contributing to these docs

The source is `docs-site/docs/` in the repository. To work on it:

```bash
python3 -m venv .venv
.venv/bin/pip install mkdocs-material==9.7.7
.venv/bin/mkdocs serve -f docs-site/mkdocs.yml
```

Before opening a pull request, ask three questions about your change:

1. Which of the four types is this page? If the answer is "two of them", split it.
2. Does every claim trace to code, or to a decision record?
3. Would this instruction have exactly one interpretation for a tired reader at 2am?

## Credits

The structure and tone are modelled on [TheMoeWay](https://learnjapanese.moe/), which is unusually
good at a hard problem: guiding a beginner through a large, intimidating subject without either
condescending or hiding the difficult parts. It is built with the same tooling as this site,
[Material for MkDocs](https://squidfunk.github.io/mkdocs-material/).
