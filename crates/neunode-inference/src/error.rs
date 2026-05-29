use neunode_core::types::TokenAmount;
use thiserror::Error;

/// Errors returned by inference operations.
#[derive(Debug, Error)]
pub enum InferenceError {
    #[error("provider unavailable")]
    ProviderUnavailable,

    #[error("model not found: {0}")]
    ModelNotFound(String),

    #[error("insufficient tokens: need {required}, have {available}")]
    InsufficientTokens { required: TokenAmount, available: TokenAmount },

    #[error("request failed: {0}")]
    RequestFailed(String),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("request timed out")]
    Timeout,

    #[error("rate limited")]
    RateLimited,

    #[error("verification failed: {0}")]
    VerificationFailed(String),

    #[error("routing error: {0}")]
    RoutingError(String),

    #[error("settlement failed: {0}")]
    SettlementFailed(String),

    #[error("protocol fee exceeds gross cost: fee={fee}, gross={gross}")]
    FeeExceedsGross { fee: TokenAmount, gross: TokenAmount },

    #[error("token count exceeds estimate: provider reports {provider_tokens} {token_type} tokens, max allowed is {max_allowed}")]
    TokenCountExceedsEstimate { provider_tokens: u32, max_allowed: u32, token_type: String },
}

/// Result type alias for inference operations.
pub type Result<T> = std::result::Result<T, InferenceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_provider_unavailable() {
        let err = InferenceError::ProviderUnavailable;
        assert_eq!(format!("{err}"), "provider unavailable");
    }

    #[test]
    fn error_display_model_not_found() {
        let err = InferenceError::ModelNotFound("llama-3b".to_string());
        assert_eq!(format!("{err}"), "model not found: llama-3b");
    }

    #[test]
    fn error_display_insufficient_tokens() {
        let err = InferenceError::InsufficientTokens {
            required: TokenAmount(100),
            available: TokenAmount(50),
        };
        assert_eq!(format!("{err}"), "insufficient tokens: need 100, have 50");
    }

    #[test]
    fn error_display_request_failed() {
        let err = InferenceError::RequestFailed("connection reset".to_string());
        assert_eq!(format!("{err}"), "request failed: connection reset");
    }

    #[test]
    fn error_display_invalid_request() {
        let err = InferenceError::InvalidRequest("empty messages".to_string());
        assert_eq!(format!("{err}"), "invalid request: empty messages");
    }

    #[test]
    fn error_display_timeout() {
        let err = InferenceError::Timeout;
        assert_eq!(format!("{err}"), "request timed out");
    }

    #[test]
    fn error_display_rate_limited() {
        let err = InferenceError::RateLimited;
        assert_eq!(format!("{err}"), "rate limited");
    }

    #[test]
    fn error_display_verification_failed() {
        let err = InferenceError::VerificationFailed("hash mismatch".to_string());
        assert_eq!(format!("{err}"), "verification failed: hash mismatch");
    }

    #[test]
    fn error_display_routing_error() {
        let err = InferenceError::RoutingError("no providers".to_string());
        assert_eq!(format!("{err}"), "routing error: no providers");
    }

    #[test]
    fn error_display_settlement_failed() {
        let err = InferenceError::SettlementFailed("balance too low".to_string());
        assert_eq!(format!("{err}"), "settlement failed: balance too low");
    }

    #[test]
    fn error_display_fee_exceeds_gross() {
        let err = InferenceError::FeeExceedsGross { fee: TokenAmount(100), gross: TokenAmount(50) };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("50"));
    }

    #[test]
    fn result_ok() {
        let res: u32 = 42;
        assert_eq!(res, 42);
    }

    #[test]
    fn result_err() {
        let res: Result<u32> = Err(InferenceError::Timeout);
        assert!(res.is_err());
    }
}
