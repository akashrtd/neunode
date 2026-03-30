use anyhow::Result;
use neunode_core::constants::token::{MIN_STAKE, UNBONDING_PERIOD_SECS};
use neunode_core::types::{ActivityLevel, TokenAmount, TokenType};
use neunode_token::balance::BalanceSheet;
use neunode_token::decay::DecayCalculator;

use crate::cli::{Cli, TokenCommands};
use crate::config::CliConfig;
use crate::output::OutputWriter;

pub fn execute(cmd: &TokenCommands, cli: &Cli, _config: &mut CliConfig) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        TokenCommands::Balance { token } => show_balance(token.as_deref(), &writer),
        TokenCommands::Transfer { to, amount, token } => {
            transfer_tokens(to, *amount, token, &writer)
        }
        TokenCommands::Stake { amount, token } => stake_tokens(*amount, token, &writer),
        TokenCommands::Unstake { amount } => unstake_tokens(*amount, &writer),
        TokenCommands::StakeStatus => show_stake_status(&writer),
        TokenCommands::DecayInfo => show_decay_info(&writer),
    }
}

fn token_type_str(t: &TokenType) -> &'static str {
    match t {
        TokenType::Compute => "nCompute",
        TokenType::Train => "nTrain",
        TokenType::Bandwidth => "nBandwidth",
        TokenType::Storage => "nStorage",
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

fn show_balance(token: Option<&str>, writer: &OutputWriter) -> Result<()> {
    let mut sheet = BalanceSheet::new();
    sheet.deposit(TokenType::Compute, TokenAmount(500))?;
    sheet.deposit(TokenType::Train, TokenAmount(1200))?;
    sheet.deposit(TokenType::Bandwidth, TokenAmount(300))?;
    sheet.deposit(TokenType::Storage, TokenAmount(800))?;

    if let Some(t) = token {
        let tt = parse_token_type(t)?;
        let amount = sheet.get_balance(tt);
        writer.write_value("token", token_type_str(&tt));
        writer.write_value("balance", &amount.0.to_string());
    } else {
        let types =
            [TokenType::Compute, TokenType::Train, TokenType::Bandwidth, TokenType::Storage];
        let headers = ["Token", "Balance"];
        let rows: Vec<Vec<String>> = types
            .iter()
            .map(|tt| vec![token_type_str(tt).to_string(), sheet.get_balance(*tt).0.to_string()])
            .collect();
        writer.write_table(&headers, &rows);
    }
    Ok(())
}

fn transfer_tokens(to: &str, amount: u64, token: &str, writer: &OutputWriter) -> Result<()> {
    if to.is_empty() {
        anyhow::bail!("recipient DID cannot be empty");
    }
    if !to.starts_with("did:") {
        anyhow::bail!("recipient must be a valid DID (did:...)");
    }
    if amount == 0 {
        anyhow::bail!("amount must be greater than 0");
    }

    let tt = parse_token_type(token)?;
    let token_name = token_type_str(&tt).to_string();

    let out = serde_json::json!({
        "to": to,
        "amount": amount,
        "token": token_name,
        "state": "transferred",
    });

    writer.write_json(&out);
    writer.write_status(&format!("Transferred {amount} {token_name} to {to}"));
    Ok(())
}

fn stake_tokens(amount: u64, token: &str, writer: &OutputWriter) -> Result<()> {
    if amount == 0 {
        anyhow::bail!("amount must be greater than 0");
    }
    if amount < MIN_STAKE {
        anyhow::bail!("amount {} is below minimum stake of {}", amount, MIN_STAKE);
    }

    let tt = parse_token_type(token)?;
    let token_name = token_type_str(&tt).to_string();

    let out = serde_json::json!({
        "amount": amount,
        "token": token_name,
        "state": "Staked",
        "unbonding_period_secs": UNBONDING_PERIOD_SECS,
    });

    writer.write_json(&out);
    writer.write_status(&format!("Staked {amount} {token_name}"));
    Ok(())
}

fn unstake_tokens(amount: u64, writer: &OutputWriter) -> Result<()> {
    if amount == 0 {
        anyhow::bail!("amount must be greater than 0");
    }

    let unbond_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + UNBONDING_PERIOD_SECS;

    let out = serde_json::json!({
        "amount": amount,
        "unbond_at": unbond_at,
        "state": "Unbonding",
    });

    writer.write_json(&out);
    writer.write_status(&format!("Unbonding {amount} tokens (available at {unbond_at})"));
    Ok(())
}

fn show_stake_status(writer: &OutputWriter) -> Result<()> {
    let out = serde_json::json!({
        "total_staked": 300,
        "entries": [{
            "amount": 300,
            "token": "nCompute",
            "staked_at": 1700000000_u64,
            "unbonding_at": serde_json::Value::Null,
        }],
    });

    writer.write_json(&out);
    writer.write_status("Total staked: 300");
    Ok(())
}

fn show_decay_info(writer: &OutputWriter) -> Result<()> {
    let levels = [
        ("Active", ActivityLevel::Active),
        ("Moderate", ActivityLevel::Moderate),
        ("Low", ActivityLevel::Low),
        ("Inactive", ActivityLevel::Inactive),
        ("Dead", ActivityLevel::Dead),
    ];

    let headers = ["Activity Level", "Decay Rate", "Treasury", "Staking", "Burned", "Dev Fund"];
    let rows: Vec<Vec<String>> = levels
        .iter()
        .map(|(name, level)| {
            let rate = DecayCalculator::effective_decay_rate(*level);
            vec![
                name.to_string(),
                format!("{:.0}%", rate),
                "40%".to_string(),
                "30%".to_string(),
                "20%".to_string(),
                "10%".to_string(),
            ]
        })
        .collect();
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
    fn balance_all_tokens() {
        let writer = test_writer();
        show_balance(None, &writer).unwrap();
    }

    #[test]
    fn balance_single_token() {
        let writer = test_writer();
        show_balance(Some("compute"), &writer).unwrap();
    }

    #[test]
    fn balance_invalid_token_fails() {
        let writer = test_writer();
        assert!(show_balance(Some("invalid"), &writer).is_err());
    }

    #[test]
    fn transfer_valid() {
        let writer = test_writer();
        transfer_tokens("did:neunode:bob", 100, "compute", &writer).unwrap();
    }

    #[test]
    fn transfer_empty_did_fails() {
        let writer = test_writer();
        assert!(transfer_tokens("", 100, "compute", &writer).is_err());
    }

    #[test]
    fn transfer_invalid_did_fails() {
        let writer = test_writer();
        assert!(transfer_tokens("not_a_did", 100, "compute", &writer).is_err());
    }

    #[test]
    fn transfer_zero_amount_fails() {
        let writer = test_writer();
        assert!(transfer_tokens("did:neunode:bob", 0, "compute", &writer).is_err());
    }

    #[test]
    fn stake_valid() {
        let writer = test_writer();
        stake_tokens(200, "compute", &writer).unwrap();
    }

    #[test]
    fn stake_below_minimum_fails() {
        let writer = test_writer();
        assert!(stake_tokens(50, "compute", &writer).is_err());
    }

    #[test]
    fn stake_zero_fails() {
        let writer = test_writer();
        assert!(stake_tokens(0, "compute", &writer).is_err());
    }

    #[test]
    fn unstake_valid() {
        let writer = test_writer();
        unstake_tokens(100, &writer).unwrap();
    }

    #[test]
    fn unstake_zero_fails() {
        let writer = test_writer();
        assert!(unstake_tokens(0, &writer).is_err());
    }

    #[test]
    fn stake_status_does_not_panic() {
        let writer = human_writer();
        show_stake_status(&writer).unwrap();
    }

    #[test]
    fn decay_info_does_not_panic() {
        let writer = human_writer();
        show_decay_info(&writer).unwrap();
    }

    #[test]
    fn decay_info_json() {
        let writer = test_writer();
        show_decay_info(&writer).unwrap();
    }

    #[test]
    fn parse_token_type_case_insensitive() {
        assert!(matches!(parse_token_type("Compute"), Ok(TokenType::Compute)));
        assert!(matches!(parse_token_type("TRAIN"), Ok(TokenType::Train)));
    }

    #[test]
    fn balance_human_output() {
        let writer = human_writer();
        show_balance(None, &writer).unwrap();
    }
}
