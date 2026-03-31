//! Integration tests for the DiLoCo training flow.
//!
//! Verifies end-to-end: worker creation → coordinator setup → local training
//! → gradient submission → aggregation → outer step → checkpoint → settlement.
//! Tests cross-crate interactions between neunode-training and neunode-storage.

use std::sync::atomic::{AtomicU64, Ordering};

use neunode_core::types::{Did, CID};
use neunode_storage::db::NeunodeDb;
use neunode_training::config::TrainingConfig;
use neunode_training::{
    AggregationMode, CheckpointMeta, CheckpointStore, GradientAggregator, GradientMessage,
    HealthMonitor, HealthState, LocalRunResult, ModelExecutor, TrainingCoordinator,
    TrainingSettlement, TrainingWorker, WorkerId,
};

// ---------------------------------------------------------------------------
// Mock executor — simulates training with quadratic loss
// ---------------------------------------------------------------------------

/// Mock executor that simulates training on f(x) = Σ x_i².
/// Gradient = 2x, so SGD with lr moves params toward zero.
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
    fn forward(&mut self, _batch_idx: u32) -> neunode_training::Result<f64> {
        Ok(self.params.iter().map(|p| (*p as f64) * (*p as f64)).sum())
    }

    fn backward(&mut self) -> neunode_training::Result<f64> {
        let grad_norm: f64 =
            self.params.iter().map(|p| (2.0 * *p as f64).powi(2)).sum::<f64>().sqrt();
        for p in self.params.iter_mut() {
            *p -= (self.lr as f32) * 2.0 * *p;
        }
        Ok(grad_norm)
    }

    fn get_parameters(&self) -> Vec<f32> {
        self.params.clone()
    }

    fn set_parameters(&mut self, params: &[f32]) -> neunode_training::Result<()> {
        self.params = params.to_vec();
        Ok(())
    }

    fn parameter_count(&self) -> usize {
        self.params.len()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

static TEST_DB_ID: AtomicU64 = AtomicU64::new(0);

fn temp_db() -> NeunodeDb {
    let id = TEST_DB_ID.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("neunode_training_flow_{:?}_{}", std::process::id(), id));
    let _ = std::fs::remove_dir_all(&dir);
    NeunodeDb::open(&dir).expect("temp db should open")
}

fn test_config() -> TrainingConfig {
    TrainingConfig {
        local_steps: 5,
        inner_lr: 0.01,
        outer_lr: 0.7,
        outer_momentum: 0.9,
        batch_size: 32,
        quantization_bits: 8,
        max_workers: 8,
        heartbeat_timeout_secs: 10,
        checkpoint_interval: 1,
        async_mode: false,
        min_workers: 2,
        max_staleness: 0,
        grace_period_secs: 10,
        collection_timeout_secs: 30,
    }
}

fn wid(name: &str) -> WorkerId {
    WorkerId(name.to_string())
}

fn test_did(name: &str) -> Did {
    Did(format!("did:neunode:0x{name}"))
}

fn make_checkpoint(step: u32, job_id: &str, num_workers: u32) -> CheckpointMeta {
    CheckpointMeta {
        cid: CID(format!("bafkrei_{job_id}_step_{step}")),
        job_id: job_id.to_string(),
        worker_did: test_did("coordinator"),
        outer_step: step,
        loss: 1.0 / (step as f64 + 1.0),
        timestamp_ms: 1000 + step as u64 * 100,
        size_bytes: 14_000_000,
        num_workers,
    }
}

// ---------------------------------------------------------------------------
// Test 1: Full DiLoCo round trip — 3 workers → coordinate → aggregate
// ---------------------------------------------------------------------------

