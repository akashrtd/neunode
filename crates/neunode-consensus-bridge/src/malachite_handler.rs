//! Malachite app-channel to Engine API state machine with crash-safe persistence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use alloy::primitives::{Address, B256};
use async_trait::async_trait;
use neunode_engine_api_client::{
    EngineApiClient, ExecutionPayloadEnvelopeV3, ExecutionPayloadV3, ForkchoiceState,
    ForkchoiceUpdated, PayloadAttributes, PayloadStatus, PayloadStatusEnum,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::{BridgeError, Result};

/// Responses returned to the Malachite channel adapter.
#[derive(Debug)]
pub enum MalachiteResponse {
    Ready { height: u64 },
    Proposal(Vec<u8>),
    ProposalReceived(Option<Vec<u8>>),
    Validity(bool),
    Finalized { height: u64, block_hash: B256 },
}

/// Core Malachite channel messages. The runtime adapter translates pinned
/// `AppMsg` values to these stable, testable messages.
#[derive(Debug)]
pub enum MalachiteEvent {
    ConsensusReady {
        reply: oneshot::Sender<Result<MalachiteResponse>>,
    },
    StartedRound {
        height: u64,
        round: i64,
        proposer: String,
    },
    GetValue {
        height: u64,
        round: i64,
        reply: oneshot::Sender<Result<MalachiteResponse>>,
    },
    ReceivedProposalPart {
        height: u64,
        round: i64,
        bytes: Vec<u8>,
        finished: bool,
        reply: oneshot::Sender<Result<MalachiteResponse>>,
    },
    ValidationRequest {
        height: u64,
        round: i64,
        proposal: Vec<u8>,
        reply: oneshot::Sender<Result<MalachiteResponse>>,
    },
    Decided {
        height: u64,
        round: i64,
        certificate: Vec<u8>,
        reply: oneshot::Sender<Result<MalachiteResponse>>,
    },
}

#[async_trait]
pub trait ExecutionEngine: Send + Sync {
    async fn block(&self, tag: &str) -> std::result::Result<serde_json::Value, BridgeError>;
    async fn forkchoice(
        &self,
        state: ForkchoiceState,
        attributes: Option<PayloadAttributes>,
    ) -> std::result::Result<ForkchoiceUpdated, BridgeError>;
    async fn payload(
        &self,
        id: alloy::rpc::types::engine::PayloadId,
    ) -> std::result::Result<ExecutionPayloadEnvelopeV3, BridgeError>;
    async fn validate(
        &self,
        payload: ExecutionPayloadV3,
        parent_root: B256,
    ) -> std::result::Result<PayloadStatus, BridgeError>;
}

#[async_trait]
impl ExecutionEngine for EngineApiClient {
    async fn block(&self, tag: &str) -> std::result::Result<serde_json::Value, BridgeError> {
        Ok(self.get_block_by_tag(tag).await?)
    }

    async fn forkchoice(
        &self,
        state: ForkchoiceState,
        attributes: Option<PayloadAttributes>,
    ) -> std::result::Result<ForkchoiceUpdated, BridgeError> {
        Ok(self.forkchoice_updated_v3(state, attributes).await?)
    }

    async fn payload(
        &self,
        id: alloy::rpc::types::engine::PayloadId,
    ) -> std::result::Result<ExecutionPayloadEnvelopeV3, BridgeError> {
        Ok(self.get_payload_v3(id).await?)
    }

    async fn validate(
        &self,
        payload: ExecutionPayloadV3,
        parent_root: B256,
    ) -> std::result::Result<PayloadStatus, BridgeError> {
        Ok(self.new_payload_v3(payload, Vec::new(), parent_root).await?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalState {
    height: u64,
    round: i64,
    proposer: Option<String>,
    head: B256,
    safe: B256,
    finalized: B256,
    proposals: BTreeMap<String, ExecutionPayloadV3>,
    validated: BTreeSet<String>,
    last_certificate: Option<Vec<u8>>,
}

impl Default for WalState {
    fn default() -> Self {
        Self {
            height: 1,
            round: 0,
            proposer: None,
            head: B256::ZERO,
            safe: B256::ZERO,
            finalized: B256::ZERO,
            proposals: BTreeMap::new(),
            validated: BTreeSet::new(),
            last_certificate: None,
        }
    }
}

/// Stateful handler for the pinned Malachite Channels API.
pub struct MalachiteHandler<E = EngineApiClient> {
    engine: E,
    fee_recipient: Address,
    wal_path: PathBuf,
    state: WalState,
    partials: BTreeMap<String, Vec<u8>>,
}

impl<E: ExecutionEngine> MalachiteHandler<E> {
    pub fn open(engine: E, fee_recipient: Address, wal_path: impl Into<PathBuf>) -> Result<Self> {
        let wal_path = wal_path.into();
        let state = load_wal(&wal_path)?;
        Ok(Self { engine, fee_recipient, wal_path, state, partials: BTreeMap::new() })
    }

    pub async fn run(mut self, mut receiver: mpsc::Receiver<MalachiteEvent>) -> Result<()> {
        while let Some(event) = receiver.recv().await {
            match event {
                MalachiteEvent::ConsensusReady { reply } => {
                    let _ = reply.send(self.consensus_ready().await);
                }
                MalachiteEvent::StartedRound { height, round, proposer } => {
                    self.state.height = height;
                    self.state.round = round;
                    self.state.proposer = Some(proposer);
                    self.persist()?;
                }
                MalachiteEvent::GetValue { height, round, reply } => {
                    let _ = reply.send(self.get_value(height, round).await);
                }
                MalachiteEvent::ReceivedProposalPart { height, round, bytes, finished, reply } => {
                    let _ = reply.send(self.receive_part(height, round, bytes, finished));
                }
                MalachiteEvent::ValidationRequest { height, round, proposal, reply } => {
                    let _ = reply.send(self.validate(height, round, proposal).await);
                }
                MalachiteEvent::Decided { height, round, certificate, reply } => {
                    let _ = reply.send(self.decide(height, round, certificate).await);
                }
            }
        }
        Err(BridgeError::Stopped)
    }

    async fn consensus_ready(&mut self) -> Result<MalachiteResponse> {
        let latest = self.engine.block("latest").await?;
        let hash = parse_hash(&latest, "hash")?;
        let number = parse_quantity(&latest, "number")?;
        if self.state.head == B256::ZERO || number >= self.state.height.saturating_sub(1) {
            self.state.head = hash;
            self.state.safe = hash;
            self.state.finalized = hash;
            self.state.height = number + 1;
            self.persist()?;
        }
        Ok(MalachiteResponse::Ready { height: self.state.height })
    }

    async fn get_value(&mut self, height: u64, round: i64) -> Result<MalachiteResponse> {
        self.require_position(height, round)?;
        let forkchoice = self.forkchoice_state();
        let attributes = PayloadAttributes {
            timestamp: now_secs(),
            prev_randao: randomness(self.state.head, height, round),
            suggested_fee_recipient: self.fee_recipient,
            withdrawals: Some(Vec::new()),
            parent_beacon_block_root: Some(self.state.head),
        };
        let update = self.engine.forkchoice(forkchoice, Some(attributes)).await?;
        require_valid(&update.payload_status)?;
        let id = update.payload_id.ok_or_else(|| {
            BridgeError::InvalidProposal("Reth did not return a payload ID".into())
        })?;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let envelope = self.engine.payload(id).await?;
        let proposal = envelope.execution_payload;
        let bytes = serde_json::to_vec(&proposal)
            .map_err(|error| BridgeError::InvalidProposal(error.to_string()))?;
        self.state.proposals.insert(key(height, round), proposal);
        self.persist()?;
        Ok(MalachiteResponse::Proposal(bytes))
    }

    fn receive_part(
        &mut self,
        height: u64,
        round: i64,
        bytes: Vec<u8>,
        finished: bool,
    ) -> Result<MalachiteResponse> {
        self.require_position(height, round)?;
        let key = key(height, round);
        self.partials.entry(key.clone()).or_default().extend(bytes);
        if !finished {
            return Ok(MalachiteResponse::ProposalReceived(None));
        }
        let bytes = self.partials.remove(&key).unwrap_or_default();
        let payload = decode_payload(&bytes)?;
        self.state.proposals.insert(key, payload);
        self.persist()?;
        Ok(MalachiteResponse::ProposalReceived(Some(bytes)))
    }

    async fn validate(
        &mut self,
        height: u64,
        round: i64,
        proposal: Vec<u8>,
    ) -> Result<MalachiteResponse> {
        self.require_position(height, round)?;
        let payload = decode_payload(&proposal)?;
        let parent = payload.payload_inner.payload_inner.parent_hash;
        if parent != self.state.head {
            return Ok(MalachiteResponse::Validity(false));
        }
        let status = self.engine.validate(payload.clone(), parent).await?;
        if require_valid(&status).is_err() {
            return Ok(MalachiteResponse::Validity(false));
        }
        let key = key(height, round);
        self.state.proposals.insert(key.clone(), payload);
        self.state.validated.insert(key);
        self.persist()?;
        Ok(MalachiteResponse::Validity(true))
    }

    async fn decide(
        &mut self,
        height: u64,
        round: i64,
        certificate: Vec<u8>,
    ) -> Result<MalachiteResponse> {
        self.require_position(height, round)?;
        let key = key(height, round);
        let payload =
            self.state.proposals.get(&key).cloned().ok_or_else(|| {
                BridgeError::InvalidProposal("decided proposal is not cached".into())
            })?;
        let parent = payload.payload_inner.payload_inner.parent_hash;
        if !self.state.validated.contains(&key) {
            require_valid(&self.engine.validate(payload.clone(), parent).await?)?;
        }
        let hash = payload.payload_inner.payload_inner.block_hash;
        let finalized = ForkchoiceState {
            head_block_hash: hash,
            safe_block_hash: hash,
            finalized_block_hash: hash,
        };
        require_valid(&self.engine.forkchoice(finalized, None).await?.payload_status)?;
        self.state.head = hash;
        self.state.safe = hash;
        self.state.finalized = hash;
        self.state.height = height + 1;
        self.state.round = 0;
        self.state.proposer = None;
        self.state.last_certificate = Some(certificate);
        self.state.proposals.retain(|proposal_key, _| proposal_key > &key);
        self.state.validated.retain(|proposal_key| proposal_key > &key);
        self.persist()?;
        Ok(MalachiteResponse::Finalized { height, block_hash: hash })
    }

    fn require_position(&self, height: u64, round: i64) -> Result<()> {
        if height != self.state.height || round < self.state.round {
            return Err(BridgeError::InvalidProposal(format!(
                "stale consensus position {height}/{round}; current is {}/{}",
                self.state.height, self.state.round
            )));
        }
        Ok(())
    }

    fn forkchoice_state(&self) -> ForkchoiceState {
        ForkchoiceState {
            head_block_hash: self.state.head,
            safe_block_hash: self.state.safe,
            finalized_block_hash: self.state.finalized,
        }
    }

    fn persist(&self) -> Result<()> {
        persist_wal(&self.wal_path, &self.state)
    }
}

fn load_wal(path: &Path) -> Result<WalState> {
    if !path.exists() {
        return Ok(WalState::default());
    }
    let bytes = std::fs::read(path).map_err(|error| BridgeError::Wal(error.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|error| BridgeError::Wal(error.to_string()))
}

fn persist_wal(path: &Path, state: &WalState) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| BridgeError::Wal(error.to_string()))?;
    }
    let temp = path.with_extension("wal.tmp");
    let bytes = serde_json::to_vec(state).map_err(|error| BridgeError::Wal(error.to_string()))?;
    std::fs::write(&temp, bytes).map_err(|error| BridgeError::Wal(error.to_string()))?;
    std::fs::rename(&temp, path).map_err(|error| BridgeError::Wal(error.to_string()))
}

fn decode_payload(bytes: &[u8]) -> Result<ExecutionPayloadV3> {
    serde_json::from_slice(bytes).map_err(|error| BridgeError::InvalidProposal(error.to_string()))
}

fn key(height: u64, round: i64) -> String {
    format!("{height:020}/{round:020}")
}

fn require_valid(status: &PayloadStatus) -> Result<()> {
    if matches!(status.status, PayloadStatusEnum::Valid) {
        Ok(())
    } else {
        Err(BridgeError::InvalidProposal(format!(
            "execution layer rejected payload: {:?}",
            status.status
        )))
    }
}

fn parse_hash(value: &serde_json::Value, field: &str) -> Result<B256> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| BridgeError::InvalidProposal(format!("block has no {field}")))?
        .parse()
        .map_err(|error| BridgeError::InvalidProposal(format!("invalid {field}: {error}")))
}

