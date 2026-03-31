use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Result, TrainingError};
use crate::worker::WorkerId;

/// Compute capabilities advertised by a training provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "bindings/provider_capabilities.ts")]
pub struct ProviderCapabilities {
    /// Number of GPUs available.
    #[ts(type = "number")]
    pub gpu_count: u32,
    /// Total GPU memory in gigabytes.
    pub gpu_memory_gb: f64,
    /// Whether the provider supports bfloat16 training.
    pub supports_bf16: bool,
    /// Largest model (by parameter count) this provider can train.
    #[ts(type = "number")]
    pub max_model_params: u64,
}

/// Current status of a training provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "bindings/provider_status.ts")]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    /// Ready to accept training work.
    Available,
    /// Currently running a training job.
    Busy { job_id: String },
    /// Not responding to heartbeats.
    Offline,
}

/// A registered training provider with capabilities and status.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "bindings/provider_entry.ts")]
pub struct ProviderEntry {
    /// Worker identifier for this provider.
    pub worker_id: WorkerId,
    /// Agent DID (decentralized identifier).
    pub did: String,
    /// Advertised compute capabilities.
    pub capabilities: ProviderCapabilities,
    /// Current availability status.
    pub status: ProviderStatus,
    /// Reputation score from 0.0 to 1.0.
    pub reputation_score: f64,
    /// Last heartbeat timestamp (milliseconds since epoch).
    #[ts(type = "number")]
    pub last_heartbeat_ms: u64,
}

/// Registry tracking training providers and their capabilities.
/// Used by the coordinator to select workers for distributed training jobs.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "bindings/provider_registry.ts")]
pub struct ProviderRegistry {
    providers: HashMap<WorkerId, ProviderEntry>,
}

impl ProviderRegistry {
    /// Create an empty provider registry.
    pub fn new() -> Self {
        Self { providers: HashMap::new() }
    }

    /// Register a new provider or update an existing one.
    /// If a provider with the same `worker_id` exists, it is replaced.
    pub fn register(&mut self, entry: ProviderEntry) -> Result<()> {
        self.providers.insert(entry.worker_id.clone(), entry);
        Ok(())
    }

    /// Remove a provider from the registry.
    /// Returns `PeerUnavailable` if the worker is not registered.
    pub fn unregister(&mut self, worker_id: &WorkerId) -> Result<()> {
        if self.providers.remove(worker_id).is_none() {
            return Err(TrainingError::PeerUnavailable(worker_id.0.clone()));
        }
        Ok(())
    }

    /// Look up a provider by worker ID.
    pub fn get(&self, worker_id: &WorkerId) -> Option<&ProviderEntry> {
        self.providers.get(worker_id)
    }

    /// Update the status of a registered provider.
    /// Returns `PeerUnavailable` if the worker is not registered.
    pub fn update_status(&mut self, worker_id: &WorkerId, status: ProviderStatus) -> Result<()> {
        let entry = self
            .providers
            .get_mut(worker_id)
            .ok_or_else(|| TrainingError::PeerUnavailable(worker_id.0.clone()))?;
        entry.status = status;
        Ok(())
    }

    /// Update the heartbeat timestamp for a registered provider.
    /// Returns `PeerUnavailable` if the worker is not registered.
    pub fn update_heartbeat(&mut self, worker_id: &WorkerId, timestamp_ms: u64) -> Result<()> {
        let entry = self
            .providers
            .get_mut(worker_id)
            .ok_or_else(|| TrainingError::PeerUnavailable(worker_id.0.clone()))?;
        entry.last_heartbeat_ms = timestamp_ms;
        Ok(())
    }

