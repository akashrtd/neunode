use std::process::ExitCode;

/// Structured CLI error with exit codes.
/// Exit codes follow the design:
///   0=success, 1=error, 2=usage, 10=network, 11=timeout,
///   20=auth, 30=insufficient, 40=notfound, 50=ratelimit, 60=conflict
#[derive(Debug)]
pub enum CliError {
    /// Generic error (exit 1)
    General { message: String, source: Option<anyhow::Error> },
    /// Usage/clap error (exit 2)
    Usage(String),
    /// Network error (exit 10)
    Network { message: String, source: Option<anyhow::Error> },
    /// Timeout (exit 11)
    Timeout(String),
    /// Authentication/identity error (exit 20)
    Auth(String),
    /// Insufficient resources (exit 30)
    Insufficient { resource: String, needed: String, available: String },
    /// Not found (exit 40)
    NotFound { resource_type: String, id: String },
    /// Rate limited (exit 50)
    RateLimited { retry_after_secs: Option<u64> },
    /// Conflict (exit 60)
    Conflict { message: String },
}

impl CliError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::General { .. } => ExitCode::from(1),
            Self::Usage(_) => ExitCode::from(2),
            Self::Network { .. } => ExitCode::from(10),
            Self::Timeout(_) => ExitCode::from(11),
            Self::Auth(_) => ExitCode::from(20),
            Self::Insufficient { .. } => ExitCode::from(30),
            Self::NotFound { .. } => ExitCode::from(40),
            Self::RateLimited { .. } => ExitCode::from(50),
            Self::Conflict { .. } => ExitCode::from(60),
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::General { message, source } => match source {
                Some(e) => format!("{message}: {e:#}"),
                None => message.clone(),
            },
            Self::Usage(msg) => format!("usage: {msg}"),
            Self::Network { message, source } => match source {
                Some(e) => format!("network: {message}: {e:#}"),
                None => format!("network: {message}"),
            },
            Self::Timeout(msg) => format!("timeout: {msg}"),
            Self::Auth(msg) => format!("auth: {msg}"),
            Self::Insufficient { resource, needed, available } => {
                format!("insufficient {resource}: need {needed}, have {available}")
            }
            Self::NotFound { resource_type, id } => format!("{resource_type} not found: {id}"),
            Self::RateLimited { retry_after_secs } => match retry_after_secs {
                Some(secs) => format!("rate limited — retry after {secs}s"),
                None => "rate limited".to_string(),
            },
            Self::Conflict { message } => format!("conflict: {message}"),
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for CliError {}

impl CliError {
    pub fn general(msg: impl Into<String>) -> Self {
        Self::General { message: msg.into(), source: None }
    }
    pub fn with_source(msg: impl Into<String>, err: anyhow::Error) -> Self {
        Self::General { message: msg.into(), source: Some(err) }
    }
    pub fn not_found(resource: &str, id: &str) -> Self {
        Self::NotFound { resource_type: resource.to_string(), id: id.to_string() }
    }
    pub fn auth(msg: impl Into<String>) -> Self {
        Self::Auth(msg.into())
    }
    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network { message: msg.into(), source: None }
    }
    pub fn insufficient(resource: &str, needed: &str, available: &str) -> Self {
        Self::Insufficient {
            resource: resource.to_string(),
            needed: needed.to_string(),
            available: available.to_string(),
        }
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict { message: msg.into() }
    }
}

impl From<anyhow::Error> for CliError {
    fn from(err: anyhow::Error) -> Self {
        Self::General { message: "error".to_string(), source: Some(err) }
    }
}

impl From<neunode_core::NeunodeError> for CliError {
    fn from(err: neunode_core::NeunodeError) -> Self {
        match &err {
            neunode_core::NeunodeError::NotFound(id) => {
                Self::NotFound { resource_type: "resource".to_string(), id: id.clone() }
            }
            neunode_core::NeunodeError::PeerNotFound(id) => {
                Self::NotFound { resource_type: "peer".to_string(), id: id.clone() }
            }
            neunode_core::NeunodeError::KeyNotFound(id) => {
                Self::NotFound { resource_type: "key".to_string(), id: id.clone() }
            }
            neunode_core::NeunodeError::InsufficientBalance { have, need } => Self::Insufficient {
                resource: "balance".to_string(),
                needed: need.to_string(),
                available: have.to_string(),
            },
            neunode_core::NeunodeError::ConnectionFailed(msg) => {
                Self::Network { message: msg.clone(), source: None }
            }
            neunode_core::NeunodeError::TimeoutExpired(msg) => Self::Timeout(msg.clone()),
            neunode_core::NeunodeError::AlreadyExists(id) => {
                Self::Conflict { message: format!("already exists: {id}") }
            }
            _ => Self::General { message: err.to_string(), source: None },
        }
    }
}

