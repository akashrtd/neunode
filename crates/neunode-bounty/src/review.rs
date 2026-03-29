use neunode_core::constants::bounty::{
    REVIEWER_WEIGHT_AVAILABILITY, REVIEWER_WEIGHT_CAPABILITY, REVIEWER_WEIGHT_RANDOM,
    REVIEWER_WEIGHT_REPUTATION, REVIEWER_WEIGHT_STAKE,
};
use neunode_core::types::{Did, Signature, Timestamp};
use serde::{Deserialize, Serialize};

use crate::error::{BountyError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Review {
    pub reviewer: Did,
    pub score: u8,
    pub notes: String,
    pub submitted_at: Timestamp,
    pub signature: Option<Signature>,
}

impl Review {
    pub fn new(
        reviewer: Did,
        score: u8,
        notes: String,
        submitted_at: Timestamp,
        signature: Option<Signature>,
    ) -> Result<Self> {
        if score > 100 {
            return Err(BountyError::InvalidScore(score));
        }
        Ok(Self { reviewer, score, notes, submitted_at, signature })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewOutcome {
    Approved,
    Rejected,
    NeedsRevision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewCommittee {
    pub reviewers: Vec<Did>,
    pub reviews: Vec<Review>,
    pub required_count: usize,
}

impl ReviewCommittee {
    pub fn new(reviewers: Vec<Did>, required_count: usize) -> Self {
        Self { reviewers, reviews: Vec::new(), required_count }
    }

    pub fn submit_review(&mut self, review: Review) -> Result<()> {
        if !self.reviewers.iter().any(|r| r == &review.reviewer) {
            return Err(BountyError::ReviewerNotOnCommittee(review.reviewer.to_string()));
        }
        if self.reviews.iter().any(|r| r.reviewer == review.reviewer) {
            return Err(BountyError::DuplicateReviewer(review.reviewer.to_string()));
        }
        if review.score > 100 {
            return Err(BountyError::InvalidScore(review.score));
        }
        self.reviews.push(review);
        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        self.reviews.len() >= self.required_count
    }

    pub fn outcome(&self) -> Option<ReviewOutcome> {
        if !self.is_complete() {
            return None;
        }

        let approved = self.reviews.iter().filter(|r| r.score >= 60).count();
        let rejected = self.reviews.iter().filter(|r| r.score < 40).count();

        if approved > self.reviews.len() / 2 {
            Some(ReviewOutcome::Approved)
        } else if rejected > self.reviews.len() / 2 {
            Some(ReviewOutcome::Rejected)
        } else {
            Some(ReviewOutcome::NeedsRevision)
        }
    }

    pub fn average_score(&self) -> f64 {
        if self.reviews.is_empty() {
            return 0.0;
        }
        let total: u32 = self.reviews.iter().map(|r| r.score as u32).sum();
        total as f64 / self.reviews.len() as f64
    }
}

/// Select `count` reviewers from candidates weighted by composite score.
/// `scores` maps each candidate DID to their pre-computed weighted score.
/// Uses deterministic sorted selection (highest scores first), with
/// REVIEWER_WEIGHT_RANDOM portion left for protocol-level randomization.
pub fn select_reviewers(
    candidates: &[Did],
    scores: &std::collections::HashMap<Did, f64>,
    count: usize,
) -> Vec<Did> {
    let mut scored: Vec<(Did, f64)> =
        candidates.iter().filter_map(|did| scores.get(did).map(|&s| (did.clone(), s))).collect();

    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.to_string().cmp(&b.0.to_string()))
    });

    scored.into_iter().take(count).map(|(did, _)| did).collect()
}

/// Compute a weighted reviewer selection score from individual factor scores (0-100 each).
pub fn compute_reviewer_score(
    capability: f64,
    reputation: f64,
    stake: f64,
    availability: f64,
    random: f64,
) -> f64 {
    (REVIEWER_WEIGHT_CAPABILITY / 100.0 * capability
        + REVIEWER_WEIGHT_REPUTATION / 100.0 * reputation
        + REVIEWER_WEIGHT_STAKE / 100.0 * stake
        + REVIEWER_WEIGHT_AVAILABILITY / 100.0 * availability
        + REVIEWER_WEIGHT_RANDOM / 100.0 * random)
        .clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_did(name: &str) -> Did {
        Did(format!("did:neunode:{name}"))
    }

    fn make_review(reviewer: &str, score: u8) -> Review {
        Review::new(test_did(reviewer), score, format!("review by {reviewer}"), 1000, None).unwrap()
    }

    #[test]
    fn review_new_valid() {
        let r = Review::new(test_did("alice"), 85, "good".to_string(), 1000, None).unwrap();
        assert_eq!(r.score, 85);
        assert_eq!(r.reviewer, test_did("alice"));
    }

    #[test]
    fn review_new_invalid_score() {
        let err = Review::new(test_did("alice"), 101, "bad".to_string(), 1000, None).unwrap_err();
        assert!(matches!(err, BountyError::InvalidScore(101)));
    }

    #[test]
    fn review_new_boundary_scores() {
        Review::new(test_did("a"), 0, String::new(), 1000, None).unwrap();
        Review::new(test_did("a"), 100, String::new(), 1000, None).unwrap();
    }

    #[test]
    fn review_serde_roundtrip() {
        let r = make_review("alice", 85);
        let json = serde_json::to_string(&r).unwrap();
        let back: Review = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn committee_submit_review() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        committee.submit_review(make_review("r1", 80)).unwrap();
        assert_eq!(committee.reviews.len(), 1);
        assert!(!committee.is_complete());
    }

    #[test]
    fn committee_complete() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        committee.submit_review(make_review("r1", 80)).unwrap();
        committee.submit_review(make_review("r2", 75)).unwrap();
        committee.submit_review(make_review("r3", 90)).unwrap();
        assert!(committee.is_complete());
    }

    #[test]
    fn committee_outcome_approved() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        committee.submit_review(make_review("r1", 80)).unwrap();
        committee.submit_review(make_review("r2", 75)).unwrap();
        committee.submit_review(make_review("r3", 90)).unwrap();
        assert_eq!(committee.outcome(), Some(ReviewOutcome::Approved));
    }

    #[test]
    fn committee_outcome_rejected() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        committee.submit_review(make_review("r1", 20)).unwrap();
        committee.submit_review(make_review("r2", 30)).unwrap();
        committee.submit_review(make_review("r3", 10)).unwrap();
        assert_eq!(committee.outcome(), Some(ReviewOutcome::Rejected));
    }

    #[test]
    fn committee_outcome_needs_revision() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        committee.submit_review(make_review("r1", 80)).unwrap();
        committee.submit_review(make_review("r2", 30)).unwrap();
        committee.submit_review(make_review("r3", 50)).unwrap();
        assert_eq!(committee.outcome(), Some(ReviewOutcome::NeedsRevision));
    }

    #[test]
    fn committee_outcome_incomplete() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        committee.submit_review(make_review("r1", 80)).unwrap();
        assert_eq!(committee.outcome(), None);
    }

    #[test]
    fn committee_duplicate_reviewer() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        committee.submit_review(make_review("r1", 80)).unwrap();
        let err = committee.submit_review(make_review("r1", 90)).unwrap_err();
        assert!(matches!(err, BountyError::DuplicateReviewer(_)));
    }

    #[test]
    fn committee_not_on_committee() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        let err = committee.submit_review(make_review("stranger", 80)).unwrap_err();
        assert!(matches!(err, BountyError::ReviewerNotOnCommittee(_)));
    }

    #[test]
    fn committee_average_score() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        committee.submit_review(make_review("r1", 80)).unwrap();
        committee.submit_review(make_review("r2", 90)).unwrap();
        committee.submit_review(make_review("r3", 70)).unwrap();
        let avg = committee.average_score();
        assert!((avg - 80.0).abs() < f64::EPSILON);
    }

    #[test]
    fn committee_average_score_empty() {
        let committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        assert!((committee.average_score() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn committee_serde_roundtrip() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        committee.submit_review(make_review("r1", 85)).unwrap();
        let json = serde_json::to_string(&committee).unwrap();
        let back: ReviewCommittee = serde_json::from_str(&json).unwrap();
        assert_eq!(committee, back);
    }

    #[test]
    fn review_outcome_serde_roundtrip() {
        for outcome in
            [ReviewOutcome::Approved, ReviewOutcome::Rejected, ReviewOutcome::NeedsRevision]
        {
            let json = serde_json::to_string(&outcome).unwrap();
            let back: ReviewOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(outcome, back);
        }
    }

    #[test]
    fn select_reviewers_basic() {
        let candidates =
            vec![test_did("a"), test_did("b"), test_did("c"), test_did("d"), test_did("e")];
        let mut scores = std::collections::HashMap::new();
        scores.insert(test_did("a"), 90.0);
        scores.insert(test_did("b"), 80.0);
        scores.insert(test_did("c"), 70.0);
        scores.insert(test_did("d"), 60.0);
        scores.insert(test_did("e"), 50.0);

        let selected = select_reviewers(&candidates, &scores, 3);
        assert_eq!(selected.len(), 3);
        assert_eq!(selected[0], test_did("a"));
        assert_eq!(selected[1], test_did("b"));
        assert_eq!(selected[2], test_did("c"));
    }

    #[test]
    fn select_reviewers_fewer_candidates_than_count() {
        let candidates = vec![test_did("a"), test_did("b")];
        let mut scores = std::collections::HashMap::new();
        scores.insert(test_did("a"), 90.0);
        scores.insert(test_did("b"), 80.0);

        let selected = select_reviewers(&candidates, &scores, 5);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn select_reviewers_no_scores() {
        let candidates = vec![test_did("a"), test_did("b")];
        let scores = std::collections::HashMap::new();
        let selected = select_reviewers(&candidates, &scores, 3);
        assert!(selected.is_empty());
    }

    #[test]
    fn select_reviewers_partial_scores() {
        let candidates = vec![test_did("a"), test_did("b"), test_did("c")];
        let mut scores = std::collections::HashMap::new();
        scores.insert(test_did("a"), 90.0);
        scores.insert(test_did("c"), 70.0);

        let selected = select_reviewers(&candidates, &scores, 3);
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0], test_did("a"));
    }

    #[test]
    fn select_reviewers_deterministic_tiebreak() {
        let candidates = vec![test_did("alpha"), test_did("beta")];
        let mut scores = std::collections::HashMap::new();
        scores.insert(test_did("alpha"), 80.0);
        scores.insert(test_did("beta"), 80.0);

        let selected = select_reviewers(&candidates, &scores, 2);
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0], test_did("alpha"));
        assert_eq!(selected[1], test_did("beta"));
    }

    #[test]
    fn compute_reviewer_score_weights() {
        let score = compute_reviewer_score(100.0, 100.0, 100.0, 100.0, 100.0);
        assert!((score - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_reviewer_score_zero() {
        let score = compute_reviewer_score(0.0, 0.0, 0.0, 0.0, 0.0);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_reviewer_score_mixed() {
        let score = compute_reviewer_score(100.0, 50.0, 0.0, 75.0, 25.0);
        assert!(score > 0.0 && score < 100.0);
        let expected = 35.0 * 1.0 + 25.0 * 0.5 + 20.0 * 0.0 + 10.0 * 0.75 + 10.0 * 0.25;
        assert!((score - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn committee_two_of_three_approves() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        committee.submit_review(make_review("r1", 80)).unwrap();
        committee.submit_review(make_review("r2", 70)).unwrap();
        committee.submit_review(make_review("r3", 30)).unwrap();
        assert_eq!(committee.outcome(), Some(ReviewOutcome::Approved));
    }

    #[test]
    fn committee_all_perfect_approves() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        committee.submit_review(make_review("r1", 100)).unwrap();
        committee.submit_review(make_review("r2", 100)).unwrap();
        committee.submit_review(make_review("r3", 100)).unwrap();
        assert_eq!(committee.outcome(), Some(ReviewOutcome::Approved));
    }

    #[test]
    fn committee_all_zero_rejects() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        committee.submit_review(make_review("r1", 0)).unwrap();
        committee.submit_review(make_review("r2", 0)).unwrap();
        committee.submit_review(make_review("r3", 0)).unwrap();
        assert_eq!(committee.outcome(), Some(ReviewOutcome::Rejected));
    }

    #[test]
    fn committee_edge_score_59_needs_revision() {
        let mut committee =
            ReviewCommittee::new(vec![test_did("r1"), test_did("r2"), test_did("r3")], 3);
        committee.submit_review(make_review("r1", 59)).unwrap();
        committee.submit_review(make_review("r2", 40)).unwrap();
        committee.submit_review(make_review("r3", 50)).unwrap();
        assert_eq!(committee.outcome(), Some(ReviewOutcome::NeedsRevision));
    }
}
