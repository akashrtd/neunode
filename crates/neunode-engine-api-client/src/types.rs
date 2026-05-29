//! Engine API request/response types.
//!
//! Re-exports types from `alloy::rpc::types::engine` and defines
//! supplemental types specific to this client.

use alloy::primitives::U256;

// Re-export alloy engine types for convenience.
pub use alloy::rpc::types::engine::{
    BlobAndProofV1, BlobsBundleV1, ExecutionPayloadBodyV1, ExecutionPayloadEnvelopeV3,
    ExecutionPayloadV3, ForkchoiceState, ForkchoiceUpdated, PayloadAttributes, PayloadId,
    PayloadStatus, PayloadStatusEnum, TransitionConfiguration,
};

/// Result of a full block proposal flow.
#[derive(Debug, Clone)]
pub struct ProposeBlockResult {
    pub payload: ExecutionPayloadV3,
    pub block_value: U256,
    pub blobs_bundle: BlobsBundleV1,
    pub should_override_builder: bool,
}

impl From<ExecutionPayloadEnvelopeV3> for ProposeBlockResult {
    fn from(resp: ExecutionPayloadEnvelopeV3) -> Self {
        Self {
            payload: resp.execution_payload,
            block_value: resp.block_value,
            blobs_bundle: resp.blobs_bundle,
            should_override_builder: resp.should_override_builder,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{B256, B64};

    #[test]
    fn forkchoice_state_roundtrip() {
        let state = ForkchoiceState {
            head_block_hash: B256::repeat_byte(0x01),
            safe_block_hash: B256::repeat_byte(0x02),
            finalized_block_hash: B256::repeat_byte(0x03),
        };
        let json = serde_json::to_value(&state).unwrap();
        let back: ForkchoiceState = serde_json::from_value(json).unwrap();
        assert_eq!(back.head_block_hash, state.head_block_hash);
        assert_eq!(back.safe_block_hash, state.safe_block_hash);
        assert_eq!(back.finalized_block_hash, state.finalized_block_hash);
    }

    #[test]
    fn forkchoice_updated_deserialize() {
        let json = serde_json::json!({
            "payloadStatus": {
                "status": "VALID",
                "latestValidHash": "0x0000000000000000000000000000000000000000000000000000000000000001"
            },
            "payloadId": "0x0000000000000000"
        });
        let resp: ForkchoiceUpdated = serde_json::from_value(json).unwrap();
        assert!(resp.payload_id.is_some());
    }

    #[test]
    fn forkchoice_updated_null_payload_id() {
        let json = serde_json::json!({
            "payloadStatus": {
                "status": "SYNCING",
                "latestValidHash": null
            },
            "payloadId": null
        });
        let resp: ForkchoiceUpdated = serde_json::from_value(json).unwrap();
        assert!(resp.payload_id.is_none());
    }

    #[test]
    fn payload_status_valid() {
        let json = serde_json::json!({
            "status": "VALID",
            "latestValidHash": "0x0000000000000000000000000000000000000000000000000000000000000001"
        });
        let status: PayloadStatus = serde_json::from_value(json).unwrap();
        assert!(matches!(status.status, PayloadStatusEnum::Valid));
        assert!(status.latest_valid_hash.is_some());
    }

    #[test]
    fn payload_status_invalid() {
        let json = serde_json::json!({
            "status": "INVALID",
            "latestValidHash": null,
            "validationError": "bad block"
        });
        let status: PayloadStatus = serde_json::from_value(json).unwrap();
        match status.status {
            PayloadStatusEnum::Invalid { validation_error } => {
                assert_eq!(validation_error, "bad block");
            }
            _ => panic!("expected Invalid status"),
        }
    }

    #[test]
    fn payload_status_syncing() {
        let json = serde_json::json!({
            "status": "SYNCING",
            "latestValidHash": null
        });
        let status: PayloadStatus = serde_json::from_value(json).unwrap();
        assert!(matches!(status.status, PayloadStatusEnum::Syncing));
    }

    #[test]
    fn payload_attributes_v3() {
        let json = serde_json::json!({
            "timestamp": "0x1000",
            "prevRandao": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "suggestedFeeRecipient": "0x0000000000000000000000000000000000000000",
            "withdrawals": [],
            "parentBeaconBlockRoot": "0x0000000000000000000000000000000000000000000000000000000000000000"
        });
        let attrs: PayloadAttributes = serde_json::from_value(json).unwrap();
        assert_eq!(attrs.timestamp, 0x1000);
        assert!(attrs.withdrawals.is_some());
        assert!(attrs.parent_beacon_block_root.is_some());
    }

    #[test]
    fn execution_payload_envelope_v3_deserialize() {
        let json = serde_json::json!({
            "executionPayload": {
                "parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "feeRecipient": "0x0000000000000000000000000000000000000000",
                "stateRoot": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "receiptsRoot": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                "prevRandao": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "blockNumber": "0x1",
                "gasLimit": "0x1c9c380",
                "gasUsed": "0x0",
                "timestamp": "0x1000",
                "extraData": "0x",
                "baseFeePerGas": "0x3b9aca00",
                "blockHash": "0x0000000000000000000000000000000000000000000000000000000000000001",
                "transactions": [],
                "withdrawals": [],
                "blobGasUsed": "0x0",
                "excessBlobGas": "0x0"
            },
            "blockValue": "0x0",
            "blobsBundle": {
                "commitments": [],
                "proofs": [],
                "blobs": []
            },
            "shouldOverrideBuilder": false
        });
        let resp: ExecutionPayloadEnvelopeV3 = serde_json::from_value(json).unwrap();
        assert!(!resp.should_override_builder);
        assert_eq!(resp.block_value, U256::ZERO);
    }

    #[test]
    fn propose_block_result_from_envelope() {
        let json = serde_json::json!({
            "executionPayload": {
                "parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "feeRecipient": "0x0000000000000000000000000000000000000000",
                "stateRoot": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "receiptsRoot": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                "prevRandao": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "blockNumber": "0x1",
                "gasLimit": "0x1c9c380",
                "gasUsed": "0x0",
                "timestamp": "0x1000",
                "extraData": "0x",
                "baseFeePerGas": "0x3b9aca00",
                "blockHash": "0x0000000000000000000000000000000000000000000000000000000000000001",
                "transactions": [],
                "withdrawals": [],
                "blobGasUsed": "0x0",
                "excessBlobGas": "0x0"
            },
            "blockValue": "0x100",
            "blobsBundle": {
                "commitments": [],
                "proofs": [],
                "blobs": []
            },
            "shouldOverrideBuilder": true
        });
        let resp: ExecutionPayloadEnvelopeV3 = serde_json::from_value(json).unwrap();
        let result = ProposeBlockResult::from(resp);
        assert_eq!(result.block_value, U256::from(0x100));
        assert!(result.should_override_builder);
    }

    #[test]
    fn transition_configuration_roundtrip() {
        let config = TransitionConfiguration {
            terminal_total_difficulty: U256::from(58750000000000000000000u128),
            terminal_block_hash: B256::ZERO,
            terminal_block_number: 0,
        };
        let json = serde_json::to_value(&config).unwrap();
        let back: TransitionConfiguration = serde_json::from_value(json).unwrap();
        assert_eq!(back.terminal_total_difficulty, config.terminal_total_difficulty);
    }

    #[test]
    fn execution_payload_body_v1_deserialize() {
        let json = serde_json::json!({
            "transactions": ["0x02f8"],
            "withdrawals": []
        });
        let body: ExecutionPayloadBodyV1 = serde_json::from_value(json).unwrap();
        assert_eq!(body.transactions.len(), 1);
        assert!(body.withdrawals.is_some());
    }

    #[test]
    fn payload_id_from_bytes() {
        let id = PayloadId::new([0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef]);
        assert_eq!(id.0, B64::from([0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef]));
    }
}
