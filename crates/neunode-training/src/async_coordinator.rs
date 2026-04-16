use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use ts_rs::TS;

use crate::config::TrainingConfig;
use crate::coordinator::MomentumBuffer;
use crate::error::{Result, TrainingError};
use crate::fault::{FaultEvent, HealthMonitor};
use crate::worker::{LocalRunResult, WorkerId};

/// Returns current time in milliseconds since Unix epoch.
fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

/// Staleness weighting strategy for async gradient aggregation.
///
/// Based on DeepMind Async Local-SGD (arxiv 2401.09135).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum StalenessWeight {
    /// Constant weight: 1/√K regardless of staleness.
    Constant,
    /// Polynomial decay: 1/√K × 1/(1+τ)^0.5 (DeepMind default).
    Polynomial,
}

/// Status of the async training coordinator.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum AsyncCoordinatorStatus {
    /// No active round.
    Idle,
    /// Collecting gradients from workers.
    Collecting,
    /// Aggregating gradients and computing parameter update.
    Aggregating,
    /// Distributing updated parameters to workers.
    Distributing,
    /// Training completed.
    Completed,
    /// Coordinator hit an unrecoverable error.
    Failed,
}

/// Result of an async aggregation step with staleness information.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "bindings/async_aggregation_result.ts")]
pub struct AsyncAggregationResult {
    /// Staleness-weighted averaged pseudo-gradient.
    pub averaged_gradient: Vec<f32>,
    /// Number of workers that contributed.
    #[ts(type = "number")]
    pub num_workers: u32,
    /// Outer step number (1-indexed after first aggregation).
    #[ts(type = "number")]
    pub outer_step: u32,
    /// Average loss across contributing workers.
    pub avg_loss: f64,
    /// Parameter update applied (velocity after momentum).
    pub parameter_update: Vec<f32>,
    /// Per-worker staleness values (τ = outer_step - global_step_at_start).
    pub stalenesses: Vec<u32>,
    /// Number of gradients dropped this round due to staleness.
    #[ts(type = "number")]
    pub dropped_count: u32,
}

/// Async training coordinator for Async Local-SGD (Async DiLoCo).
///
/// Replaces the synchronous barrier in [`TrainingCoordinator`] with a
/// tokio::select! event loop. Workers send gradients asynchronously via
/// an mpsc channel; the coordinator aggregates when a quorum is reached
/// or a timeout fires.
pub struct AsyncCoordinator {
    config: TrainingConfig,
    status: AsyncCoordinatorStatus,
    current_params: Vec<f32>,
    momentum: MomentumBuffer,
    outer_step: u32,
    rx: mpsc::Receiver<LocalRunResult>,
    buffer: HashMap<String, LocalRunResult>,
    health_monitor: HealthMonitor,
    collection_start: Option<Instant>,
    staleness_weight: StalenessWeight,
    dropped_count: u32,
}

impl std::fmt::Debug for AsyncCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncCoordinator")
            .field("status", &self.status)
            .field("outer_step", &self.outer_step)
            .field("param_count", &self.current_params.len())
            .field("buffer_len", &self.buffer.len())
            .field("staleness_weight", &self.staleness_weight)
            .finish()
    }
}

impl AsyncCoordinator {
    /// Create a new async coordinator with initial model parameters.
    ///
    /// Returns the coordinator and an mpsc sender for workers to submit
    /// their local run results.
    pub fn new(
        config: TrainingConfig,
        initial_params: Vec<f32>,
    ) -> Result<(Self, mpsc::Sender<LocalRunResult>)> {
        config.validate()?;
        if initial_params.is_empty() {
            return Err(TrainingError::ConfigInvalid(
                "initial params must be non-empty".to_string(),
            ));
        }
        let param_count = initial_params.len();
        let capacity = config.max_workers as usize * 2;
        let (tx, rx) = mpsc::channel(capacity);
        let momentum = MomentumBuffer::new(param_count, config.outer_momentum);
        let health_monitor = HealthMonitor::new(config.heartbeat_timeout_secs);

        Ok((
            Self {
                config,
                status: AsyncCoordinatorStatus::Idle,
                current_params: initial_params,
                momentum,
                outer_step: 0,
                rx,
                buffer: HashMap::new(),
                health_monitor,
                collection_start: None,
                staleness_weight: StalenessWeight::Polynomial,
                dropped_count: 0,
            },
            tx,
        ))
    }

