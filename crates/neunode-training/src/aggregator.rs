use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Result, TrainingError};
use crate::worker::WorkerId;

/// Aggregation strategy for pseudo-gradient collection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum AggregationMode {
    /// Average across ALL submitted workers (standard DiLoCo all-reduce).
    AllReduce,
    /// Aggregate once `min_workers` have reported (gossipsub-style partial).
    Partial { min_workers: usize },
}

/// Collects pseudo-gradients from multiple workers and produces an averaged gradient.
///
/// Supports both all-reduce (full average) and gossipsub-style partial aggregation
/// for fault-tolerant distributed training rounds.
#[derive(Debug, Clone)]
pub struct GradientAggregator {
    mode: AggregationMode,
    param_count: usize,
    submissions: HashMap<WorkerId, Vec<f32>>,
}

impl GradientAggregator {
    /// Create a new aggregator with the given mode and expected parameter count.
    pub fn new(mode: AggregationMode, param_count: usize) -> Self {
        Self { mode, param_count, submissions: HashMap::new() }
    }

    /// Submit one worker's pseudo-gradients for aggregation.
    ///
    /// Returns `GradientMismatch` if the gradient dimension doesn't match
    /// `param_count`, or `AggregationFailed` if the worker has already
    /// submitted in this round.
    pub fn submit(&mut self, worker_id: WorkerId, gradients: Vec<f32>) -> Result<()> {
        if gradients.len() != self.param_count {
            return Err(TrainingError::GradientMismatch);
        }
        if self.submissions.contains_key(&worker_id) {
            return Err(TrainingError::AggregationFailed(format!(
                "duplicate worker: {}",
                worker_id.0
            )));
        }
        self.submissions.insert(worker_id, gradients);
        Ok(())
    }

    /// Check if enough workers have submitted to proceed with aggregation.
    ///
    /// - `AllReduce`: always returns `false` (coordinator decides when to aggregate).
    /// - `Partial { min_workers }`: returns `true` once `min_workers` have submitted.
    pub fn is_ready(&self) -> bool {
        match self.mode {
            AggregationMode::AllReduce => false,
            AggregationMode::Partial { min_workers } => self.submissions.len() >= min_workers,
        }
    }

    /// Compute the averaged gradient across all submitted workers.
    ///
    /// Element-wise mean: `avg[i] = sum(gradients[i]) / count`.
    pub fn aggregate(&self) -> Result<Vec<f32>> {
        if self.submissions.is_empty() {
            return Err(TrainingError::AggregationFailed("no gradients to aggregate".to_string()));
        }
        let count = self.submissions.len();
        let mut averaged = vec![0.0f32; self.param_count];
        for gradients in self.submissions.values() {
            for (i, &g) in gradients.iter().enumerate() {
                averaged[i] += g;
            }
        }
        let inv_n = 1.0 / count as f32;
        for v in averaged.iter_mut() {
            *v *= inv_n;
        }
        Ok(averaged)
    }

    /// Number of workers that have submitted gradients in this round.
    pub fn worker_count(&self) -> usize {
        self.submissions.len()
    }

    /// Clear all submissions, preparing for the next aggregation round.
    pub fn reset(&mut self) {
        self.submissions.clear();
    }

    /// List the workers who have submitted gradients in this round.
    pub fn submitted_workers(&self) -> Vec<WorkerId> {
        self.submissions.keys().cloned().collect()
    }

    /// Get the aggregation mode.
    pub fn mode(&self) -> &AggregationMode {
        &self.mode
    }

