use anyhow::Result;
use neunode_storage::db::NeunodeDb;
use serde::{Deserialize, Serialize};

use crate::cli::{Cli, TrainCommands};
use crate::output::OutputWriter;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TrainingJob {
    job_id: String,
    model: String,
    dataset: String,
    config: String,
    status: String,
    created_at: u64,
    method: String,
}

pub fn execute(cmd: &TrainCommands, cli: &Cli, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        TrainCommands::Start { model, dataset, config } => {
            train_start(model, dataset, config.as_deref(), &writer, state)
        }
        TrainCommands::Status { job_id } => train_status(job_id.as_deref(), &writer, state),
        TrainCommands::Stop { job_id } => train_stop(job_id, &writer, state),
        TrainCommands::List => train_list(&writer, state),
    }
}

// ---------------------------------------------------------------------------
// Training job DB helpers
// ---------------------------------------------------------------------------

fn train_db_key(job_id: &str) -> String {
    format!("job:{job_id}")
}

fn store_job(db: &NeunodeDb, job: &TrainingJob) -> Result<()> {
    let key_bytes =
        bincode::serialize(&train_db_key(&job.job_id)).map_err(|e| anyhow::anyhow!("key: {e}"))?;
    let value = bincode::serialize(job).map_err(|e| anyhow::anyhow!("serialize job: {e}"))?;
    db.put_raw(neunode_storage::cf::CF_TRAINING, &key_bytes, &value)?;
    Ok(())
}

fn load_all_jobs(db: &NeunodeDb) -> Vec<TrainingJob> {
    let entries = match db.prefix_scan(neunode_storage::cf::CF_TRAINING, &[]) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .iter()
        .filter(|(k, _)| {
            let key_str = bincode::deserialize::<String>(k).ok().unwrap_or_default();
            key_str.starts_with("job:")
        })
        .filter_map(|(_, v)| bincode::deserialize::<TrainingJob>(v).ok())
        .collect()
}

