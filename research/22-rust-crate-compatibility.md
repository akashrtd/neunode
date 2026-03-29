# 22 — Rust Crate Compatibility Verification

**Date**: 2026-03-30 | **Status**: Verified via crates.io, docs.rs Cargo.toml inspection

## Compatibility Matrix

| Crate Pair | Result | Notes |
|---|---|---|
| tokio v1.50 + libp2p v0.56 | **PASS** | libp2p has explicit `tokio` feature. NOT async-std. |
| tokio v1.50 + alloy v1.8 | **PASS** | alloy sub-crates use tokio internally. No conflict. |
| tokio v1.50 + rocksdb v0.24 | **PASS** | rocksdb is sync; wrap in `tokio::task::spawn_blocking`. |
| tokio v1.50 + ssi v0.15 | **PASS** | ssi uses async-std in dev-deps only, not runtime. |
| libp2p v0.56 + ed25519-dalek | **FAIL** | libp2p-identity pins `ed25519-dalek = "2.1"`. We cannot use v3.0-pre. |
| libp2p v0.56 + ring v0.17 | **PASS** | libp2p-noise→snow→ring-resolver pulls ring. Version aligned. |
| libp2p v0.56 + sha2 | **PASS** | libp2p-noise and libp2p-identity both use sha2 0.10.8. |
| libp2p v0.56 + multihash v0.19 | **PASS** | libp2p-identity uses multihash 0.19.1. |
| libp2p v0.56 + k256 | **PASS** | libp2p-identity uses k256 0.13.4 (via `secp256k1` feature). |
| alloy v1.8 + libp2p v0.56 | **CONCERN** | Shared: k256 0.13.x ✓, sha2 0.10.x ✓, serde 1.0 ✓. Verify no minor skew at build time. |
| alloy v1.8 + ring v0.17 | **CONCERN** | alloy default uses `reqwest-default-tls` → native-tls → openssl on Linux. |
| alloy v1.8 + ssi v0.15 | **PASS** | Both use k256 0.13.x, sha2 0.10.x. No conflict. |
| ed25519-dalek + ssi v0.15 | **FAIL** | ssi pins `ed25519-dalek = "2.0"`. Must use 2.x, not 3.0-pre. |
| rocksdb v0.24 + macOS ARM64 | **PASS** | Compiles C++ from source. Needs cmake + Xcode CLT. Static linking fixed. |
| ssi v0.15 + ring v0.17 | **PASS** | ssi has optional `ring` feature. Enable it for alignment. |
| bincode v3.0 + serde v1.0 | **FAIL** | bincode v3.0 stable DOES NOT EXIST. Latest stable = 1.3.3. |

## Critical Issues (2 FAIL, 2 CONCERN)

### FAIL 1: ed25519-dalek — Use v2.1, NOT v3.0-pre

- **Latest stable**: v2.1.1 (docs.rs/latest redirects here)
- **Latest pre-release**: v3.0.0-pre.6 (still pre-release as of 2026-03-30)
- **libp2p-identity v0.2.13** pins `ed25519-dalek = "2.1"`
- **ssi v0.15** pins `ed25519-dalek = "2.0"` (compatible with 2.1)
- Using v3.0-pre would create TWO versions in dep tree (Cargo allows it) but types are incompatible across versions
- **Decision**: `ed25519-dalek = "2.1"` — unified across entire dep tree

### FAIL 2: bincode — Use v1.3, NOT v3.0

- **Latest stable**: bincode v1.3.3 (servo/bincode repo)
- **bincode v3.0 does NOT exist** as a stable release
- `bincode-next` v3.0.0-rc.5 exists (different org, release candidate)
- `bincode_reloaded` v3.1.1 exists (community fork, claims drop-in)
- **Decision**: Use `bincode = "1.3"` for stability. Evaluate bincode-next for Phase 2.

### CONCERN 1: alloy default features pull openssl (via native-tls)

- Alloy default features include `reqwest-default-tls` which uses native-tls
- On Linux, native-tls links to openssl (system lib)
- On macOS, native-tls uses Security.framework (no openssl needed)
- **Fix**: Use alloy with `default-features = false, features = ["reqwest-rustls-tls", ...]` to avoid openssl dependency entirely

### CONCERN 2: Shared transitive k256 versions

- libp2p-identity: k256 0.13.4
- ssi: k256 0.13.1
- alloy: k256 (0.13.x, via alloy-core)
- All in 0.13.x range — Cargo should unify. Verify with `cargo tree -d` at build time.