impl From<neunode_storage::error::StorageError> for CliError {
    fn from(err: neunode_storage::error::StorageError) -> Self {
        match &err {
            neunode_storage::error::StorageError::KeyNotFound { cf, key } => {
                Self::NotFound { resource_type: format!("{cf} key"), id: key.clone() }
            }
            neunode_storage::error::StorageError::InsufficientBalance { required, available } => {
                Self::Insufficient {
                    resource: "balance".to_string(),
                    needed: required.to_string(),
                    available: available.to_string(),
                }
            }
            _ => Self::General { message: err.to_string(), source: None },
        }
    }
}

impl From<neunode_p2p::error::P2pError> for CliError {
    fn from(err: neunode_p2p::error::P2pError) -> Self {
        match &err {
            neunode_p2p::error::P2pError::ConnectionFailed(msg)
            | neunode_p2p::error::P2pError::DialFailed(msg) => {
                Self::Network { message: msg.clone(), source: None }
            }
            neunode_p2p::error::P2pError::PeerNotFound(id) => {
                Self::NotFound { resource_type: "peer".to_string(), id: id.clone() }
            }
            neunode_p2p::error::P2pError::Timeout(msg) => Self::Timeout(msg.clone()),
            _ => Self::General { message: err.to_string(), source: None },
        }
    }
}

impl From<neunode_feed::error::FeedError> for CliError {
    fn from(err: neunode_feed::error::FeedError) -> Self {
        Self::General { message: err.to_string(), source: None }
    }
}

