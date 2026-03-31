use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Result, TrainingError};
use crate::worker::WorkerId;

/// Default protocol fee percentage (2.0%).
const DEFAULT_PROTOCOL_FEE_PCT: f64 = 2.0;

/// A single milestone payout for a training worker's contribution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct Milestone {
    /// Outer step number this milestone corresponds to.
    #[ts(type = "number")]
    pub step: u64,
    /// The worker who earned this milestone.
    pub worker_id: WorkerId,
    /// Contribution quality score weighted by gradient quality (0.0–1.0).
    pub contribution_score: f64,
    /// Token payout for this milestone.
    #[ts(type = "number")]
    pub token_amount: u64,
}

/// Status of a training job settlement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case", tag = "state", content = "details")]
pub enum SettlementStatus {
    /// Job running, no payouts yet.
    Pending,
    /// Some milestones have been paid out.
    Partial {
        /// Number of outer steps that have been paid.
        #[ts(type = "number")]
        paid_steps: u64,
    },
    /// All milestones paid, settlement finalized.
    Completed {
        /// Total tokens paid out across all milestones.
        #[ts(type = "number")]
        total_paid: u64,
    },
    /// Job cancelled, deposit returned to requester.
    Refunded,
}

/// Training job settlement that manages milestone-based payouts.
///
/// Follows the same escrow pattern as the bounty crate: deposit → milestones →
/// release/refund, but adapted for distributed training where payouts are
/// proportional to each worker's contribution score per outer step.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TrainingSettlement {
    /// Unique job identifier.
    pub job_id: String,
    /// DID of the agent requesting training.
    pub requester: String,
    /// Total tokens deposited for this training job.
    #[ts(type = "number")]
    pub total_deposit: u64,
    /// Milestones that have been paid out.
    pub milestones: Vec<Milestone>,
    /// Current settlement status.
    pub status: SettlementStatus,
    /// Protocol fee percentage (default 2.0%).
    pub protocol_fee_pct: f64,
}

impl TrainingSettlement {
    /// Create a new pending settlement with the default 2% protocol fee.
    pub fn new(job_id: &str, requester: &str, total_deposit: u64) -> Self {
        Self {
            job_id: job_id.to_string(),
            requester: requester.to_string(),
            total_deposit,
            milestones: Vec::new(),
            status: SettlementStatus::Pending,
            protocol_fee_pct: DEFAULT_PROTOCOL_FEE_PCT,
        }
    }

    /// Record a milestone payout for a worker's contribution.
    ///
    /// Returns the payout amount in tokens. The payout is calculated as:
    /// `payout = total_deposit * (1.0 - protocol_fee_pct/100.0) * contribution_score`
    /// capped to the remaining budget.
    pub fn record_milestone(
        &mut self,
        step: u64,
        worker_id: WorkerId,
        contribution_score: f64,
    ) -> Result<u64> {
        let budget = self.remaining_budget();
        if budget == 0 {
            return Err(TrainingError::EscrowError(
                "milestone exceeds remaining budget".to_string(),
            ));
        }

        if contribution_score <= 0.0 || contribution_score > 1.0 {
            return Err(TrainingError::EscrowError(
                "contribution score must be in (0.0, 1.0]".to_string(),
            ));
        }

        let gross = self.total_deposit as f64 * (1.0 - self.protocol_fee_pct / 100.0);
        let payout = (gross * contribution_score) as u64;
        let payout = payout.min(budget);

        if payout == 0 {
            return Err(TrainingError::EscrowError(
                "milestone exceeds remaining budget".to_string(),
            ));
        }

        self.milestones.push(Milestone {
            step,
            worker_id,
            contribution_score,
            token_amount: payout,
        });

        self.status = SettlementStatus::Partial { paid_steps: self.milestones.len() as u64 };

        Ok(payout)
    }

    /// Tokens remaining after protocol fee and paid milestones.
    pub fn remaining_budget(&self) -> u64 {
        self.total_deposit.saturating_sub(self.protocol_fee()).saturating_sub(self.total_paid())
    }

    /// Sum of all milestone payouts.
    pub fn total_paid(&self) -> u64 {
        self.milestones.iter().map(|m| m.token_amount).sum()
    }

    /// Protocol fee in tokens.
    pub fn protocol_fee(&self) -> u64 {
        (self.total_deposit as f64 * self.protocol_fee_pct / 100.0) as u64
    }

    /// Finalize the settlement. Error if already finalized.
    pub fn finalize(&mut self) -> Result<()> {
        match &self.status {
            SettlementStatus::Completed { .. } => {
                Err(TrainingError::EscrowError("settlement already finalized".to_string()))
            }
            SettlementStatus::Refunded => {
                Err(TrainingError::EscrowError("settlement already finalized".to_string()))
            }
            _ => {
                let total = self.total_paid();
                self.status = SettlementStatus::Completed { total_paid: total };
                Ok(())
            }
        }
    }