    /// Register a worker for health monitoring.
    pub fn register_worker(&mut self, worker_id: WorkerId) {
        self.health_monitor.register(worker_id, now_ms());
    }

    /// Remove a worker from health monitoring.
    pub fn remove_worker(&mut self, worker_id: &WorkerId) {
        self.health_monitor.remove(worker_id);
    }

    /// Get current model parameters.
    pub fn parameters(&self) -> &[f32] {
        &self.current_params
    }

    /// Get current outer step number.
    pub fn outer_step(&self) -> u32 {
        self.outer_step
    }

    /// Get coordinator status.
    pub fn status(&self) -> AsyncCoordinatorStatus {
        self.status
    }

    /// Process a single gradient result: check staleness, buffer it.
    fn process_gradient(&mut self, result: LocalRunResult) {
        let staleness = self.outer_step.saturating_sub(result.global_step_at_start);

        // Check staleness cap (max_staleness == 0 means unlimited).
        if self.config.max_staleness > 0 && staleness > self.config.max_staleness {
            tracing::warn!(
                worker_id = %result.worker_id.0,
                staleness,
                max_allowed = self.config.max_staleness,
                "dropping stale gradient"
            );
            self.dropped_count += 1;
            return;
        }

        // Validate gradient dimension.
        if result.pseudo_gradients.len() != self.current_params.len() {
            tracing::error!(
                worker_id = %result.worker_id.0,
                expected = self.current_params.len(),
                got = result.pseudo_gradients.len(),
                "gradient dimension mismatch, dropping"
            );
            return;
        }

        self.status = AsyncCoordinatorStatus::Collecting;

        // Record heartbeat for this worker.
        self.health_monitor.heartbeat(&result.worker_id, now_ms());

        // Buffer the result (latest from each worker wins).
        self.buffer.insert(result.worker_id.0.clone(), result);

        // Mark collection start on first result.
        if self.collection_start.is_none() {
            self.collection_start = Some(Instant::now());
        }
    }

