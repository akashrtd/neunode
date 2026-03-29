use std::collections::HashMap;

use neunode_core::constants::bounty::{
    DEFAULT_CLAIM_DEADLINE_SECS, DEFAULT_DISPUTE_DEADLINE_SECS, DEFAULT_REVIEW_DEADLINE_SECS,
    DEFAULT_REVISION_DEADLINE_SECS, DEFAULT_WORK_DEADLINE_SECS, REVIEWER_COUNT,
};
use neunode_core::types::{BountyId, BountyState, Did, Hash256, Timestamp, TokenAmount, TokenType};
use serde::{Deserialize, Serialize};

use crate::error::{BountyError, Result};
use crate::escrow::{EscrowManager, FeeBreakdown};
use crate::review::{Review, ReviewCommittee, ReviewOutcome};
use crate::state_machine::{BountyData, BountyEvent, BountyStateMachine, Deadlines};

/// Deadline configuration for a bounty lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BountyDeadlines {
    pub created_at: Timestamp,
    pub claim_deadline: Timestamp,
    pub work_deadline: Timestamp,
    pub review_deadline: Timestamp,
    pub revision_deadline: Timestamp,
    pub dispute_deadline: Timestamp,
    pub grace_period_secs: u64,
}

impl BountyDeadlines {
    /// Create deadlines using default constants from the moment of creation.
    pub fn new(created_at: Timestamp) -> Self {
        Self {
            created_at,
            claim_deadline: created_at.saturating_add(DEFAULT_CLAIM_DEADLINE_SECS),
            work_deadline: created_at.saturating_add(DEFAULT_WORK_DEADLINE_SECS),
            review_deadline: created_at
                .saturating_add(DEFAULT_WORK_DEADLINE_SECS)
                .saturating_add(DEFAULT_REVIEW_DEADLINE_SECS),
            revision_deadline: created_at
                .saturating_add(DEFAULT_WORK_DEADLINE_SECS)
                .saturating_add(DEFAULT_REVIEW_DEADLINE_SECS)
                .saturating_add(DEFAULT_REVISION_DEADLINE_SECS),
            dispute_deadline: created_at
                .saturating_add(DEFAULT_WORK_DEADLINE_SECS)
                .saturating_add(DEFAULT_REVIEW_DEADLINE_SECS)
                .saturating_add(DEFAULT_DISPUTE_DEADLINE_SECS),
            grace_period_secs: 3600,
        }
    }

    /// Builder: override claim deadline as duration from creation.
    pub fn with_claim_deadline(mut self, secs: u64) -> Self {
        self.claim_deadline = self.created_at.saturating_add(secs);
        self
    }

    /// Builder: override work deadline as duration from creation.
    pub fn with_work_deadline(mut self, secs: u64) -> Self {
        self.work_deadline = self.created_at.saturating_add(secs);
        self
    }

    /// Builder: override review deadline as duration from creation.
    pub fn with_review_deadline(mut self, secs: u64) -> Self {
        self.review_deadline = self.created_at.saturating_add(secs);
        self
    }

    /// Builder: override revision deadline as duration from creation.
    pub fn with_revision_deadline(mut self, secs: u64) -> Self {
        self.revision_deadline = self.created_at.saturating_add(secs);
        self
    }

    /// Builder: override dispute deadline as duration from creation.
    pub fn with_dispute_deadline(mut self, secs: u64) -> Self {
        self.dispute_deadline = self.created_at.saturating_add(secs);
        self
    }

    /// Builder: override grace period.
    pub fn with_grace_period(mut self, secs: u64) -> Self {
        self.grace_period_secs = secs;
        self
    }

    /// Check if the claim period has expired (including grace).
    pub fn is_claim_expired(&self, now: Timestamp) -> bool {
        now > self.claim_deadline.saturating_add(self.grace_period_secs)
    }

    /// Check if the work period has expired (including grace).
    pub fn is_work_expired(&self, now: Timestamp) -> bool {
        now > self.work_deadline.saturating_add(self.grace_period_secs)
    }

    /// Check if the review period has expired (including grace).
    pub fn is_review_expired(&self, now: Timestamp) -> bool {
        now > self.review_deadline.saturating_add(self.grace_period_secs)
    }

    /// Check if the revision period has expired (including grace).
    pub fn is_revision_expired(&self, now: Timestamp) -> bool {
        now > self.revision_deadline.saturating_add(self.grace_period_secs)
    }

    /// Check if the dispute period has expired (including grace).
    pub fn is_dispute_expired(&self, now: Timestamp) -> bool {
        now > self.dispute_deadline.saturating_add(self.grace_period_secs)
    }

