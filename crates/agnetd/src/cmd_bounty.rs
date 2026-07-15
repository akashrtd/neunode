use anyhow::Result;
use neunode_bounty::state_machine::{BountyData as LibBountyData, BountyStateMachine, Deadlines};
use neunode_core::types::{BountyId, BountyState, Timestamp, TokenAmount, TokenType};

use crate::cli::{BountyCommands, GlobalArgs};
use crate::output::OutputWriter;
use crate::state::AppState;
use crate::util::{parse_token_type, token_type_display};

pub fn execute(cmd: &BountyCommands, args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(args.output);
    match cmd {
        BountyCommands::Create {
            title,
            description,
            reward,
            token,
            claim_deadline,
            work_deadline,
        } => create_bounty(
            title,
            description,
            *reward,
            token,
            Some(*claim_deadline),
            Some(*work_deadline),
            &writer,
            state,
        ),
        BountyCommands::Claim { id, stake } => claim_bounty(id, *stake, &writer, state),
        BountyCommands::Submit { id, artifact, evidence } => {
            submit_bounty(id, artifact, evidence.as_deref().unwrap_or(""), &writer, state)
        }
        BountyCommands::Review { id, score, feedback } => {
            review_bounty(id, *score, feedback, &writer, state)
        }
        BountyCommands::List { state: bstate, creator, limit } => {
            list_bounties(bstate.as_deref(), creator.as_deref(), *limit, &writer, state)
        }
        BountyCommands::Show { id } => show_bounty(id, &writer, state),
        BountyCommands::Cancel { id, reason } => {
            cancel_bounty(id, reason.as_deref().unwrap_or(""), &writer, state)
        }
        BountyCommands::Pay { id } => pay_bounty(id, &writer, state),
    }
}

fn current_timestamp() -> Timestamp {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn generate_bounty_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let cnt = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("bnty_{:012x}{:04x}", ts, cnt & 0xFFFF)
}

// --- Private conversion helpers ---

fn parse_bounty_state(s: &str) -> Option<BountyState> {
    match s.to_lowercase().as_str() {
        "open" => Some(BountyState::Open),
        "claimed" => Some(BountyState::Claimed),
        "submitted" => Some(BountyState::Submitted),
        "underreview" => Some(BountyState::UnderReview),
        "revision" => Some(BountyState::Revision),
        "accepted" => Some(BountyState::Accepted),
        "rejected" => Some(BountyState::Rejected),
        "disputed" => Some(BountyState::Disputed),
        "paid" => Some(BountyState::Paid),
        "expired" => Some(BountyState::Expired),
        "cancelled" => Some(BountyState::Cancelled),
        _ => None,
    }
}

fn token_type_to_u8(t: &TokenType) -> u8 {
    match t {
        TokenType::Compute => 0x01,
        TokenType::Train => 0x02,
        TokenType::Bandwidth => 0x03,
        TokenType::Storage => 0x04,
    }
}

fn lib_to_storage(data: &LibBountyData, escrow: u64) -> neunode_storage::bounty_store::BountyData {
    neunode_storage::bounty_store::BountyData {
        id: data.id.0.clone(),
        state: format!("{:?}", data.state),
        requester_did: data.creator.0.clone(),
        provider_did: data.claimant.as_ref().map(|d| d.0.clone()),
        reward_amount: data.reward_amount.0 as u64,
        reward_token_type: token_type_to_u8(&data.reward_token),
        deadline: data.deadlines.work,
        created_at: data.created_at,
        escrow_deposited: escrow,
        title: data.title.clone(),
        description: data.description.clone(),
        claim_deadline: data.deadlines.claim,
        work_deadline: data.deadlines.work,
        review_deadline: data.deadlines.review,
        artifact_hash: data.artifact_hash.as_ref().map(|h| h.0.clone()),
        bond: data.bond.map(|b| b.0 as u64),
    }
}

// --- Subcommand handlers ---

