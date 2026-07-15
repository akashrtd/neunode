use std::fmt;

use neunode_bounty::state_machine::{
    BountyData as LibBountyData, BountyEvent, BountyStateMachine, Deadlines,
};
use neunode_core::types::{BountyId, Did, Hash256, Timestamp, TokenAmount, TokenType};
use neunode_storage::bounty_store::{BountyData, BountyStore};
use neunode_storage::db::NeunodeDb;
use neunode_storage::error::StorageError;

#[derive(Debug)]
pub enum BountyServiceError {
    NotFound(String),
    Invalid(String),
    Storage(StorageError),
}

impl fmt::Display for BountyServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "bounty '{id}' not found"),
            Self::Invalid(message) => f.write_str(message),
            Self::Storage(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for BountyServiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            _ => None,
        }
    }
}

impl From<StorageError> for BountyServiceError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

pub type Result<T> = std::result::Result<T, BountyServiceError>;

#[derive(Debug)]
pub struct PaymentResult {
    pub bounty: BountyData,
    pub claimant: String,
    pub reward_paid: u64,
    pub bond_returned: u64,
}

pub fn claim(
    db: &NeunodeDb,
    bounty_id: &str,
    claimant: &Did,
    stake: u64,
    now: Timestamp,
) -> Result<BountyData> {
    db.with_ledger_write(|| claim_locked(db, bounty_id, claimant, stake, now))
}

fn claim_locked(
    db: &NeunodeDb,
    bounty_id: &str,
    claimant: &Did,
    stake: u64,
    now: Timestamp,
) -> Result<BountyData> {
    if bounty_id.is_empty() {
        return Err(BountyServiceError::Invalid("bounty id cannot be empty".to_string()));
    }
    if stake == 0 {
        return Err(BountyServiceError::Invalid("stake must be greater than 0".to_string()));
    }

    let store = BountyStore::new(db);
    let bounty = load(&store, bounty_id)?;
    let mut sm = BountyStateMachine::new(storage_to_lib(&bounty)?);
    sm.try_transition(
        BountyEvent::Claim { claimant: claimant.clone(), bond: TokenAmount(stake as u128) },
        now,
    )
    .map_err(|error| BountyServiceError::Invalid(error.to_string()))?;

    let updated = lib_to_storage(sm.data(), bounty.escrow_deposited)?;
    store.transition_with_transfer(
        &updated,
        &claimant.0,
        &escrow_did(bounty_id),
        bounty.reward_token_type,
        stake as u128,
        now,
    )?;
    Ok(updated)
}

pub fn submit(
    db: &NeunodeDb,
    bounty_id: &str,
    actor: &Did,
    artifact: &str,
    now: Timestamp,
) -> Result<BountyData> {
    db.with_ledger_write(|| submit_locked(db, bounty_id, actor, artifact, now))
}

fn submit_locked(
    db: &NeunodeDb,
    bounty_id: &str,
    actor: &Did,
    artifact: &str,
    now: Timestamp,
) -> Result<BountyData> {
    if bounty_id.is_empty() {
        return Err(BountyServiceError::Invalid("bounty id cannot be empty".to_string()));
    }
    if artifact.is_empty() {
        return Err(BountyServiceError::Invalid("artifact CID cannot be empty".to_string()));
    }

    let store = BountyStore::new(db);
    let bounty = load(&store, bounty_id)?;
    let mut sm = BountyStateMachine::new(storage_to_lib(&bounty)?);
    sm.try_transition(BountyEvent::Submit { artifact_hash: Hash256(artifact.to_string()) }, now)
        .map_err(|error| BountyServiceError::Invalid(error.to_string()))?;
    require_claimant(&bounty, actor)?;
    let updated = lib_to_storage(sm.data(), bounty.escrow_deposited)?;
    store.put_with_audit(&updated, &actor.0, "bounty.submit", now)?;
    Ok(updated)
}

pub fn review(
    db: &NeunodeDb,
    bounty_id: &str,
    reviewer: &Did,
    score: u8,
    feedback: &str,
    now: Timestamp,
) -> Result<BountyData> {
    db.with_ledger_write(|| review_locked(db, bounty_id, reviewer, score, feedback, now))
}

