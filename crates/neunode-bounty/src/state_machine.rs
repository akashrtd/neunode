use neunode_core::constants::bounty::{
    DEFAULT_CLAIM_DEADLINE_SECS, DEFAULT_DISPUTE_DEADLINE_SECS, DEFAULT_REVIEW_DEADLINE_SECS,
    DEFAULT_REVISION_DEADLINE_SECS, DEFAULT_WORK_DEADLINE_SECS, GRACE_PERIOD_SECS,
    PROVIDER_BOND_PCT,
};
use neunode_core::types::{
    BountyId, BountyState, Did, Hash256, Signature, Timestamp, TokenAmount, TokenType,
};
use serde::{Deserialize, Serialize};

use crate::error::{BountyError, Result};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct Deadlines {
    pub claim: Timestamp,
    pub work: Timestamp,
    pub review: Timestamp,
    pub revision: Timestamp,
    pub dispute: Timestamp,
}

impl Deadlines {
    pub fn from_created_at(created_at: Timestamp) -> Self {
        Self {
            claim: created_at.saturating_add(DEFAULT_CLAIM_DEADLINE_SECS),
            work: created_at.saturating_add(DEFAULT_WORK_DEADLINE_SECS),
            review: created_at
                .saturating_add(DEFAULT_WORK_DEADLINE_SECS + DEFAULT_REVIEW_DEADLINE_SECS),
            revision: created_at.saturating_add(
                DEFAULT_WORK_DEADLINE_SECS
                    + DEFAULT_REVIEW_DEADLINE_SECS
                    + DEFAULT_REVISION_DEADLINE_SECS,
            ),
            dispute: created_at.saturating_add(
                DEFAULT_WORK_DEADLINE_SECS
                    + DEFAULT_REVIEW_DEADLINE_SECS
                    + DEFAULT_DISPUTE_DEADLINE_SECS,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct BountyData {
    pub id: BountyId,
    pub creator: Did,
    pub title: String,
    pub description: String,
    pub reward_amount: TokenAmount,
    pub reward_token: TokenType,
    pub state: BountyState,
    pub claimant: Option<Did>,
    pub created_at: Timestamp,
    pub deadlines: Deadlines,
    pub artifact_hash: Option<Hash256>,
    pub bond: Option<TokenAmount>,
}

impl BountyData {
    pub fn required_bond(&self) -> TokenAmount {
        let bond_units = (self.reward_amount.0 as f64 * PROVIDER_BOND_PCT / 100.0).ceil() as u64;
        TokenAmount(bond_units)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub enum BountyEvent {
    Claim { claimant: Did, bond: TokenAmount },
    Submit { artifact_hash: Hash256 },
    StartReview,
    SubmitReview { reviewer: Did, score: u8, notes: String, signature: Option<Signature> },
    RequestRevision,
    Accept,
    Reject,
    Dispute { reason: String },
    Resolve { accept: bool },
    Pay,
    Cancel,
    Expire,
}

#[derive(Debug, Clone)]
pub struct BountyStateMachine {
    data: BountyData,
}

impl BountyStateMachine {
    pub fn new(data: BountyData) -> Self {
        Self { data }
    }

    pub fn try_transition(&mut self, event: BountyEvent, now: Timestamp) -> Result<()> {
        let is_pay_from_accepted =
            self.data.state == BountyState::Accepted && matches!(event, BountyEvent::Pay);
        if self.data.state.is_terminal() && !is_pay_from_accepted {
            return Err(BountyError::TerminalState(self.data.state));
        }

        let next_state = self.compute_next_state(&event, now)?;
        self.apply_side_effects(&event)?;
        self.data.state = next_state;
        Ok(())
    }

    pub fn current_state(&self) -> BountyState {
        self.data.state
    }

    pub fn is_terminal(&self) -> bool {
        self.data.state.is_terminal()
    }

    pub fn allowed_transitions(&self) -> Vec<BountyState> {
        match self.data.state {
            BountyState::Open => {
                vec![BountyState::Claimed, BountyState::Expired, BountyState::Cancelled]
            }
            BountyState::Claimed => {
                vec![BountyState::Submitted, BountyState::Expired, BountyState::Cancelled]
            }
            BountyState::Submitted => vec![BountyState::UnderReview, BountyState::Revision],
            BountyState::UnderReview => vec![
                BountyState::Accepted,
                BountyState::Rejected,
                BountyState::Revision,
                BountyState::Disputed,
            ],
            BountyState::Revision => vec![BountyState::Submitted, BountyState::Expired],
            BountyState::Disputed => {
                vec![BountyState::Accepted, BountyState::Rejected, BountyState::Paid]
            }
            BountyState::Accepted => vec![BountyState::Paid],
            BountyState::Rejected
            | BountyState::Paid
            | BountyState::Expired
            | BountyState::Cancelled => vec![],
        }
    }

    pub fn data(&self) -> &BountyData {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut BountyData {
        &mut self.data
    }

    fn compute_next_state(&self, event: &BountyEvent, now: Timestamp) -> Result<BountyState> {
        match (&self.data.state, event) {
            (BountyState::Open, BountyEvent::Claim { claimant, bond }) => {
                if now > self.data.deadlines.claim.saturating_add(GRACE_PERIOD_SECS) {
                    return Err(BountyError::DeadlineExceeded {
                        deadline_type: "claim".to_string(),
                        deadline: self.data.deadlines.claim,
                        now,
                    });
                }
                let required = self.data.required_bond();
                if *bond < required {
                    return Err(BountyError::InsufficientBond { required, provided: *bond });
                }
                if self.data.claimant.is_some() {
                    return Err(BountyError::AlreadyClaimed(claimant.to_string()));
                }
                Ok(BountyState::Claimed)
            }

            (BountyState::Open, BountyEvent::Expire) => {
                if now <= self.data.deadlines.claim {
                    return Err(BountyError::InvalidTransition {
                        from: self.data.state,
                        event: "Expire".to_string(),
                        to: BountyState::Expired,
                    });
                }
                Ok(BountyState::Expired)
            }

            (BountyState::Open, BountyEvent::Cancel) => Ok(BountyState::Cancelled),

            (BountyState::Claimed, BountyEvent::Submit { .. }) => {
                if now > self.data.deadlines.work.saturating_add(GRACE_PERIOD_SECS) {
                    return Err(BountyError::DeadlineExceeded {
                        deadline_type: "work".to_string(),
                        deadline: self.data.deadlines.work,
                        now,
                    });
                }
                Ok(BountyState::Submitted)
            }

            (BountyState::Claimed, BountyEvent::Expire) => {
                if now <= self.data.deadlines.work {
                    return Err(BountyError::InvalidTransition {
                        from: self.data.state,
                        event: "Expire".to_string(),
                        to: BountyState::Expired,
                    });
                }
                Ok(BountyState::Expired)
            }

            (BountyState::Claimed, BountyEvent::Cancel) => Ok(BountyState::Cancelled),

            (BountyState::Submitted, BountyEvent::StartReview) => Ok(BountyState::UnderReview),

            (BountyState::Submitted, BountyEvent::RequestRevision) => Ok(BountyState::Revision),

            (BountyState::UnderReview, BountyEvent::Accept) => Ok(BountyState::Accepted),

            (BountyState::UnderReview, BountyEvent::Reject) => Ok(BountyState::Rejected),

            (BountyState::UnderReview, BountyEvent::RequestRevision) => Ok(BountyState::Revision),

            (BountyState::UnderReview, BountyEvent::Dispute { .. }) => Ok(BountyState::Disputed),

            (BountyState::UnderReview, BountyEvent::SubmitReview { .. }) => {
                Ok(BountyState::UnderReview)
            }

            (BountyState::Revision, BountyEvent::Submit { .. }) => {
                if now > self.data.deadlines.revision.saturating_add(GRACE_PERIOD_SECS) {
                    return Err(BountyError::DeadlineExceeded {
                        deadline_type: "revision".to_string(),
                        deadline: self.data.deadlines.revision,
                        now,
                    });
                }
                Ok(BountyState::Submitted)
            }

            (BountyState::Revision, BountyEvent::Expire) => {
                if now <= self.data.deadlines.revision {
                    return Err(BountyError::InvalidTransition {
                        from: self.data.state,
                        event: "Expire".to_string(),
                        to: BountyState::Expired,
                    });
                }
                Ok(BountyState::Expired)
            }

            (BountyState::Disputed, BountyEvent::Resolve { accept: true }) => {
                Ok(BountyState::Accepted)
            }

            (BountyState::Disputed, BountyEvent::Resolve { accept: false }) => {
                Ok(BountyState::Rejected)
            }

            (BountyState::Disputed, BountyEvent::Pay) => Ok(BountyState::Paid),

            (BountyState::Accepted, BountyEvent::Pay) => Ok(BountyState::Paid),

            _ => Err(BountyError::InvalidTransition {
                from: self.data.state,
                event: format!("{event:?}")
                    .split_whitespace()
                    .next()
                    .unwrap_or("Unknown")
                    .to_string(),
                to: self.data.state,
            }),
        }
    }

    fn apply_side_effects(&mut self, event: &BountyEvent) -> Result<()> {
        match event {
            BountyEvent::Claim { claimant, bond } => {
                self.data.claimant = Some(claimant.clone());
                self.data.bond = Some(*bond);
            }
            BountyEvent::Submit { artifact_hash } => {
                self.data.artifact_hash = Some(artifact_hash.clone());
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_did(name: &str) -> Did {
        Did(format!("did:neunode:{name}"))
    }

    fn test_bounty(created_at: Timestamp) -> BountyStateMachine {
        let data = BountyData {
            id: BountyId("bnty_test".to_string()),
            creator: test_did("creator"),
            title: "Test bounty".to_string(),
            description: "A test bounty".to_string(),
            reward_amount: TokenAmount(1000),
            reward_token: TokenType::Compute,
            state: BountyState::Open,
            claimant: None,
            created_at,
            deadlines: Deadlines::from_created_at(created_at),
            artifact_hash: None,
            bond: None,
        };
        BountyStateMachine::new(data)
    }

    fn test_hash() -> Hash256 {
        Hash256("abc123def456".to_string())
    }

    #[test]
    fn deadlines_from_created_at() {
        let dl = Deadlines::from_created_at(1000);
        assert_eq!(dl.claim, 1000 + DEFAULT_CLAIM_DEADLINE_SECS);
        assert_eq!(dl.work, 1000 + DEFAULT_WORK_DEADLINE_SECS);
        assert!(dl.review > dl.work);
        assert!(dl.revision > dl.review);
        assert!(dl.dispute > dl.review);
    }

    #[test]
    fn required_bond_15_pct() {
        let data = BountyData {
            id: BountyId("bnty_test".to_string()),
            creator: test_did("creator"),
            title: "Test".to_string(),
            description: String::new(),
            reward_amount: TokenAmount(1000),
            reward_token: TokenType::Compute,
            state: BountyState::Open,
            claimant: None,
            created_at: 0,
            deadlines: Deadlines::from_created_at(0),
            artifact_hash: None,
            bond: None,
        };
        let bond = data.required_bond();
        assert_eq!(bond, TokenAmount(150));
    }

    #[test]
    fn valid_open_to_claimed() {
        let mut sm = test_bounty(1000);
        let now = 1000 + 100;
        let bond = TokenAmount(200);
        sm.try_transition(BountyEvent::Claim { claimant: test_did("worker"), bond }, now).unwrap();
        assert_eq!(sm.current_state(), BountyState::Claimed);
        assert_eq!(sm.data().claimant, Some(test_did("worker")));
        assert_eq!(sm.data().bond, Some(TokenAmount(200)));
    }

    #[test]
    fn claim_insufficient_bond() {
        let mut sm = test_bounty(1000);
        let now = 1000 + 100;
        let bond = TokenAmount(10);
        let err = sm
            .try_transition(BountyEvent::Claim { claimant: test_did("worker"), bond }, now)
            .unwrap_err();
        assert!(matches!(err, BountyError::InsufficientBond { .. }));
    }

    #[test]
    fn claim_after_deadline() {
        let mut sm = test_bounty(1000);
        let past_deadline = 1000 + DEFAULT_CLAIM_DEADLINE_SECS + GRACE_PERIOD_SECS + 1;
        let bond = TokenAmount(200);
        let err = sm
            .try_transition(
                BountyEvent::Claim { claimant: test_did("worker"), bond },
                past_deadline,
            )
            .unwrap_err();
        assert!(matches!(err, BountyError::DeadlineExceeded { .. }));
    }

    #[test]
    fn valid_claimed_to_submitted() {
        let mut sm = test_bounty(1000);
        let now = 1000 + 100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        assert_eq!(sm.current_state(), BountyState::Submitted);
        assert_eq!(sm.data().artifact_hash, Some(test_hash()));
    }

    #[test]
    fn submit_after_work_deadline() {
        let mut sm = test_bounty(1000);
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            1000 + 100,
        )
        .unwrap();
        let past_deadline = 1000 + DEFAULT_WORK_DEADLINE_SECS + GRACE_PERIOD_SECS + 1;
        let err = sm
            .try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, past_deadline)
            .unwrap_err();
        assert!(matches!(err, BountyError::DeadlineExceeded { .. }));
    }

    #[test]
    fn valid_submitted_to_under_review() {
        let mut sm = test_bounty(1000);
        let now = 1100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        assert_eq!(sm.current_state(), BountyState::UnderReview);
    }

    #[test]
    fn valid_under_review_to_accepted() {
        let mut sm = test_bounty(1000);
        let now = 1100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        sm.try_transition(BountyEvent::Accept, now + 300).unwrap();
        assert_eq!(sm.current_state(), BountyState::Accepted);
    }

    #[test]
    fn valid_under_review_to_rejected() {
        let mut sm = test_bounty(1000);
        let now = 1100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        sm.try_transition(BountyEvent::Reject, now + 300).unwrap();
        assert_eq!(sm.current_state(), BountyState::Rejected);
    }

    #[test]
    fn valid_under_review_to_revision() {
        let mut sm = test_bounty(1000);
        let now = 1100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        sm.try_transition(BountyEvent::RequestRevision, now + 300).unwrap();
        assert_eq!(sm.current_state(), BountyState::Revision);
    }

    #[test]
    fn valid_revision_to_submitted() {
        let mut sm = test_bounty(1000);
        let now = 1100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        sm.try_transition(BountyEvent::RequestRevision, now + 300).unwrap();
        let new_hash = Hash256("revision_hash".to_string());
        sm.try_transition(BountyEvent::Submit { artifact_hash: new_hash.clone() }, now + 400)
            .unwrap();
        assert_eq!(sm.current_state(), BountyState::Submitted);
        assert_eq!(sm.data().artifact_hash, Some(new_hash));
    }

    #[test]
    fn revision_after_deadline() {
        let mut sm = test_bounty(1000);
        let now = 1100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        sm.try_transition(BountyEvent::RequestRevision, now + 300).unwrap();
        let past_deadline = 1000
            + DEFAULT_WORK_DEADLINE_SECS
            + DEFAULT_REVIEW_DEADLINE_SECS
            + DEFAULT_REVISION_DEADLINE_SECS
            + GRACE_PERIOD_SECS
            + 1;
        let err = sm
            .try_transition(
                BountyEvent::Submit { artifact_hash: Hash256("new".to_string()) },
                past_deadline,
            )
            .unwrap_err();
        assert!(matches!(err, BountyError::DeadlineExceeded { .. }));
    }

    #[test]
    fn valid_under_review_to_disputed() {
        let mut sm = test_bounty(1000);
        let now = 1100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        sm.try_transition(
            BountyEvent::Dispute { reason: "quality concerns".to_string() },
            now + 300,
        )
        .unwrap();
        assert_eq!(sm.current_state(), BountyState::Disputed);
    }

    #[test]
    fn valid_disputed_to_accepted_via_resolve() {
        let mut sm = test_bounty(1000);
        let now = 1100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        sm.try_transition(BountyEvent::Dispute { reason: "dispute".to_string() }, now + 300)
            .unwrap();
        sm.try_transition(BountyEvent::Resolve { accept: true }, now + 400).unwrap();
        assert_eq!(sm.current_state(), BountyState::Accepted);
    }

    #[test]
    fn valid_disputed_to_rejected_via_resolve() {
        let mut sm = test_bounty(1000);
        let now = 1100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        sm.try_transition(BountyEvent::Dispute { reason: "dispute".to_string() }, now + 300)
            .unwrap();
        sm.try_transition(BountyEvent::Resolve { accept: false }, now + 400).unwrap();
        assert_eq!(sm.current_state(), BountyState::Rejected);
    }

    #[test]
    fn valid_disputed_to_paid() {
        let mut sm = test_bounty(1000);
        let now = 1100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        sm.try_transition(BountyEvent::Dispute { reason: "dispute".to_string() }, now + 300)
            .unwrap();
        sm.try_transition(BountyEvent::Pay, now + 400).unwrap();
        assert_eq!(sm.current_state(), BountyState::Paid);
    }

    #[test]
    fn valid_accepted_to_paid() {
        let mut sm = test_bounty(1000);
        let now = 1100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        sm.try_transition(BountyEvent::Accept, now + 300).unwrap();
        sm.try_transition(BountyEvent::Pay, now + 400).unwrap();
        assert_eq!(sm.current_state(), BountyState::Paid);
    }

    #[test]
    fn valid_open_to_cancelled() {
        let mut sm = test_bounty(1000);
        sm.try_transition(BountyEvent::Cancel, 1100).unwrap();
        assert_eq!(sm.current_state(), BountyState::Cancelled);
    }

    #[test]
    fn valid_open_to_expired() {
        let mut sm = test_bounty(1000);
        let past_claim = 1000 + DEFAULT_CLAIM_DEADLINE_SECS + 1;
        sm.try_transition(BountyEvent::Expire, past_claim).unwrap();
        assert_eq!(sm.current_state(), BountyState::Expired);
    }

    #[test]
    fn expire_before_claim_deadline_fails() {
        let mut sm = test_bounty(1000);
        let before_deadline = 1000 + DEFAULT_CLAIM_DEADLINE_SECS - 1;
        let err = sm.try_transition(BountyEvent::Expire, before_deadline).unwrap_err();
        assert!(matches!(err, BountyError::InvalidTransition { .. }));
    }

    #[test]
    fn terminal_state_rejects_all() {
        let mut sm = test_bounty(1000);
        sm.try_transition(BountyEvent::Cancel, 1100).unwrap();
        assert!(sm.is_terminal());

        let err = sm
            .try_transition(
                BountyEvent::Claim { claimant: test_did("x"), bond: TokenAmount(200) },
                1200,
            )
            .unwrap_err();
        assert!(matches!(err, BountyError::TerminalState(_)));
    }

    #[test]
    fn invalid_transition_claim_on_claimed() {
        let mut sm = test_bounty(1000);
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            1100,
        )
        .unwrap();
        let err = sm
            .try_transition(
                BountyEvent::Claim { claimant: test_did("other"), bond: TokenAmount(200) },
                1200,
            )
            .unwrap_err();
        assert!(matches!(
            err,
            BountyError::TerminalState(_) | BountyError::InvalidTransition { .. }
        ));
    }

    #[test]
    fn submit_without_claim_fails() {
        let mut sm = test_bounty(1000);
        let err = sm
            .try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, 1100)
            .unwrap_err();
        assert!(matches!(err, BountyError::InvalidTransition { .. }));
    }

    #[test]
    fn accept_without_review_fails() {
        let mut sm = test_bounty(1000);
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            1100,
        )
        .unwrap();
        let err = sm.try_transition(BountyEvent::Accept, 1200).unwrap_err();
        assert!(matches!(err, BountyError::InvalidTransition { .. }));
    }

    #[test]
    fn allowed_transitions_open() {
        let sm = test_bounty(1000);
        let transitions = sm.allowed_transitions();
        assert_eq!(
            transitions,
            vec![BountyState::Claimed, BountyState::Expired, BountyState::Cancelled]
        );
    }

    #[test]
    fn allowed_transitions_terminal_empty() {
        let mut sm = test_bounty(1000);
        sm.try_transition(BountyEvent::Cancel, 1100).unwrap();
        assert!(sm.allowed_transitions().is_empty());
    }

    #[test]
    fn allowed_transitions_under_review() {
        let mut sm = test_bounty(1000);
        let now = 1100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        let transitions = sm.allowed_transitions();
        assert!(transitions.contains(&BountyState::Accepted));
        assert!(transitions.contains(&BountyState::Rejected));
        assert!(transitions.contains(&BountyState::Revision));
        assert!(transitions.contains(&BountyState::Disputed));
    }

    #[test]
    fn full_happy_path() {
        let mut sm = test_bounty(1000);
        let now = 1100;

        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        assert_eq!(sm.current_state(), BountyState::Claimed);

        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        assert_eq!(sm.current_state(), BountyState::Submitted);

        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        assert_eq!(sm.current_state(), BountyState::UnderReview);

        sm.try_transition(BountyEvent::Accept, now + 300).unwrap();
        assert_eq!(sm.current_state(), BountyState::Accepted);

        sm.try_transition(BountyEvent::Pay, now + 400).unwrap();
        assert_eq!(sm.current_state(), BountyState::Paid);
        assert!(sm.is_terminal());
    }

    #[test]
    fn happy_path_with_revision() {
        let mut sm = test_bounty(1000);
        let now = 1100;

        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        sm.try_transition(BountyEvent::RequestRevision, now + 300).unwrap();
        assert_eq!(sm.current_state(), BountyState::Revision);

        let v2_hash = Hash256("revision_v2".to_string());
        sm.try_transition(BountyEvent::Submit { artifact_hash: v2_hash }, now + 400).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 500).unwrap();
        sm.try_transition(BountyEvent::Accept, now + 600).unwrap();
        sm.try_transition(BountyEvent::Pay, now + 700).unwrap();
        assert_eq!(sm.current_state(), BountyState::Paid);
    }

    #[test]
    fn submit_review_is_valid_in_under_review() {
        let mut sm = test_bounty(1000);
        let now = 1100;
        sm.try_transition(
            BountyEvent::Claim { claimant: test_did("worker"), bond: TokenAmount(200) },
            now,
        )
        .unwrap();
        sm.try_transition(BountyEvent::Submit { artifact_hash: test_hash() }, now + 100).unwrap();
        sm.try_transition(BountyEvent::StartReview, now + 200).unwrap();
        sm.try_transition(
            BountyEvent::SubmitReview {
                reviewer: test_did("reviewer"),
                score: 85,
                notes: "good".to_string(),
                signature: None,
            },
            now + 300,
        )
        .unwrap();
        assert_eq!(sm.current_state(), BountyState::UnderReview);
    }

    #[test]
    fn data_accessors() {
        let sm = test_bounty(1000);
        assert_eq!(sm.data().id, BountyId("bnty_test".to_string()));
        assert_eq!(sm.data().creator, test_did("creator"));
    }

    #[test]
    fn deadlines_saturating_add() {
        let dl = Deadlines::from_created_at(u64::MAX);
        assert_eq!(dl.claim, u64::MAX);
    }
}