#[test]
fn full_diloco_round_trip() {
    let config = test_config();
    let param_count = 10;
    let initial_params: Vec<f32> = vec![1.0; param_count];

    // Spawn 3 workers, each with a MockExecutor.
    let mut workers: Vec<TrainingWorker<MockExecutor>> = (0..3)
        .map(|i| {
            TrainingWorker::new(
                wid(&format!("worker-{i}")),
                config.clone(),
                MockExecutor::new(param_count, 0.01),
            )
            .expect("worker creation should succeed")
        })
        .collect();

    // Create coordinator with initial parameters.
    let mut coord = TrainingCoordinator::new(config, initial_params).expect("coordinator creation");

    assert_eq!(coord.outer_step(), 0, "should start at step 0");

    // Each worker runs local steps and submits results.
    let mut results: Vec<LocalRunResult> = Vec::new();
    for worker in &mut workers {
        let result = worker.run_local_steps().expect("local steps should complete");
        assert!(result.completed, "local run should complete");
        assert_eq!(result.steps_completed, 5, "should complete 5 local steps");
        assert_eq!(result.pseudo_gradients.len(), param_count);
        results.push(result);
    }

    // Submit all results to coordinator.
    for result in results {
        coord.submit_result(result).expect("submit should succeed");
    }

    assert_eq!(coord.pending_count(), 3, "should have 3 pending results");
    assert!(coord.ready_to_aggregate());

    // Aggregate and apply outer optimizer step.
    let agg = coord.aggregate_and_step().expect("aggregation should succeed");

    assert_eq!(agg.num_workers, 3, "all 3 workers should be counted");
    assert_eq!(agg.outer_step, 1, "outer step should be 1");
    assert_eq!(agg.averaged_gradient.len(), param_count);

    // Averaged gradient should be element-wise mean of all pseudo-gradients.
    // All workers start from same params with same lr, so pseudo-gradients are identical.
    // The mean should equal any single worker's pseudo-gradient.
    assert!(agg.avg_loss > 0.0, "average loss should be positive");

    // Parameters should have changed.
    let updated_params = coord.parameters();
    assert!(
        updated_params.iter().any(|&p| (p - 1.0f32).abs() > 1e-6),
        "parameters should have changed from initial 1.0"
    );

    // Coordinator back to Idle, ready for next round.
    assert_eq!(coord.status(), neunode_training::CoordinatorStatus::Idle);
    assert_eq!(coord.pending_count(), 0, "pending should be cleared");
}

// ---------------------------------------------------------------------------
// Test 2: Gradient aggregation — AllReduce mode, element-wise mean
// ---------------------------------------------------------------------------

#[test]
fn gradient_aggregation_all_reduce() {
    let mut agg = GradientAggregator::new(AggregationMode::AllReduce, 4);

    agg.submit(wid("w1"), vec![1.0, 2.0, 3.0, 4.0]).expect("submit w1");
    agg.submit(wid("w2"), vec![5.0, 6.0, 7.0, 8.0]).expect("submit w2");
    agg.submit(wid("w3"), vec![9.0, 10.0, 11.0, 12.0]).expect("submit w3");
    agg.submit(wid("w4"), vec![13.0, 14.0, 15.0, 16.0]).expect("submit w4");

    // AllReduce: is_ready() always false (coordinator decides).
    assert!(!agg.is_ready());
    assert_eq!(agg.worker_count(), 4);

    let result = agg.aggregate().expect("aggregate should succeed");

    // Element-wise mean: [(1+5+9+13)/4, (2+6+10+14)/4, ...] = [7, 8, 9, 10]
    let expected = [7.0, 8.0, 9.0, 10.0];
    for (a, b) in result.iter().zip(expected.iter()) {
        assert!((a - b).abs() < 1e-5, "expected {b}, got {a}");
    }
}

// ---------------------------------------------------------------------------
// Test 3: Gradient aggregation — Partial mode with min_workers threshold
// ---------------------------------------------------------------------------

