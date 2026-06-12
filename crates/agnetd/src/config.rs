use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use neunode_core::config::AppConfig;

const CONFIG_DIR_NAME: &str = ".agnetd";
const CONFIG_FILE_NAME: &str = "config.toml";

/// Type-safe config key enum — replaces stringly-typed key matching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigKey {
    AgentName,
    AgentDidMethod,
    AgentDataDir,
    AgentLogLevel,
    NetworkListenAddr,
    NetworkMeshDegree,
    NetworkEnableMdns,
    NetworkEnableRelay,
    NetworkBootstrapPeers,
    StorageDbPath,
    StorageCacheSize,
    StorageCacheTtlSecs,
    TokensDecayCheckIntervalSecs,
    TokensUnbondingPeriodSecs,
    ContractsEthRpcUrl,
    ContractsIdentityContractAddress,
}

impl ConfigKey {
    pub fn from_dot_path(key: &str) -> Option<Self> {
        match key {
            "agent.name" => Some(Self::AgentName),
            "agent.did_method" => Some(Self::AgentDidMethod),
            "agent.data_dir" => Some(Self::AgentDataDir),
            "agent.log_level" => Some(Self::AgentLogLevel),
            "network.listen_addr" => Some(Self::NetworkListenAddr),
            "network.mesh_degree" => Some(Self::NetworkMeshDegree),
            "network.enable_mdns" => Some(Self::NetworkEnableMdns),
            "network.enable_relay" => Some(Self::NetworkEnableRelay),
            "network.bootstrap_peers" => Some(Self::NetworkBootstrapPeers),
            "storage.db_path" => Some(Self::StorageDbPath),
            "storage.cache_size" => Some(Self::StorageCacheSize),
            "storage.cache_ttl_secs" => Some(Self::StorageCacheTtlSecs),
            "tokens.decay_check_interval_secs" => Some(Self::TokensDecayCheckIntervalSecs),
            "tokens.unbonding_period_secs" => Some(Self::TokensUnbondingPeriodSecs),
            "contracts.eth_rpc_url" => Some(Self::ContractsEthRpcUrl),
            "contracts.identity_contract_address" => Some(Self::ContractsIdentityContractAddress),
            _ => None,
        }
    }

    pub fn to_dot_path(self) -> &'static str {
        match self {
            Self::AgentName => "agent.name",
            Self::AgentDidMethod => "agent.did_method",
            Self::AgentDataDir => "agent.data_dir",
            Self::AgentLogLevel => "agent.log_level",
            Self::NetworkListenAddr => "network.listen_addr",
            Self::NetworkMeshDegree => "network.mesh_degree",
            Self::NetworkEnableMdns => "network.enable_mdns",
            Self::NetworkEnableRelay => "network.enable_relay",
            Self::NetworkBootstrapPeers => "network.bootstrap_peers",
            Self::StorageDbPath => "storage.db_path",
            Self::StorageCacheSize => "storage.cache_size",
            Self::StorageCacheTtlSecs => "storage.cache_ttl_secs",
            Self::TokensDecayCheckIntervalSecs => "tokens.decay_check_interval_secs",
            Self::TokensUnbondingPeriodSecs => "tokens.unbonding_period_secs",
            Self::ContractsEthRpcUrl => "contracts.eth_rpc_url",
            Self::ContractsIdentityContractAddress => "contracts.identity_contract_address",
        }
    }

    pub const ALL: [Self; 16] = [
        Self::AgentName,
        Self::AgentDidMethod,
        Self::AgentDataDir,
        Self::AgentLogLevel,
        Self::NetworkListenAddr,
        Self::NetworkMeshDegree,
        Self::NetworkEnableMdns,
        Self::NetworkEnableRelay,
        Self::NetworkBootstrapPeers,
        Self::StorageDbPath,
        Self::StorageCacheSize,
        Self::StorageCacheTtlSecs,
        Self::TokensDecayCheckIntervalSecs,
        Self::TokensUnbondingPeriodSecs,
        Self::ContractsEthRpcUrl,
        Self::ContractsIdentityContractAddress,
    ];
}

