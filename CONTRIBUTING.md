# Contributing to Neunode

Thank you for your interest in contributing to Neunode. This guide covers setup, development workflow, and contribution guidelines.

## Prerequisites

- **Rust 1.93+** (edition 2021) -- install via [rustup](https://rustup.rs/)
- **Node.js 20+** and npm (for SDK development)
- **Foundry** (for Solidity contracts) -- install via [foundryup](https://book.getfoundry.sh/getting-started/installation)
- **A C compiler** (gcc or clang, required for RocksDB compilation)
- **Git**

## Development Setup

### Rust Workspace

```bash
git clone https://github.com/akashrtd/neunode.git
cd neunode
cargo build
cargo test
```

The binary builds to `target/debug/agnetd` (or `target/release/agnetd` with `cargo build --release`).

### TypeScript SDK

```bash
cd sdk
npm install
npm run build
npm test
```

### Smart Contracts

```bash
cd contracts
forge build
forge test
```

## Project Structure

```
neunode/
├── crates/                    # Rust workspace members
│   ├── neunode-core/          #   Shared types, errors, config, kind taxonomy
│   ├── neunode-crypto/        #   Ed25519, secp256k1, hashing, EIP-712
│   ├── neunode-identity/      #   DID, keyring, agent card
│   ├── neunode-storage/       #   RocksDB, 20 column families, moka cache
│   ├── neunode-p2p/           #   libp2p networking (Gossipsub, KadDHT)
│   ├── neunode-feed/          #   Sigchain, events, schemas, filters
│   ├── neunode-token/         #   Balance, staking, decay
│   ├── neunode-reputation/    #   5-factor scoring, attestations
│   ├── neunode-bounty/        #   State machine, escrow, verification
│   ├── neunode-inference/     #   OpenAI-compatible marketplace
│   └── agnetd/                #   CLI binary (clap 4)
├── contracts/                 # Solidity smart contracts (Foundry, EIP-2535 Diamond)
│   ├── src/
│   │   ├── tokens/            #   4 resource ERC-20 tokens + base
│   │   ├── bounty/            #   Review, escrow interfaces
│   │   ├── diamond/           #   EIP-2535 Diamond proxy facets
│   │   ├── royalty/           #   Model registry, ERC-2981 splitter
│   │   ├── governance/        #   DAO governance
│   │   └── interfaces/        #   Shared interfaces
│   └── foundry.toml           #   Solidity 0.8.24, optimizer 200 runs, via_ir
├── sdk/                       # TypeScript SDK (@neunode/sdk)
│   ├── src/
│   │   ├── client/            #   Client factory and configuration
│   │   ├── transport/         #   CLI subprocess + Viem direct transports
│   │   ├── resources/         #   10 resource modules (identity, feed, bounty, etc.)
│   │   ├── contracts/         #   14 Solidity ABIs, addresses, getContract helpers
│   │   ├── types/             #   Branded types, enums, JSON envelope
│   │   └── utils/             #   Shared utilities
│   └── tests/
│       ├── e2e/               #   End-to-end tests (requires Anvil)
│       └── integration/       #   Integration tests (requires agnetd binary)
├── tests/                     # Rust integration tests (bounty, feed, identity, inference, p2p flows)
├── research/                  # 22 research documents (architecture, protocols, verification)
└── .github/workflows/         # CI: Rust lint+test, Foundry build+test+gas, Release cross-compile
```

## Code Style

### Rust

- Formatted with `cargo fmt`. Configuration in `rustfmt.toml`.
- Linted with `cargo clippy`. Configuration in `clippy.toml` (cognitive-complexity threshold: 30).
- Toolchain pinned in `rust-toolchain.toml` (Rust 1.93, components: rustfmt, clippy).
- **Error handling**: Use `Result<T, E>` with crate-specific error enums. Convert to `NeunodeError` at crate boundaries. Use `thiserror` for error derives.
- **Naming**: `snake_case` for functions and variables, `CamelCase` for types and traits, `SCREAMING_SNAKE_CASE` for constants.
- **Async**: All async operations use `tokio`. Do not mix runtimes.

### TypeScript

- ESM modules (`"type": "module"` in `package.json`).
- Strict TypeScript (`strict: true` in `tsconfig.json`).
- No `as any`, `@ts-ignore`, or `@ts-expect-error` without explicit justification.
- Formatted and linted with Biome (`npm run lint`).
- **Naming**: `camelCase` for functions and variables, `PascalCase` for types, interfaces, and classes.
- **Branded types**: Use branded types for `Did`, `CID`, `PeerId`, `BountyId` to prevent accidental mixing.

### Solidity

- Solidity 0.8.24.
- Formatted with `forge fmt` (line length 100, tab width 4, configured in `foundry.toml`).
- Follow OpenZeppelin patterns for upgradeable contracts.
- NatSpec comments (`///` or `/** */`) for all `public` and `external` functions.
- Use custom errors instead of `require` strings for gas efficiency.
- All upgradeable contracts use the EIP-2535 Diamond proxy pattern.

## Testing

### Rust Tests

```bash
cargo test                        # Run all tests across workspace
cargo test -p agnetd              # CLI-specific tests
cargo test -p neunode-bounty      # Bounty crate tests
cargo test -p neunode-core        # Core type tests
cargo test --release              # Tests in release mode
```

### SDK Tests

```bash
cd sdk
npm test                          # Unit tests (mocked transports)
npm run test:watch                # Watch mode
npm run test:e2e                  # E2E tests (requires running Anvil instance)
npm run test:integration          # Integration tests (requires agnetd binary in PATH)
npm run typecheck                 # TypeScript type checking (tsc --noEmit)
```

### Contract Tests

```bash
cd contracts
forge test -vvv                   # All Solidity tests with verbose output
forge test --match-test "testBounty"   # Run specific test
forge snapshot --check            # Gas snapshot regression check
forge coverage                    # Coverage report
```

## Pull Request Process

1. Fork the repository.
2. Create a feature branch: `git checkout -b feature/my-feature`.
3. Make changes with appropriate tests.
4. Run all CI checks locally:

   **Rust:**
   ```bash
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   ```

   **SDK:**
   ```bash
   cd sdk
   npm run lint
   npm test
   npm run typecheck
   ```

   **Contracts:**
   ```bash
   cd contracts
   forge fmt --check
   forge build --sizes
   forge test -vvv
   forge snapshot --check
   ```

5. Push and create a PR against `main`.
6. CI runs automatically on all pull requests.

## CI Pipeline

Three GitHub Actions workflows:

### ci.yml -- Rust

Triggers on push and PR to `main`.

- **lint**: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`
- **test**: `cargo test --workspace`
- **check**: `cargo check --workspace --all-features`

### contracts-ci.yml -- Solidity

Triggers on push and PR to `main` when `contracts/**` paths change.

- `forge fmt --check`
- `forge build --sizes`
- `forge test -vvv`
- `forge snapshot --check`

### release.yml -- Cross-Compile

Triggers on tag push (`v*`).

Cross-compiles `agnetd` for three targets:
- `x86_64-unknown-linux-gnu` (Linux AMD64)
- `aarch64-unknown-linux-gnu` (Linux ARM64)
- `aarch64-apple-darwin` (macOS Apple Silicon)

Builds release binaries, uploads artifacts, and creates a GitHub Release with auto-generated notes.

## Reporting Issues

Use GitHub Issues: https://github.com/akashrathod/neunode/issues

When reporting a bug, include:

- Rust version (`rustc --version`)
- Operating system
- Command that failed (full invocation)
- Full error output with backtrace (`RUST_BACKTRACE=1`)

For feature requests, describe the use case and expected behavior.

## License

By contributing to Neunode, you agree that your contributions will be licensed under the [MIT License](./LICENSE).