#[test]
fn gradient_aggregation_partial_mode() {
    let mut agg = GradientAggregator::new(AggregationMode::Partial { min_workers: 2 }, 3);

    // 0 workers → not ready.
    assert!(!agg.is_ready());

    // 1 worker → not ready.
    agg.submit(wid("w1"), vec![2.0, 4.0, 6.0]).expect("submit w1");
    assert!(!agg.is_ready());
    assert_eq!(agg.worker_count(), 1);

    // 2 workers → ready.
    agg.submit(wid("w2"), vec![4.0, 8.0, 12.0]).expect("submit w2");
    assert!(agg.is_ready());
    assert_eq!(agg.worker_count(), 2);

    // Aggregate with 2 workers → element-wise mean.
    let result1 = agg.aggregate().expect("aggregate with 2 workers");
    let expected1 = [3.0, 6.0, 9.0];
    for (a, b) in result1.iter().zip(expected1.iter()) {
        assert!((a - b).abs() < 1e-5, "expected {b}, got {a}");
    }

    // Reset and submit 3 workers.
    agg.reset();
    agg.submit(wid("w1"), vec![3.0, 6.0, 9.0]).expect("submit w1");
    agg.submit(wid("w2"), vec![6.0, 12.0, 18.0]).expect("submit w2");
    agg.submit(wid("w3"), vec![9.0, 18.0, 27.0]).expect("submit w3");

    let result2 = agg.aggregate().expect("aggregate with 3 workers");
    let expected2 = [6.0, 12.0, 18.0];
    for (a, b) in result2.iter().zip(expected2.iter()) {
        assert!((a - b).abs() < 1e-5, "expected {b}, got {a}");
    }
}

// ---------------------------------------------------------------------------
// Test 4: Gradient wire format roundtrip — F32 exact + Int8 approximate
// ---------------------------------------------------------------------------

#[test]
fn gradient_wire_format_roundtrip() {
    let gradients = vec![0.1, -0.2, 0.3, -0.4, 0.5, -0.6, 0.7, -0.8];

    // F32 roundtrip — exact.
    let msg_f32 =
        GradientMessage::encode_f32("worker-1", "job-42", 10, &gradients).expect("encode f32");
    assert!(msg_f32.verify_checksum(), "f32 checksum should verify");

    let decoded_f32 = msg_f32.decode().expect("decode f32");
    assert_eq!(decoded_f32, gradients, "f32 roundtrip should be exact");

    // Int8 roundtrip — approximate (cosine similarity > 0.95).
    let scale = 0.01f32;
    let msg_int8 = GradientMessage::encode_int8("worker-1", "job-42", 10, &gradients, scale)
        .expect("encode int8");
    assert!(msg_int8.verify_checksum(), "int8 checksum should verify");

    let decoded_int8 = msg_int8.decode().expect("decode int8");
    assert_eq!(decoded_int8.len(), gradients.len(), "int8 length should match");

    // Compute cosine similarity.
    let dot: f32 = gradients.iter().zip(decoded_int8.iter()).map(|(a, b)| a * b).sum();
    let norm_a: f32 = gradients.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = decoded_int8.iter().map(|x| x * x).sum::<f32>().sqrt();
    let cosine_sim = dot / (norm_a * norm_b);
    assert!(cosine_sim > 0.95, "int8 cosine similarity should be > 0.95, got {cosine_sim}");

    // Metadata should be preserved.
    assert_eq!(msg_f32.worker_id, "worker-1");
    assert_eq!(msg_f32.job_id, "job-42");
    assert_eq!(msg_f32.outer_step, 10);
}

// ---------------------------------------------------------------------------
// Test 5: Settlement milestone tracking — milestones → total → finalize
// ---------------------------------------------------------------------------

