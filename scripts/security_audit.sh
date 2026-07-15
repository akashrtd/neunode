#!/usr/bin/env bash
set -euo pipefail

# Cargo.lock includes optional dependencies even when no enabled feature can
# compile them. Guard each lock-only exception with an explicit reachability
# check before asking cargo-audit to ignore it.
assert_unreachable() {
  local package=$1
  shift

  local tree
  tree=$(cargo tree "$@" -i "$package" 2>/dev/null)
  if [[ -n "$tree" ]]; then
    echo "security exception became reachable: $package" >&2
    echo "$tree" >&2
    exit 1
  fi
}

# libp2p 0.56 records optional DNS/mDNS crates in Cargo.lock. Neunode does not
# enable either transport, so the vulnerable Hickory version is unreachable.
assert_unreachable hickory-proto@0.25.2 -p neunode-p2p

# dcap-qvl-webpki records its optional RustCrypto RSA backend in Cargo.lock.
# Neunode's Intel verifier selects the Ring backend only.
assert_unreachable rsa@0.9.10 -p neunode-verification --features tee-intel

# Alloy's integer stack records optional arkworks integration in Cargo.lock.
# No Neunode target enables those features; remove this exception when ruint
# drops ark-ff/derivative from its published dependency metadata.
assert_unreachable derivative@2.2.0

# lazy_static records its no_std spin backend in Cargo.lock, but every Neunode
# target uses the standard-library backend. Remove this guard when lazy_static
# publishes a release without the yanked optional dependency.
assert_unreachable spin@0.9.8

cargo audit \
  --ignore RUSTSEC-2026-0118 \
  --ignore RUSTSEC-2026-0119 \
  --ignore RUSTSEC-2023-0071 \
  --ignore RUSTSEC-2024-0388
