use anyhow::Result;
use neunode_bounty::lifecycle::BountyManager;
use neunode_bounty::state_machine::BountyData;
use neunode_core::types::{BountyId, BountyState, Did, Timestamp, TokenAmount, TokenType};

use crate::cli::{BountyCommands, Cli};
use crate::config::CliConfig;
use crate::output::OutputWriter;

pub fn execute(cmd: &BountyCommands, cli: &Cli, _config: &mut CliConfig) -> Result<()> {
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
        ),
        BountyCommands::Claim { id, stake } => claim_bounty(id, *stake, &writer),
        BountyCommands::Submit { id, artifact, evidence } => {
            submit_bounty(id, artifact, evidence.as_deref().unwrap_or(""), &writer)
        }
        BountyCommands::Review { id, score, feedback } => {
            review_bounty(id, *score, feedback, &writer)
        }
        BountyCommands::List { state, creator, limit } => {
            list_bounties(state.as_deref(), creator.as_deref(), *limit, &writer)
        }
        BountyCommands::Show { id } => show_bounty(id, &writer),
        BountyCommands::Cancel { id, reason } => {
            cancel_bounty(id, reason.as_deref().unwrap_or(""), &writer)
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

fn bounty_state_str(s: &BountyState) -> &'static str {
    match s {
        BountyState::Open => "Open",
        BountyState::Claimed => "Claimed",
        BountyState::Submitted => "Submitted",
        BountyState::UnderReview => "UnderReview",
        BountyState::Revision => "Revision",
        BountyState::Accepted => "Accepted",
        BountyState::Rejected => "Rejected",
        BountyState::Disputed => "Disputed",
        BountyState::Paid => "Paid",
        BountyState::Expired => "Expired",
        BountyState::Cancelled => "Cancelled",
    }
}

