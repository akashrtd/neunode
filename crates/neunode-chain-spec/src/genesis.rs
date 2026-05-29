//! Genesis configuration for the Neunode L1 chain.
//!
//! Builds a Reth-compatible genesis JSON with all hardforks activated at block 0,
//! predeployed contracts in the alloc section, and pre-funded validator accounts.

use crate::chain::*;
use crate::gas::NeunodeEip1559Config;
use crate::predeploys::neunode_predeploys;
use alloy::hex;
use alloy::primitives::{address, Address, B256, U256};
use std::collections::BTreeMap;

/// Hardfork activation blocks/timestamps.
/// All hardforks are activated at genesis (block 0 / timestamp 0).
/// Prague is intentionally delayed per spike doc Appendix B.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HardforkConfig {
    /// Paris (PoS merge) activation block.
    pub paris_block: u64,
    /// Shanghai activation timestamp.
    pub shanghai_block: u64,
    /// Cancun activation timestamp.
    pub cancun_block: u64,
    /// Prague activation timestamp (0 = active at genesis, None = not activated).
    pub prague_block: Option<u64>,
}

/// A genesis account with balance and optional code/storage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenesisAccount {
    /// Account address.
    pub address: Address,
    /// Balance in wei.
    pub balance: U256,
    /// Account nonce.
    #[serde(default, skip_serializing_if = "is_default_nonce")]
    pub nonce: u64,
    /// Deployed bytecode (for predeployed contracts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Vec<u8>>,
    /// Pre-initialized storage slots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage: Option<BTreeMap<B256, B256>>,
}

fn is_default_nonce(nonce: &u64) -> bool {
    *nonce == 0
}

/// Complete Neunode genesis configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NeunodeGenesis {
    /// Chain ID.
    pub chain_id: u64,
    /// Genesis accounts (predeployed contracts + funded validators).
    pub alloc: Vec<GenesisAccount>,
    /// Hardfork activation schedule.
    pub hardforks: HardforkConfig,
    /// EIP-1559 gas parameters.
    pub gas_config: NeunodeEip1559Config,
}

/// Pre-funded validator balance: 1M NEU (1_000_000 * 10^18).
pub const VALIDATOR_BALANCE: u64 = 1_000_000;

/// Returns the 1M NEU balance as U256.
pub fn validator_balance() -> U256 {
    U256::from(VALIDATOR_BALANCE) * U256::from(10u128).pow(U256::from(18))
}

/// Placeholder validator addresses.
/// TODO: Replace with actual validator addresses before mainnet.
pub const VALIDATOR_ADDRESSES: [Address; INITIAL_VALIDATORS] = [
    address!("1000000000000000000000000000000000000000"),
    address!("2000000000000000000000000000000000000000"),
    address!("3000000000000000000000000000000000000000"),
    address!("4000000000000000000000000000000000000000"),
];

/// Placeholder deployer address (receives contract ownership).
/// TODO: Replace with actual deployer/multisig address.
pub const DEPLOYER_ADDRESS: Address = address!("dEaD000000000000000000000000000000000000");

/// Builds the complete Neunode genesis configuration.
pub fn neunode_genesis() -> NeunodeGenesis {
    let balance = validator_balance();
    let mut alloc = Vec::new();

    // Add funded validator accounts.
    for &addr in &VALIDATOR_ADDRESSES {
        alloc.push(GenesisAccount { address: addr, balance, nonce: 0, code: None, storage: None });
    }

    // Add deployer account with balance for initial transactions.
    alloc.push(GenesisAccount {
        address: DEPLOYER_ADDRESS,
        balance,
        nonce: 0,
        code: None,
        storage: None,
    });

    // Add predeployed contracts.
    // TODO: When compiled bytecode is available, fill code and storage fields.
    for contract in neunode_predeploys() {
        let storage =
            if contract.storage.is_empty() { None } else { Some(contract.storage.clone()) };
        alloc.push(GenesisAccount {
            address: contract.address,
            balance: U256::ZERO,
            nonce: 0,
            code: if contract.bytecode.is_empty() {
                None
            } else {
                Some(contract.bytecode.to_vec())
            },
            storage,
        });
    }

    NeunodeGenesis {
        chain_id: CHAIN_ID,
        alloc,
        hardforks: HardforkConfig::default(),
        gas_config: NeunodeEip1559Config::default(),
    }
}

