use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use neunode_core::config::AppConfig;

const CONFIG_DIR_NAME: &str = ".agnetd";
const CONFIG_FILE_NAME: &str = "config.toml";

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

    // TODO: Replace string key matching with a ConfigKey enum for type safety
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["agent", "name"] => self.app_config.agent.name = value.to_string(),
            ["agent", "did_method"] => self.app_config.agent.did_method = value.to_string(),
            ["agent", "data_dir"] => self.app_config.agent.data_dir = value.to_string(),
            ["agent", "log_level"] => self.app_config.agent.log_level = value.to_string(),
            ["network", "listen_addr"] => self.app_config.network.listen_addr = value.to_string(),
            ["network", "mesh_degree"] => {
                self.app_config.network.mesh_degree =
                    value.parse().with_context(|| format!("invalid integer: {value}"))?;
            }
            ["network", "enable_mdns"] => {
                self.app_config.network.enable_mdns =
                    value.parse().with_context(|| format!("invalid boolean: {value}"))?;
            }
            ["network", "enable_relay"] => {
                self.app_config.network.enable_relay =
                    value.parse().with_context(|| format!("invalid boolean: {value}"))?;
            }
            ["storage", "db_path"] => self.app_config.storage.db_path = value.to_string(),
            ["storage", "cache_size"] => {
                self.app_config.storage.cache_size =
                    value.parse().with_context(|| format!("invalid integer: {value}"))?;
            }
            ["storage", "cache_ttl_secs"] => {
                self.app_config.storage.cache_ttl_secs =
                    value.parse().with_context(|| format!("invalid integer: {value}"))?;
            }
            ["tokens", "decay_check_interval_secs"] => {
                self.app_config.tokens.decay_check_interval_secs =
                    value.parse().with_context(|| format!("invalid integer: {value}"))?;
            }
            ["tokens", "unbonding_period_secs"] => {
                self.app_config.tokens.unbonding_period_secs =
                    value.parse().with_context(|| format!("invalid integer: {value}"))?;
            }
            _ => anyhow::bail!("unknown config key: {key}"),
        }
        Ok(())
    }

    // TODO: Replace string key matching with a ConfigKey enum for type safety
    pub fn get(&self, key: &str) -> Option<String> {
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["agent", "name"] => Some(self.app_config.agent.name.clone()),
            ["agent", "did_method"] => Some(self.app_config.agent.did_method.clone()),
            ["agent", "data_dir"] => Some(self.app_config.agent.data_dir.clone()),
            ["agent", "log_level"] => Some(self.app_config.agent.log_level.clone()),
            ["network", "listen_addr"] => Some(self.app_config.network.listen_addr.clone()),
            ["network", "mesh_degree"] => Some(self.app_config.network.mesh_degree.to_string()),
            ["network", "enable_mdns"] => Some(self.app_config.network.enable_mdns.to_string()),
            ["network", "enable_relay"] => Some(self.app_config.network.enable_relay.to_string()),
            ["storage", "db_path"] => Some(self.app_config.storage.db_path.clone()),
            ["storage", "cache_size"] => Some(self.app_config.storage.cache_size.to_string()),
            ["storage", "cache_ttl_secs"] => {
                Some(self.app_config.storage.cache_ttl_secs.to_string())
            }
            ["tokens", "decay_check_interval_secs"] => {
                Some(self.app_config.tokens.decay_check_interval_secs.to_string())
            }
            ["tokens", "unbonding_period_secs"] => {
                Some(self.app_config.tokens.unbonding_period_secs.to_string())
            }
            _ => None,
        }
    }

    pub fn list_all(&self) -> Vec<(String, String)> {
        vec![
            ("agent.name".into(), self.app_config.agent.name.clone()),
            ("agent.did_method".into(), self.app_config.agent.did_method.clone()),
            ("agent.data_dir".into(), self.app_config.agent.data_dir.clone()),
            ("agent.log_level".into(), self.app_config.agent.log_level.clone()),
            ("network.listen_addr".into(), self.app_config.network.listen_addr.clone()),
            ("network.mesh_degree".into(), self.app_config.network.mesh_degree.to_string()),
            ("network.enable_mdns".into(), self.app_config.network.enable_mdns.to_string()),
            ("network.enable_relay".into(), self.app_config.network.enable_relay.to_string()),
            ("storage.db_path".into(), self.app_config.storage.db_path.clone()),
            ("storage.cache_size".into(), self.app_config.storage.cache_size.to_string()),
            ("storage.cache_ttl_secs".into(), self.app_config.storage.cache_ttl_secs.to_string()),
            (
                "tokens.decay_check_interval_secs".into(),
                self.app_config.tokens.decay_check_interval_secs.to_string(),
            ),
            (
                "tokens.unbonding_period_secs".into(),
                self.app_config.tokens.unbonding_period_secs.to_string(),
            ),
        ]
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
}