fn review_locked(
    db: &NeunodeDb,
    bounty_id: &str,
    reviewer: &Did,
    score: u8,
    feedback: &str,
    now: Timestamp,
) -> Result<BountyData> {
    if bounty_id.is_empty() {
        return Err(BountyServiceError::Invalid("bounty id cannot be empty".to_string()));
    }
    if score > 100 {
        return Err(BountyServiceError::Invalid(format!("invalid score: {score} (must be 0-100)")));
    }

    let store = BountyStore::new(db);
    let bounty = load(&store, bounty_id)?;
    require_creator(&bounty, reviewer)?;
    let mut sm = BountyStateMachine::new(storage_to_lib(&bounty)?);
    if sm.current_state() == neunode_core::types::BountyState::Submitted {
        sm.try_transition(BountyEvent::StartReview, now)
            .map_err(|error| BountyServiceError::Invalid(error.to_string()))?;
    }
    sm.try_transition(
        BountyEvent::SubmitReview {
            reviewer: reviewer.clone(),
            score,
            notes: feedback.to_string(),
            signature: None,
        },
        now,
    )
    .map_err(|error| BountyServiceError::Invalid(error.to_string()))?;
    if score >= 60 {
        sm.try_transition(BountyEvent::Accept, now)
            .map_err(|error| BountyServiceError::Invalid(error.to_string()))?;
    } else if score < 40 {
        sm.try_transition(BountyEvent::Reject, now)
            .map_err(|error| BountyServiceError::Invalid(error.to_string()))?;
    }
    let updated = lib_to_storage(sm.data(), bounty.escrow_deposited)?;
    store.put_with_audit(&updated, &reviewer.0, "bounty.review", now)?;
    Ok(updated)
}

pub fn cancel(db: &NeunodeDb, bounty_id: &str, actor: &Did, now: Timestamp) -> Result<BountyData> {
    db.with_ledger_write(|| cancel_locked(db, bounty_id, actor, now))
}

fn cancel_locked(
    db: &NeunodeDb,
    bounty_id: &str,
    actor: &Did,
    now: Timestamp,
) -> Result<BountyData> {
    if bounty_id.is_empty() {
        return Err(BountyServiceError::Invalid("bounty id cannot be empty".to_string()));
    }

    let store = BountyStore::new(db);
    let bounty = load(&store, bounty_id)?;
    require_creator(&bounty, actor)?;
    let mut sm = BountyStateMachine::new(storage_to_lib(&bounty)?);
    sm.try_transition(BountyEvent::Cancel, now)
        .map_err(|error| BountyServiceError::Invalid(error.to_string()))?;

    let mut updated = lib_to_storage(sm.data(), 0)?;
    updated.escrow_deposited = 0;
    let bond = bounty.bond.unwrap_or(0);
    let mut payouts = vec![(bounty.requester_did.as_str(), bounty.reward_amount as u128)];
    if bond > 0 {
        let provider = bounty.provider_did.as_deref().ok_or_else(|| {
            BountyServiceError::Invalid("claimed bounty has no provider".to_string())
        })?;
        payouts.push((provider, bond as u128));
    }
    store.transition_with_payouts(
        &updated,
        &escrow_did(bounty_id),
        bounty.reward_token_type,
        &payouts,
        now,
    )?;
    Ok(updated)
}

pub fn pay(db: &NeunodeDb, bounty_id: &str, actor: &Did, now: Timestamp) -> Result<PaymentResult> {
    db.with_ledger_write(|| pay_locked(db, bounty_id, actor, now))
}

