use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[ts(export)]
pub struct AppConfig {
    pub agent: AgentConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub tokens: TokenConfig,
    #[serde(default)]
    pub active_identity: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[ts(export)]
pub struct AgentConfig {
    pub name: String,
    #[serde(default = "default_did_method")]
    pub did_method: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_did_method() -> String {
    "key".to_string()
}

fn default_data_dir() -> String {
    "~/.neunode".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[ts(export)]
pub struct NetworkConfig {
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
    #[serde(default)]
    pub bootstrap_peers: Vec<String>,
    #[serde(default = "default_mesh_degree")]
    pub mesh_degree: usize,
    #[serde(default = "default_true")]
    pub enable_mdns: bool,
    #[serde(default = "default_true")]
    pub enable_relay: bool,
}

fn default_listen_addr() -> String {
    "/ip4/0.0.0.0/tcp/0".to_string()
}

fn default_mesh_degree() -> usize {
    6
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[ts(export)]
pub struct StorageConfig {
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default = "default_cache_size")]
    pub cache_size: usize,
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_secs: u64,
}

fn default_db_path() -> String {
    "~/.neunode/db".to_string()
}

fn default_cache_size() -> usize {
    256 * 1024 * 1024
}

fn default_cache_ttl() -> u64 {
    300
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[ts(export)]
pub struct TokenConfig {
    #[serde(default = "default_decay_interval")]
    pub decay_check_interval_secs: u64,
    #[serde(default = "default_unbonding_period")]
    pub unbonding_period_secs: u64,
}

fn default_decay_interval() -> u64 {
    3600
}

fn default_unbonding_period() -> u64 {
    7 * 24 * 3600
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: default_listen_addr(),
            bootstrap_peers: Vec::new(),
            mesh_degree: default_mesh_degree(),
            enable_mdns: true,
            enable_relay: true,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            cache_size: default_cache_size(),
            cache_ttl_secs: default_cache_ttl(),
        }
    }
}

impl Default for TokenConfig {
    fn default() -> Self {
        Self {
            decay_check_interval_secs: default_decay_interval(),
            unbonding_period_secs: default_unbonding_period(),
        }
    }
}

impl AppConfig {
    pub fn from_toml(toml_str: &str) -> crate::error::Result<Self> {
        toml::from_str(toml_str).map_err(|e| crate::error::NeunodeError::ConfigError(e.to_string()))
    }

    pub fn to_toml(&self) -> crate::error::Result<String> {
        toml::to_string_pretty(self)
            .map_err(|e| crate::error::NeunodeError::ConfigError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_toml_parse() {
        let toml = r#"
[agent]
name = "test-agent"
did_method = "neunode"
data_dir = "/data/agent"
log_level = "debug"

[network]
listen_addr = "/ip4/0.0.0.0/tcp/41000"
bootstrap_peers = ["/ip4/1.2.3.4/tcp/4001/p2p/QmABC"]
mesh_degree = 8
enable_mdns = false
enable_relay = false

[storage]
db_path = "/data/db"
cache_size = 536870912
cache_ttl_secs = 600

[tokens]
decay_check_interval_secs = 7200
unbonding_period_secs = 1209600
"#;
        let config = AppConfig::from_toml(toml).unwrap();
        assert_eq!(config.agent.name, "test-agent");
        assert_eq!(config.agent.did_method, "neunode");
        assert_eq!(config.agent.data_dir, "/data/agent");
        assert_eq!(config.agent.log_level, "debug");
        assert_eq!(config.network.listen_addr, "/ip4/0.0.0.0/tcp/41000");
        assert_eq!(config.network.bootstrap_peers.len(), 1);
        assert_eq!(config.network.mesh_degree, 8);
        assert!(!config.network.enable_mdns);
        assert!(!config.network.enable_relay);
        assert_eq!(config.storage.db_path, "/data/db");
        assert_eq!(config.storage.cache_size, 536870912);
        assert_eq!(config.storage.cache_ttl_secs, 600);
        assert_eq!(config.tokens.decay_check_interval_secs, 7200);
        assert_eq!(config.tokens.unbonding_period_secs, 1209600);
    }

    #[test]
    fn toml_defaults() {
        let toml = r#"
[agent]
name = "minimal"
"#;
        let config = AppConfig::from_toml(toml).unwrap();
        assert_eq!(config.agent.name, "minimal");
        assert_eq!(config.agent.did_method, "key");
        assert_eq!(config.agent.data_dir, "~/.neunode");
        assert_eq!(config.agent.log_level, "info");
        assert_eq!(config.network.listen_addr, "/ip4/0.0.0.0/tcp/0");
        assert!(config.network.bootstrap_peers.is_empty());
        assert_eq!(config.network.mesh_degree, 6);
        assert!(config.network.enable_mdns);
        assert!(config.network.enable_relay);
        assert_eq!(config.storage.db_path, "~/.neunode/db");
        assert_eq!(config.storage.cache_size, 256 * 1024 * 1024);
        assert_eq!(config.storage.cache_ttl_secs, 300);
        assert_eq!(config.tokens.decay_check_interval_secs, 3600);
        assert_eq!(config.tokens.unbonding_period_secs, 7 * 24 * 3600);
    }

    #[test]
    fn roundtrip_parse_serialize_parse() {
        let toml = r#"
[agent]
name = "roundtrip-agent"

[network]
listen_addr = "/ip4/0.0.0.0/tcp/41000"
bootstrap_peers = ["/ip4/1.2.3.4/tcp/4001/p2p/QmABC"]
mesh_degree = 8
enable_mdns = true
enable_relay = true

[storage]
db_path = "/data/db"
cache_size = 536870912
cache_ttl_secs = 600

[tokens]
decay_check_interval_secs = 7200
unbonding_period_secs = 1209600
"#;
        let config1 = AppConfig::from_toml(toml).unwrap();
        let serialized = config1.to_toml().unwrap();
        let config2 = AppConfig::from_toml(&serialized).unwrap();
        assert_eq!(config1, config2);
    }

    #[test]
    fn invalid_toml_returns_config_error() {
        let result = AppConfig::from_toml("this is not valid toml {{{{");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::error::NeunodeError::ConfigError(_)));
        assert!(err.to_string().contains("config error"));
    }

    #[test]
    fn missing_agent_name_returns_config_error() {
        let toml = r#"
[network]
listen_addr = "/ip4/0.0.0.0/tcp/0"
"#;
        let result = AppConfig::from_toml(toml);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), crate::error::NeunodeError::ConfigError(_)));
    }

    #[test]
    fn to_toml_produces_valid_toml() {
        let config = AppConfig {
            agent: AgentConfig {
                name: "test".to_string(),
                did_method: "key".to_string(),
                data_dir: "~/.neunode".to_string(),
                log_level: "info".to_string(),
            },
            network: NetworkConfig {
                listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
                bootstrap_peers: vec![],
                mesh_degree: 6,
                enable_mdns: true,
                enable_relay: true,
            },
            storage: StorageConfig {
                db_path: "~/.neunode/db".to_string(),
                cache_size: 256 * 1024 * 1024,
                cache_ttl_secs: 300,
            },
            tokens: TokenConfig {
                decay_check_interval_secs: 3600,
                unbonding_period_secs: 7 * 24 * 3600,
            },
            active_identity: None,
        };
        let toml_str = config.to_toml().unwrap();
        assert!(toml_str.contains("test"));
        assert!(toml_str.contains("agent"));
        assert!(toml_str.contains("network"));
        assert!(toml_str.contains("storage"));
        assert!(toml_str.contains("tokens"));
    }
}