    /// Return the next applicable deadline for the given bounty state.
    ///
    /// Returns `(deadline_name, timestamp)` for the first unexpired deadline
    /// relevant to the current state, or `None` if no deadline applies.
    pub fn next_deadline(&self, state: BountyState, now: Timestamp) -> Option<(String, Timestamp)> {
        match state {
            BountyState::Open => {
                if now <= self.claim_deadline {
                    Some(("claim".to_string(), self.claim_deadline))
                } else {
                    None
                }
            }
            BountyState::Claimed => {
                if now <= self.work_deadline {
                    Some(("work".to_string(), self.work_deadline))
                } else {
                    None
                }
            }
            BountyState::Submitted => {
                if now <= self.review_deadline {
                    Some(("review".to_string(), self.review_deadline))
                } else {
                    None
                }
            }
            BountyState::UnderReview => {
                if now <= self.review_deadline {
                    Some(("review".to_string(), self.review_deadline))
                } else {
                    None
                }
            }
            BountyState::Revision => {
                if now <= self.revision_deadline {
                    Some(("revision".to_string(), self.revision_deadline))
                } else {
                    None
                }
            }
            BountyState::Disputed => {
                if now <= self.dispute_deadline {
                    Some(("dispute".to_string(), self.dispute_deadline))
                } else {
                    None
                }
            }
            // Terminal states: no deadlines apply.
            BountyState::Accepted
            | BountyState::Rejected
            | BountyState::Paid
            | BountyState::Expired
            | BountyState::Cancelled => None,
        }
    }
}

/// The full bounty record with all metadata and state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BountyRecord {
    pub id: BountyId,
    pub title: String,
    pub description: String,
    pub creator: Did,
    pub claimant: Option<Did>,
    pub reward: TokenAmount,
    pub reward_token: TokenType,
    pub state: BountyState,
    pub deadlines: BountyDeadlines,
    pub reviewers: Vec<Did>,
    pub artifact_cid: Option<String>,
    pub created_at: Timestamp,
}

impl BountyRecord {
    /// Create a new bounty record in the Open state.
    pub fn new(
        id: BountyId,
        title: String,
        description: String,
        creator: Did,
        reward: TokenAmount,
        reward_token: TokenType,
        created_at: Timestamp,
    ) -> Self {
        let deadlines = BountyDeadlines::new(created_at);
        Self {
            id,
            title,
            description,
            creator,
            claimant: None,
            reward,
            reward_token,
            state: BountyState::Open,
            deadlines,
            reviewers: Vec::new(),
            artifact_cid: None,
            created_at,
        }
    }

    /// Builder: override the entire deadlines config.
    pub fn with_deadlines(mut self, deadlines: BountyDeadlines) -> Self {
        self.deadlines = deadlines;
        self
    }

    /// Claim the bounty — transitions to Claimed state.
    pub fn claim(&mut self, claimant: Did, now: Timestamp) -> Result<()> {
        if self.state != BountyState::Open {
            return Err(BountyError::InvalidState(self.state));
        }
        if self.deadlines.is_claim_expired(now) {
            return Err(BountyError::DeadlineExceeded {
                deadline_type: "claim".to_string(),
                deadline: self.deadlines.claim_deadline,
                now,
            });
        }
        self.state = BountyState::Claimed;
        self.claimant = Some(claimant);
        Ok(())
    }

    /// Submit work artifact — transitions to Submitted.
    pub fn submit(&mut self, artifact_cid: String, now: Timestamp) -> Result<()> {
        match self.state {
            BountyState::Claimed | BountyState::Revision => {}
            _ => {
                return Err(BountyError::InvalidState(self.state));
            }
        }
        if self.state == BountyState::Claimed && self.deadlines.is_work_expired(now) {
            return Err(BountyError::DeadlineExceeded {
                deadline_type: "work".to_string(),
                deadline: self.deadlines.work_deadline,
                now,
            });
        }
        if self.state == BountyState::Revision && self.deadlines.is_revision_expired(now) {
            return Err(BountyError::DeadlineExceeded {
                deadline_type: "revision".to_string(),
                deadline: self.deadlines.revision_deadline,
                now,
            });
        }
        self.state = BountyState::Submitted;
        self.artifact_cid = Some(artifact_cid);
        Ok(())
    }

    /// Check for automatic expiry based on current time and state.
    pub fn check_expiry(&mut self, now: Timestamp) -> bool {
        let expired = match self.state {
            BountyState::Open => self.deadlines.is_claim_expired(now),
            BountyState::Claimed => self.deadlines.is_work_expired(now),
            BountyState::Revision => self.deadlines.is_revision_expired(now),
            _ => false,
        };
        if expired {
            self.state = BountyState::Expired;
        }
        expired
    }
}

#[derive(Debug)]
pub struct BountyManager {
    state_machines: HashMap<BountyId, BountyStateMachine>,
    escrow_manager: EscrowManager,
    review_committees: HashMap<BountyId, ReviewCommittee>,
    next_id: u64,
}

