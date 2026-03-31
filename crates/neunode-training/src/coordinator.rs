use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::config::TrainingConfig;
use crate::error::{Result, TrainingError};
use crate::worker::LocalRunResult;

/// Status of the training coordinator.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum CoordinatorStatus {
    /// No active round.
    Idle,
    /// Collecting pseudo-gradients from workers.
    Collecting,
    /// Aggregating gradients and applying outer optimizer step.
    Aggregating,
    /// Distributing updated parameters to workers.
    Distributing,
    /// Training completed (all outer steps done).
    Completed,
    /// Coordinator hit an unrecoverable error.
    Failed,
}

/// State of the Nesterov momentum buffer.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MomentumBuffer {
    /// Velocity vector (same dimension as model parameters).
    pub velocity: Vec<f32>,
    /// Momentum coefficient.
    pub momentum: f64,
}

impl MomentumBuffer {
    /// Create a zeroed momentum buffer.
    pub fn new(param_count: usize, momentum: f64) -> Self {
        Self { velocity: vec![0.0; param_count], momentum }
    }

    /// Apply Nesterov momentum update.
    ///
    /// Standard velocity update: `v = momentum * v + gradient`
    /// Returns the velocity vector (the look-ahead direction for Nesterov).
    pub fn update(&mut self, gradient: &[f32]) -> Vec<f32> {
        for (i, &g) in gradient.iter().enumerate() {
            self.velocity[i] = self.momentum as f32 * self.velocity[i] + g;
        }
        self.velocity.clone()
    }

    /// Reset the momentum buffer to zero velocity.
    pub fn reset(&mut self) {
        self.velocity.fill(0.0);
    }
}

/// Result of a single outer aggregation step.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AggregationResult {
    /// Averaged pseudo-gradient across all workers.
    pub averaged_gradient: Vec<f32>,
    /// Number of workers that contributed.
    #[ts(type = "number")]
    pub num_workers: u32,
    /// Outer step number (1-indexed after first aggregation).
    #[ts(type = "number")]
    pub outer_step: u32,
    /// Average loss across all workers (last local step loss).
    pub avg_loss: f64,
    /// Parameter update applied (velocity after momentum).
    pub parameter_update: Vec<f32>,
}

/// The DiLoCo training coordinator.
///
/// Manages the outer optimization loop: receives pseudo-gradients from workers,
/// averages them, applies Nesterov SGD, and tracks updated parameters.
#[derive(Debug)]
pub struct TrainingCoordinator {
    config: TrainingConfig,
    status: CoordinatorStatus,
    current_params: Vec<f32>,
    momentum: MomentumBuffer,
    outer_step: u32,
    pending_results: HashMap<String, LocalRunResult>,
}

