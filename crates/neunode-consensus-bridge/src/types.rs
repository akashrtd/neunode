use alloy::primitives::{Address, B256, U256};

/// A validator in the consensus set.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidatorInfo {
    /// Ethereum address controlling this validator.
    pub address: Address,
    /// Voting power (proportional to reputation score in production).
    /// For single-node mode, this is always 1.
    pub voting_power: u64,
}

/// The full validator set at a given height.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidatorSet {
    pub validators: Vec<ValidatorInfo>,
    pub total_voting_power: u64,
}

impl ValidatorSet {
    /// Single-validator set for Phase 2 single-node mode.
    pub fn single(validator: Address) -> Self {
        Self {
            validators: vec![ValidatorInfo { address: validator, voting_power: 1 }],
            total_voting_power: 1,
        }
    }

    /// Build a validator set from the chain-spec genesis validators.
    /// Each validator gets equal voting power (1) in Phase 2.
    /// Phase 4 will replace this with reputation-weighted power.
    pub fn from_genesis() -> Self {
        let validators: Vec<ValidatorInfo> = neunode_chain_spec::VALIDATOR_ADDRESSES
            .iter()
            .map(|&addr| ValidatorInfo { address: addr, voting_power: 1 })
            .collect();
        let total = validators.len() as u64;
        Self { validators, total_voting_power: total }
    }
}

/// Result of producing a single block.
#[derive(Debug, Clone)]
pub struct BlockProduced {
    /// Hash of the newly produced block.
    pub block_hash: B256,
    /// Block number.
    pub block_number: u64,
    /// Block timestamp.
    pub timestamp: u64,
    /// Gas used by transactions in this block.
    pub gas_used: U256,
    /// Number of transactions included.
    pub tx_count: usize,
}

/// Current state of the bridge driver.
#[derive(Debug, Clone)]
pub struct BridgeState {
    /// Hash of the current head block.
    pub head_block_hash: B256,
    /// Hash of the current safe block.
    pub safe_block_hash: B256,
    /// Hash of the current finalized block.
    pub finalized_block_hash: B256,
    /// Current block height.
    pub head_block_number: u64,
}