impl BountyManager {
    pub fn new() -> Self {
        Self {
            state_machines: HashMap::new(),
            escrow_manager: EscrowManager::new(),
            review_committees: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn create_bounty(
        &mut self,
        creator: Did,
        title: String,
        description: String,
        reward: TokenAmount,
        token_type: TokenType,
        now: Timestamp,
    ) -> BountyData {
        let id_str = format!("bnty_{:08x}", self.next_id);
        self.next_id += 1;
        let id = BountyId(id_str);

        let data = BountyData {
            id: id.clone(),
            creator: creator.clone(),
            title,
            description,
            reward_amount: reward,
            reward_token: token_type,
            state: BountyState::Open,
            claimant: None,
            created_at: now,
            deadlines: Deadlines::from_created_at(now),
            artifact_hash: None,
            bond: None,
        };

        let sm = BountyStateMachine::new(data.clone());
        self.state_machines.insert(id.clone(), sm);

        let _ = self.escrow_manager.create_escrow(id, creator, reward, token_type, now);

        data
    }

    pub fn claim_bounty(
        &mut self,
        bounty_id: &BountyId,
        claimant: Did,
        bond: TokenAmount,
        now: Timestamp,
    ) -> Result<()> {
        let sm = self
            .state_machines
            .get_mut(bounty_id)
            .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?;
        sm.try_transition(BountyEvent::Claim { claimant, bond }, now)
    }

    pub fn submit_work(
        &mut self,
        bounty_id: &BountyId,
        artifact_hash: Hash256,
        now: Timestamp,
    ) -> Result<()> {
        let sm = self
            .state_machines
            .get_mut(bounty_id)
            .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?;
        sm.try_transition(BountyEvent::Submit { artifact_hash }, now)
    }

    pub fn start_review(
        &mut self,
        bounty_id: &BountyId,
        reviewers: Vec<Did>,
        now: Timestamp,
    ) -> Result<()> {
        let sm = self
            .state_machines
            .get_mut(bounty_id)
            .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?;
        sm.try_transition(BountyEvent::StartReview, now)?;

        let committee = ReviewCommittee::new(reviewers, REVIEWER_COUNT);
        self.review_committees.insert(bounty_id.clone(), committee);
        Ok(())
    }

    pub fn submit_review(
        &mut self,
        bounty_id: &BountyId,
        review: Review,
        _now: Timestamp,
    ) -> Result<()> {
        let sm = self
            .state_machines
            .get(bounty_id)
            .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?;
        if sm.current_state() != BountyState::UnderReview {
            return Err(BountyError::InvalidState(sm.current_state()));
        }

        let committee = self
            .review_committees
            .get_mut(bounty_id)
            .ok_or_else(|| BountyError::NotFound(format!("no committee for {}", bounty_id)))?;
        committee.submit_review(review)
    }

    pub fn complete_review(
        &mut self,
        bounty_id: &BountyId,
        now: Timestamp,
    ) -> Result<ReviewOutcome> {
        let committee = self
            .review_committees
            .get(bounty_id)
            .ok_or_else(|| BountyError::NotFound(format!("no committee for {}", bounty_id)))?;

        if !committee.is_complete() {
            return Err(BountyError::ReviewIncomplete {
                submitted: committee.reviews.len(),
                required: committee.required_count,
            });
        }

        let outcome = committee
            .outcome()
            .ok_or_else(|| BountyError::EscrowError("review outcome undetermined".to_string()))?;

        let sm = self
            .state_machines
            .get_mut(bounty_id)
            .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?;

        match outcome {
            ReviewOutcome::Approved => {
                sm.try_transition(BountyEvent::Accept, now)?;
            }
            ReviewOutcome::Rejected => {
                sm.try_transition(BountyEvent::Reject, now)?;
            }
            ReviewOutcome::NeedsRevision => {
                sm.try_transition(BountyEvent::RequestRevision, now)?;
            }
        }

        Ok(outcome)
    }

    pub fn resolve_bounty(
        &mut self,
        bounty_id: &BountyId,
        accept: bool,
        now: Timestamp,
    ) -> Result<FeeBreakdown> {
        let sm = self
            .state_machines
            .get_mut(bounty_id)
            .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?;
        sm.try_transition(BountyEvent::Resolve { accept }, now)?;

        let fees = self
            .escrow_manager
            .calculate_fees(bounty_id)
            .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?;

        if accept {
            let claimant = {
                let data = self
                    .state_machines
                    .get(bounty_id)
                    .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?
                    .data()
                    .claimant
                    .clone();
                data.ok_or(BountyError::NotClaimed)?
            };
            self.escrow_manager.release(bounty_id, claimant)?;
        } else {
            self.escrow_manager.refund(bounty_id)?;
        }

        Ok(fees)
    }

    pub fn pay_bounty(&mut self, bounty_id: &BountyId, now: Timestamp) -> Result<FeeBreakdown> {
        let sm = self
            .state_machines
            .get_mut(bounty_id)
            .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?;
        sm.try_transition(BountyEvent::Pay, now)?;

        let claimant = {
            let data = self
                .state_machines
                .get(bounty_id)
                .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?
                .data()
                .claimant
                .clone();
            data.ok_or(BountyError::NotClaimed)?
        };

        self.escrow_manager.release(bounty_id, claimant)?;

        self.escrow_manager
            .calculate_fees(bounty_id)
            .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))
    }

