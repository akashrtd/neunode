use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use crate::error::{InferenceError, Result};
use crate::provider::InferenceProvider;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RoutingStrategy {
    Cheapest,
    Fastest,
    HighestReputation,
    Random,
    RoundRobin,
}

pub struct Router {
    strategy: RoutingStrategy,
    round_robin_index: AtomicUsize,
}

impl Router {
    pub fn new(strategy: RoutingStrategy) -> Self {
        Self { strategy, round_robin_index: AtomicUsize::new(0) }
    }

    pub fn route<'a>(
        &self,
        providers: &'a [InferenceProvider],
        model_id: &str,
        seed: Option<u64>,
    ) -> Result<&'a InferenceProvider> {
        let eligible: Vec<&InferenceProvider> =
            providers.iter().filter(|p| p.is_available() && p.has_model(model_id)).collect();

        if eligible.is_empty() {
            return Err(InferenceError::RoutingError(format!(
                "no available providers for model: {model_id}"
            )));
        }

        match self.strategy {
            RoutingStrategy::Cheapest => {
                let best = eligible.iter().copied().min_by(|a, b| {
                    let price_a = a
                        .find_model(model_id)
                        .map(|m| m.total_price_per_million().0)
                        .unwrap_or(u64::MAX);
                    let price_b = b
                        .find_model(model_id)
                        .map(|m| m.total_price_per_million().0)
                        .unwrap_or(u64::MAX);
                    price_a.cmp(&price_b)
                });
                best.ok_or_else(|| {
                    InferenceError::RoutingError("no pricing found for model".to_string())
                })
            }
            RoutingStrategy::Fastest => eligible
                .iter()
                .copied()
                .min_by_key(|p| p.avg_latency_ms)
                .ok_or_else(|| InferenceError::RoutingError("no providers".to_string())),
            RoutingStrategy::HighestReputation => eligible
                .iter()
                .copied()
                .max_by(|a, b| {
                    a.reputation_score
                        .partial_cmp(&b.reputation_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .ok_or_else(|| InferenceError::RoutingError("no providers".to_string())),
            RoutingStrategy::Random => {
                let s = seed.unwrap_or(0);
                let idx = (s as usize) % eligible.len();
                Ok(eligible[idx])
            }
            RoutingStrategy::RoundRobin => {
                let idx = self.round_robin_index.fetch_add(1, Ordering::Relaxed) % eligible.len();
                Ok(eligible[idx])
            }
        }
    }

    pub fn route_top_n<'a>(
        &self,
        providers: &'a [InferenceProvider],
        model_id: &str,
        n: usize,
        seed: Option<u64>,
    ) -> Result<Vec<&'a InferenceProvider>> {
        let mut eligible: Vec<&'a InferenceProvider> =
            providers.iter().filter(|p| p.is_available() && p.has_model(model_id)).collect();

        if eligible.is_empty() {
            return Err(InferenceError::RoutingError(format!(
                "no available providers for model: {model_id}"
            )));
        }

        match self.strategy {
            RoutingStrategy::Cheapest => {
                eligible.sort_by(|a, b| {
                    let price_a = a
                        .find_model(model_id)
                        .map(|m| m.total_price_per_million().0)
                        .unwrap_or(u64::MAX);
                    let price_b = b
                        .find_model(model_id)
                        .map(|m| m.total_price_per_million().0)
                        .unwrap_or(u64::MAX);
                    price_a.cmp(&price_b)
                });
            }
            RoutingStrategy::Fastest => {
                eligible.sort_by_key(|p| p.avg_latency_ms);
            }
            RoutingStrategy::HighestReputation => {
                eligible.sort_by(|a, b| {
                    b.reputation_score
                        .partial_cmp(&a.reputation_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            RoutingStrategy::Random | RoutingStrategy::RoundRobin => {
                let s = seed.unwrap_or(0);
                let start = (s as usize) % eligible.len();
                eligible.rotate_left(start);
            }
        }

        Ok(eligible.into_iter().take(n).collect())
    }

    pub fn set_strategy(&mut self, strategy: RoutingStrategy) {
        self.strategy = strategy;
    }
}

#[cfg(test)]
mod tests {
    use neunode_core::types::{Did, TokenAmount};

    use super::*;
    use crate::provider::{InferenceProvider, ModelInfo, ProviderStatus};

    fn test_did(n: u32) -> Did {
        Did(format!("did:neunode:0x{n:040x}"))
    }

    fn make_model(model_id: &str, input_price: u64, output_price: u64) -> ModelInfo {
        ModelInfo {
            id: model_id.to_string(),
            base_model: None,
            context_length: 4096,
            input_price_per_million: TokenAmount(input_price),
            output_price_per_million: TokenAmount(output_price),
            capabilities: vec!["chat".to_string()],
        }
    }

    fn make_provider(
        did_num: u32,
        model_id: &str,
        input_price: u64,
        output_price: u64,
        rep: f64,
        latency_ms: u32,
    ) -> InferenceProvider {
        InferenceProvider {
            did: test_did(did_num),
            name: format!("provider-{did_num}"),
            endpoint: format!("https://p{did_num}.neunode.io/v1"),
            models: vec![make_model(model_id, input_price, output_price)],
            reputation_score: rep,
            stake_amount: TokenAmount(1000),
            status: ProviderStatus::Online,
            last_heartbeat: 1000,
            total_requests_served: 0,
            avg_latency_ms: latency_ms,
        }
    }

    #[test]
    fn routing_strategy_serde_roundtrip() {
        for strategy in [
            RoutingStrategy::Cheapest,
            RoutingStrategy::Fastest,
            RoutingStrategy::HighestReputation,
            RoutingStrategy::Random,
            RoutingStrategy::RoundRobin,
        ] {
            let json = serde_json::to_string(&strategy).unwrap();
            let back: RoutingStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(strategy, back);
        }
    }

    #[test]
    fn route_empty_providers() {
        let router = Router::new(RoutingStrategy::Cheapest);
        let result = router.route(&[], "llama-3b", None);
        assert!(result.is_err());
    }

    #[test]
    fn route_no_matching_model() {
        let router = Router::new(RoutingStrategy::Cheapest);
        let providers = vec![make_provider(1, "llama-3b", 100, 200, 50.0, 100)];
        let result = router.route(&providers, "gpt-4", None);
        assert!(result.is_err());
    }

    #[test]
    fn route_cheapest() {
        let router = Router::new(RoutingStrategy::Cheapest);
        let providers = vec![
            make_provider(1, "llama-3b", 1000, 2000, 50.0, 100),
            make_provider(2, "llama-3b", 500, 500, 80.0, 200),
            make_provider(3, "llama-3b", 2000, 3000, 90.0, 50),
        ];
        let chosen = router.route(&providers, "llama-3b", None).unwrap();
        assert_eq!(chosen.did, test_did(2));
    }

    #[test]
    fn route_fastest() {
        let router = Router::new(RoutingStrategy::Fastest);
        let providers = vec![
            make_provider(1, "llama-3b", 100, 200, 50.0, 300),
            make_provider(2, "llama-3b", 500, 500, 80.0, 50),
            make_provider(3, "llama-3b", 200, 300, 90.0, 150),
        ];
        let chosen = router.route(&providers, "llama-3b", None).unwrap();
        assert_eq!(chosen.did, test_did(2));
    }

    #[test]
    fn route_highest_reputation() {
        let router = Router::new(RoutingStrategy::HighestReputation);
        let providers = vec![
            make_provider(1, "llama-3b", 100, 200, 30.0, 100),
            make_provider(2, "llama-3b", 500, 500, 95.0, 200),
            make_provider(3, "llama-3b", 200, 300, 60.0, 50),
        ];
        let chosen = router.route(&providers, "llama-3b", None).unwrap();
        assert_eq!(chosen.did, test_did(2));
    }

    #[test]
    fn route_random_deterministic_with_seed() {
        let router = Router::new(RoutingStrategy::Random);
        let providers = vec![
            make_provider(1, "llama-3b", 100, 200, 50.0, 100),
            make_provider(2, "llama-3b", 500, 500, 80.0, 200),
        ];
        let first = router.route(&providers, "llama-3b", Some(0)).unwrap();
        let second = router.route(&providers, "llama-3b", Some(0)).unwrap();
        assert_eq!(first.did, second.did);
    }

    #[test]
    fn route_random_different_seeds() {
        let router = Router::new(RoutingStrategy::Random);
        let providers = vec![
            make_provider(1, "llama-3b", 100, 200, 50.0, 100),
            make_provider(2, "llama-3b", 500, 500, 80.0, 200),
        ];
        let a = router.route(&providers, "llama-3b", Some(0)).unwrap();
        let b = router.route(&providers, "llama-3b", Some(1)).unwrap();
        assert_ne!(a.did, b.did);
    }

    #[test]
    fn route_round_robin_cycles() {
        let router = Router::new(RoutingStrategy::RoundRobin);
        let providers = vec![
            make_provider(1, "llama-3b", 100, 200, 50.0, 100),
            make_provider(2, "llama-3b", 500, 500, 80.0, 200),
            make_provider(3, "llama-3b", 200, 300, 60.0, 50),
        ];
        let first = router.route(&providers, "llama-3b", None).unwrap();
        let second = router.route(&providers, "llama-3b", None).unwrap();
        let third = router.route(&providers, "llama-3b", None).unwrap();
        let fourth = router.route(&providers, "llama-3b", None).unwrap();

        assert_eq!(first.did, test_did(1));
        assert_eq!(second.did, test_did(2));
        assert_eq!(third.did, test_did(3));
        assert_eq!(fourth.did, test_did(1));
    }

    #[test]
    fn route_excludes_offline() {
        let router = Router::new(RoutingStrategy::Cheapest);
        let mut providers = vec![
            make_provider(1, "llama-3b", 100, 200, 50.0, 100),
            make_provider(2, "llama-3b", 500, 500, 80.0, 200),
        ];
        providers[0].status = ProviderStatus::Offline;
        let chosen = router.route(&providers, "llama-3b", None).unwrap();
        assert_eq!(chosen.did, test_did(2));
    }

    #[test]
    fn route_top_n_cheapest() {
        let router = Router::new(RoutingStrategy::Cheapest);
        let providers = vec![
            make_provider(1, "llama-3b", 1000, 2000, 50.0, 300),
            make_provider(2, "llama-3b", 500, 500, 80.0, 200),
            make_provider(3, "llama-3b", 2000, 3000, 90.0, 50),
        ];
        let top2 = router.route_top_n(&providers, "llama-3b", 2, None).unwrap();
        assert_eq!(top2.len(), 2);
        assert_eq!(top2[0].did, test_did(2));
        assert_eq!(top2[1].did, test_did(1));
    }

    #[test]
    fn route_top_n_more_than_available() {
        let router = Router::new(RoutingStrategy::Fastest);
        let providers = vec![
            make_provider(1, "llama-3b", 100, 200, 50.0, 100),
            make_provider(2, "llama-3b", 500, 500, 80.0, 200),
        ];
        let top5 = router.route_top_n(&providers, "llama-3b", 5, None).unwrap();
        assert_eq!(top5.len(), 2);
    }

    #[test]
    fn route_top_n_empty() {
        let router = Router::new(RoutingStrategy::Cheapest);
        let result = router.route_top_n(&[], "llama-3b", 3, None);
        assert!(result.is_err());
    }

    #[test]
    fn set_strategy_changes_behavior() {
        let mut router = Router::new(RoutingStrategy::Cheapest);
        let providers = vec![
            make_provider(1, "llama-3b", 100, 200, 30.0, 300),
            make_provider(2, "llama-3b", 500, 500, 95.0, 50),
        ];

        let cheapest = router.route(&providers, "llama-3b", None).unwrap();
        assert_eq!(cheapest.did, test_did(1));

        router.set_strategy(RoutingStrategy::HighestReputation);
        let best_rep = router.route(&providers, "llama-3b", None).unwrap();
        assert_eq!(best_rep.did, test_did(2));
    }

    #[test]
    fn route_top_n_reputation_order() {
        let router = Router::new(RoutingStrategy::HighestReputation);
        let providers = vec![
            make_provider(1, "llama-3b", 100, 200, 30.0, 100),
            make_provider(2, "llama-3b", 500, 500, 95.0, 200),
            make_provider(3, "llama-3b", 200, 300, 60.0, 50),
        ];
        let top = router.route_top_n(&providers, "llama-3b", 3, None).unwrap();
        assert_eq!(top[0].did, test_did(2));
        assert_eq!(top[1].did, test_did(3));
        assert_eq!(top[2].did, test_did(1));
    }
}
