use anyhow::Result;
use neunode_core::types::TokenAmount;
use neunode_reputation::attestation::Attestation;
use neunode_reputation::factors::FactorWeights;
use neunode_reputation::score::{FactorInputs, ReputationGrade, ReputationScore};

use crate::cli::{Cli, ReputationCommands};
use crate::output::OutputWriter;
use crate::state::AppState;

pub fn execute(cmd: &ReputationCommands, cli: &Cli, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        ReputationCommands::Show { agent } => show_reputation(agent.as_deref(), &writer, state),
        ReputationCommands::Attest { to, score, comment } => {
            attest_agent(to, *score, comment.as_deref().unwrap_or(""), &writer, state)
        }
        ReputationCommands::Leaderboard { limit } => show_leaderboard(*limit, &writer, state),
        ReputationCommands::Factors { agent } => {
            show_factors(agent.as_deref().unwrap_or("active"), &writer, state)
        }
    }
}

fn show_reputation(agent: Option<&str>, writer: &OutputWriter, state: &AppState) -> Result<()> {
    let agent_did = match agent {
        Some(d) => d.to_string(),
        None => state.require_did().map(|d| d.0.clone())?,
    };

    let db = state.db();
    let attestations = load_attestations_for(db, &agent_did);

    let avg_score = if attestations.is_empty() {
        0.0
    } else {
        attestations.iter().map(|a| a.score).sum::<f64>() / attestations.len() as f64
    };

    let inputs = FactorInputs {
        staked_amount: TokenAmount(0),
        total_staked: TokenAmount(0),
        attestation_count: attestations.len() as u32,
        avg_attestation_score: avg_score,
        events_per_day: 0.0,
        days_active: 0,
        tasks_completed: 0,
        tasks_failed: 0,
        days_since_creation: 0,
    };
    let score = ReputationScore::compute_default(&inputs);
    let grade = score.grade();

    let out = serde_json::json!({
        "agent": agent_did,
        "score": score.total,
        "grade": format!("{}", grade),
        "attestation_count": attestations.len(),
        "avg_attestation_score": avg_score,
        "factors": {
            "stake": score.stake_factor,
            "attest": score.attest_factor,
            "activity": score.activity_factor,
            "verify": score.verify_factor,
            "tenure": score.tenure_factor,
        },
    });

    writer.write_json(&out);
    Ok(())
}

fn attest_agent(
    to: &str,
    score: u8,
    comment: &str,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    if to.is_empty() {
        anyhow::bail!("target DID cannot be empty");
    }
    if !to.starts_with("did:") {
        anyhow::bail!("target must be a valid DID (did:...)");
    }
    if score > 100 {
        anyhow::bail!("invalid score: {} (must be 0-100)", score);
    }

    let keyring = state.require_keyring()?;
    let attester_did = state.require_did()?;

    let claim = if comment.is_empty() { "general".to_string() } else { comment.to_string() };

    let mut attestation = Attestation::new(
        attester_did.clone(),
        neunode_core::types::Did(to.to_string()),
        claim,
        score as f64,
        neunode_core::types::Hash256("0".to_string()),
    )?;

    let (ed_bytes, _) = keyring.to_bytes();
    let ed_bytes_fixed: [u8; 32] = ed_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid ed25519 key length"))?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&ed_bytes_fixed);
    attestation.sign(&signing_key);

    let db = state.db();
    persist_attestation(db, &attestation)?;

    let out = serde_json::json!({
        "attester": attester_did.0,
        "target": to,
        "score": score,
        "comment": comment,
        "signed": attestation.signature.is_some(),
    });

    writer.write_json(&out);
    writer.write_status(&format!(
        "Attestation submitted: {} -> {to} (score: {}/100)",
        attester_did.0, score
    ));
    Ok(())
}

fn show_leaderboard(limit: usize, writer: &OutputWriter, state: &AppState) -> Result<()> {
    let db = state.db();
    let entries = db.prefix_scan(neunode_storage::cf::CF_REPUTATION, &[])?;

    let mut agent_scores: std::collections::HashMap<String, (f64, usize)> =
        std::collections::HashMap::new();

    for (_, value_bytes) in &entries {
        if let Ok(att) = bincode::deserialize::<Attestation>(value_bytes) {
            let entry = agent_scores.entry(att.target.0.clone()).or_insert((0.0, 0));
            entry.0 += att.score;
            entry.1 += 1;
        }
    }

    let mut ranked: Vec<(String, f64)> =
        agent_scores.into_iter().map(|(did, (sum, count))| (did, sum / count as f64)).collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if ranked.is_empty() {
        writer.write_status("No attestations found — leaderboard is empty");
    } else {
        let headers = ["Rank", "Agent", "Score", "Grade"];
        let rows: Vec<Vec<String>> = ranked
            .iter()
            .take(limit)
            .enumerate()
            .map(|(i, (agent, score))| {
                vec![
                    format!("#{}", i + 1),
                    agent.to_string(),
                    format!("{:.1}", score),
                    format!("{}", ReputationGrade::from_score(*score)),
                ]
            })
            .collect();
        writer.write_table(&headers, &rows);
    }
    Ok(())
}