    pub fn cancel_bounty(&mut self, bounty_id: &BountyId, now: Timestamp) -> Result<()> {
        let sm = self
            .state_machines
            .get_mut(bounty_id)
            .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?;
        sm.try_transition(BountyEvent::Cancel, now)?;

        if self.escrow_manager.get_escrow(bounty_id).is_some() {
            let _ = self.escrow_manager.refund(bounty_id);
        }

        Ok(())
    }

    pub fn get_bounty(&self, bounty_id: &BountyId) -> Option<&BountyData> {
        self.state_machines.get(bounty_id).map(|sm| sm.data())
    }

    pub fn get_state(&self, bounty_id: &BountyId) -> Option<BountyState> {
        self.state_machines.get(bounty_id).map(|sm| sm.current_state())
    }

    pub fn get_escrow_manager(&self) -> &EscrowManager {
        &self.escrow_manager
    }

    pub fn get_review_committee(&self, bounty_id: &BountyId) -> Option<&ReviewCommittee> {
        self.review_committees.get(bounty_id)
    }
}

impl Default for BountyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_did(name: &str) -> Did {
        Did(format!("did:neunode:{name}"))
    }

    fn test_bounty_id(name: &str) -> BountyId {
        BountyId(format!("bnty_{name}"))
    }

    fn base_time() -> Timestamp {
        1_700_000_000
    }

    // ── BountyDeadlines tests ──────────────────────────────────────────

    #[test]
    fn deadlines_new_uses_defaults() {
        let dl = BountyDeadlines::new(base_time());
        assert_eq!(dl.created_at, base_time());
        assert_eq!(dl.claim_deadline, base_time() + DEFAULT_CLAIM_DEADLINE_SECS);
        assert_eq!(dl.work_deadline, base_time() + DEFAULT_WORK_DEADLINE_SECS);
        assert_eq!(
            dl.review_deadline,
            base_time() + DEFAULT_WORK_DEADLINE_SECS + DEFAULT_REVIEW_DEADLINE_SECS
        );
        assert!(dl.revision_deadline > dl.review_deadline);
        assert!(dl.dispute_deadline > dl.review_deadline);
    }

    #[test]
    fn deadlines_builder_claim_override() {
        let dl = BountyDeadlines::new(base_time()).with_claim_deadline(3600);
        assert_eq!(dl.claim_deadline, base_time() + 3600);
    }

    #[test]
    fn deadlines_builder_work_override() {
        let dl = BountyDeadlines::new(base_time()).with_work_deadline(7200);
        assert_eq!(dl.work_deadline, base_time() + 7200);
    }

    #[test]
    fn deadlines_builder_review_override() {
        let dl = BountyDeadlines::new(base_time()).with_review_deadline(9999);
        assert_eq!(dl.review_deadline, base_time() + 9999);
    }

    #[test]
    fn deadlines_builder_revision_override() {
        let dl = BountyDeadlines::new(base_time()).with_revision_deadline(12345);
        assert_eq!(dl.revision_deadline, base_time() + 12345);
    }

    #[test]
    fn deadlines_builder_dispute_override() {
        let dl = BountyDeadlines::new(base_time()).with_dispute_deadline(55555);
        assert_eq!(dl.dispute_deadline, base_time() + 55555);
    }

    #[test]
    fn deadlines_builder_grace_period() {
        let dl = BountyDeadlines::new(base_time()).with_grace_period(7200);
        assert_eq!(dl.grace_period_secs, 7200);
    }

    #[test]
    fn deadlines_builder_chain() {
        let dl = BountyDeadlines::new(base_time())
            .with_claim_deadline(100)
            .with_work_deadline(200)
            .with_review_deadline(300)
            .with_grace_period(500);
        assert_eq!(dl.claim_deadline, base_time() + 100);
        assert_eq!(dl.work_deadline, base_time() + 200);
        assert_eq!(dl.review_deadline, base_time() + 300);
        assert_eq!(dl.grace_period_secs, 500);
    }

    #[test]
    fn is_claim_expired_false_before_deadline() {
        let dl = BountyDeadlines::new(base_time());
        assert!(!dl.is_claim_expired(base_time() + 100));
    }

    #[test]
    fn is_claim_expired_true_after_deadline_plus_grace() {
        let dl = BountyDeadlines::new(base_time());
        let past = base_time() + DEFAULT_CLAIM_DEADLINE_SECS + 3600 + 1;
        assert!(dl.is_claim_expired(past));
    }

    #[test]
    fn is_work_expired_false_before_deadline() {
        let dl = BountyDeadlines::new(base_time());
        assert!(!dl.is_work_expired(base_time() + 100));
    }

    #[test]
    fn is_work_expired_true_after_deadline_plus_grace() {
        let dl = BountyDeadlines::new(base_time());
        let past = base_time() + DEFAULT_WORK_DEADLINE_SECS + 3600 + 1;
        assert!(dl.is_work_expired(past));
    }

    #[test]
    fn is_review_expired_boundary() {
        let dl = BountyDeadlines::new(base_time());
        let deadline = base_time() + DEFAULT_WORK_DEADLINE_SECS + DEFAULT_REVIEW_DEADLINE_SECS;
        assert!(!dl.is_review_expired(deadline));
        assert!(dl.is_review_expired(deadline + 3601));
    }

    #[test]
    fn is_revision_expired_boundary() {
        let dl = BountyDeadlines::new(base_time());
        let deadline = base_time()
            + DEFAULT_WORK_DEADLINE_SECS
            + DEFAULT_REVIEW_DEADLINE_SECS
            + DEFAULT_REVISION_DEADLINE_SECS;
        assert!(!dl.is_revision_expired(deadline));
        assert!(dl.is_revision_expired(deadline + 3601));
    }

    #[test]
    fn is_dispute_expired_boundary() {
        let dl = BountyDeadlines::new(base_time());
        let deadline = base_time()
            + DEFAULT_WORK_DEADLINE_SECS
            + DEFAULT_REVIEW_DEADLINE_SECS
            + DEFAULT_DISPUTE_DEADLINE_SECS;
        assert!(!dl.is_dispute_expired(deadline));
        assert!(dl.is_dispute_expired(deadline + 3601));
    }

    #[test]
    fn next_deadline_open_returns_claim() {
        let dl = BountyDeadlines::new(base_time());
        let result = dl.next_deadline(BountyState::Open, base_time());
        assert_eq!(result, Some(("claim".to_string(), dl.claim_deadline)));
    }

    #[test]
    fn next_deadline_claimed_returns_work() {
        let dl = BountyDeadlines::new(base_time());
        let result = dl.next_deadline(BountyState::Claimed, base_time());
        assert_eq!(result, Some(("work".to_string(), dl.work_deadline)));
    }

    #[test]
    fn next_deadline_submitted_returns_review() {
        let dl = BountyDeadlines::new(base_time());
        let result = dl.next_deadline(BountyState::Submitted, base_time());
        assert_eq!(result, Some(("review".to_string(), dl.review_deadline)));
    }

    #[test]
    fn next_deadline_under_review_returns_review() {
        let dl = BountyDeadlines::new(base_time());
        let result = dl.next_deadline(BountyState::UnderReview, base_time());
        assert_eq!(result, Some(("review".to_string(), dl.review_deadline)));
    }

    #[test]
    fn next_deadline_revision_returns_revision() {
        let dl = BountyDeadlines::new(base_time());
        let result = dl.next_deadline(BountyState::Revision, base_time());
        assert_eq!(result, Some(("revision".to_string(), dl.revision_deadline)));
    }

    #[test]
    fn next_deadline_disputed_returns_dispute() {
        let dl = BountyDeadlines::new(base_time());
        let result = dl.next_deadline(BountyState::Disputed, base_time());
        assert_eq!(result, Some(("dispute".to_string(), dl.dispute_deadline)));
    }

    #[test]
    fn next_deadline_terminal_states_return_none() {
        let dl = BountyDeadlines::new(base_time());
        for state in [
            BountyState::Accepted,
            BountyState::Rejected,
            BountyState::Paid,
            BountyState::Expired,
            BountyState::Cancelled,
        ] {
            assert_eq!(dl.next_deadline(state, base_time()), None);
        }
    }

    #[test]
    fn next_deadline_open_after_claim_deadline_returns_none() {
        let dl = BountyDeadlines::new(base_time());
        let past = dl.claim_deadline + 1;
        assert_eq!(dl.next_deadline(BountyState::Open, past), None);
    }

    // ── BountyRecord tests ─────────────────────────────────────────────

    #[test]
    fn record_new_is_open() {
        let rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        assert_eq!(rec.state, BountyState::Open);
        assert!(rec.claimant.is_none());
        assert!(rec.artifact_cid.is_none());
        assert!(rec.reviewers.is_empty());
        assert_eq!(rec.created_at, base_time());
    }

    #[test]
    fn record_claim_success() {
        let mut rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        rec.claim(test_did("worker"), base_time() + 100).unwrap();
        assert_eq!(rec.state, BountyState::Claimed);
        assert_eq!(rec.claimant, Some(test_did("worker")));
    }

    #[test]
    fn record_claim_wrong_state_fails() {
        let mut rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        rec.claim(test_did("worker"), base_time() + 100).unwrap();
        let err = rec.claim(test_did("other"), base_time() + 200).unwrap_err();
        assert!(matches!(err, BountyError::InvalidState(_)));
    }

    #[test]
    fn record_claim_expired_fails() {
        let mut rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        let past = base_time() + DEFAULT_CLAIM_DEADLINE_SECS + 3601;
        let err = rec.claim(test_did("worker"), past).unwrap_err();
        assert!(matches!(err, BountyError::DeadlineExceeded { .. }));
    }

    #[test]
    fn record_submit_from_claimed() {
        let mut rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        rec.claim(test_did("worker"), base_time() + 100).unwrap();
        rec.submit("ipfs://QmX".to_string(), base_time() + 200).unwrap();
        assert_eq!(rec.state, BountyState::Submitted);
        assert_eq!(rec.artifact_cid, Some("ipfs://QmX".to_string()));
    }

    #[test]
    fn record_submit_from_revision() {
        let mut rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        rec.claim(test_did("worker"), base_time() + 100).unwrap();
        rec.submit("ipfs://QmV1".to_string(), base_time() + 200).unwrap();
        // Manually set to Revision for this test
        rec.state = BountyState::Revision;
        rec.submit("ipfs://QmV2".to_string(), base_time() + 300).unwrap();
        assert_eq!(rec.state, BountyState::Submitted);
        assert_eq!(rec.artifact_cid, Some("ipfs://QmV2".to_string()));
    }

    #[test]
    fn record_submit_wrong_state_fails() {
        let mut rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        let err = rec.submit("ipfs://QmX".to_string(), base_time() + 100).unwrap_err();
        assert!(matches!(err, BountyError::InvalidState(_)));
    }

    #[test]
    fn record_submit_claimed_work_expired() {
        let mut rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        rec.claim(test_did("worker"), base_time() + 100).unwrap();
        let past = base_time() + DEFAULT_WORK_DEADLINE_SECS + 3601;
        let err = rec.submit("ipfs://QmX".to_string(), past).unwrap_err();
        assert!(matches!(err, BountyError::DeadlineExceeded { .. }));
    }

    #[test]
    fn record_check_expiry_open() {
        let mut rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        let past = base_time() + DEFAULT_CLAIM_DEADLINE_SECS + 3601;
        assert!(rec.check_expiry(past));
        assert_eq!(rec.state, BountyState::Expired);
    }

    #[test]
    fn record_check_expiry_not_yet() {
        let mut rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        assert!(!rec.check_expiry(base_time() + 100));
        assert_eq!(rec.state, BountyState::Open);
    }

    #[test]
    fn record_check_expiry_claimed() {
        let mut rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        rec.claim(test_did("worker"), base_time() + 100).unwrap();
        let past = base_time() + DEFAULT_WORK_DEADLINE_SECS + 3601;
        assert!(rec.check_expiry(past));
        assert_eq!(rec.state, BountyState::Expired);
    }

    #[test]
    fn record_check_expiry_revision() {
        let mut rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        rec.state = BountyState::Revision;
        let deadline = base_time()
            + DEFAULT_WORK_DEADLINE_SECS
            + DEFAULT_REVIEW_DEADLINE_SECS
            + DEFAULT_REVISION_DEADLINE_SECS;
        assert!(!rec.check_expiry(deadline));
        assert!(rec.check_expiry(deadline + 3601));
        assert_eq!(rec.state, BountyState::Expired);
    }

    #[test]
    fn record_check_expiry_terminal_noop() {
        let mut rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        rec.state = BountyState::Accepted;
        assert!(!rec.check_expiry(base_time() + 999_999));
        assert_eq!(rec.state, BountyState::Accepted);
    }

    #[test]
    fn record_with_deadlines_builder() {
        let custom = BountyDeadlines::new(base_time()).with_claim_deadline(600);
        let rec = BountyRecord::new(
            test_bounty_id("1"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        )
        .with_deadlines(custom.clone());
        assert_eq!(rec.deadlines, custom);
    }

    #[test]
    fn record_serde_roundtrip() {
        let rec = BountyRecord::new(
            test_bounty_id("serde"),
            "Title".to_string(),
            "Desc".to_string(),
            test_did("creator"),
            TokenAmount(500),
            TokenType::Train,
            base_time(),
        );
        let json = serde_json::to_string(&rec).unwrap();
        let back: BountyRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rec, back);
    }

    #[test]
    fn deadlines_serde_roundtrip() {
        let dl = BountyDeadlines::new(base_time()).with_claim_deadline(600);
        let json = serde_json::to_string(&dl).unwrap();
        let back: BountyDeadlines = serde_json::from_str(&json).unwrap();
        assert_eq!(dl, back);
    }

    // ── BountyManager tests ─────────────────────────────────────────

    #[test]
    fn manager_create_bounty() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "Test bounty".to_string(),
            "Description".to_string(),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        assert_eq!(data.state, BountyState::Open);
        assert_eq!(data.reward_amount, TokenAmount(1000));
        assert!(data.claimant.is_none());
    }

    #[test]
    fn manager_create_multiple_ids_increment() {
        let mut mgr = BountyManager::new();
        let d1 = mgr.create_bounty(
            test_did("a"),
            "b1".to_string(),
            String::new(),
            TokenAmount(100),
            TokenType::Compute,
            base_time(),
        );
        let d2 = mgr.create_bounty(
            test_did("a"),
            "b2".to_string(),
            String::new(),
            TokenAmount(200),
            TokenType::Train,
            base_time(),
        );
        assert_ne!(d1.id, d2.id);
    }

    #[test]
    fn manager_claim_success() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "Test".to_string(),
            String::new(),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        mgr.claim_bounty(&data.id, test_did("worker"), TokenAmount(200), base_time() + 100)
            .unwrap();
        assert_eq!(mgr.get_state(&data.id), Some(BountyState::Claimed));
    }

    #[test]
    fn manager_claim_nonexistent() {
        let mut mgr = BountyManager::new();
        let err = mgr
            .claim_bounty(
                &BountyId("missing".to_string()),
                test_did("worker"),
                TokenAmount(200),
                base_time(),
            )
            .unwrap_err();
        assert!(matches!(err, BountyError::NotFound(_)));
    }

    #[test]
    fn manager_submit_work_success() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "Test".to_string(),
            String::new(),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        mgr.claim_bounty(&data.id, test_did("worker"), TokenAmount(200), base_time() + 100)
            .unwrap();
        mgr.submit_work(&data.id, Hash256("artifact".to_string()), base_time() + 200).unwrap();
        assert_eq!(mgr.get_state(&data.id), Some(BountyState::Submitted));
    }

    #[test]
    fn manager_submit_without_claiming() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "Test".to_string(),
            String::new(),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        let err = mgr
            .submit_work(&data.id, Hash256("artifact".to_string()), base_time() + 100)
            .unwrap_err();
        assert!(matches!(err, BountyError::InvalidTransition { .. }));
    }

    #[test]
    fn manager_full_happy_path() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "Test".to_string(),
            "Full lifecycle".to_string(),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );

        mgr.claim_bounty(&data.id, test_did("worker"), TokenAmount(200), base_time() + 100)
            .unwrap();

        mgr.submit_work(&data.id, Hash256("artifact".to_string()), base_time() + 200).unwrap();

        let reviewers = vec![test_did("r1"), test_did("r2"), test_did("r3")];
        mgr.start_review(&data.id, reviewers, base_time() + 300).unwrap();

        mgr.submit_review(&data.id, make_review("r1", 85), base_time() + 400).unwrap();
        mgr.submit_review(&data.id, make_review("r2", 90), base_time() + 401).unwrap();
        mgr.submit_review(&data.id, make_review("r3", 80), base_time() + 402).unwrap();

        let outcome = mgr.complete_review(&data.id, base_time() + 500).unwrap();
        assert_eq!(outcome, ReviewOutcome::Approved);

        let fees = mgr.pay_bounty(&data.id, base_time() + 600).unwrap();
        assert_eq!(mgr.get_state(&data.id), Some(BountyState::Paid));
        assert_eq!(fees.net_amount, TokenAmount(930));
    }

    #[test]
    fn manager_full_rejection_path() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "Test".to_string(),
            String::new(),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );

        mgr.claim_bounty(&data.id, test_did("worker"), TokenAmount(200), base_time() + 100)
            .unwrap();
        mgr.submit_work(&data.id, Hash256("artifact".to_string()), base_time() + 200).unwrap();

        let reviewers = vec![test_did("r1"), test_did("r2"), test_did("r3")];
        mgr.start_review(&data.id, reviewers, base_time() + 300).unwrap();

        mgr.submit_review(&data.id, make_review("r1", 20), base_time() + 400).unwrap();
        mgr.submit_review(&data.id, make_review("r2", 30), base_time() + 401).unwrap();
        mgr.submit_review(&data.id, make_review("r3", 10), base_time() + 402).unwrap();

        let outcome = mgr.complete_review(&data.id, base_time() + 500).unwrap();
        assert_eq!(outcome, ReviewOutcome::Rejected);
    }

    #[test]
    fn manager_revision_path() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "Test".to_string(),
            String::new(),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );

        mgr.claim_bounty(&data.id, test_did("worker"), TokenAmount(200), base_time() + 100)
            .unwrap();
        mgr.submit_work(&data.id, Hash256("v1".to_string()), base_time() + 200).unwrap();

        let reviewers = vec![test_did("r1"), test_did("r2"), test_did("r3")];
        mgr.start_review(&data.id, reviewers, base_time() + 300).unwrap();

        mgr.submit_review(&data.id, make_review("r1", 50), base_time() + 400).unwrap();
        mgr.submit_review(&data.id, make_review("r2", 55), base_time() + 401).unwrap();
        mgr.submit_review(&data.id, make_review("r3", 45), base_time() + 402).unwrap();

        let outcome = mgr.complete_review(&data.id, base_time() + 500).unwrap();
        assert_eq!(outcome, ReviewOutcome::NeedsRevision);
        assert_eq!(mgr.get_state(&data.id), Some(BountyState::Revision));

        mgr.submit_work(&data.id, Hash256("v2".to_string()), base_time() + 600).unwrap();
        let reviewers2 = vec![test_did("r1"), test_did("r2"), test_did("r3")];
        mgr.start_review(&data.id, reviewers2, base_time() + 700).unwrap();
        mgr.submit_review(&data.id, make_review("r1", 90), base_time() + 800).unwrap();
        mgr.submit_review(&data.id, make_review("r2", 85), base_time() + 801).unwrap();
        mgr.submit_review(&data.id, make_review("r3", 80), base_time() + 802).unwrap();

        let outcome2 = mgr.complete_review(&data.id, base_time() + 900).unwrap();
        assert_eq!(outcome2, ReviewOutcome::Approved);
    }

    #[test]
    fn manager_cancel_bounty() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "Test".to_string(),
            String::new(),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        mgr.cancel_bounty(&data.id, base_time() + 100).unwrap();
        assert_eq!(mgr.get_state(&data.id), Some(BountyState::Cancelled));
    }

    #[test]
    fn manager_complete_review_incomplete() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "Test".to_string(),
            String::new(),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        mgr.claim_bounty(&data.id, test_did("worker"), TokenAmount(200), base_time() + 100)
            .unwrap();
        mgr.submit_work(&data.id, Hash256("artifact".to_string()), base_time() + 200).unwrap();

        let reviewers = vec![test_did("r1"), test_did("r2"), test_did("r3")];
        mgr.start_review(&data.id, reviewers, base_time() + 300).unwrap();
        mgr.submit_review(&data.id, make_review("r1", 80), base_time() + 400).unwrap();

        let err = mgr.complete_review(&data.id, base_time() + 500).unwrap_err();
        assert!(matches!(err, BountyError::ReviewIncomplete { .. }));
    }

    #[test]
    fn manager_get_bounty_returns_data() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "My bounty".to_string(),
            "Desc".to_string(),
            TokenAmount(500),
            TokenType::Train,
            base_time(),
        );
        let fetched = mgr.get_bounty(&data.id).unwrap();
        assert_eq!(fetched.title, "My bounty");
    }

    #[test]
    fn manager_get_bounty_missing() {
        let mgr = BountyManager::new();
        assert!(mgr.get_bounty(&BountyId("missing".to_string())).is_none());
    }

    #[test]
    fn manager_escrow_created_with_bounty() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "Test".to_string(),
            String::new(),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        let escrow = mgr.get_escrow_manager().get_escrow(&data.id).unwrap();
        assert_eq!(escrow.amount, TokenAmount(1000));
    }

    #[test]
    fn manager_resolve_bounty_accept() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "Test".to_string(),
            String::new(),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        mgr.claim_bounty(&data.id, test_did("worker"), TokenAmount(200), base_time() + 100)
            .unwrap();
        mgr.submit_work(&data.id, Hash256("artifact".to_string()), base_time() + 200).unwrap();
        let reviewers = vec![test_did("r1"), test_did("r2"), test_did("r3")];
        mgr.start_review(&data.id, reviewers, base_time() + 300).unwrap();

        let sm = mgr.state_machines.get_mut(&data.id).unwrap();
        sm.try_transition(
            BountyEvent::Dispute { reason: "quality concern".to_string() },
            base_time() + 400,
        )
        .unwrap();

        let fees = mgr.resolve_bounty(&data.id, true, base_time() + 500).unwrap();
        assert_eq!(fees.net_amount, TokenAmount(930));
    }

    #[test]
    fn manager_dispute_and_resolve_path() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "Test".to_string(),
            String::new(),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        mgr.claim_bounty(&data.id, test_did("worker"), TokenAmount(200), base_time() + 100)
            .unwrap();
        mgr.submit_work(&data.id, Hash256("artifact".to_string()), base_time() + 200).unwrap();
        let reviewers = vec![test_did("r1"), test_did("r2"), test_did("r3")];
        mgr.start_review(&data.id, reviewers, base_time() + 300).unwrap();

        let sm = mgr.state_machines.get_mut(&data.id).unwrap();
        sm.try_transition(
            BountyEvent::Dispute { reason: "quality concern".to_string() },
            base_time() + 400,
        )
        .unwrap();

        let fees = mgr.resolve_bounty(&data.id, true, base_time() + 500).unwrap();
        assert_eq!(fees.net_amount, TokenAmount(930));
    }

    #[test]
    fn manager_default() {
        let mgr = BountyManager::default();
        assert!(mgr.state_machines.is_empty());
    }

    #[test]
    fn manager_get_review_committee() {
        let mut mgr = BountyManager::new();
        let data = mgr.create_bounty(
            test_did("creator"),
            "Test".to_string(),
            String::new(),
            TokenAmount(1000),
            TokenType::Compute,
            base_time(),
        );
        assert!(mgr.get_review_committee(&data.id).is_none());

        mgr.claim_bounty(&data.id, test_did("worker"), TokenAmount(200), base_time() + 100)
            .unwrap();
        mgr.submit_work(&data.id, Hash256("artifact".to_string()), base_time() + 200).unwrap();
        let reviewers = vec![test_did("r1"), test_did("r2"), test_did("r3")];
        mgr.start_review(&data.id, reviewers, base_time() + 300).unwrap();
        assert!(mgr.get_review_committee(&data.id).is_some());
    }

    fn make_review(reviewer: &str, score: u8) -> Review {
        Review::new(test_did(reviewer), score, String::new(), base_time(), None).unwrap()
    }
}
