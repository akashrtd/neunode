#!/usr/bin/env python3
"""
Genesis predeploy codegen for Neunode L1 chain.

Deploys all contracts on Anvil, dumps the resulting state, and generates
a genesis predeploy JSON file with bytecode + storage at deterministic addresses.

Usage:
    python3 scripts/genesis_codegen.py

Prerequisites:
    - forge, cast, anvil in PATH
    - contracts compiled: cd contracts && forge build

Output:
    crates/neunode-chain-spec/src/genesis_predeploys.json
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path
from web3 import Web3

REPO_ROOT = Path(__file__).resolve().parent.parent
CONTRACTS_DIR = REPO_ROOT / "contracts"
OUTPUT_FILE = REPO_ROOT / "crates" / "neunode-chain-spec" / "src" / "genesis_predeploys.json"

# Deterministic predeploy addresses (match predeploys.rs constants)
PREDEPLOY_ADDRESSES = {
    "DiamondProxy":    "0x0000000000000000000000000000000000001001",
    "NeunodeIdentity": "0x0000000000000000000000000000000000001002",
    "NeunodeBounty":   "0x0000000000000000000000000000000000001003",
    "NeunodeEscrow":   "0x0000000000000000000000000000000000001004",
    "NeunodeRegistry": "0x0000000000000000000000000000000000001005",
    "nCompute":        "0x0000000000000000000000000000000000001006",
    "nTrain":          "0x0000000000000000000000000000000000001007",
    "nBandwidth":      "0x0000000000000000000000000000000000001008",
    "nStorage":        "0x0000000000000000000000000000000000001009",
    "ModelRegistry":   "0x000000000000000000000000000000000000100a",
    "RoyaltySplitter": "0x000000000000000000000000000000000000100b",
    "NeunodeGovernance":"0x000000000000000000000000000000000000100c",
    "StakingEscrow":   "0x000000000000000000000000000000000000100d",
    "BountyReview":    "0x000000000000000000000000000000000000100e",
    "DiamondCutFacet":  "0x0000000000000000000000000000000000001010",
    "DiamondLoupeFacet":"0x0000000000000000000000000000000000001011",
}

# Anvil account 0 (default deployer)
DEPLOYER = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
DEPLOYER_PK = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
ANVIL_URL = "http://127.0.0.1:9545"

# Governance defaults (must match Deploy.s.sol)
GOV_PARAMS = {
    "voting_delay": 86400,        # 1 day
    "voting_period": 604800,      # 7 days
    "proposal_threshold": 100 * 10**18,
    "quorum_bps": 400,
    "timelock": 172800,           # 2 days
    "execution_window": 1209600,  # 14 days
}


def run(cmd, cwd=None, check=True, capture=True):
    """Run a command and return output."""
    result = subprocess.run(
        cmd, cwd=cwd, check=check,
        capture_output=capture, text=True
    )
    return result.stdout.strip() if capture else None


def start_anvil():
    """Start Anvil on port 9545 and return the process."""
    proc = subprocess.Popen(
        ["anvil", "--port", "9545", "--accounts", "10", "--balance", "100000"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
    )
    # Wait for Anvil to be ready
    w3 = Web3(Web3.HTTPProvider(ANVIL_URL))
    for _ in range(30):
        try:
            if w3.is_connected():
                return proc
        except Exception:
            pass
        time.sleep(0.5)
    proc.kill()
    raise RuntimeError("Anvil did not start within 15 seconds")


def deploy_contracts(w3):
    """Deploy all contracts and return {name: address} mapping."""
    accounts = w3.eth.accounts
    deployer = accounts[0]

    addresses = {}
    nonce = w3.eth.get_transaction_count(deployer)

    def deploy(contract_name, abi_file, bytecode_hex, constructor_args=b"", label=None):
        nonlocal nonce
        label = label or contract_name

        # Build deployment transaction
        bytecode = bytes.fromhex(bytecode_hex[2:]) if bytecode_hex.startswith("0x") else bytes.fromhex(bytecode_hex)
        data = bytecode + constructor_args

        tx = {
            "from": deployer,
            "data": "0x" + data.hex(),
            "nonce": nonce,
            "gas": 10_000_000,
            "gasPrice": w3.eth.gas_price,
            "chainId": w3.eth.chain_id,
        }

        tx_hash = w3.eth.send_transaction(tx)
        receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
        addr = receipt.contractAddress
        nonce += 1

        if receipt.status != 1:
            raise RuntimeError(f"Deployment of {label} failed")

        addresses[label] = addr
        print(f"  {label}: {addr}")
        return addr

    # Read forge artifacts
    def read_artifact(sol_file, contract_name):
        path = CONTRACTS_DIR / "out" / f"{sol_file}" / f"{contract_name}.json"
        with open(path) as f:
            return json.load(f)

    def encode_address(addr):
        """Encode an address as constructor arg."""
        return bytes.fromhex(addr[2:].lower())

    def encode_uint256(val):
        """Encode a uint256 as constructor arg."""
        return val.to_bytes(32, "big")

    def encode_string(s):
        """Encode a string as a Solidity constructor arg (dynamic type)."""
        # ABI encoding for string: offset (32 bytes) + length (32 bytes) + data (padded)
        data = s.encode("utf-8")
        offset = encode_uint256(32)
        length = encode_uint256(len(data))
        padded_data = data + b"\x00" * (32 - len(data) % 32) if len(data) % 32 else data
        return offset + length + padded_data

    def encode_tuple_address_uints(addr, uints):
        """Encode (address, uint256, uint256, ...) as constructor args."""
        result = encode_address(addr)
        for u in uints:
            result += encode_uint256(u)
        return result

    print("Deploying contracts...")

    # 1. Tokens (no constructor args)
    for sol, name, label in [
        ("ComputeToken.sol", "ComputeToken", "nCompute"),
        ("TrainingToken.sol", "TrainingToken", "nTrain"),
        ("BandwidthToken.sol", "BandwidthToken", "nBandwidth"),
        ("StorageToken.sol", "StorageToken", "nStorage"),
    ]:
        art = read_artifact(sol, sol.replace(".sol", ""))
        bytecode = art["bytecode"]["object"]
        deploy(sol, sol, bytecode, label=label)

    # 2. Identity (no constructor args)
    art = read_artifact("NeunodeIdentity.sol", "NeunodeIdentity")
    deploy("NeunodeIdentity", "NeunodeIdentity.sol", art["bytecode"]["object"])

    # 3. Registry (needs identity address)
    # Constructor: address identity_
    identity_addr = addresses["NeunodeIdentity"]
    art = read_artifact("NeunodeRegistry.sol", "NeunodeRegistry")
    # ABI encode: just the address padded to 32 bytes
    ctor_arg = encode_address(identity_addr).rjust(32, b"\x00")
    deploy("NeunodeRegistry", "NeunodeRegistry.sol", art["bytecode"]["object"], ctor_arg)

    # 4. Bounty system (no constructor args, wired later)
    art = read_artifact("NeunodeBounty.sol", "NeunodeBounty")
    deploy("NeunodeBounty", "NeunodeBounty.sol", art["bytecode"]["object"])
    art = read_artifact("NeunodeEscrow.sol", "NeunodeEscrow")
    deploy("NeunodeEscrow", "NeunodeEscrow.sol", art["bytecode"]["object"])
    art = read_artifact("BountyReview.sol", "BountyReview")
    deploy("BountyReview", "BountyReview.sol", art["bytecode"]["object"])

    # 5. Royalty system
    art = read_artifact("ModelRegistry.sol", "ModelRegistry")
    deploy("ModelRegistry", "ModelRegistry.sol", art["bytecode"]["object"])
    model_addr = addresses["ModelRegistry"]
    art = read_artifact("RoyaltySplitter.sol", "RoyaltySplitter")
    ctor_arg = encode_address(model_addr).rjust(32, b"\x00")
    deploy("RoyaltySplitter", "RoyaltySplitter.sol", art["bytecode"]["object"], ctor_arg)

    # 6. Governance (needs compute token + params)
    compute_addr = addresses["nCompute"]
    art = read_artifact("NeunodeGovernance.sol", "NeunodeGovernance")
    ctor_arg = encode_address(compute_addr).rjust(32, b"\x00")
    ctor_arg += encode_uint256(GOV_PARAMS["voting_delay"])
    ctor_arg += encode_uint256(GOV_PARAMS["voting_period"])
    ctor_arg += encode_uint256(GOV_PARAMS["proposal_threshold"])
    ctor_arg += encode_uint256(GOV_PARAMS["quorum_bps"])
    ctor_arg += encode_uint256(GOV_PARAMS["timelock"])
    ctor_arg += encode_uint256(GOV_PARAMS["execution_window"])
    deploy("NeunodeGovernance", "NeunodeGovernance.sol", art["bytecode"]["object"], ctor_arg)

    # 7. StakingEscrow (needs token address)
    art = read_artifact("StakingEscrow.sol", "StakingEscrow")
    ctor_arg = encode_address(compute_addr).rjust(32, b"\x00")
    deploy("StakingEscrow", "StakingEscrow.sol", art["bytecode"]["object"], ctor_arg)

    # 8. Wire bounty <-> escrow + review
    print("Wiring contracts...")

    def send_tx(target, calldata):
        nonlocal nonce
        tx = {
            "from": deployer,
            "to": target,
            "data": calldata,
            "nonce": nonce,
            "gas": 1_000_000,
            "gasPrice": w3.eth.gas_price,
            "chainId": w3.eth.chain_id,
        }
        tx_hash = w3.eth.send_transaction(tx)
        receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
        nonce += 1
        return receipt.status == 1

    bounty_addr = addresses["NeunodeBounty"]
    escrow_addr = addresses["NeunodeEscrow"]
    review_addr = addresses["BountyReview"]

    # setEscrow(address)
    set_escrow_sel = Web3.keccak(text="setEscrow(address)")[:4].hex()
    set_escrow_data = "0x" + set_escrow_sel + escrow_addr[2:].lower().rjust(64, "0")
    if not send_tx(bounty_addr, set_escrow_data):
        raise RuntimeError("setEscrow failed")

    # setReviewContract(address)
    set_review_sel = Web3.keccak(text="setReviewContract(address)")[:4].hex()
    set_review_data = "0x" + set_review_sel + review_addr[2:].lower().rjust(64, "0")
    if not send_tx(bounty_addr, set_review_data):
        raise RuntimeError("setReviewContract failed")

    # registerBountyContract(address)
    reg_bounty_sel = Web3.keccak(text="registerBountyContract(address)")[:4].hex()
    reg_bounty_data = "0x" + reg_bounty_sel + bounty_addr[2:].lower().rjust(64, "0")
    if not send_tx(escrow_addr, reg_bounty_data):
        raise RuntimeError("registerBountyContract failed")

    # 9. Deploy Diamond proxy with Cut + Loupe facets
    print("Deploying Diamond proxy...")
    diamond_cut_art = read_artifact("DiamondCutFacet.sol", "DiamondCutFacet")
    deploy("DiamondCutFacet", "DiamondCutFacet.sol", diamond_cut_art["bytecode"]["object"])
    cut_addr = addresses["DiamondCutFacet"]

    diamond_loupe_art = read_artifact("DiamondLoupeFacet.sol", "DiamondLoupeFacet")
    deploy("DiamondLoupeFacet", "DiamondLoupeFacet.sol", diamond_loupe_art["bytecode"]["object"])
    loupe_addr = addresses["DiamondLoupeFacet"]

    # Deploy Diamond using forge create (handles complex ABI encoding)
    # Constructor: (FacetCut[] _diamondCut, address _init, bytes _calldata, address _owner)
    diamond_create_cmd = [
        "forge", "create",
        "src/diamond/Diamond.sol:Diamond",
        "--rpc-url", ANVIL_URL,
        "--private-key", DEPLOYER_PK,
        "--broadcast",
        "--constructor-args-path", "/dev/stdin",
    ]

    # Build the constructor args as a JSON array that forge can parse
    # FacetCut: (address facetAddress, uint8 action, bytes4[] functionSelectors)
    cut_selector = "0x1f931c1c"  # diamondCut selector
    loupe_selectors = [
        "0x7a0ed627",  # facets()
        "0xadfca15e",  # facetFunctionSelectors(address)
        "0x52ef6b2c",  # facetAddresses()
        "0xcdffacc6",  # facetAddress(bytes4)
    ]

    # Use cast to ABI-encode the constructor args
    # The constructor signature: ((address,uint8,bytes4[])[],address,bytes,address)
    encode_cmd = [
        "cast", "abi-encode",
        "f((address,uint8,bytes4[])[],address,bytes,address)",
        f"[({cut_addr},0,[{cut_selector}]),({loupe_addr},0,[{','.join(loupe_selectors)}])]",
        "0x0000000000000000000000000000000000000000",
        "0x",
        DEPLOYER,
    ]
    encoded_args = run(encode_cmd, cwd=CONTRACTS_DIR)

    # Deploy Diamond with the encoded constructor args
    diamond_art = read_artifact("Diamond.sol", "Diamond")
    diamond_bytecode = diamond_art["bytecode"]["object"]
    deploy("DiamondProxy", "Diamond.sol", diamond_bytecode, bytes.fromhex(encoded_args[2:]))

    return addresses


def keccak256(data):
    from eth_hash.auto import keccak
    return keccak(data)


def encode_uint256(val):
    return val.to_bytes(32, "big")


def encode_address_padded(addr_hex):
    """ABI-encode an address as a 32-byte left-padded value."""
    addr_bytes = bytes.fromhex(addr_hex[2:] if addr_hex.startswith("0x") else addr_hex)
    return addr_bytes.rjust(32, b"\x00")


def mapping_slot(key_bytes, base_slot):
    """Compute the storage slot for mapping[key] at base_slot."""
    return keccak256(key_bytes + encode_uint256(base_slot))


def nested_mapping_slot(key1_bytes, key2_bytes, base_slot):
    """Compute the storage slot for mapping[key1][key2] at base_slot."""
    inner_slot = mapping_slot(key1_bytes, base_slot)
    return keccak256(key2_bytes + inner_slot)


def read_contract_code(w3, addr):
    """Read deployed bytecode at an address."""
    code = w3.eth.get_code(addr)
    return "0x" + code.hex() if code else "0x"


def read_all_storage(w3, addr, contract_name, deployer_addr, address_map):
    """Read all non-zero storage slots by scanning + computing known mapping entries."""

    storage = {}

    # 1. Scan slots 0-150 (catches simple vars, arrays, etc.)
    for slot in range(150):
        val = w3.eth.get_storage_at(addr, slot)
        # Ensure exactly 32 bytes
        val_bytes = bytes(val)
        if len(val_bytes) > 32:
            val_bytes = val_bytes[:32]
        elif len(val_bytes) < 32:
            val_bytes = val_bytes.rjust(32, b"\x00")
        if int.from_bytes(val_bytes, "big") != 0:
            storage[f"0x{slot:064x}"] = "0x" + val_bytes.hex()

    # 2. Compute AccessControl role membership slots
    # For NeunodeToken-based contracts (tokens), _roles is at slot 6
    token_contracts = {"nCompute", "nTrain", "nBandwidth", "nStorage"}
    roles_base_slot = 6 if contract_name in token_contracts else None

    if roles_base_slot is not None:
        deployer_padded = encode_address_padded(deployer_addr)

        for role_name in ["", "MINTER_ROLE", "BURNER_ROLE", "GOVERNANCE_ROLE"]:
            role_hash = keccak256(role_name.encode()) if role_name else bytes(32)

            # RoleData base slot: keccak256(role_hash . roles_base_slot)
            role_base = mapping_slot(role_hash, roles_base_slot)

            # members[deployer] slot: keccak256(deployer . role_base)
            member_slot = nested_mapping_slot_at(deployer_padded, role_base)
            val = w3.eth.get_storage_at(addr, member_slot)
            val_bytes = bytes(val)[:32].rjust(32, b"\x00")
            if int.from_bytes(val_bytes, "big") != 0:
                storage[f"0x{member_slot.hex()}"] = "0x" + val_bytes.hex()

            # adminRole slot: role_base + 1
            admin_slot_int = int.from_bytes(role_base, "big") + 1
            admin_slot = admin_slot_int.to_bytes(32, "big")
            val = w3.eth.get_storage_at(addr, admin_slot)
            val_bytes = bytes(val)[:32].rjust(32, b"\x00")
            if int.from_bytes(val_bytes, "big") != 0:
                storage[f"0x{admin_slot.hex()}"] = "0x" + val_bytes.hex()

    # 3. For NeunodeBounty and DiamondProxy, check extended storage ranges
    if contract_name in ("NeunodeBounty", "DiamondProxy"):
        # Scan more slots for these contracts
        for slot in range(150, 300):
            val = w3.eth.get_storage_at(addr, slot)
            val_bytes = bytes(val)[:32].rjust(32, b"\x00")
            if int.from_bytes(val_bytes, "big") != 0:
                storage[f"0x{slot:064x}"] = "0x" + val_bytes.hex()

    # 4. For DiamondProxy, compute LibDiamond storage slots
    # LibDiamond uses STORAGE_POSITION = keccak256("diamond.neunode.storage")
    if contract_name == "DiamondProxy":
        lib_pos = keccak256(b"diamond.neunode.storage")
        base_slot_int = int.from_bytes(lib_pos, "big")

        # Owner at base_slot + 0
        for offset in range(10):
            slot = base_slot_int + offset
            slot_bytes = slot.to_bytes(32, "big")
            val = w3.eth.get_storage_at(addr, slot_bytes)
            val_bytes = bytes(val)[:32].rjust(32, b"\x00")
            if int.from_bytes(val_bytes, "big") != 0:
                storage[f"0x{slot_bytes.hex()}"] = "0x" + val_bytes.hex()

        # selectorToFacet mapping at base_slot + 1
        # For each known selector, compute its mapping slot
        selector_to_facet_base = base_slot_int + 1
        known_selectors = [
            bytes.fromhex("1f931c1c"),  # diamondCut
            bytes.fromhex("7a0ed627"),  # facets
            bytes.fromhex("adfca15e"),  # facetFunctionSelectors
            bytes.fromhex("52ef6b2c"),  # facetAddresses
            bytes.fromhex("cdffacc6"),  # facetAddress
        ]
        for sel in known_selectors:
            mapping_key = sel.rjust(32, b"\x00")
            slot = keccak256(mapping_key + selector_to_facet_base.to_bytes(32, "big"))
            val = w3.eth.get_storage_at(addr, slot)
            val_bytes = bytes(val)[:32].rjust(32, b"\x00")
            if int.from_bytes(val_bytes, "big") != 0:
                storage[f"0x{slot.hex()}"] = "0x" + val_bytes.hex()

        # facetFunctionSelectors mapping at base_slot + 2
        # facetAddresses array at base_slot + 3
        facet_addresses_base = base_slot_int + 3
        arr_len_val = w3.eth.get_storage_at(addr, facet_addresses_base)
        arr_len = int.from_bytes(bytes(arr_len_val)[:32].rjust(32, b"\x00"), "big")

        if arr_len > 0:
            # Array data starts at keccak256(facet_addresses_base)
            arr_data_slot = keccak256(facet_addresses_base.to_bytes(32, "big"))
            for i in range(arr_len):
                slot_int = int.from_bytes(arr_data_slot, "big") + i
                slot_bytes = slot_int.to_bytes(32, "big")
                val = w3.eth.get_storage_at(addr, slot_bytes)
                val_bytes = bytes(val)[:32].rjust(32, b"\x00")
                if int.from_bytes(val_bytes, "big") != 0:
                    storage[f"0x{slot_bytes.hex()}"] = "0x" + val_bytes.hex()

        # facetFunctionSelectors mapping (address => bytes4[])
        # For each known facet address, compute its slot
        ffs_base = base_slot_int + 2
        # We need the facet addresses from the deployed state
        for facet_name in ("DiamondCutFacet", "DiamondLoupeFacet"):
            if facet_name in address_map:
                facet_addr = address_map[facet_name]
                facet_padded = encode_address_padded(facet_addr)
                # mapping[facet_addr] array length
                ffs_len_slot = keccak256(facet_padded + ffs_base.to_bytes(32, "big"))
                ffs_len_val = w3.eth.get_storage_at(addr, ffs_len_slot)
                ffs_len = int.from_bytes(bytes(ffs_len_val)[:32].rjust(32, b"\x00"), "big")

                if ffs_len > 0:
                    # Store the length slot
                    storage[f"0x{ffs_len_slot.hex()}"] = "0x" + bytes(ffs_len_val)[:32].rjust(32, b"\x00").hex()
                    # Array data starts at keccak256(ffs_len_slot)
                    ffs_data_slot = keccak256(ffs_len_slot)
                    for i in range(ffs_len):
                        slot_int = int.from_bytes(ffs_data_slot, "big") + i
                        slot_bytes = slot_int.to_bytes(32, "big")
                        val = w3.eth.get_storage_at(addr, slot_bytes)
                        val_bytes = bytes(val)[:32].rjust(32, b"\x00")
                        if int.from_bytes(val_bytes, "big") != 0:
                            storage[f"0x{slot_bytes.hex()}"] = "0x" + val_bytes.hex()

    return storage


def nested_mapping_slot_at(key_padded, inner_slot_bytes):
    """Compute keccak256(key_padded ++ inner_slot_bytes) for nested mapping."""
    return keccak256(key_padded + inner_slot_bytes)


def extract_contract_state(w3, contract_addresses, deployer_addr):
    """Extract code + storage for each deployed contract."""
    contracts = {}

    for name, addr in contract_addresses.items():
        code = read_contract_code(w3, addr)
        if code == "0x":
            continue

        storage = read_all_storage(w3, addr, name, deployer_addr, contract_addresses)
        contracts[name] = {
            "code": code,
            "storage": storage,
        }

    return contracts


def patch_address_references(contracts, address_map):
    """Replace old deployed addresses with predeploy addresses in storage."""
    # Build remapping: deployed_addr -> predeploy_addr
    remap = {}
    for name, deployed_addr in address_map.items():
        if name in PREDEPLOY_ADDRESSES:
            predeploy = PREDEPLOY_ADDRESSES[name].lower()[2:]
            deployed = deployed_addr.lower()[2:]
            if deployed != predeploy:
                remap[deployed] = predeploy

    if not remap:
        return contracts

    patched_count = 0
    for name, data in contracts.items():
        code_clean = data["code"].lower().replace("0x", "")
        for deployed, predeploy in remap.items():
            occurrences = code_clean.count(deployed)
            if occurrences:
                code_clean = code_clean.replace(deployed, predeploy)
                patched_count += occurrences
        data["code"] = "0x" + code_clean

        patched_storage = {}
        for slot, value in data["storage"].items():
            value_clean = value.lower().replace("0x", "")
            # Search for any deployed address substring and replace it
            for deployed, predeploy in remap.items():
                if deployed in value_clean:
                    value_clean = value_clean.replace(deployed, predeploy)
                    patched_count += 1
            patched_storage[slot] = "0x" + value_clean
        data["storage"] = patched_storage

    print(f"  Patched {patched_count} address references")
    return contracts


def generate_genesis_predeploys(contracts):
    """Generate the final genesis predeploy JSON."""
    predeploys = []

    for name, predeploy_addr in PREDEPLOY_ADDRESSES.items():
        if name not in contracts:
            print(f"  WARNING: {name} not found in deployed state, using empty bytecode")
            predeploys.append({
                "name": name,
                "address": predeploy_addr,
                "bytecode": "0x",
                "storage": {},
            })
            continue

        data = contracts[name]
        predeploys.append({
            "name": name,
            "address": predeploy_addr,
            "bytecode": data["code"],
            "storage": data["storage"],
        })

    return {"predeploys": predeploys}


def main():
    os.chdir(REPO_ROOT)

    # Ensure contracts are compiled (skip if already built)
    out_dir = CONTRACTS_DIR / "out"
    if not out_dir.exists() or not (out_dir / "NeunodeBounty.sol").exists():
        print("Compiling contracts...")
        run(["forge", "build"], cwd=CONTRACTS_DIR)
    else:
        print("Contracts already compiled.")

    # Start Anvil
    print("Starting Anvil...")
    anvil_proc = start_anvil()

    try:
        w3 = Web3(Web3.HTTPProvider(ANVIL_URL))
        assert w3.is_connected(), "Failed to connect to Anvil"

        # Fund deployer (Anvil account 0 is already funded)
        balance = w3.eth.get_balance(DEPLOYER)
        print(f"Deployer balance: {balance / 1e18:.0f} ETH")

        # Deploy contracts
        addresses = deploy_contracts(w3)

        # Extract code + storage for each contract
        print("\nExtracting contract state...")
        contracts = extract_contract_state(w3, addresses, DEPLOYER)
        print(f"Extracted {len(contracts)} contracts from Anvil")
        for name in contracts:
            code_size = len(contracts[name]["code"]) // 2
            storage_count = len(contracts[name]["storage"])
            print(f"  {name}: {code_size}B code, {storage_count} storage slots")

        # Patch address references
        contracts = patch_address_references(contracts, addresses)

        # Generate genesis
        genesis = generate_genesis_predeploys(contracts)

        # Write output
        OUTPUT_FILE.parent.mkdir(parents=True, exist_ok=True)
        with open(OUTPUT_FILE, "w") as f:
            json.dump(genesis, f, indent=2)

        print(f"\nGenerated genesis predeploys: {OUTPUT_FILE}")
        print(f"Total contracts: {len(genesis['predeploys'])}")

    finally:
        anvil_proc.kill()
        print("Anvil stopped.")


if __name__ == "__main__":
    main()
