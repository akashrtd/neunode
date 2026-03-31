use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::config::TrainingConfig;
use crate::error::{Result, TrainingError};

/// Unique identifier for a training worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub struct WorkerId(pub String);

/// Status of a training worker.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum WorkerStatus {
    /// Worker is idle, waiting for assignment.
    Idle,
    /// Worker is actively training local steps.
    Training,
    /// Worker has finished local steps, sending gradients.
    Reporting,
    /// Worker failed and needs recovery.
    Failed,
}

/// Result of a single local training step.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StepResult {
    /// Current step number (0-indexed within the local run).
    #[ts(type = "number")]
    pub step: u32,
    /// Loss value after this step.
    pub loss: f64,
    /// Learning rate used for this step.
    pub lr: f64,
    /// Timestamp of step completion (millis since epoch).
    #[ts(type = "number")]
    pub timestamp_ms: u64,
}

/// Result of a complete local training run (N steps).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LocalRunResult {
    /// The worker that produced this result.
    pub worker_id: WorkerId,
    /// Pseudo-gradients: θ_final - θ_initial (parameter delta).
    /// Length = number of model parameters.
    pub pseudo_gradients: Vec<f32>,
    /// Loss values from each step.
    pub step_losses: Vec<f64>,
    /// Total number of steps completed.
    #[ts(type = "number")]
    pub steps_completed: u32,
    /// Whether all local_steps were completed.
    pub completed: bool,
    /// Timestamp when run started (millis since epoch).
    #[ts(type = "number")]
    pub started_at_ms: u64,
    /// Timestamp when run completed (millis since epoch).
    #[ts(type = "number")]
    pub completed_at_ms: u64,
    /// Coordinator's outer_step when this worker began training.
    /// Used to compute staleness: coordinator.outer_step - global_step_at_start.
    /// Defaults to 0 for backward compatibility with sync coordinator.
    #[serde(default)]
    #[ts(type = "number")]
    pub global_step_at_start: u32,
}

/// Trait for executing model operations.
/// Abstract over the actual ML framework (no torch dependency).
pub trait ModelExecutor {
    /// Run a forward pass on a batch, return loss.
    fn forward(&mut self, batch_idx: u32) -> Result<f64>;

    /// Run backward pass, return gradient norms for diagnostics.
    fn backward(&mut self) -> Result<f64>;

    /// Get current model parameters as a flat f32 vector.
    fn get_parameters(&self) -> Vec<f32>;

    /// Set model parameters from a flat f32 vector.
    fn set_parameters(&mut self, params: &[f32]) -> Result<()>;

    /// Get the number of trainable parameters.
    fn parameter_count(&self) -> usize;

    /// Get current learning rate (may be scheduled).
    /// Default: constant learning rate. Override for warmup/decay.
    fn current_lr(&self, step: u32, base_lr: f64) -> f64 {
        let _ = step;
        base_lr
    }
}

/// A training worker that runs local DiLoCo steps.
pub struct TrainingWorker<E: ModelExecutor> {
    /// Unique worker identifier.
    pub worker_id: WorkerId,
    /// Current status.
    pub status: WorkerStatus,
    /// Training configuration.
    config: TrainingConfig,
    /// Model executor (abstract over ML framework).
    executor: E,
}

impl<E: ModelExecutor> TrainingWorker<E> {
    /// Create a new training worker.
    pub fn new(worker_id: WorkerId, config: TrainingConfig, executor: E) -> Result<Self> {
        config.validate()?;
        if executor.parameter_count() == 0 {
            return Err(TrainingError::ConfigInvalid(
                "executor must have > 0 parameters".to_string(),
            ));
        }
        Ok(Self { worker_id, status: WorkerStatus::Idle, config, executor })
    }

