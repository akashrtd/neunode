use anyhow::Result;
use neunode_core::types::{Timestamp, TokenType};

use crate::cli::{BountyCommands, Cli};
use crate::output::OutputWriter;
use crate::state::AppState;

pub fn execute(cmd: &BountyCommands, cli: &Cli, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
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
    }
}

fn parse_token_type(s: &str) -> Result<TokenType> {
    match s.to_lowercase().as_str() {
        "compute" | "ncompute" => Ok(TokenType::Compute),
        "train" | "ntrain" => Ok(TokenType::Train),
        "bandwidth" | "nbandwidth" => Ok(TokenType::Bandwidth),
        "storage" | "nstorage" => Ok(TokenType::Storage),
        _ => anyhow::bail!(
            "invalid token type '{}'. Must be one of: compute, train, bandwidth, storage",
            s
        ),
    }
}

fn token_type_str(t: &TokenType) -> &'static str {
    match t {
        TokenType::Compute => "Compute",
        TokenType::Train => "Train",
        TokenType::Bandwidth => "Bandwidth",
        TokenType::Storage => "Storage",
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

    let claim_dl =
        claim_deadline.map(|d| now.saturating_add(d)).unwrap_or(now.saturating_add(7 * 24 * 3600));
    let work_dl =
        work_deadline.map(|d| now.saturating_add(d)).unwrap_or(now.saturating_add(30 * 24 * 3600));

    let store = state.bounty_store();
    store.put(&neunode_storage::bounty_store::BountyData {
        id: bounty_id.clone(),
        state: "Open".to_string(),
        requester_did: creator.to_string(),
        provider_did: None,
        reward_amount: reward,
        reward_token_type: token_type_to_u8(&token_type),
        deadline: work_dl,
        created_at: now,
        escrow_deposited: reward,
    })?;

    let out = serde_json::json!({
        "id": bounty_id,
        "title": title,
        "reward": reward,
        "token": token_type_str(&token_type),
        "state": "Open",
        "claim_deadline": claim_dl,
        "work_deadline": work_dl,
    });

    writer.write_json(&out);
    writer.write_status(&format!("Bounty created and persisted: {bounty_id}"));
    Ok(())
}

fn token_type_to_u8(t: &TokenType) -> u8 {
    match t {
        TokenType::Compute => 0x01,
        TokenType::Train => 0x02,
        TokenType::Bandwidth => 0x03,
        TokenType::Storage => 0x04,
    }
}