impl From<neunode_bounty::error::BountyError> for CliError {
    fn from(err: neunode_bounty::error::BountyError) -> Self {
        match &err {
            neunode_bounty::error::BountyError::NotFound(id) => {
                Self::NotFound { resource_type: "bounty".to_string(), id: id.clone() }
            }
            neunode_bounty::error::BountyError::InsufficientFunds { required, available } => {
                Self::Insufficient {
                    resource: "funds".to_string(),
                    needed: required.0.to_string(),
                    available: available.0.to_string(),
                }
            }
            neunode_bounty::error::BountyError::InsufficientBond { required, provided } => {
                Self::Insufficient {
                    resource: "bond".to_string(),
                    needed: required.0.to_string(),
                    available: provided.0.to_string(),
                }
            }
            neunode_bounty::error::BountyError::Unauthorized(msg) => Self::Auth(msg.clone()),
            neunode_bounty::error::BountyError::AlreadyClaimed(by) => {
                Self::Conflict { message: format!("bounty already claimed by {by}") }
            }
            neunode_bounty::error::BountyError::AlreadyExists(id) => {
                Self::Conflict { message: format!("bounty already exists: {id}") }
            }
            neunode_bounty::error::BountyError::DeadlineExceeded { .. } => {
                Self::Timeout(err.to_string())
            }
            _ => Self::General { message: err.to_string(), source: None },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general_exit_code() {
        let err = CliError::general("something went wrong");
        assert_eq!(err.exit_code(), ExitCode::from(1));
    }

    #[test]
    fn usage_exit_code() {
        let err = CliError::Usage("bad args".to_string());
        assert_eq!(err.exit_code(), ExitCode::from(2));
    }

    #[test]
    fn network_exit_code() {
        let err = CliError::network("connection refused");
        assert_eq!(err.exit_code(), ExitCode::from(10));
    }

    #[test]
    fn timeout_exit_code() {
        let err = CliError::Timeout("request timed out".to_string());
        assert_eq!(err.exit_code(), ExitCode::from(11));
    }

    #[test]
    fn auth_exit_code() {
        let err = CliError::auth("no active identity");
        assert_eq!(err.exit_code(), ExitCode::from(20));
    }

    #[test]
    fn insufficient_exit_code() {
        let err = CliError::insufficient("nCompute", "1000", "500");
        assert_eq!(err.exit_code(), ExitCode::from(30));
    }

    #[test]
    fn not_found_exit_code() {
        let err = CliError::not_found("bounty", "bnty_abc");
        assert_eq!(err.exit_code(), ExitCode::from(40));
    }

    #[test]
    fn rate_limited_exit_code() {
        let err = CliError::RateLimited { retry_after_secs: Some(60) };
        assert_eq!(err.exit_code(), ExitCode::from(50));
    }

    #[test]
    fn conflict_exit_code() {
        let err = CliError::conflict("already claimed");
        assert_eq!(err.exit_code(), ExitCode::from(60));
    }

    #[test]
    fn message_formatting() {
        let err = CliError::not_found("model", "llama-3b");
        assert_eq!(err.message(), "model not found: llama-3b");
        assert_eq!(err.to_string(), "model not found: llama-3b");
    }

    #[test]
    fn from_anyhow() {
        let err: CliError = anyhow::anyhow!("something broke").into();
        assert_eq!(err.exit_code(), ExitCode::from(1));
    }

    #[test]
    fn insufficient_message() {
        let err = CliError::insufficient("nCompute", "1000", "500");
        assert_eq!(err.message(), "insufficient nCompute: need 1000, have 500");
    }

    // --- From<NeunodeError> tests ---

    #[test]
    fn from_neunode_error_not_found() {
        let err: CliError = neunode_core::NeunodeError::NotFound("agent:123".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(40));
        assert!(err.message().contains("agent:123"));
    }

    #[test]
    fn from_neunode_error_peer_not_found() {
        let err: CliError = neunode_core::NeunodeError::PeerNotFound("12D3Koo".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(40));
        assert!(err.message().contains("12D3Koo"));
    }

    #[test]
    fn from_neunode_error_insufficient_balance() {
        let err: CliError =
            neunode_core::NeunodeError::InsufficientBalance { have: 10, need: 50 }.into();
        assert_eq!(err.exit_code(), ExitCode::from(30));
    }

    #[test]
    fn from_neunode_error_connection_failed() {
        let err: CliError =
            neunode_core::NeunodeError::ConnectionFailed("refused".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(10));
    }

    #[test]
    fn from_neunode_error_timeout() {
        let err: CliError =
            neunode_core::NeunodeError::TimeoutExpired("deadline".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(11));
    }

    #[test]
    fn from_neunode_error_already_exists() {
        let err: CliError =
            neunode_core::NeunodeError::AlreadyExists("did:neunode:abc".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(60));
    }

    #[test]
    fn from_neunode_error_general_fallback() {
        let err: CliError = neunode_core::NeunodeError::InvalidDid("bad".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(1));
        assert!(err.message().contains("invalid DID"));
    }

    // --- From<StorageError> tests ---

    #[test]
    fn from_storage_error_key_not_found() {
        let err: CliError = neunode_storage::error::StorageError::KeyNotFound {
            cf: "identity".to_string(),
            key: "did:test".to_string(),
        }
        .into();
        assert_eq!(err.exit_code(), ExitCode::from(40));
        assert!(err.message().contains("did:test"));
    }

    #[test]
    fn from_storage_error_insufficient_balance() {
        let err: CliError = neunode_storage::error::StorageError::InsufficientBalance {
            required: 100,
            available: 50,
        }
        .into();
        assert_eq!(err.exit_code(), ExitCode::from(30));
    }

    #[test]
    fn from_storage_error_general() {
        let err: CliError =
            neunode_storage::error::StorageError::Serialization("json".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(1));
    }

    // --- From<P2pError> tests ---

    #[test]
    fn from_p2p_error_connection_failed() {
        let err: CliError =
            neunode_p2p::error::P2pError::ConnectionFailed("refused".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(10));
    }

    #[test]
    fn from_p2p_error_peer_not_found() {
        let err: CliError = neunode_p2p::error::P2pError::PeerNotFound("12D3".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(40));
    }

    #[test]
    fn from_p2p_error_timeout() {
        let err: CliError = neunode_p2p::error::P2pError::Timeout("bootstrap".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(11));
    }

    #[test]
    fn from_p2p_error_general() {
        let err: CliError = neunode_p2p::error::P2pError::ChannelClosed.into();
        assert_eq!(err.exit_code(), ExitCode::from(1));
    }

    // --- From<FeedError> tests ---

    #[test]
    fn from_feed_error() {
        let err: CliError = neunode_feed::error::FeedError::InvalidEvent("bad".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(1));
        assert!(err.message().contains("invalid event"));
    }

    #[test]
    fn from_feed_error_hash_chain() {
        let err: CliError = neunode_feed::error::FeedError::HashChainBroken { seq: 42 }.into();
        assert_eq!(err.exit_code(), ExitCode::from(1));
        assert!(err.message().contains("hash chain broken"));
    }

    // --- From<BountyError> tests ---

    #[test]
    fn from_bounty_error_not_found() {
        let err: CliError =
            neunode_bounty::error::BountyError::NotFound("bnty_abc".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(40));
        assert!(err.message().contains("bnty_abc"));
    }

    #[test]
    fn from_bounty_error_unauthorized() {
        let err: CliError =
            neunode_bounty::error::BountyError::Unauthorized("wrong agent".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(20));
    }

    #[test]
    fn from_bounty_error_already_claimed() {
        let err: CliError =
            neunode_bounty::error::BountyError::AlreadyClaimed("did:other".to_string()).into();
        assert_eq!(err.exit_code(), ExitCode::from(60));
    }

    #[test]
    fn from_bounty_error_deadline_exceeded() {
        let err: CliError = neunode_bounty::error::BountyError::DeadlineExceeded {
            deadline_type: "claim".to_string(),
            deadline: 100,
            now: 200,
        }
        .into();
        assert_eq!(err.exit_code(), ExitCode::from(11));
    }

    #[test]
    fn from_bounty_error_insufficient_funds() {
        use neunode_core::types::TokenAmount;
        let err: CliError = neunode_bounty::error::BountyError::InsufficientFunds {
            required: TokenAmount(1000),
            available: TokenAmount(500),
        }
        .into();
        assert_eq!(err.exit_code(), ExitCode::from(30));
    }

    #[test]
    fn from_bounty_error_general() {
        let err: CliError = neunode_bounty::error::BountyError::NotClaimed.into();
        assert_eq!(err.exit_code(), ExitCode::from(1));
    }
}