    /// Run local training steps.
    /// Returns pseudo-gradients (θ_final - θ_initial) and step results.
    pub fn run_local_steps(&mut self) -> Result<LocalRunResult> {
        let started_at_ms = now_ms();

        // 1. Record initial parameters.
        let initial_params = self.executor.get_parameters();
        let param_count = initial_params.len();

        // 2. Set status to Training.
        self.status = WorkerStatus::Training;

        // 3. Run local steps.
        let mut step_losses = Vec::with_capacity(self.config.local_steps as usize);

        for step in 0..self.config.local_steps {
            let loss = self.executor.forward(step)?;
            let _grad_norm = self.executor.backward()?;

            // Check for NaN/Inf loss.
            if !loss.is_finite() {
                self.status = WorkerStatus::Failed;
                return Err(TrainingError::WorkerFailed {
                    worker_id: self.worker_id.0.clone(),
                    reason: format!("non-finite loss at step {step}: {loss}"),
                });
            }

            step_losses.push(loss);
        }

        // 4. Get final parameters.
        let final_params = self.executor.get_parameters();

        // 5. Compute pseudo-gradients: θ_final - θ_initial.
        let pseudo_gradients: Vec<f32> = final_params
            .iter()
            .zip(initial_params.iter())
            .map(|(final_p, init_p)| final_p - init_p)
            .collect();

        // Sanity check: length must match.
        if pseudo_gradients.len() != param_count {
            self.status = WorkerStatus::Failed;
            return Err(TrainingError::GradientMismatch);
        }

        let completed_at_ms = now_ms();

        // 6. Set status to Reporting.
        self.status = WorkerStatus::Reporting;

        Ok(LocalRunResult {
            worker_id: self.worker_id.clone(),
            pseudo_gradients,
            step_losses,
            steps_completed: self.config.local_steps,
            completed: true,
            started_at_ms,
            completed_at_ms,
            global_step_at_start: 0,
        })
    }

    /// Get current worker status.
    pub fn status(&self) -> WorkerStatus {
        self.status
    }

    /// Get reference to the model executor.
    pub fn executor(&self) -> &E {
        &self.executor
    }

    /// Get mutable reference to the model executor.
    pub fn executor_mut(&mut self) -> &mut E {
        &mut self.executor
    }

    /// Get the worker ID.
    pub fn id(&self) -> &WorkerId {
        &self.worker_id
    }

    /// Get the training config.
    pub fn config(&self) -> &TrainingConfig {
        &self.config
    }
}

