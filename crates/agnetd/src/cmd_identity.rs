use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::cli::{GlobalArgs, IdentityCommands};
use crate::output::OutputWriter;
use crate::state::AppState;

pub fn execute(cmd: &IdentityCommands, args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(args.output);
    match cmd {
        IdentityCommands::Create { name, method, output_dir } => {
            create_identity(name, method, output_dir.as_deref(), &writer, state)
        }
        IdentityCommands::Show { did } => show_identity(did.as_deref(), &writer, state),
        IdentityCommands::List => list_identities(&writer, state),
        IdentityCommands::Export { did, file } => {
            export_identity(did.as_deref(), file, &writer, state)
        }
    }
}

fn create_identity(
    name: &str,
    method: &str,
    output_dir: Option<&str>,
    writer: &OutputWriter,
    state: &mut AppState,
) -> Result<()> {
    if !matches!(method, "key" | "neunode") {
        anyhow::bail!("unsupported DID method: {method}. Use 'key' or 'neunode'.");
    }

    let keyring = neunode_identity::keyring::Keyring::generate();
    let did = keyring.to_did().map_err(|e| anyhow::anyhow!("{e}"))?;
    let did_key = keyring.to_did_key();
    let peer_id = neunode_identity::did::did_to_peer_id(&did_key)
        .map(|p| p.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let eth_addr = keyring.ethereum_address().map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut capabilities = Vec::new();
    if method == "neunode" {
        capabilities.push("inference".to_string());
        capabilities.push("training".to_string());
    }
    let card =
        neunode_identity::agent_card::AgentCard::new(name, &keyring, capabilities, HashMap::new())?;
    let signed_card = card.sign(&keyring);
    let doc = neunode_identity::document::DidDocument::from_keyring(&keyring)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

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
    let key_json = serde_json::to_string_pretty(&key_data)?;
    let machine_key = neunode_crypto::aead::derive_machine_key();
    let encrypted = neunode_crypto::aead::encrypt(&machine_key, key_json.as_bytes())
        .with_context(|| "failed to encrypt key data")?;
    let keys_path = dir.join("keys.json.enc");
    fs::write(&keys_path, &encrypted).with_context(|| "failed to write keys.json.enc")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        fs::set_permissions(&keys_path, perms)
            .with_context(|| "failed to set permissions on keys.json.enc")?;
    }

    fs::write(dir.join("did_document.json"), doc.to_json()?)
        .with_context(|| "failed to write did_document.json")?;

    fs::write(dir.join("agent_card.json"), serde_json::to_string_pretty(&signed_card)?)
        .with_context(|| "failed to write agent_card.json")?;

    let bundle = keyring.export_public().map_err(|e| anyhow::anyhow!("{e}"))?;
    fs::write(dir.join("public_keys.json"), serde_json::to_string_pretty(&bundle)?)
        .with_context(|| "failed to write public_keys.json")?;

    let did_str = did.to_string();
    let store = state.identity_store();
    let doc_json = doc.to_json()?;
    store.put(&did_str, &doc_json)?;

    state.config.active_identity = Some(did_str.clone());
    state.config.app_config.active_identity = Some(did_str.clone());
    state.active_keyring = Some(keyring);
    state.active_did = Some(neunode_core::types::Did(did_str.clone()));
    state.save_config()?;

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
    writer.write_status(&format!("Identity '{name}' created, persisted to DB, set as active"));
    Ok(())
}

fn show_identity(did_arg: Option<&str>, writer: &OutputWriter, state: &AppState) -> Result<()> {
    let target_did = match did_arg {
        Some(d) => d.to_string(),
        None => state.require_did().map(|d| d.0.clone())?,
    };

    let store = state.identity_store();
    match store.get::<String>(&target_did)? {
        Some(doc_json) => {
            let doc = neunode_identity::document::DidDocument::from_json(&doc_json)?;
            let info = serde_json::json!({
                "did": doc.id,
                "method": "persisted",
                "verification_methods": doc.verification_method.len(),
                "services": doc.service.len(),
                "document": serde_json::from_str::<serde_json::Value>(&doc_json).unwrap_or_default(),
            });
            writer.write_json(&info);

            let pairs = [
                ("DID", doc.id.as_str()),
                ("Verification Methods", &doc.verification_method.len().to_string()),
                ("Services", &doc.service.len().to_string()),
            ];
            writer.write_key_value_pairs(&pairs);
        }
        None => {
            anyhow::bail!("identity '{target_did}' not found in local store");
        }
    }
    Ok(())
}

fn list_identities(writer: &OutputWriter, state: &AppState) -> Result<()> {
    let db = state.db();
    let entries = db.prefix_scan(neunode_storage::cf::CF_IDENTITY, &[])?;

    if entries.is_empty() {
        writer.write_status(
            "No identities found. Create one with: agnetd identity create --name <name>",
        );
        return Ok(());
    }

    let mut identities: Vec<(String, String)> = Vec::new();
    for (_, value_bytes) in &entries {
        if let Ok(doc_json) = bincode::deserialize::<String>(value_bytes) {
            if let Ok(doc) = neunode_identity::document::DidDocument::from_json(&doc_json) {
                identities.push((doc.id.clone(), "stored".to_string()));
            }
        }
    }

    if identities.is_empty() {
        writer.write_status(
            "No identities found. Create one with: agnetd identity create --name <name>",
        );
    } else {
        let headers = ["DID", "Status"];
        let rows: Vec<Vec<String>> =
            identities.iter().map(|(did, status)| vec![did.clone(), status.clone()]).collect();
        writer.write_table(&headers, &rows);
        writer.write_status(&format!("{} identit(es) found", identities.len()));
    }
    Ok(())
}

fn export_identity(
    did_arg: Option<&str>,
    output: &str,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    let target_did = match did_arg {
        Some(d) => d.to_string(),
        None => state.require_did().map(|d| d.0.clone())?,
    };

    let store = state.identity_store();
    let doc_json = store
        .get::<String>(&target_did)?
        .ok_or_else(|| anyhow::anyhow!("identity '{target_did}' not found in local store"))?;
    let doc = neunode_identity::document::DidDocument::from_json(&doc_json)?;

    let export_data = serde_json::json!({
        "did": doc.id,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "did_document": serde_json::from_str::<serde_json::Value>(&doc_json).unwrap_or_default(),
        "verification_methods": doc.verification_method.len(),
    });

    let json = serde_json::to_string_pretty(&export_data)?;
    fs::write(output, &json).with_context(|| format!("failed to write export to {output}"))?;

    writer.write_status(&format!("Identity {target_did} exported to {output}"));
    Ok(())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
