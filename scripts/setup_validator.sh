#!/usr/bin/env bash
#
# scripts/setup_validator.sh
#
# Neunode L1 Validator Setup
# ===========================
# Purpose: Generate validator keys, register with governance contract,
#          configure and start a Neunode L1 validator node combining
#          Reth (EL) + Malachite (CL) + Engine API Shim + Reputation Module.
#
# Requirements:
#   - jq, curl, sed, openssl
#   - Reth (execution client, Go binary)
#   - Malachite (consensus client, Rust/Go binary)
#   - Engine API shim (Go binary)
#   - cast (Foundry) for contract interaction
#
# Environment variables (can also be set in .env file):
#   NEUNODE_HOME          - Base directory for all node data (default: ${HOME}/.neunode)
#   RETH_BINARY           - Path to reth binary (default: reth)
#   MALACHITE_BINARY      - Path to malachite binary (default: malachite)
#   ENGINE_API_BINARY     - Path to engine-api binary (default: engine-api-shim)
#   CAST_BINARY           - Path to cast binary (default: cast)
#   GOVERNANCE_RPC_URL    - RPC URL of deployed governance contract (for registration)
#   GOVERNANCE_ADDRESS    - Address of governance contract
#   VALIDATOR_PRIVATE_KEY - Private key for validator (if not generating)
#   CHAIN_ID              - Neunode L1 chain ID (default: 1337)
#   LOG_DIR               - Log directory (default: ${NEUNODE_HOME}/logs)
#   DATA_DIR              - Data directory (default: ${NEUNODE_HOME}/data)
#   CONFIG_DIR            - Config directory (default: ${NEUNODE_HOME}/config)
#
# Actions:
#   1. Install and configure dependencies
#   2. Generate validator key pair (ECDSA)
#   3. Generate node key (for libp2p networking)
#   4. Create Reth configuration files (genesis, toml)
#   5. Create Malachite configuration files (config.toml, validator key)
#   6. Create Engine API shim configuration
#   7. Register validator with governance contract (if RPC provided)
#   8. Start all components in order with health checks
#
# Usage:
#   ./setup_validator.sh [--help|--init|--start|--status|--stop]
#     (default: perform full setup and start)

set -euo pipefail

# -----------------------------------------------------------------------------
# Constants and defaults
# -----------------------------------------------------------------------------
SCRIPT_VERSION="1.0.0"
NEUNODE_HOME="${NEUNODE_HOME:-${HOME}/.neunode}"
DATA_DIR="${DATA_DIR:-${NEUNODE_HOME}/data}"
CONFIG_DIR="${CONFIG_DIR:-${NEUNODE_HOME}/config}"
LOG_DIR="${LOG_DIR:-${NEUNODE_HOME}/logs}"
CHAIN_ID="${CHAIN_ID:-1337}"

RETH_BINARY="${RETH_BINARY:-reth}"
MALACHITE_BINARY="${MALACHITE_BINARY:-malachite}"
ENGINE_API_BINARY="${ENGINE_API_BINARY:-engine-api-shim}"
CAST_BINARY="${CAST_BINARY:-cast}"

GOVERNANCE_RPC_URL="${GOVERNANCE_RPC_URL:-}"
GOVERNANCE_ADDRESS="${GOVERNANCE_ADDRESS:-}"
VALIDATOR_PRIVATE_KEY="${VALIDATOR_PRIVATE_KEY:-}"

# Default ports (can be overridden via config files after generation)
RETH_AUTHPORT="${RETH_AUTHPORT:-8551}"
RETH_HTTPPORT="${RETH_HTTPPORT:-8545}"
MALACHITE_P2PPORT="${MALACHITE_P2PPORT:-26656}"
MALACHITE_RPCPORT="${MALACHITE_RPCPORT:-26657}"
ENGINE_API_PORT="${ENGINE_API_PORT:-8080}"

# -----------------------------------------------------------------------------
# Logging and error handling
# -----------------------------------------------------------------------------
log_info()  { echo "[INFO]  $(date '+%Y-%m-%d %H:%M:%S') $*"; }
log_warn()  { echo "[WARN]  $(date '+%Y-%m-%d %H:%M:%S') $*" >&2; }
log_error() { echo "[ERROR] $(date '+%Y-%m-%d %H:%M:%S') $*" >&2; }

