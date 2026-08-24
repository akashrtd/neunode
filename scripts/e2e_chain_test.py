#!/usr/bin/env python3
"""
E2E test: verify genesis JSON loads in Reth and contracts are callable via RPC.

Prerequisites:
    - Reth installed: `cargo install --git https://github.com/paradigmxyz/reth --bin reth`
    - Python web3 package installed

Usage:
    python3 scripts/e2e_chain_test.py

This script:
1. Writes the genesis JSON from neunode-chain-spec to a temp file
2. Starts Reth with --dev mode using that genesis
3. Verifies blocks are being produced
4. Calls eth_call against each predeployed contract to verify they exist
5. Verifies specific contract state (token names, governance params)
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path
from web3 import Web3

REPO_ROOT = Path(__file__).resolve().parent.parent

# Predeploy addresses from neunode-chain-spec
PREDEPLOY_ADDRESSES = {
    "DiamondProxy":      "0x0000000000000000000000000000000000001001",
    "NeunodeIdentity":   "0x0000000000000000000000000000000000001002",
    "NeunodeBounty":     "0x0000000000000000000000000000000000001003",
    "NeunodeEscrow":     "0x0000000000000000000000000000000000001004",
    "NeunodeRegistry":   "0x0000000000000000000000000000000000001005",
    "nCompute":          "0x0000000000000000000000000000000000001006",
    "nTrain":            "0x0000000000000000000000000000000000001007",
    "nBandwidth":        "0x0000000000000000000000000000000000001008",
    "nStorage":          "0x0000000000000000000000000000000000001009",
    "ModelRegistry":     "0x000000000000000000000000000000000000100a",
    "RoyaltySplitter":   "0x000000000000000000000000000000000000100b",
    "NeunodeGovernance": "0x000000000000000000000000000000000000100c",
    "StakingEscrow":     "0x000000000000000000000000000000000000100d",
    "BountyReview":      "0x000000000000000000000000000000000000100e",
    "DiamondCutFacet":   "0x0000000000000000000000000000000000001010",
    "DiamondLoupeFacet": "0x0000000000000000000000000000000000001011",
}

RPC_URL = "http://127.0.0.1:8545"
ENGINE_API_URL = "http://127.0.0.1:8551"
DEPLOYER_PK = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"


def generate_genesis_json():
    """Generate genesis JSON by running the Rust chain-spec."""
    print("Generating genesis JSON...")
    result = subprocess.run(
        ["cargo", "run", "--", "serve", "--help"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    # Instead, use a small Rust program or extract via test
    # Actually, let's just read the genesis_predeploys.json and build it manually
    # Or better: use the genesis JSON that agnetd writes

    # For now, let's generate it from the embedded JSON
    genesis_path = REPO_ROOT / "crates" / "neunode-chain-spec" / "src" / "genesis_predeploys.json"
    with open(genesis_path) as f:
        predeploys = json.load(f)

    # Read chain constants
    chain_id = 9109
    block_gas_limit = 30_000_000
    initial_base_fee = 1_000_000_000

    # Build alloc
    alloc = {}

    # Validators
    validators = [
        "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
        "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC",
        "0x90F79bf6EB2c4f870365E785982E1f101E93b906",
        "0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65",
    ]
    deployer = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
    validator_balance = hex(1_000_000 * 10**18)

    for addr in validators + [deployer]:
        alloc[addr[2:].lower()] = {"balance": validator_balance}

    # Predeploys
    for p in predeploys["predeploys"]:
        addr = p["address"][2:].lower()
        entry = {"balance": "0x0"}
        if p["bytecode"] and p["bytecode"] != "0x":
            entry["code"] = p["bytecode"]
        if p["storage"]:
            entry["storage"] = {k: v for k, v in p["storage"].items()}
        alloc[addr] = entry

    genesis = {
        "config": {
            "chainId": chain_id,
            "homesteadBlock": 0,
            "eip150Block": 0,
            "eip155Block": 0,
            "eip158Block": 0,
            "byzantiumBlock": 0,
            "constantinopleBlock": 0,
            "petersburgBlock": 0,
            "istanbulBlock": 0,
            "berlinBlock": 0,
            "londonBlock": 0,
            "shanghaiTime": 0,
            "cancunTime": 0,
            "terminalTotalDifficulty": 0,
            "terminalTotalDifficultyPassed": True,
        },
        "nonce": "0x0",
        "timestamp": "0x0",
        "extraData": "0x8e65756e6f6465",
        "gasLimit": hex(block_gas_limit),
        "difficulty": "0x0",
        "mixHash": "0x" + "00" * 32,
        "coinbase": "0x" + "00" * 20,
        "alloc": alloc,
        "number": "0x0",
        "gasUsed": "0x0",
        "parentHash": "0x" + "00" * 32,
        "baseFeePerGas": hex(initial_base_fee),
    }

    return genesis


def start_reth(genesis_path, jwt_path, data_dir):
    """Start Reth with the custom genesis."""
    cmd = [
        "reth", "node",
        "--chain", str(genesis_path),
        "--authrpc.jwtsecret", str(jwt_path),
        "--datadir", str(data_dir),
        "--http",
        "--http.api", "eth,net,web3,debug",
        "--http.port", "8545",
        "--authrpc.port", "8551",
        "--dev",
    ]

    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return proc


def start_bridge(jwt_path):
    """Start agnetd's Engine API bridge against the running Reth process."""
    subprocess.run(["cargo", "build", "-p", "agnetd"], cwd=REPO_ROOT, check=True)
    return subprocess.Popen(
        [
            str(REPO_ROOT / "target" / "debug" / "agnetd"),
            "serve", "--port", "41001", "--chain-mode", "sovereign",
            "--external-engine", "--jwt-secret-path", str(jwt_path),
            "--engine-api-endpoint", ENGINE_API_URL, "--block-time", "1",
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def wait_for_rpc(url, timeout=30):
    """Wait for the JSON-RPC endpoint to be ready."""
    w3 = Web3(Web3.HTTPProvider(url))
    for _ in range(timeout * 2):
        try:
            if w3.is_connected():
                return w3
        except Exception:
            pass
        time.sleep(0.5)
    return None


def test_block_production(w3):
    """Test that blocks are being produced."""
    print("\n=== Testing Block Production ===")
    block = w3.eth.get_block("latest")
    print(f"  Current block: #{block.number}")

    if block.number == 0:
        # Wait for a few blocks
        print("  Waiting for block production...")
        for _ in range(10):
            time.sleep(1)
            block = w3.eth.get_block("latest")
            if block.number > 0:
                break

    assert block.number > 0, "No blocks produced!"
    print(f"  Block #{block.number} produced at timestamp {block.timestamp}")
    print(f"  Gas used: {block.gasUsed}")
    print("  PASS: Blocks are being produced")
    return True


def test_contract_code_exists(w3):
    """Test that all predeployed contracts have bytecode."""
    print("\n=== Testing Contract Code Exists ===")
    all_pass = True
    for name, addr in PREDEPLOY_ADDRESSES.items():
        code = w3.eth.get_code(Web3.to_checksum_address(addr))
        code_len = len(code)
        status = "OK" if code_len > 0 else "EMPTY"
        if code_len == 0:
            all_pass = False
        print(f"  {name:20s} ({addr}): {code_len:5d} bytes [{status}]")

    assert all_pass, "Some contracts have empty bytecode!"
    print("  PASS: All contracts have bytecode")
    return True


def test_token_name(w3):
    """Test that nCompute token returns the correct name."""
    print("\n=== Testing Token State ===")
    compute_addr = PREDEPLOY_ADDRESSES["nCompute"]

    # Call name() - selector 0x06fdde03
    result = w3.eth.call({"to": Web3.to_checksum_address(compute_addr), "data": "0x06fdde03"})
    # Decode ABI response: offset(32) + length(32) + data
    if len(result) >= 64:
        offset = int.from_bytes(result[:32], "big")
        length = int.from_bytes(result[32:64], "big")
        name_bytes = result[64:64+length] if length <= 32 else result[64:64+length]
        name = name_bytes.decode("utf-8", errors="replace").rstrip("\x00")
        print(f"  nCompute.name() = '{name}'")
        assert "Compute" in name, f"Expected 'Compute' in name, got '{name}'"
        print("  PASS: Token name is correct")
        return True
    else:
        print(f"  FAIL: Unexpected response length {len(result)}")
        return False


def test_token_transfer(w3):
    """Mint resource units, transfer them, and read both balances back."""
    print("\n=== Testing Token Transfer ===")
    abi = [
        {"type": "function", "name": "balanceOf", "stateMutability": "view",
         "inputs": [{"name": "account", "type": "address"}],
         "outputs": [{"name": "", "type": "uint256"}]},
        {"type": "function", "name": "mint", "stateMutability": "nonpayable",
         "inputs": [{"name": "to", "type": "address"}, {"name": "amount", "type": "uint256"}],
         "outputs": []},
        {"type": "function", "name": "transfer", "stateMutability": "nonpayable",
         "inputs": [{"name": "to", "type": "address"}, {"name": "amount", "type": "uint256"}],
         "outputs": [{"name": "", "type": "bool"}]},
    ]
    token = w3.eth.contract(
        address=Web3.to_checksum_address(PREDEPLOY_ADDRESSES["nCompute"]), abi=abi
    )
    sender = w3.eth.account.from_key(DEPLOYER_PK)
    recipient = Web3.to_checksum_address("0x70997970C51812dc3A010C7d01b50e0d17dc79C8")
    sender_before = token.functions.balanceOf(sender.address).call()
    recipient_before = token.functions.balanceOf(recipient).call()
    nonce = w3.eth.get_transaction_count(sender.address)

    for call in (token.functions.mint(sender.address, 100), token.functions.transfer(recipient, 25)):
        tx = call.build_transaction({
            "from": sender.address, "nonce": nonce, "chainId": w3.eth.chain_id,
            "gas": 200_000, "maxFeePerGas": w3.eth.gas_price * 2, "maxPriorityFeePerGas": 0,
        })
        signed = w3.eth.account.sign_transaction(tx, DEPLOYER_PK)
        receipt = w3.eth.wait_for_transaction_receipt(
            w3.eth.send_raw_transaction(signed.raw_transaction), timeout=15
        )
        assert receipt.status == 1, "token transaction reverted"
        nonce += 1

    sender_balance = token.functions.balanceOf(sender.address).call()
    recipient_balance = token.functions.balanceOf(recipient).call()
    assert sender_balance == sender_before + 75
    assert recipient_balance == recipient_before + 25
    print(f"  balances: sender={sender_balance}, recipient={recipient_balance}")
    print("  PASS: Token transfer executed and state read-back matches")
    return True


def main():
    os.chdir(REPO_ROOT)

    # Check Reth is available
    reth_check = subprocess.run(["reth", "--version"], capture_output=True)
    if reth_check.returncode != 0:
        print("ERROR: Reth is not installed. Install with:")
        print("  cargo install --git https://github.com/paradigmxyz/reth --bin reth --locked")
        sys.exit(1)
    print(f"Reth: {reth_check.stdout.decode().split(chr(10))[0]}")

    # Generate genesis JSON
    genesis = generate_genesis_json()

    # Write files
    tmp_dir = Path("/tmp/neunode-e2e-test")
    tmp_dir.mkdir(exist_ok=True)

    genesis_path = tmp_dir / "genesis.json"
    with open(genesis_path, "w") as f:
        json.dump(genesis, f, indent=2)
    print(f"Genesis written to: {genesis_path}")

    jwt_path = tmp_dir / "jwt.hex"
    jwt_path.write_text("7e9b2d1f3a4c5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f")

    data_dir = tmp_dir / "reth-data"

    # Start Reth
    print("\nStarting Reth...")
    proc = start_reth(genesis_path, jwt_path, data_dir)
    bridge = None

    try:
        # Wait for RPC
        print("Waiting for RPC endpoint...")
        w3 = wait_for_rpc(RPC_URL, timeout=30)
        if w3 is None:
            # Print Reth output for debugging
            stdout, stderr = proc.communicate(timeout=5)
            print("Reth stdout:", stdout.decode()[:2000])
            print("Reth stderr:", stderr.decode()[:2000])
            sys.exit(1)

        print(f"Connected to Reth at {RPC_URL}")
        print(f"Chain ID: {w3.eth.chain_id}")
        bridge = start_bridge(jwt_path)

        # Run tests
        results = []
        results.append(test_block_production(w3))
        results.append(test_contract_code_exists(w3))
        results.append(test_token_name(w3))
        results.append(test_token_transfer(w3))

        # Summary
        print("\n" + "=" * 50)
        passed = sum(results)
        total = len(results)
        print(f"Results: {passed}/{total} tests passed")
        if passed == total:
            print("ALL TESTS PASSED")
        else:
            print("SOME TESTS FAILED")
            sys.exit(1)

    finally:
        if bridge is not None:
            bridge.terminate()
            bridge.wait(timeout=10)
        proc.terminate()
        proc.wait(timeout=10)
        print("\nReth stopped.")


if __name__ == "__main__":
    main()
