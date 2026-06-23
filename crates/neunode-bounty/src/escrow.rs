use std::collections::HashMap;

use neunode_core::types::{BountyId, Did, Timestamp, TokenAmount, TokenType};
use serde::{Deserialize, Serialize};

use crate::error::{BountyError, Result};
use ts_rs::TS;

const PROTOCOL_FEE_BPS: u64 = 300;
const REVIEWER_FEE_BPS: u64 = 400;
const BPS_DENOMINATOR: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum EscrowState {
    Funded,
    Released,
    Refunded,
    Disputed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct Escrow {
    pub amount: TokenAmount,
    pub token_type: TokenType,
    pub depositor: Did,
    pub beneficiary: Option<Did>,
    pub state: EscrowState,
    pub created_at: Timestamp,
}

impl Escrow {
    pub fn new(
        depositor: Did,
        amount: TokenAmount,
        token_type: TokenType,
        created_at: Timestamp,
    ) -> Self {
        Self {
            amount,
            token_type,
            depositor,
            beneficiary: None,
            state: EscrowState::Funded,
            created_at,
        }
    }

    pub fn release(&mut self, to: Did) -> Result<()> {
        if self.state != EscrowState::Funded && self.state != EscrowState::Disputed {
            return Err(BountyError::EscrowError(format!(
                "cannot release from state {:?}",
                self.state
            )));
        }
        self.beneficiary = Some(to);
        self.state = EscrowState::Released;
        Ok(())
    }

    pub fn refund(&mut self) -> Result<()> {
        if self.state != EscrowState::Funded {
            return Err(BountyError::EscrowError(format!(
                "cannot refund from state {:?}",
                self.state
            )));
        }
        self.state = EscrowState::Refunded;
        Ok(())
    }

    pub fn dispute(&mut self) -> Result<()> {
        if self.state != EscrowState::Funded {
            return Err(BountyError::EscrowError(format!(
                "cannot dispute from state {:?}",
                self.state
            )));
        }
        self.state = EscrowState::Disputed;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct FeeBreakdown {
    pub gross_amount: TokenAmount,
    pub protocol_fee: TokenAmount,
    pub reviewer_fee: TokenAmount,
    pub net_amount: TokenAmount,
}

impl FeeBreakdown {
    pub fn from_gross(gross: TokenAmount) -> Self {
        let protocol_units = (gross.0 * PROTOCOL_FEE_BPS as u128).div_ceil(BPS_DENOMINATOR as u128);
        let reviewer_units = (gross.0 * REVIEWER_FEE_BPS as u128).div_ceil(BPS_DENOMINATOR as u128);
        let total_fees = protocol_units.saturating_add(reviewer_units);
        let capped_fees = total_fees.min(gross.0);
        let net = gross.0.saturating_sub(capped_fees);

        let (protocol_fee, reviewer_fee) = if total_fees > gross.0 && total_fees > 0 {
            let pf = gross.0 * protocol_units / total_fees;
            let rf = gross.0.saturating_sub(pf);
            (TokenAmount(pf), TokenAmount(rf))
        } else {
            (TokenAmount(protocol_units), TokenAmount(reviewer_units))
        };

        Self { gross_amount: gross, protocol_fee, reviewer_fee, net_amount: TokenAmount(net) }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EscrowManager {
    escrows: HashMap<BountyId, Escrow>,
}

impl EscrowManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_escrow(
        &mut self,
        bounty_id: BountyId,
        depositor: Did,
        amount: TokenAmount,
        token_type: TokenType,
        created_at: Timestamp,
    ) -> Result<()> {
        if self.escrows.contains_key(&bounty_id) {
            return Err(BountyError::AlreadyExists(bounty_id.to_string()));
        }
        let escrow = Escrow::new(depositor, amount, token_type, created_at);
        self.escrows.insert(bounty_id, escrow);
        Ok(())
    }

    pub fn release(&mut self, bounty_id: &BountyId, to: Did) -> Result<()> {
        let escrow = self
            .escrows
            .get_mut(bounty_id)
            .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?;
        escrow.release(to)
    }

    pub fn refund(&mut self, bounty_id: &BountyId) -> Result<()> {
        let escrow = self
            .escrows
            .get_mut(bounty_id)
            .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?;
        escrow.refund()
    }

    pub fn dispute(&mut self, bounty_id: &BountyId) -> Result<()> {
        let escrow = self
            .escrows
            .get_mut(bounty_id)
            .ok_or_else(|| BountyError::NotFound(bounty_id.to_string()))?;
        escrow.dispute()
    }

    pub fn get_escrow(&self, bounty_id: &BountyId) -> Option<&Escrow> {
        self.escrows.get(bounty_id)
    }

    pub fn calculate_fees(&self, bounty_id: &BountyId) -> Option<FeeBreakdown> {
        self.escrows.get(bounty_id).map(|e| FeeBreakdown::from_gross(e.amount))
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

    #[test]
    fn escrow_new() {
        let escrow = Escrow::new(test_did("alice"), TokenAmount(1000), TokenType::Compute, 100);
        assert_eq!(escrow.amount, TokenAmount(1000));
        assert_eq!(escrow.token_type, TokenType::Compute);
        assert_eq!(escrow.depositor, test_did("alice"));
        assert!(escrow.beneficiary.is_none());
        assert_eq!(escrow.state, EscrowState::Funded);
        assert_eq!(escrow.created_at, 100);
    }

    #[test]
    fn escrow_release() {
        let mut escrow = Escrow::new(test_did("alice"), TokenAmount(1000), TokenType::Compute, 100);
        escrow.release(test_did("bob")).unwrap();
        assert_eq!(escrow.state, EscrowState::Released);
        assert_eq!(escrow.beneficiary, Some(test_did("bob")));
    }

    #[test]
    fn escrow_refund() {
        let mut escrow = Escrow::new(test_did("alice"), TokenAmount(1000), TokenType::Compute, 100);
        escrow.refund().unwrap();
        assert_eq!(escrow.state, EscrowState::Refunded);
    }

    #[test]
    fn escrow_dispute() {
        let mut escrow = Escrow::new(test_did("alice"), TokenAmount(1000), TokenType::Compute, 100);
        escrow.dispute().unwrap();
        assert_eq!(escrow.state, EscrowState::Disputed);
    }

    #[test]
    fn escrow_release_from_disputed() {
        let mut escrow = Escrow::new(test_did("alice"), TokenAmount(1000), TokenType::Compute, 100);
        escrow.dispute().unwrap();
        escrow.release(test_did("bob")).unwrap();
        assert_eq!(escrow.state, EscrowState::Released);
    }

    #[test]
    fn escrow_release_from_released_fails() {
        let mut escrow = Escrow::new(test_did("alice"), TokenAmount(1000), TokenType::Compute, 100);
        escrow.release(test_did("bob")).unwrap();
        let err = escrow.release(test_did("charlie")).unwrap_err();
        assert!(matches!(err, BountyError::EscrowError(_)));
    }

    #[test]
    fn escrow_refund_from_released_fails() {
        let mut escrow = Escrow::new(test_did("alice"), TokenAmount(1000), TokenType::Compute, 100);
        escrow.release(test_did("bob")).unwrap();
        let err = escrow.refund().unwrap_err();
        assert!(matches!(err, BountyError::EscrowError(_)));
    }

    #[test]
    fn escrow_dispute_from_refunded_fails() {
        let mut escrow = Escrow::new(test_did("alice"), TokenAmount(1000), TokenType::Compute, 100);
        escrow.refund().unwrap();
        let err = escrow.dispute().unwrap_err();
        assert!(matches!(err, BountyError::EscrowError(_)));
    }

    #[test]
    fn escrow_refund_from_disputed_fails() {
        let mut escrow = Escrow::new(test_did("alice"), TokenAmount(1000), TokenType::Compute, 100);
        escrow.dispute().unwrap();
        let err = escrow.refund().unwrap_err();
        assert!(matches!(err, BountyError::EscrowError(_)));
    }

    #[test]
    fn fee_breakdown_calculation() {
        let fb = FeeBreakdown::from_gross(TokenAmount(1000));
        assert_eq!(fb.gross_amount, TokenAmount(1000));
        assert_eq!(fb.protocol_fee, TokenAmount(30));
        assert_eq!(fb.reviewer_fee, TokenAmount(40));
        assert_eq!(fb.net_amount, TokenAmount(930));
    }

    #[test]
    fn fee_breakdown_zero() {
        let fb = FeeBreakdown::from_gross(TokenAmount(0));
        assert_eq!(fb.protocol_fee, TokenAmount(0));
        assert_eq!(fb.reviewer_fee, TokenAmount(0));
        assert_eq!(fb.net_amount, TokenAmount(0));
    }

    #[test]
    fn fee_breakdown_small_amount() {
        let fb = FeeBreakdown::from_gross(TokenAmount(1));
        let total_fees = fb.protocol_fee.0.saturating_add(fb.reviewer_fee.0);
        assert!(total_fees <= fb.gross_amount.0);
    }

    #[test]
    fn fee_breakdown_large_amount() {
        let fb = FeeBreakdown::from_gross(TokenAmount(1_000_000));
        assert_eq!(fb.protocol_fee, TokenAmount(30_000));
        assert_eq!(fb.reviewer_fee, TokenAmount(40_000));
        assert_eq!(fb.net_amount, TokenAmount(930_000));
    }

    #[test]
    fn fee_fees_dont_exceed_gross() {
        for amount in [1u64, 10, 100, 1000, 10000, 1_000_000] {
            let fb = FeeBreakdown::from_gross(TokenAmount(amount as u128));
            let total_fees = fb.protocol_fee.0.saturating_add(fb.reviewer_fee.0);
            assert!(total_fees <= amount as u128, "fees exceed gross for amount {amount}");
        }
    }

    #[test]
    fn manager_create_escrow() {
        let mut mgr = EscrowManager::new();
        let id = test_bounty_id("test");
        mgr.create_escrow(
            id.clone(),
            test_did("alice"),
            TokenAmount(1000),
            TokenType::Compute,
            100,
        )
        .unwrap();
        let escrow = mgr.get_escrow(&id).unwrap();
        assert_eq!(escrow.amount, TokenAmount(1000));
    }

    #[test]
    fn manager_duplicate_escrow_fails() {
        let mut mgr = EscrowManager::new();
        let id = test_bounty_id("test");
        mgr.create_escrow(
            id.clone(),
            test_did("alice"),
            TokenAmount(1000),
            TokenType::Compute,
            100,
        )
        .unwrap();
        let err = mgr
            .create_escrow(
                id.clone(),
                test_did("alice"),
                TokenAmount(1000),
                TokenType::Compute,
                200,
            )
            .unwrap_err();
        assert!(matches!(err, BountyError::AlreadyExists(_)));
    }

    #[test]
    fn manager_release() {
        let mut mgr = EscrowManager::new();
        let id = test_bounty_id("test");
        mgr.create_escrow(
            id.clone(),
            test_did("alice"),
            TokenAmount(1000),
            TokenType::Compute,
            100,
        )
        .unwrap();
        mgr.release(&id, test_did("bob")).unwrap();
        assert_eq!(mgr.get_escrow(&id).unwrap().state, EscrowState::Released);
    }

    #[test]
    fn manager_refund() {
        let mut mgr = EscrowManager::new();
        let id = test_bounty_id("test");
        mgr.create_escrow(
            id.clone(),
            test_did("alice"),
            TokenAmount(1000),
            TokenType::Compute,
            100,
        )
        .unwrap();
        mgr.refund(&id).unwrap();
        assert_eq!(mgr.get_escrow(&id).unwrap().state, EscrowState::Refunded);
    }

    #[test]
    fn manager_dispute() {
        let mut mgr = EscrowManager::new();
        let id = test_bounty_id("test");
        mgr.create_escrow(
            id.clone(),
            test_did("alice"),
            TokenAmount(1000),
            TokenType::Compute,
            100,
        )
        .unwrap();
        mgr.dispute(&id).unwrap();
        assert_eq!(mgr.get_escrow(&id).unwrap().state, EscrowState::Disputed);
    }

    #[test]
    fn manager_not_found() {
        let mgr = EscrowManager::new();
        assert!(mgr.get_escrow(&test_bounty_id("missing")).is_none());
    }

    #[test]
    fn manager_calculate_fees() {
        let mut mgr = EscrowManager::new();
        let id = test_bounty_id("test");
        mgr.create_escrow(
            id.clone(),
            test_did("alice"),
            TokenAmount(1000),
            TokenType::Compute,
            100,
        )
        .unwrap();
        let fees = mgr.calculate_fees(&id).unwrap();
        assert_eq!(fees.net_amount, TokenAmount(930));
    }

    #[test]
    fn manager_calculate_fees_missing() {
        let mgr = EscrowManager::new();
        assert!(mgr.calculate_fees(&test_bounty_id("missing")).is_none());
    }

    #[test]
    fn manager_double_release_prevented() {
        let mut mgr = EscrowManager::new();
        let id = test_bounty_id("test");
        mgr.create_escrow(
            id.clone(),
            test_did("alice"),
            TokenAmount(1000),
            TokenType::Compute,
            100,
        )
        .unwrap();
        mgr.release(&id, test_did("bob")).unwrap();
        let err = mgr.release(&id, test_did("charlie")).unwrap_err();
        assert!(matches!(err, BountyError::EscrowError(_)));
    }

    #[test]
    fn manager_double_refund_prevented() {
        let mut mgr = EscrowManager::new();
        let id = test_bounty_id("test");
        mgr.create_escrow(
            id.clone(),
            test_did("alice"),
            TokenAmount(1000),
            TokenType::Compute,
            100,
        )
        .unwrap();
        mgr.refund(&id).unwrap();
        let err = mgr.refund(&id).unwrap_err();
        assert!(matches!(err, BountyError::EscrowError(_)));
    }

    #[test]
    fn escrow_state_serde_roundtrip() {
        for state in [
            EscrowState::Funded,
            EscrowState::Released,
            EscrowState::Refunded,
            EscrowState::Disputed,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let back: EscrowState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, back);
        }
    }

    #[test]
    fn escrow_serde_roundtrip() {
        let escrow = Escrow::new(test_did("alice"), TokenAmount(500), TokenType::Train, 42);
        let json = serde_json::to_string(&escrow).unwrap();
        let back: Escrow = serde_json::from_str(&json).unwrap();
        assert_eq!(escrow, back);
    }

    #[test]
    fn fee_breakdown_serde_roundtrip() {
        let fb = FeeBreakdown::from_gross(TokenAmount(1000));
        let json = serde_json::to_string(&fb).unwrap();
        let back: FeeBreakdown = serde_json::from_str(&json).unwrap();
        assert_eq!(fb, back);
    }
}