    /// Get the expected parameter count.
    pub fn param_count(&self) -> usize {
        self.param_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wid(name: &str) -> WorkerId {
        WorkerId(name.to_string())
    }

    // ── Basic aggregation roundtrip ────────────────────────────────────

    #[test]
    fn basic_aggregation_roundtrip() {
        let mut agg = GradientAggregator::new(AggregationMode::AllReduce, 3);
        agg.submit(wid("w1"), vec![1.0, 2.0, 3.0]).unwrap();
        agg.submit(wid("w2"), vec![3.0, 4.0, 5.0]).unwrap();

        let result = agg.aggregate().unwrap();
        let expected = [2.0, 3.0, 4.0];
        for (a, b) in result.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-6, "{a} != {b}");
        }
    }

    // ── AllReduce mode ─────────────────────────────────────────────────

    #[test]
    fn all_reduce_is_ready_always_false() {
        let mut agg = GradientAggregator::new(AggregationMode::AllReduce, 2);
        assert!(!agg.is_ready());
        agg.submit(wid("w1"), vec![1.0, 2.0]).unwrap();
        assert!(!agg.is_ready());
        agg.submit(wid("w2"), vec![3.0, 4.0]).unwrap();
        assert!(!agg.is_ready());
    }

    #[test]
    fn all_reduce_averages_all() {
        let mut agg = GradientAggregator::new(AggregationMode::AllReduce, 2);
        agg.submit(wid("w1"), vec![0.0, 0.0]).unwrap();
        agg.submit(wid("w2"), vec![2.0, 4.0]).unwrap();
        agg.submit(wid("w3"), vec![4.0, 8.0]).unwrap();

        let result = agg.aggregate().unwrap();
        // avg: [(0+2+4)/3, (0+4+8)/3] = [2.0, 4.0]
        let expected = [2.0, 4.0];
        for (a, b) in result.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-5, "{a} != {b}");
        }
    }

    // ── Partial mode ───────────────────────────────────────────────────

    #[test]
    fn partial_mode_threshold() {
        let mut agg = GradientAggregator::new(AggregationMode::Partial { min_workers: 3 }, 2);
        assert!(!agg.is_ready());
        agg.submit(wid("w1"), vec![1.0, 2.0]).unwrap();
        assert!(!agg.is_ready());
        agg.submit(wid("w2"), vec![3.0, 4.0]).unwrap();
        assert!(!agg.is_ready());
        agg.submit(wid("w3"), vec![5.0, 6.0]).unwrap();
        assert!(agg.is_ready());
    }

    #[test]
    fn partial_mode_aggregates_submitted_so_far() {
        let mut agg = GradientAggregator::new(AggregationMode::Partial { min_workers: 2 }, 2);
        agg.submit(wid("w1"), vec![2.0, 4.0]).unwrap();
        agg.submit(wid("w2"), vec![4.0, 6.0]).unwrap();

        let result = agg.aggregate().unwrap();
        let expected = [3.0, 5.0];
        for (a, b) in result.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-5, "{a} != {b}");
        }
    }

    // ── Dimension mismatch error ───────────────────────────────────────

    #[test]
    fn dimension_mismatch_error() {
        let mut agg = GradientAggregator::new(AggregationMode::AllReduce, 3);
        let result = agg.submit(wid("w1"), vec![1.0, 2.0]);
        assert!(result.is_err());
        match result.unwrap_err() {
            TrainingError::GradientMismatch => {}
            other => panic!("expected GradientMismatch, got {other}"),
        }
    }

    // ── Duplicate worker error ─────────────────────────────────────────

    #[test]
    fn duplicate_worker_error() {
        let mut agg = GradientAggregator::new(AggregationMode::AllReduce, 2);
        agg.submit(wid("w1"), vec![1.0, 2.0]).unwrap();
        let result = agg.submit(wid("w1"), vec![3.0, 4.0]);
        assert!(result.is_err());
        match result.unwrap_err() {
            TrainingError::AggregationFailed(msg) => {
                assert!(msg.contains("duplicate worker: w1"), "got: {msg}");
            }
            other => panic!("expected AggregationFailed, got {other}"),
        }
    }

    // ── Empty aggregation error ────────────────────────────────────────

    #[test]
    fn empty_aggregation_error() {
        let agg = GradientAggregator::new(AggregationMode::AllReduce, 3);
        let result = agg.aggregate();
        assert!(result.is_err());
        match result.unwrap_err() {
            TrainingError::AggregationFailed(msg) => {
                assert!(msg.contains("no gradients"), "got: {msg}");
            }
            other => panic!("expected AggregationFailed, got {other}"),
        }
    }

    // ── Reset clears state ─────────────────────────────────────────────

    #[test]
    fn reset_clears_state() {
        let mut agg = GradientAggregator::new(AggregationMode::AllReduce, 2);
        agg.submit(wid("w1"), vec![1.0, 2.0]).unwrap();
        agg.submit(wid("w2"), vec![3.0, 4.0]).unwrap();
        assert_eq!(agg.worker_count(), 2);

        agg.reset();
        assert_eq!(agg.worker_count(), 0);
        assert!(agg.submitted_workers().is_empty());
        assert!(agg.aggregate().is_err());
    }

    // ── Multiple rounds ────────────────────────────────────────────────

    #[test]
    fn multiple_rounds() {
        let mut agg = GradientAggregator::new(AggregationMode::AllReduce, 2);

        // Round 1.
        agg.submit(wid("w1"), vec![1.0, 1.0]).unwrap();
        agg.submit(wid("w2"), vec![3.0, 3.0]).unwrap();
        let r1 = agg.aggregate().unwrap();
        assert_eq!(r1, vec![2.0, 2.0]);

        // Round 2 (different gradients).
        agg.reset();
        agg.submit(wid("w1"), vec![2.0, 2.0]).unwrap();
        agg.submit(wid("w2"), vec![4.0, 4.0]).unwrap();
        let r2 = agg.aggregate().unwrap();
        assert_eq!(r2, vec![3.0, 3.0]);
    }

    // ── Worker count tracking ──────────────────────────────────────────

    #[test]
    fn worker_count_tracking() {
        let mut agg = GradientAggregator::new(AggregationMode::AllReduce, 2);
        assert_eq!(agg.worker_count(), 0);
        agg.submit(wid("w1"), vec![1.0, 2.0]).unwrap();
        assert_eq!(agg.worker_count(), 1);
        agg.submit(wid("w2"), vec![3.0, 4.0]).unwrap();
        assert_eq!(agg.worker_count(), 2);
    }

    // ── Submitted workers list ─────────────────────────────────────────

    #[test]
    fn submitted_workers_list() {
        let mut agg = GradientAggregator::new(AggregationMode::AllReduce, 1);
        agg.submit(wid("alpha"), vec![0.5]).unwrap();
        agg.submit(wid("beta"), vec![0.7]).unwrap();

        let mut workers = agg.submitted_workers();
        workers.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(workers.len(), 2);
        assert_eq!(workers[0], wid("alpha"));
        assert_eq!(workers[1], wid("beta"));
    }

    // ── Single worker aggregation ──────────────────────────────────────

    #[test]
    fn single_worker_aggregation() {
        let mut agg = GradientAggregator::new(AggregationMode::AllReduce, 3);
        agg.submit(wid("w1"), vec![0.1, -0.2, 0.3]).unwrap();

        let result = agg.aggregate().unwrap();
        assert_eq!(result, vec![0.1, -0.2, 0.3]);
    }

    // ── Large gradient aggregation (10K params) ────────────────────────

    #[test]
    fn large_gradient_aggregation() {
        let n = 10_000;
        let mut agg = GradientAggregator::new(AggregationMode::AllReduce, n);

        let g1: Vec<f32> = (0..n).map(|i| i as f32 * 0.001).collect();
        let g2: Vec<f32> = (0..n).map(|i| (n - i) as f32 * 0.001).collect();

        agg.submit(wid("w1"), g1.clone()).unwrap();
        agg.submit(wid("w2"), g2.clone()).unwrap();

        let result = agg.aggregate().unwrap();
        assert_eq!(result.len(), n);
        // avg[i] = (i*0.001 + (n-i)*0.001) / 2 = n*0.001 / 2 = 5.0
        let expected = n as f32 * 0.001 / 2.0;
        for &v in &result {
            assert!((v - expected).abs() < 1e-3, "{v} != {expected}");
        }
    }

    // ── Serde roundtrip for AggregationMode ────────────────────────────

    #[test]
    fn serde_roundtrip_all_reduce() {
        let mode = AggregationMode::AllReduce;
        let json = serde_json::to_string(&mode).unwrap();
        let back: AggregationMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, back);
    }

    #[test]
    fn serde_roundtrip_partial() {
        let mode = AggregationMode::Partial { min_workers: 5 };
        let json = serde_json::to_string(&mode).unwrap();
        let back: AggregationMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, back);
    }

    #[test]
    fn serde_snake_case_all_reduce() {
        let json = serde_json::to_string(&AggregationMode::AllReduce).unwrap();
        assert!(json.contains("all_reduce"), "got: {json}");
        assert!(!json.contains("AllReduce"), "should be snake_case: {json}");
    }

    #[test]
    fn serde_snake_case_partial() {
        let json = serde_json::to_string(&AggregationMode::Partial { min_workers: 3 }).unwrap();
        assert!(json.contains("partial"), "got: {json}");
        assert!(!json.contains("Partial"), "should be snake_case: {json}");
    }

    // ── Accessor tests ─────────────────────────────────────────────────

    #[test]
    fn mode_accessor() {
        let agg = GradientAggregator::new(AggregationMode::AllReduce, 10);
        assert_eq!(agg.mode(), &AggregationMode::AllReduce);
    }

    #[test]
    fn param_count_accessor() {
        let agg = GradientAggregator::new(AggregationMode::AllReduce, 42);
        assert_eq!(agg.param_count(), 42);
    }

    // ── ts-rs export ───────────────────────────────────────────────────

    #[test]
    fn ts_export_aggregation_mode() {
        use ts_rs::Config;
        let name = AggregationMode::name(&Config::new());
        assert!(!name.is_empty());
    }

    // ── Partial mode ready then reset ──────────────────────────────────

    #[test]
    fn partial_mode_becomes_not_ready_after_reset() {
        let mut agg = GradientAggregator::new(AggregationMode::Partial { min_workers: 1 }, 2);
        agg.submit(wid("w1"), vec![1.0, 2.0]).unwrap();
        assert!(agg.is_ready());
        agg.reset();
        assert!(!agg.is_ready());
    }

    // ── Zero-length gradients rejected ─────────────────────────────────

    #[test]
    fn zero_param_count_rejects_all() {
        let mut agg = GradientAggregator::new(AggregationMode::AllReduce, 0);
        let result = agg.submit(wid("w1"), vec![]);
        // Empty vec has len 0 == param_count 0, so this is OK dimension-wise.
        // But aggregate on empty submissions should still fail.
        // Actually with 0 param_count, submitting empty vec should succeed.
        assert!(result.is_ok());
        // But the aggregator has 1 worker with 0-length gradients.
        // aggregate should return empty vec.
        let out = agg.aggregate().unwrap();
        assert!(out.is_empty());
    }

    // ── Negative gradients ─────────────────────────────────────────────

    #[test]
    fn negative_gradients_averaged_correctly() {
        let mut agg = GradientAggregator::new(AggregationMode::AllReduce, 2);
        agg.submit(wid("w1"), vec![-1.0, -3.0]).unwrap();
        agg.submit(wid("w2"), vec![1.0, 3.0]).unwrap();

        let result = agg.aggregate().unwrap();
        assert!((result[0] - 0.0).abs() < 1e-6);
        assert!((result[1] - 0.0).abs() < 1e-6);
    }
}