/// Generates Reth-compatible genesis JSON.
///
/// This produces a standard geth/Reth genesis.json that can be loaded via
/// `reth node --chain genesis.json` or `reth init`.
pub fn neunode_genesis_json() -> String {
    let genesis = neunode_genesis();

    // Build the Reth-compatible alloc map (address -> account info).
    let mut alloc_map = serde_json::Map::new();
    for account in &genesis.alloc {
        let mut account_obj = serde_json::Map::new();
        account_obj
            .insert("balance".into(), serde_json::Value::String(format!("{:#x}", account.balance)));
        if let Some(ref code) = account.code {
            if !code.is_empty() {
                account_obj.insert(
                    "code".into(),
                    serde_json::Value::String(format!("0x{}", hex::encode(code))),
                );
            }
        }
        if let Some(ref storage) = account.storage {
            if !storage.is_empty() {
                let mut storage_obj = serde_json::Map::new();
                for (slot, value) in storage {
                    storage_obj.insert(
                        format!("{slot:#x}"),
                        serde_json::Value::String(format!("{value:#x}")),
                    );
                }
                account_obj.insert("storage".into(), serde_json::Value::Object(storage_obj));
            }
        }
        alloc_map.insert(format!("{:#x}", account.address), serde_json::Value::Object(account_obj));
    }

    // Build the full genesis object matching Reth/geth format.
    let mut genesis_json = serde_json::Map::new();

    // config section
    let mut config = serde_json::Map::new();
    config.insert("chainId".into(), serde_json::Value::Number(serde_json::Number::from(CHAIN_ID)));
    config.insert("homesteadBlock".into(), serde_json::Value::Number(serde_json::Number::from(0)));
    config.insert("eip150Block".into(), serde_json::Value::Number(serde_json::Number::from(0)));
    config.insert("eip155Block".into(), serde_json::Value::Number(serde_json::Number::from(0)));
    config.insert("eip158Block".into(), serde_json::Value::Number(serde_json::Number::from(0)));
    config.insert("byzantiumBlock".into(), serde_json::Value::Number(serde_json::Number::from(0)));
    config.insert(
        "constantinopleBlock".into(),
        serde_json::Value::Number(serde_json::Number::from(0)),
    );
    config.insert("petersburgBlock".into(), serde_json::Value::Number(serde_json::Number::from(0)));
    config.insert("istanbulBlock".into(), serde_json::Value::Number(serde_json::Number::from(0)));
    config.insert("berlinBlock".into(), serde_json::Value::Number(serde_json::Number::from(0)));
    config.insert("londonBlock".into(), serde_json::Value::Number(serde_json::Number::from(0)));
    config.insert("shanghaiTime".into(), serde_json::Value::Number(serde_json::Number::from(0)));
    config.insert("cancunTime".into(), serde_json::Value::Number(serde_json::Number::from(0)));
    config.insert(
        "terminalTotalDifficulty".into(),
        serde_json::Value::Number(serde_json::Number::from(0)),
    );
    config.insert("terminalTotalDifficultyPassed".into(), serde_json::Value::Bool(true));

    // Blob schedule (Cancun + Prague defaults)
    let mut blob_schedule = serde_json::Map::new();
    let mut cancun_blobs = serde_json::Map::new();
    cancun_blobs.insert("target".into(), serde_json::Value::Number(serde_json::Number::from(3)));
    cancun_blobs.insert("max".into(), serde_json::Value::Number(serde_json::Number::from(6)));
    cancun_blobs.insert(
        "baseFeeUpdateFraction".into(),
        serde_json::Value::Number(serde_json::Number::from(3_338_477)),
    );
    blob_schedule.insert("cancun".into(), serde_json::Value::Object(cancun_blobs));

    if let Some(_prague_block) = genesis.hardforks.prague_block {
        let mut prague_blobs = serde_json::Map::new();
        prague_blobs
            .insert("target".into(), serde_json::Value::Number(serde_json::Number::from(6)));
        prague_blobs.insert("max".into(), serde_json::Value::Number(serde_json::Number::from(9)));
        prague_blobs.insert(
            "baseFeeUpdateFraction".into(),
            serde_json::Value::Number(serde_json::Number::from(5_007_716)),
        );
        blob_schedule.insert("prague".into(), serde_json::Value::Object(prague_blobs));
    }

    config.insert("blobSchedule".into(), serde_json::Value::Object(blob_schedule));
    genesis_json.insert("config".into(), serde_json::Value::Object(config));

    // Genesis block fields
    genesis_json.insert("nonce".into(), serde_json::Value::String("0x0".into()));
    genesis_json.insert("timestamp".into(), serde_json::Value::String("0x0".into()));
    genesis_json.insert(
        "extraData".into(),
        serde_json::Value::String(format!("0x{}", hex::encode(GENESIS_EXTRA_DATA))),
    );
    genesis_json
        .insert("gasLimit".into(), serde_json::Value::String(format!("{:#x}", BLOCK_GAS_LIMIT)));
    genesis_json.insert("difficulty".into(), serde_json::Value::String("0x0".into()));
    genesis_json.insert("mixHash".into(), serde_json::Value::String(format!("{:#x}", B256::ZERO)));
    genesis_json
        .insert("coinbase".into(), serde_json::Value::String(format!("{:#x}", Address::ZERO)));
    genesis_json.insert("alloc".into(), serde_json::Value::Object(alloc_map));
    genesis_json.insert("number".into(), serde_json::Value::String("0x0".into()));
    genesis_json.insert("gasUsed".into(), serde_json::Value::String("0x0".into()));
    genesis_json
        .insert("parentHash".into(), serde_json::Value::String(format!("{:#x}", B256::ZERO)));
    genesis_json.insert(
        "baseFeePerGas".into(),
        serde_json::Value::String(format!("{:#x}", INITIAL_BASE_FEE)),
    );

    serde_json::to_string_pretty(&serde_json::Value::Object(genesis_json))
        .expect("genesis JSON serialization is infallible")
}