fn show_factors(agent: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    if agent.is_empty() {
        anyhow::bail!("agent DID cannot be empty");
    }

    let db = state.db();
    let attestations = load_attestations_for(db, agent);
    let avg_score = if attestations.is_empty() {
        0.0
    } else {
        attestations.iter().map(|a| a.score).sum::<f64>() / attestations.len() as f64
    };

    let inputs = FactorInputs {
        staked_amount: TokenAmount(0),
        total_staked: TokenAmount(0),
        attestation_count: attestations.len() as u32,
        avg_attestation_score: avg_score,
        events_per_day: 0.0,
        days_active: 0,
        tasks_completed: 0,
        tasks_failed: 0,
        days_since_creation: 0,
    };

    let weights = FactorWeights::default();
    let score = ReputationScore::compute(&weights, &inputs);

    let headers = ["Factor", "Weight", "Score"];
    let factors = [
        ("Stake", weights.stake, score.stake_factor),
        ("Attestation", weights.attest, score.attest_factor),
        ("Activity", weights.activity, score.activity_factor),
        ("Verification", weights.verify, score.verify_factor),
        ("Tenure", weights.tenure, score.tenure_factor),
    ];
    let rows: Vec<Vec<String>> = factors
        .iter()
        .map(|(name, weight, value)| {
            vec![name.to_string(), format!("{:.0}%", weight), format!("{:.2}", value)]
        })
        .collect();

    writer.write_value("agent", agent);
    writer.write_value("total_score", &format!("{:.2}", score.total));
    writer.write_value("grade", &format!("{}", score.grade()));
    writer.write_table(&headers, &rows);
    Ok(())
}

fn persist_attestation(
    db: &neunode_storage::db::NeunodeDb,
    attestation: &Attestation,
) -> Result<()> {
    let key = format!("att_{}_{}", attestation.attester.0, attestation.timestamp);
    let key_bytes =
        bincode::serialize(&key).map_err(|e| anyhow::anyhow!("key serialization: {e}"))?;
    let value_bytes =
        bincode::serialize(attestation).map_err(|e| anyhow::anyhow!("value serialization: {e}"))?;
    db.put_raw(neunode_storage::cf::CF_REPUTATION, &key_bytes, &value_bytes)?;
    Ok(())
}

fn load_attestations_for(db: &neunode_storage::db::NeunodeDb, did: &str) -> Vec<Attestation> {
    let entries = match db.prefix_scan(neunode_storage::cf::CF_REPUTATION, &[]) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .iter()
        .filter_map(|(_, v)| bincode::deserialize::<Attestation>(v).ok())
        .filter(|a| a.target.0 == did)
        .collect()
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
        let dir =
            std::env::temp_dir().join(format!("agnetd_test_rep_{:?}_{}", std::process::id(), id));
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
    fn show_default_agent() {
        let state = test_state();
        let writer = test_writer();
        show_reputation(None, &writer, &state).unwrap();
    }

    #[test]
    fn show_specific_agent() {
        let state = test_state();
        let writer = test_writer();
        show_reputation(Some("did:neunode:alice"), &writer, &state).unwrap();
    }

    #[test]
    fn attest_valid() {
        let state = test_state();
        let writer = test_writer();
        attest_agent("did:neunode:alice", 85, "Great work", &writer, &state).unwrap();
    }

    #[test]
    fn attest_persists() {
        let state = test_state();
        let writer = test_writer();
        attest_agent("did:neunode:target", 90, "Good", &writer, &state).unwrap();
        let attestations = load_attestations_for(state.db(), "did:neunode:target");
        assert_eq!(attestations.len(), 1);
        assert_eq!(attestations[0].score, 90.0);
    }

    #[test]
    fn attest_score_over_100_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(attest_agent("did:neunode:alice", 101, "Too high", &writer, &state).is_err());
    }

    #[test]
    fn attest_empty_target_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(attest_agent("", 80, "comment", &writer, &state).is_err());
    }

    #[test]
    fn attest_invalid_did_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(attest_agent("not_a_did", 80, "comment", &writer, &state).is_err());
    }

    #[test]
    fn leaderboard_empty() {
        let state = test_state();
        let writer = human_writer();
        show_leaderboard(5, &writer, &state).unwrap();
    }

    #[test]
    fn leaderboard_after_attest() {
        let state = test_state();
        let writer = test_writer();
        attest_agent("did:neunode:alice", 90, "Good", &writer, &state).unwrap();
        attest_agent("did:neunode:bob", 70, "OK", &writer, &state).unwrap();
        let writer2 = human_writer();
        show_leaderboard(10, &writer2, &state).unwrap();
    }

    #[test]
    fn factors_valid() {
        let state = test_state();
        let writer = test_writer();
        show_factors("did:neunode:alice", &writer, &state).unwrap();
    }

    #[test]
    fn factors_empty_agent_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(show_factors("", &writer, &state).is_err());
    }
}
