//! Predeployed contract addresses and bytecode for the Neunode genesis block.
//!
//! Contracts are deployed at deterministic addresses in the `0x0000...00XX` range.
//! Actual deployed bytecode comes from `forge build` in CI — placeholders are used here.

use alloy::primitives::{address, Address, Bytes};
use std::collections::BTreeMap;

/// Descriptor for a contract predeployed at genesis.
#[derive(Debug, Clone)]
pub struct PredeployedContract {
    /// Human-readable name for logging and tooling.
    pub name: &'static str,
    /// Deterministic address where the contract is deployed.
    pub address: Address,
    /// Deployed bytecode (from `forge build`). Placeholder until CI fills it.
    pub bytecode: Bytes,
    /// Constructor arguments appended to init code.
    pub constructor_args: Bytes,
    /// Pre-initialized storage slots (slot -> value).
    pub storage: BTreeMap<alloy::primitives::B256, alloy::primitives::B256>,
}

// Deterministic predeploy addresses.
// Using the pattern 0x0000_0000_0000_0000_0000_0000_0000_0000_0000_00XX.

/// Diamond proxy (EIP-2535) — entry point for all facets.
pub const DIAMOND_PROXY: Address = address!("0000000000000000000000000000000000000001");

/// NeunodeIdentity — DID registry for AI agents.
pub const NEUNODE_IDENTITY: Address = address!("0000000000000000000000000000000000000002");

/// NeunodeBounty — bounty state machine.
pub const NEUNODE_BOUNTY: Address = address!("0000000000000000000000000000000000000003");

/// NeunodeEscrow — bilateral escrow for bounty payments.
pub const NEUNODE_ESCROW: Address = address!("0000000000000000000000000000000000000004");

/// NeunodeRegistry — agent capability and endpoint registry.
pub const NEUNODE_REGISTRY: Address = address!("0000000000000000000000000000000000000005");

/// nCompute token — GPU/CPU hours resource token.
pub const COMPUTE_TOKEN: Address = address!("0000000000000000000000000000000000000006");

/// nTrain token — training compute resource token.
pub const TRAIN_TOKEN: Address = address!("0000000000000000000000000000000000000007");

/// nBandwidth token — bandwidth resource token.
pub const BANDWIDTH_TOKEN: Address = address!("0000000000000000000000000000000000000008");

/// nStorage token — storage resource token.
pub const STORAGE_TOKEN: Address = address!("0000000000000000000000000000000000000009");

/// ModelRegistry — model lineage DAG with content-addressed models.
pub const MODEL_REGISTRY: Address = address!("000000000000000000000000000000000000000a");

/// RoyaltySplitter — ERC-2981 royalty distribution with BFS lineage traversal.
pub const ROYALTY_SPLITTER: Address = address!("000000000000000000000000000000000000000b");

/// NeunodeGovernance — on-chain governance with staked token voting.
pub const NEUNODE_GOVERNANCE: Address = address!("000000000000000000000000000000000000000c");

/// StakingEscrow — inactivity decay manager.
pub const STAKING_ESCROW: Address = address!("000000000000000000000000000000000000000d");

/// BountyReview — 2-of-3 review committee for bounty submissions.
pub const BOUNTY_REVIEW: Address = address!("000000000000000000000000000000000000000e");

/// EIP-4788 beacon roots contract (system predeploy for Cancun).
pub const BEACON_ROOTS: Address = address!("000F3df6D732807Ef1319fB7B8bB8522d0Beac02");

/// Returns all Neunode-specific predeployed contracts with placeholder bytecode.
///
/// Bytecode must be filled from `forge build` artifacts before actual deployment.
/// Storage slots should be initialized for constructor-set values (e.g., owner, roles).
pub fn neunode_predeploys() -> Vec<PredeployedContract> {
    vec![
        // TODO: Replace empty bytecode with compiled artifacts from `forge build`.
        // Each contract's init code + constructor args must be in the `bytecode` field,
        // and constructor-initialized storage (owner, roles, immutable references)
        // must be in the `storage` map.
        PredeployedContract {
            name: "DiamondProxy",
            address: DIAMOND_PROXY,
            bytecode: Bytes::new(),
            constructor_args: Bytes::new(),
            storage: BTreeMap::new(),
        },
        PredeployedContract {
            name: "NeunodeIdentity",
            address: NEUNODE_IDENTITY,
            bytecode: Bytes::new(),
            constructor_args: Bytes::new(),
            storage: BTreeMap::new(),
        },
        PredeployedContract {
            name: "NeunodeBounty",
            address: NEUNODE_BOUNTY,
            bytecode: Bytes::new(),
            constructor_args: Bytes::new(),
            storage: BTreeMap::new(),
        },
        PredeployedContract {
            name: "NeunodeEscrow",
            address: NEUNODE_ESCROW,
            bytecode: Bytes::new(),
            constructor_args: Bytes::new(),
            storage: BTreeMap::new(),
        },
        PredeployedContract {
            name: "NeunodeRegistry",
            address: NEUNODE_REGISTRY,
            bytecode: Bytes::new(),
            // Constructor arg: identity contract address
            constructor_args: NEUNODE_IDENTITY.to_vec().into(),
            storage: BTreeMap::new(),
        },
        PredeployedContract {
            name: "nCompute",
            address: COMPUTE_TOKEN,
            bytecode: Bytes::new(),
            // Constructor args: name, symbol, decimals — encoded in init code
            constructor_args: Bytes::new(),
            storage: BTreeMap::new(),
        },
        PredeployedContract {
            name: "nTrain",
            address: TRAIN_TOKEN,
            bytecode: Bytes::new(),
            constructor_args: Bytes::new(),
            storage: BTreeMap::new(),
        },
        PredeployedContract {
            name: "nBandwidth",
            address: BANDWIDTH_TOKEN,
            bytecode: Bytes::new(),
            constructor_args: Bytes::new(),
            storage: BTreeMap::new(),
        },
        PredeployedContract {
            name: "nStorage",
            address: STORAGE_TOKEN,
            bytecode: Bytes::new(),
            constructor_args: Bytes::new(),
            storage: BTreeMap::new(),
        },
        PredeployedContract {
            name: "ModelRegistry",
            address: MODEL_REGISTRY,
            bytecode: Bytes::new(),
            constructor_args: Bytes::new(),
            storage: BTreeMap::new(),
        },
        PredeployedContract {
            name: "RoyaltySplitter",
            address: ROYALTY_SPLITTER,
            bytecode: Bytes::new(),
            // Constructor arg: ModelRegistry address
            constructor_args: MODEL_REGISTRY.to_vec().into(),
            storage: BTreeMap::new(),
        },
        PredeployedContract {
            name: "NeunodeGovernance",
            address: NEUNODE_GOVERNANCE,
            bytecode: Bytes::new(),
            // Constructor args: token address, voting params — encoded in init code
            constructor_args: Bytes::new(),
            storage: BTreeMap::new(),
        },
        PredeployedContract {
            name: "StakingEscrow",
            address: STAKING_ESCROW,
            bytecode: Bytes::new(),
            // Constructor arg: NeunodeToken address
            constructor_args: Bytes::new(),
            storage: BTreeMap::new(),
        },
        PredeployedContract {
            name: "BountyReview",
            address: BOUNTY_REVIEW,
            bytecode: Bytes::new(),
            constructor_args: Bytes::new(),
            storage: BTreeMap::new(),
        },
    ]
}

/// Returns all predeploy addresses as a slice, useful for quick lookups.
pub fn predeploy_addresses() -> Vec<Address> {
    neunode_predeploys().iter().map(|c| c.address).collect()
}
