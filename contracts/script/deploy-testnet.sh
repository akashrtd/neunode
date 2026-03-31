#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Load .env if present
if [[ -f "$PROJECT_ROOT/.env" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$PROJECT_ROOT/.env"
  set +a
fi

# Defaults
CHAIN="${CHAIN:-holesky}"
PRIVATE_KEY="${PRIVATE_KEY:-}"
RPC_URL="${RPC_URL:-}"
VERIFY="${VERIFY:-true}"

# Chain ID mapping
declare -A CHAIN_IDS=( [holesky]=17000 [mainnet]=1 [anvil]=31337 )
# Default RPC URLs
declare -A CHAIN_RPCS=( [holesky]="https://ethereum-holesky-rpc.publicnode.com" [mainnet]="" [anvil]="http://127.0.0.1:8545" )

# Parse args
while [[ $# -gt 0 ]]; do
  case $1 in
    --chain) CHAIN="$2"; shift 2 ;;
    --rpc-url) RPC_URL="$2"; shift 2 ;;
    --private-key) PRIVATE_KEY="$2"; shift 2 ;;
    --no-verify) VERIFY="false"; shift ;;
    --verify) VERIFY="$2"; shift 2 ;;
    *) echo "Unknown arg: $1"; exit 1 ;;
  esac
done

# Resolve chain ID
CHAIN_ID="${CHAIN_IDS[$CHAIN]:-}"
if [[ -z "$CHAIN_ID" ]]; then
  echo "Error: Unknown chain '$CHAIN'. Supported: ${!CHAIN_IDS[*]}"
  exit 1
fi

# Resolve RPC URL (arg > env > chain default)
if [[ -z "$RPC_URL" ]]; then
  RPC_URL="${CHAIN_RPCS[$CHAIN]}"
fi
if [[ -z "$RPC_URL" ]]; then
  echo "Error: RPC_URL required for chain '$CHAIN' (use --rpc-url or RPC_URL env var)"
  exit 1
fi

# Private key required (except anvil)
if [[ "$CHAIN" != "anvil" ]] && [[ -z "$PRIVATE_KEY" ]]; then
  echo "Error: PRIVATE_KEY required (via --private-key, env var, or .env file)"
  exit 1
fi

if ! command -v forge &>/dev/null; then
  echo "Error: forge not found. Install: https://book.getfoundry.sh/getting-started/installation"
  exit 1
fi

echo "=== Deploying Neunode contracts ==="
echo "Chain: $CHAIN (id: $CHAIN_ID)"
echo "RPC: $RPC_URL"
echo ""

forge build --root "$PROJECT_ROOT"

FORGE_ARGS=(--root "$PROJECT_ROOT" --rpc-url "$RPC_URL" --broadcast --slow -vvv)
if [[ -n "$PRIVATE_KEY" ]]; then
  FORGE_ARGS+=(--private-key "$PRIVATE_KEY")
fi

forge script Deploy "${FORGE_ARGS[@]}"

echo ""
echo "Deployment complete!"

# Extract addresses
BROADCAST_DIR="$PROJECT_ROOT/broadcast/Deploy.s.sol/$CHAIN_ID"
npx tsx "$PROJECT_ROOT/../sdk/scripts/extract-addresses.ts" \
  --chain-id "$CHAIN_ID" \
  --broadcast-dir "$BROADCAST_DIR"

echo "Addresses updated in sdk/src/contracts/addresses.ts"

# Verify contracts on block explorer
if [[ "$VERIFY" == "true" ]] && [[ "$CHAIN" != "anvil" ]]; then
  echo ""
  if [[ -z "${ETHERSCAN_API_KEY:-}" ]]; then
    echo "Warning: ETHERSCAN_API_KEY not set, skipping automatic verification"
    echo "To verify contracts manually:"
    echo "  forge verify-contract <address> <ContractName> --chain $CHAIN --watch"
  else
    echo "Verifying contracts on block explorer..."
    BROADCAST_FILE="$BROADCAST_DIR/run-latest.json"
    if [[ -f "$BROADCAST_FILE" ]]; then
      # Parse created contracts from broadcast and verify each
      CONTRACTS=$(jq -r '.transactions[] | select(.transactionType=="CREATE" and .contractName!=null) | "\(.contractAddress) \(.contractName)"' "$BROADCAST_FILE" 2>/dev/null || true)
      while IFS=' ' read -r addr name; do
        if [[ -n "$addr" ]] && [[ -n "$name" ]]; then
          echo "  Verifying $name at $addr..."
          forge verify-contract "$addr" "$name" --root "$PROJECT_ROOT" --chain "$CHAIN" --watch 2>/dev/null || true
        fi
      done <<< "$CONTRACTS"
    fi
  fi
fi

echo ""
echo "Done!"