cleanup() {
    log_info "Cleaning up temporary files..."
    # Add any cleanup if needed
}
trap cleanup EXIT

die() {
    log_error "$*"
    exit 1
}

# -----------------------------------------------------------------------------
# Prerequisite checks
# -----------------------------------------------------------------------------
check_prerequisites() {
    local missing=0
    for cmd in "$RETH_BINARY" "$MALACHITE_BINARY" "$ENGINE_API_BINARY" "$CAST_BINARY" jq curl; do
        if ! command -v "$cmd" &>/dev/null; then
            log_error "Required command '$cmd' not found in PATH."
            missing=1
        fi
    done
    if [[ $missing -eq 1 ]]; then
        die "Please install all required binaries and dependencies."
    fi
    log_info "All prerequisite commands are available."
}

# -----------------------------------------------------------------------------
# Directory setup
# -----------------------------------------------------------------------------
setup_directories() {
    mkdir -p "$DATA_DIR" "$CONFIG_DIR" "$LOG_DIR"
    mkdir -p "$DATA_DIR/reth" "$DATA_DIR/malachite" "$DATA_DIR/engine-api"
    log_info "Directory structure created at $NEUNODE_HOME"
}

# -----------------------------------------------------------------------------
# Key generation
# -----------------------------------------------------------------------------
generate_validator_key() {
    local key_dir="$CONFIG_DIR/validator"
    mkdir -p "$key_dir"

    if [[ -f "$key_dir/private_key.pem" && -f "$key_dir/public_key.pem" ]]; then
        log_info "Validator keys already exist, skipping generation."
        return
    fi

    log_info "Generating ECDSA validator key pair..."
    openssl ecparam -name secp256k1 -genkey -noout -out "$key_dir/private_key.pem" 2>/dev/null
    openssl ec -in "$key_dir/private_key.pem" -pubout -out "$key_dir/public_key.pem" 2>/dev/null
    # Also export hex private key for cast (unencrypted, careful)
    openssl ec -in "$key_dir/private_key.pem" -outform DER 2>/dev/null | tail -c 32 | xxd -p -c 32 > "$key_dir/private_key_hex.txt"
    chmod 600 "$key_dir/private_key.pem" "$key_dir/private_key_hex.txt"
    log_info "Validator ECDSA keys generated at $key_dir"
}

generate_node_key() {
    local node_key_file="$CONFIG_DIR/node_key.txt"

    if [[ -f "$node_key_file" ]]; then
        log_info "Node key already exists, skipping generation."
        return
    fi

    log_info "Generating libp2p node key (ECDSA)..."
    openssl ecparam -name secp256k1 -genkey -noout -out "$CONFIG_DIR/node_key_temp.pem" 2>/dev/null
    # Extract private key bytes and hex encode for libp2p
    openssl ec -in "$CONFIG_DIR/node_key_temp.pem" -outform DER 2>/dev/null | tail -c 32 | xxd -p -c 32 > "$node_key_file"
    rm -f "$CONFIG_DIR/node_key_temp.pem"
    chmod 600 "$node_key_file"
    log_info "Node key generated at $node_key_file"

    # Also compute peer ID (optional, just for reference)
    echo "Peer ID will be derived on first start."
}

# -----------------------------------------------------------------------------
# Configuration file generation
# -----------------------------------------------------------------------------
generate_reth_config() {
    local reth_config="$CONFIG_DIR/reth.toml"
    local reth_data="$DATA_DIR/reth"

    if [[ -f "$reth_config" ]]; then
        log_info "Reth configuration already exists, skipping."
        return
    fi

    log_info "Generating Reth configuration..."

    cat > "$reth_config" <<EOF
[chain]
chain_id = $CHAIN_ID
datadir = "$reth_data"

[http]
enabled = true
address = "127.0.0.1"
port = $RETH_HTTPPORT
authport = $RETH_AUTHPORT

[engine]
jwt_secret = "$CONFIG_DIR/jwt_secret.txt"
EOF

    # Generate JWT secret for Engine API communication
    openssl rand -hex 32 > "$CONFIG_DIR/jwt_secret.txt"
    chmod 600 "$CONFIG_DIR/jwt_secret.txt"

    log_info "Reth config written to $reth_config"
}

