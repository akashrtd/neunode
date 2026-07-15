use anyhow::Result;
use neunode_core::types::TokenAmount;
use neunode_inference::provider::ModelInfo;
use neunode_storage::db::NeunodeDb;

use crate::cli::{GlobalArgs, ModelCommands};
use crate::output::OutputWriter;
use crate::state::AppState;

pub fn execute(cmd: &ModelCommands, args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(args.output);
    match cmd {
        ModelCommands::List { provider } => model_list(provider.as_deref(), &writer, state),
        ModelCommands::Show { model_id } => model_show(model_id, &writer, state),
        ModelCommands::Push { path, name } => model_push(path, name, &writer, state),
        ModelCommands::Rm { model_id } => model_rm(model_id, &writer, state),
    }
}

// ---------------------------------------------------------------------------
// Model DB helpers
// ---------------------------------------------------------------------------

pub(crate) fn store_model(db: &NeunodeDb, model: &ModelInfo) -> Result<()> {
    let key = format!("model:{}", model.id);
    let key_bytes = bincode::serialize(&key).map_err(|e| anyhow::anyhow!("key: {e}"))?;
    let value = bincode::serialize(model).map_err(|e| anyhow::anyhow!("serialize model: {e}"))?;
    db.put_raw(neunode_storage::cf::CF_MODELS, &key_bytes, &value)?;
    Ok(())
}

fn load_all_models(db: &NeunodeDb) -> Vec<ModelInfo> {
    let entries = match db.prefix_scan(neunode_storage::cf::CF_MODELS, &[]) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .iter()
        .filter(|(k, _)| {
            let key_str = bincode::deserialize::<String>(k).ok().unwrap_or_default();
            key_str.starts_with("model:")
        })
        .filter_map(|(_, v)| bincode::deserialize::<ModelInfo>(v).ok())
        .collect()
}

pub(crate) fn load_model(db: &NeunodeDb, model_id: &str) -> Result<Option<ModelInfo>> {
    let key = format!("model:{model_id}");
    let key_bytes = bincode::serialize(&key).map_err(|e| anyhow::anyhow!("key: {e}"))?;
    match db.get_raw(neunode_storage::cf::CF_MODELS, &key_bytes)? {
        Some(bytes) => {
            let model: ModelInfo =
                bincode::deserialize(&bytes).map_err(|e| anyhow::anyhow!("deserialize: {e}"))?;
            Ok(Some(model))
        }
        None => Ok(None),
    }
}

fn delete_model(db: &NeunodeDb, model_id: &str) -> Result<()> {
    let key = format!("model:{model_id}");
    let key_bytes = bincode::serialize(&key).map_err(|e| anyhow::anyhow!("key: {e}"))?;
    db.delete(neunode_storage::cf::CF_MODELS, &key_bytes)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Command implementations
// ---------------------------------------------------------------------------

fn model_list(provider: Option<&str>, writer: &OutputWriter, state: &AppState) -> Result<()> {
    let models = load_all_models(state.db());

    if models.is_empty() {
        writer.write_status("No models registered — use `agnetd model push` to add one");
        return Ok(());
    }

    let filtered: Vec<&ModelInfo> = match provider {
        Some(p) => models.iter().filter(|m| m.id.contains(p)).collect(),
        None => models.iter().collect(),
    };

    if filtered.is_empty() {
        writer.write_status("No models found matching filter");
        return Ok(());
    }

    let headers =
        ["Model ID", "Base Model", "Context", "Input/MTok", "Output/MTok", "Capabilities"];
    let rows: Vec<Vec<String>> = filtered
        .iter()
        .map(|m| {
            vec![
                m.id.clone(),
                m.base_model.clone().unwrap_or_else(|| "—".to_string()),
                m.context_length.to_string(),
                format!("{} nCompute", m.input_price_per_million),
                format!("{} nCompute", m.output_price_per_million),
                m.capabilities.join(", "),
            ]
        })
        .collect();
    writer.write_table(&headers, &rows);
    Ok(())
}

fn model_show(model_id: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    match load_model(state.db(), model_id)? {
        Some(m) => {
            let pairs = [
                ("Model ID", m.id.as_str()),
                ("Base Model", m.base_model.as_deref().unwrap_or("—")),
                ("Context Length", &m.context_length.to_string()),
                ("Input Price/MTok", &format!("{} nCompute", m.input_price_per_million)),
                ("Output Price/MTok", &format!("{} nCompute", m.output_price_per_million)),
                ("Total Price/MTok", &format!("{} nCompute", m.total_price_per_million())),
                ("Capabilities", &m.capabilities.join(", ")),
            ];
            writer.write_key_value_pairs(&pairs);
        }
        None => {
            anyhow::bail!("model not found: {model_id}");
        }
    }
    Ok(())
}

fn model_push(path: &str, name: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    let model = ModelInfo {
        id: name.to_string(),
        base_model: None,
        context_length: 4096,
        input_price_per_million: TokenAmount(100),
        output_price_per_million: TokenAmount(200),
        capabilities: vec!["chat".to_string()],
    };

    let pb = indicatif::ProgressBar::new_spinner();
    pb.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| indicatif::ProgressStyle::default_spinner()),
    );
    pb.set_message(format!("Registering model {name}..."));

    store_model(state.db(), &model)?;

    pb.finish_with_message(format!("Model '{name}' registered"));

    let pairs = [
        ("Status", "registered"),
        ("Model ID", name),
        ("Source", path),
        ("Context Length", "4096"),
        ("Input Price/MTok", "100 nCompute"),
        ("Output Price/MTok", "200 nCompute"),
    ];
    writer.write_key_value_pairs(&pairs);
    writer.write_status(&format!("Model '{name}' registered from {path}"));
    Ok(())
}

