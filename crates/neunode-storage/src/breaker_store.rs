//! Persistent manual safety stops, shared by every economic entry point.
use serde::{Deserialize, Serialize};

use crate::{
    cf,
    db::NeunodeDb,
    error::{Result, StorageError},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum BreakerState {
    Closed,
    Open,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BreakerRecord {
    pub state: BreakerState,
    pub tripped_at: Option<u64>,
    pub trip_count: u64,
}

impl Default for BreakerRecord {
    fn default() -> Self {
        Self { state: BreakerState::Closed, tripped_at: None, trip_count: 0 }
    }
}

pub const NAMES: [&str; 3] = ["token_volume", "reputation", "bounty_drain"];

pub fn load(db: &NeunodeDb, name: &str) -> Result<BreakerRecord> {
    // Preserve the pre-beta key location to enforce already-tripped stops during upgrades.
    Ok(db.get(cf::CF_IDENTITY, &format!("breaker:{name}"))?.unwrap_or_default())
}

pub fn save(db: &NeunodeDb, name: &str, record: &BreakerRecord) -> Result<()> {
    if !NAMES.contains(&name) {
        return Err(StorageError::InvalidKeyFormat("unknown circuit breaker".into()));
    }
    db.put(cf::CF_IDENTITY, &format!("breaker:{name}"), record)
}

pub fn ensure_closed(db: &NeunodeDb, name: &str) -> Result<()> {
    if load(db, name)?.state == BreakerState::Open {
        return Err(StorageError::CircuitBreakerOpen(name.into()));
    }
    Ok(())
}
