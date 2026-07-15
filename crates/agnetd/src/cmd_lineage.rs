use anyhow::Result;
use neunode_lineage::{
    compute_content_hash, compute_royalties, ContributionType, LineageDag, ModelMetadata, ModelNode,
};
use neunode_storage::cf::CF_MODELS;

use crate::cli::{GlobalArgs, LineageCommands};
use crate::output::OutputWriter;
use crate::state::AppState;

const LINEAGE_PREFIX: &str = "lineage:";

pub fn execute(cmd: &LineageCommands, args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(args.output);
    match cmd {
        LineageCommands::Register { cid, parents, contribution_type, lora_rank, lora_alpha } => {
            register_model(
                cid,
                parents.as_deref(),
                contribution_type,
                *lora_rank,
                *lora_alpha,
                &writer,
                state,
            )
        }
        LineageCommands::Show { cid } => show_model(cid, &writer, state),
        LineageCommands::Parents { cid } => show_parents(cid, &writer, state),
        LineageCommands::Children { cid } => show_children(cid, &writer, state),
        LineageCommands::Ancestors { cid } => show_ancestors(cid, &writer, state),
        LineageCommands::Depth { cid } => show_depth(cid, &writer, state),
        LineageCommands::Royalties { cid, amount } => show_royalties(cid, *amount, &writer, state),
        LineageCommands::Hash { file } => hash_file(file, &writer),
        LineageCommands::Verify { cid } => verify_signature(cid, &writer, state),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a CLI contribution type string into the enum.
fn parse_contribution_type(
    raw: &str,
    lora_rank: Option<u32>,
    lora_alpha: Option<f64>,
) -> Result<ContributionType> {
    match raw {
        "pre_training" => Ok(ContributionType::PreTraining),
        "fine_tune" => Ok(ContributionType::FineTune {
            lora_rank: lora_rank.unwrap_or(8),
            lora_alpha: lora_alpha.unwrap_or(16.0),
        }),
        "merge" => Ok(ContributionType::Merge { merge_method: "default".to_string() }),
        "rl" => Ok(ContributionType::RL { reward_model_cid: String::new() }),
        "data" => Ok(ContributionType::Data { dataset_hash: String::new() }),
        "compute" => Ok(ContributionType::Compute { duration_secs: 0.0 }),
        other => anyhow::bail!(
            "unknown contribution type '{other}' \
             (expected: pre_training, fine_tune, merge, rl, data, compute)"
        ),
    }
}

/// Human-readable label for a contribution type.
fn ct_label(ct: &ContributionType) -> String {
    match ct {
        ContributionType::PreTraining => "pre_training".to_string(),
        ContributionType::FineTune { lora_rank, lora_alpha } => {
            format!("fine_tune(rank={lora_rank}, alpha={lora_alpha})")
        }
        ContributionType::Merge { merge_method } => format!("merge({merge_method})"),
        ContributionType::RL { reward_model_cid } => format!("rl(reward={reward_model_cid})"),
        ContributionType::Data { dataset_hash } => format!("data(hash={dataset_hash})"),
        ContributionType::Compute { duration_secs } => format!("compute({duration_secs}s)"),
    }
}

/// Validate CID has the `sha256:` prefix format.
fn validate_cid(cid: &str) -> Result<()> {
    if cid.is_empty() {
        anyhow::bail!("CID cannot be empty");
    }
    if !cid.starts_with("sha256:") {
        anyhow::bail!("CID must start with 'sha256:' (got '{cid}')");
    }
    let hex = &cid[7..];
    if hex.is_empty() {
        anyhow::bail!("CID hex portion cannot be empty");
    }
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!("CID hex portion contains non-hex characters");
    }
    Ok(())
}

/// Store a model node as JSON in the `models` column family.
pub(crate) fn store_model_node(
    db: &neunode_storage::db::NeunodeDb,
    node: &ModelNode,
) -> Result<()> {
    let key = format!("{LINEAGE_PREFIX}{}", node.cid);
    let value = serde_json::to_vec(node)?;
    db.put_raw(CF_MODELS, key.as_bytes(), &value)?;
    Ok(())
}

/// Load a single model node by CID from the `models` CF.
pub(crate) fn load_model_node(
    db: &neunode_storage::db::NeunodeDb,
    cid: &str,
) -> Result<Option<ModelNode>> {
    let key = format!("{LINEAGE_PREFIX}{cid}");
    match db.get_raw(CF_MODELS, key.as_bytes())? {
        Some(bytes) => {
            let node: ModelNode = serde_json::from_slice(&bytes)?;
            Ok(Some(node))
        }
        None => match db.get_raw(CF_MODELS, cid.as_bytes())? {
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            None => Ok(None),
        },
    }
}

pub(crate) fn rebuild_dag(db: &neunode_storage::db::NeunodeDb) -> Result<LineageDag> {
    let mut all_kv = db.prefix_scan(CF_MODELS, b"sha256:")?;
    all_kv.extend(db.prefix_scan(CF_MODELS, LINEAGE_PREFIX.as_bytes())?);
    let mut nodes_by_cid = std::collections::HashMap::new();
    for (_k, v) in all_kv {
        let node: ModelNode = serde_json::from_slice(&v)?;
        nodes_by_cid.insert(node.cid.clone(), node);
    }
    let mut dag = LineageDag::new();
    while !nodes_by_cid.is_empty() {
        let mut ready: Vec<String> = nodes_by_cid
            .iter()
            .filter(|(_, node)| node.parent_cids.iter().all(|parent| dag.contains(parent)))
            .map(|(cid, _)| cid.clone())
            .collect();
        ready.sort();
        if ready.is_empty() {
            let unresolved = nodes_by_cid.keys().cloned().collect::<Vec<_>>().join(", ");
            anyhow::bail!("lineage contains missing parents or a cycle: {unresolved}");
        }
        for cid in ready {
            let node = nodes_by_cid
                .remove(&cid)
                .ok_or_else(|| anyhow::anyhow!("lineage reconstruction lost node {cid}"))?;
            if !verify_node(db, &node)? {
                anyhow::bail!("invalid lineage signature for {cid}");
            }
            dag.register(node)?;
        }
    }
    Ok(dag)
}

pub(crate) fn sign_node(
    keyring: &neunode_identity::keyring::Keyring,
    expected_did: &str,
    node: &mut ModelNode,
) -> Result<()> {
    if keyring.to_did().as_str() != expected_did || node.contributor_did != expected_did {
        anyhow::bail!("active keyring does not control lineage contributor DID");
    }
    let payload = neunode_lineage::sigchain::model_node_signing_payload(node);
    node.signature = keyring
        .sign_ed25519_domain(neunode_crypto::hash::DOMAIN_MODEL_LINEAGE, &payload)
        .to_bytes()
        .to_vec();
    Ok(())
}

pub(crate) fn verify_node(db: &neunode_storage::db::NeunodeDb, node: &ModelNode) -> Result<bool> {
    let store = neunode_storage::identity_store::IdentityStore::new(db);
    let document_json = store
        .get::<String>(&node.contributor_did)?
        .ok_or_else(|| anyhow::anyhow!("DID document not found: {}", node.contributor_did))?;
    let document = neunode_identity::document::DidDocument::from_json(&document_json)
        .map_err(|e| anyhow::anyhow!("invalid DID document for {}: {e}", node.contributor_did))?;
    if document.id != node.contributor_did {
        anyhow::bail!("DID document subject does not match lineage contributor");
    }
    let verifying_key = document
        .ed25519_verifying_key()
        .map_err(|e| anyhow::anyhow!("cannot resolve contributor verification key: {e}"))?;
    Ok(neunode_lineage::sigchain::verify_model_node(&verifying_key, node))
}

// ---------------------------------------------------------------------------
// Subcommand handlers
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn register_model(
    cid: &str,
    parents: Option<&str>,
    contribution_type: &str,
    lora_rank: Option<u32>,
    lora_alpha: Option<f64>,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    validate_cid(cid)?;

    let ct = parse_contribution_type(contribution_type, lora_rank, lora_alpha)?;

    let parent_cids: Vec<String> = match parents {
        Some(p) if !p.is_empty() => p.split(',').map(|s| s.trim().to_string()).collect(),
        _ => vec![],
    };

    // Validate parent CID format.
    for pcid in &parent_cids {
        validate_cid(pcid)?;
    }

    let contributor = state.require_did()?.clone();
    let keyring = state.require_keyring()?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let mut node = ModelNode {
        cid: cid.to_string(),
        parent_cids,
        contributor_did: contributor.0.clone(),
        contribution_type: ct.clone(),
        signature: vec![],
        created_at: now_ms,
        metadata: ModelMetadata::default(),
    };
    sign_node(keyring, &contributor.0, &mut node)?;

    // Rebuild DAG and validate registration.
    let db = state.db();
    let mut dag = rebuild_dag(db)?;
    dag.register(node.clone())?;

    // Persist node to RocksDB.
    store_model_node(db, &node)?;

    let out = serde_json::json!({
        "cid": node.cid,
        "parent_cids": node.parent_cids,
        "contributor_did": node.contributor_did,
        "contribution_type": ct_label(&ct),
        "created_at": node.created_at,
    });

    writer.write_json(&out);
    writer.write_status(&format!("Model registered: {cid}"));
    Ok(())
}

fn show_model(cid: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    validate_cid(cid)?;

    let db = state.db();
    let node =
        load_model_node(db, cid)?.ok_or_else(|| anyhow::anyhow!("model '{cid}' not found"))?;

    let parents_str = if node.parent_cids.is_empty() {
        "(base model)".to_string()
    } else {
        node.parent_cids.join(", ")
    };

    let sig_hex: String = node.signature.iter().map(|b| format!("{b:02x}")).collect();
    let sig_display = if sig_hex.len() > 16 { format!("{}...", &sig_hex[..16]) } else { sig_hex };

    let pairs = [
        ("CID", node.cid.clone()),
        ("Parents", parents_str),
        ("Contribution", ct_label(&node.contribution_type)),
        ("Contributor", node.contributor_did.clone()),
        ("Created At", node.created_at.to_string()),
        ("Signature", sig_display),
        ("Dataset Hash", node.metadata.dataset_hash.unwrap_or_default()),
        ("Base Model", node.metadata.base_model_hash.unwrap_or_default()),
        (
            "Training Duration",
            node.metadata.training_duration_secs.map(|d| format!("{d}s")).unwrap_or_default(),
        ),
    ];

    writer.write_key_value_pairs(&pairs.iter().map(|(k, v)| (*k, v.as_str())).collect::<Vec<_>>());
    Ok(())
}

fn show_parents(cid: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    validate_cid(cid)?;

    let db = state.db();
    let dag = rebuild_dag(db)?;
    let parents = dag.parents(cid)?;

    if parents.is_empty() {
        writer.write_status(&format!("{cid} is a base model (no parents)"));
        return Ok(());
    }

    let headers = ["CID", "Contribution", "Contributor", "Created"];
    let rows: Vec<Vec<String>> = parents
        .iter()
        .map(|p| {
            vec![
                p.cid.clone(),
                ct_label(&p.contribution_type),
                p.contributor_did.clone(),
                p.created_at.to_string(),
            ]
        })
        .collect();

    writer.write_table(&headers, &rows);
    Ok(())
}

fn show_children(cid: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    validate_cid(cid)?;

    let db = state.db();
    let dag = rebuild_dag(db)?;
    let children = dag.children(cid)?;

    if children.is_empty() {
        writer.write_status(&format!("{cid} has no children"));
        return Ok(());
    }

    let headers = ["CID", "Contribution", "Contributor", "Created"];
    let rows: Vec<Vec<String>> = children
        .iter()
        .map(|c| {
            vec![
                c.cid.clone(),
                ct_label(&c.contribution_type),
                c.contributor_did.clone(),
                c.created_at.to_string(),
            ]
        })
        .collect();

    writer.write_table(&headers, &rows);
    Ok(())
}

fn show_ancestors(cid: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    validate_cid(cid)?;

    let db = state.db();
    let dag = rebuild_dag(db)?;
    let ancestors = dag.ancestors(cid)?;

    if ancestors.is_empty() {
        writer.write_status(&format!("{cid} has no ancestors (base model)"));
        return Ok(());
    }

    let headers = ["CID", "Contribution", "Contributor", "Created"];
    let rows: Vec<Vec<String>> = ancestors
        .iter()
        .map(|a| {
            vec![
                a.cid.clone(),
                ct_label(&a.contribution_type),
                a.contributor_did.clone(),
                a.created_at.to_string(),
            ]
        })
        .collect();

    writer.write_table(&headers, &rows);
    Ok(())
}

fn show_depth(cid: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    validate_cid(cid)?;

    let db = state.db();
    let dag = rebuild_dag(db)?;
    let depth = dag.lineage_depth(cid)?;

    let out = serde_json::json!({
        "cid": cid,
        "lineage_depth": depth,
    });

    writer.write_json(&out);
    writer.write_status(&format!("Lineage depth for {cid}: {depth}"));
    Ok(())
}

fn show_royalties(cid: &str, amount: u32, writer: &OutputWriter, state: &AppState) -> Result<()> {
    validate_cid(cid)?;

    let db = state.db();
    let dag = rebuild_dag(db)?;
    let allocs = compute_royalties(&dag, cid, amount)?;

    if allocs.is_empty() {
        writer.write_status(&format!("{cid} has no ancestors — no royalties to distribute"));
        return Ok(());
    }

    let headers = ["Contributor", "Type", "Hops", "Weight", "Basis Points", "Percentage"];
    let rows: Vec<Vec<String>> = allocs
        .iter()
        .map(|a| {
            let pct = if amount > 0 {
                format!("{:.2}%", (a.amount_basis_points as f64 / amount as f64) * 100.0)
            } else {
                "0.00%".to_string()
            };
            vec![
                a.contributor_did.clone(),
                ct_label(&a.contribution_type),
                a.hops.to_string(),
                format!("{:.4}", a.weight),
                a.amount_basis_points.to_string(),
                pct,
            ]
        })
        .collect();

    writer.write_table(&headers, &rows);
    Ok(())
}

fn hash_file(file: &str, writer: &OutputWriter) -> Result<()> {
    if file.is_empty() {
        anyhow::bail!("file path cannot be empty");
    }

    let data = std::fs::read(file).map_err(|e| anyhow::anyhow!("failed to read '{file}': {e}"))?;

    let (hash, method) = if file.ends_with(".safetensors") {
        let h = neunode_lineage::provenance::compute_safetensors_hash(&data)?;
        (h, "safetensors")
    } else {
        (compute_content_hash(&data), "sha256")
    };

    let out = serde_json::json!({
        "file": file,
        "hash": hash,
        "method": method,
        "size_bytes": data.len(),
    });

    writer.write_json(&out);
    writer.write_status(&format!("Content hash: {hash}"));
    Ok(())
}

fn verify_signature(cid: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    validate_cid(cid)?;

    let db = state.db();
    let node =
        load_model_node(db, cid)?.ok_or_else(|| anyhow::anyhow!("model '{cid}' not found"))?;

    let sig_valid = verify_node(db, &node)?;
    // Check that CID, contributor_did are non-empty.
    let fields_valid = !node.cid.is_empty() && !node.contributor_did.is_empty();

    let all_valid = sig_valid && fields_valid;

    let out = serde_json::json!({
        "cid": cid,
        "signature_valid": sig_valid,
        "fields_valid": fields_valid,
        "verified": all_valid,
        "signature_length": node.signature.len(),
        "contributor_did": node.contributor_did,
    });

    writer.write_json(&out);

    if all_valid {
        writer.write_status(&format!("Model {cid} verified successfully"));
    } else {
        writer.write_error(&format!("Model {cid} verification failed"));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{human_writer, json_writer as test_writer, test_state};
    use neunode_lineage::ContributionType;

    fn persist_active_document(state: &AppState) {
        let keyring = state.require_keyring().unwrap();
        let document = neunode_identity::document::DidDocument::from_keyring(keyring).unwrap();
        state.identity_store().put(document.id.as_str(), &document.to_json().unwrap()).unwrap();
    }

    // ---- Register tests ----

    #[test]
    fn register_base_model() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:abc123", None, "pre_training", None, None, &writer, &state).unwrap();

        let db = state.db();
        let loaded = load_model_node(db, "sha256:abc123").unwrap();
        assert!(loaded.is_some());
        let node = loaded.unwrap();
        assert_eq!(node.cid, "sha256:abc123");
        assert_eq!(node.signature.len(), 64);
        assert!(node.parent_cids.is_empty());
        assert!(matches!(node.contribution_type, ContributionType::PreTraining));
    }

    #[test]
    fn registered_node_verifies_against_contributor_did_document() {
        let state = test_state();
        persist_active_document(&state);
        register_model("sha256:a123", None, "pre_training", None, None, &test_writer(), &state)
            .unwrap();
        let node = load_model_node(state.db(), "sha256:a123").unwrap().unwrap();
        assert!(verify_node(state.db(), &node).unwrap());

        let mut tampered = node;
        tampered.contribution_type =
            ContributionType::Merge { merge_method: "malicious".to_string() };
        assert!(!verify_node(state.db(), &tampered).unwrap());
    }

    #[test]
    fn legacy_raw_cid_records_remain_readable() {
        let state = test_state();
        let mut node = ModelNode {
            cid: "sha256:c123".to_string(),
            parent_cids: vec![],
            contributor_did: state.require_did().unwrap().0.clone(),
            contribution_type: ContributionType::PreTraining,
            signature: vec![],
            created_at: 1,
            metadata: ModelMetadata::default(),
        };
        sign_node(state.require_keyring().unwrap(), &node.contributor_did.clone(), &mut node)
            .unwrap();
        state
            .db()
            .put_raw(CF_MODELS, node.cid.as_bytes(), &serde_json::to_vec(&node).unwrap())
            .unwrap();

        assert_eq!(load_model_node(state.db(), &node.cid).unwrap().unwrap().cid, node.cid);
        assert_eq!(rebuild_dag(state.db()).unwrap().len(), 1);
    }

    #[test]
    fn verification_rejects_substituted_did_document_key() {
        let state = test_state();
        persist_active_document(&state);
        register_model("sha256:b123", None, "pre_training", None, None, &test_writer(), &state)
            .unwrap();
        let node = load_model_node(state.db(), "sha256:b123").unwrap().unwrap();

        let attacker = neunode_identity::keyring::Keyring::generate();
        let mut document =
            neunode_identity::document::DidDocument::from_keyring(&attacker).unwrap();
        let contributor = node.contributor_did.clone();
        document.id = contributor.clone();
        for method in &mut document.verification_method {
            let fragment = method.id.rsplit_once('#').map(|(_, value)| value).unwrap_or("keys-1");
            method.id = format!("{contributor}#{fragment}");
            method.controller = contributor.clone();
        }
        document.authentication = vec![format!("{contributor}#keys-1")];
        state.identity_store().put(&contributor, &document.to_json().unwrap()).unwrap();
        assert!(!verify_node(state.db(), &node).unwrap());
    }

    #[test]
    fn register_fine_tune() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:babe", None, "pre_training", None, None, &writer, &state).unwrap();

        let writer2 = test_writer();
        register_model(
            "sha256:f1ee",
            Some("sha256:babe"),
            "fine_tune",
            Some(16),
            Some(32.0),
            &writer2,
            &state,
        )
        .unwrap();

        let node = load_model_node(state.db(), "sha256:f1ee").unwrap().unwrap();
        assert_eq!(node.parent_cids, vec!["sha256:babe"]);
        match node.contribution_type {
            ContributionType::FineTune { lora_rank, lora_alpha } => {
                assert_eq!(lora_rank, 16);
                assert!((lora_alpha - 32.0).abs() < f64::EPSILON);
            }
            other => panic!("expected FineTune, got {other:?}"),
        }
    }

    #[test]
    fn register_fine_tune_default_params() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:ba5e", None, "pre_training", None, None, &writer, &state).unwrap();

        let writer2 = test_writer();
        register_model(
            "sha256:feed",
            Some("sha256:ba5e"),
            "fine_tune",
            None,
            None,
            &writer2,
            &state,
        )
        .unwrap();

        let node = load_model_node(state.db(), "sha256:feed").unwrap().unwrap();
        match node.contribution_type {
            ContributionType::FineTune { lora_rank, lora_alpha } => {
                assert_eq!(lora_rank, 8);
                assert!((lora_alpha - 16.0).abs() < f64::EPSILON);
            }
            other => panic!("expected FineTune, got {other:?}"),
        }
    }

    #[test]
    fn register_merge_two_parents() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:aa00", None, "pre_training", None, None, &writer, &state).unwrap();
        let writer2 = test_writer();
        register_model("sha256:bb00", None, "pre_training", None, None, &writer2, &state).unwrap();

        let writer3 = test_writer();
        register_model(
            "sha256:cc00",
            Some("sha256:aa00,sha256:bb00"),
            "merge",
            None,
            None,
            &writer3,
            &state,
        )
        .unwrap();

        let node = load_model_node(state.db(), "sha256:cc00").unwrap().unwrap();
        assert_eq!(node.parent_cids.len(), 2);
        assert!(matches!(node.contribution_type, ContributionType::Merge { .. }));
    }

    #[test]
    fn register_all_contribution_types() {
        let state = test_state();
        let cases = [
            ("sha256:a111", "pre_training", "PreTraining"),
            ("sha256:b222", "rl", "RL"),
            ("sha256:c333", "data", "Data"),
            ("sha256:d444", "compute", "Compute"),
        ];
        for (cid, ct_str, expected_variant) in cases {
            let writer = test_writer();
            register_model(cid, None, ct_str, None, None, &writer, &state).unwrap();
            let node = load_model_node(state.db(), cid).unwrap().unwrap();
            assert_eq!(
                format!("{:?}", node.contribution_type).split('{').next().unwrap().trim(),
                expected_variant
            );
        }
    }

    #[test]
    fn register_invalid_cid_no_prefix() {
        let state = test_state();
        let writer = test_writer();
        let result =
            register_model("invalidcid", None, "pre_training", None, None, &writer, &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("sha256:"));
    }

    #[test]
    fn register_empty_cid() {
        let state = test_state();
        let writer = test_writer();
        let result = register_model("", None, "pre_training", None, None, &writer, &state);
        assert!(result.is_err());
    }

    #[test]
    fn register_unknown_contribution_type() {
        let state = test_state();
        let writer = test_writer();
        let result =
            register_model("sha256:abc", None, "unknown_type", None, None, &writer, &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown contribution type"));
    }

    #[test]
    fn register_parent_not_found() {
        let state = test_state();
        let writer = test_writer();
        let result = register_model(
            "sha256:c0de",
            Some("sha256:9999"),
            "fine_tune",
            None,
            None,
            &writer,
            &state,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn register_duplicate_cid() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:d000", None, "pre_training", None, None, &writer, &state).unwrap();

        let writer2 = test_writer();
        let result =
            register_model("sha256:d000", None, "pre_training", None, None, &writer2, &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already registered"));
    }

    // ---- Show tests ----

    #[test]
    fn show_model_valid() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:5e01", None, "pre_training", None, None, &writer, &state).unwrap();

        let writer2 = test_writer();
        show_model("sha256:5e01", &writer2, &state).unwrap();
    }

    #[test]
    fn show_model_not_found() {
        let state = test_state();
        let writer = test_writer();
        let result = show_model("sha256:ffff", &writer, &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn show_model_empty_cid() {
        let state = test_state();
        let writer = test_writer();
        let result = show_model("", &writer, &state);
        assert!(result.is_err());
    }

    // ---- Parents tests ----

    #[test]
    fn show_parents_of_base_model() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:1000", None, "pre_training", None, None, &writer, &state).unwrap();

        let writer2 = human_writer();
        show_parents("sha256:1000", &writer2, &state).unwrap();
    }

    #[test]
    fn show_parents_of_fine_tune() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:2000", None, "pre_training", None, None, &writer, &state).unwrap();
        let writer2 = test_writer();
        register_model(
            "sha256:2001",
            Some("sha256:2000"),
            "fine_tune",
            None,
            None,
            &writer2,
            &state,
        )
        .unwrap();

        let writer3 = test_writer();
        show_parents("sha256:2001", &writer3, &state).unwrap();
    }

    // ---- Children tests ----

    #[test]
    fn show_children_of_leaf() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:3000", None, "pre_training", None, None, &writer, &state).unwrap();
        let writer2 = test_writer();
        register_model(
            "sha256:3001",
            Some("sha256:3000"),
            "fine_tune",
            None,
            None,
            &writer2,
            &state,
        )
        .unwrap();

        let writer3 = human_writer();
        show_children("sha256:3001", &writer3, &state).unwrap();
    }

    #[test]
    fn show_children_of_base() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:3100", None, "pre_training", None, None, &writer, &state).unwrap();
        let writer2 = test_writer();
        register_model(
            "sha256:3101",
            Some("sha256:3100"),
            "fine_tune",
            None,
            None,
            &writer2,
            &state,
        )
        .unwrap();

        let writer3 = test_writer();
        show_children("sha256:3100", &writer3, &state).unwrap();
    }

    // ---- Ancestors tests ----

    #[test]
    fn show_ancestors_linear_chain() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:a100", None, "pre_training", None, None, &writer, &state).unwrap();
        let writer2 = test_writer();
        register_model(
            "sha256:b100",
            Some("sha256:a100"),
            "fine_tune",
            None,
            None,
            &writer2,
            &state,
        )
        .unwrap();
        let writer3 = test_writer();
        register_model(
            "sha256:c100",
            Some("sha256:b100"),
            "fine_tune",
            None,
            None,
            &writer3,
            &state,
        )
        .unwrap();

        let writer4 = test_writer();
        show_ancestors("sha256:c100", &writer4, &state).unwrap();
    }

    #[test]
    fn show_ancestors_base_model() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:4000", None, "pre_training", None, None, &writer, &state).unwrap();

        let writer2 = human_writer();
        show_ancestors("sha256:4000", &writer2, &state).unwrap();
    }

    // ---- Depth tests ----

    #[test]
    fn show_depth_base_model() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:4100", None, "pre_training", None, None, &writer, &state).unwrap();

        let writer2 = test_writer();
        show_depth("sha256:4100", &writer2, &state).unwrap();
    }

    #[test]
    fn show_depth_chain() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:5000", None, "pre_training", None, None, &writer, &state).unwrap();
        let writer2 = test_writer();
        register_model(
            "sha256:5001",
            Some("sha256:5000"),
            "fine_tune",
            None,
            None,
            &writer2,
            &state,
        )
        .unwrap();
        let writer3 = test_writer();
        register_model("sha256:5002", Some("sha256:5001"), "rl", None, None, &writer3, &state)
            .unwrap();

        let dag = rebuild_dag(state.db()).unwrap();
        assert_eq!(dag.lineage_depth("sha256:5000").unwrap(), 0);
        assert_eq!(dag.lineage_depth("sha256:5001").unwrap(), 1);
        assert_eq!(dag.lineage_depth("sha256:5002").unwrap(), 2);
    }

    // ---- Royalties tests ----

    #[test]
    fn show_royalties_single_parent() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:6000", None, "pre_training", None, None, &writer, &state).unwrap();
        let writer2 = test_writer();
        register_model(
            "sha256:6001",
            Some("sha256:6000"),
            "fine_tune",
            None,
            None,
            &writer2,
            &state,
        )
        .unwrap();

        let writer3 = test_writer();
        show_royalties("sha256:6001", 1000, &writer3, &state).unwrap();
    }

    #[test]
    fn show_royalties_no_ancestors() {
        let state = test_state();
        let writer = test_writer();
        register_model("sha256:6100", None, "pre_training", None, None, &writer, &state).unwrap();

        let writer2 = human_writer();
        show_royalties("sha256:6100", 1000, &writer2, &state).unwrap();
    }

    // ---- Hash tests ----

    #[test]
    fn hash_file_valid() {
        let dir = std::env::temp_dir().join("agnetd_lineage_hash_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.bin");
        std::fs::write(&path, b"hello lineage").unwrap();

        let writer = test_writer();
        hash_file(path.to_str().unwrap(), &writer).unwrap();

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hash_file_empty_path() {
        let writer = test_writer();
        let result = hash_file("", &writer);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn hash_file_not_found() {
        let writer = test_writer();
        let result = hash_file("/nonexistent/path/file.bin", &writer);
        assert!(result.is_err());
    }

    #[test]
    fn hash_file_safetensors() {
        let dir = std::env::temp_dir().join("agnetd_lineage_safetensors_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model.safetensors");

        let header = r#"{"t":{"dtype":"F32","shape":[2],"data_offsets":[0,8]}}"#;
        let header_bytes = header.as_bytes();
        let header_len = header_bytes.len() as u64;
        let mut data = Vec::new();
        data.extend_from_slice(&header_len.to_le_bytes());
        data.extend_from_slice(header_bytes);
        data.extend_from_slice(&[0u8; 8]);
        std::fs::write(&path, &data).unwrap();

        let writer = test_writer();
        hash_file(path.to_str().unwrap(), &writer).unwrap();

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- Verify tests ----

    #[test]
    fn verify_model_valid() {
        let state = test_state();
        persist_active_document(&state);
        let writer = test_writer();
        register_model("sha256:7000", None, "pre_training", None, None, &writer, &state).unwrap();

        let writer2 = test_writer();
        verify_signature("sha256:7000", &writer2, &state).unwrap();
    }

    #[test]
    fn verify_model_not_found() {
        let state = test_state();
        let writer = test_writer();
        let result = verify_signature("sha256:9999", &writer, &state);
        assert!(result.is_err());
    }

    #[test]
    fn verify_model_invalid_cid() {
        let state = test_state();
        let writer = test_writer();
        let result = verify_signature("bogus", &writer, &state);
        assert!(result.is_err());
    }

    // ---- Helper tests ----

    #[test]
    fn parse_contribution_type_all_variants() {
        assert!(matches!(
            parse_contribution_type("pre_training", None, None).unwrap(),
            ContributionType::PreTraining
        ));
        assert!(matches!(
            parse_contribution_type("fine_tune", Some(4), Some(8.0)).unwrap(),
            ContributionType::FineTune { lora_rank: 4, lora_alpha: 8.0 }
        ));
        assert!(matches!(
            parse_contribution_type("merge", None, None).unwrap(),
            ContributionType::Merge { .. }
        ));
        assert!(matches!(
            parse_contribution_type("rl", None, None).unwrap(),
            ContributionType::RL { .. }
        ));
        assert!(matches!(
            parse_contribution_type("data", None, None).unwrap(),
            ContributionType::Data { .. }
        ));
        assert!(matches!(
            parse_contribution_type("compute", None, None).unwrap(),
            ContributionType::Compute { .. }
        ));
    }

    #[test]
    fn parse_contribution_type_unknown_fails() {
        assert!(parse_contribution_type("bogus", None, None).is_err());
    }

    #[test]
    fn validate_cid_valid() {
        validate_cid("sha256:abc123").unwrap();
    }

    #[test]
    fn validate_cid_empty() {
        assert!(validate_cid("").is_err());
    }

    #[test]
    fn validate_cid_no_prefix() {
        assert!(validate_cid("abc123").is_err());
    }

    #[test]
    fn validate_cid_empty_hex() {
        assert!(validate_cid("sha256:").is_err());
    }

    #[test]
    fn validate_cid_non_hex() {
        assert!(validate_cid("sha256:xyz!!!").is_err());
    }

    #[test]
    fn ct_label_variants() {
        assert_eq!(ct_label(&ContributionType::PreTraining), "pre_training");
        assert!(ct_label(&ContributionType::FineTune { lora_rank: 8, lora_alpha: 16.0 })
            .contains("rank=8"));
        assert!(
            ct_label(&ContributionType::Merge { merge_method: "slerp".into() }).contains("slerp")
        );
    }

    // ---- Integration: 3-level DAG ----

    #[test]
    fn three_level_dag_full_workflow() {
        let state = test_state();
        persist_active_document(&state);
        let writer = test_writer();

        register_model("sha256:8000", None, "pre_training", None, None, &writer, &state).unwrap();
        let w2 = test_writer();
        register_model(
            "sha256:8001",
            Some("sha256:8000"),
            "fine_tune",
            Some(16),
            Some(32.0),
            &w2,
            &state,
        )
        .unwrap();
        let w3 = test_writer();
        register_model("sha256:8002", Some("sha256:8001"), "rl", None, None, &w3, &state).unwrap();

        let dag = rebuild_dag(state.db()).unwrap();
        assert_eq!(dag.lineage_depth("sha256:8000").unwrap(), 0);
        assert_eq!(dag.lineage_depth("sha256:8001").unwrap(), 1);
        assert_eq!(dag.lineage_depth("sha256:8002").unwrap(), 2);

        let w4 = test_writer();
        show_parents("sha256:8002", &w4, &state).unwrap();

        let w5 = test_writer();
        show_children("sha256:8000", &w5, &state).unwrap();

        let ancestors = dag.ancestors("sha256:8002").unwrap();
        assert_eq!(ancestors.len(), 2);

        let w6 = test_writer();
        show_royalties("sha256:8002", 1000, &w6, &state).unwrap();

        let w7 = test_writer();
        verify_signature("sha256:8002", &w7, &state).unwrap();
    }

    #[test]
    fn model_not_found_for_dag_query() {
        let state = test_state();
        let writer = test_writer();
        let result = show_depth("sha256:ffff", &writer, &state);
        assert!(result.is_err());
    }
}
