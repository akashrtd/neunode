use std::ops::Deref;
use std::sync::{Arc, Mutex};

use neunode_core::types::Did;
use neunode_identity::keyring::Keyring;
use neunode_storage::db::NeunodeDb;

use crate::config::CliConfig;
use crate::mesh_handle::MeshHandle;

/// Shared state for all `/api/v1/*` handlers.
#[derive(Clone)]
pub struct ApiState {
    pub db: Arc<NeunodeDb>,
    pub active_did: Option<Did>,
    pub active_keyring: Arc<Mutex<Option<Keyring>>>,
    pub mesh_handle: Arc<tokio::sync::RwLock<Option<MeshHandle>>>,
    pub config: CliConfig,
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
    pub fn require_did(&self) -> Result<&Did, super::error::ApiError> {
        self.active_did.as_ref().ok_or(super::error::ApiError::NoIdentity)
    }

    /// Lock the keyring and return a guard that dereferences to `&Keyring`.
    pub fn require_keyring(&self) -> Result<KeyringGuard<'_>, super::error::ApiError> {
        let guard = self.active_keyring.lock().unwrap();
        if guard.is_none() {
            return Err(super::error::ApiError::NoIdentity);
        }
        Ok(KeyringGuard(guard))
    }
}
