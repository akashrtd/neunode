use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::cli::GlobalArgs;
use crate::output::OutputWriter;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Agent type presets
// ---------------------------------------------------------------------------

struct AgentPreset {
    name: &'static str,
    description: &'static str,
    capabilities: &'static [&'static str],
}

const PRESETS: &[AgentPreset] = &[
    AgentPreset {
        name: "coding",
        description: "Code generation and review",
        capabilities: &["inference", "bounty"],
    },
    AgentPreset {
        name: "research",
        description: "Data analysis and summarization",
        capabilities: &["inference", "knowledge"],
    },
    AgentPreset {
        name: "inference-provider",
        description: "Serve models to the network",
        capabilities: &["inference", "training"],
    },
    AgentPreset { name: "custom", description: "Configure manually", capabilities: &[] },
];

// ---------------------------------------------------------------------------
// Init command
// ---------------------------------------------------------------------------

pub fn execute(yes: bool, args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(args.output);

    if state.active_did.is_some() {
        writer
            .write_status("Identity already configured. Use 'agnetd identity create' to add more.");
        return Ok(());
    }

    println!(
        "{}  Welcome to Neunode! Let's set up your agent.\n",
        console::style(">").cyan().bold()
    );

    let preset = if yes { &PRESETS[0] } else { select_preset()? };

    let agent_name = if yes { preset.name.to_string() } else { prompt_name(preset.name)? };

    writer.write_status(&format!("Creating {} agent '{}'...", preset.name, agent_name));

    // Generate identity
    let keyring = neunode_identity::keyring::Keyring::generate();
    let did = keyring.to_did();
    let did_key = keyring.to_did_key();
    let peer_id = neunode_identity::did::did_to_peer_id(&did_key)
        .map(|p| p.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let eth_addr = keyring.ethereum_address();

    let capabilities: Vec<String> = preset.capabilities.iter().map(|s| s.to_string()).collect();
    let card = neunode_identity::agent_card::AgentCard::new(
        &agent_name,
        &keyring,
        capabilities,
        HashMap::new(),
    )?;
    let signed_card = card.sign(&keyring);
    let doc = neunode_identity::document::DidDocument::from_keyring(&keyring)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Persist keys
    let dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".neunode")
        .join("identities")
        .join(did.to_string().replace(':', "_"));

    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create identity directory {}", dir.display()))?;

    let (ed_bytes, secp_bytes) = keyring.to_bytes();
    let key_data = serde_json::json!({
        "ed25519_private": bytes_to_hex(&ed_bytes),
        "secp256k1_private": bytes_to_hex(&secp_bytes),
    });
    let key_json = serde_json::to_string_pretty(&key_data)?;
    #[allow(deprecated)]
    let machine_key = neunode_crypto::aead::derive_machine_key();
    let encrypted = neunode_crypto::aead::encrypt(&machine_key, key_json.as_bytes())
        .with_context(|| "failed to encrypt key data")?;
    let keys_path = dir.join("keys.json.enc");
    std::fs::write(&keys_path, &encrypted).with_context(|| "failed to write keys.json.enc")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&keys_path, perms)
            .with_context(|| "failed to set permissions on keys.json.enc")?;
    }

    std::fs::write(dir.join("did_document.json"), doc.to_json()?)
        .with_context(|| "failed to write did_document.json")?;
    std::fs::write(dir.join("agent_card.json"), serde_json::to_string_pretty(&signed_card)?)
        .with_context(|| "failed to write agent_card.json")?;

    let bundle = keyring.export_public();
    std::fs::write(dir.join("public_keys.json"), serde_json::to_string_pretty(&bundle)?)
        .with_context(|| "failed to write public_keys.json")?;

    // Store in DB and set active
    let did_str = did.to_string();
    let store = state.identity_store();
    store.put(&did_str, &doc.to_json()?)?;

    state.config.active_identity = Some(did_str.clone());
    state.config.app_config.active_identity = Some(did_str.clone());
    state.active_keyring = Some(keyring);
    state.active_did = Some(neunode_core::types::Did(did_str.clone()));
    state.save_config()?;

    // Grant seed tokens
    grant_seed_tokens(&did_str, &writer, state)?;

    // Attempt on-chain registration (uses shared logic from cmd_identity)
    let keyring_ref = state.require_keyring()?;
    match crate::cmd_identity::attempt_onchain_registration(keyring_ref, &state.config)? {
        Some(result) => writer.write_status(&format!(
            "DID registered on-chain: tx={}, block={}",
            result.tx_hash, result.block_number
        )),
        None => writer.write_status(
            "On-chain registration skipped (no RPC configured). \
             Run 'agnetd identity register-on-chain' later.",
        ),
    }

    // Print summary
    println!();
    println!("  {} Agent initialized successfully!", console::style("✓").green().bold());
    println!();
    let pairs = [
        ("DID", did_str.as_str()),
        ("Peer ID", peer_id.as_str()),
        ("Ethereum", eth_addr.as_str()),
        ("Type", preset.name),
        ("Directory", dir.to_str().unwrap_or("?")),
    ];
    writer.write_key_value_pairs(&pairs);
    println!();
    writer.write_status(
        "Run 'agnetd serve' to start the dashboard, or 'agnetd mesh start' to join the network.",
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Interactive prompts (simple stdin-based, no external deps)
// ---------------------------------------------------------------------------

fn select_preset() -> Result<&'static AgentPreset> {
    println!("  Select agent type:");
    for (i, p) in PRESETS.iter().enumerate() {
        println!(
            "    {}) {} — {}",
            console::style(i + 1).cyan(),
            console::style(p.name).bold(),
            p.description
        );
    }
    println!();

    loop {
        eprint!("  Enter choice [1-{}]: ", PRESETS.len());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let choice: usize = input.trim().parse().unwrap_or(0);
        if choice >= 1 && choice <= PRESETS.len() {
            return Ok(&PRESETS[choice - 1]);
        }
        eprintln!("  Invalid choice. Try again.");
    }
}