fn pay_locked(
    db: &NeunodeDb,
    bounty_id: &str,
    actor: &Did,
    now: Timestamp,
) -> Result<PaymentResult> {
    if bounty_id.is_empty() {
        return Err(BountyServiceError::Invalid("bounty id cannot be empty".to_string()));
    }

    let store = BountyStore::new(db);
    let bounty = load(&store, bounty_id)?;
    require_creator(&bounty, actor)?;
    let mut sm = BountyStateMachine::new(storage_to_lib(&bounty)?);
    sm.try_transition(BountyEvent::Pay, now)
        .map_err(|error| BountyServiceError::Invalid(error.to_string()))?;
    let claimant = sm
        .data()
        .claimant
        .as_ref()
        .ok_or_else(|| BountyServiceError::Invalid("bounty has no claimant".to_string()))?
        .0
        .clone();
    let bond = bounty.bond.unwrap_or(0);
    let total = (bounty.reward_amount as u128)
        .checked_add(bond as u128)
        .ok_or_else(|| BountyServiceError::Invalid("payout amount overflow".to_string()))?;
    let mut updated = lib_to_storage(sm.data(), 0)?;
    updated.escrow_deposited = 0;
    store.transition_with_payouts(
        &updated,
        &escrow_did(bounty_id),
        bounty.reward_token_type,
        &[(claimant.as_str(), total)],
        now,
    )?;

    Ok(PaymentResult {
        bounty: updated,
        claimant,
        reward_paid: bounty.reward_amount,
        bond_returned: bond,
    })
}

fn load(store: &BountyStore<'_>, bounty_id: &str) -> Result<BountyData> {
    store.get(bounty_id)?.ok_or_else(|| BountyServiceError::NotFound(bounty_id.to_string()))
}

fn require_creator(bounty: &BountyData, actor: &Did) -> Result<()> {
    if bounty.requester_did != actor.0 {
        return Err(BountyServiceError::Invalid(
            "only the bounty creator may perform this action".to_string(),
        ));
    }
    Ok(())
}

fn require_claimant(bounty: &BountyData, actor: &Did) -> Result<()> {
    if bounty.provider_did.as_deref() != Some(actor.0.as_str()) {
        return Err(BountyServiceError::Invalid(
            "only the bounty claimant may submit work".to_string(),
        ));
    }
    Ok(())
}

fn escrow_did(bounty_id: &str) -> String {
    format!("escrow:{bounty_id}")
}

fn parse_bounty_state(value: &str) -> Result<neunode_core::types::BountyState> {
    use neunode_core::types::BountyState;
    match value.to_ascii_lowercase().as_str() {
        "open" => Ok(BountyState::Open),
        "claimed" => Ok(BountyState::Claimed),
        "submitted" => Ok(BountyState::Submitted),
        "underreview" => Ok(BountyState::UnderReview),
        "revision" => Ok(BountyState::Revision),
        "accepted" => Ok(BountyState::Accepted),
        "rejected" => Ok(BountyState::Rejected),
        "disputed" => Ok(BountyState::Disputed),
        "paid" => Ok(BountyState::Paid),
        "expired" => Ok(BountyState::Expired),
        "cancelled" => Ok(BountyState::Cancelled),
        _ => {
            Err(BountyServiceError::Invalid(format!("invalid bounty state in storage: '{value}'")))
        }
    }
}

fn token_type_from_u8(value: u8) -> Result<TokenType> {
    match value {
        0x01 => Ok(TokenType::Compute),
        0x02 => Ok(TokenType::Train),
        0x03 => Ok(TokenType::Bandwidth),
        0x04 => Ok(TokenType::Storage),
        _ => Err(BountyServiceError::Invalid(format!("invalid token type code: {value}"))),
    }
}

fn token_type_to_u8(value: &TokenType) -> u8 {
    match value {
        TokenType::Compute => 0x01,
        TokenType::Train => 0x02,
        TokenType::Bandwidth => 0x03,
        TokenType::Storage => 0x04,
    }
}

fn storage_to_lib(data: &BountyData) -> Result<LibBountyData> {
    let mut deadlines = Deadlines::from_created_at(data.created_at);
    deadlines.claim = data.claim_deadline;
    deadlines.work = data.work_deadline;
    deadlines.review = data.review_deadline;
    Ok(LibBountyData {
        id: BountyId(data.id.clone()),
        creator: Did(data.requester_did.clone()),
        title: data.title.clone(),
        description: data.description.clone(),
        reward_amount: TokenAmount(data.reward_amount as u128),
        reward_token: token_type_from_u8(data.reward_token_type)?,
        state: parse_bounty_state(&data.state)?,
        claimant: data.provider_did.as_ref().map(|did| Did(did.clone())),
        created_at: data.created_at,
        deadlines,
        artifact_hash: data.artifact_hash.as_ref().map(|hash| Hash256(hash.clone())),
        bond: data.bond.map(|bond| TokenAmount(bond as u128)),
    })
}

