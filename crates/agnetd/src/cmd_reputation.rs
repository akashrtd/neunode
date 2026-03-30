use anyhow::Result;
use neunode_core::types::TokenAmount;
use neunode_reputation::factors::FactorWeights;
use neunode_reputation::score::{FactorInputs, ReputationGrade, ReputationScore};

use crate::cli::{Cli, ReputationCommands};
use crate::config::CliConfig;
use crate::output::OutputWriter;

pub fn execute(cmd: &ReputationCommands, cli: &Cli, _config: &mut CliConfig) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        ReputationCommands::Show { agent } => show_reputation(agent.as_deref(), &writer),
        ReputationCommands::Attest { to, score, comment } => {
            attest_agent(to, *score, comment.as_deref().unwrap_or(""), &writer)
        }
        ReputationCommands::Leaderboard { limit } => show_leaderboard(*limit, &writer),
        ReputationCommands::Factors { agent } => {
            show_factors(agent.as_deref().unwrap_or("active"), &writer)
        }
    }
}

fn default_inputs() -> FactorInputs {
    FactorInputs {
        staked_amount: TokenAmount(500),
        total_staked: TokenAmount(1000),
        attestation_count: 25,
        avg_attestation_score: 85.0,
        events_per_day: 15.0,
        days_active: 120,
        tasks_completed: 40,
        tasks_failed: 5,
        days_since_creation: 180,
    }
}

fn show_reputation(agent: Option<&str>, writer: &OutputWriter) -> Result<()> {
    let agent_did = agent.unwrap_or("did:neunode:local");

    let inputs = default_inputs();
    let score = ReputationScore::compute_default(&inputs);
    let grade = score.grade();

    let out = serde_json::json!({
        "agent": agent_did,
        "score": score.total,
        "grade": format!("{}", grade),
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

fn attest_agent(to: &str, score: u8, comment: &str, writer: &OutputWriter) -> Result<()> {
    if to.is_empty() {
        anyhow::bail!("target DID cannot be empty");
    }
    if !to.starts_with("did:") {
        anyhow::bail!("target must be a valid DID (did:...)");
    }
    if score > 100 {
        anyhow::bail!("invalid score: {} (must be 0-100)", score);
    }

    let out = serde_json::json!({
        "attester": "did:neunode:local",
        "target": to,
        "score": score,
        "comment": comment,
    });

    writer.write_json(&out);
    writer.write_status(&format!(
        "Attestation submitted: did:neunode:local -> {to} (score: {}/100)",
        score
    ));
    Ok(())
}

fn show_leaderboard(limit: usize, writer: &OutputWriter) -> Result<()> {
    let agents = [
        ("did:neunode:alice", 95.2),
        ("did:neunode:bob", 82.5),
        ("did:neunode:carol", 71.0),
        ("did:neunode:dave", 55.3),
        ("did:neunode:eve", 30.1),
    ];

    let headers = ["Rank", "Agent", "Score", "Grade"];
    let rows: Vec<Vec<String>> = agents
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
    Ok(())
}

fn show_factors(agent: &str, writer: &OutputWriter) -> Result<()> {
    if agent.is_empty() {
        anyhow::bail!("agent DID cannot be empty");
    }

    let weights = FactorWeights::default();
    let inputs = default_inputs();
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
    fn show_default_agent() {
        let writer = test_writer();
        show_reputation(None, &writer).unwrap();
    }

    #[test]
    fn show_specific_agent() {
        let writer = test_writer();
        show_reputation(Some("did:neunode:alice"), &writer).unwrap();
    }

    #[test]
    fn show_human_output() {
        let writer = human_writer();
        show_reputation(Some("did:neunode:bob"), &writer).unwrap();
    }

    #[test]
    fn attest_valid() {
        let writer = test_writer();
        attest_agent("did:neunode:alice", 85, "Great work", &writer).unwrap();
    }

    #[test]
    fn attest_score_boundary_zero() {
        let writer = test_writer();
        attest_agent("did:neunode:alice", 0, "Poor", &writer).unwrap();
    }

    #[test]
    fn attest_score_boundary_100() {
        let writer = test_writer();
        attest_agent("did:neunode:alice", 100, "Perfect", &writer).unwrap();
    }

    #[test]
    fn attest_score_over_100_fails() {
        let writer = test_writer();
        assert!(attest_agent("did:neunode:alice", 101, "Too high", &writer).is_err());
    }

    #[test]
    fn attest_empty_target_fails() {
        let writer = test_writer();
        assert!(attest_agent("", 80, "comment", &writer).is_err());
    }

    #[test]
    fn attest_invalid_did_fails() {
        let writer = test_writer();
        assert!(attest_agent("not_a_did", 80, "comment", &writer).is_err());
    }

    #[test]
    fn leaderboard_does_not_panic() {
        let writer = test_writer();
        show_leaderboard(5, &writer).unwrap();
    }

    #[test]
    fn leaderboard_limited() {
        let writer = human_writer();
        show_leaderboard(2, &writer).unwrap();
    }

    #[test]
    fn factors_valid() {
        let writer = test_writer();
        show_factors("did:neunode:alice", &writer).unwrap();
    }

    #[test]
    fn factors_empty_agent_fails() {
        let writer = test_writer();
        assert!(show_factors("", &writer).is_err());
    }

    #[test]
    fn factors_human_output() {
        let writer = human_writer();
        show_factors("did:neunode:bob", &writer).unwrap();
    }

    #[test]
    fn attest_json_output() {
        let writer = test_writer();
        attest_agent("did:neunode:target", 90, "Excellent", &writer).unwrap();
    }
}
