use anyhow::Result;

use crate::cli::{Cli, TrainCommands};
use crate::config::CliConfig;
use crate::output::OutputWriter;

pub fn execute(cmd: &TrainCommands, cli: &Cli, _config: &mut CliConfig) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        TrainCommands::Start { model, dataset, config } => {
            train_start(model, dataset, config.as_deref(), &writer)
        }
        TrainCommands::Status { job_id } => train_status(job_id.as_deref(), &writer),
        TrainCommands::Stop { job_id } => train_stop(job_id, &writer),
        TrainCommands::List => train_list(&writer),
    }
}

fn train_start(
    model: &str,
    dataset: &str,
    config: Option<&str>,
    writer: &OutputWriter,
) -> Result<()> {
    let job_id = format!(
        "train_{}",
        bytes_to_hex(&neunode_crypto::hash::sha256(format!("{model}{dataset}").as_bytes(),)[..8])
    );

    let config_display = config.unwrap_or("{}").to_string();
    let status = "queued".to_string();
    let method = "DiLoCo + SWARM".to_string();
    let model_owned = model.to_string();
    let dataset_owned = dataset.to_string();

    let pairs = [
        ("Job ID", job_id.as_str()),
        ("Model", model_owned.as_str()),
        ("Dataset", dataset_owned.as_str()),
        ("Config", config_display.as_str()),
        ("Status", status.as_str()),
        ("Method", method.as_str()),
    ];
    writer.write_key_value_pairs(&pairs);
    writer.write_status(&format!("Training job {job_id} queued"));
    writer.write_warning("Training not yet available in Phase 1 MVP — dry run");
    Ok(())
}

fn train_status(job_id: Option<&str>, writer: &OutputWriter) -> Result<()> {
    match job_id {
        Some(id) => {
            let info = serde_json::json!({
                "job_id": id,
                "status": "not_found",
                "note": "Phase 1 MVP — training status not yet connected",
            });
            writer.write_json(&info);
        }
        None => {
            writer.write_status("No active training jobs (Phase 1 MVP)");
        }
    }
    Ok(())
}

fn train_stop(job_id: &str, writer: &OutputWriter) -> Result<()> {
    writer.write_status(&format!("Training job {job_id} stopped"));
    writer.write_warning("Training stop not yet available in Phase 1 MVP — dry run");
    let info = serde_json::json!({
        "job_id": job_id,
        "action": "stop",
        "status": "dry_run",
    });
    writer.write_json(&info);
    Ok(())
}

fn train_list(writer: &OutputWriter) -> Result<()> {
    let headers = ["Job ID", "Model", "Dataset", "Status", "Progress"];
    let rows: Vec<Vec<String>> = Vec::new();
    writer.write_table(&headers, &rows);
    writer.write_status("No training jobs (Phase 1 MVP)");
    Ok(())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
