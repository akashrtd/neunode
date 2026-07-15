use anyhow::Result;
use neunode_core::constants::token::MIN_STAKE;
use neunode_core::types::{ActivityLevel, TokenType};
use neunode_storage::token_store::{TOKEN_BANDWIDTH, TOKEN_COMPUTE, TOKEN_STORAGE, TOKEN_TRAINING};
use neunode_storage::unbonding_store::UnbondingStore;
use neunode_token::decay::DecayCalculator;

use crate::cli::{GlobalArgs, OutputFormat, TokenCommands};
use crate::output::OutputWriter;
use crate::state::AppState;
use crate::token_wire::{TokenBalanceWire, TokenBalancesWire};
use crate::util::{parse_token_type, token_type_display};

pub fn execute(cmd: &TokenCommands, args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(args.output);
    match cmd {
        TokenCommands::Balance { token } => show_balance(token.as_deref(), &writer, state),
        TokenCommands::Transfer { to, amount, token } => {
            transfer_tokens(to, *amount, token, &writer, state)
        }
        TokenCommands::Stake { amount, token } => stake_tokens(*amount, token, &writer, state),
        TokenCommands::Unstake { amount } => unstake_tokens(*amount, &writer, state),
        TokenCommands::ClaimUnbonded => claim_unbonded(&writer, state),
        TokenCommands::StakeStatus => show_stake_status(&writer, state),
        TokenCommands::DecayInfo => show_decay_info(&writer),
        TokenCommands::Seed { agent } => seed_tokens(agent.as_deref(), &writer, state),
    }
}

/// Map TokenType enum to the u8 byte used in TokenStore keys.
fn token_type_to_u8(t: &TokenType) -> u8 {
    match t {
        TokenType::Compute => TOKEN_COMPUTE,
        TokenType::Train => TOKEN_TRAINING,
        TokenType::Bandwidth => TOKEN_BANDWIDTH,
        TokenType::Storage => TOKEN_STORAGE,
    }
}

fn show_balance(token: Option<&str>, writer: &OutputWriter, state: &AppState) -> Result<()> {
    let did = state.require_did()?;
    let store = state.token_store();

    if let Some(t) = token {
        let tt = parse_token_type(t)?;
        let bal = store.get_balance(&did.0, token_type_to_u8(&tt))?;
        let response = TokenBalanceWire::new(token_type_display(&tt), bal.balance, bal.staked);
        match writer.format() {
            OutputFormat::Human => {
                writer.write_value("token", &response.token);
                writer.write_value("balance", &response.balance);
                writer.write_value("staked", &response.staked);
            }
            OutputFormat::Json | OutputFormat::JsonCompact | OutputFormat::Ndjson => {
                writer.write_json(&response);
            }
        }
    } else {
        let types =
            [TokenType::Compute, TokenType::Train, TokenType::Bandwidth, TokenType::Storage];
        let balances: Vec<TokenBalanceWire> = types
            .iter()
            .map(|tt| {
                let bal = store.get_balance(&did.0, token_type_to_u8(tt)).unwrap_or_default();
                TokenBalanceWire::new(token_type_display(tt), bal.balance, bal.staked)
            })
            .collect();
        match writer.format() {
            OutputFormat::Human => {
                let rows = balances
                    .iter()
                    .map(|entry| {
                        vec![entry.token.clone(), entry.balance.clone(), entry.staked.clone()]
                    })
                    .collect::<Vec<_>>();
                writer.write_table(&["Token", "Balance", "Staked"], &rows);
            }
            OutputFormat::Json | OutputFormat::JsonCompact | OutputFormat::Ndjson => {
                writer.write_json(&TokenBalancesWire { balances });
            }
        }
    }
    Ok(())
}

fn transfer_tokens(
    to: &str,
    amount: u64,
    token: &str,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    if to.is_empty() {
        anyhow::bail!("recipient DID cannot be empty");
    }
    if !to.starts_with("did:") {
        anyhow::bail!("recipient must be a valid DID (did:...)");
    }
    if amount == 0 {
        anyhow::bail!("amount must be greater than 0");
    }

    let did = state.require_did()?;
    let tt = parse_token_type(token)?;
    let token_name = token_type_display(&tt).to_string();
    let token_byte = token_type_to_u8(&tt);

    let store = state.token_store();
    store.transfer(&did.0, to, token_byte, amount as u128)?;

    let out = serde_json::json!({
        "from": did.0,
        "to": to,
        "amount": amount,
        "token": token_name,
        "state": "transferred",
    });

    writer.write_json(&out);
    writer.write_status(&format!("Transferred {amount} {token_name} to {to}"));
    Ok(())
}