#[allow(clippy::too_many_arguments)]
fn create_bounty(
    title: &str,
    description: &str,
    reward: u64,
    token: &str,
    claim_deadline: Option<u64>,
    work_deadline: Option<u64>,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    if title.is_empty() {
        anyhow::bail!("title cannot be empty");
    }
    if description.is_empty() {
        anyhow::bail!("description cannot be empty");
    }
    if reward == 0 {
        anyhow::bail!("reward must be greater than 0");
    }

    let token_type = parse_token_type(token)?;
    let now = current_timestamp();
    let creator = state.require_did()?.clone();
    let bounty_id = generate_bounty_id();

    let mut deadlines = Deadlines::from_created_at(now);
    if let Some(hours) = claim_deadline {
        deadlines.claim = now.saturating_add(hours * 3600);
    }
    if let Some(hours) = work_deadline {
        deadlines.work = now.saturating_add(hours * 3600);
    }

    let lib_data = LibBountyData {
        id: BountyId(bounty_id.clone()),
        creator,
        title: title.to_string(),
        description: description.to_string(),
        reward_amount: TokenAmount(reward as u128),
        reward_token: token_type,
        state: BountyState::Open,
        claimant: None,
        created_at: now,
        deadlines,
        artifact_hash: None,
        bond: None,
    };

    let sm = BountyStateMachine::new(lib_data);
    let store_data = lib_to_storage(sm.data(), reward);

    let store = state.bounty_store();
    let token_byte = token_type_to_u8(&token_type);
    let creator_did = sm.data().creator.0.clone();
    let escrow_did = format!("escrow:{bounty_id}");
    store.create_with_escrow(&store_data, &creator_did, &escrow_did, token_byte, reward as u128)?;

    let out = serde_json::json!({
        "id": bounty_id,
        "title": title,
        "reward": reward,
        "token": token_type_display(&token_type),
        "state": "Open",
        "claim_deadline": sm.data().deadlines.claim,
        "work_deadline": sm.data().deadlines.work,
    });

    writer.write_json(&out);
    writer.write_status(&format!("Bounty created and persisted: {bounty_id}"));
    Ok(())
}

fn claim_bounty(
    bounty_id: &str,
    stake: u64,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    let claimant = state.require_did()?.clone();
    let updated =
        crate::bounty_service::claim(state.db(), bounty_id, &claimant, stake, current_timestamp())?;

    let out = serde_json::json!({
        "bounty_id": bounty_id,
        "claimant": claimant.to_string(),
        "bond": stake,
        "state": updated.state,
    });

    writer.write_json(&out);
    writer.write_status(&format!("Bounty claimed: {bounty_id}"));
    Ok(())
}

fn submit_bounty(
    bounty_id: &str,
    artifact: &str,
    _evidence: &str,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    let actor = state.require_did()?;
    let updated =
        crate::bounty_service::submit(state.db(), bounty_id, actor, artifact, current_timestamp())?;

    let out = serde_json::json!({
        "bounty_id": bounty_id,
        "artifact_cid": artifact,
        "state": updated.state,
    });

    writer.write_json(&out);
    writer.write_status(&format!("Work submitted: {bounty_id}"));
    Ok(())
}

fn review_bounty(
    bounty_id: &str,
    score: u8,
    feedback: &str,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    let reviewer = state.require_did()?.clone();
    let updated = crate::bounty_service::review(
        state.db(),
        bounty_id,
        &reviewer,
        score,
        feedback,
        current_timestamp(),
    )?;

    let out = serde_json::json!({
        "bounty_id": bounty_id,
        "score": score,
        "feedback": feedback,
        "state": updated.state,
    });

    writer.write_json(&out);
    writer.write_status(&format!("Review submitted: {bounty_id} (score: {}/100)", score));
    Ok(())
}