impl TrainingCoordinator {
    /// Create a new coordinator with initial model parameters.
    ///
    /// Validates the config and rejects empty parameter vectors.
    pub fn new(config: TrainingConfig, initial_params: Vec<f32>) -> Result<Self> {
        config.validate()?;
        if initial_params.is_empty() {
            return Err(TrainingError::ConfigInvalid(
                "initial params must be non-empty".to_string(),
            ));
        }
        let param_count = initial_params.len();
        let momentum = MomentumBuffer::new(param_count, config.outer_momentum);
        Ok(Self {
            config,
            status: CoordinatorStatus::Idle,
            current_params: initial_params,
            momentum,
            outer_step: 0,
            pending_results: HashMap::new(),
        })
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
    pub fn status(&self) -> CoordinatorStatus {
        self.status
    }

    /// Get the training config reference.
    pub fn config(&self) -> &TrainingConfig {
        &self.config
    }

    /// Submit a worker's local run result for aggregation.
    ///
    /// Transitions status to `Collecting` on first submission.
    /// Rejects results whose gradient dimension doesn't match parameters.
    pub fn submit_result(&mut self, result: LocalRunResult) -> Result<()> {
        if self.status != CoordinatorStatus::Collecting && self.status != CoordinatorStatus::Idle {
            return Err(TrainingError::CoordinatorTimeout(format!(
                "coordinator not collecting (status: {:?})",
                self.status
            )));
        }
        self.status = CoordinatorStatus::Collecting;

        // Validate gradient dimension matches parameters.
        if result.pseudo_gradients.len() != self.current_params.len() {
            return Err(TrainingError::GradientMismatch);
        }

        self.pending_results.insert(result.worker_id.0.clone(), result);
        Ok(())
    }

    /// Check if any results have been submitted (ready to aggregate).
    pub fn ready_to_aggregate(&self) -> bool {
        !self.pending_results.is_empty()
    }

    /// Number of pending worker results.
    pub fn pending_count(&self) -> usize {
        self.pending_results.len()
    }

    /// Aggregate all submitted pseudo-gradients and apply outer optimizer step.
    ///
    /// 1. Average pseudo-gradients: `Δθ_avg = (1/N) Σ Δθ_i`
    /// 2. Nesterov velocity: `v = momentum * v + Δθ_avg`
    /// 3. Update params: `θ -= outer_lr * v`
    ///
    /// Returns the aggregation result. Clears pending results afterward.
    pub fn aggregate_and_step(&mut self) -> Result<AggregationResult> {
        if self.pending_results.is_empty() {
            return Err(TrainingError::AggregationFailed("no results to aggregate".to_string()));
        }
        self.status = CoordinatorStatus::Aggregating;

        let param_count = self.current_params.len();
        let num_workers = self.pending_results.len() as u32;

        // 1. Average the pseudo-gradients.
        let mut averaged = vec![0.0f32; param_count];
        let mut total_loss = 0.0f64;
        for result in self.pending_results.values() {
            for (i, &g) in result.pseudo_gradients.iter().enumerate() {
                averaged[i] += g;
            }
            total_loss += result.step_losses.last().copied().unwrap_or(0.0);
        }
        let inv_n = 1.0 / num_workers as f32;
        for v in averaged.iter_mut() {
            *v *= inv_n;
        }
        let avg_loss = total_loss / num_workers as f64;

        // 2. Apply Nesterov momentum: v = momentum * v + gradient.
        let parameter_update = self.momentum.update(&averaged);

        // 3. Update parameters: θ_new = θ - lr * v.
        let lr = self.config.outer_lr as f32;
        for (i, &update) in parameter_update.iter().enumerate() {
            self.current_params[i] -= lr * update;
        }

        self.outer_step += 1;
        self.status = CoordinatorStatus::Distributing;

        let result = AggregationResult {
            averaged_gradient: averaged,
            num_workers,
            outer_step: self.outer_step,
            avg_loss,
            parameter_update,
        };

        self.pending_results.clear();
        self.status = CoordinatorStatus::Idle;
        Ok(result)
    }

    /// Reset coordinator state (keeps config and current parameters).
    ///
    /// Clears momentum buffer, resets step counter, drops pending results.
    pub fn reset(&mut self) {
        self.momentum.reset();
        self.outer_step = 0;
        self.pending_results.clear();
        self.status = CoordinatorStatus::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::WorkerId;

    /// Helper: valid TrainingConfig for tests.
    fn test_config() -> TrainingConfig {
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
            async_mode: false,
            min_workers: 2,
            max_staleness: 0,
            grace_period_secs: 5,
            collection_timeout_secs: 15,
        }
    }

    /// Helper: build a LocalRunResult with given worker ID and gradients.
    fn make_result(worker_id: &str, gradients: Vec<f32>) -> LocalRunResult {
        LocalRunResult {
            worker_id: WorkerId(worker_id.to_string()),
            pseudo_gradients: gradients,
            step_losses: vec![1.0],
            steps_completed: 10,
            completed: true,
            started_at_ms: 1000,
            completed_at_ms: 2000,
            global_step_at_start: 0,
        }
    }

    /// Helper: create a coordinator with 3 parameters all set to 1.0.
    fn make_coordinator() -> TrainingCoordinator {
        TrainingCoordinator::new(test_config(), vec![1.0, 1.0, 1.0]).unwrap()
    }

    // ── MomentumBuffer tests ───────────────────────────────────────────

    #[test]
    fn momentum_buffer_new() {
        let buf = MomentumBuffer::new(4, 0.9);
        assert_eq!(buf.velocity, vec![0.0, 0.0, 0.0, 0.0]);
        assert!((buf.momentum - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn momentum_update() {
        let mut buf = MomentumBuffer::new(3, 0.9);
        let grad = vec![1.0, 2.0, 3.0];
        let updated = buf.update(&grad);
        // v = 0.9 * [0,0,0] + [1,2,3] = [1,2,3]
        assert_eq!(updated, vec![1.0, 2.0, 3.0]);
        assert_eq!(buf.velocity, vec![1.0, 2.0, 3.0]);

        // Second update: v = 0.9 * [1,2,3] + [1,2,3] = [1.9, 3.8, 5.7]
        let updated2 = buf.update(&grad);
        let expected = vec![1.9, 3.8, 5.7];
        for (a, b) in updated2.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-5, "{a} != {b}");
        }
    }

    #[test]
    fn momentum_reset() {
        let mut buf = MomentumBuffer::new(3, 0.9);
        buf.update(&[1.0, 2.0, 3.0]);
        assert!(buf.velocity.iter().any(|&v| v != 0.0));
        buf.reset();
        assert_eq!(buf.velocity, vec![0.0, 0.0, 0.0]);
    }

    // ── Coordinator creation tests ─────────────────────────────────────

    #[test]
    fn coordinator_new() {
        let coord = make_coordinator();
        assert_eq!(coord.parameters(), &[1.0, 1.0, 1.0]);
        assert_eq!(coord.outer_step(), 0);
    }

    #[test]
    fn coordinator_new_empty_params() {
        let result = TrainingCoordinator::new(test_config(), vec![]);
        assert!(result.is_err());
        match result.unwrap_err() {
            TrainingError::ConfigInvalid(msg) => {
                assert!(msg.contains("non-empty"));
            }
            other => panic!("expected ConfigInvalid, got {other}"),
        }
    }

    #[test]
    fn coordinator_initial_status() {
        let coord = make_coordinator();
        assert_eq!(coord.status(), CoordinatorStatus::Idle);
    }

    // ── Submit result tests ────────────────────────────────────────────

    #[test]
    fn submit_result_transitions_to_collecting() {
        let mut coord = make_coordinator();
        assert_eq!(coord.status(), CoordinatorStatus::Idle);
        coord.submit_result(make_result("w1", vec![0.1, 0.2, 0.3])).unwrap();
        assert_eq!(coord.status(), CoordinatorStatus::Collecting);
    }

    #[test]
    fn submit_result_stores_result() {
        let mut coord = make_coordinator();
        assert_eq!(coord.pending_count(), 0);
        coord.submit_result(make_result("w1", vec![0.1, 0.2, 0.3])).unwrap();
        assert_eq!(coord.pending_count(), 1);
        coord.submit_result(make_result("w2", vec![0.4, 0.5, 0.6])).unwrap();
        assert_eq!(coord.pending_count(), 2);
    }

    #[test]
    fn submit_result_wrong_gradient_dim() {
        let mut coord = make_coordinator();
        let result = coord.submit_result(make_result("w1", vec![0.1, 0.2]));
        assert!(result.is_err());
        match result.unwrap_err() {
            TrainingError::GradientMismatch => {}
            other => panic!("expected GradientMismatch, got {other}"),
        }
    }

    #[test]
    fn submit_result_rejects_when_completed() {
        let mut coord = make_coordinator();
        coord.status = CoordinatorStatus::Completed;
        let result = coord.submit_result(make_result("w1", vec![0.1, 0.2, 0.3]));
        assert!(result.is_err());
    }

    // ── Ready to aggregate tests ───────────────────────────────────────

    #[test]
    fn ready_to_aggregate_false_initially() {
        let coord = make_coordinator();
        assert!(!coord.ready_to_aggregate());
    }

    #[test]
    fn ready_to_aggregate_true_after_submit() {
        let mut coord = make_coordinator();
        coord.submit_result(make_result("w1", vec![0.1, 0.2, 0.3])).unwrap();
        assert!(coord.ready_to_aggregate());
    }

    // ── Aggregation tests ──────────────────────────────────────────────

    #[test]
    fn aggregate_empty_fails() {
        let mut coord = make_coordinator();
        let result = coord.aggregate_and_step();
        assert!(result.is_err());
        match result.unwrap_err() {
            TrainingError::AggregationFailed(msg) => {
                assert!(msg.contains("no results"));
            }
            other => panic!("expected AggregationFailed, got {other}"),
        }
    }

    #[test]
    fn aggregate_single_worker() {
        let mut coord = make_coordinator();
        let gradients = vec![0.1, 0.2, 0.3];
        coord.submit_result(make_result("w1", gradients.clone())).unwrap();

        let agg = coord.aggregate_and_step().unwrap();
        assert_eq!(agg.num_workers, 1);
        assert_eq!(agg.outer_step, 1);
        // Averaged gradient = gradients (only 1 worker).
        assert_eq!(agg.averaged_gradient, gradients);
        // With momentum=0.9: v = 0.9*[0,0,0] + [0.1,0.2,0.3] = [0.1,0.2,0.3]
        assert_eq!(agg.parameter_update, gradients);
    }

    #[test]
    fn aggregate_two_workers() {
        let mut coord = make_coordinator();
        coord.submit_result(make_result("w1", vec![0.2, 0.4, 0.6])).unwrap();
        coord.submit_result(make_result("w2", vec![0.4, 0.6, 0.8])).unwrap();

        let agg = coord.aggregate_and_step().unwrap();
        assert_eq!(agg.num_workers, 2);
        // Averaged: [(0.2+0.4)/2, (0.4+0.6)/2, (0.6+0.8)/2] = [0.3, 0.5, 0.7]
        let expected_avg = vec![0.3, 0.5, 0.7];
        for (a, b) in agg.averaged_gradient.iter().zip(expected_avg.iter()) {
            assert!((a - b).abs() < 1e-6, "{a} != {b}");
        }
    }

    #[test]
    fn aggregate_updates_params() {
        let mut coord = make_coordinator();
        let initial = coord.parameters().to_vec();
        coord.submit_result(make_result("w1", vec![0.1, 0.2, 0.3])).unwrap();
        coord.aggregate_and_step().unwrap();

        let updated = coord.parameters().to_vec();
        // Params should have changed.
        assert_ne!(initial, updated);
        // θ_new = [1,1,1] - 0.7 * [0.1,0.2,0.3] = [0.93, 0.86, 0.79]
        let expected = vec![0.93, 0.86, 0.79];
        for (a, b) in updated.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-5, "{a} != {b}");
        }
    }

