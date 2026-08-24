//! Single-node block production driver for Phase 2.
//!
//! This driver produces blocks by driving a Reth execution layer via the Engine API.
//! In single-node mode, there is no BFT consensus: every block is immediately finalized.
//! Phase 3 will replace this with a multi-validator Malachite consensus driver.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use alloy::primitives::{Address, B256, U256};
use neunode_engine_api_client::{
    EngineApiClient, EngineApiClientConfig, ForkchoiceState, PayloadAttributes, PayloadStatusEnum,
};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::error::{BridgeError, Result};
use crate::types::{BlockProduced, BridgeState, ValidatorSet};

/// Configuration for the single-node consensus driver.
#[derive(Debug, Clone)]
pub struct SingleNodeConfig {
    /// Engine API client configuration (endpoint, JWT, retries).
    pub engine_api: EngineApiClientConfig,
    /// Address that receives transaction fees (coinbase / fee recipient).
    pub fee_recipient: Address,
    /// Target block time in seconds.
    pub block_time_secs: u64,
    /// Genesis block hash (the hash of block 0 from the chain spec).
    pub genesis_hash: Option<B256>,
}

/// Single-node consensus driver that produces blocks via the Engine API.
///
/// Block production loop:
/// 1. `forkchoiceUpdated` with payload attributes -> trigger block building
/// 2. `getPayload` -> retrieve the built block
/// 3. `newPayload` -> submit for validation
/// 4. `forkchoiceUpdated` -> set as head + safe + finalized
///
/// In Phase 2, every produced block is immediately finalized (single validator).
pub struct SingleNodeDriver {
    client: EngineApiClient,
    config: SingleNodeConfig,
    state: Arc<RwLock<BridgeState>>,
    validator_set: ValidatorSet,
}

impl SingleNodeDriver {
    /// Create a new single-node driver.
    pub async fn new(config: SingleNodeConfig) -> Result<Self> {
        let client = EngineApiClient::new(config.engine_api.clone()).await?;
        let validator_set = ValidatorSet::from_genesis();

        let state = if let Some(genesis_hash) = config.genesis_hash {
            BridgeState {
                head_block_hash: genesis_hash,
                safe_block_hash: genesis_hash,
                finalized_block_hash: genesis_hash,
                head_block_number: 0,
            }
        } else {
            BridgeState {
                head_block_hash: B256::ZERO,
                safe_block_hash: B256::ZERO,
                finalized_block_hash: B256::ZERO,
                head_block_number: 0,
            }
        };

        Ok(Self { client, config, state: Arc::new(RwLock::new(state)), validator_set })
    }

    /// Initialize the chain by setting the genesis block as head/safe/finalized.
    ///
    /// Must be called once after Reth has imported the genesis block.
    pub async fn init_chain(&self, genesis_hash: B256) -> Result<()> {
        info!(genesis = %genesis_hash, "initializing chain");

        let fcu_state = ForkchoiceState {
            head_block_hash: genesis_hash,
            safe_block_hash: genesis_hash,
            finalized_block_hash: genesis_hash,
        };

        let response = self.client.forkchoice_updated_v3(fcu_state, None).await?;

        match response.payload_status.status {
            PayloadStatusEnum::Valid => {
                info!(genesis = %genesis_hash, "chain initialized");
                let mut state = self.state.write().await;
                state.head_block_hash = genesis_hash;
                state.safe_block_hash = genesis_hash;
                state.finalized_block_hash = genesis_hash;
                state.head_block_number = 0;
                Ok(())
            }
            ref status => {
                error!(?status, "forkchoiceUpdated rejected genesis");
                Err(BridgeError::BlockProduction(format!(
                    "genesis initialization rejected: {status:?}"
                )))
            }
        }
    }