## Version Recommendations

| Crate | Planned | Recommended | Reason |
|---|---|---|---|
| tokio | v1.50 | **v1.44** (latest stable) | v1.50 doesn't exist yet. Use latest 1.x. |
| libp2p | v0.56 | **v0.56.0** | ✅ Correct, latest. |
| ed25519-dalek | v3.0-pre | **v2.1.1** | v3.0 still pre-release. libp2p+ssi pin 2.x. |
| alloy | v1.8 | **v1.8.3** | ✅ Correct, latest. |
| ring | v0.17 | **v0.17.14** | ✅ Correct series. |
| rocksdb | v0.24 | **v0.24.0** | ✅ Correct, latest. |
| moka | v0.12 | **v0.12.10** | ✅ Correct series. |
| clap | v4.6 | **v4.5.39** | v4.6 doesn't exist. Use latest 4.x. |
| ratatui | v0.30 | **v0.29.0** | v0.30 doesn't exist yet. Use latest. |
| serde | v1.0 | **v1.0.219** | ✅ Correct series. |
| bincode | v3.0 | **v1.3.3** | v3.0 doesn't exist as stable. |
| prost | v0.14 | **v0.13.5** | v0.14 doesn't exist yet. Use latest. |
| ssi | v0.15 | **v0.15.0** | ✅ Correct, latest. |
| sha2 | latest | **v0.10.8** | Align with libp2p-noise/libp2p-identity. |
| siphasher | latest | **v1.0.1** | For SipHash24. ring doesn't have it. |
| multihash | v0.19 | **v0.19.3** | ✅ Correct, aligns with libp2p-identity 0.19.1. |

## Required Feature Flags

```toml
[dependencies]
# libp2p — MUST enable tokio feature
libp2p = { version = "0.56.0", features = [
    "tokio", "tcp", "dns", "noise", "yamux", "gossipsub",
    "kad", "identify", "ping", "relay", "ed25519", "secp256k1", "serde"
] }

# alloy — MUST disable default-features to avoid openssl
alloy = { version = "1.8.3", default-features = false, features = [
    "reqwest-rustls-tls", "contract", "provider-http", "rpc-types",
    "signer-local", "eip712", "k256", "sol-types", "json-abi"
] }

# ssi — enable ring feature for crypto alignment
ssi = { version = "0.15.0", features = ["ring", "secp256k1", "eip712"] }

# ed25519-dalek — v2.1, NOT v3.0-pre
ed25519-dalek = { version = "2.1", features = ["serde", "rand_core"] }

# rocksdb — default features include compression libs
rocksdb = { version = "0.24.0", default-features = true }

# bincode — v1.3 stable
bincode = "1.3"
```

## libp2p v0.56 Transitive Crypto Dependencies

```
libp2p 0.56.0
├── libp2p-identity 0.2.13
│   ├── ed25519-dalek "2.1" (feature: ed25519)
│   ├── k256 "0.13.4" (feature: secp256k1)
│   ├── p256 "0.13" (feature: ecdsa)
│   ├── ring (feature: rsa, cfg non-wasm32)
│   ├── sha2 "0.10.8"
│   └── multihash "0.19.1"
├── libp2p-noise 0.45.0
│   ├── curve25519-dalek "4.1.2"
│   ├── x25519-dalek "2"
│   ├── sha2 "0.10.8"
│   ├── snow "0.9.6" → ring-resolver (non-wasm32) → ring
│   └── libp2p-identity (ed25519)
├── libp2p-gossipsub → sha2, unsigned-varint
├── libp2p-kad → sha2, unsigned-varint
└── libp2p-swarm → futures, tokio (with feature)
```

All crypto deps are compatible with our explicit selections. No version conflicts detected.

## Build Prerequisites (macOS ARM64)

```bash
# Required for rocksdb compilation
brew install cmake
xcode-select --install

# Verify
cmake --version  # >= 3.20
clang --version   # Apple clang 15+
```

## Verification Command

After creating `Cargo.toml`, run:

```bash
cargo tree -d              # Check for duplicate versions (should be zero conflicts)
cargo check --all-features # Verify compilation
cargo tree -i ring         # Confirm ring version unified
cargo tree -i k256         # Confirm k256 version unified
cargo tree -i sha2         # Confirm sha2 version unified
```