    #[test]
    fn aggregate_increments_step() {
        let mut coord = make_coordinator();
        assert_eq!(coord.outer_step(), 0);
        coord.submit_result(make_result("w1", vec![0.1, 0.2, 0.3])).unwrap();
        coord.aggregate_and_step().unwrap();
        assert_eq!(coord.outer_step(), 1);
    }

    #[test]
    fn aggregate_clears_pending() {
        let mut coord = make_coordinator();
        coord.submit_result(make_result("w1", vec![0.1, 0.2, 0.3])).unwrap();
        coord.submit_result(make_result("w2", vec![0.2, 0.3, 0.4])).unwrap();
        assert_eq!(coord.pending_count(), 2);

        coord.aggregate_and_step().unwrap();
        assert_eq!(coord.pending_count(), 0);
        assert!(!coord.ready_to_aggregate());
    }

    #[test]
    fn aggregate_avg_loss() {
        let mut coord = make_coordinator();
        let mut r1 = make_result("w1", vec![0.1, 0.2, 0.3]);
        r1.step_losses = vec![1.0, 0.8, 0.5];
        let mut r2 = make_result("w2", vec![0.2, 0.3, 0.4]);
        r2.step_losses = vec![2.0, 1.5, 1.0];
        coord.submit_result(r1).unwrap();
        coord.submit_result(r2).unwrap();

        let agg = coord.aggregate_and_step().unwrap();
        // avg_loss = (0.5 + 1.0) / 2 = 0.75 (last step loss from each worker)
        assert!((agg.avg_loss - 0.75).abs() < 1e-10);
    }