fn list_bounties(
    state_filter: Option<&str>,
    creator_filter: Option<&str>,
    limit: usize,
    writer: &OutputWriter,
    app_state: &AppState,
) -> Result<()> {
    let store = app_state.bounty_store();
    let all = store.list_all()?;

    let filtered: Vec<_> = all
        .iter()
        .filter(|b| {
            state_filter.is_none_or(|sf| parse_bounty_state(sf) == parse_bounty_state(&b.state))
        })
        .filter(|b| creator_filter.is_none_or(|cf| b.requester_did.contains(cf)))
        .take(limit)
        .collect();

    if filtered.is_empty() {
        writer.write_status("No bounties found");
    } else {
        let headers = ["ID", "State", "Reward", "Creator", "Created"];
        let rows: Vec<Vec<String>> = filtered
            .iter()
            .map(|b| {
                vec![
                    b.id.clone(),
                    b.state.clone(),
                    format!("{} (type {})", b.reward_amount, b.reward_token_type),
                    b.requester_did.clone(),
                    b.created_at.to_string(),
                ]
            })
            .collect();
        writer.write_table(&headers, &rows);
    }
    Ok(())
}

fn show_bounty(bounty_id: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    if bounty_id.is_empty() {
        anyhow::bail!("bounty id cannot be empty");
    }

    let store = state.bounty_store();
    let bounty =
        store.get(bounty_id)?.ok_or_else(|| anyhow::anyhow!("bounty '{bounty_id}' not found"))?;

    let pairs = [
        ("ID", bounty.id.clone()),
        ("Title", bounty.title.clone()),
        ("Description", bounty.description.clone()),
        ("State", bounty.state.clone()),
        ("Creator", bounty.requester_did.clone()),
        ("Claimant", bounty.provider_did.clone().unwrap_or_else(|| "none".to_string())),
        ("Reward", format!("{} (type {})", bounty.reward_amount, bounty.reward_token_type)),
        ("Escrow", bounty.escrow_deposited.to_string()),
        ("Created", bounty.created_at.to_string()),
        ("Claim Deadline", bounty.claim_deadline.to_string()),
        ("Work Deadline", bounty.work_deadline.to_string()),
        ("Review Deadline", bounty.review_deadline.to_string()),
    ];

    writer.write_key_value_pairs(&pairs.iter().map(|(k, v)| (*k, v.as_str())).collect::<Vec<_>>());
    Ok(())
}

fn cancel_bounty(
    bounty_id: &str,
    reason: &str,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    let actor = state.require_did()?;
    crate::bounty_service::cancel(state.db(), bounty_id, actor, current_timestamp())?;

    let out = serde_json::json!({
        "bounty_id": bounty_id,
        "state": "Cancelled",
        "reason": reason,
    });

    writer.write_json(&out);
    writer.write_status(&format!("Bounty cancelled: {bounty_id}"));
    Ok(())
}

