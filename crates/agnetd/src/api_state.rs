use std::ops::Deref;
use std::sync::{Arc, Mutex, RwLock};

use neunode_core::types::Did;
use neunode_identity::keyring::Keyring;
use neunode_storage::db::NeunodeDb;

use crate::config::CliConfig;
use crate::mesh_handle::MeshHandle;

/// Shared state for all `/api/v1/*` handlers.
#[derive(Clone)]
pub struct ApiState {
    pub db: Arc<NeunodeDb>,
    pub active_did: Arc<RwLock<Option<Did>>>,
    pub active_keyring: Arc<Mutex<Option<Keyring>>>,
    pub mesh_handle: Arc<tokio::sync::RwLock<Option<MeshHandle>>>,
    pub(crate) config: Arc<RwLock<CliConfig>>,
    #[allow(dead_code)]
    pub feed_tx: tokio::sync::broadcast::Sender<FeedEventUpdate>,
}

#[derive(Clone, serde::Serialize, utoipa::ToSchema)]
pub struct FeedEventUpdate {
    pub kind: u16,
    pub author_did: String,
    pub author_short: String,
    pub kind_label: String,
    pub preview: String,
    pub time_ago: String,
}

/// RAII guard that dereferences to `Keyring`.
pub struct KeyringGuard<'a>(std::sync::MutexGuard<'a, Option<Keyring>>);

impl<'a> Deref for KeyringGuard<'a> {
    type Target = Keyring;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}

impl ApiState {
    pub fn require_did(&self) -> Result<Did, super::error::ApiError> {
        self.active_did
            .read()
            .map_err(|_| super::error::ApiError::Internal("identity lock poisoned".into()))?
            .clone()
            .ok_or(super::error::ApiError::NoIdentity)
    }

    /// Lock the keyring and return a guard that dereferences to `&Keyring`.
    pub fn require_keyring(&self) -> Result<KeyringGuard<'_>, super::error::ApiError> {
        let guard = self.active_keyring.lock().unwrap();
        if guard.is_none() {
            return Err(super::error::ApiError::NoIdentity);
        }
        Ok(KeyringGuard(guard))
    }

    pub async fn start_mesh(&self) -> Result<(), super::error::ApiError> {
        let mut mesh = self.mesh_handle.write().await;
        if mesh.is_some() {
            return Ok(());
        }
        let config = self.config_snapshot()?;
        let keypair = {
            let keyring = match self.require_keyring() {
                Ok(keys) => keys,
                Err(super::error::ApiError::NoIdentity) => return Ok(()),
                Err(error) => return Err(error),
            };
            let (ed, _) = keyring.to_bytes();
            libp2p::identity::Keypair::ed25519_from_bytes(ed)
                .map_err(|error| super::error::ApiError::Internal(error.to_string()))?
        };
        let network = &config.app_config.network;
        let listen = network.listen_addr.parse().map_err(|error| {
            super::error::ApiError::BadRequest(format!("invalid network listener: {error}"))
        })?;
        let peers = network
            .bootstrap_peers
            .iter()
            .map(|peer| peer.parse())
            .collect::<Result<Vec<libp2p::Multiaddr>, _>>()
            .map_err(|error| super::error::ApiError::BadRequest(error.to_string()))?;
        let data = &config.app_config.agent.data_dir;
        let path = if let Some(rest) = data.strip_prefix("~/") {
            dirs::home_dir()
                .ok_or_else(|| {
                    super::error::ApiError::Internal("home directory unavailable".into())
                })?
                .join(rest)
        } else {
            std::path::PathBuf::from(data)
        };
        let mut handle = crate::mesh_handle::spawn_mesh_task(
            keypair,
            listen,
            peers,
            true,
            Arc::clone(&self.db),
            path,
        )?;
        if let Some(mut events) = handle.take_event_stream() {
            let feed_tx = self.feed_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = events.recv().await {
                    let _ = feed_tx.send(FeedEventUpdate {
                        kind: event.kind.as_u16(),
                        author_did: event.author.0.clone(),
                        author_short: event.author.0.chars().take(18).collect(),
                        kind_label: event.kind.as_u16().to_string(),
                        preview: event.content.chars().take(80).collect(),
                        time_ago: "now".into(),
                    });
                }
            });
        }
        *mesh = Some(handle);
        Ok(())
    }

    pub fn config_snapshot(&self) -> Result<CliConfig, super::error::ApiError> {
        self.config
            .read()
            .map(|config| config.clone())
            .map_err(|_| super::error::ApiError::Internal("config lock is poisoned".to_string()))
    }

    pub fn set_config_value(&self, key: &str, value: &str) -> Result<(), super::error::ApiError> {
        if key == "active_identity" {
            let replacement =
                if value.is_empty() { None } else { Some(crate::keystore::load(value)?) };
            let mut active = self
                .active_keyring
                .lock()
                .map_err(|_| super::error::ApiError::Internal("identity lock poisoned".into()))?;
            let mesh = self
                .mesh_handle
                .try_read()
                .map_err(|_| super::error::ApiError::BadRequest("mesh is busy".into()))?;
            if mesh.is_some()
                && active.as_ref().map(|keys| keys.to_did().0).as_deref() != Some(value)
            {
                return Err(super::error::ApiError::BadRequest(
                    "stop the daemon before selecting another network identity".into(),
                ));
            }
            let mut config = self
                .config
                .write()
                .map_err(|_| super::error::ApiError::Internal("config lock poisoned".into()))?;
            let mut candidate = config.clone();
            candidate.set(key, value)?;
            candidate.save()?;
            *config = candidate;
            *self
                .active_did
                .write()
                .map_err(|_| super::error::ApiError::Internal("identity lock poisoned".into()))? =
                replacement.as_ref().map(Keyring::to_did);
            *active = replacement;
            return Ok(());
        }
        let mut live = self
            .config
            .write()
            .map_err(|_| super::error::ApiError::Internal("config lock is poisoned".to_string()))?;
        let mut candidate = live.clone();
        candidate
            .set(key, value)
            .map_err(|error| super::error::ApiError::BadRequest(error.to_string()))?;
        candidate.save().map_err(|error| super::error::ApiError::Internal(error.to_string()))?;
        *live = candidate;
        Ok(())
    }
}
