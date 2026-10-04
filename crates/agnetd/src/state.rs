use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use neunode_core::types::Did;
use neunode_identity::keyring::Keyring;
use neunode_storage::bounty_store::BountyStore;
use neunode_storage::db::NeunodeDb;
use neunode_storage::feed_store::FeedStore;
use neunode_storage::identity_store::IdentityStore;
use neunode_storage::token_store::TokenStore;

pub struct AppState {
    pub(crate) db: Arc<NeunodeDb>,
    pub(crate) db_path: PathBuf,
    pub config: crate::config::CliConfig,
    pub active_keyring: Option<Keyring>,
    pub active_did: Option<Did>,
    pub(crate) mesh_handle: Option<crate::mesh_handle::MeshHandle>,
}

#[allow(dead_code)]
impl AppState {
    pub fn init(cli: &crate::cli::Cli) -> Result<Self> {
        let config = crate::config::CliConfig::load(cli.config.as_deref())?;
        Self::init_with_config(config, cli.identity.as_deref(), cli.db_path.as_deref())
    }

    pub fn init_from_globals(args: &crate::cli::GlobalArgs) -> Result<Self> {
        let config = crate::config::CliConfig::load(args.config.as_deref())?;
        Self::init_with_config(config, args.identity.as_deref(), args.db_path.as_deref())
    }

    fn init_with_config(
        config: crate::config::CliConfig,
        identity_override: Option<&str>,
        db_path_override: Option<&str>,
    ) -> Result<Self> {
        if db_path_override.is_some_and(|path| path.trim().is_empty()) {
            anyhow::bail!("--db-path cannot be empty");
        }
        let db_path =
            expand_db_path(db_path_override.unwrap_or(config.app_config.storage.db_path.as_str()));
        std::fs::create_dir_all(&db_path)
            .with_context(|| format!("failed to create DB directory {}", db_path.display()))?;

        let db = NeunodeDb::open(&db_path).with_context(|| {
            format!(
                "failed to open database at {}; if another agnetd process owns it, use that \
                 daemon's HTTP API or select an isolated store with --db-path",
                db_path.display()
            )
        })?;

        let selected_identity = identity_override.or(config.active_identity.as_deref());
        let (active_keyring, active_did) = match selected_identity {
            Some(did_str) => {
                let kr = Some(
                    crate::keystore::load(did_str)
                        .with_context(|| format!("failed to unlock identity {did_str}"))?,
                );
                let did = Did(did_str.to_string());
                (kr, Some(did))
            }
            None => (None, None),
        };

        Ok(Self {
            db: Arc::new(db),
            db_path,
            config,
            active_keyring,
            active_did,
            mesh_handle: None,
        })
    }

    pub fn identity_store(&self) -> IdentityStore<'_> {
        IdentityStore::new(&self.db)
    }

    pub fn feed_store(&self) -> FeedStore<'_> {
        FeedStore::new(&self.db)
    }

    pub fn token_store(&self) -> TokenStore<'_> {
        TokenStore::new(&self.db)
    }

    pub fn bounty_store(&self) -> BountyStore<'_> {
        BountyStore::new(&self.db)
    }

    pub fn require_keyring(&self) -> Result<&Keyring> {
        self.active_keyring
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no active identity — run `agnetd identity create`"))
    }

    pub fn require_did(&self) -> Result<&Did> {
        self.active_did
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no active identity — run `agnetd identity create`"))
    }

    pub fn save_config(&mut self) -> Result<()> {
        self.config.save()
    }

    pub fn db(&self) -> &Arc<NeunodeDb> {
        &self.db
    }

    pub fn data_dir(&self) -> PathBuf {
        self.db_path.clone()
    }

    pub(crate) fn set_mesh_handle(&mut self, handle: crate::mesh_handle::MeshHandle) {
        self.mesh_handle = Some(handle);
    }

    pub fn mesh_handle(&self) -> Option<&crate::mesh_handle::MeshHandle> {
        self.mesh_handle.as_ref()
    }

    pub fn require_mesh_handle(&self) -> Result<&crate::mesh_handle::MeshHandle> {
        self.mesh_handle
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("mesh not running — run `agnetd mesh start` first"))
    }
}

/// Expand `~` prefix in a path to the user's home directory.
fn expand_db_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix('~') {
        if rest.is_empty() || rest.starts_with('/') {
            if let Some(home) = dirs::home_dir() {
                return home.join(rest.strip_prefix('/').unwrap_or(rest));
            }
        }
    }
    PathBuf::from(path)
}

/// Return the identity directory for a given DID string.
/// `~/.neunode/identities/<sanitized_did>/`
#[cfg(test)]
fn identity_dir_for_did(did: &str) -> PathBuf {
    let sanitized = did.replace(':', "_");
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".neunode")
        .join("identities")
        .join(sanitized)
}

/// Decode a hex string into a fixed-size 32-byte array.
#[cfg(test)]
fn hex_to_bytes(hex: &str) -> Result<[u8; 32]> {
    if hex.len() != 64 {
        anyhow::bail!("expected 64 hex characters, got {}", hex.len());
    }
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .with_context(|| format!("invalid hex at position {}", i * 2))?;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_to_home() {
        let expanded = expand_db_path("~/data/neunode");
        assert!(!expanded.starts_with("~"));
        assert!(expanded.to_string_lossy().contains("data/neunode"));
    }

    #[test]
    fn expand_no_tilde_unchanged() {
        let expanded = expand_db_path("/var/lib/neunode");
        assert_eq!(expanded, PathBuf::from("/var/lib/neunode"));
    }

    #[test]
    fn expand_bare_tilde() {
        let expanded = expand_db_path("~");
        assert!(!expanded.starts_with("~"));
    }

    #[test]
    fn identity_dir_sanitizes_colons() {
        let dir = identity_dir_for_did("did:neunode:0xabc");
        assert!(dir.to_string_lossy().contains("did_neunode_0xabc"));
    }

    #[test]
    fn hex_to_bytes_valid() {
        let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let bytes = hex_to_bytes(hex).unwrap();
        assert_eq!(bytes[0], 0x01);
        assert_eq!(bytes[1], 0x23);
    }

    #[test]
    fn hex_to_bytes_wrong_length() {
        assert!(hex_to_bytes("abcd").is_err());
    }

    #[test]
    fn hex_to_bytes_invalid_chars() {
        let hex = "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz";
        assert!(hex_to_bytes(hex).is_err());
    }
}