    #[test]
    fn aggregate_status_back_to_idle() {
        let mut coord = make_coordinator();
        coord.submit_result(make_result("w1", vec![0.1, 0.2, 0.3])).unwrap();
        coord.aggregate_and_step().unwrap();
        assert_eq!(coord.status(), CoordinatorStatus::Idle);
    }

    // ── Multiple rounds ────────────────────────────────────────────────

    #[test]
    fn multiple_rounds() {
        let mut coord = make_coordinator();
        let mut prev_params = coord.parameters().to_vec();

        for round in 1..=3 {
            coord.submit_result(make_result("w1", vec![0.1 * round as f32, 0.1, 0.1])).unwrap();
            let agg = coord.aggregate_and_step().unwrap();
            assert_eq!(agg.outer_step, round as u32);

            let new_params = coord.parameters().to_vec();
            assert_ne!(prev_params, new_params, "params should change in round {round}");
            prev_params = new_params;
        }
        assert_eq!(coord.outer_step(), 3);
    }

    #[test]
    fn nesterov_momentum() {
        // Verify that momentum accumulates across rounds.
        let config = TrainingConfig {
            outer_momentum: 0.9,
            outer_lr: 1.0, // lr=1 for easier math
            ..test_config()
        };
        let mut coord = TrainingCoordinator::new(config, vec![0.0, 0.0]).unwrap();

        // Round 1: gradient [1, 1]
        // v1 = 0.9*[0,0] + [1,1] = [1,1]
        // params = [0,0] - 1.0*[1,1] = [-1,-1]
        coord.submit_result(make_result("w1", vec![1.0, 1.0])).unwrap();
        let agg1 = coord.aggregate_and_step().unwrap();
        assert_eq!(agg1.parameter_update, vec![1.0, 1.0]);
        let p1 = coord.parameters().to_vec();
        assert_eq!(p1, vec![-1.0, -1.0]);

        // Round 2: gradient [1, 1]
        // v2 = 0.9*[1,1] + [1,1] = [1.9, 1.9]
        // params = [-1,-1] - 1.0*[1.9,1.9] = [-2.9, -2.9]
        coord.submit_result(make_result("w1", vec![1.0, 1.0])).unwrap();
        let agg2 = coord.aggregate_and_step().unwrap();
        let expected_v = vec![1.9, 1.9];
        for (a, b) in agg2.parameter_update.iter().zip(expected_v.iter()) {
            assert!((a - b).abs() < 1e-5, "{a} != {b}");
        }
        let p2 = coord.parameters().to_vec();
        let expected_p = vec![-2.9, -2.9];
        for (a, b) in p2.iter().zip(expected_p.iter()) {
            assert!((a - b).abs() < 1e-5, "{a} != {b}");
        }
    }