fn stake_tokens(amount: u64, token: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    if amount == 0 {
        anyhow::bail!("amount must be greater than 0");
    }
    if amount < MIN_STAKE {
        anyhow::bail!("amount {} is below minimum stake of {}", amount, MIN_STAKE);
    }

    let did = state.require_did()?;
    let tt = parse_token_type(token)?;
    let token_name = token_type_display(&tt).to_string();
    let token_byte = token_type_to_u8(&tt);

    let store = state.token_store();
    store.stake(&did.0, token_byte, amount as u128)?;

    let unbonding_period_secs = state.config.app_config.tokens.unbonding_period_secs;
    let out = serde_json::json!({
        "amount": amount,
        "token": token_name,
        "state": "Staked",
        "unbonding_period_secs": unbonding_period_secs,
    });

    writer.write_json(&out);
    writer.write_status(&format!("Staked {amount} {token_name}"));
    Ok(())
}

fn unstake_tokens(amount: u64, writer: &OutputWriter, state: &AppState) -> Result<()> {
    if amount == 0 {
        anyhow::bail!("amount must be greater than 0");
    }

    let did = state.require_did()?;
    let token_types = [TOKEN_COMPUTE, TOKEN_TRAINING, TOKEN_BANDWIDTH, TOKEN_STORAGE];
    let now = current_timestamp();
    let entry = match UnbondingStore::new(state.db()).begin(
        &did.0,
        &token_types,
        amount as u128,
        now,
        state.config.app_config.tokens.unbonding_period_secs,
    ) {
        Ok(entry) => entry,
        Err(neunode_storage::error::StorageError::InsufficientStakedBalance { .. }) => {
            anyhow::bail!("no staked tokens found with sufficient balance to unstake {amount}")
        }
        Err(error) => return Err(error.into()),
    };
    let tt = match entry.token_type {
        TOKEN_COMPUTE => TokenType::Compute,
        TOKEN_TRAINING => TokenType::Train,
        TOKEN_BANDWIDTH => TokenType::Bandwidth,
        TOKEN_STORAGE => TokenType::Storage,
        _ => unreachable!("known token type selected"),
    };

    let token_name = token_type_display(&tt).to_string();
    let out = serde_json::json!({
        "id": entry.id,
        "amount": amount,
        "token": token_name,
        "unbond_at": entry.unlock_at,
        "state": "Unbonding",
    });

    writer.write_json(&out);
    writer.write_status(&format!(
        "Unbonding {amount} {token_name} (claimable at {})",
        entry.unlock_at
    ));
    Ok(())
}

fn claim_unbonded(writer: &OutputWriter, state: &AppState) -> Result<()> {
    let did = state.require_did()?;
    let claimed = UnbondingStore::new(state.db()).claim_matured(&did.0, current_timestamp())?;
    let out = serde_json::json!({
        "claimed_amount": claimed.total,
        "claimed_positions": claimed.entries.len(),
        "state": if claimed.entries.is_empty() { "NothingMatured" } else { "Claimed" },
    });
    writer.write_json(&out);
    writer.write_status(&format!(
        "Claimed {} tokens from {} matured position(s)",
        claimed.total,
        claimed.entries.len()
    ));
    Ok(())
}

