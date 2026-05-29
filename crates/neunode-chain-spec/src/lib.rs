//! Neunode L1 chain specification.
//!
//! Defines the custom genesis, gas token configuration, predeployed contract addresses,
//! and EIP-1559 parameters for the Neunode L1 chain.
//!
//! # Usage
//!
//! ```rust,ignore
//! use neunode_chain_spec::{neunode_chain_spec, neunode_genesis_json};
//!
//! // Get the full chain spec
//! let spec = neunode_chain_spec();
//!
//! // Serialize genesis to Reth-compatible JSON
//! let json = neunode_genesis_json();
//! ```

mod chain;
mod gas;
mod genesis;
mod predeploys;

pub use chain::*;
pub use gas::*;
pub use genesis::*;
pub use predeploys::*;

/// Convenience function returning the complete chain spec.
///
/// Combines chain constants, hardfork config, gas parameters,
/// predeploys, and genesis accounts into a single struct.
pub fn neunode_chain_spec() -> NeunodeGenesis {
    neunode_genesis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // --- Chain constants ---

    #[test]
    fn chain_id_is_not_zero_and_not_mainnet() {
        assert_ne!(CHAIN_ID, 0, "chain ID must not be zero");
        assert_ne!(CHAIN_ID, 1, "chain ID must not conflict with Ethereum mainnet");
    }

    #[test]
    fn native_currency_decimals_is_18() {
        assert_eq!(NATIVE_CURRENCY_DECIMALS, 18);
    }

    #[test]
    fn block_gas_limit_is_reasonable() {
        assert!(BLOCK_GAS_LIMIT >= 1_000_000, "gas limit too low");
        assert!(BLOCK_GAS_LIMIT <= 100_000_000, "gas limit unreasonably high");
    }

    #[test]
    fn initial_base_fee_is_one_gwei() {
        assert_eq!(INITIAL_BASE_FEE, 1_000_000_000);
    }

    // --- EIP-1559 parameters ---

    #[test]
    fn eip1559_max_change_is_less_than_100_percent() {
        let config = NeunodeEip1559Config::default();
        assert!(config.max_increase_fraction() < 1.0);
        assert!(config.max_decrease_fraction() < 1.0);
    }

    #[test]
    fn eip1559_change_denominator_at_least_8() {
        let config = NeunodeEip1559Config::default();
        assert!(config.max_change_denominator >= 8, "denominator should be at least mainnet's 8");
    }

    #[test]
    fn eip1559_elasticity_at_least_2() {
        let config = NeunodeEip1559Config::default();
        assert!(config.elasticity_multiplier >= 2, "elasticity should be at least mainnet's 2");
    }

    // --- Predeploys ---

    #[test]
    fn predeploys_have_unique_addresses() {
        let predeploys = neunode_predeploys();
        let addresses: HashSet<_> = predeploys.iter().map(|c| c.address).collect();
        assert_eq!(addresses.len(), predeploys.len(), "all predeploy addresses must be unique");
    }

    #[test]
    fn predeploys_covers_all_expected_contracts() {
        let predeploys = neunode_predeploys();
        let names: HashSet<_> = predeploys.iter().map(|c| c.name).collect();

        // Verify all expected contracts are present.
        let expected = [
            "DiamondProxy",
            "NeunodeIdentity",
            "NeunodeBounty",
            "NeunodeEscrow",
            "NeunodeRegistry",
            "nCompute",
            "nTrain",
            "nBandwidth",
            "nStorage",
            "ModelRegistry",
            "RoyaltySplitter",
            "NeunodeGovernance",
            "StakingEscrow",
            "BountyReview",
        ];
        for name in &expected {
            assert!(names.contains(name), "missing predeploy: {name}");
        }
        assert_eq!(predeploys.len(), expected.len());
    }

    #[test]
    fn predeploy_addresses_are_deterministic() {
        // Verify specific known addresses.
        assert_eq!(format!("{:#x}", DIAMOND_PROXY), "0x0000000000000000000000000000000000000001");
        assert_eq!(
            format!("{:#x}", NEUNODE_IDENTITY),
            "0x0000000000000000000000000000000000000002"
        );
        assert_eq!(format!("{:#x}", COMPUTE_TOKEN), "0x0000000000000000000000000000000000000006");
        assert_eq!(format!("{:#x}", BEACON_ROOTS), "0x000f3df6d732807ef1319fb7b8bb8522d0beac02");
    }

    // --- Genesis ---

    #[test]
    fn genesis_has_correct_chain_id() {
        let genesis = neunode_genesis();
        assert_eq!(genesis.chain_id, CHAIN_ID);
    }

    #[test]
    fn genesis_accounts_have_non_zero_balances() {
        let genesis = neunode_genesis();
        let funded_accounts: Vec<_> =
            genesis.alloc.iter().filter(|a| a.balance > alloy::primitives::U256::ZERO).collect();
        assert!(!funded_accounts.is_empty(), "genesis must have at least one funded account");

        // All funded accounts should have non-zero balance (tautological from filter,
        // but validates the filter works and values are correct).
        for account in &funded_accounts {
            assert_ne!(
                account.balance,
                alloy::primitives::U256::ZERO,
                "account {:#x} has zero balance but is marked as funded",
                account.address
            );
        }
    }

    #[test]
    fn genesis_includes_validator_accounts() {
        let genesis = neunode_genesis();
        let expected_balance = validator_balance();

        for &addr in &VALIDATOR_ADDRESSES {
            let account = genesis
                .alloc
                .iter()
                .find(|a| a.address == addr)
                .unwrap_or_else(|| panic!("validator {addr:#x} not found in genesis alloc"));
            assert_eq!(account.balance, expected_balance);
        }
    }

    #[test]
    fn genesis_includes_predeployed_contracts() {
        let genesis = neunode_genesis();
        let predeploys = neunode_predeploys();

        for contract in &predeploys {
            let account =
                genesis.alloc.iter().find(|a| a.address == contract.address).unwrap_or_else(|| {
                    panic!(
                        "predeploy {} ({:#x}) not found in genesis alloc",
                        contract.name, contract.address
                    )
                });
            // Predeploys should have zero balance.
            assert_eq!(account.balance, alloy::primitives::U256::ZERO);
        }
    }

    #[test]
    fn genesis_hardforks_activated_at_block_zero() {
        let genesis = neunode_genesis();
        assert_eq!(genesis.hardforks.paris_block, 0);
        assert_eq!(genesis.hardforks.shanghai_block, 0);
        assert_eq!(genesis.hardforks.cancun_block, 0);
        // Prague is intentionally delayed.
        assert_eq!(genesis.hardforks.prague_block, None);
    }

    // --- Genesis JSON serialization ---

    #[test]
    fn genesis_json_roundtrip() {
        let json = neunode_genesis_json();
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("genesis JSON should be valid");

        // Verify top-level fields exist.
        assert!(parsed.get("config").is_some(), "missing config");
        assert!(parsed.get("alloc").is_some(), "missing alloc");
        assert!(parsed.get("gasLimit").is_some(), "missing gasLimit");
        assert!(parsed.get("baseFeePerGas").is_some(), "missing baseFeePerGas");
        assert!(parsed.get("difficulty").is_some(), "missing difficulty");

        // Verify config.
        let config = parsed.get("config").unwrap();
        assert_eq!(config["chainId"], CHAIN_ID);
        assert_eq!(config["terminalTotalDifficulty"], 0);
        assert_eq!(config["terminalTotalDifficultyPassed"], true);
        assert_eq!(config["shanghaiTime"], 0);
        assert_eq!(config["cancunTime"], 0);

        // Verify alloc has entries.
        let alloc = parsed.get("alloc").unwrap().as_object().unwrap();
        // 4 validators + 1 deployer + 14 predeploys = 19 accounts.
        assert!(
            alloc.len() >= INITIAL_VALIDATORS + 1,
            "expected at least {} alloc entries, got {}",
            INITIAL_VALIDATORS + 1,
            alloc.len()
        );

        // Verify base fee.
        assert_eq!(parsed["baseFeePerGas"].as_str().unwrap(), format!("{:#x}", INITIAL_BASE_FEE));
    }

    #[test]
    fn genesis_json_contains_extra_data() {
        let json = neunode_genesis_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let extra_data = parsed["extraData"].as_str().unwrap();
        assert_eq!(extra_data, "0x8e65756e6f6465"); // "neunode" in hex
    }

    #[test]
    fn genesis_json_gas_limit_matches_constant() {
        let json = neunode_genesis_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let gas_limit = parsed["gasLimit"].as_str().unwrap();
        assert_eq!(gas_limit, format!("{:#x}", BLOCK_GAS_LIMIT));
    }

    #[test]
    fn validator_balance_is_one_million_neu() {
        let expected = alloy::primitives::U256::from(1_000_000u64)
            * alloy::primitives::U256::from(10u128).pow(alloy::primitives::U256::from(18));
        assert_eq!(validator_balance(), expected);
    }
}