generate_malachite_config() {
    local malachite_config="$CONFIG_DIR/malachite.toml"
    local malachite_data="$DATA_DIR/malachite"

    if [[ -f "$malachite_config" ]]; then
        log_info "Malachite configuration already exists, skipping."
        return
    fi

    log_info "Generating Malachite configuration..."

    # Read public key for validator identity
    local pubkey_hex
    pubkey_hex=$(openssl ec -in "$CONFIG_DIR/validator/public_key.pem" -pubin -outform DER 2>/dev/null | xxd -p -c 256)

    cat > "$malachite_config" <<EOF
[validator]
key_file = "$CONFIG_DIR/validator/private_key.pem"
pubkey_hex = "$pubkey_hex"

[consensus]
timeout_propose = "3s"
timeout_prevote = "1s"
timeout_precommit = "1s"
timeout_commit = "5s"

[p2p]
laddr = "tcp://0.0.0.0:$MALACHITE_P2PPORT"
persistent_peers = ""
seeds = ""

[rpc]
laddr = "tcp://127.0.0.1:$MALACHITE_RPCPORT"

[db]
path = "$malachite_data"
EOF

    log_info "Malachite config written to $malachite_config"
}

generate_engine_api_config() {
    local engine_config="$CONFIG_DIR/engine-api.toml"

    if [[ -f "$engine_config" ]]; then
        log_info "Engine API shim configuration already exists, skipping."
        return
    fi

    log_info "Generating Engine API shim configuration..."

    cat > "$engine_config" <<EOF
[server]
listen_address = "127.0.0.1:$ENGINE_API_PORT"

[reth]
rpc_url = "http://127.0.0.1:$RETH_HTTPPORT"
jwt_secret = "$CONFIG_DIR/jwt_secret.txt"

[malachite]
rpc_url = "http://127.0.0.1:$MALACHITE_RPCPORT"

[reputation]
weight_stake = 0.30
weight_attestations = 0.25
weight_activity = 0.20
weight_verify = 0.15
weight_tenure = 0.10
EOF

    log_info "Engine API config written to $engine_config"
}

generate_genesis() {
    local genesis_file="$CONFIG_DIR/genesis.json"

    if [[ -f "$genesis_file" ]]; then
        log_info "Genesis file already exists, skipping."
        return
    fi

    log_info "Generating genesis.json for Neunode L1..."

    cat > "$genesis_file" <<EOF
{
  "config": {
    "chainId": $CHAIN_ID,
    "homesteadBlock": 0,
    "eip150Block": 0,
    "eip150Hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "eip155Block": 0,
    "eip158Block": 0,
    "byzantiumBlock": 0,
    "constantinopleBlock": 0,
    "petersburgBlock": 0,
    "istanbulBlock": 0,
    "berlinBlock": 0,
    "londonBlock": 0,
    "clique": {
      "period": 5,
      "epoch": 30000
    }
  },
  "nonce": "0x0",
  "timestamp": "0x0",
  "extraData": "0x0000000000000000000000000000000000000000000000000000000000000000",
  "gasLimit": "0x1c9c380",
  "difficulty": "0x1",
  "mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
  "coinbase": "0x0000000000000000000000000000000000000000",
  "alloc": {},
  "number": "0x0",
  "gasUsed": "0x0",
  "parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
}
EOF

    log_info "Genesis file created at $genesis_file"
}