    // ── Reset test ─────────────────────────────────────────────────────

    #[test]
    fn coordinator_reset() {
        let mut coord = make_coordinator();
        coord.submit_result(make_result("w1", vec![0.1, 0.2, 0.3])).unwrap();
        coord.aggregate_and_step().unwrap();
        assert_eq!(coord.outer_step(), 1);

        coord.reset();
        assert_eq!(coord.outer_step(), 0);
        assert_eq!(coord.status(), CoordinatorStatus::Idle);
        assert_eq!(coord.pending_count(), 0);
        // Momentum velocity should be zeroed.
        assert!(coord.momentum.velocity.iter().all(|&v| v == 0.0));
        // But params should be kept (not reset to initial).
        assert_ne!(coord.parameters(), &[1.0, 1.0, 1.0]);
    }

    // ── Serde roundtrip tests ──────────────────────────────────────────

    #[test]
    fn coordinator_status_serde() {
        for status in [
            CoordinatorStatus::Idle,
            CoordinatorStatus::Collecting,
            CoordinatorStatus::Aggregating,
            CoordinatorStatus::Distributing,
            CoordinatorStatus::Completed,
            CoordinatorStatus::Failed,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: CoordinatorStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, back);
        }
    }

    #[test]
    fn aggregation_result_serde() {
        let agg = AggregationResult {
            averaged_gradient: vec![0.1, -0.2, 0.3],
            num_workers: 3,
            outer_step: 7,
            avg_loss: 0.42,
            parameter_update: vec![0.15, -0.1, 0.25],
        };
        let json = serde_json::to_string(&agg).unwrap();
        let back: AggregationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(agg.averaged_gradient, back.averaged_gradient);
        assert_eq!(agg.num_workers, back.num_workers);
        assert_eq!(agg.outer_step, back.outer_step);
        assert!((agg.avg_loss - back.avg_loss).abs() < f64::EPSILON);
        assert_eq!(agg.parameter_update, back.parameter_update);
    }

    #[test]
    fn momentum_buffer_serde() {
        let buf = MomentumBuffer { velocity: vec![1.0, 2.0, 3.0], momentum: 0.9 };
        let json = serde_json::to_string(&buf).unwrap();
        let back: MomentumBuffer = serde_json::from_str(&json).unwrap();
        assert_eq!(buf.velocity, back.velocity);
        assert!((buf.momentum - back.momentum).abs() < f64::EPSILON);
    }

    // ── ts-rs export tests ─────────────────────────────────────────────

    #[test]
    fn ts_export_coordinator_status() {
        use ts_rs::Config;
        let name = CoordinatorStatus::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_aggregation_result() {
        use ts_rs::Config;
        let name = AggregationResult::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_momentum_buffer() {
        use ts_rs::Config;
        let name = MomentumBuffer::name(&Config::new());
        assert!(!name.is_empty());
    }

    // ── Snake case serde ───────────────────────────────────────────────

    #[test]
    fn coordinator_status_snake_case() {
        let json = serde_json::to_string(&CoordinatorStatus::Collecting).unwrap();
        assert!(json.contains("collecting"), "got: {json}");
        assert!(!json.contains("Collecting"), "should be snake_case: {json}");
    }

    // ── Config accessor test ───────────────────────────────────────────

    #[test]
    fn coordinator_config_accessor() {
        let coord = make_coordinator();
        assert_eq!(coord.config().local_steps, 10);
        assert!((coord.config().outer_lr - 0.7).abs() < f64::EPSILON);
    }

    // ── Zero momentum test ─────────────────────────────────────────────

    #[test]
    fn zero_momentum_no_accumulation() {
        let config = TrainingConfig { outer_momentum: 0.0, outer_lr: 1.0, ..test_config() };
        let mut coord = TrainingCoordinator::new(config, vec![0.0]).unwrap();

        // With momentum=0: v = 0*v + gradient = gradient every time.
        coord.submit_result(make_result("w1", vec![1.0])).unwrap();
        let agg1 = coord.aggregate_and_step().unwrap();
        assert_eq!(agg1.parameter_update, vec![1.0]);

        coord.submit_result(make_result("w1", vec![2.0])).unwrap();
        let agg2 = coord.aggregate_and_step().unwrap();
        // v = 0*1.0 + 2.0 = 2.0 (no accumulation)
        assert_eq!(agg2.parameter_update, vec![2.0]);
    }

    // ── Duplicate worker overwrites ────────────────────────────────────

    #[test]
    fn duplicate_worker_overwrites() {
        let mut coord = make_coordinator();
        coord.submit_result(make_result("w1", vec![0.1, 0.2, 0.3])).unwrap();
        coord.submit_result(make_result("w1", vec![0.5, 0.6, 0.7])).unwrap();
        // Same worker_id overwrites: still 1 pending.
        assert_eq!(coord.pending_count(), 1);

        let agg = coord.aggregate_and_step().unwrap();
        // Should use the latest gradient.
        assert_eq!(agg.averaged_gradient, vec![0.5, 0.6, 0.7]);
    }
}
