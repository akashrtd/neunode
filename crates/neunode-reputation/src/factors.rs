use neunode_core::constants::reputation::{
    WEIGHT_ACTIVITY, WEIGHT_ATTEST, WEIGHT_STAKE, WEIGHT_TENURE, WEIGHT_VERIFY,
};
use neunode_core::TokenAmount;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct FactorWeights {
    pub stake: f64,
    pub attest: f64,
    pub activity: f64,
    pub verify: f64,
    pub tenure: f64,
}

impl FactorWeights {
    pub fn new(stake: f64, attest: f64, activity: f64, verify: f64, tenure: f64) -> Self {
        Self { stake, attest, activity, verify, tenure }
    }

    pub fn sum(&self) -> f64 {
        self.stake + self.attest + self.activity + self.verify + self.tenure
    }

    pub fn is_normalized(&self) -> bool {
        (self.sum() - 100.0).abs() < f64::EPSILON
    }
}

impl Default for FactorWeights {
    fn default() -> Self {
        Self {
            stake: WEIGHT_STAKE,
            attest: WEIGHT_ATTEST,
            activity: WEIGHT_ACTIVITY,
            verify: WEIGHT_VERIFY,
            tenure: WEIGHT_TENURE,
        }
    }
}

pub fn compute_stake_factor(staked_amount: TokenAmount, total_staked: TokenAmount) -> f64 {
    if total_staked.0 == 0 {
        return 0.0;
    }
    let ratio = staked_amount.0 as f64 / total_staked.0 as f64;
    (ratio * 100.0).min(100.0)
}

pub fn compute_attestation_factor(attestation_count: u32, avg_score: f64) -> f64 {
    let count_factor = (attestation_count as f64).ln_1p() / 10.0_f64.ln_1p() * 100.0;
    let clamped_count = count_factor.min(100.0);
    (clamped_count * (avg_score / 100.0)).min(100.0)
}

pub fn compute_activity_factor(events_per_day: f64, days_active: u32) -> f64 {
    if days_active == 0 {
        return 0.0;
    }
    let frequency = events_per_day.min(50.0);
    let consistency = (days_active as f64).ln_1p() / 365.0_f64.ln_1p() * 100.0;
    let raw = (frequency / 50.0) * 50.0 + consistency * 0.5;
    raw.min(100.0)
}

pub fn compute_verify_factor(tasks_completed: u32, tasks_failed: u32) -> f64 {
    let total = tasks_completed + tasks_failed;
    if total == 0 {
        return 0.0;
    }
    let success_rate = tasks_completed as f64 / total as f64;
    let volume_bonus = (tasks_completed as f64).ln_1p() / 100.0_f64.ln_1p() * 10.0;
    (success_rate * 90.0 + volume_bonus).min(100.0)
}

pub fn compute_tenure_factor(days_since_creation: u32) -> f64 {
    if days_since_creation == 0 {
        return 0.0;
    }
    (days_since_creation as f64).ln_1p() / 365.0_f64.ln_1p() * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_weights_default_values() {
        let w = FactorWeights::default();
        assert!((w.stake - 30.0).abs() < f64::EPSILON);
        assert!((w.attest - 25.0).abs() < f64::EPSILON);
        assert!((w.activity - 20.0).abs() < f64::EPSILON);
        assert!((w.verify - 15.0).abs() < f64::EPSILON);
        assert!((w.tenure - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn factor_weights_default_is_normalized() {
        let w = FactorWeights::default();
        assert!(w.is_normalized());
        assert!((w.sum() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn factor_weights_new_custom() {
        let w = FactorWeights::new(20.0, 20.0, 20.0, 20.0, 20.0);
        assert!((w.sum() - 100.0).abs() < f64::EPSILON);
        assert!(w.is_normalized());
    }

    #[test]
    fn factor_weights_serde_roundtrip() {
        let w = FactorWeights::default();
        let json = serde_json::to_string(&w).unwrap();
        let back: FactorWeights = serde_json::from_str(&json).unwrap();
        assert_eq!(w, back);
    }

    #[test]
    fn stake_factor_zero_total() {
        assert!(
            (compute_stake_factor(TokenAmount(100), TokenAmount(0)) - 0.0).abs() < f64::EPSILON
        );
    }

    #[test]
    fn stake_factor_proportional() {
        let score = compute_stake_factor(TokenAmount(500), TokenAmount(1000));
        assert!((score - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stake_factor_full_stake() {
        let score = compute_stake_factor(TokenAmount(1000), TokenAmount(1000));
        assert!((score - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stake_factor_zero_stake() {
        let score = compute_stake_factor(TokenAmount(0), TokenAmount(1000));
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stake_factor_caps_at_100() {
        let score = compute_stake_factor(TokenAmount(2000), TokenAmount(1000));
        assert!((score - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn attest_factor_no_attestations() {
        let score = compute_attestation_factor(0, 0.0);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn attest_factor_few_attestations_high_score() {
        let score = compute_attestation_factor(5, 80.0);
        assert!(score > 0.0 && score <= 100.0);
    }

    #[test]
    fn attest_factor_many_attestations_perfect_score() {
        let score = compute_attestation_factor(100, 100.0);
        assert!(score > 90.0);
    }

    #[test]
    fn attest_factor_capped_at_100() {
        let score = compute_attestation_factor(10000, 100.0);
        assert!(score <= 100.0);
    }

    #[test]
    fn activity_factor_zero_days() {
        assert!((compute_activity_factor(10.0, 0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn activity_factor_low_activity() {
        let score = compute_activity_factor(1.0, 7);
        assert!(score > 0.0 && score < 50.0);
    }

    #[test]
    fn activity_factor_high_activity() {
        let score = compute_activity_factor(50.0, 365);
        assert!(score > 50.0);
    }

    #[test]
    fn activity_factor_capped_at_100() {
        let score = compute_activity_factor(100.0, 1000);
        assert!(score <= 100.0);
    }

    #[test]
    fn verify_factor_no_tasks() {
        assert!((compute_verify_factor(0, 0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn verify_factor_perfect_rate() {
        let score = compute_verify_factor(100, 0);
        assert!(score > 90.0);
    }

    #[test]
    fn verify_factor_zero_success() {
        let score = compute_verify_factor(0, 100);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn verify_factor_mixed_rate() {
        let score = compute_verify_factor(80, 20);
        assert!(score > 60.0 && score < 95.0);
    }

    #[test]
    fn verify_factor_capped_at_100() {
        let score = compute_verify_factor(10000, 0);
        assert!(score <= 100.0);
    }

    #[test]
    fn tenure_factor_day_zero() {
        assert!((compute_tenure_factor(0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tenure_factor_day_one() {
        let score = compute_tenure_factor(1);
        assert!(score > 0.0 && score < 20.0);
    }

    #[test]
    fn tenure_factor_one_year() {
        let score = compute_tenure_factor(365);
        assert!((score - 100.0).abs() < 1.0);
    }

    #[test]
    fn tenure_factor_monotonic_growth() {
        let s1 = compute_tenure_factor(30);
        let s2 = compute_tenure_factor(90);
        let s3 = compute_tenure_factor(365);
        assert!(s1 < s2);
        assert!(s2 < s3);
    }
}