fn show_stake_status(writer: &OutputWriter, state: &AppState) -> Result<()> {
    let did = state.require_did()?;
    let store = state.token_store();

    let token_types = [
        (TokenType::Compute, TOKEN_COMPUTE),
        (TokenType::Train, TOKEN_TRAINING),
        (TokenType::Bandwidth, TOKEN_BANDWIDTH),
        (TokenType::Storage, TOKEN_STORAGE),
    ];

    let mut total_staked: u128 = 0;
    let mut entries = Vec::new();
    let pending = UnbondingStore::new(state.db()).list(&did.0)?;

    for (tt, byte) in &token_types {
        let bal = store.get_balance(&did.0, *byte)?;
        if bal.staked > 0 {
            total_staked += bal.staked;
            entries.push(serde_json::json!({
                "amount": bal.staked,
                "token": token_type_display(tt),
                "available": bal.balance,
            }));
        }
    }

    if entries.is_empty() && pending.is_empty() {
        writer.write_status("No tokens staked");
    } else {
        let out = serde_json::json!({
            "total_staked": total_staked,
            "entries": entries,
            "unbonding": pending,
        });
        writer.write_json(&out);
        writer.write_status(&format!("Total staked: {total_staked}"));
    }
    Ok(())
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
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
                format!("{rate:.0}%"),
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

fn seed_tokens(agent: Option<&str>, writer: &OutputWriter, state: &AppState) -> Result<()> {
    let did = match agent {
        Some(d) => d.to_string(),
        None => state.require_did()?.0.clone(),
    };

    use neunode_core::constants::token;
    let seeds = [
        ("nCompute", TOKEN_COMPUTE, token::SEED_COMPUTE),
        ("nTrain", TOKEN_TRAINING, token::SEED_TRAINING),
        ("nBandwidth", TOKEN_BANDWIDTH, token::SEED_BANDWIDTH),
        ("nStorage", TOKEN_STORAGE, token::SEED_STORAGE),
    ];

    let store = state.token_store();
    let mut total_seed: u128 = 0;
    let mut granted = Vec::new();

    for (name, token_byte, amount) in &seeds {
        let mut bal = store.get_balance(&did, *token_byte)?;
        // Only grant if not already seeded (staked == 0 and balance == 0)
        if bal.balance == 0 && bal.staked == 0 && *amount > 0 {
            bal.staked = *amount;
            store.set_balance(&did, *token_byte, &bal)?;
            total_seed += amount;
            granted.push(format!("{name}: {amount} (staked)"));
        }
    }

    if granted.is_empty() {
        writer.write_status(&format!("{did} already has tokens — seed skipped"));
    } else {
        let headers = ["Token", "Amount", "Type"];
        let rows: Vec<Vec<String>> = seeds
            .iter()
            .filter(|(_, _, amt)| *amt > 0)
            .map(|(name, _, amt)| vec![name.to_string(), amt.to_string(), "staked".to_string()])
            .collect();
        writer.write_table(&headers, &rows);
        writer.write_status(&format!(
            "Seeded {total_seed} tokens to {did} (staked only, not spendable)"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CliConfig;
    use crate::testutil::{human_writer, json_writer as test_writer, test_state};
    use neunode_storage::token_store::TokenBalance;
    use std::sync::Arc;

    fn seed_balance(state: &AppState, token_byte: u8, balance: u128, staked: u128) {
        let did = state.active_did.as_ref().unwrap();
        let store = state.token_store();
        store
            .set_balance(&did.0, token_byte, &TokenBalance { balance, staked, last_decay_epoch: 0 })
            .unwrap();
    }

    #[test]
    fn balance_all_tokens() {
        let state = test_state();
        seed_balance(&state, TOKEN_COMPUTE, 500, 0);
        seed_balance(&state, TOKEN_TRAINING, 1200, 0);
        let writer = test_writer();
        show_balance(None, &writer, &state).unwrap();
    }

    #[test]
    fn balance_single_token() {
        let state = test_state();
        seed_balance(&state, TOKEN_COMPUTE, 500, 0);
        let writer = test_writer();
        show_balance(Some("compute"), &writer, &state).unwrap();
    }

    #[test]
    fn balance_no_identity_fails() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static TEST_ID: AtomicU64 = AtomicU64::new(0);
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "agnetd_test_token_noid_{:?}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = neunode_storage::db::NeunodeDb::open(&dir).unwrap();

        let state = AppState {
            db: Arc::new(db),
            config: CliConfig::load(None).unwrap(),
            active_keyring: None,
            active_did: None,
            mesh_handle: None,
        };
        let writer = test_writer();
        assert!(show_balance(None, &writer, &state).is_err());
    }

    #[test]
    fn balance_invalid_token_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(show_balance(Some("invalid"), &writer, &state).is_err());
    }

    #[test]
    fn transfer_valid() {
        let state = test_state();
        seed_balance(&state, TOKEN_COMPUTE, 1000, 0);
        let writer = test_writer();
        transfer_tokens("did:neunode:bob", 100, "compute", &writer, &state).unwrap();
        // Verify balance decreased
        let store = state.token_store();
        let did = state.active_did.as_ref().unwrap();
        let bal = store.get_balance(&did.0, TOKEN_COMPUTE).unwrap();
        assert_eq!(bal.balance, 900);
    }

    #[test]
    fn transfer_insufficient_fails() {
        let state = test_state();
        seed_balance(&state, TOKEN_COMPUTE, 50, 0);
        let writer = test_writer();
        assert!(transfer_tokens("did:neunode:bob", 100, "compute", &writer, &state).is_err());
    }

    #[test]
    fn transfer_empty_did_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(transfer_tokens("", 100, "compute", &writer, &state).is_err());
    }

    #[test]
    fn transfer_invalid_did_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(transfer_tokens("not_a_did", 100, "compute", &writer, &state).is_err());
    }

    #[test]
    fn transfer_zero_amount_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(transfer_tokens("did:neunode:bob", 0, "compute", &writer, &state).is_err());
    }

    #[test]
    fn stake_valid() {
        let state = test_state();
        seed_balance(&state, TOKEN_COMPUTE, 500, 0);
        let writer = test_writer();
        stake_tokens(200, "compute", &writer, &state).unwrap();
        // Verify balance moved to staked
        let store = state.token_store();
        let did = state.active_did.as_ref().unwrap();
        let bal = store.get_balance(&did.0, TOKEN_COMPUTE).unwrap();
        assert_eq!(bal.balance, 300);
        assert_eq!(bal.staked, 200);
    }

    #[test]
    fn stake_insufficient_balance_fails() {
        let state = test_state();
        seed_balance(&state, TOKEN_COMPUTE, 100, 0);
        let writer = test_writer();
        assert!(stake_tokens(200, "compute", &writer, &state).is_err());
    }

    #[test]
    fn stake_below_minimum_fails() {
        let state = test_state();
        seed_balance(&state, TOKEN_COMPUTE, 500, 0);
        let writer = test_writer();
        assert!(stake_tokens(50, "compute", &writer, &state).is_err());
    }

    #[test]
    fn stake_zero_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(stake_tokens(0, "compute", &writer, &state).is_err());
    }

    #[test]
    fn unstake_valid() {
        let state = test_state();
        seed_balance(&state, TOKEN_COMPUTE, 300, 200);
        let writer = test_writer();
        unstake_tokens(100, &writer, &state).unwrap();
        // Unstaking decreases stake but remains non-spendable until maturity.
        let store = state.token_store();
        let did = state.active_did.as_ref().unwrap();
        let bal = store.get_balance(&did.0, TOKEN_COMPUTE).unwrap();
        assert_eq!(bal.balance, 300);
        assert_eq!(bal.staked, 100);
        let pending = UnbondingStore::new(state.db()).list(&did.0).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].amount, 100);
    }

    #[test]
    fn unstake_zero_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(unstake_tokens(0, &writer, &state).is_err());
    }

    #[test]
    fn claim_unbonded_before_maturity_keeps_balance_locked() {
        let state = test_state();
        seed_balance(&state, TOKEN_COMPUTE, 300, 200);
        unstake_tokens(100, &test_writer(), &state).unwrap();

        claim_unbonded(&test_writer(), &state).unwrap();

        let did = state.active_did.as_ref().unwrap();
        let balance = state.token_store().get_balance(&did.0, TOKEN_COMPUTE).unwrap();
        assert_eq!(balance.balance, 300);
        assert_eq!(UnbondingStore::new(state.db()).list(&did.0).unwrap().len(), 1);
    }

    #[test]
    fn stake_status_with_stake() {
        let state = test_state();
        seed_balance(&state, TOKEN_COMPUTE, 200, 300);
        let writer = human_writer();
        show_stake_status(&writer, &state).unwrap();
    }

    #[test]
    fn stake_status_empty() {
        let state = test_state();
        let writer = human_writer();
        show_stake_status(&writer, &state).unwrap();
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
    fn balance_human_output() {
        let state = test_state();
        seed_balance(&state, TOKEN_COMPUTE, 500, 0);
        let writer = human_writer();
        show_balance(None, &writer, &state).unwrap();
    }
}