fn claim_bounty(
    bounty_id: &str,
    stake: u64,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    if bounty_id.is_empty() {
        anyhow::bail!("bounty id cannot be empty");
    }
    if stake == 0 {
        anyhow::bail!("stake must be greater than 0");
    }

    let claimant = state.require_did()?.clone();
    let store = state.bounty_store();

    let mut bounty =
        store.get(bounty_id)?.ok_or_else(|| anyhow::anyhow!("bounty '{bounty_id}' not found"))?;

    if bounty.state != "Open" {
        anyhow::bail!("bounty '{bounty_id}' is not Open (current state: {})", bounty.state);
    }

    bounty.state = "Claimed".to_string();
    bounty.provider_did = Some(claimant.0.clone());
    store.put(&bounty)?;

    let out = serde_json::json!({
        "bounty_id": bounty_id,
        "claimant": claimant.to_string(),
        "bond": stake,
        "state": "Claimed",
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
    if bounty_id.is_empty() {
        anyhow::bail!("bounty id cannot be empty");
    }
    if artifact.is_empty() {
        anyhow::bail!("artifact CID cannot be empty");
    }

    let store = state.bounty_store();
    let _ = store.update_state(bounty_id, "Submitted").ok();

    let out = serde_json::json!({
        "bounty_id": bounty_id,
        "artifact_cid": artifact,
        "state": "Submitted",
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
    if bounty_id.is_empty() {
        anyhow::bail!("bounty id cannot be empty");
    }
    if score > 100 {
        anyhow::bail!("invalid score: {} (must be 0-100)", score);
    }

    let store = state.bounty_store();
    let new_state = if score >= 70 { "Accepted" } else { "UnderReview" };
    store.update_state(bounty_id, new_state)?;

    let out = serde_json::json!({
        "bounty_id": bounty_id,
        "score": score,
        "feedback": feedback,
        "state": new_state,
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
        .filter(|b| state_filter.is_none_or(|sf| b.state.eq_ignore_ascii_case(sf)))
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
        ("State", bounty.state.clone()),
        ("Creator", bounty.requester_did.clone()),
        ("Claimant", bounty.provider_did.clone().unwrap_or_else(|| "none".to_string())),
        ("Reward", format!("{} (type {})", bounty.reward_amount, bounty.reward_token_type)),
        ("Deadline", bounty.deadline.to_string()),
        ("Created", bounty.created_at.to_string()),
        ("Escrow", bounty.escrow_deposited.to_string()),
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
    if bounty_id.is_empty() {
        anyhow::bail!("bounty id cannot be empty");
    }

    let store = state.bounty_store();
    store.update_state(bounty_id, "Cancelled")?;

    let out = serde_json::json!({
        "bounty_id": bounty_id,
        "state": "Cancelled",
        "reason": reason,
    });

    writer.write_json(&out);
    writer.write_status(&format!("Bounty cancelled: {bounty_id}"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::OutputFormat;
    use crate::config::CliConfig;
    use crate::state::AppState;
    use neunode_identity::keyring::Keyring;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    };

    fn test_writer() -> OutputWriter {
        OutputWriter::new(OutputFormat::Json)
    }

    fn human_writer() -> OutputWriter {
        OutputWriter::new(OutputFormat::Human)
    }

    fn test_state() -> AppState {
        static TEST_ID: AtomicU64 = AtomicU64::new(0);
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "agnetd_test_bounty_{:?}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = neunode_storage::db::NeunodeDb::open(&dir).unwrap();

        let kr = Keyring::generate();
        let did = kr.to_did();

        AppState {
            db: Arc::new(db),
            config: CliConfig::load(None).unwrap(),
            active_keyring: Some(kr),
            active_did: Some(did),
        }
    }

    #[test]
    fn create_bounty_valid() {
        let state = test_state();
        let writer = test_writer();
        create_bounty("Test", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
    }

    #[test]
    fn create_bounty_persists() {
        let state = test_state();
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
        let state = test_state();
        let writer = test_writer();
        assert!(create_bounty("", "Desc", 1000, "compute", None, None, &writer, &state).is_err());
    }

    #[test]
    fn create_bounty_empty_desc_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(create_bounty("Title", "", 1000, "compute", None, None, &writer, &state).is_err());
    }

    #[test]
    fn create_bounty_zero_reward_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(create_bounty("Title", "Desc", 0, "compute", None, None, &writer, &state).is_err());
    }

    #[test]
    fn create_bounty_invalid_token_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(
            create_bounty("Title", "Desc", 100, "invalid", None, None, &writer, &state).is_err()
        );
    }

    #[test]
    fn create_bounty_with_deadlines() {
        let state = test_state();
        let writer = test_writer();
        create_bounty("Title", "Desc", 500, "compute", Some(3600), Some(7200), &writer, &state)
            .unwrap();
    }

    #[test]
    fn claim_bounty_valid() {
        let state = test_state();
        let writer = test_writer();
        create_bounty("Claimable", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();

        let store = state.bounty_store();
        let all = store.list_all().unwrap();
        let bounty_id = &all[0].id;

        let writer2 = test_writer();
        claim_bounty(bounty_id, 100, &writer2, &state).unwrap();

        let updated = store.get(bounty_id).unwrap().unwrap();
        assert_eq!(updated.state, "Claimed");
    }

    #[test]
    fn claim_bounty_not_found_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(claim_bounty("nonexistent", 100, &writer, &state).is_err());
    }

    #[test]
    fn claim_bounty_zero_stake_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(claim_bounty("bnty_001", 0, &writer, &state).is_err());
    }

    #[test]
    fn claim_bounty_empty_id_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(claim_bounty("", 100, &writer, &state).is_err());
    }

    #[test]
    fn submit_bounty_valid() {
        let state = test_state();
        let writer = test_writer();
        submit_bounty("bnty_001", "ipfs://QmX7b", "{}", &writer, &state).unwrap();
    }

    #[test]
    fn submit_bounty_empty_cid_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(submit_bounty("bnty_001", "", "{}", &writer, &state).is_err());
    }

    #[test]
    fn submit_bounty_empty_id_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(submit_bounty("", "ipfs://QmX7b", "{}", &writer, &state).is_err());
    }

    #[test]
    fn review_bounty_valid_score() {
        let state = test_state();
        let writer = test_writer();
        create_bounty("Reviewable", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();

        let writer2 = test_writer();
        review_bounty(&bounty_id, 85, "Good work", &writer2, &state).unwrap();
    }

    #[test]
    fn review_bounty_score_over_100_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(review_bounty("bnty_001", 101, "Too high", &writer, &state).is_err());
    }

    #[test]
    fn list_bounties_empty() {
        let state = test_state();
        let writer = human_writer();
        list_bounties(None, None, 10, &writer, &state).unwrap();
    }

    #[test]
    fn list_bounties_after_create() {
        let state = test_state();
        let writer = test_writer();
        create_bounty("List1", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
        create_bounty("List2", "Desc", 2000, "train", None, None, &writer, &state).unwrap();

        let writer2 = human_writer();
        list_bounties(None, None, 10, &writer2, &state).unwrap();

        assert_eq!(state.bounty_store().list_all().unwrap().len(), 2);
    }

    #[test]
    fn show_bounty_valid() {
        let state = test_state();
        let writer = test_writer();
        create_bounty("Showable", "Desc", 1000, "compute", None, None, &writer, &state).unwrap();
        let bounty_id = state.bounty_store().list_all().unwrap()[0].id.clone();

        let writer2 = test_writer();
        show_bounty(&bounty_id, &writer2, &state).unwrap();
    }

    #[test]
    fn show_bounty_not_found() {
        let state = test_state();
        let writer = test_writer();
        assert!(show_bounty("nonexistent", &writer, &state).is_err());
    }

    #[test]
    fn show_bounty_empty_id_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(show_bounty("", &writer, &state).is_err());
    }

    #[test]
    fn cancel_bounty_valid() {
        let state = test_state();
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
        let state = test_state();
        let writer = test_writer();
        assert!(cancel_bounty("", "reason", &writer, &state).is_err());
    }

    #[test]
    fn parse_token_type_all_variants() {
        assert!(matches!(parse_token_type("compute"), Ok(TokenType::Compute)));
        assert!(matches!(parse_token_type("ncompute"), Ok(TokenType::Compute)));
        assert!(matches!(parse_token_type("train"), Ok(TokenType::Train)));
        assert!(matches!(parse_token_type("ntrain"), Ok(TokenType::Train)));
        assert!(matches!(parse_token_type("bandwidth"), Ok(TokenType::Bandwidth)));
        assert!(matches!(parse_token_type("nbandwidth"), Ok(TokenType::Bandwidth)));
        assert!(matches!(parse_token_type("storage"), Ok(TokenType::Storage)));
        assert!(matches!(parse_token_type("nstorage"), Ok(TokenType::Storage)));
        assert!(parse_token_type("invalid").is_err());
    }
}
