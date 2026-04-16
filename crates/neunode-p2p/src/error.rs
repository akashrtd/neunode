use thiserror::Error;

/// Errors that can occur in P2P networking operations.
#[derive(Error, Debug)]
pub enum P2pError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("dial failed: {0}")]
    DialFailed(String),

    #[error("publish failed: {0}")]
    PublishFailed(String),

    #[error("subscription failed: {0}")]
    SubscriptionFailed(String),

    #[error("DHT error: {0}")]
    DhtError(String),

    #[error("discovery error: {0}")]
    DiscoveryError(String),

    #[error("peer not found: {0}")]
    PeerNotFound(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("invalid address: {0}")]
    InvalidAddress(String),

    #[error("encryption error: {0}")]
    EncryptionError(String),

    #[error("channel closed")]
    ChannelClosed,

    #[error("configuration error: {0}")]
    ConfigError(String),
}

/// Result alias for P2P operations.
pub type Result<T> = std::result::Result<T, P2pError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_failed_display() {
        let err = P2pError::ConnectionFailed("refused".to_string());
        assert_eq!(err.to_string(), "connection failed: refused");
    }

    #[test]
    fn dial_failed_display() {
        let err = P2pError::DialFailed("timeout".to_string());
        assert_eq!(err.to_string(), "dial failed: timeout");
    }

    #[test]
    fn publish_failed_display() {
        let err = P2pError::PublishFailed("topic not subscribed".to_string());
        assert_eq!(err.to_string(), "publish failed: topic not subscribed");
    }

    #[test]
    fn subscription_failed_display() {
        let err = P2pError::SubscriptionFailed("max topics exceeded".to_string());
        assert_eq!(err.to_string(), "subscription failed: max topics exceeded");
    }

    #[test]
    fn dht_error_display() {
        let err = P2pError::DhtError("record not found".to_string());
        assert_eq!(err.to_string(), "DHT error: record not found");
    }

    #[test]
    fn discovery_error_display() {
        let err = P2pError::DiscoveryError("no bootstrap peers".to_string());
        assert_eq!(err.to_string(), "discovery error: no bootstrap peers");
    }

    #[test]
    fn peer_not_found_display() {
        let err = P2pError::PeerNotFound("12D3KooABC".to_string());
        assert_eq!(err.to_string(), "peer not found: 12D3KooABC");
    }

    #[test]
    fn timeout_display() {
        let err = P2pError::Timeout("bootstrap".to_string());
        assert_eq!(err.to_string(), "timeout: bootstrap");
    }

    #[test]
    fn invalid_address_display() {
        let err = P2pError::InvalidAddress("garbage".to_string());
        assert_eq!(err.to_string(), "invalid address: garbage");
    }

    #[test]
    fn encryption_error_display() {
        let err = P2pError::EncryptionError("handshake failed".to_string());
        assert_eq!(err.to_string(), "encryption error: handshake failed");
    }

    #[test]
    fn channel_closed_display() {
        let err = P2pError::ChannelClosed;
        assert_eq!(err.to_string(), "channel closed");
    }

    #[test]
    fn result_ok() {
        let res: u32 = 42;
        assert_eq!(res, 42);
    }

    #[test]
    fn result_err() {
        let res: Result<u32> = Err(P2pError::PeerNotFound("x".to_string()));
        assert!(res.is_err());
    }
}