    /// Run the main event loop.
    ///
    /// Processes gradients, health checks, and collection timeouts
    /// until all senders are dropped (channel closed).
    pub async fn run(&mut self) -> Result<()> {
        let collection_dur = Duration::from_secs(self.config.collection_timeout_secs);
        let health_dur = Duration::from_secs(self.config.heartbeat_timeout_secs);
        let mut collection_interval = tokio::time::interval(collection_dur);
        let mut health_interval = tokio::time::interval(health_dur);

        // Consume the first immediate tick from both intervals.
        collection_interval.tick().await;
        health_interval.tick().await;

        loop {
            tokio::select! {
                biased;

                // Branch 1 — Gradient reception (highest priority).
                result = self.rx.recv() => {
                    let result = match result {
                        Some(r) => r,
                        None => {
                            tracing::info!(
                                "all workers disconnected, shutting down"
                            );
                            break;
                        }
                    };

                    self.process_gradient(result);

                    // Drain loop (Sui pattern): process buffered messages.
                    while let Ok(r) = self.rx.try_recv() {
                        self.process_gradient(r);
                    }

                    // Check if quorum reached.
                    if self.buffer.len()
                        >= self.config.min_workers as usize
                    {
                        let _ = self.aggregate_and_step();
                    }
                }

                // Branch 2 — Health check.
                _ = health_interval.tick() => {
                    let t = now_ms();
                    let events = self.health_monitor.check(t);
                    for event in events {
                        match &event {
                            FaultEvent::WorkerDead { worker_id } => {
                                tracing::warn!(
                                    worker_id = %worker_id.0,
                                    "worker dead, removing from buffer"
                                );
                                self.buffer.remove(&worker_id.0);
                            }
                            FaultEvent::RecoveryNeeded { .. } => {
                                tracing::warn!("recovery needed");
                            }
                            FaultEvent::WorkerRecovered { worker_id } => {
                                tracing::info!(
                                    worker_id = %worker_id.0,
                                    "worker recovered"
                                );
                            }
                            FaultEvent::WorkerSuspect { worker_id } => {
                                tracing::warn!(
                                    worker_id = %worker_id.0,
                                    "worker suspect"
                                );
                            }
                        }
                    }
                }

                // Branch 3 — Collection timeout.
                _ = collection_interval.tick() => {
                    let min = self.config.min_workers as usize;
                    if self.buffer.len() >= min {
                        let _ = self.aggregate_and_step();
                    } else if !self.buffer.is_empty() {
                        if let Some(start) = self.collection_start {
                            let elapsed = start.elapsed();
                            let grace = Duration::from_secs(
                                self.config.grace_period_secs,
                            );
                            if elapsed > grace {
                                tracing::warn!(
                                    available = self.buffer.len(),
                                    required = min,
                                    elapsed_secs = elapsed.as_secs(),
                                    "grace period expired, aggregating"
                                );
                                let _ = self.aggregate_and_step();
                            }
                        }
                    } else {
                        tracing::debug!(
                            "timer fired but quorum not reached"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Aggregate buffered gradients with staleness weighting and update params.
    ///
    /// 1. Compute staleness-weighted average pseudo-gradient.
    /// 2. Apply momentum: `v = momentum * v + averaged_gradient`
    /// 3. Update params: `θ -= outer_lr * v`
    pub fn aggregate_and_step(&mut self) -> Result<AsyncAggregationResult> {
        if self.buffer.is_empty() {
            return Err(TrainingError::AggregationFailed("no results to aggregate".to_string()));
        }
        self.status = AsyncCoordinatorStatus::Aggregating;

        let param_count = self.current_params.len();
        let num_workers = self.buffer.len() as u32;
        let k = num_workers as f32;
        let base_weight = k.sqrt() / k;

        // Compute staleness-weighted average.
        let mut averaged = vec![0.0f32; param_count];
        let mut total_weight = 0.0f32;
        let mut total_loss = 0.0f64;
        let mut stalenesses = Vec::with_capacity(self.buffer.len());

        for result in self.buffer.values() {
            let staleness = self.outer_step.saturating_sub(result.global_step_at_start);
            stalenesses.push(staleness);

            let weight = match self.staleness_weight {
                StalenessWeight::Constant => base_weight,
                StalenessWeight::Polynomial => base_weight / (1.0 + staleness as f32).sqrt(),
            };
            total_weight += weight;

            for (i, &g) in result.pseudo_gradients.iter().enumerate() {
                averaged[i] += weight * g;
            }
            total_loss += result.step_losses.last().copied().unwrap_or(0.0);
        }

        // Normalize by total weight.
        if total_weight > 0.0 {
            let inv_w = 1.0 / total_weight;
            for v in averaged.iter_mut() {
                *v *= inv_w;
            }
        }

        let avg_loss = total_loss / num_workers as f64;

        // Apply momentum: v = momentum * v + gradient.
        let parameter_update = self.momentum.update(&averaged);

        // Update parameters: θ -= outer_lr * v.
        let lr = self.config.outer_lr as f32;
        for (i, &update) in parameter_update.iter().enumerate() {
            self.current_params[i] -= lr * update;
        }

        self.outer_step += 1;
        self.status = AsyncCoordinatorStatus::Distributing;

        let result = AsyncAggregationResult {
            averaged_gradient: averaged,
            num_workers,
            outer_step: self.outer_step,
            avg_loss,
            parameter_update,
            stalenesses,
            dropped_count: self.dropped_count,
        };

        // Reset for next round.
        self.buffer.clear();
        self.collection_start = None;
        self.dropped_count = 0;
        self.status = AsyncCoordinatorStatus::Idle;

        Ok(result)
    }

    /// Gracefully shut down the coordinator.
    ///
    /// Attempts final aggregation if buffered results exist,
    /// then sets status to Completed.
    pub fn shutdown(&mut self) {
        if !self.buffer.is_empty() {
            let _ = self.aggregate_and_step();
        }
        self.status = AsyncCoordinatorStatus::Completed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fault::HealthState;

    /// Helper: valid TrainingConfig for async tests.
    fn async_test_config() -> TrainingConfig {
        TrainingConfig {
            local_steps: 10,
            inner_lr: 0.01,
            outer_lr: 0.7,
            outer_momentum: 0.9,
            batch_size: 32,
            quantization_bits: 8,
            max_workers: 4,
            heartbeat_timeout_secs: 6,
            checkpoint_interval: 10,
            async_mode: true,
            min_workers: 2,
            max_staleness: 5,
            grace_period_secs: 2,
            collection_timeout_secs: 60,
        }
    }

    /// Helper: config with short timeouts for timeout tests.
    fn short_timeout_config() -> TrainingConfig {
        TrainingConfig {
            local_steps: 10,
            inner_lr: 0.01,
            outer_lr: 0.7,
            outer_momentum: 0.9,
            batch_size: 32,
            quantization_bits: 8,
            max_workers: 4,
            heartbeat_timeout_secs: 60,
            checkpoint_interval: 10,
            async_mode: true,
            min_workers: 3,
            max_staleness: 0,
            grace_period_secs: 1,
            collection_timeout_secs: 1,
        }
    }

    /// Helper: config for single-worker aggregation.
    fn single_worker_config() -> TrainingConfig {
        TrainingConfig { min_workers: 1, max_workers: 1, ..async_test_config() }
    }

    /// Helper: create async coordinator with params all 1.0.
    fn make_async_coordinator(
        param_count: usize,
    ) -> (AsyncCoordinator, mpsc::Sender<LocalRunResult>) {
        let params = vec![1.0; param_count];
        AsyncCoordinator::new(async_test_config(), params).unwrap()
    }

    /// Helper: build a LocalRunResult for testing.
    fn make_async_result(worker_id: &str, gradients: Vec<f32>, global_step: u32) -> LocalRunResult {
        LocalRunResult {
            worker_id: WorkerId(worker_id.to_string()),
            pseudo_gradients: gradients,
            step_losses: vec![1.0],
            steps_completed: 10,
            completed: true,
            started_at_ms: 1000,
            completed_at_ms: 2000,
            global_step_at_start: global_step,
        }
    }

    // ── Creation tests ───────────────────────────────────────────────

    #[test]
    fn async_coordinator_new() {
        let (coord, _tx) = make_async_coordinator(3);
        assert_eq!(coord.status(), AsyncCoordinatorStatus::Idle);
        assert_eq!(coord.outer_step(), 0);
        assert_eq!(coord.parameters(), &[1.0, 1.0, 1.0]);
    }

    #[test]
    fn async_coordinator_new_empty_params() {
        let result = AsyncCoordinator::new(async_test_config(), vec![]);
        assert!(result.is_err());
        match result.unwrap_err() {
            TrainingError::ConfigInvalid(msg) => {
                assert!(msg.contains("non-empty"));
            }
            other => panic!("expected ConfigInvalid, got {other}"),
        }
    }

    #[test]
    fn async_coordinator_channel_works() {
        let (mut coord, tx) = make_async_coordinator(3);
        let result = make_async_result("w1", vec![0.1, 0.2, 0.3], 0);
        tx.try_send(result).unwrap();
        let received = coord.rx.try_recv().unwrap();
        assert_eq!(received.worker_id.0, "w1");
    }

    // ── StalenessWeight tests ────────────────────────────────────────

    #[test]
    fn staleness_weight_constant() {
        // With Constant weight, all workers get the same weight
        // regardless of staleness. The result should equal simple mean.
        let (mut coord, _tx) = make_async_coordinator(3);
        coord.staleness_weight = StalenessWeight::Constant;
        coord.outer_step = 5;

        let r1 = make_async_result("w1", vec![0.2, 0.4, 0.6], 0);
        let r2 = make_async_result("w2", vec![0.8, 0.6, 0.4], 5);
        coord.buffer.insert("w1".to_string(), r1);
        coord.buffer.insert("w2".to_string(), r2);

        let agg = coord.aggregate_and_step().unwrap();
        // Constant weight = simple average: [0.5, 0.5, 0.5]
        let expected = [0.5, 0.5, 0.5];
        for (a, b) in agg.averaged_gradient.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-5, "{a} != {b}");
        }
    }

    #[test]
    fn staleness_weight_polynomial() {
        // With Polynomial weight, stale workers get lower weight.
        let (mut coord, _tx) = make_async_coordinator(3);
        coord.outer_step = 5;

        // w1: staleness=5 (high), w2: staleness=0 (fresh)
        let r1 = make_async_result("w1", vec![0.2, 0.4, 0.6], 0);
        let r2 = make_async_result("w2", vec![0.8, 0.6, 0.4], 5);
        coord.buffer.insert("w1".to_string(), r1);
        coord.buffer.insert("w2".to_string(), r2);

        let agg = coord.aggregate_and_step().unwrap();
        // Result should be closer to w2 (staleness=0).
        let d_to_w1 = (agg.averaged_gradient[0] - 0.2).abs();
        let d_to_w2 = (agg.averaged_gradient[0] - 0.8).abs();
        assert!(d_to_w2 < d_to_w1, "should be closer to fresh worker: {:?}", agg.averaged_gradient);
    }

    // ── Serde/ts-rs tests ───────────────────────────────────────────

    #[test]
    fn async_coordinator_status_serde() {
        let variants = [
            AsyncCoordinatorStatus::Idle,
            AsyncCoordinatorStatus::Collecting,
            AsyncCoordinatorStatus::Aggregating,
            AsyncCoordinatorStatus::Distributing,
            AsyncCoordinatorStatus::Completed,
            AsyncCoordinatorStatus::Failed,
        ];
        for status in variants {
            let json = serde_json::to_string(&status).unwrap();
            let back: AsyncCoordinatorStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, back);
        }
    }

    #[test]
    fn async_aggregation_result_serde() {
        let agg = AsyncAggregationResult {
            averaged_gradient: vec![0.1, -0.2, 0.3],
            num_workers: 3,
            outer_step: 7,
            avg_loss: 0.42,
            parameter_update: vec![0.15, -0.1, 0.25],
            stalenesses: vec![0, 2, 5],
            dropped_count: 1,
        };
        let json = serde_json::to_string(&agg).unwrap();
        let back: AsyncAggregationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(agg.averaged_gradient, back.averaged_gradient);
        assert_eq!(agg.num_workers, back.num_workers);
        assert_eq!(agg.outer_step, back.outer_step);
        assert!((agg.avg_loss - back.avg_loss).abs() < f64::EPSILON);
        assert_eq!(agg.parameter_update, back.parameter_update);
        assert_eq!(agg.stalenesses, back.stalenesses);
        assert_eq!(agg.dropped_count, back.dropped_count);
    }

    #[test]
    fn async_coordinator_status_snake_case() {
        let json = serde_json::to_string(&AsyncCoordinatorStatus::Collecting).unwrap();
        assert!(json.contains("collecting"), "got: {json}");
        assert!(!json.contains("Collecting"), "should be snake_case: {json}");
    }

    // ── Staleness computation tests ─────────────────────────────────

    #[test]
    fn staleness_zero_when_current_step() {
        let (mut coord, _tx) = make_async_coordinator(3);
        coord.outer_step = 5;
        let result = make_async_result("w1", vec![0.1, 0.2, 0.3], 5);
        coord.process_gradient(result);
        assert_eq!(coord.buffer.len(), 1);
        // staleness = 5 - 5 = 0, accepted
    }

    #[test]
    fn staleness_positive_when_behind() {
        let (mut coord, _tx) = make_async_coordinator(3);
        coord.outer_step = 5;
        let result = make_async_result("w1", vec![0.1, 0.2, 0.3], 2);
        coord.process_gradient(result);
        assert_eq!(coord.buffer.len(), 1);
        // staleness = 5 - 2 = 3 < max_staleness(5), accepted
    }

    #[test]
    fn staleness_drops_exceeding_max() {
        let (mut coord, _tx) = make_async_coordinator(3);
        coord.outer_step = 20;
        // staleness = 20 - 0 = 20 > max_staleness(5)
        let result = make_async_result("w1", vec![0.1, 0.2, 0.3], 0);
        coord.process_gradient(result);
        assert!(coord.buffer.is_empty());
        assert_eq!(coord.dropped_count, 1);
    }

    // ── Aggregation tests ───────────────────────────────────────────

    #[test]
    fn aggregate_single_worker() {
        let config = single_worker_config();
        let params = vec![1.0, 1.0, 1.0];
        let (mut coord, _tx) = AsyncCoordinator::new(config, params).unwrap();

        let r = make_async_result("w1", vec![0.1, 0.2, 0.3], 0);
        coord.buffer.insert("w1".to_string(), r);

        let agg = coord.aggregate_and_step().unwrap();
        assert_eq!(agg.num_workers, 1);
        assert_eq!(agg.outer_step, 1);
        assert_eq!(agg.averaged_gradient, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn aggregate_two_workers_staleness_weighted() {
        let (mut coord, _tx) = make_async_coordinator(3);
        coord.outer_step = 5;

        // w1: staleness=5, w2: staleness=0
        let r1 = make_async_result("w1", vec![0.2, 0.4, 0.6], 0);
        let r2 = make_async_result("w2", vec![0.8, 0.6, 0.4], 5);
        coord.buffer.insert("w1".to_string(), r1);
        coord.buffer.insert("w2".to_string(), r2);

        let agg = coord.aggregate_and_step().unwrap();
        assert_eq!(agg.num_workers, 2);
        // Verify stalenesses recorded.
        assert!(agg.stalenesses.contains(&5));
        assert!(agg.stalenesses.contains(&0));
        // Weighted result is NOT the simple mean [0.5, 0.5, 0.5].
        assert!((agg.averaged_gradient[0] - 0.5).abs() > 0.01, "should differ from simple average");
    }

    #[test]
    fn aggregate_applies_momentum() {
        let (mut coord, _tx) = make_async_coordinator(3);

        // Round 1.
        let r1 = make_async_result("w1", vec![0.1, 0.2, 0.3], 0);
        coord.buffer.insert("w1".to_string(), r1);
        let agg1 = coord.aggregate_and_step().unwrap();

        // Round 2 with different worker.
        let r2 = make_async_result("w2", vec![0.1, 0.2, 0.3], 1);
        coord.buffer.insert("w2".to_string(), r2);
        let agg2 = coord.aggregate_and_step().unwrap();

        // Velocity norm should be larger in round 2 (momentum accumulates).
        let v1_norm: f32 = agg1.parameter_update.iter().map(|v| v * v).sum::<f32>().sqrt();
        let v2_norm: f32 = agg2.parameter_update.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!(v2_norm > v1_norm, "momentum should accumulate");
    }

    #[test]
    fn aggregate_updates_params() {
        let (mut coord, _tx) = make_async_coordinator(3);
        let initial = coord.parameters().to_vec();

        let r = make_async_result("w1", vec![0.1, 0.2, 0.3], 0);
        coord.buffer.insert("w1".to_string(), r);
        coord.aggregate_and_step().unwrap();

        assert_ne!(coord.parameters(), initial.as_slice());
    }

    #[test]
    fn aggregate_clears_buffer() {
        let (mut coord, _tx) = make_async_coordinator(3);
        let r1 = make_async_result("w1", vec![0.1, 0.2, 0.3], 0);
        let r2 = make_async_result("w2", vec![0.4, 0.5, 0.6], 0);
        coord.buffer.insert("w1".to_string(), r1);
        coord.buffer.insert("w2".to_string(), r2);

        coord.aggregate_and_step().unwrap();
        assert!(coord.buffer.is_empty());
        assert!(coord.collection_start.is_none());
    }

    // ── Health monitor tests ─────────────────────────────────────────

    #[test]
    fn health_monitor_detects_dead_worker() {
        let (mut coord, _tx) = make_async_coordinator(3);
        let wid = WorkerId("w1".to_string());
        coord.register_worker(wid.clone());

        let timeout_ms = coord.config.heartbeat_timeout_secs * 1000;
        let future_ms = now_ms() + timeout_ms * 2 + 1;
        let events = coord.health_monitor.check(future_ms);

        let dead: Vec<_> =
            events.iter().filter(|e| matches!(e, FaultEvent::WorkerDead { .. })).collect();
        assert_eq!(dead.len(), 1);
    }

    #[test]
    fn health_monitor_heartbeat_resets() {
        let (mut coord, _tx) = make_async_coordinator(3);
        let wid = WorkerId("w1".to_string());
        coord.register_worker(wid.clone());

        let t = now_ms();
        coord.health_monitor.heartbeat(&wid, t);

        let events = coord.health_monitor.check(t + 100);
        assert!(events.is_empty());
        let health = coord.health_monitor.get_health(&wid).unwrap();
        assert_eq!(health.state, HealthState::Healthy);
    }

    #[test]
    fn dead_worker_removed_from_buffer() {
        let (mut coord, _tx) = make_async_coordinator(3);
        let wid = WorkerId("w1".to_string());
        coord.register_worker(wid.clone());

        // Insert into buffer manually.
        let r = make_async_result("w1", vec![0.1, 0.2, 0.3], 0);
        coord.buffer.insert("w1".to_string(), r);
        assert_eq!(coord.buffer.len(), 1);

        // Simulate worker death via health check.
        let timeout_ms = coord.config.heartbeat_timeout_secs * 1000;
        let future_ms = now_ms() + timeout_ms * 2 + 1;
        let events = coord.health_monitor.check(future_ms);

        // Process events like the run loop would.
        for event in &events {
            if let FaultEvent::WorkerDead { worker_id } = event {
                coord.buffer.remove(&worker_id.0);
            }
        }
        assert_eq!(coord.buffer.len(), 0);
    }

    // ── Event loop tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn run_collects_and_aggregates() {
        let (mut coord, tx) = make_async_coordinator(3);
        coord.register_worker(WorkerId("w1".to_string()));
        coord.register_worker(WorkerId("w2".to_string()));

        tx.send(make_async_result("w1", vec![0.1, 0.2, 0.3], 0)).await.unwrap();
        tx.send(make_async_result("w2", vec![0.4, 0.5, 0.6], 0)).await.unwrap();
        drop(tx);

        coord.run().await.unwrap();

        assert_eq!(coord.outer_step(), 1);
        assert_ne!(coord.parameters(), &[1.0, 1.0, 1.0]);
    }

    #[tokio::test]
    async fn run_drops_stale_gradients() {
        let (mut coord, tx) = make_async_coordinator(3);
        coord.outer_step = 20;

        // Staleness = 20 > max_staleness(5) → dropped.
        tx.send(make_async_result("w1", vec![0.1, 0.2, 0.3], 0)).await.unwrap();
        // Staleness = 10 > max_staleness(5) → dropped.
        tx.send(make_async_result("w2", vec![0.4, 0.5, 0.6], 10)).await.unwrap();
        drop(tx);

        coord.run().await.unwrap();

        // outer_step unchanged (no aggregation happened).
        assert_eq!(coord.outer_step(), 20);
        assert_eq!(coord.dropped_count, 2);
    }

    #[tokio::test]
    async fn run_timeout_triggers_aggregation() {
        let config = short_timeout_config();
        let params = vec![1.0, 1.0, 1.0];
        let (mut coord, tx) = AsyncCoordinator::new(config, params).unwrap();
        coord.register_worker(WorkerId("w1".to_string()));

        // Send 1 result (< min_workers=3), keep channel open.
        tx.send(make_async_result("w1", vec![0.1, 0.2, 0.3], 0)).await.unwrap();

        // Run with timeout — grace period should trigger aggregation.
        let run_result = tokio::time::timeout(Duration::from_secs(5), coord.run()).await;

        // Timeout expected (tx still alive), but aggregation should
        // have happened via collection timeout + grace period.
        assert!(run_result.is_err() || run_result.is_ok());
        assert_eq!(coord.outer_step(), 1, "timeout should have triggered aggregation");
    }

    #[tokio::test]
    async fn run_shutdown_drains_buffer() {
        let (mut coord, tx) = make_async_coordinator(3);
        coord.register_worker(WorkerId("w1".to_string()));

        // Send 1 result (< min_workers=2).
        tx.send(make_async_result("w1", vec![0.1, 0.2, 0.3], 0)).await.unwrap();
        drop(tx);

        coord.run().await.unwrap();

        // No aggregation happened (quorum not reached).
        assert_eq!(coord.outer_step(), 0);
        assert_eq!(coord.buffer.len(), 1);

        // Shutdown attempts final aggregation.
        coord.shutdown();
        assert_eq!(coord.outer_step(), 1);
        assert_eq!(coord.status(), AsyncCoordinatorStatus::Completed);
        assert!(coord.buffer.is_empty());
    }

    // ── Edge case tests ──────────────────────────────────────────────

    #[test]
    fn duplicate_worker_overwrites() {
        let (mut coord, _tx) = make_async_coordinator(3);
        let r1 = make_async_result("w1", vec![0.1, 0.2, 0.3], 0);
        let r2 = make_async_result("w1", vec![0.5, 0.6, 0.7], 0);

        coord.process_gradient(r1);
        coord.process_gradient(r2);

        assert_eq!(coord.buffer.len(), 1);
        let agg = coord.aggregate_and_step().unwrap();
        // Should use the latest gradient.
        assert_eq!(agg.averaged_gradient, vec![0.5, 0.6, 0.7]);
    }

    #[test]
    fn zero_momentum_no_accumulation() {
        let mut config = async_test_config();
        config.outer_momentum = 0.0;
        config.outer_lr = 1.0;
        config.min_workers = 1;
        config.max_workers = 1;
        let params = vec![0.0];
        let (mut coord, _tx) = AsyncCoordinator::new(config, params).unwrap();

        // With momentum=0: v = gradient every time.
        let r1 = make_async_result("w1", vec![1.0], 0);
        coord.buffer.insert("w1".to_string(), r1);
        let agg1 = coord.aggregate_and_step().unwrap();
        assert_eq!(agg1.parameter_update, vec![1.0]);

        let r2 = make_async_result("w1", vec![2.0], 1);
        coord.buffer.insert("w1".to_string(), r2);
        let agg2 = coord.aggregate_and_step().unwrap();
        // v = 0*1.0 + 2.0 = 2.0 (no accumulation).
        assert_eq!(agg2.parameter_update, vec![2.0]);
    }

    #[test]
    fn all_workers_stale() {
        let (mut coord, _tx) = make_async_coordinator(3);
        coord.outer_step = 20;

        // All exceed max_staleness(5).
        let r1 = make_async_result("w1", vec![0.1, 0.2, 0.3], 0);
        let r2 = make_async_result("w2", vec![0.4, 0.5, 0.6], 0);

        coord.process_gradient(r1);
        coord.process_gradient(r2);

        assert!(coord.buffer.is_empty());
        assert_eq!(coord.dropped_count, 2);
    }

    // ── ts-rs export tests ───────────────────────────────────────────

    #[test]
    fn ts_export_staleness_weight() {
        use ts_rs::Config;
        let name = StalenessWeight::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_async_coordinator_status() {
        use ts_rs::Config;
        let name = AsyncCoordinatorStatus::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_async_aggregation_result() {
        use ts_rs::Config;
        let name = AsyncAggregationResult::name(&Config::new());
        assert!(!name.is_empty());
    }
}
