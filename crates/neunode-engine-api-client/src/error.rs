use thiserror::Error;

/// Errors from the Engine API client.
#[derive(Error, Debug)]
pub enum EngineApiError {
    /// HTTP/transport layer error.
    #[error("transport error: {0}")]
    Transport(String),

    /// JSON-RPC error response from the EL.
    #[error("JSON-RPC error {code}: {message}")]
    JsonRpc { code: i64, message: String, data: Option<String> },

    /// Payload validation returned INVALID.
    #[error("invalid payload: {validation_error:?} (latestValidHash: {latest_valid_hash:?})")]
    InvalidPayload { latest_valid_hash: Option<String>, validation_error: Option<String> },

    /// EL is syncing; payload not yet validated.
    #[error("EL is syncing")]
    Syncing,

    /// Unknown payload ID returned by getPayload.
    #[error("unknown payload: {0}")]
    UnknownPayload(String),

    /// Forkchoice state is invalid.
    #[error("invalid forkchoice state: {0}")]
    InvalidForkchoiceState(String),

    /// Payload attributes rejected.
    #[error("invalid payload attributes: {0}")]
    InvalidPayloadAttributes(String),

    /// Unsupported fork.
    #[error("unsupported fork: {0}")]
    UnsupportedFork(String),

    /// Reorg too deep.
    #[error("too deep reorg: {0}")]
    TooDeepReorg(String),

    /// JWT authentication failure.
    #[error("JWT auth error: {0}")]
    JwtAuth(String),

    /// Request timed out.
    #[error("request timed out after {0}ms")]
    Timeout(u64),

    /// Connection lost.
    #[error("connection lost: {0}")]
    ConnectionLost(String),

    /// All retry attempts exhausted.
    #[error("retry exhausted after {attempts} attempts")]
    RetryExhausted { attempts: u32 },

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(String),
}

impl EngineApiError {
    /// Whether the caller should retry this operation.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Syncing
                | Self::Timeout(_)
                | Self::ConnectionLost(_)
                | Self::RetryExhausted { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::EngineApiError;

    #[test]
    fn transport_display() {
        let err = EngineApiError::Transport("connection refused".to_string());
        assert_eq!(err.to_string(), "transport error: connection refused");
    }

    #[test]
    fn json_rpc_display() {
        let err = EngineApiError::JsonRpc {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        };
        assert_eq!(err.to_string(), "JSON-RPC error -32602: Invalid params");
    }

    #[test]
    fn json_rpc_display_with_data() {
        let err = EngineApiError::JsonRpc {
            code: -32000,
            message: "Server error".to_string(),
            data: Some("details".to_string()),
        };
        assert_eq!(err.to_string(), "JSON-RPC error -32000: Server error");
    }

    #[test]
    fn invalid_payload_display() {
        let err = EngineApiError::InvalidPayload {
            latest_valid_hash: Some("0xabc".to_string()),
            validation_error: Some("bad tx".to_string()),
        };
        assert!(err.to_string().contains("bad tx"));
        assert!(err.to_string().contains("0xabc"));
    }

    #[test]
    fn syncing_display() {
        let err = EngineApiError::Syncing;
        assert_eq!(err.to_string(), "EL is syncing");
    }

    #[test]
    fn unknown_payload_display() {
        let err = EngineApiError::UnknownPayload("0x1234".to_string());
        assert_eq!(err.to_string(), "unknown payload: 0x1234");
    }

    #[test]
    fn invalid_forkchoice_state_display() {
        let err = EngineApiError::InvalidForkchoiceState("inconsistent".to_string());
        assert_eq!(err.to_string(), "invalid forkchoice state: inconsistent");
    }

    #[test]
    fn invalid_payload_attributes_display() {
        let err = EngineApiError::InvalidPayloadAttributes("bad attrs".to_string());
        assert_eq!(err.to_string(), "invalid payload attributes: bad attrs");
    }

    #[test]
    fn unsupported_fork_display() {
        let err = EngineApiError::UnsupportedFork("Frontier".to_string());
        assert_eq!(err.to_string(), "unsupported fork: Frontier");
    }

    #[test]
    fn too_deep_reorg_display() {
        let err = EngineApiError::TooDeepReorg("depth 64".to_string());
        assert_eq!(err.to_string(), "too deep reorg: depth 64");
    }

    #[test]
    fn jwt_auth_display() {
        let err = EngineApiError::JwtAuth("bad token".to_string());
        assert_eq!(err.to_string(), "JWT auth error: bad token");
    }

    #[test]
    fn timeout_display() {
        let err = EngineApiError::Timeout(8000);
        assert_eq!(err.to_string(), "request timed out after 8000ms");
    }

    #[test]
    fn connection_lost_display() {
        let err = EngineApiError::ConnectionLost("eof".to_string());
        assert_eq!(err.to_string(), "connection lost: eof");
    }

    #[test]
    fn retry_exhausted_display() {
        let err = EngineApiError::RetryExhausted { attempts: 5 };
        assert_eq!(err.to_string(), "retry exhausted after 5 attempts");
    }

    #[test]
    fn serialization_display() {
        let err = EngineApiError::Serialization("json parse".to_string());
        assert_eq!(err.to_string(), "serialization error: json parse");
    }

    #[test]
    fn is_retryable_syncing() {
        assert!(EngineApiError::Syncing.is_retryable());
    }

    #[test]
    fn is_retryable_timeout() {
        assert!(EngineApiError::Timeout(1000).is_retryable());
    }

    #[test]
    fn is_retryable_connection_lost() {
        assert!(EngineApiError::ConnectionLost("refused".to_string()).is_retryable());
    }

    #[test]
    fn is_retryable_retry_exhausted() {
        assert!(EngineApiError::RetryExhausted { attempts: 5 }.is_retryable());
    }

    #[test]
    fn is_not_retryable_transport() {
        assert!(!EngineApiError::Transport("error".to_string()).is_retryable());
    }

    #[test]
    fn is_not_retryable_invalid_payload() {
        assert!(!EngineApiError::InvalidPayload {
            latest_valid_hash: None,
            validation_error: None,
        }
        .is_retryable());
    }

    #[test]
    fn is_not_retryable_jwt_auth() {
        assert!(!EngineApiError::JwtAuth("bad".to_string()).is_retryable());
    }

    #[test]
    fn is_not_retryable_json_rpc() {
        assert!(!EngineApiError::JsonRpc { code: -32600, message: "bad".to_string(), data: None }
            .is_retryable());
    }

    #[test]
    fn is_not_retryable_unknown_payload() {
        assert!(!EngineApiError::UnknownPayload("0x".to_string()).is_retryable());
    }

    #[test]
    fn is_not_retryable_invalid_forkchoice() {
        assert!(!EngineApiError::InvalidForkchoiceState("bad".to_string()).is_retryable());
    }

    #[test]
    fn is_not_retryable_unsupported_fork() {
        assert!(!EngineApiError::UnsupportedFork("old".to_string()).is_retryable());
    }
}