fn load_job(db: &NeunodeDb, job_id: &str) -> Result<Option<TrainingJob>> {
    let key_bytes =
        bincode::serialize(&train_db_key(job_id)).map_err(|e| anyhow::anyhow!("key: {e}"))?;
    match db.get_raw(neunode_storage::cf::CF_TRAINING, &key_bytes)? {
        Some(bytes) => {
            let job: TrainingJob =
                bincode::deserialize(&bytes).map_err(|e| anyhow::anyhow!("deserialize: {e}"))?;
            Ok(Some(job))
        }
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Command implementations
// ---------------------------------------------------------------------------

fn train_start(
    model: &str,
    dataset: &str,
    config: Option<&str>,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let hash_input = format!("{model}{dataset}{now}");
    let hash = neunode_crypto::hash::sha256(hash_input.as_bytes());
    let job_id = format!("train_{}", bytes_to_hex(&hash[..8]));

    let job = TrainingJob {
        job_id: job_id.clone(),
        model: model.to_string(),
        dataset: dataset.to_string(),
        config: config.unwrap_or("{}").to_string(),
        status: "queued".to_string(),
        created_at: now,
        method: "DiLoCo + SWARM".to_string(),
    };

    store_job(state.db(), &job)?;

    let pairs = [
        ("Job ID", job_id.as_str()),
        ("Model", model),
        ("Dataset", dataset),
        ("Config", job.config.as_str()),
        ("Status", "queued"),
        ("Method", "DiLoCo + SWARM"),
    ];
    writer.write_key_value_pairs(&pairs);
    writer.write_status(&format!("Training job {job_id} queued"));
    Ok(())
}

fn train_status(job_id: Option<&str>, writer: &OutputWriter, state: &AppState) -> Result<()> {
    match job_id {
        Some(id) => match load_job(state.db(), id)? {
            Some(job) => {
                let out = serde_json::json!({
                    "job_id": job.job_id,
                    "model": job.model,
                    "dataset": job.dataset,
                    "status": job.status,
                    "created_at": job.created_at,
                    "method": job.method,
                });
                writer.write_json(&out);
            }
            None => {
                let info = serde_json::json!({
                    "job_id": id,
                    "status": "not_found",
                });
                writer.write_json(&info);
                anyhow::bail!("training job not found: {id}");
            }
        },
        None => {
            let jobs = load_all_jobs(state.db());
            if jobs.is_empty() {
                writer.write_status("No training jobs found");
            } else {
                for job in &jobs {
                    let pairs = [
                        ("Job ID", job.job_id.as_str()),
                        ("Model", job.model.as_str()),
                        ("Status", job.status.as_str()),
                    ];
                    writer.write_key_value_pairs(&pairs);
                }
            }
        }
    }
    Ok(())
}

fn train_stop(job_id: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    match load_job(state.db(), job_id)? {
        Some(mut job) => {
            job.status = "stopped".to_string();
            store_job(state.db(), &job)?;

            writer.write_status(&format!("Training job {job_id} stopped"));
            let info = serde_json::json!({
                "job_id": job_id,
                "action": "stop",
                "status": "stopped",
            });
            writer.write_json(&info);
            Ok(())
        }
        None => {
            writer.write_error(&format!("training job not found: {job_id}"));
            anyhow::bail!("training job not found: {job_id}");
        }
    }
}

fn train_list(writer: &OutputWriter, state: &AppState) -> Result<()> {
    let jobs = load_all_jobs(state.db());

    if jobs.is_empty() {
        writer.write_status("No training jobs found");
        return Ok(());
    }

    let headers = ["Job ID", "Model", "Dataset", "Status", "Method"];
    let rows: Vec<Vec<String>> = jobs
        .iter()
        .map(|j| {
            vec![
                j.job_id.clone(),
                j.model.clone(),
                j.dataset.clone(),
                j.status.clone(),
                j.method.clone(),
            ]
        })
        .collect();
    writer.write_table(&headers, &rows);
    Ok(())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::OutputFormat;
    use crate::config::CliConfig;
    use crate::state::AppState;
    use neunode_identity::keyring::Keyring;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    };

    fn test_writer() -> OutputWriter {
        OutputWriter::new(OutputFormat::Json)
    }

    fn human_writer() -> OutputWriter {
        OutputWriter::new(OutputFormat::Human)
    }

    fn test_state() -> AppState {
        static TEST_ID: AtomicU64 = AtomicU64::new(0);
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("agnetd_test_train_{:?}_{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = neunode_storage::db::NeunodeDb::open(&dir).unwrap();

        let kr = Keyring::generate();
        let did = kr.to_did();

        AppState {
            db: Arc::new(db),
            config: CliConfig::load(None).unwrap(),
            active_keyring: Some(kr),
            active_did: Some(did),
            mesh_handle: None,
        }
    }

    #[test]
    fn start_creates_job() {
        let state = test_state();
        let writer = test_writer();
        train_start("llama-3b", "bafkrei123", None, &writer, &state).unwrap();
        let jobs = load_all_jobs(state.db());
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].model, "llama-3b");
        assert_eq!(jobs[0].status, "queued");
    }

    #[test]
    fn start_with_config() {
        let state = test_state();
        let writer = human_writer();
        train_start("llama-3b", "dataset-cid", Some("{\"lr\":0.001}"), &writer, &state).unwrap();
        let jobs = load_all_jobs(state.db());
        assert_eq!(jobs[0].config, "{\"lr\":0.001}");
    }

    #[test]
    fn status_found() {
        let state = test_state();
        let writer = test_writer();
        train_start("llama-3b", "dataset", None, &writer, &state).unwrap();
        let jobs = load_all_jobs(state.db());
        let job_id = jobs[0].job_id.clone();

        let writer2 = human_writer();
        train_status(Some(&job_id), &writer2, &state).unwrap();
    }

    #[test]
    fn status_not_found() {
        let state = test_state();
        let writer = test_writer();
        assert!(train_status(Some("train_nonexistent"), &writer, &state).is_err());
    }

    #[test]
    fn status_no_id_shows_all() {
        let state = test_state();
        let writer = human_writer();
        train_status(None, &writer, &state).unwrap();
    }

    #[test]
    fn stop_updates_status() {
        let state = test_state();
        let writer = test_writer();
        train_start("llama-3b", "dataset", None, &writer, &state).unwrap();
        let jobs = load_all_jobs(state.db());
        let job_id = jobs[0].job_id.clone();

        let writer2 = test_writer();
        train_stop(&job_id, &writer2, &state).unwrap();

        let updated = load_job(state.db(), &job_id).unwrap().unwrap();
        assert_eq!(updated.status, "stopped");
    }

    #[test]
    fn stop_not_found() {
        let state = test_state();
        let writer = test_writer();
        assert!(train_stop("train_nonexistent", &writer, &state).is_err());
    }

    #[test]
    fn list_empty() {
        let state = test_state();
        let writer = human_writer();
        train_list(&writer, &state).unwrap();
    }

    #[test]
    fn list_with_jobs() {
        let state = test_state();
        let writer = test_writer();
        train_start("model-a", "ds-a", None, &writer, &state).unwrap();
        train_start("model-b", "ds-b", None, &writer, &state).unwrap();

        let writer2 = human_writer();
        train_list(&writer2, &state).unwrap();

        let jobs = load_all_jobs(state.db());
        assert_eq!(jobs.len(), 2);
    }

    #[test]
    fn store_and_load_job() {
        let state = test_state();
        let job = TrainingJob {
            job_id: "train_abcd1234".to_string(),
            model: "llama-3b".to_string(),
            dataset: "bafkrei5678".to_string(),
            config: "{}".to_string(),
            status: "queued".to_string(),
            created_at: 1000000,
            method: "DiLoCo + SWARM".to_string(),
        };

        store_job(state.db(), &job).unwrap();
        let loaded = load_job(state.db(), "train_abcd1234").unwrap().unwrap();
        assert_eq!(loaded, job);
    }
}