    /// Find providers matching minimum requirements, sorted by reputation descending.
    ///
    /// Filters by GPU count, total GPU memory, and max model parameter support.
    /// Only returns providers with `ProviderStatus::Available`.
    pub fn find_available(
        &self,
        min_gpu_count: u32,
        min_memory_gb: f64,
        min_params: u64,
    ) -> Vec<&ProviderEntry> {
        let mut matches: Vec<&ProviderEntry> = self
            .providers
            .values()
            .filter(|p| {
                matches!(p.status, ProviderStatus::Available)
                    && p.capabilities.gpu_count >= min_gpu_count
                    && p.capabilities.gpu_memory_gb >= min_memory_gb
                    && p.capabilities.max_model_params >= min_params
            })
            .collect();
        matches.sort_by(|a, b| {
            b.reputation_score.partial_cmp(&a.reputation_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        matches
    }

    /// Total number of registered providers.
    pub fn count(&self) -> usize {
        self.providers.len()
    }

    /// Number of providers currently in `Available` status.
    pub fn available_count(&self) -> usize {
        self.providers.values().filter(|p| matches!(p.status, ProviderStatus::Available)).count()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(
        worker_id: &str,
        gpu_count: u32,
        memory_gb: f64,
        reputation: f64,
    ) -> ProviderEntry {
        ProviderEntry {
            worker_id: WorkerId(worker_id.to_string()),
            did: format!("did:neunode:{worker_id}"),
            capabilities: ProviderCapabilities {
                gpu_count,
                gpu_memory_gb: memory_gb,
                supports_bf16: true,
                max_model_params: 7_000_000_000,
            },
            status: ProviderStatus::Available,
            reputation_score: reputation,
            last_heartbeat_ms: 1700000000000,
        }
    }

    // ── Register + Get ─────────────────────────────────────────────────

    #[test]
    fn register_and_get() {
        let mut reg = ProviderRegistry::new();
        let entry = make_entry("worker-1", 4, 80.0, 0.9);
        reg.register(entry.clone()).unwrap();

        let got = reg.get(&WorkerId("worker-1".to_string())).unwrap();
        assert_eq!(got.worker_id, entry.worker_id);
        assert_eq!(got.did, entry.did);
    }

    #[test]
    fn get_missing_returns_none() {
        let reg = ProviderRegistry::new();
        assert!(reg.get(&WorkerId("ghost".to_string())).is_none());
    }

    // ── Register duplicate (update) ────────────────────────────────────

    #[test]
    fn register_duplicate_updates() {
        let mut reg = ProviderRegistry::new();
        reg.register(make_entry("w1", 2, 40.0, 0.5)).unwrap();
        reg.register(make_entry("w1", 8, 160.0, 0.95)).unwrap();

        let got = reg.get(&WorkerId("w1".to_string())).unwrap();
        assert_eq!(got.capabilities.gpu_count, 8);
        assert_eq!(got.reputation_score, 0.95);
        assert_eq!(reg.count(), 1);
    }

    // ── Unregister ─────────────────────────────────────────────────────

    #[test]
    fn unregister_removes_provider() {
        let mut reg = ProviderRegistry::new();
        reg.register(make_entry("w1", 4, 80.0, 0.8)).unwrap();
        reg.unregister(&WorkerId("w1".to_string())).unwrap();
        assert!(reg.get(&WorkerId("w1".to_string())).is_none());
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn unregister_unknown_returns_error() {
        let mut reg = ProviderRegistry::new();
        let err = reg.unregister(&WorkerId("unknown".to_string())).unwrap_err();
        match err {
            TrainingError::PeerUnavailable(id) => assert_eq!(id, "unknown"),
            other => panic!("expected PeerUnavailable, got: {other}"),
        }
    }

    // ── Update Status ──────────────────────────────────────────────────

    #[test]
    fn update_status_succeeds() {
        let mut reg = ProviderRegistry::new();
        reg.register(make_entry("w1", 4, 80.0, 0.8)).unwrap();

        reg.update_status(
            &WorkerId("w1".to_string()),
            ProviderStatus::Busy { job_id: "job-42".to_string() },
        )
        .unwrap();

        let got = reg.get(&WorkerId("w1".to_string())).unwrap();
        assert_eq!(got.status, ProviderStatus::Busy { job_id: "job-42".to_string() });
    }

    #[test]
    fn update_status_unknown_returns_error() {
        let mut reg = ProviderRegistry::new();
        let err =
            reg.update_status(&WorkerId("ghost".to_string()), ProviderStatus::Offline).unwrap_err();
        match err {
            TrainingError::PeerUnavailable(id) => assert_eq!(id, "ghost"),
            other => panic!("expected PeerUnavailable, got: {other}"),
        }
    }

    // ── Update Heartbeat ───────────────────────────────────────────────

    #[test]
    fn update_heartbeat_succeeds() {
        let mut reg = ProviderRegistry::new();
        reg.register(make_entry("w1", 4, 80.0, 0.8)).unwrap();

        reg.update_heartbeat(&WorkerId("w1".to_string()), 1700000099000).unwrap();

        let got = reg.get(&WorkerId("w1".to_string())).unwrap();
        assert_eq!(got.last_heartbeat_ms, 1700000099000);
    }

    #[test]
    fn update_heartbeat_unknown_returns_error() {
        let mut reg = ProviderRegistry::new();
        let err = reg.update_heartbeat(&WorkerId("ghost".to_string()), 1234).unwrap_err();
        match err {
            TrainingError::PeerUnavailable(id) => assert_eq!(id, "ghost"),
            other => panic!("expected PeerUnavailable, got: {other}"),
        }
    }

    // ── Find Available ─────────────────────────────────────────────────

    #[test]
    fn find_available_filters_by_requirements() {
        let mut reg = ProviderRegistry::new();
        reg.register(make_entry("small", 1, 16.0, 0.9)).unwrap();
        reg.register(make_entry("medium", 4, 80.0, 0.7)).unwrap();
        reg.register(make_entry("big", 8, 160.0, 0.8)).unwrap();

        let results = reg.find_available(4, 64.0, 7_000_000_000);
        assert_eq!(results.len(), 2);
        // small has only 1 GPU, should be filtered out
        for p in &results {
            assert!(p.capabilities.gpu_count >= 4);
            assert!(p.capabilities.gpu_memory_gb >= 64.0);
        }
    }

    #[test]
    fn find_available_sorted_by_reputation() {
        let mut reg = ProviderRegistry::new();
        reg.register(make_entry("low", 4, 80.0, 0.3)).unwrap();
        reg.register(make_entry("high", 4, 80.0, 0.95)).unwrap();
        reg.register(make_entry("mid", 4, 80.0, 0.6)).unwrap();

        let results = reg.find_available(1, 0.0, 0);
        assert_eq!(results.len(), 3);
        assert!((results[0].reputation_score - 0.95).abs() < f64::EPSILON);
        assert!((results[1].reputation_score - 0.6).abs() < f64::EPSILON);
        assert!((results[2].reputation_score - 0.3).abs() < f64::EPSILON);
    }

    // ── Count and Available Count ──────────────────────────────────────

    #[test]
    fn count_and_available_count() {
        let mut reg = ProviderRegistry::new();
        assert_eq!(reg.count(), 0);
        assert_eq!(reg.available_count(), 0);

        reg.register(make_entry("w1", 4, 80.0, 0.8)).unwrap();
        reg.register(make_entry("w2", 4, 80.0, 0.7)).unwrap();
        assert_eq!(reg.count(), 2);
        assert_eq!(reg.available_count(), 2);

        reg.update_status(&WorkerId("w1".to_string()), ProviderStatus::Offline).unwrap();
        assert_eq!(reg.count(), 2);
        assert_eq!(reg.available_count(), 1);
    }

    // ── Empty Registry ─────────────────────────────────────────────────

    #[test]
    fn empty_registry_find_available() {
        let reg = ProviderRegistry::new();
        assert!(reg.find_available(1, 0.0, 0).is_empty());
    }

    // ── All Offline ────────────────────────────────────────────────────

    #[test]
    fn find_available_all_offline() {
        let mut reg = ProviderRegistry::new();
        reg.register(make_entry("w1", 8, 160.0, 0.9)).unwrap();
        reg.update_status(&WorkerId("w1".to_string()), ProviderStatus::Offline).unwrap();

        assert_eq!(reg.available_count(), 0);
        assert!(reg.find_available(1, 0.0, 0).is_empty());
    }

    // ── Busy Provider ──────────────────────────────────────────────────

    #[test]
    fn find_available_excludes_busy() {
        let mut reg = ProviderRegistry::new();
        reg.register(make_entry("w1", 4, 80.0, 0.9)).unwrap();
        reg.register(make_entry("w2", 4, 80.0, 0.8)).unwrap();

        reg.update_status(
            &WorkerId("w1".to_string()),
            ProviderStatus::Busy { job_id: "j1".to_string() },
        )
        .unwrap();

        let results = reg.find_available(1, 0.0, 0);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].worker_id.0, "w2");
    }

    // ── Serde Roundtrips ───────────────────────────────────────────────

    #[test]
    fn provider_capabilities_serde_roundtrip() {
        let caps = ProviderCapabilities {
            gpu_count: 8,
            gpu_memory_gb: 160.0,
            supports_bf16: true,
            max_model_params: 70_000_000_000,
        };
        let json = serde_json::to_string(&caps).unwrap();
        let back: ProviderCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(caps, back);
    }

    #[test]
    fn provider_status_serde_roundtrip() {
        let statuses = vec![
            ProviderStatus::Available,
            ProviderStatus::Busy { job_id: "job-1".to_string() },
            ProviderStatus::Offline,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let back: ProviderStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, back);
        }
    }

    #[test]
    fn provider_status_snake_case() {
        let json = serde_json::to_string(&ProviderStatus::Available).unwrap();
        assert!(json.contains("available"), "got: {json}");
        assert!(!json.contains("Available"), "should be snake_case: {json}");

        let json = serde_json::to_string(&ProviderStatus::Offline).unwrap();
        assert!(json.contains("offline"), "got: {json}");
    }

    // ── ts-rs Export ───────────────────────────────────────────────────

    #[test]
    fn ts_export_provider_capabilities() {
        use ts_rs::Config;
        let name = ProviderCapabilities::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_provider_status() {
        use ts_rs::Config;
        let name = ProviderStatus::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_provider_entry() {
        use ts_rs::Config;
        let name = ProviderEntry::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_provider_registry() {
        use ts_rs::Config;
        let name = ProviderRegistry::name(&Config::new());
        assert!(!name.is_empty());
    }

    // ── Default ────────────────────────────────────────────────────────

    #[test]
    fn default_is_empty() {
        let reg = ProviderRegistry::default();
        assert_eq!(reg.count(), 0);
    }
}
