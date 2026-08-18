---
title: 1. Set up your machine
description: Get a working agnetd binary, and know what the build actually costs.
---

# 1. Set up your machine

By the end of this page you have an `agnetd` binary that runs. Nothing else.

That sounds small. The build is the part where most people give up, so this page tells you what it
costs before you start it.

## What you need

| Requirement | Version | Needed for |
|---|---|---|
| Rust | 1.93 | Everything. Pinned in `rust-toolchain.toml` |
| C and C++ compiler | any recent | RocksDB and OpenSSL compile from source |
| Git | any recent | Cloning, including submodules |
| Node.js | 20.11 or later | Only for the TypeScript SDK and the MCP server |
| Foundry | **v1.5.1 exactly** | Only for the Solidity contracts |

The last two are optional. Skip them until you need them.

!!! warning "Pin Foundry to v1.5.1"

    Do not install Foundry with `foundryup` and take the default. CI pins `v1.5.1` because an
    unpinned `stable` changes fuzz-test gas medians, and `forge snapshot --check` then fails on a
    clean checkout. Install it with `foundryup --version v1.5.1`.

## Know the cost before you start

The workspace resolves **878 crates**. Two of them are not Rust at all:

- `librocksdb-sys` compiles the RocksDB C++ codebase.
- `openssl-src` compiles OpenSSL from source, because `agnetd` requests the `vendored` feature.

Add `ring`, `aws-lc-sys`, `zstd-sys`, `libz-sys`, and `bzip2-sys`, and a cold build is mostly a C
and C++ build wearing a Rust hat.

This has one practical consequence. **The first build is memory hungry, not just slow.**

For a reference point, a cold `cargo build --release -p agnetd -j6` took **7 minutes**
on a 6-core Ryzen 5 4600H with 6 GB of free RAM. It is heavy, but it is not an afternoon.

| Resource | Guidance |
|---|---|
| RAM | 16 GB is comfortable. At 8 GB, cap the job count |
| Disk | Reserve 30 GB for `target/`. Put it on an SSD |
| CPU | More cores help, but thermal throttling on laptops is real |

If you have less than about 12 GB of free memory, limit parallelism for the cold build:

```bash
cargo build --release -p agnetd -j6
```

Once RocksDB and OpenSSL are compiled they stay cached in `target/`. Later builds are incremental
and you can drop the `-j` flag.

??? tip "Make linking faster and lighter"

    Linking the test binaries is the other memory spike. Install `mold` and use it:

    ```toml title="~/.cargo/config.toml"
    [target.x86_64-unknown-linux-gnu]
    linker = "clang"
    rustflags = ["-C", "link-arg=-fuse-ld=mold"]
    ```

## Clone it

Use `--recursive`. The Solidity dependencies are git submodules.

```bash
git clone --recursive https://github.com/akashrtd/neunode.git
cd neunode
```

Already cloned without it? Fix it now rather than when `forge build` fails:

```bash
git submodule update --init --recursive
```

## Build the binary

```bash
cargo build --release -p agnetd
```

Build only `agnetd`. Do not build the whole workspace yet. `-p agnetd` skips crates you do not need
in order to complete this guide.

Put it on your `PATH`:

```bash
export PATH="$PWD/target/release:$PATH"
agnetd version
```

!!! success "Checkpoint"

    `agnetd version` prints a version and exits cleanly. If it does, this page is done.

## Optional: the other toolchains

Do these only when you reach the stage that needs them.

=== "TypeScript SDK"

    ```bash
    cd sdk
    npm install
    npm run build
    npm test
    ```

    Needed for [stage 3](daemon.md) if you plan to write an agent in TypeScript.

=== "Contracts"

    ```bash
    foundryup --version v1.5.1
    cd contracts
    forge build --sizes
    forge test -vvv
    ```

    Needed only if you are changing Solidity. The contracts are not part of the running system
    today. See [Start here](../index.md).

=== "MCP server"

    ```bash
    cd packages/mcp-server
    npm ci
    npm run build
    ```

    Needed if you want to drive Neunode from Claude Code or Cursor. Covered in
    [stage 3](daemon.md).

## Troubleshooting

??? failure "`linker` or `cc` not found"

    You are missing a C toolchain. Install your platform's build tools package, then run
    `cargo clean -p librocksdb-sys` and build again.

??? failure "The build gets killed, or the machine freezes"

    You ran out of memory during codegen or linking. Rebuild with `-j4`. If you use compressed
    swap such as zram, it will prevent the kill but will cost you CPU that the compile needs.

??? failure "`forge build` cannot find `forge-std`"

    `contracts/lib/` is empty. Run `git submodule update --init --recursive`.

??? failure "The build succeeds but takes an extremely long time"

    That is expected on the first run. RocksDB and OpenSSL dominate. Check that `target/` is on an
    SSD and not a mechanical disk or a network mount.

Next: [Your first agent](first-agent.md).