# -----------------------------------------------------------------------------
# Governance contract interaction
# -----------------------------------------------------------------------------
register_with_governance() {
    if [[ -z "$GOVERNANCE_RPC_URL" || -z "$GOVERNANCE_ADDRESS" ]]; then
        log_warn "GOVERNANCE_RPC_URL or GOVERNANCE_ADDRESS not set, skipping governance registration."
        return
    fi

    local privkey_hex
    privkey_hex=$(cat "$CONFIG_DIR/validator/private_key_hex.txt" 2>/dev/null || echo "")
    if [[ -z "$privkey_hex" ]]; then
        log_warn "Validator hex private key not found, cannot register."
        return
    fi

    local val_pubkey_hex
    val_pubkey_hex=$(openssl ec -in "$CONFIG_DIR/validator/public_key.pem" -pubin -outform DER 2>/dev/null | xxd -p -c 256 | head -1)

    log_info "Registering validator with governance contract at $GOVERNANCE_ADDRESS..."

    # Call registerValidator(address _signer, bytes memory _pubkey, uint256 _stake) using cast
    # Adjust function call according to actual governance ABI
    local tx_hash
    tx_hash=$("$CAST_BINARY" send \
        --rpc-url "$GOVERNANCE_RPC_URL" \
        --private-key "$privkey_hex" \
        "$GOVERNANCE_ADDRESS" \
        "registerValidator(address,bytes)(bool)" \
        "$(cat "$CONFIG_DIR/validator/public_key_eth_address.txt" 2>/dev/null || echo "0x0000000000000000000000000000000000000000")" \
        "0x${val_pubkey_hex}" \
        --value 1000000000000000000 2>/dev/null || {
            log_warn "Governance registration transaction failed. Continuing..."
            return
        })

    log_info "Registration transaction sent: $tx_hash"
    # Optional: wait for receipt
    sleep 2
    local receipt
    receipt=$("$CAST_BINARY" receipt --rpc-url "$GOVERNANCE_RPC_URL" "$tx_hash" 2>/dev/null || echo "")
    if [[ -n "$receipt" ]]; then
        log_info "Transaction confirmed: $receipt"
    fi
}

# -----------------------------------------------------------------------------
# Process management
# -----------------------------------------------------------------------------
start_reth() {
    log_info "Starting Reth execution layer..."
    local reth_log="$LOG_DIR/reth.log"

    # Check if already running
    if pgrep -f "reth.*--datadir $DATA_DIR/reth" >/dev/null 2>&1; then
        log_warn "Reth already running."
        return
    fi

    "$RETH_BINARY" \
        --datadir "$DATA_DIR/reth" \
        --config "$CONFIG_DIR/reth.toml" \
        --chain "$CONFIG_DIR/genesis.json" \
        --http \
        --http.port "$RETH_HTTPPORT" \
        --authrpc.port "$RETH_AUTHPORT" \
        --authrpc.jwtsecret "$CONFIG_DIR/jwt_secret.txt" \
        >> "$reth_log" 2>&1 &

    local pid=$!
    echo "$pid" > "$DATA_DIR/reth.pid"
    log_info "Reth started with PID $pid (log: $reth_log)"

    # Wait for RPC to be available
    for i in $(seq 1 30); do
        if curl -s -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' "http://127.0.0.1:$RETH_HTTPPORT" >/dev/null 2>&1; then
            log_info "Reth RPC ready."
            break
        fi
        sleep 2
    done
}

start_malachite() {
    log_info "Starting Malachite consensus layer..."
    local malachite_log="$LOG_DIR/malachite.log"

    if pgrep -f "malachite.*--config $CONFIG_DIR/malachite.toml" >/dev/null 2>&1; then
        log_warn "Malachite already running."
        return
    fi

    "$MALACHITE_BINARY" \
        --config "$CONFIG_DIR/malachite.toml" \
        --data "$DATA_DIR/malachite" \
        --validator-key "$CONFIG_DIR/validator/private_key.pem" \
        >> "$malachite_log" 2>&1 &

    local pid=$!
    echo "$pid" > "$DATA_DIR/malachite.pid"
    log_info "Malachite started with PID $pid (log: $malachite_log)"
}

start_engine_api() {
    log_info "Starting Engine API shim..."
    local engine_log="$LOG_DIR/engine-api.log"

    if pgrep -f "engine-api-shim.*--config $CONFIG_DIR/engine-api.toml" >/dev/null 2>&1; then
        log_warn "Engine API shim already running."
        return
    fi

    "$ENGINE_API_BINARY" \
        --config "$CONFIG_DIR/engine-api.toml" \
        >> "$engine_log" 2>&1 &

    local pid=$!
    echo "$pid" > "$DATA_DIR/engine-api.pid"
    log_info "Engine API shim started with PID $pid (log: $engine_log)"
}