#[test]
fn settlement_milestone_tracking() {
    let mut settlement = TrainingSettlement::new("train-llama-3b", "did:neunode:0xCreator", 10_000);

    assert_eq!(settlement.total_deposit, 10_000);
    assert_eq!(settlement.protocol_fee(), 200, "2% of 10_000 = 200");
    assert_eq!(settlement.remaining_budget(), 9_800, "10000 - 200 fee = 9800");
    assert_eq!(settlement.total_paid(), 0);

    // Record milestones for 3 workers.
    let p1 = settlement.record_milestone(1, wid("worker-a"), 0.3).expect("milestone 1");
    // payout = 10000 * 0.98 * 0.3 = 2940
    assert_eq!(p1, 2_940);

    let p2 = settlement.record_milestone(2, wid("worker-b"), 0.3).expect("milestone 2");
    assert!(p2 > 0, "second payout should be positive");

    let p3 = settlement.record_milestone(3, wid("worker-c"), 0.2).expect("milestone 3");
    assert!(p3 > 0, "third payout should be positive");

    // Verify totals.
    let total_paid = settlement.total_paid();
    assert_eq!(total_paid, p1 + p2 + p3, "total paid should equal sum of payouts");
    assert!(total_paid <= 9_800, "total paid should not exceed net budget");

    let remaining = settlement.remaining_budget();
    assert_eq!(remaining + total_paid + settlement.protocol_fee(), 10_000);

    // Finalize.
    settlement.finalize().expect("finalize should succeed");
    match &settlement.status {
        neunode_training::SettlementStatus::Completed { total_paid: tp } => {
            assert_eq!(*tp, total_paid);
        }
        other => panic!("expected Completed, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test 6: Fault tolerance — heartbeat tracking → suspect/dead transitions
// ---------------------------------------------------------------------------

#[test]
fn fault_tolerance_heartbeat_tracking() {
    let timeout_secs: u64 = 10;
    let timeout_ms = timeout_secs * 1000;
    let mut monitor = HealthMonitor::new(timeout_secs);

    // Register 4 workers at time 0.
    monitor.register(wid("w1"), 0);
    monitor.register(wid("w2"), 0);
    monitor.register(wid("w3"), 0);
    monitor.register(wid("w4"), 0);
    assert_eq!(monitor.total_count(), 4);
    assert_eq!(monitor.healthy_count(), 4);

    // At 5 seconds: all still healthy.
    let events = monitor.check(5_000);
    assert!(events.is_empty(), "no events expected at 5s");
    assert_eq!(monitor.healthy_count(), 4);

    // At 10 seconds: all transition to Suspect.
    let events = monitor.check(timeout_ms);
    let suspect_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, neunode_training::FaultEvent::WorkerSuspect { .. }))
        .collect();
    assert_eq!(suspect_events.len(), 4, "all 4 workers should be suspect");
    assert_eq!(monitor.healthy_count(), 0);

    // Keep w4 alive with a heartbeat close to the next check.
    let recovery = monitor.heartbeat(&wid("w4"), timeout_ms * 2);
    assert!(
        recovery.iter().any(|e| matches!(e, neunode_training::FaultEvent::WorkerRecovered { .. })),
        "w4 should recover"
    );

    // At 2x timeout (20s): w1, w2, w3 transition to Dead. w4 still healthy.
    let events = monitor.check(timeout_ms * 2 + 1);
    let dead_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, neunode_training::FaultEvent::WorkerDead { .. }))
        .collect();
    assert_eq!(dead_events.len(), 3, "3 workers should be dead");

    // 3/4 dead = 75% → RecoveryNeeded event.
    let recovery_needed: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, neunode_training::FaultEvent::RecoveryNeeded { .. }))
        .collect();
    assert_eq!(recovery_needed.len(), 1, "majority dead should trigger recovery");

    // Verify dead workers list.
    let dead = monitor.dead_workers();
    assert_eq!(dead.len(), 3);

    // w4 still healthy.
    let w4_health = monitor.get_health(&wid("w4")).expect("w4 should exist");
    assert_eq!(w4_health.state, HealthState::Healthy);
}

// ---------------------------------------------------------------------------
// Test 7: Checkpoint store lifecycle — save → load → list → delete
// ---------------------------------------------------------------------------