/// Returns current time in milliseconds since Unix epoch.
fn now_ms() -> u64 {
    use std::time::SystemTime;
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock executor that simulates training with a simple quadratic loss.
    /// Loss = sum(params²). Optimal params = all zeros.
    struct MockExecutor {
        params: Vec<f32>,
        lr: f64,
    }

    impl MockExecutor {
        fn new(param_count: usize, lr: f64) -> Self {
            Self { params: vec![1.0; param_count], lr }
        }
    }

    impl ModelExecutor for MockExecutor {
        fn forward(&mut self, _batch_idx: u32) -> Result<f64> {
            // Loss = sum of params squared.
            Ok(self.params.iter().map(|p| (*p as f64) * (*p as f64)).sum())
        }

        fn backward(&mut self) -> Result<f64> {
            // Gradient = 2 * param. Apply SGD step.
            let grad_norm: f64 =
                self.params.iter().map(|p| (2.0 * *p as f64).powi(2)).sum::<f64>().sqrt();
            for p in self.params.iter_mut() {
                *p -= (self.lr as f32) * 2.0 * *p; // gradient step
            }
            Ok(grad_norm)
        }

        fn get_parameters(&self) -> Vec<f32> {
            self.params.clone()
        }

        fn set_parameters(&mut self, params: &[f32]) -> Result<()> {
            if params.len() != self.params.len() {
                return Err(TrainingError::WorkerFailed {
                    worker_id: "mock".to_string(),
                    reason: format!(
                        "param count mismatch: expected {}, got {}",
                        self.params.len(),
                        params.len()
                    ),
                });
            }
            self.params = params.to_vec();
            Ok(())
        }

        fn parameter_count(&self) -> usize {
            self.params.len()
        }
    }

    /// Helper: create a valid TrainingConfig with small local_steps for tests.
    fn test_config(local_steps: u32) -> TrainingConfig {
        TrainingConfig {
            local_steps,
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

    fn make_worker(steps: u32) -> TrainingWorker<MockExecutor> {
        TrainingWorker::new(
            WorkerId("test-worker".to_string()),
            test_config(steps),
            MockExecutor::new(10, 0.01),
        )
        .unwrap()
    }

    // ── Serde roundtrip tests ──────────────────────────────────────────

    #[test]
    fn worker_id_serde() {
        let id = WorkerId("worker-42".to_string());
        let json = serde_json::to_string(&id).unwrap();
        let back: WorkerId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn worker_status_serde() {
        for status in [
            WorkerStatus::Idle,
            WorkerStatus::Training,
            WorkerStatus::Reporting,
            WorkerStatus::Failed,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: WorkerStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, back);
        }
    }

    #[test]
    fn step_result_serde() {
        let sr = StepResult { step: 7, loss: 0.42, lr: 4e-4, timestamp_ms: 1700000000000 };
        let json = serde_json::to_string(&sr).unwrap();
        let back: StepResult = serde_json::from_str(&json).unwrap();
        assert_eq!(sr.step, back.step);
        assert!((sr.loss - back.loss).abs() < f64::EPSILON);
        assert!((sr.lr - back.lr).abs() < f64::EPSILON);
        assert_eq!(sr.timestamp_ms, back.timestamp_ms);
    }

    #[test]
    fn local_run_result_serde() {
        let result = LocalRunResult {
            worker_id: WorkerId("w1".to_string()),
            pseudo_gradients: vec![0.1, -0.2, 0.3],
            step_losses: vec![1.0, 0.8, 0.6],
            steps_completed: 3,
            completed: true,
            started_at_ms: 1000,
            completed_at_ms: 2000,
            global_step_at_start: 0,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: LocalRunResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.worker_id, back.worker_id);
        assert_eq!(result.pseudo_gradients, back.pseudo_gradients);
        assert_eq!(result.step_losses, back.step_losses);
        assert_eq!(result.steps_completed, back.steps_completed);
        assert_eq!(result.completed, back.completed);
        assert_eq!(result.started_at_ms, back.started_at_ms);
        assert_eq!(result.completed_at_ms, back.completed_at_ms);
    }

    // ── Worker creation tests ─────────────────────────────────────────

    #[test]
    fn worker_new_succeeds() {
        let worker = make_worker(10);
        assert_eq!(worker.worker_id, WorkerId("test-worker".to_string()));
        assert_eq!(worker.status(), WorkerStatus::Idle);
    }

    #[test]
    fn worker_new_invalid_config() {
        let result = TrainingWorker::new(
            WorkerId("bad".to_string()),
            test_config(0), // local_steps=0 is invalid
            MockExecutor::new(10, 0.01),
        );
        assert!(result.is_err());
    }

    #[test]
    fn worker_initial_status_idle() {
        let worker = make_worker(5);
        assert_eq!(worker.status(), WorkerStatus::Idle);
    }

    // ── Local run tests ───────────────────────────────────────────────

    #[test]
    fn run_local_steps_completes() {
        let mut worker = make_worker(10);
        let result = worker.run_local_steps().unwrap();
        assert!(result.completed);
    }

    #[test]
    fn run_local_steps_correct_count() {
        let mut worker = make_worker(5);
        let result = worker.run_local_steps().unwrap();
        assert_eq!(result.steps_completed, 5);
    }

    #[test]
    fn run_local_steps_losses_recorded() {
        let mut worker = make_worker(8);
        let result = worker.run_local_steps().unwrap();
        assert_eq!(result.step_losses.len(), 8);
    }

    #[test]
    fn run_local_steps_pseudo_gradients() {
        let param_count = 10;
        let mut worker = TrainingWorker::new(
            WorkerId("pg-test".to_string()),
            test_config(3),
            MockExecutor::new(param_count, 0.01),
        )
        .unwrap();
        let result = worker.run_local_steps().unwrap();
        assert_eq!(result.pseudo_gradients.len(), param_count);
    }

    #[test]
    fn run_local_steps_loss_decreases() {
        let mut worker = make_worker(20);
        let result = worker.run_local_steps().unwrap();
        // MockExecutor on quadratic loss should decrease monotonically.
        let first = result.step_losses.first().unwrap();
        let last = result.step_losses.last().unwrap();
        assert!(last < first, "loss should decrease: first={first}, last={last}");
    }

    #[test]
    fn run_local_steps_status_transitions() {
        let mut worker = make_worker(3);
        assert_eq!(worker.status(), WorkerStatus::Idle);
        let _ = worker.run_local_steps().unwrap();
        assert_eq!(worker.status(), WorkerStatus::Reporting);
    }

    #[test]
    fn run_local_steps_timestamps() {
        let mut worker = make_worker(2);
        let result = worker.run_local_steps().unwrap();
        assert!(
            result.completed_at_ms >= result.started_at_ms,
            "completed_at_ms should be >= started_at_ms"
        );
    }

    // ── ModelExecutor trait object test ────────────────────────────────

    #[test]
    fn model_executor_trait() {
        fn uses_executor(exec: &mut dyn ModelExecutor) -> Result<f64> {
            let loss = exec.forward(0)?;
            exec.backward()?;
            Ok(loss)
        }
        let mut mock = MockExecutor::new(5, 0.01);
        let loss = uses_executor(&mut mock).unwrap();
        assert!(loss > 0.0);
    }

    #[test]
    fn mock_executor_parameter_count() {
        let mock = MockExecutor::new(42, 0.01);
        assert_eq!(mock.parameter_count(), 42);
    }

    #[test]
    fn mock_executor_set_get_roundtrip() {
        let mut mock = MockExecutor::new(3, 0.01);
        let params = vec![1.0, 2.0, 3.0];
        mock.set_parameters(&params).unwrap();
        let got = mock.get_parameters();
        assert_eq!(got, params);
    }

    #[test]
    fn mock_executor_set_wrong_count() {
        let mut mock = MockExecutor::new(3, 0.01);
        let result = mock.set_parameters(&[1.0, 2.0]);
        assert!(result.is_err());
    }

    #[test]
    fn config_zero_steps_fails() {
        let result = TrainingWorker::new(
            WorkerId("zero".to_string()),
            test_config(0),
            MockExecutor::new(5, 0.01),
        );
        let err = result.err().expect("should fail with zero steps");
        match err {
            TrainingError::ConfigInvalid(msg) => assert!(msg.contains("local_steps")),
            other => panic!("expected ConfigInvalid, got {other}"),
        }
    }

    // ── ts-rs export tests ─────────────────────────────────────────────

    #[test]
    fn ts_export_worker_id() {
        use ts_rs::Config;
        let name = WorkerId::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_worker_status() {
        use ts_rs::Config;
        let name = WorkerStatus::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_step_result() {
        use ts_rs::Config;
        let name = StepResult::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_local_run_result() {
        use ts_rs::Config;
        let name = LocalRunResult::name(&Config::new());
        assert!(!name.is_empty());
    }

    // ── Worker accessor tests ──────────────────────────────────────────

    #[test]
    fn worker_id_accessor() {
        let worker = make_worker(5);
        assert_eq!(worker.id().0, "test-worker");
    }

    #[test]
    fn worker_config_accessor() {
        let worker = make_worker(5);
        assert_eq!(worker.config().local_steps, 5);
    }

    #[test]
    fn worker_executor_accessor() {
        let mut worker = make_worker(3);
        assert_eq!(worker.executor().parameter_count(), 10);
        // Mutable accessor.
        worker.executor_mut().set_parameters(&vec![0.5; 10]).unwrap();
        assert_eq!(worker.executor().get_parameters(), vec![0.5; 10]);
    }

    #[test]
    fn pseudo_gradients_are_nonzero() {
        // After training with MockExecutor (starting at 1.0, moving toward 0),
        // pseudo-gradients should be negative.
        let mut worker = make_worker(5);
        let result = worker.run_local_steps().unwrap();
        // All pseudo-gradients should be negative (params moved from 1.0 toward 0).
        for pg in &result.pseudo_gradients {
            assert!(*pg < 0.0, "pseudo-gradient should be negative (params decreased): {pg}");
        }
    }

    #[test]
    fn worker_status_snake_case() {
        // Verify serde uses snake_case.
        let json = serde_json::to_string(&WorkerStatus::Training).unwrap();
        assert!(json.contains("training"), "got: {json}");
        assert!(!json.contains("Training"), "should be snake_case: {json}");
    }
}