fn lib_to_storage(data: &LibBountyData, escrow: u64) -> Result<BountyData> {
    let reward_amount = u64::try_from(data.reward_amount.0)
        .map_err(|_| BountyServiceError::Invalid("reward exceeds storage range".to_string()))?;
    let bond = data
        .bond
        .map(|value| u64::try_from(value.0))
        .transpose()
        .map_err(|_| BountyServiceError::Invalid("bond exceeds storage range".to_string()))?;
    Ok(BountyData {
        id: data.id.0.clone(),
        state: format!("{:?}", data.state),
        requester_did: data.creator.0.clone(),
        provider_did: data.claimant.as_ref().map(|did| did.0.clone()),
        reward_amount,
        reward_token_type: token_type_to_u8(&data.reward_token),
        deadline: data.deadlines.work,
        created_at: data.created_at,
        escrow_deposited: escrow,
        title: data.title.clone(),
        description: data.description.clone(),
        claim_deadline: data.deadlines.claim,
        work_deadline: data.deadlines.work,
        review_deadline: data.deadlines.review,
        artifact_hash: data.artifact_hash.as_ref().map(|hash| hash.0.clone()),
        bond,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use neunode_storage::token_store::{TokenBalance, TokenStore, TOKEN_COMPUTE};
    use std::sync::{Arc, Barrier};

    fn temp_db() -> NeunodeDb {
        let path = std::env::temp_dir().join(format!(
            "neunode_bounty_service_concurrency_{}_{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        NeunodeDb::open(&path).unwrap()
    }

    #[test]
    fn concurrent_claims_lock_exactly_one_provider_bond() {
        let db = Arc::new(temp_db());
        let creator = "did:neunode:creator";
        let claimants = [Did("did:neunode:alice".to_string()), Did("did:neunode:bob".to_string())];
        let token_store = TokenStore::new(&db);
        token_store
            .set_balance(
                creator,
                TOKEN_COMPUTE,
                &TokenBalance { balance: 500, ..Default::default() },
            )
            .unwrap();
        for claimant in &claimants {
            token_store
                .set_balance(
                    &claimant.0,
                    TOKEN_COMPUTE,
                    &TokenBalance { balance: 100, ..Default::default() },
                )
                .unwrap();
        }
        let bounty = BountyData {
            id: "bnty_concurrent".to_string(),
            state: "Open".to_string(),
            requester_did: creator.to_string(),
            provider_did: None,
            reward_amount: 500,
            reward_token_type: TOKEN_COMPUTE,
            deadline: 20_000,
            created_at: 1_000,
            escrow_deposited: 500,
            title: "Concurrent claim".to_string(),
            description: "Only one claimant may win".to_string(),
            claim_deadline: 10_000,
            work_deadline: 20_000,
            review_deadline: 30_000,
            artifact_hash: None,
            bond: None,
        };
        BountyStore::new(&db)
            .create_with_escrow(&bounty, creator, "escrow:bnty_concurrent", TOKEN_COMPUTE, 500)
            .unwrap();

        let barrier = Arc::new(Barrier::new(3));
        let handles: Vec<_> = claimants
            .iter()
            .cloned()
            .map(|claimant| {
                let db = Arc::clone(&db);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    claim(&db, "bnty_concurrent", &claimant, 75, 2_000)
                })
            })
            .collect();
        barrier.wait();
        let results: Vec<_> = handles.into_iter().map(|handle| handle.join().unwrap()).collect();

        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        let stored = BountyStore::new(&db).get("bnty_concurrent").unwrap().unwrap();
        let winner = stored.provider_did.expect("one claimant persisted");
        assert_eq!(stored.state, "Claimed");
        assert_eq!(stored.bond, Some(75));
        assert_eq!(
            TokenStore::new(&db)
                .get_balance("escrow:bnty_concurrent", TOKEN_COMPUTE)
                .unwrap()
                .balance,
            575
        );
        for claimant in claimants {
            let expected = if claimant.0 == winner { 25 } else { 100 };
            assert_eq!(
                TokenStore::new(&db).get_balance(&claimant.0, TOKEN_COMPUTE).unwrap().balance,
                expected
            );
        }
    }
}