    /// Produce a single block and return its hash.
    pub async fn produce_block(&self) -> Result<BlockProduced> {
        let state = self.state.read().await.clone();
        let timestamp = current_timestamp();
        let prev_randao = randomness(state.head_block_hash, timestamp);

        debug!(
            head = %state.head_block_hash,
            height = state.head_block_number,
            timestamp,
            "producing block"
        );

        // Step 1: Trigger block building.
        let fcu_state = ForkchoiceState {
            head_block_hash: state.head_block_hash,
            safe_block_hash: state.safe_block_hash,
            finalized_block_hash: state.finalized_block_hash,
        };

        let payload_attrs = PayloadAttributes {
            timestamp,
            prev_randao,
            suggested_fee_recipient: self.config.fee_recipient,
            withdrawals: Some(Vec::new()),
            parent_beacon_block_root: Some(state.head_block_hash),
        };

        let fcu_response =
            self.client.forkchoice_updated_v3(fcu_state, Some(payload_attrs)).await?;

        let payload_id = fcu_response.payload_id.ok_or_else(|| {
            BridgeError::BlockProduction(format!(
                "no payload ID returned (status: {:?})",
                fcu_response.payload_status.status
            ))
        })?;

        // Step 2: Retrieve the built payload.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let envelope = self.client.get_payload_v3(payload_id).await?;
        let block_hash = envelope.execution_payload.payload_inner.payload_inner.block_hash;
        let block_number = envelope.execution_payload.payload_inner.payload_inner.block_number;
        let gas_used = envelope.execution_payload.payload_inner.payload_inner.gas_used;
        let tx_count = envelope.execution_payload.payload_inner.payload_inner.transactions.len();

        debug!(block = %block_hash, number = block_number, "payload built");

        // Step 3: Submit as new payload.
        let payload_status = self
            .client
            .new_payload_v3(envelope.execution_payload.clone(), vec![], state.head_block_hash)
            .await?;

        match payload_status.status {
            PayloadStatusEnum::Valid => {}
            ref status => {
                error!(?status, block = %block_hash, "newPayload rejected block");
                return Err(BridgeError::BlockProduction(format!(
                    "block {block_hash} rejected by EL: {status:?}"
                )));
            }
        }

        // Step 4: Advance forkchoice (head = safe = finalized in single-node mode).
        let new_fcu = ForkchoiceState {
            head_block_hash: block_hash,
            safe_block_hash: block_hash,
            finalized_block_hash: block_hash,
        };

        let final_response = self.client.forkchoice_updated_v3(new_fcu, None).await?;

        match final_response.payload_status.status {
            PayloadStatusEnum::Valid => {
                info!(block = %block_hash, number = block_number, gas_used = %gas_used, "block finalized");

                let mut state = self.state.write().await;
                state.head_block_hash = block_hash;
                state.safe_block_hash = block_hash;
                state.finalized_block_hash = block_hash;
                state.head_block_number = block_number;

                Ok(BlockProduced {
                    block_hash,
                    block_number,
                    timestamp,
                    gas_used: U256::from(gas_used),
                    tx_count,
                })
            }
            ref status => {
                error!(?status, "forkchoiceUpdated rejected new head");
                Err(BridgeError::BlockProduction(format!(
                    "forkchoice update for block {block_hash} rejected: {status:?}"
                )))
            }
        }
    }

    /// Run the block production loop indefinitely.
    ///
    /// Produces a block every `block_time_secs` seconds.
    /// Errors are logged but do not stop the loop.
    pub async fn run_loop(&self) -> Result<()> {
        if self.state.read().await.head_block_hash == B256::ZERO {
            let genesis = self.client.get_block_by_number(0).await?;
            let hash = genesis
                .get("hash")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| BridgeError::BlockProduction("genesis block has no hash".into()))?
                .parse()
                .map_err(|error| {
                    BridgeError::BlockProduction(format!("invalid genesis block hash: {error}"))
                })?;
            self.init_chain(hash).await?;
        }

        let block_interval = std::time::Duration::from_secs(self.config.block_time_secs);
        info!(interval = ?block_interval, "starting block production loop");

        loop {
            if let Err(ref e) = self.produce_block().await {
                error!(error = %e, "block production failed, will retry next tick");
            }
            tokio::time::sleep(block_interval).await;
        }
    }

    /// Get the current bridge state.
    pub async fn state(&self) -> BridgeState {
        self.state.read().await.clone()
    }

    /// Get the validator set.
    pub fn validator_set(&self) -> &ValidatorSet {
        &self.validator_set
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn randomness(prev_hash: B256, timestamp: u64) -> B256 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    prev_hash.hash(&mut hasher);
    timestamp.hash(&mut hasher);
    let h = hasher.finish();
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&h.to_be_bytes());
    B256::from(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ValidatorSet;

    #[test]
    fn validator_set_single() {
        let addr = Address::ZERO;
        let set = ValidatorSet::single(addr);
        assert_eq!(set.validators.len(), 1);
        assert_eq!(set.total_voting_power, 1);
        assert_eq!(set.validators[0].address, addr);
    }

    #[test]
    fn validator_set_from_genesis() {
        let set = ValidatorSet::from_genesis();
        assert_eq!(set.validators.len(), neunode_chain_spec::INITIAL_VALIDATORS);
        assert_eq!(set.total_voting_power, neunode_chain_spec::INITIAL_VALIDATORS as u64);
    }

    #[test]
    fn randomness_changes_with_inputs() {
        let hash = B256::repeat_byte(0x01);
        let r1 = randomness(hash, 100);
        let r2 = randomness(hash, 200);
        let r3 = randomness(hash, 100);
        assert_ne!(r1, r2);
        assert_eq!(r1, r3);
    }

    #[test]
    fn current_timestamp_nonzero() {
        let ts = current_timestamp();
        assert!(ts > 1_700_000_000);
    }
}