fn prompt_name(default: &str) -> Result<String> {
    eprint!("  Agent name [{}]: ", default);
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let name = input.trim().to_string();
    if name.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(name)
    }
}

// ---------------------------------------------------------------------------
// Seed tokens
// ---------------------------------------------------------------------------

fn grant_seed_tokens(did: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    use neunode_core::constants::token;
    use neunode_storage::token_store::{
        TOKEN_BANDWIDTH, TOKEN_COMPUTE, TOKEN_STORAGE, TOKEN_TRAINING,
    };

    let seeds = [
        ("nCompute", TOKEN_COMPUTE, token::SEED_COMPUTE),
        ("nTrain", TOKEN_TRAINING, token::SEED_TRAINING),
        ("nBandwidth", TOKEN_BANDWIDTH, token::SEED_BANDWIDTH),
        ("nStorage", TOKEN_STORAGE, token::SEED_STORAGE),
    ];

    let store = state.token_store();
    let mut granted = Vec::new();

    for (name, token_byte, amount) in &seeds {
        let mut bal = store.get_balance(did, *token_byte)?;
        if bal.balance == 0 && bal.staked == 0 && *amount > 0 {
            bal.staked = *amount;
            store.set_balance(did, *token_byte, &bal)?;
            granted.push(format!("{name}: {amount} (staked)"));
        }
    }

    if !granted.is_empty() {
        writer.write_status(&format!("Seed tokens granted: {}", granted.join(", ")));
    }

    Ok(())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_count() {
        assert_eq!(PRESETS.len(), 4);
    }

    #[test]
    fn preset_names_unique() {
        let names: std::collections::HashSet<&str> = PRESETS.iter().map(|p| p.name).collect();
        assert_eq!(names.len(), PRESETS.len());
    }

    #[test]
    fn coding_preset_has_capabilities() {
        assert!(!PRESETS[0].capabilities.is_empty());
    }

    #[test]
    fn custom_preset_has_no_capabilities() {
        assert!(PRESETS[3].capabilities.is_empty());
    }
}