fn model_rm(model_id: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    match load_model(state.db(), model_id)? {
        Some(_) => {
            delete_model(state.db(), model_id)?;
            writer.write_status(&format!("Removed model {model_id}"));
            let info = serde_json::json!({
                "action": "remove",
                "model_id": model_id,
                "status": "removed",
            });
            writer.write_json(&info);
            Ok(())
        }
        None => {
            anyhow::bail!("model not found: {model_id}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{human_writer, json_writer as test_writer, test_state};

    #[test]
    fn list_empty_db() {
        let state = test_state();
        let writer = human_writer();
        model_list(None, &writer, &state).unwrap();
    }

    #[test]
    fn list_with_filter_no_match() {
        let state = test_state();
        let writer = test_writer();
        model_list(Some("nonexistent"), &writer, &state).unwrap();
    }

    #[test]
    fn show_not_found() {
        let state = test_state();
        let writer = test_writer();
        assert!(model_show("neunode/missing", &writer, &state).is_err());
    }

    #[test]
    fn push_and_show() {
        let state = test_state();
        let writer = human_writer();
        model_push("/tmp/model.gguf", "neunode/test-push", &writer, &state).unwrap();

        let writer2 = test_writer();
        model_show("neunode/test-push", &writer2, &state).unwrap();
    }

    #[test]
    fn push_list_and_rm() {
        let state = test_state();
        let writer = human_writer();

        model_push("/tmp/a.gguf", "neunode/model-a", &writer, &state).unwrap();
        model_push("/tmp/b.gguf", "neunode/model-b", &writer, &state).unwrap();

        let models = load_all_models(state.db());
        assert_eq!(models.len(), 2);

        let writer2 = test_writer();
        model_rm("neunode/model-a", &writer2, &state).unwrap();

        let remaining = load_all_models(state.db());
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "neunode/model-b");
    }

    #[test]
    fn rm_not_found() {
        let state = test_state();
        let writer = test_writer();
        assert!(model_rm("neunode/nonexistent", &writer, &state).is_err());
    }

    #[test]
    fn store_and_load_model() {
        let state = test_state();
        let model = ModelInfo {
            id: "neunode/roundtrip".to_string(),
            base_model: Some("meta-llama/Llama-3.2-3B".to_string()),
            context_length: 8192,
            input_price_per_million: TokenAmount(150),
            output_price_per_million: TokenAmount(300),
            capabilities: vec!["chat".to_string(), "streaming".to_string()],
        };

        store_model(state.db(), &model).unwrap();
        let loaded = load_model(state.db(), "neunode/roundtrip").unwrap().unwrap();
        assert_eq!(loaded.id, model.id);
        assert_eq!(loaded.base_model, model.base_model);
        assert_eq!(loaded.context_length, model.context_length);
        assert_eq!(loaded.capabilities, model.capabilities);
    }
}