fn pay_bounty(bounty_id: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    let actor = state.require_did()?;
    let payment = crate::bounty_service::pay(state.db(), bounty_id, actor, current_timestamp())?;

    let out = serde_json::json!({
        "bounty_id": bounty_id,
        "claimant": payment.claimant,
        "amount_paid": payment.reward_paid,
        "bond_returned": payment.bond_returned,
        "token_type": payment.bounty.reward_token_type,
        "state": "Paid",
    });

    writer.write_json(&out);
    writer.write_status(&format!(
        "Bounty paid: {bounty_id} → {} tokens to {}",
        payment.reward_paid, payment.claimant
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{human_writer, json_writer as test_writer, test_state};

    #[test]
    fn create_bounty_valid() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("Test", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
    }

    #[test]
    fn create_bounty_persists() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("Persist Test", "Desc", 1000, "compute", None, None, &writer, &state)
            .unwrap();

        let store = state.bounty_store();
        let all = store.list_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].state, "Open");
        assert_eq!(all[0].reward_amount, 1000);
    }

    #[test]
    fn create_bounty_empty_title_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(create_bounty("", "Desc", 1000, "compute", None, None, &writer, &state).is_err());
    }

    #[test]
    fn create_bounty_empty_desc_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(create_bounty("Title", "", 1000, "compute", None, None, &writer, &state).is_err());
    }

    #[test]
    fn create_bounty_zero_reward_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(create_bounty("Title", "Desc", 0, "compute", None, None, &writer, &state).is_err());
    }

    #[test]
    fn create_bounty_invalid_token_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(
            create_bounty("Title", "Desc", 100, "invalid", None, None, &writer, &state).is_err()
        );
    }

    #[test]
    fn create_bounty_with_deadlines() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("Title", "Desc", 500, "compute", Some(48), Some(72), &writer, &state)
            .unwrap();
    }

    #[test]
    fn claim_bounty_valid() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("Claimable", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();

        let store = state.bounty_store();
        let all = store.list_all().unwrap();
        let bounty_id = &all[0].id;

        let writer2 = test_writer();
        // Stake must be >= 15% of reward (150 for 1000 reward)
        claim_bounty(bounty_id, 200, &writer2, &state).unwrap();

        let updated = store.get(bounty_id).unwrap().unwrap();
        assert_eq!(updated.state, "Claimed");
    }

    #[test]
    fn claim_bounty_not_found_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(claim_bounty("nonexistent", 200, &writer, &state).is_err());
    }

    #[test]
    fn claim_bounty_zero_stake_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(claim_bounty("bnty_001", 0, &writer, &state).is_err());
    }

    #[test]
    fn claim_bounty_empty_id_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(claim_bounty("", 200, &writer, &state).is_err());
    }

    #[test]
    fn submit_bounty_valid() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("Submit", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();

        let writer2 = test_writer();
        claim_bounty(&bounty_id, 200, &writer2, &state).unwrap();

        let writer3 = test_writer();
        submit_bounty(&bounty_id, "ipfs://QmX7b", "{}", &writer3, &state).unwrap();
    }

    #[test]
    fn submit_bounty_empty_cid_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(submit_bounty("bnty_001", "", "{}", &writer, &state).is_err());
    }

    #[test]
    fn submit_bounty_empty_id_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(submit_bounty("", "ipfs://QmX7b", "{}", &writer, &state).is_err());
    }

    #[test]
    fn review_bounty_valid_score() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("Reviewable", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();

        let writer2 = test_writer();
        claim_bounty(&bounty_id, 200, &writer2, &state).unwrap();

        let writer3 = test_writer();
        submit_bounty(&bounty_id, "ipfs://QmX7b", "{}", &writer3, &state).unwrap();

        let writer4 = test_writer();
        review_bounty(&bounty_id, 85, "Good work", &writer4, &state).unwrap();
    }

    #[test]
    fn review_bounty_score_over_100_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(review_bounty("bnty_001", 101, "Too high", &writer, &state).is_err());
    }

    #[test]
    fn list_bounties_empty() {
        let state = seeded_state();
        let writer = human_writer();
        list_bounties(None, None, 10, &writer, &state).unwrap();
    }

    #[test]
    fn list_bounties_after_create() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("List1", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
        create_bounty("List2", "Desc", 2000, "train", None, None, &writer, &state).unwrap();

        let writer2 = human_writer();
        list_bounties(None, None, 10, &writer2, &state).unwrap();

        assert_eq!(state.bounty_store().list_all().unwrap().len(), 2);
    }

    #[test]
    fn show_bounty_valid() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("Showable", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();

        let writer2 = test_writer();
        show_bounty(&bounty_id, &writer2, &state).unwrap();
    }

    #[test]
    fn show_bounty_not_found() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(show_bounty("nonexistent", &writer, &state).is_err());
    }

    #[test]
    fn show_bounty_empty_id_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(show_bounty("", &writer, &state).is_err());
    }

    #[test]
    fn cancel_bounty_valid() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("Cancellable", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();

        let writer2 = test_writer();
        cancel_bounty(&bounty_id, "No longer needed", &writer2, &state).unwrap();

        let updated = state.bounty_store().get(&bounty_id).unwrap().unwrap();
        assert_eq!(updated.state, "Cancelled");
    }

    #[test]
    fn cancel_bounty_empty_id_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(cancel_bounty("", "reason", &writer, &state).is_err());
    }

    #[test]
    fn submit_not_claimed_fails() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("NoClaim", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();

        let writer2 = test_writer();
        let result = submit_bounty(&bounty_id, "ipfs://QmX7b", "{}", &writer2, &state);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid state transition"), "unexpected error message: {err}");
    }

    #[test]
    fn submit_by_non_claimant_is_rejected_without_state_change() {
        let mut state = seeded_state();
        create_bounty(
            "Authorized submit",
            "Desc",
            1000,
            "compute",
            None,
            None,
            &test_writer(),
            &state,
        )
        .unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();
        claim_bounty(&bounty_id, 200, &test_writer(), &state).unwrap();
        state.active_did = Some(neunode_core::types::Did("did:neunode:intruder".to_string()));

        let result =
            submit_bounty(&bounty_id, "ipfs://QmUnauthorized", "{}", &test_writer(), &state);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("only the bounty claimant"));
        let bounty = state.bounty_store().get(&bounty_id).unwrap().unwrap();
        assert_eq!(bounty.state, "Claimed");
        assert!(bounty.artifact_hash.is_none());
    }

    #[test]
    fn review_not_submitted_fails() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("NoSubmit", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();

        let writer2 = test_writer();
        let result = review_bounty(&bounty_id, 80, "nice", &writer2, &state);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid state transition"), "unexpected error message: {err}");
    }

    #[test]
    fn cancel_wrong_state_fails() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("NoCancel", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();

        let writer2 = test_writer();
        claim_bounty(&bounty_id, 200, &writer2, &state).unwrap();

        let writer3 = test_writer();
        submit_bounty(&bounty_id, "ipfs://QmX7b", "{}", &writer3, &state).unwrap();

        let writer4 = test_writer();
        let result = cancel_bounty(&bounty_id, "too late", &writer4, &state);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid state transition"), "unexpected error message: {err}");
    }

    #[test]
    fn cancel_from_claimed_ok() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("CancelClaimed", "Desc", 1000, "compute", None, None, &writer, &state)
            .unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();

        let writer2 = test_writer();
        claim_bounty(&bounty_id, 200, &writer2, &state).unwrap();

        let writer3 = test_writer();
        cancel_bounty(&bounty_id, "changed mind", &writer3, &state).unwrap();

        let updated = state.bounty_store().get(&bounty_id).unwrap().unwrap();
        assert_eq!(updated.state, "Cancelled");
        assert_eq!(updated.escrow_deposited, 0);
        let escrow = state.token_store().get_balance(&format!("escrow:{bounty_id}"), 0x01).unwrap();
        assert_eq!(escrow.balance, 0, "reward and bond must both leave escrow");
        let actor = state.active_did.as_ref().unwrap();
        let actor_balance = state.token_store().get_balance(&actor.0, 0x01).unwrap();
        assert_eq!(actor_balance.balance, 100_000, "reward and provider bond must be returned");
    }

    // --- pay_bounty tests ---

    fn seed_token_balance(state: &AppState, token_byte: u8, balance: u128) {
        let did = state.active_did.as_ref().unwrap();
        let store = state.token_store();
        store
            .set_balance(
                &did.0,
                token_byte,
                &neunode_storage::token_store::TokenBalance {
                    balance,
                    staked: 0,
                    last_decay_epoch: 0,
                },
            )
            .unwrap();
    }

    fn seeded_state() -> AppState {
        let state = test_state();
        seed_token_balance(&state, 0x01, 100_000);
        seed_token_balance(&state, 0x02, 100_000);
        state
    }

    fn full_bounty_lifecycle_to_accepted(state: &AppState) -> String {
        let writer = test_writer();
        create_bounty("Payable", "Desc", 1000, "compute", None, None, &writer, state).unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();

        let writer2 = test_writer();
        claim_bounty(&bounty_id, 200, &writer2, state).unwrap();

        let writer3 = test_writer();
        submit_bounty(&bounty_id, "ipfs://QmPaid", "{}", &writer3, state).unwrap();

        let writer4 = test_writer();
        review_bounty(&bounty_id, 85, "Good work", &writer4, state).unwrap();

        bounty_id
    }

    #[test]
    fn pay_bounty_full_flow() {
        let state = seeded_state();

        let bounty_id = full_bounty_lifecycle_to_accepted(&state);

        let writer = test_writer();
        pay_bounty(&bounty_id, &writer, &state).unwrap();

        let updated = state.bounty_store().get(&bounty_id).unwrap().unwrap();
        assert_eq!(updated.state, "Paid");
    }

    #[test]
    fn pay_bounty_transfers_tokens_to_claimant() {
        let state = seeded_state();

        let bounty_id = full_bounty_lifecycle_to_accepted(&state);

        let bounty = state.bounty_store().get(&bounty_id).unwrap().unwrap();
        let claimant_did = bounty.provider_did.unwrap();

        let writer = test_writer();
        pay_bounty(&bounty_id, &writer, &state).unwrap();

        let escrow_bal =
            state.token_store().get_balance(&format!("escrow:{}", bounty_id), 0x01).unwrap();
        assert_eq!(escrow_bal.balance, 0, "reward and provider bond must leave escrow");

        let claimant_bal = state.token_store().get_balance(&claimant_did, 0x01).unwrap();
        // Same DID for creator+claimant: both reward and bond return on the happy path.
        assert_eq!(claimant_bal.balance, 100_000, "payout must conserve reward and bond");
        let paid = state.bounty_store().get(&bounty_id).unwrap().unwrap();
        assert_eq!(paid.escrow_deposited, 0);
    }

    #[test]
    fn claim_insufficient_balance_preserves_open_state_and_reward_escrow() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("Atomic claim", "Desc", 1000, "compute", None, None, &writer, &state)
            .unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();
        let actor = state.active_did.as_ref().unwrap();
        state
            .token_store()
            .set_balance(&actor.0, 0x01, &neunode_storage::token_store::TokenBalance::default())
            .unwrap();

        let result = claim_bounty(&bounty_id, 200, &test_writer(), &state);

        assert!(result.is_err());
        let bounty = state.bounty_store().get(&bounty_id).unwrap().unwrap();
        assert_eq!(bounty.state, "Open");
        assert!(bounty.provider_did.is_none());
        assert!(bounty.bond.is_none());
        let escrow = state.token_store().get_balance(&format!("escrow:{bounty_id}"), 0x01).unwrap();
        assert_eq!(escrow.balance, 1000, "reward escrow must remain unchanged");
    }

    #[test]
    fn pay_bounty_empty_id_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(pay_bounty("", &writer, &state).is_err());
    }

    #[test]
    fn pay_bounty_not_found_fails() {
        let state = seeded_state();
        let writer = test_writer();
        assert!(pay_bounty("nonexistent", &writer, &state).is_err());
    }

    #[test]
    fn pay_bounty_not_accepted_fails() {
        let state = seeded_state();
        let writer = test_writer();
        create_bounty("NotAccepted", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();

        let writer2 = test_writer();
        let result = pay_bounty(&bounty_id, &writer2, &state);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid state transition"), "unexpected error: {err}");
    }

    #[test]
    fn create_bounty_escrows_tokens() {
        let state = seeded_state();
        seed_token_balance(&state, 0x01, 5000);
        let writer = test_writer();
        create_bounty("EscrowTest", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();

        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();
        let escrow_bal =
            state.token_store().get_balance(&format!("escrow:{}", bounty_id), 0x01).unwrap();
        assert_eq!(escrow_bal.balance, 1000);

        let did = state.active_did.as_ref().unwrap();
        let creator_bal = state.token_store().get_balance(&did.0, 0x01).unwrap();
        assert_eq!(creator_bal.balance, 4000);
    }

    #[test]
    fn cancel_bounty_refunds_escrow() {
        let state = seeded_state();
        seed_token_balance(&state, 0x01, 5000);
        let writer = test_writer();
        create_bounty("RefundTest", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();

        let did = state.active_did.as_ref().unwrap();
        let after_create = state.token_store().get_balance(&did.0, 0x01).unwrap();
        assert_eq!(after_create.balance, 4000);

        let writer2 = test_writer();
        cancel_bounty(&bounty_id, "changed mind", &writer2, &state).unwrap();

        let after_cancel = state.token_store().get_balance(&did.0, 0x01).unwrap();
        assert_eq!(after_cancel.balance, 5000, "creator should be refunded");
    }
}
