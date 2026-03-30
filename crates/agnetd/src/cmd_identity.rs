use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::cli::{Cli, IdentityCommands};
use crate::config::CliConfig;
use crate::output::OutputWriter;

pub fn execute(cmd: &IdentityCommands, cli: &Cli, config: &mut CliConfig) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        IdentityCommands::Create { name, method, output_dir } => {
            create_identity(name, method, output_dir.as_deref(), &writer)
        }
        IdentityCommands::Show { did } => show_identity(did.as_deref(), &writer, config),
        IdentityCommands::List => list_identities(&writer),
        IdentityCommands::Export { did, output } => {
            export_identity(did.as_deref(), output, &writer, config)
        }
    }
}

fn create_identity(
    name: &str,
    method: &str,
    output_dir: Option<&str>,
    writer: &OutputWriter,
) -> Result<()> {
    if !matches!(method, "key" | "neunode") {
        anyhow::bail!("unsupported DID method: {method}. Use 'key' or 'neunode'.");
    }

    let keyring = neunode_identity::keyring::Keyring::generate();
    let did = keyring.to_did();
    let did_key = keyring.to_did_key();
    let peer_id = neunode_identity::did::did_to_peer_id(&did_key)
        .map(|p| p.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let eth_addr = keyring.ethereum_address();

    let mut capabilities = Vec::new();
    if method == "neunode" {
        capabilities.push("inference".to_string());
        capabilities.push("training".to_string());
    }
    let card =
        neunode_identity::agent_card::AgentCard::new(name, &keyring, capabilities, HashMap::new());
    let signed_card = card.sign(&keyring);
    let doc = neunode_identity::document::DidDocument::from_keyring(&keyring);

    let dir = match output_dir {
        Some(d) => PathBuf::from(d),
        None => dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".neunode")
            .join("identities")
            .join(did.to_string().replace(':', "_")),
    };

    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create identity directory {}", dir.display()))?;

    let (ed_bytes, secp_bytes) = keyring.to_bytes();
    let key_data = serde_json::json!({
        "ed25519_private": bytes_to_hex(&ed_bytes),
        "secp256k1_private": bytes_to_hex(&secp_bytes),
    });
    fs::write(dir.join("keys.json"), serde_json::to_string_pretty(&key_data)?)
        .with_context(|| "failed to write keys.json")?;

    fs::write(dir.join("did_document.json"), doc.to_json()?)
        .with_context(|| "failed to write did_document.json")?;

    fs::write(dir.join("agent_card.json"), serde_json::to_string_pretty(&signed_card)?)
        .with_context(|| "failed to write agent_card.json")?;

    let bundle = keyring.export_public();
    fs::write(dir.join("public_keys.json"), serde_json::to_string_pretty(&bundle)?)
        .with_context(|| "failed to write public_keys.json")?;

    let did_str = did.to_string();
    let did_key_str = did_key.to_string();
    let card_cid = card.to_cid().to_string();
    let dir_str = dir.to_str().unwrap_or("?").to_string();
    let pairs = [
        ("DID", did_str.as_str()),
        ("DID (key)", did_key_str.as_str()),
        ("Peer ID", peer_id.as_str()),
        ("Ethereum", eth_addr.as_str()),
        ("Name", name),
        ("Method", method),
        ("Directory", dir_str.as_str()),
        ("Card CID", card_cid.as_str()),
    ];
    writer.write_key_value_pairs(&pairs);
    writer.write_status(&format!("Identity '{name}' created and stored to {}", dir.display()));
    Ok(())
}

fn show_identity(did: Option<&str>, writer: &OutputWriter, _config: &CliConfig) -> Result<()> {
    let target_did = did.unwrap_or("(active identity not set)");

    let info = serde_json::json!({
        "did": target_did,
        "status": "Phase 1 MVP — identity lookup not yet connected to storage",
    });
    writer.write_json(&info);
    Ok(())
}

fn list_identities(writer: &OutputWriter) -> Result<()> {
    let identities_dir =
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".neunode").join("identities");

    let mut identities: Vec<serde_json::Value> = Vec::new();

    if identities_dir.exists() {
        let entries = fs::read_dir(&identities_dir)
            .with_context(|| format!("failed to read {}", identities_dir.display()))?;
        for entry in entries {
            let entry = entry.with_context(|| "failed to read directory entry")?;
            let path = entry.path();
            if path.is_dir() {
                let doc_path = path.join("did_document.json");
                if doc_path.exists() {
                    let contents = fs::read_to_string(&doc_path)
                        .with_context(|| format!("failed to read {}", doc_path.display()))?;
                    if let Ok(doc) = serde_json::from_str::<serde_json::Value>(&contents) {
                        identities.push(serde_json::json!({
                            "did": doc.get("id").and_then(|v| v.as_str()).unwrap_or("unknown"),
                            "path": path.to_str().unwrap_or("?"),
                        }));
                    }
                }
            }
        }
    }

    if identities.is_empty() {
        writer.write_status(
            "No identities found. Create one with: agnetd identity create --name <name>",
        );
    } else {
        let headers = ["DID", "Path"];
        let rows: Vec<Vec<String>> = identities
            .iter()
            .map(|id| {
                vec![
                    id.get("did").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                    id.get("path").and_then(|v| v.as_str()).unwrap_or("?").to_string(),
                ]
            })
            .collect();
        writer.write_table(&headers, &rows);
    }
    Ok(())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn export_identity(
    did: Option<&str>,
    output: &str,
    writer: &OutputWriter,
    _config: &CliConfig,
) -> Result<()> {
    let target_did = did.unwrap_or("(active)");
    let export_data = serde_json::json!({
        "did": target_did,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "note": "Phase 1 MVP — full export requires identity storage connection",
    });

    let json = serde_json::to_string_pretty(&export_data)?;
    fs::write(output, &json).with_context(|| format!("failed to write export to {output}"))?;

    writer.write_status(&format!("Identity exported to {output}"));
    Ok(())
}
