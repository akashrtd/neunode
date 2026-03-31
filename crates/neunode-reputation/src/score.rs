use neunode_core::constants::reputation::MAX_SCORE;
use neunode_core::TokenAmount;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::attestation::AttestationGraph;
use crate::factors::{
    compute_activity_factor, compute_attestation_factor, compute_stake_factor,
    compute_tenure_factor, compute_verify_factor, FactorWeights,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ReputationGrade {
    A,
    B,
    C,
    D,
    F,
}

impl ReputationGrade {
    pub fn from_score(score: f64) -> Self {
        if score >= 90.0 {
            Self::A
        } else if score >= 75.0 {
            Self::B
        } else if score >= 50.0 {
            Self::C
        } else if score >= 25.0 {
            Self::D
        } else {
            Self::F
        }
    }

    pub fn min_score(&self) -> f64 {
        match self {
            Self::A => 90.0,
            Self::B => 75.0,
            Self::C => 50.0,
            Self::D => 25.0,
            Self::F => 0.0,
        }
    }
}

impl std::fmt::Display for ReputationGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A => write!(f, "A (90-100)"),
            Self::B => write!(f, "B (75-89)"),
            Self::C => write!(f, "C (50-74)"),
            Self::D => write!(f, "D (25-49)"),
            Self::F => write!(f, "F (0-24)"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct FactorInputs {
    pub staked_amount: TokenAmount,
    pub total_staked: TokenAmount,
    pub attestation_count: u32,
    pub avg_attestation_score: f64,
    pub events_per_day: f64,
    pub days_active: u32,
    pub tasks_completed: u32,
    pub tasks_failed: u32,
    pub days_since_creation: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct ReputationScore {
    pub total: f64,
    pub stake_factor: f64,
    pub attest_factor: f64,
    pub activity_factor: f64,
    pub verify_factor: f64,
    pub tenure_factor: f64,
    pub updated_at: neunode_core::Timestamp,
}

impl ReputationScore {
    pub fn compute(weights: &FactorWeights, factors: &FactorInputs) -> Self {
        let stake_factor = compute_stake_factor(factors.staked_amount, factors.total_staked);
        let attest_factor =
            compute_attestation_factor(factors.attestation_count, factors.avg_attestation_score);
        let activity_factor = compute_activity_factor(factors.events_per_day, factors.days_active);
        let verify_factor = compute_verify_factor(factors.tasks_completed, factors.tasks_failed);
        let tenure_factor = compute_tenure_factor(factors.days_since_creation);

        let total = (weights.stake / 100.0 * stake_factor
            + weights.attest / 100.0 * attest_factor
            + weights.activity / 100.0 * activity_factor
            + weights.verify / 100.0 * verify_factor
            + weights.tenure / 100.0 * tenure_factor)
            .min(MAX_SCORE);

        let updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            total,
            stake_factor,
            attest_factor,
            activity_factor,
            verify_factor,
            tenure_factor,
            updated_at,
        }
    }

    pub fn compute_default(factors: &FactorInputs) -> Self {
        Self::compute(&FactorWeights::default(), factors)
    }

    pub fn from_inputs(factors: &FactorInputs) -> Self {
        Self::compute_default(factors)
    }

    pub fn from_graph_inputs(
        weights: &FactorWeights,
        factors: &FactorInputs,
        graph: &AttestationGraph,
        did: &neunode_core::Did,
    ) -> Self {
        let attestation_count = graph.attestation_count(did) as u32;
        let avg_attestation_score = graph.avg_score(did);
        let enriched = FactorInputs { attestation_count, avg_attestation_score, ..*factors };
        Self::compute(weights, &enriched)
    }

    pub fn grade(&self) -> ReputationGrade {
        ReputationGrade::from_score(self.total)
    }

    pub fn serialize_summary(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perfect_inputs() -> FactorInputs {
        FactorInputs {
            staked_amount: TokenAmount(1000),
            total_staked: TokenAmount(1000),
            attestation_count: 100,
            avg_attestation_score: 100.0,
            events_per_day: 50.0,
            days_active: 365,
            tasks_completed: 200,
            tasks_failed: 0,
            days_since_creation: 365,
        }
    }

    fn zero_inputs() -> FactorInputs {
        FactorInputs {
            staked_amount: TokenAmount(0),
            total_staked: TokenAmount(1000),
            attestation_count: 0,
            avg_attestation_score: 0.0,
            events_per_day: 0.0,
            days_active: 0,
            tasks_completed: 0,
            tasks_failed: 0,
            days_since_creation: 0,
        }
    }

    #[test]
    fn grade_a() {
        assert_eq!(ReputationGrade::from_score(90.0), ReputationGrade::A);
        assert_eq!(ReputationGrade::from_score(95.0), ReputationGrade::A);
        assert_eq!(ReputationGrade::from_score(100.0), ReputationGrade::A);
    }

    #[test]
    fn grade_b() {
        assert_eq!(ReputationGrade::from_score(75.0), ReputationGrade::B);
        assert_eq!(ReputationGrade::from_score(89.0), ReputationGrade::B);
        assert_eq!(ReputationGrade::from_score(82.0), ReputationGrade::B);
    }

    #[test]
    fn grade_c() {
        assert_eq!(ReputationGrade::from_score(50.0), ReputationGrade::C);
        assert_eq!(ReputationGrade::from_score(74.0), ReputationGrade::C);
    }

    #[test]
    fn grade_d() {
        assert_eq!(ReputationGrade::from_score(25.0), ReputationGrade::D);
        assert_eq!(ReputationGrade::from_score(49.0), ReputationGrade::D);
    }

    #[test]
    fn grade_f() {
        assert_eq!(ReputationGrade::from_score(0.0), ReputationGrade::F);
        assert_eq!(ReputationGrade::from_score(24.0), ReputationGrade::F);
    }

    #[test]
    fn grade_boundary_89() {
        assert_eq!(ReputationGrade::from_score(89.99), ReputationGrade::B);
    }

    #[test]
    fn grade_min_scores() {
        assert!((ReputationGrade::A.min_score() - 90.0).abs() < f64::EPSILON);
        assert!((ReputationGrade::B.min_score() - 75.0).abs() < f64::EPSILON);
        assert!((ReputationGrade::C.min_score() - 50.0).abs() < f64::EPSILON);
        assert!((ReputationGrade::D.min_score() - 25.0).abs() < f64::EPSILON);
        assert!((ReputationGrade::F.min_score() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn grade_display() {
        assert_eq!(format!("{}", ReputationGrade::A), "A (90-100)");
        assert_eq!(format!("{}", ReputationGrade::B), "B (75-89)");
        assert_eq!(format!("{}", ReputationGrade::C), "C (50-74)");
        assert_eq!(format!("{}", ReputationGrade::D), "D (25-49)");
        assert_eq!(format!("{}", ReputationGrade::F), "F (0-24)");
    }

    #[test]
    fn grade_serde_roundtrip() {
        for grade in [
            ReputationGrade::A,
            ReputationGrade::B,
            ReputationGrade::C,
            ReputationGrade::D,
            ReputationGrade::F,
        ] {
            let json = serde_json::to_string(&grade).unwrap();
            let back: ReputationGrade = serde_json::from_str(&json).unwrap();
            assert_eq!(grade, back);
        }
    }

    #[test]
    fn score_from_zero_inputs() {
        let score = ReputationScore::compute_default(&zero_inputs());
        assert!((score.total - 0.0).abs() < f64::EPSILON);
        assert_eq!(score.grade(), ReputationGrade::F);
    }

    #[test]
    fn score_from_perfect_inputs() {
        let score = ReputationScore::compute_default(&perfect_inputs());
        assert!(score.total > 80.0);
        assert!(score.total <= 100.0);
    }

    #[test]
    fn score_capped_at_max() {
        let score = ReputationScore::compute_default(&perfect_inputs());
        assert!(score.total <= MAX_SCORE);
    }

    #[test]
    fn score_with_custom_weights() {
        let weights = FactorWeights::new(50.0, 10.0, 10.0, 10.0, 20.0);
        let score = ReputationScore::compute(&weights, &perfect_inputs());
        assert!(score.total > 0.0 && score.total <= 100.0);
    }

    #[test]
    fn score_individual_factors_populated() {
        let score = ReputationScore::compute_default(&perfect_inputs());
        assert!(score.stake_factor > 0.0);
        assert!(score.attest_factor > 0.0);
        assert!(score.activity_factor > 0.0);
        assert!(score.verify_factor > 0.0);
        assert!(score.tenure_factor > 0.0);
    }

    #[test]
    fn score_from_inputs_alias() {
        let s1 = ReputationScore::compute_default(&perfect_inputs());
        let s2 = ReputationScore::from_inputs(&perfect_inputs());
        assert!((s1.total - s2.total).abs() < f64::EPSILON);
    }

    #[test]
    fn score_updated_at_set() {
        let score = ReputationScore::compute_default(&perfect_inputs());
        assert!(score.updated_at > 0);
    }

    #[test]
    fn score_serialize_summary_valid_json() {
        let score = ReputationScore::compute_default(&perfect_inputs());
        let json = score.serialize_summary();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());
        assert!(parsed.get("total").is_some());
    }

    #[test]
    fn score_serde_roundtrip() {
        let score = ReputationScore::compute_default(&perfect_inputs());
        let json = serde_json::to_string(&score).unwrap();
        let back: ReputationScore = serde_json::from_str(&json).unwrap();
        assert_eq!(score, back);
    }

    #[test]
    fn factor_inputs_serde_roundtrip() {
        let inputs = perfect_inputs();
        let json = serde_json::to_string(&inputs).unwrap();
        let back: FactorInputs = serde_json::from_str(&json).unwrap();
        assert_eq!(inputs, back);
    }

    #[test]
    fn score_grade_integration() {
        let mut inputs = zero_inputs();
        inputs.staked_amount = TokenAmount(500);
        inputs.total_staked = TokenAmount(1000);
        inputs.attestation_count = 50;
        inputs.avg_attestation_score = 90.0;
        inputs.events_per_day = 20.0;
        inputs.days_active = 180;
        inputs.tasks_completed = 50;
        inputs.tasks_failed = 5;
        inputs.days_since_creation = 180;

        let score = ReputationScore::compute_default(&inputs);
        assert!(score.total > 30.0);
        assert!(score.total < 100.0);
    }

    #[test]
    fn score_stake_heavy() {
        let weights = FactorWeights::new(100.0, 0.0, 0.0, 0.0, 0.0);
        let inputs = FactorInputs {
            staked_amount: TokenAmount(500),
            total_staked: TokenAmount(1000),
            ..zero_inputs()
        };
        let score = ReputationScore::compute(&weights, &inputs);
        assert!((score.total - 50.0).abs() < 1.0);
    }

    #[test]
    fn score_from_graph_inputs() {
        use crate::attestation::Attestation;
        use neunode_core::Hash256;

        let mut graph = crate::attestation::AttestationGraph::new();
        let alice = neunode_core::Did("did:neunode:alice".to_string());
        let bob = neunode_core::Did("did:neunode:bob".to_string());
        let hash = Hash256("abc".to_string());

        let att =
            Attestation::new(alice, bob.clone(), "good work".to_string(), 85.0, hash).unwrap();
        graph.add_attestation(att).unwrap();

        let factors = FactorInputs {
            staked_amount: TokenAmount(500),
            total_staked: TokenAmount(1000),
            attestation_count: 0,
            avg_attestation_score: 0.0,
            events_per_day: 10.0,
            days_active: 90,
            tasks_completed: 20,
            tasks_failed: 2,
            days_since_creation: 90,
        };

        let score =
            ReputationScore::from_graph_inputs(&FactorWeights::default(), &factors, &graph, &bob);
        assert!(score.total > 0.0);
        assert!(score.attest_factor > 0.0);
    }
}