#[derive(Clone)]
pub struct CliConfig {
    pub config_path: PathBuf,
    pub app_config: AppConfig,
    #[allow(dead_code)]
    pub active_identity: Option<String>,
}

impl CliConfig {
    pub fn load(config_path: Option<&str>) -> Result<Self> {
        let path = match config_path {
            Some(p) => PathBuf::from(p),
            None => Self::default_path(),
        };

        let app_config = if path.exists() {
            let contents = fs::read_to_string(&path)
                .with_context(|| format!("failed to read config from {}", path.display()))?;
            AppConfig::from_toml(&contents)
                .with_context(|| format!("failed to parse config from {}", path.display()))?
        } else {
            let default = Self::default_app_config();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create config directory {}", parent.display())
                })?;
            }
            let toml_str = default
                .to_toml()
                .with_context(|| "failed to serialize default config".to_string())?;
            fs::write(&path, &toml_str)
                .with_context(|| format!("failed to write default config to {}", path.display()))?;
            default
        };

        let active_identity = app_config.active_identity.clone();
        Ok(Self { config_path: path, app_config, active_identity })
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        let toml_str =
            self.app_config.to_toml().with_context(|| "failed to serialize config".to_string())?;
        fs::write(&self.config_path, &toml_str)
            .with_context(|| format!("failed to write config to {}", self.config_path.display()))?;
        Ok(())
    }

    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(CONFIG_DIR_NAME)
            .join(CONFIG_FILE_NAME)
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let ck = ConfigKey::from_dot_path(key)
            .ok_or_else(|| anyhow::anyhow!("unknown config key: {key}"))?;
        self.set_typed(ck, value)
    }

    pub fn set_typed(&mut self, key: ConfigKey, value: &str) -> Result<()> {
        match key {
            ConfigKey::AgentName => self.app_config.agent.name = value.to_string(),
            ConfigKey::AgentDidMethod => self.app_config.agent.did_method = value.to_string(),
            ConfigKey::AgentDataDir => self.app_config.agent.data_dir = value.to_string(),
            ConfigKey::AgentLogLevel => self.app_config.agent.log_level = value.to_string(),
            ConfigKey::NetworkListenAddr => {
                self.app_config.network.listen_addr = value.to_string()
            }
            ConfigKey::NetworkMeshDegree => {
                self.app_config.network.mesh_degree =
                    value.parse().with_context(|| format!("invalid integer: {value}"))?;
            }
            ConfigKey::NetworkEnableMdns => {
                self.app_config.network.enable_mdns =
                    value.parse().with_context(|| format!("invalid boolean: {value}"))?;
            }
            ConfigKey::NetworkEnableRelay => {
                self.app_config.network.enable_relay =
                    value.parse().with_context(|| format!("invalid boolean: {value}"))?;
            }
            ConfigKey::NetworkBootstrapPeers => {
                self.app_config.network.bootstrap_peers = if value.is_empty() {
                    Vec::new()
                } else {
                    value.split(',').map(|s| s.trim().to_string()).collect()
                };
            }
            ConfigKey::StorageDbPath => self.app_config.storage.db_path = value.to_string(),
            ConfigKey::StorageCacheSize => {
                self.app_config.storage.cache_size =
                    value.parse().with_context(|| format!("invalid integer: {value}"))?;
            }
            ConfigKey::StorageCacheTtlSecs => {
                self.app_config.storage.cache_ttl_secs =
                    value.parse().with_context(|| format!("invalid integer: {value}"))?;
            }
            ConfigKey::TokensDecayCheckIntervalSecs => {
                self.app_config.tokens.decay_check_interval_secs =
                    value.parse().with_context(|| format!("invalid integer: {value}"))?;
            }
            ConfigKey::TokensUnbondingPeriodSecs => {
                self.app_config.tokens.unbonding_period_secs =
                    value.parse().with_context(|| format!("invalid integer: {value}"))?;
            }
            ConfigKey::ContractsEthRpcUrl => {
                self.app_config.contracts.eth_rpc_url = Some(value.to_string());
            }
            ConfigKey::ContractsIdentityContractAddress => {
                self.app_config.contracts.identity_contract_address = Some(value.to_string());
            }
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<String> {
        ConfigKey::from_dot_path(key).map(|ck| self.get_typed(ck))
    }

    pub fn get_typed(&self, key: ConfigKey) -> String {
        match key {
            ConfigKey::AgentName => self.app_config.agent.name.clone(),
            ConfigKey::AgentDidMethod => self.app_config.agent.did_method.clone(),
            ConfigKey::AgentDataDir => self.app_config.agent.data_dir.clone(),
            ConfigKey::AgentLogLevel => self.app_config.agent.log_level.clone(),
            ConfigKey::NetworkListenAddr => self.app_config.network.listen_addr.clone(),
            ConfigKey::NetworkMeshDegree => self.app_config.network.mesh_degree.to_string(),
            ConfigKey::NetworkEnableMdns => self.app_config.network.enable_mdns.to_string(),
            ConfigKey::NetworkEnableRelay => self.app_config.network.enable_relay.to_string(),
            ConfigKey::NetworkBootstrapPeers => {
                self.app_config.network.bootstrap_peers.join(", ")
            }
            ConfigKey::StorageDbPath => self.app_config.storage.db_path.clone(),
            ConfigKey::StorageCacheSize => self.app_config.storage.cache_size.to_string(),
            ConfigKey::StorageCacheTtlSecs => self.app_config.storage.cache_ttl_secs.to_string(),
            ConfigKey::TokensDecayCheckIntervalSecs => {
                self.app_config.tokens.decay_check_interval_secs.to_string()
            }
            ConfigKey::TokensUnbondingPeriodSecs => {
                self.app_config.tokens.unbonding_period_secs.to_string()
            }
            ConfigKey::ContractsEthRpcUrl => {
                self.app_config.contracts.eth_rpc_url.clone().unwrap_or_default()
            }
            ConfigKey::ContractsIdentityContractAddress => {
                self.app_config.contracts.identity_contract_address.clone().unwrap_or_default()
            }
        }
    }

    pub fn list_all(&self) -> Vec<(String, String)> {
        ConfigKey::ALL
            .map(|ck| (ck.to_dot_path().to_string(), self.get_typed(ck)))
            .to_vec()
    }

    fn default_app_config() -> AppConfig {
        AppConfig {
            agent: neunode_core::config::AgentConfig {
                name: "default".to_string(),
                did_method: "key".to_string(),
                data_dir: dirs::home_dir()
                    .map(|p| p.join(".neunode").to_string_lossy().to_string())
                    .unwrap_or_else(|| "~/.neunode".to_string()),
                log_level: "info".to_string(),
            },
            network: neunode_core::config::NetworkConfig::default(),
            storage: neunode_core::config::StorageConfig::default(),
            tokens: neunode_core::config::TokenConfig::default(),
            contracts: neunode_core::config::ContractsConfig::default(),
            active_identity: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config_path() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join("agnetd_test_config");
        let _ = fs::create_dir_all(&dir);
        dir.join(format!("config_{}_{}.toml", std::process::id(), id))
    }

    fn cleanup(path: &PathBuf) {
        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_creates_default_when_missing() {
        let path = temp_config_path();
        cleanup(&path);
        let config = CliConfig::load(Some(path.to_str().unwrap())).expect("load");
        assert_eq!(config.app_config.agent.name, "default");
        assert!(path.exists());
        cleanup(&path);
    }

    #[test]
    fn save_and_reload_roundtrip() {
        let path = temp_config_path();
        cleanup(&path);
        let mut config = CliConfig::load(Some(path.to_str().unwrap())).expect("load");
        config.app_config.agent.name = "roundtrip-test".to_string();
        config.save().expect("save");

        let reloaded = CliConfig::load(Some(path.to_str().unwrap())).expect("reload");
        assert_eq!(reloaded.app_config.agent.name, "roundtrip-test");
        cleanup(&path);
    }

    #[test]
    fn set_agent_name() {
        let path = temp_config_path();
        cleanup(&path);
        let mut config = CliConfig::load(Some(path.to_str().unwrap())).expect("load");
        config.set("agent.name", "new-name").expect("set");
        assert_eq!(config.app_config.agent.name, "new-name");
        cleanup(&path);
    }

    #[test]
    fn set_network_mesh_degree() {
        let path = temp_config_path();
        cleanup(&path);
        let mut config = CliConfig::load(Some(path.to_str().unwrap())).expect("load");
        config.set("network.mesh_degree", "12").expect("set");
        assert_eq!(config.app_config.network.mesh_degree, 12);
        cleanup(&path);
    }

    #[test]
    fn set_invalid_mesh_degree_fails() {
        let path = temp_config_path();
        cleanup(&path);
        let mut config = CliConfig::load(Some(path.to_str().unwrap())).expect("load");
        assert!(config.set("network.mesh_degree", "not_a_number").is_err());
        cleanup(&path);
    }

    #[test]
    fn set_unknown_key_fails() {
        let path = temp_config_path();
        cleanup(&path);
        let mut config = CliConfig::load(Some(path.to_str().unwrap())).expect("load");
        assert!(config.set("nonexistent.key", "value").is_err());
        cleanup(&path);
    }

    #[test]
    fn get_existing_key() {
        let path = temp_config_path();
        cleanup(&path);
        let config = CliConfig::load(Some(path.to_str().unwrap())).expect("load");
        assert_eq!(config.get("agent.name"), Some("default".to_string()));
        assert_eq!(config.get("network.mesh_degree"), Some("6".to_string()));
        cleanup(&path);
    }

    #[test]
    fn get_nonexistent_key_returns_none() {
        let path = temp_config_path();
        cleanup(&path);
        let config = CliConfig::load(Some(path.to_str().unwrap())).expect("load");
        assert_eq!(config.get("nonexistent.key"), None);
        cleanup(&path);
    }

    #[test]
    fn list_all_returns_all_keys() {
        let path = temp_config_path();
        cleanup(&path);
        let config = CliConfig::load(Some(path.to_str().unwrap())).expect("load");
        let all = config.list_all();
        assert!(all.len() >= 13);
        assert!(all.iter().any(|(k, _)| k == "agent.name"));
        assert!(all.iter().any(|(k, _)| k == "network.mesh_degree"));
        assert!(all.iter().any(|(k, _)| k == "storage.db_path"));
        assert!(all.iter().any(|(k, _)| k == "tokens.unbonding_period_secs"));
        cleanup(&path);
    }

    #[test]
    fn set_boolean_field() {
        let path = temp_config_path();
        cleanup(&path);
        let mut config = CliConfig::load(Some(path.to_str().unwrap())).expect("load");
        config.set("network.enable_mdns", "false").expect("set");
        assert!(!config.app_config.network.enable_mdns);
        cleanup(&path);
    }

    #[test]
    fn set_all_agent_fields() {
        let path = temp_config_path();
        cleanup(&path);
        let mut config = CliConfig::load(Some(path.to_str().unwrap())).expect("load");
        config.set("agent.name", "test").unwrap();
        config.set("agent.did_method", "neunode").unwrap();
        config.set("agent.data_dir", "/tmp/test").unwrap();
        config.set("agent.log_level", "debug").unwrap();
        assert_eq!(config.app_config.agent.name, "test");
        assert_eq!(config.app_config.agent.did_method, "neunode");
        assert_eq!(config.app_config.agent.data_dir, "/tmp/test");
        assert_eq!(config.app_config.agent.log_level, "debug");
        cleanup(&path);
    }

    #[test]
    fn config_key_from_dot_path_roundtrip() {
        for ck in ConfigKey::ALL {
            let path = ck.to_dot_path();
            assert_eq!(ConfigKey::from_dot_path(path), Some(ck));
        }
    }

    #[test]
    fn config_key_all_count() {
        assert_eq!(ConfigKey::ALL.len(), 16);
    }

    #[test]
    fn set_typed_equivalent_to_set() {
        let path = temp_config_path();
        cleanup(&path);
        let mut config = CliConfig::load(Some(path.to_str().unwrap())).expect("load");
        config.set_typed(ConfigKey::AgentName, "typed-test").unwrap();
        assert_eq!(config.get_typed(ConfigKey::AgentName), "typed-test");
        assert_eq!(config.get("agent.name"), Some("typed-test".to_string()));
        cleanup(&path);
    }
}