#[test]
fn checkpoint_store_lifecycle() {
    let db = temp_db();
    let store = CheckpointStore::new(&db);
    let job_id = "train-llama-3b";

    // Save 3 checkpoints at different steps.
    let ckpt10 = make_checkpoint(10, job_id, 3);
    let ckpt20 = make_checkpoint(20, job_id, 3);
    let ckpt30 = make_checkpoint(30, job_id, 4);

    store.save(&ckpt10).expect("save ckpt10");
    store.save(&ckpt20).expect("save ckpt20");
    store.save(&ckpt30).expect("save ckpt30");

    // Load specific checkpoint.
    let loaded = store.load(job_id, 20).expect("load should succeed").expect("should exist");
    assert_eq!(loaded.cid.0, ckpt20.cid.0);
    assert_eq!(loaded.outer_step, 20);
    assert_eq!(loaded.num_workers, 3);
    assert_eq!(loaded.worker_did.0, "did:neunode:0xcoordinator");

    // List all checkpoints — should be ordered by step.
    let all = store.list(job_id).expect("list should succeed");
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].outer_step, 10);
    assert_eq!(all[1].outer_step, 20);
    assert_eq!(all[2].outer_step, 30);

    // Latest checkpoint.
    let latest = store.latest(job_id).expect("latest").expect("should exist");
    assert_eq!(latest.outer_step, 30);

    // CID management — verify CID values.
    assert_eq!(all[0].cid.0, format!("bafkrei_{job_id}_step_10"));
    assert_eq!(all[2].cid.0, format!("bafkrei_{job_id}_step_30"));

    // Exists check.
    assert!(store.exists(job_id, 20).expect("exists"));
    assert!(!store.exists(job_id, 99).expect("exists"));

    // Delete one checkpoint.
    store.delete(job_id, 20).expect("delete");
    assert!(!store.exists(job_id, 20).expect("exists after delete"));
    assert_eq!(store.list(job_id).expect("list").len(), 2);

    // Delete all remaining.
    let count = store.delete_all(job_id).expect("delete_all");
    assert_eq!(count, 2);
    assert!(store.list(job_id).expect("list").is_empty());
}

// ---------------------------------------------------------------------------
// Test 8: Training config validation — valid + multiple invalid configs
// ---------------------------------------------------------------------------

#[test]
fn training_config_validation() {
    // Valid config should pass.
    let valid = test_config();
    assert!(valid.validate().is_ok(), "default test config should be valid");

    // Zero local_steps.
    let mut cfg = test_config();
    cfg.local_steps = 0;
    assert!(cfg.validate().is_err(), "zero local_steps should fail");

    // Negative inner_lr.
    let mut cfg = test_config();
    cfg.inner_lr = -0.01;
    assert!(cfg.validate().is_err(), "negative inner_lr should fail");

    // Zero inner_lr.
    let mut cfg = test_config();
    cfg.inner_lr = 0.0;
    assert!(cfg.validate().is_err(), "zero inner_lr should fail");

    // Negative outer_lr.
    let mut cfg = test_config();
    cfg.outer_lr = -1.0;
    assert!(cfg.validate().is_err(), "negative outer_lr should fail");

    // Zero batch_size.
    let mut cfg = test_config();
    cfg.batch_size = 0;
    assert!(cfg.validate().is_err(), "zero batch_size should fail");

    // Zero quantization_bits.
    let mut cfg = test_config();
    cfg.quantization_bits = 0;
    assert!(cfg.validate().is_err(), "zero quantization_bits should fail");

    // Quantization_bits > 16.
    let mut cfg = test_config();
    cfg.quantization_bits = 17;
    assert!(cfg.validate().is_err(), "quantization_bits > 16 should fail");

    // Zero max_workers.
    let mut cfg = test_config();
    cfg.max_workers = 0;
    assert!(cfg.validate().is_err(), "zero max_workers should fail");

    // Zero heartbeat_timeout_secs.
    let mut cfg = test_config();
    cfg.heartbeat_timeout_secs = 0;
    assert!(cfg.validate().is_err(), "zero heartbeat_timeout should fail");

    // Zero checkpoint_interval.
    let mut cfg = test_config();
    cfg.checkpoint_interval = 0;
    assert!(cfg.validate().is_err(), "zero checkpoint_interval should fail");

    // Boundary values should pass.
    let boundary = TrainingConfig {
        local_steps: 1,
        inner_lr: f64::MIN_POSITIVE,
        outer_lr: f64::MIN_POSITIVE,
        outer_momentum: 0.0,
        batch_size: 1,
        quantization_bits: 1,
        max_workers: 1,
        heartbeat_timeout_secs: 1,
        checkpoint_interval: 1,
        async_mode: true,
        min_workers: 1,
        max_staleness: 5,
        grace_period_secs: 1,
        collection_timeout_secs: 1,
    };
    assert!(boundary.validate().is_ok(), "boundary config should be valid");
}