stop_all() {
    log_info "Stopping all Neunode components..."
    if [[ -f "$DATA_DIR/engine-api.pid" ]]; then
        kill "$(cat "$DATA_DIR/engine-api.pid")" 2>/dev/null || true
        rm -f "$DATA_DIR/engine-api.pid"
    fi
    if [[ -f "$DATA_DIR/malachite.pid" ]]; then
        kill "$(cat "$DATA_DIR/malachite.pid")" 2>/dev/null || true
        rm -f "$DATA_DIR/malachite.pid"
    fi
    if [[ -f "$DATA_DIR/reth.pid" ]]; then
        kill "$(cat "$DATA_DIR/reth.pid")" 2>/dev/null || true
        rm -f "$DATA_DIR/reth.pid"
    fi
    log_info "All components stopped."
}

status_all() {
    echo ""
    echo "=== Neunode Validator Status ==="
    echo ""

    if [[ -f "$DATA_DIR/reth.pid" ]] && kill -0 "$(cat "$DATA_DIR/reth.pid")" 2>/dev/null; then
        echo "Reth EL:           RUNNING (PID $(cat "$DATA_DIR/reth.pid"))"
    else
        echo "Reth EL:           STOPPED"
    fi

    if [[ -f "$DATA_DIR/malachite.pid" ]] && kill -0 "$(cat "$DATA_DIR/malachite.pid")" 2>/dev/null; then
        echo "Malachite CL:      RUNNING (PID $(cat "$DATA_DIR/malachite.pid"))"
    else
        echo "Malachite CL:      STOPPED"
    fi

    if [[ -f "$DATA_DIR/engine-api.pid" ]] && kill -0 "$(cat "$DATA_DIR/engine-api.pid")" 2>/dev/null; then
        echo "Engine API Shim:   RUNNING (PID $(cat "$DATA_DIR/engine-api.pid"))"
    else
        echo "Engine API Shim:   STOPPED"
    fi

    echo ""
    echo "Data directory: $DATA_DIR"
    echo "Config directory: $CONFIG_DIR"
    echo "Log directory: $LOG_DIR"
    echo ""
}

# -----------------------------------------------------------------------------
# Initialization (setup without starting)
# -----------------------------------------------------------------------------
do_init() {
    check_prerequisites
    setup_directories
    generate_validator_key
    generate_node_key
    generate_genesis
    generate_reth_config
    generate_malachite_config
    generate_engine_api_config
    register_with_governance
    log_info "Initialisation complete. Use --start to run the node."
}

# -----------------------------------------------------------------------------
# Start (assumes init has been run)
# -----------------------------------------------------------------------------
do_start() {
    if [[ ! -f "$CONFIG_DIR/genesis.json" || ! -f "$CONFIG_DIR/reth.toml" ]]; then
        log_warn "Configuration files missing. Running init first..."
        do_init
    fi
    start_reth
    start_malachite
    start_engine_api
    log_info "All components started. Use --status to verify."
}

# -----------------------------------------------------------------------------
# Usage
# -----------------------------------------------------------------------------
usage() {
    cat <<EOF
Usage: $0 [OPTION]

Neunode L1 Validator Setup Script v${SCRIPT_VERSION}

Options:
  --help        Show this message and exit
  --init        Generate keys and configuration only (do not start)
  --start       Start all components (init if needed)
  --status      Show running status of components
  --stop        Stop all running components

Without options, performs full setup and start.

Environment variables (with defaults):
  NEUNODE_HOME            ${NEUNODE_HOME}
  RETH_BINARY             ${RETH_BINARY}
  MALACHITE_BINARY        ${MALACHITE_BINARY}
  ENGINE_API_BINARY       ${ENGINE_API_BINARY}
  CAST_BINARY             ${CAST_BINARY}
  CHAIN_ID                ${CHAIN_ID}
  GOVERNANCE_RPC_URL      ${GOVERNANCE_RPC_URL:-<unset>}
  GOVERNANCE_ADDRESS      ${GOVERNANCE_ADDRESS:-<unset>}
EOF
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------
main() {
    local action="${1:-full}"

    case "$action" in
        --help|-h)
            usage
            exit 0
            ;;
        --init)
            do_init
            ;;
        --start)
            do_start
            ;;
        --status)
            status_all
            ;;
        --stop)
            stop_all
            ;;
        *)
            # Full setup and start
            log_info "Starting full validator setup..."
            do_init
            do_start
            status_all
            ;;
    esac
}

main "$@"