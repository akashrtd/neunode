use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("no candidates match the required criteria")]
    NoMatches,

    #[error("invalid scoring weights: sum = {actual}, expected 1.0")]
    InvalidWeights { actual: f64 },

    #[error("capability not found: {0}")]
    CapabilityNotFound(String),

    #[error("empty candidate pool")]
    EmptyPool,

    #[error("invalid config: {0}")]
    ConfigInvalid(String),
}

pub type Result<T> = std::result::Result<T, DiscoveryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_no_matches() {
        let err = DiscoveryError::NoMatches;
        assert_eq!(format!("{err}"), "no candidates match the required criteria");
    }

    #[test]
    fn error_display_invalid_weights() {
        let err = DiscoveryError::InvalidWeights { actual: 0.95 };
        assert_eq!(format!("{err}"), "invalid scoring weights: sum = 0.95, expected 1.0");
    }

    #[test]
    fn error_display_capability_not_found() {
        let err = DiscoveryError::CapabilityNotFound("inference:llm".to_string());
        assert_eq!(format!("{err}"), "capability not found: inference:llm");
    }

    #[test]
    fn error_display_empty_pool() {
        let err = DiscoveryError::EmptyPool;
        assert_eq!(format!("{err}"), "empty candidate pool");
    }

    #[test]
    fn error_display_config_invalid() {
        let err = DiscoveryError::ConfigInvalid("max_results must be > 0".to_string());
        assert_eq!(format!("{err}"), "invalid config: max_results must be > 0");
    }

    #[test]
    fn result_ok() {
        let res: Result<u32> = Ok(42);
        assert_eq!(res.unwrap(), 42);
    }

    #[test]
    fn result_err() {
        let res: Result<u32> = Err(DiscoveryError::EmptyPool);
        assert!(res.is_err());
    }
}
