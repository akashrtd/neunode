use anyhow::Result;

use crate::cli::{Cli, ModelCommands};
use crate::config::CliConfig;
use crate::output::OutputWriter;

pub fn execute(cmd: &ModelCommands, cli: &Cli, _config: &mut CliConfig) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        ModelCommands::List { provider } => model_list(provider.as_deref(), &writer),
        ModelCommands::Show { model_id } => model_show(model_id, &writer),
        ModelCommands::Push { path, name } => model_push(path, name, &writer),
        ModelCommands::Rm { model_id } => model_rm(model_id, &writer),
    }
}

fn model_list(provider: Option<&str>, writer: &OutputWriter) -> Result<()> {
    let models = sample_models();
    let filtered: Vec<&neunode_inference::provider::ModelInfo> = match provider {
        Some(p) => models.iter().filter(|m| m.id.contains(p)).collect(),
        None => models.iter().collect(),
    };

    if filtered.is_empty() {
        writer.write_status("No models found");
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

fn model_show(model_id: &str, writer: &OutputWriter) -> Result<()> {
    let models = sample_models();
    let model = models.iter().find(|m| m.id == model_id);

    match model {
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
            writer.write_error(&format!("model not found: {model_id}"));
            anyhow::bail!("model not found: {model_id}");
        }
    }
    Ok(())
}

fn model_push(path: &str, name: &str, writer: &OutputWriter) -> Result<()> {
    writer.write_status(&format!("Pushing model from {path} as '{name}'"));
    writer.write_warning("Model push not yet available in Phase 1 MVP — dry run");
    let info = serde_json::json!({
        "action": "push",
        "path": path,
        "name": name,
        "status": "dry_run",
    });
    writer.write_json(&info);
    Ok(())
}

fn model_rm(model_id: &str, writer: &OutputWriter) -> Result<()> {
    writer.write_status(&format!("Removed model {model_id}"));
    writer.write_warning("Model removal not yet available in Phase 1 MVP — dry run");
    let info = serde_json::json!({
        "action": "remove",
        "model_id": model_id,
        "status": "dry_run",
    });
    writer.write_json(&info);
    Ok(())
}

fn sample_models() -> Vec<neunode_inference::provider::ModelInfo> {
    use neunode_core::types::TokenAmount;
    vec![
        neunode_inference::provider::ModelInfo {
            id: "neunode/llama-3b-v1".to_string(),
            base_model: Some("meta-llama/Llama-3.2-3B".to_string()),
            context_length: 4096,
            input_price_per_million: TokenAmount(100),
            output_price_per_million: TokenAmount(200),
            capabilities: vec!["chat".to_string(), "streaming".to_string()],
        },
        neunode_inference::provider::ModelInfo {
            id: "neunode/llama-3b-medical-v2".to_string(),
            base_model: Some("neunode/llama-3b-v1".to_string()),
            context_length: 8192,
            input_price_per_million: TokenAmount(150),
            output_price_per_million: TokenAmount(300),
            capabilities: vec!["chat".to_string(), "streaming".to_string(), "medical".to_string()],
        },
        neunode_inference::provider::ModelInfo {
            id: "neunode/mistral-7b-v1".to_string(),
            base_model: Some("mistralai/Mistral-7B-v0.3".to_string()),
            context_length: 32768,
            input_price_per_million: TokenAmount(200),
            output_price_per_million: TokenAmount(400),
            capabilities: vec!["chat".to_string(), "streaming".to_string(), "code".to_string()],
        },
    ]
}