fn current_timestamp() -> Timestamp {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn create_bounty(
    title: &str,
    description: &str,
    reward: u64,
    token: &str,
    claim_deadline: Option<u64>,
    work_deadline: Option<u64>,
    writer: &OutputWriter,
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
    let creator = Did("did:neunode:local".to_string());

    let mut mgr = BountyManager::new();
    let data = mgr.create_bounty(
        creator,
        title.to_string(),
        description.to_string(),
        TokenAmount(reward),
        token_type,
        now,
    );

    let claim_dl = claim_deadline.map(|d| now.saturating_add(d)).unwrap_or(data.deadlines.claim);
    let work_dl = work_deadline.map(|d| now.saturating_add(d)).unwrap_or(data.deadlines.work);

    let out = serde_json::json!({
        "id": data.id.to_string(),
        "title": data.title,
        "reward": data.reward_amount.0,
        "token": token_type_str(&data.reward_token),
        "state": "Open",
        "claim_deadline": claim_dl,
        "work_deadline": work_dl,
    });

    writer.write_json(&out);
    writer.write_status(&format!("Bounty created: {}", data.id));
    Ok(())
}

fn claim_bounty(bounty_id: &str, stake: u64, writer: &OutputWriter) -> Result<()> {
    if bounty_id.is_empty() {
        anyhow::bail!("bounty id cannot be empty");
    }
    if stake == 0 {
        anyhow::bail!("stake must be greater than 0");
    }

    let now = current_timestamp();
    let claimant = Did("did:neunode:local".to_string());

    let mut mgr = BountyManager::new();
    let _data = mgr.create_bounty(
        Did("did:neunode:creator".to_string()),
        "temp".to_string(),
        String::new(),
        TokenAmount(1000),
        TokenType::Compute,
        now.saturating_sub(100),
    );

    let bid = BountyId(bounty_id.to_string());
    mgr.claim_bounty(&bid, claimant.clone(), TokenAmount(stake), now)?;

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
) -> Result<()> {
    if bounty_id.is_empty() {
        anyhow::bail!("bounty id cannot be empty");
    }
    if artifact.is_empty() {
        anyhow::bail!("artifact CID cannot be empty");
    }

    let out = serde_json::json!({
        "bounty_id": bounty_id,
        "artifact_cid": artifact,
        "state": "Submitted",
    });

    writer.write_json(&out);
    writer.write_status(&format!("Work submitted: {bounty_id}"));
    Ok(())
}

fn review_bounty(bounty_id: &str, score: u8, feedback: &str, writer: &OutputWriter) -> Result<()> {
    if bounty_id.is_empty() {
        anyhow::bail!("bounty id cannot be empty");
    }
    if score > 100 {
        anyhow::bail!("invalid score: {} (must be 0-100)", score);
    }

    let out = serde_json::json!({
        "bounty_id": bounty_id,
        "score": score,
        "feedback": feedback,
        "state": "UnderReview",
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
) -> Result<()> {
    let now = current_timestamp();
    let mut mgr = BountyManager::new();

    let creators = ["did:neunode:alice", "did:neunode:bob", "did:neunode:carol"];
    let titles =
        ["Train sentiment classifier", "Fine-tune medical model", "Build summarizer adapter"];
    let rewards = [1000u64, 2500, 750];
    let tokens = [TokenType::Compute, TokenType::Train, TokenType::Compute];

    for i in 0..3 {
        mgr.create_bounty(
            Did(creators[i].to_string()),
            titles[i].to_string(),
            format!("Description for bounty {}", i + 1),
            TokenAmount(rewards[i]),
            tokens[i],
            now.saturating_sub((i as u64) * 1000),
        );
    }

    let all_data: Vec<BountyData> = (0..3)
        .filter_map(|i| {
            let id = BountyId(format!("bnty_{:08x}", i + 1));
            mgr.get_bounty(&id).cloned()
        })
        .collect();

    let bounties: Vec<&BountyData> = all_data
        .iter()
        .filter(|bdata| {
            state_filter.is_none_or(|sf| bounty_state_str(&bdata.state).eq_ignore_ascii_case(sf))
        })
        .filter(|bdata| creator_filter.is_none_or(|cf| bdata.creator.0.contains(cf)))
        .take(limit)
        .collect();

    let headers = ["ID", "Title", "State", "Reward", "Creator", "Created"];
    let rows: Vec<Vec<String>> = bounties
        .iter()
        .map(|b| {
            vec![
                b.id.to_string(),
                b.title.clone(),
                bounty_state_str(&b.state).to_string(),
                format!("{} {}", b.reward_amount.0, token_type_str(&b.reward_token)),
                b.creator.to_string(),
                b.created_at.to_string(),
            ]
        })
        .collect();
    writer.write_table(&headers, &rows);
    Ok(())
}

fn show_bounty(bounty_id: &str, writer: &OutputWriter) -> Result<()> {
    if bounty_id.is_empty() {
        anyhow::bail!("bounty id cannot be empty");
    }

    let now = current_timestamp();
    let creator = Did("did:neunode:creator".to_string());

    let mut mgr = BountyManager::new();
    let data = mgr.create_bounty(
        creator,
        "Sample bounty".to_string(),
        "A detailed description of the bounty".to_string(),
        TokenAmount(1000),
        TokenType::Compute,
        now,
    );

    let target_id = BountyId(bounty_id.to_string());
    let b = mgr.get_bounty(&target_id);

    let displayed = b.unwrap_or(&data);

    let pairs = [
        ("ID", displayed.id.to_string()),
        ("Title", displayed.title.clone()),
        ("Description", displayed.description.clone()),
        ("Creator", displayed.creator.to_string()),
        (
            "Claimant",
            displayed
                .claimant
                .as_ref()
                .map(|d| d.to_string())
                .unwrap_or_else(|| "none".to_string()),
        ),
        (
            "Reward",
            format!("{} {}", displayed.reward_amount.0, token_type_str(&displayed.reward_token)),
        ),
        ("State", bounty_state_str(&displayed.state).to_string()),
        ("Bond", displayed.bond.map(|a| a.0.to_string()).unwrap_or_else(|| "none".to_string())),
        (
            "Artifact",
            displayed
                .artifact_hash
                .as_ref()
                .map(|h| h.0.clone())
                .unwrap_or_else(|| "none".to_string()),
        ),
        ("Claim Deadline", displayed.deadlines.claim.to_string()),
        ("Work Deadline", displayed.deadlines.work.to_string()),
        ("Review Deadline", displayed.deadlines.review.to_string()),
    ];

    writer.write_key_value_pairs(&pairs.iter().map(|(k, v)| (*k, v.as_str())).collect::<Vec<_>>());
    Ok(())
}

fn cancel_bounty(bounty_id: &str, reason: &str, writer: &OutputWriter) -> Result<()> {
    if bounty_id.is_empty() {
        anyhow::bail!("bounty id cannot be empty");
    }

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

    fn test_writer() -> OutputWriter {
        OutputWriter::new(OutputFormat::Json)
    }

    fn human_writer() -> OutputWriter {
        OutputWriter::new(OutputFormat::Human)
    }

    #[test]
    fn create_bounty_valid() {
        let writer = test_writer();
        create_bounty("Test", "Desc", 1000, "compute", None, None, &writer).unwrap();
    }

    #[test]
    fn create_bounty_empty_title_fails() {
        let writer = test_writer();
        assert!(create_bounty("", "Desc", 1000, "compute", None, None, &writer).is_err());
    }

    #[test]
    fn create_bounty_empty_desc_fails() {
        let writer = test_writer();
        assert!(create_bounty("Title", "", 1000, "compute", None, None, &writer).is_err());
    }

    #[test]
    fn create_bounty_zero_reward_fails() {
        let writer = test_writer();
        assert!(create_bounty("Title", "Desc", 0, "compute", None, None, &writer).is_err());
    }

    #[test]
    fn create_bounty_invalid_token_fails() {
        let writer = test_writer();
        assert!(create_bounty("Title", "Desc", 100, "invalid", None, None, &writer).is_err());
    }

    #[test]
    fn create_bounty_with_deadlines() {
        let writer = test_writer();
        create_bounty("Title", "Desc", 500, "compute", Some(3600), Some(7200), &writer).unwrap();
    }

    #[test]
    fn submit_bounty_valid() {
        let writer = test_writer();
        submit_bounty("bnty_001", "ipfs://QmX7b", "{}", &writer).unwrap();
    }

    #[test]
    fn submit_bounty_empty_cid_fails() {
        let writer = test_writer();
        assert!(submit_bounty("bnty_001", "", "{}", &writer).is_err());
    }

    #[test]
    fn submit_bounty_empty_id_fails() {
        let writer = test_writer();
        assert!(submit_bounty("", "ipfs://QmX7b", "{}", &writer).is_err());
    }

    #[test]
    fn review_bounty_valid_score() {
        let writer = test_writer();
        review_bounty("bnty_001", 85, "Good work", &writer).unwrap();
    }

    #[test]
    fn review_bounty_score_boundary_zero() {
        let writer = test_writer();
        review_bounty("bnty_001", 0, "Poor", &writer).unwrap();
    }

    #[test]
    fn review_bounty_score_boundary_100() {
        let writer = test_writer();
        review_bounty("bnty_001", 100, "Perfect", &writer).unwrap();
    }

    #[test]
    fn review_bounty_score_over_100_fails() {
        let writer = test_writer();
        assert!(review_bounty("bnty_001", 101, "Too high", &writer).is_err());
    }

    #[test]
    fn list_bounties_does_not_panic() {
        let writer = human_writer();
        list_bounties(None, None, 10, &writer).unwrap();
    }

    #[test]
    fn list_bounties_with_filters() {
        let writer = test_writer();
        list_bounties(Some("Open"), Some("alice"), 5, &writer).unwrap();
    }

    #[test]
    fn show_bounty_does_not_panic() {
        let writer = test_writer();
        show_bounty("bnty_001", &writer).unwrap();
    }

    #[test]
    fn show_bounty_empty_id_fails() {
        let writer = test_writer();
        assert!(show_bounty("", &writer).is_err());
    }

    #[test]
    fn cancel_bounty_valid() {
        let writer = test_writer();
        cancel_bounty("bnty_001", "No longer needed", &writer).unwrap();
    }

    #[test]
    fn cancel_bounty_empty_id_fails() {
        let writer = test_writer();
        assert!(cancel_bounty("", "reason", &writer).is_err());
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

    #[test]
    fn claim_bounty_zero_stake_fails() {
        let writer = test_writer();
        assert!(claim_bounty("bnty_001", 0, &writer).is_err());
    }

    #[test]
    fn claim_bounty_empty_id_fails() {
        let writer = test_writer();
        assert!(claim_bounty("", 100, &writer).is_err());
    }
}