    /// Refund the settlement. Error if already completed or refunded.
    pub fn refund(&mut self) -> Result<()> {
        match &self.status {
            SettlementStatus::Completed { .. } => {
                Err(TrainingError::EscrowError("cannot refund completed settlement".to_string()))
            }
            SettlementStatus::Refunded => {
                Err(TrainingError::EscrowError("cannot refund completed settlement".to_string()))
            }
            _ => {
                self.status = SettlementStatus::Refunded;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worker(name: &str) -> WorkerId {
        WorkerId(name.to_string())
    }

    // ── Construction tests ──────────────────────────────────────────────

    #[test]
    fn new_settlement_is_pending() {
        let s = TrainingSettlement::new("job-1", "did:neunode:alice", 10_000);
        assert_eq!(s.job_id, "job-1");
        assert_eq!(s.requester, "did:neunode:alice");
        assert_eq!(s.total_deposit, 10_000);
        assert_eq!(s.status, SettlementStatus::Pending);
        assert!(s.milestones.is_empty());
        assert!((s.protocol_fee_pct - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn protocol_fee_calculation() {
        let s = TrainingSettlement::new("j", "req", 10_000);
        assert_eq!(s.protocol_fee(), 200); // 2% of 10_000
    }

    #[test]
    fn protocol_fee_zero_deposit() {
        let s = TrainingSettlement::new("j", "req", 0);
        assert_eq!(s.protocol_fee(), 0);
    }

    // ── Milestone recording tests ───────────────────────────────────────

    #[test]
    fn record_single_milestone() {
        let mut s = TrainingSettlement::new("job-1", "req", 10_000);
        let payout = s.record_milestone(1, worker("w1"), 0.5).unwrap();
        // payout = 10000 * 0.98 * 0.5 = 4900
        assert_eq!(payout, 4_900);
        assert_eq!(s.milestones.len(), 1);
        assert_eq!(s.total_paid(), 4_900);
    }

    #[test]
    fn record_multiple_milestones() {
        let mut s = TrainingSettlement::new("job-1", "req", 10_000);
        let p1 = s.record_milestone(1, worker("w1"), 0.3).unwrap();
        let p2 = s.record_milestone(2, worker("w2"), 0.3).unwrap();
        // p1 = 10000 * 0.98 * 0.3 = 2940
        // p2 = 10000 * 0.98 * 0.3 = 2940, capped to remaining
        assert_eq!(p1, 2_940);
        assert!(p2 > 0);
        assert_eq!(s.milestones.len(), 2);
    }

    #[test]
    fn remaining_budget_tracks_payments() {
        let mut s = TrainingSettlement::new("job-1", "req", 10_000);
        assert_eq!(s.remaining_budget(), 9_800); // 10000 - 200 fee
        s.record_milestone(1, worker("w1"), 0.5).unwrap();
        assert_eq!(s.remaining_budget(), 4_900); // 9800 - 4900
    }

    #[test]
    fn total_paid_sums_milestones() {
        let mut s = TrainingSettlement::new("job-1", "req", 10_000);
        assert_eq!(s.total_paid(), 0);
        s.record_milestone(1, worker("w1"), 0.2).unwrap();
        s.record_milestone(2, worker("w2"), 0.3).unwrap();
        let total = s.total_paid();
        assert!(total > 0);
        assert_eq!(total, s.milestones.iter().map(|m| m.token_amount).sum::<u64>());
    }

    #[test]
    fn milestone_exceeds_budget_errors() {
        let mut s = TrainingSettlement::new("job-1", "req", 100);
        // First milestone takes most of the budget.
        s.record_milestone(1, worker("w1"), 1.0).unwrap();
        // Now remaining budget is 0 (100 - 2 fee = 98, payout = 98, remaining = 0).
        assert_eq!(s.remaining_budget(), 0);
        let result = s.record_milestone(2, worker("w2"), 0.5);
        assert!(result.is_err());
        match result.unwrap_err() {
            TrainingError::EscrowError(msg) => {
                assert!(msg.contains("milestone exceeds remaining budget"));
            }
            other => panic!("expected EscrowError, got {other}"),
        }
    }

    // ── Status transition tests ─────────────────────────────────────────

    #[test]
    fn status_transitions_to_partial_on_milestone() {
        let mut s = TrainingSettlement::new("job-1", "req", 10_000);
        assert_eq!(s.status, SettlementStatus::Pending);
        s.record_milestone(1, worker("w1"), 0.5).unwrap();
        assert_eq!(s.status, SettlementStatus::Partial { paid_steps: 1 });
    }

    #[test]
    fn finalize_marks_completed() {
        let mut s = TrainingSettlement::new("job-1", "req", 10_000);
        s.record_milestone(1, worker("w1"), 0.5).unwrap();
        s.finalize().unwrap();
        match &s.status {
            SettlementStatus::Completed { total_paid } => {
                assert_eq!(*total_paid, 4_900);
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[test]
    fn finalize_twice_errors() {
        let mut s = TrainingSettlement::new("job-1", "req", 10_000);
        s.finalize().unwrap();
        let err = s.finalize().unwrap_err();
        match err {
            TrainingError::EscrowError(msg) => {
                assert!(msg.contains("settlement already finalized"));
            }
            other => panic!("expected EscrowError, got {other}"),
        }
    }

    #[test]
    fn refund_from_pending() {
        let mut s = TrainingSettlement::new("job-1", "req", 10_000);
        s.refund().unwrap();
        assert_eq!(s.status, SettlementStatus::Refunded);
    }

    #[test]
    fn refund_from_partial() {
        let mut s = TrainingSettlement::new("job-1", "req", 10_000);
        s.record_milestone(1, worker("w1"), 0.3).unwrap();
        s.refund().unwrap();
        assert_eq!(s.status, SettlementStatus::Refunded);
    }

    #[test]
    fn refund_after_finalize_errors() {
        let mut s = TrainingSettlement::new("job-1", "req", 10_000);
        s.finalize().unwrap();
        let err = s.refund().unwrap_err();
        match err {
            TrainingError::EscrowError(msg) => {
                assert!(msg.contains("cannot refund completed settlement"));
            }
            other => panic!("expected EscrowError, got {other}"),
        }
    }

    #[test]
    fn refund_after_refund_errors() {
        let mut s = TrainingSettlement::new("job-1", "req", 10_000);
        s.refund().unwrap();
        let err = s.refund().unwrap_err();
        assert!(matches!(err, TrainingError::EscrowError(_)));
    }

    // ── Edge case tests ─────────────────────────────────────────────────

    #[test]
    fn contribution_score_zero_fails() {
        let mut s = TrainingSettlement::new("job-1", "req", 10_000);
        // payout = 10000 * 0.98 * 0.0 = 0, which fails the zero-payout guard.
        let result = s.record_milestone(1, worker("w1"), 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn large_deposit_milestone() {
        let mut s = TrainingSettlement::new("job-1", "req", 1_000_000_000);
        let payout = s.record_milestone(1, worker("w1"), 0.1).unwrap();
        // payout = 1_000_000_000 * 0.98 * 0.1 = 98_000_000
        assert_eq!(payout, 98_000_000);
        assert_eq!(s.protocol_fee(), 20_000_000); // 2% of 1B
    }

    // ── Serde roundtrip tests ───────────────────────────────────────────

    #[test]
    fn milestone_serde_roundtrip() {
        let m = Milestone {
            step: 42,
            worker_id: worker("w1"),
            contribution_score: 0.75,
            token_amount: 7350,
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: Milestone = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn settlement_status_serde_roundtrip() {
        let statuses = vec![
            SettlementStatus::Pending,
            SettlementStatus::Partial { paid_steps: 3 },
            SettlementStatus::Completed { total_paid: 5000 },
            SettlementStatus::Refunded,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let back: SettlementStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, back);
        }
    }

    #[test]
    fn training_settlement_serde_roundtrip() {
        let mut s = TrainingSettlement::new("job-42", "did:neunode:bob", 50_000);
        s.record_milestone(1, worker("w1"), 0.4).unwrap();
        let json = serde_json::to_string(&s).unwrap();
        let back: TrainingSettlement = serde_json::from_str(&json).unwrap();
        assert_eq!(s.job_id, back.job_id);
        assert_eq!(s.requester, back.requester);
        assert_eq!(s.total_deposit, back.total_deposit);
        assert_eq!(s.milestones.len(), back.milestones.len());
        assert_eq!(s.milestones[0].step, back.milestones[0].step);
        assert_eq!(s.milestones[0].token_amount, back.milestones[0].token_amount);
        assert!((s.protocol_fee_pct - back.protocol_fee_pct).abs() < f64::EPSILON);
    }

    // ── ts-rs export tests ──────────────────────────────────────────────

    #[test]
    fn ts_export_milestone() {
        use ts_rs::Config;
        let name = Milestone::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_settlement_status() {
        use ts_rs::Config;
        let name = SettlementStatus::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_training_settlement() {
        use ts_rs::Config;
        let name = TrainingSettlement::name(&Config::new());
        assert!(!name.is_empty());
    }

    // ── SettlementStatus snake_case serde ────────────────────────────────

    #[test]
    fn settlement_status_snake_case() {
        let json = serde_json::to_string(&SettlementStatus::Pending).unwrap();
        assert!(json.contains("pending"), "got: {json}");
        assert!(!json.contains("Pending"), "should be snake_case: {json}");
    }
}