fn parse_quantity(value: &serde_json::Value, field: &str) -> Result<u64> {
    let raw = value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| BridgeError::InvalidProposal(format!("block has no {field}")))?;
    u64::from_str_radix(raw.trim_start_matches("0x"), 16)
        .map_err(|error| BridgeError::InvalidProposal(format!("invalid {field}: {error}")))
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn randomness(head: B256, height: u64, round: i64) -> B256 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    head.hash(&mut hasher);
    height.hash(&mut hasher);
    round.hash(&mut hasher);
    let mut bytes = [0_u8; 32];
    bytes[..8].copy_from_slice(&hasher.finish().to_be_bytes());
    B256::from(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wal_round_trip_preserves_consensus_position() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bridge.wal");
        let state = WalState {
            height: 42,
            round: 3,
            proposer: Some("validator-2".into()),
            head: B256::repeat_byte(1),
            safe: B256::repeat_byte(2),
            finalized: B256::repeat_byte(3),
            last_certificate: Some(vec![4, 5, 6]),
            ..WalState::default()
        };
        persist_wal(&path, &state).unwrap();
        let restored = load_wal(&path).unwrap();
        assert_eq!(restored.height, 42);
        assert_eq!(restored.round, 3);
        assert_eq!(restored.proposer.as_deref(), Some("validator-2"));
        assert_eq!(restored.finalized, B256::repeat_byte(3));
        assert_eq!(restored.last_certificate, Some(vec![4, 5, 6]));
    }

    #[test]
    fn partial_proposal_is_reassembled_and_persisted() {
        let payload: ExecutionPayloadV3 = serde_json::from_value(payload_fixture()).unwrap();
        let bytes = serde_json::to_vec(&payload).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bridge.wal");
        let engine = MockEngine;
        let mut handler = MalachiteHandler::open(engine, Address::ZERO, &path).unwrap();
        let split = bytes.len() / 2;
        assert!(matches!(
            handler.receive_part(1, 0, bytes[..split].to_vec(), false).unwrap(),
            MalachiteResponse::ProposalReceived(None)
        ));
        assert!(matches!(
            handler.receive_part(1, 0, bytes[split..].to_vec(), true).unwrap(),
            MalachiteResponse::ProposalReceived(Some(_))
        ));
        let restored = load_wal(&path).unwrap();
        assert!(restored.proposals.contains_key(&key(1, 0)));
    }

    struct MockEngine;

    #[async_trait]
    impl ExecutionEngine for MockEngine {
        async fn block(&self, _: &str) -> std::result::Result<serde_json::Value, BridgeError> {
            unreachable!()
        }
        async fn forkchoice(
            &self,
            _: ForkchoiceState,
            _: Option<PayloadAttributes>,
        ) -> std::result::Result<ForkchoiceUpdated, BridgeError> {
            unreachable!()
        }
        async fn payload(
            &self,
            _: alloy::rpc::types::engine::PayloadId,
        ) -> std::result::Result<ExecutionPayloadEnvelopeV3, BridgeError> {
            unreachable!()
        }
        async fn validate(
            &self,
            _: ExecutionPayloadV3,
            _: B256,
        ) -> std::result::Result<PayloadStatus, BridgeError> {
            unreachable!()
        }
    }

    fn payload_fixture() -> serde_json::Value {
        serde_json::json!({
            "parentHash": format!("{:#x}", B256::ZERO),
            "feeRecipient": format!("{:#x}", Address::ZERO),
            "stateRoot": format!("{:#x}", B256::ZERO),
            "receiptsRoot": format!("{:#x}", B256::ZERO),
            "logsBloom": format!("0x{}", "00".repeat(256)),
            "prevRandao": format!("{:#x}", B256::ZERO),
            "blockNumber": "0x1", "gasLimit": "0x1c9c380", "gasUsed": "0x0",
            "timestamp": "0x1", "extraData": "0x", "baseFeePerGas": "0x1",
            "blockHash": format!("{:#x}", B256::repeat_byte(1)),
            "transactions": [], "withdrawals": [], "blobGasUsed": "0x0",
            "excessBlobGas": "0x0"
        })
    }
}
