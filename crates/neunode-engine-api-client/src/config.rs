use std::time::Duration;

/// Configuration for the Engine API client.
#[derive(Debug, Clone)]
pub struct EngineApiClientConfig {
    /// URL of the EL Engine API endpoint.
    /// Default: "http://127.0.0.1:8551"
    pub endpoint: String,

    /// Path to the JWT secret file (hex-encoded 256-bit key).
    pub jwt_secret_path: Option<std::path::PathBuf>,

    /// Raw JWT secret bytes (alternative to file-based loading).
    pub jwt_secret: Option<Vec<u8>>,

    /// HTTP request timeout.
    /// Default: 10 seconds (covers 8s engine methods).
    pub request_timeout: Duration,

    /// Maximum number of retry attempts for transient failures.
    /// Default: 5.
    pub max_retries: u32,

    /// Base delay for exponential backoff on retries.
    /// Default: 500ms.
    pub retry_base_delay: Duration,

    /// Maximum backoff delay cap.
    /// Default: 30 seconds.
    pub retry_max_delay: Duration,

    /// Connection timeout for establishing TCP connection.
    /// Default: 5 seconds.
    pub connect_timeout: Duration,
}

impl Default for EngineApiClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:8551".into(),
            jwt_secret_path: None,
            jwt_secret: None,
            request_timeout: Duration::from_secs(10),
            max_retries: 5,
            retry_base_delay: Duration::from_millis(500),
            retry_max_delay: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = EngineApiClientConfig::default();
        assert_eq!(config.endpoint, "http://127.0.0.1:8551");
        assert!(config.jwt_secret_path.is_none());
        assert!(config.jwt_secret.is_none());
        assert_eq!(config.request_timeout, Duration::from_secs(10));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.retry_base_delay, Duration::from_millis(500));
        assert_eq!(config.retry_max_delay, Duration::from_secs(30));
        assert_eq!(config.connect_timeout, Duration::from_secs(5));
    }

    #[test]
    fn custom_config() {
        let config = EngineApiClientConfig {
            endpoint: "http://reth:8551".into(),
            jwt_secret_path: Some("/data/jwt.hex".into()),
            request_timeout: Duration::from_secs(20),
            max_retries: 10,
            retry_base_delay: Duration::from_millis(200),
            retry_max_delay: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(3),
            ..Default::default()
        };
        assert_eq!(config.endpoint, "http://reth:8551");
        assert_eq!(config.jwt_secret_path.unwrap().to_str().unwrap(), "/data/jwt.hex");
        assert_eq!(config.request_timeout, Duration::from_secs(20));
        assert_eq!(config.max_retries, 10);
    }

    #[test]
    fn config_with_raw_secret() {
        let config =
            EngineApiClientConfig { jwt_secret: Some(vec![42u8; 32]), ..Default::default() };
        assert_eq!(config.jwt_secret.unwrap().len(), 32);
    }

    #[test]
    fn config_debug_format() {
        let config = EngineApiClientConfig::default();
        let debug = format!("{config:?}");
        assert!(debug.contains("http://127.0.0.1:8551"));
        assert!(debug.contains("max_retries: 5"));
    }

    #[test]
    fn config_clone() {
        let config = EngineApiClientConfig::default();
        let cloned = config.clone();
        assert_eq!(config.endpoint, cloned.endpoint);
        assert_eq!(config.max_retries, cloned.max_retries);
    }
}
