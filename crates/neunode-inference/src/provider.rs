use std::collections::HashMap;

use neunode_core::types::{Did, Timestamp, TokenAmount};
use serde::{Deserialize, Serialize};

use crate::error::{InferenceError, Result};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct InferenceProvider {
    pub did: Did,
    pub name: String,
    pub endpoint: String,
    pub models: Vec<ModelInfo>,
    pub reputation_score: f64,
    pub stake_amount: TokenAmount,
    pub status: ProviderStatus,
    pub last_heartbeat: Timestamp,
    pub total_requests_served: u64,
    pub avg_latency_ms: u32,
}

impl InferenceProvider {
    pub fn has_model(&self, model_id: &str) -> bool {
        self.models.iter().any(|m| m.id == model_id)
    }

    pub fn find_model(&self, model_id: &str) -> Option<&ModelInfo> {
        self.models.iter().find(|m| m.id == model_id)
    }

    pub fn is_available(&self) -> bool {
        self.status == ProviderStatus::Online || self.status == ProviderStatus::Degraded
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct ModelInfo {
    pub id: String,
    pub base_model: Option<String>,
    pub context_length: u32,
    pub input_price_per_million: TokenAmount,
    pub output_price_per_million: TokenAmount,
    pub capabilities: Vec<String>,
}

impl ModelInfo {
    pub fn has_capability(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }

    pub fn total_price_per_million(&self) -> TokenAmount {
        self.input_price_per_million
            .checked_add(self.output_price_per_million)
            .unwrap_or(TokenAmount(u64::MAX))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Online,
    Degraded,
    Offline,
}

pub struct ProviderRegistry {
    providers: HashMap<Did, InferenceProvider>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { providers: HashMap::new() }
    }

    pub fn register(&mut self, provider: InferenceProvider) -> Result<()> {
        if self.providers.contains_key(&provider.did) {
            return Err(InferenceError::InvalidRequest(format!(
                "provider already registered: {}",
                provider.did
            )));
        }
        self.providers.insert(provider.did.clone(), provider);
        Ok(())
    }

    pub fn deregister(&mut self, did: &Did) -> Result<InferenceProvider> {
        self.providers.remove(did).ok_or(InferenceError::ProviderUnavailable)
    }

    pub fn get(&self, did: &Did) -> Option<&InferenceProvider> {
        self.providers.get(did)
    }

    pub fn get_mut(&mut self, did: &Did) -> Option<&mut InferenceProvider> {
        self.providers.get_mut(did)
    }

    /// Record a provider heartbeat using a server-derived timestamp.
    ///
    /// `now` MUST come from the server's clock (e.g. `SystemTime::now()`),
    /// never from the provider's self-reported timestamp, to prevent
    /// providers from faking liveness or instantly recovering from Offline.
    pub fn update_heartbeat(&mut self, did: &Did, now: Timestamp) -> Result<()> {
        let provider = self.providers.get_mut(did).ok_or(InferenceError::ProviderUnavailable)?;
        provider.last_heartbeat = now;
        if provider.status == ProviderStatus::Offline {
            provider.status = ProviderStatus::Online;
        }
        Ok(())
    }

    /// Record latency measured by a requester (not self-reported by the provider).
    /// Uses exponential moving average (α = 0.3) to smooth individual measurements.
    pub fn record_latency(&mut self, did: &Did, measured_latency_ms: u32) -> Result<()> {
        let provider = self.providers.get_mut(did).ok_or(InferenceError::ProviderUnavailable)?;
        let alpha: f64 = 0.3;
        provider.avg_latency_ms =
            ((alpha * measured_latency_ms as f64) + ((1.0 - alpha) * provider.avg_latency_ms as f64))
                as u32;
        Ok(())
    }

    pub fn providers_for_model(&self, model_id: &str) -> Vec<&InferenceProvider> {
        self.providers.values().filter(|p| p.is_available() && p.has_model(model_id)).collect()
    }

    pub fn is_healthy(&self, did: &Did, now: Timestamp, timeout_secs: u64) -> bool {
        match self.providers.get(did) {
            Some(p) => now.saturating_sub(p.last_heartbeat) <= timeout_secs,
            None => false,
        }
    }

    pub fn remove_stale(&mut self, now: Timestamp, timeout_secs: u64) -> Vec<InferenceProvider> {
        let stale_dids: Vec<Did> = self
            .providers
            .iter()
            .filter(|(_, p)| now.saturating_sub(p.last_heartbeat) > timeout_secs)
            .map(|(did, _)| did.clone())
            .collect();

        let removed: Vec<InferenceProvider> =
            stale_dids.iter().filter_map(|did| self.providers.remove(did)).collect();

        removed
    }

    pub fn all_providers(&self) -> Vec<&InferenceProvider> {
        self.providers.values().collect()
    }

    pub fn online_count(&self) -> usize {
        self.providers.values().filter(|p| p.is_available()).count()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_did(n: u32) -> Did {
        Did(format!("did:neunode:0x{n:040x}"))
    }

    fn test_model(model_id: &str) -> ModelInfo {
        ModelInfo {
            id: model_id.to_string(),
            base_model: Some("llama-3b".to_string()),
            context_length: 4096,
            input_price_per_million: TokenAmount(100),
            output_price_per_million: TokenAmount(200),
            capabilities: vec!["chat".to_string(), "streaming".to_string()],
        }
    }

    fn test_provider(did_num: u32, model_ids: &[&str]) -> InferenceProvider {
        InferenceProvider {
            did: test_did(did_num),
            name: format!("provider-{did_num}"),
            endpoint: format!("https://provider-{did_num}.neunode.io/v1"),
            models: model_ids.iter().map(|m| test_model(m)).collect(),
            reputation_score: 50.0,
            stake_amount: TokenAmount(1000),
            status: ProviderStatus::Online,
            last_heartbeat: 1000,
            total_requests_served: 0,
            avg_latency_ms: 100,
        }
    }

    #[test]
    fn provider_status_serde_roundtrip() {
        for status in [ProviderStatus::Online, ProviderStatus::Degraded, ProviderStatus::Offline] {
            let json = serde_json::to_string(&status).unwrap();
            let back: ProviderStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, back);
        }
    }

    #[test]
    fn model_info_serde_roundtrip() {
        let m = test_model("neunode/llama-3b");
        let json = serde_json::to_string(&m).unwrap();
        let back: ModelInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn model_info_has_capability() {
        let m = test_model("llama-3b");
        assert!(m.has_capability("chat"));
        assert!(m.has_capability("streaming"));
        assert!(!m.has_capability("vision"));
    }

    #[test]
    fn model_info_total_price() {
        let m = test_model("llama-3b");
        assert_eq!(m.total_price_per_million(), TokenAmount(300));
    }

    #[test]
    fn provider_serde_roundtrip() {
        let p = test_provider(1, &["llama-3b"]);
        let json = serde_json::to_string(&p).unwrap();
        let back: InferenceProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn provider_has_model() {
        let p = test_provider(1, &["llama-3b", "gpt-4"]);
        assert!(p.has_model("llama-3b"));
        assert!(p.has_model("gpt-4"));
        assert!(!p.has_model("mistral-7b"));
    }

    #[test]
    fn provider_find_model() {
        let p = test_provider(1, &["llama-3b"]);
        assert!(p.find_model("llama-3b").is_some());
        assert!(p.find_model("gpt-4").is_none());
    }

    #[test]
    fn provider_is_available() {
        let mut p = test_provider(1, &["llama-3b"]);
        assert!(p.is_available());
        p.status = ProviderStatus::Degraded;
        assert!(p.is_available());
        p.status = ProviderStatus::Offline;
        assert!(!p.is_available());
    }

    #[test]
    fn registry_register() {
        let mut reg = ProviderRegistry::new();
        let p = test_provider(1, &["llama-3b"]);
        assert!(reg.register(p).is_ok());
        assert_eq!(reg.all_providers().len(), 1);
    }

    #[test]
    fn registry_register_rejects_duplicate() {
        let mut reg = ProviderRegistry::new();
        reg.register(test_provider(1, &["llama-3b"])).unwrap();
        let result = reg.register(test_provider(1, &["gpt-4"]));
        assert!(result.is_err());
    }

    #[test]
    fn registry_deregister() {
        let mut reg = ProviderRegistry::new();
        reg.register(test_provider(1, &["llama-3b"])).unwrap();
        let removed = reg.deregister(&test_did(1)).unwrap();
        assert_eq!(removed.name, "provider-1");
        assert!(reg.get(&test_did(1)).is_none());
    }

    #[test]
    fn registry_deregister_missing() {
        let mut reg = ProviderRegistry::new();
        let result = reg.deregister(&test_did(99));
        assert!(result.is_err());
    }

    #[test]
    fn registry_get() {
        let mut reg = ProviderRegistry::new();
        reg.register(test_provider(1, &["llama-3b"])).unwrap();
        assert!(reg.get(&test_did(1)).is_some());
        assert!(reg.get(&test_did(2)).is_none());
    }

    #[test]
    fn registry_get_mut() {
        let mut reg = ProviderRegistry::new();
        reg.register(test_provider(1, &["llama-3b"])).unwrap();
        let p = reg.get_mut(&test_did(1)).unwrap();
        p.reputation_score = 90.0;
        assert_eq!(reg.get(&test_did(1)).unwrap().reputation_score, 90.0);
    }

    #[test]
    fn registry_update_heartbeat() {
        let mut reg = ProviderRegistry::new();
        let mut p = test_provider(1, &["llama-3b"]);
        p.status = ProviderStatus::Offline;
        p.last_heartbeat = 0;
        reg.register(p).unwrap();

        // `now` is server-derived, not from the provider
        let server_now: Timestamp = 2000;
        reg.update_heartbeat(&test_did(1), server_now).unwrap();

        let updated = reg.get(&test_did(1)).unwrap();
        assert_eq!(updated.last_heartbeat, 2000);
        assert_eq!(updated.status, ProviderStatus::Online);
    }

    #[test]
    fn registry_update_heartbeat_missing() {
        let mut reg = ProviderRegistry::new();
        let result = reg.update_heartbeat(&test_did(99), 1000);
        assert!(result.is_err());
    }

    #[test]
    fn registry_record_latency_updates_ema() {
        let mut reg = ProviderRegistry::new();
        let mut p = test_provider(1, &["llama-3b"]);
        p.avg_latency_ms = 100;
        reg.register(p).unwrap();

        // Requester measures 200ms actual latency
        reg.record_latency(&test_did(1), 200).unwrap();
        // EMA: 0.3 * 200 + 0.7 * 100 = 130
        assert_eq!(reg.get(&test_did(1)).unwrap().avg_latency_ms, 130);

        // Second measurement of 50ms
        reg.record_latency(&test_did(1), 50).unwrap();
        // EMA: 0.3 * 50 + 0.7 * 130 = 106
        assert_eq!(reg.get(&test_did(1)).unwrap().avg_latency_ms, 106);
    }

    #[test]
    fn registry_record_latency_missing_provider() {
        let mut reg = ProviderRegistry::new();
        let result = reg.record_latency(&test_did(99), 100);
        assert!(result.is_err());
    }

    #[test]
    fn registry_providers_for_model() {
        let mut reg = ProviderRegistry::new();
        reg.register(test_provider(1, &["llama-3b", "gpt-4"])).unwrap();
        reg.register(test_provider(2, &["llama-3b"])).unwrap();
        reg.register(test_provider(3, &["gpt-4"])).unwrap();

        let llama_providers = reg.providers_for_model("llama-3b");
        assert_eq!(llama_providers.len(), 2);

        let gpt_providers = reg.providers_for_model("gpt-4");
        assert_eq!(gpt_providers.len(), 2);

        let none = reg.providers_for_model("nonexistent");
        assert!(none.is_empty());
    }

    #[test]
    fn registry_providers_for_model_excludes_offline() {
        let mut reg = ProviderRegistry::new();
        let mut p1 = test_provider(1, &["llama-3b"]);
        p1.status = ProviderStatus::Offline;
        reg.register(p1).unwrap();
        reg.register(test_provider(2, &["llama-3b"])).unwrap();

        let providers = reg.providers_for_model("llama-3b");
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].did, test_did(2));
    }

    #[test]
    fn registry_is_healthy() {
        let mut reg = ProviderRegistry::new();
        reg.register(test_provider(1, &["llama-3b"])).unwrap();

        assert!(reg.is_healthy(&test_did(1), 1500, 600));
        assert!(!reg.is_healthy(&test_did(1), 2000, 500));
        assert!(!reg.is_healthy(&test_did(99), 1000, 600));
    }

    #[test]
    fn registry_remove_stale() {
        let mut reg = ProviderRegistry::new();
        let mut p1 = test_provider(1, &["llama-3b"]);
        p1.last_heartbeat = 500;
        reg.register(p1).unwrap();

        let mut p2 = test_provider(2, &["llama-3b"]);
        p2.last_heartbeat = 1500;
        reg.register(p2).unwrap();

        reg.register(test_provider(3, &["llama-3b"])).unwrap();

        let stale = reg.remove_stale(1000, 400);
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].did, test_did(1));
        assert_eq!(reg.all_providers().len(), 2);
    }

    #[test]
    fn registry_remove_stale_none() {
        let mut reg = ProviderRegistry::new();
        reg.register(test_provider(1, &["llama-3b"])).unwrap();
        let stale = reg.remove_stale(1500, 600);
        assert!(stale.is_empty());
    }

    #[test]
    fn registry_online_count() {
        let mut reg = ProviderRegistry::new();
        reg.register(test_provider(1, &["llama-3b"])).unwrap();

        let mut p2 = test_provider(2, &["llama-3b"]);
        p2.status = ProviderStatus::Offline;
        reg.register(p2).unwrap();

        assert_eq!(reg.online_count(), 1);
    }

    #[test]
    fn registry_default_is_new() {
        assert_eq!(ProviderRegistry::default().all_providers().len(), 0);
    }
}
