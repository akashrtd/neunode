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

# cargo-audit's advisory ignores are intentionally narrow, but it does not have
# an equivalent ignore mechanism for yanked lock-only packages. Pin the entire
# observed finding set so any newly introduced vulnerability, advisory, or yank
# fails this gate instead of blending into the known upstream exceptions.
report=$(mktemp)
trap 'rm -f "$report"' EXIT
cargo audit --json >"$report" || true

assert_exact() {
  local label=$1
  local actual=$2
  local expected=$3

  if [[ "$actual" != "$expected" ]]; then
    echo "unexpected cargo-audit $label" >&2
    echo "expected: $expected" >&2
    echo "actual:   $actual" >&2
    exit 1
  fi
}

vulnerabilities=$(jq -r '[.vulnerabilities.list[].advisory.id] | sort | join(" ")' "$report")
unmaintained=$(jq -r '[.warnings.unmaintained[].advisory.id] | sort | join(" ")' "$report")
yanked=$(jq -r '[.warnings.yanked[] | "\(.package.name)@\(.package.version)"] | sort | join(" ")' "$report")

assert_exact vulnerabilities "$vulnerabilities" \
  "RUSTSEC-2023-0071 RUSTSEC-2026-0118 RUSTSEC-2026-0119"
assert_exact unmaintained "$unmaintained" \
  "RUSTSEC-2024-0388 RUSTSEC-2024-0436 RUSTSEC-2025-0141 RUSTSEC-2026-0173"
assert_exact yanked "$yanked" "spin@0.9.8"

cargo audit \
  --ignore RUSTSEC-2026-0118 \
  --ignore RUSTSEC-2026-0119 \
  --ignore RUSTSEC-2023-0071 \
  --ignore RUSTSEC-2024-0388
