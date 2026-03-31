use anyhow::Result;
use neunode_storage::db::NeunodeDb;
use serde::{Deserialize, Serialize};

use crate::cli::{GlobalArgs, TrainCommands};
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

pub fn execute(cmd: &TrainCommands, args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(args.output);
    match cmd {
        TrainCommands::Start { model, dataset, config } => {
            train_start(model, dataset, config.as_deref(), &writer, state)
        }
        TrainCommands::Status { job_id } => train_status(job_id.as_deref(), &writer, state),
        TrainCommands::Stop { job_id } => train_stop(job_id, &writer, state),
        TrainCommands::List => train_list(&writer, state),
        TrainCommands::WorkerRegister { gpu_count, gpu_memory, max_params, bf16 } => {
            worker_register(*gpu_count, *gpu_memory, *max_params, *bf16, &writer, state)
        }
        TrainCommands::WorkerList { min_gpu, min_memory } => {
            worker_list(min_gpu, min_memory, &writer, state)
        }
        TrainCommands::CoordinatorStatus { job_id } => coordinator_status(job_id, &writer, state),
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
// Worker info
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WorkerInfo {
    worker_id: String,
    gpu_count: u32,
    gpu_memory_gb: f64,
    max_model_params: u64,
    supports_bf16: bool,
    status: String,
    registered_at: u64,
}

fn worker_db_key(worker_id: &str) -> String {
    format!("worker:{worker_id}")
}

fn store_worker(db: &NeunodeDb, worker: &WorkerInfo) -> Result<()> {
    let key_bytes = bincode::serialize(&worker_db_key(&worker.worker_id))
        .map_err(|e| anyhow::anyhow!("key: {e}"))?;
    let value = bincode::serialize(worker).map_err(|e| anyhow::anyhow!("serialize worker: {e}"))?;
    db.put_raw(neunode_storage::cf::CF_TRAINING, &key_bytes, &value)?;
    Ok(())
}

fn load_all_workers(db: &NeunodeDb) -> Vec<WorkerInfo> {
    let entries = match db.prefix_scan(neunode_storage::cf::CF_TRAINING, &[]) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .iter()
        .filter(|(k, _)| {
            let key_str = bincode::deserialize::<String>(k).ok().unwrap_or_default();
            key_str.starts_with("worker:")
        })
        .filter_map(|(_, v)| bincode::deserialize::<WorkerInfo>(v).ok())
        .collect()
}

#[allow(dead_code)]
fn load_worker(db: &NeunodeDb, worker_id: &str) -> Result<Option<WorkerInfo>> {
    let key_bytes =
        bincode::serialize(&worker_db_key(worker_id)).map_err(|e| anyhow::anyhow!("key: {e}"))?;
    match db.get_raw(neunode_storage::cf::CF_TRAINING, &key_bytes)? {
        Some(bytes) => {
            let worker: WorkerInfo =
                bincode::deserialize(&bytes).map_err(|e| anyhow::anyhow!("deserialize: {e}"))?;
            Ok(Some(worker))
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

    let pb = indicatif::ProgressBar::new_spinner();
    pb.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| indicatif::ProgressStyle::default_spinner()),
    );
    pb.set_message(format!("Queuing training job for {model}..."));

    store_job(state.db(), &job)?;

    pb.finish_with_message(format!("Training job {job_id} queued"));

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

fn worker_register(
    gpu_count: u32,
    gpu_memory: f64,
    max_params: u64,
    bf16: bool,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let entropy = format!(
        "worker{}{}{}{}{}",
        now.as_nanos(),
        gpu_count,
        gpu_memory.to_bits(),
        max_params,
        bf16
    );
    let random_bytes = neunode_crypto::hash::sha256(entropy.as_bytes());
    let worker_id = format!("worker_{}", bytes_to_hex(&random_bytes[..8]));

    let worker = WorkerInfo {
        worker_id: worker_id.clone(),
        gpu_count,
        gpu_memory_gb: gpu_memory,
        max_model_params: max_params,
        supports_bf16: bf16,
        status: "available".to_string(),
        registered_at: now.as_secs(),
    };

    let pb = indicatif::ProgressBar::new_spinner();
    pb.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| indicatif::ProgressStyle::default_spinner()),
    );
    pb.set_message("Registering training worker...");

    store_worker(state.db(), &worker)?;

    pb.finish_with_message(format!("Worker {worker_id} registered"));

    let pairs = [
        ("Worker ID", worker_id.as_str()),
        ("GPUs", &gpu_count.to_string()),
        ("GPU Memory (GB)", &gpu_memory.to_string()),
        ("Max Params", &max_params.to_string()),
        ("BF16", &bf16.to_string()),
        ("Status", "available"),
    ];
    writer.write_key_value_pairs(&pairs);
    writer.write_status(&format!("Worker {worker_id} registered"));
    Ok(())
}

fn worker_list(
    min_gpu: &Option<u32>,
    min_memory: &Option<f64>,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    let mut workers = load_all_workers(state.db());

    if let Some(min) = min_gpu {
        workers.retain(|w| w.gpu_count >= *min);
    }
    if let Some(min) = min_memory {
        workers.retain(|w| w.gpu_memory_gb >= *min);
    }

    if workers.is_empty() {
        writer.write_status("No training workers found");
        return Ok(());
    }

    let headers = ["Worker ID", "GPUs", "Memory (GB)", "BF16", "Status"];
    let rows: Vec<Vec<String>> = workers
        .iter()
        .map(|w| {
            vec![
                w.worker_id.clone(),
                w.gpu_count.to_string(),
                w.gpu_memory_gb.to_string(),
                w.supports_bf16.to_string(),
                w.status.clone(),
            ]
        })
        .collect();
    writer.write_table(&headers, &rows);
    Ok(())
}

fn coordinator_status(job_id: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    match load_job(state.db(), job_id)? {
        Some(job) => {
            let workers = load_all_workers(state.db());
            let active_workers = workers.iter().filter(|w| w.status == "available").count();

            let out = serde_json::json!({
                "job_id": job.job_id,
                "job_status": job.status,
                "model": job.model,
                "method": job.method,
                "coordinator": {
                    "active_workers": active_workers,
                    "outer_step": 0,
                    "global_step": 0,
                    "phase": match job.status.as_str() {
                        "running" => "training",
                        "queued" => "scheduling",
                        "stopped" => "stopped",
                        _ => "unknown",
                    },
                },
            });
            writer.write_json(&out);
            Ok(())
        }
        None => {
            anyhow::bail!("training job not found: {job_id}");
        }
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{human_writer, json_writer as test_writer, test_state};

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

    // --- Worker tests ---

    #[test]
    fn worker_register_stores_in_db() {
        let state = test_state();
        let writer = test_writer();
        worker_register(4, 80.0, 7_000_000_000, true, &writer, &state).unwrap();

        let workers = load_all_workers(state.db());
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].gpu_count, 4);
        assert_eq!(workers[0].gpu_memory_gb, 80.0);
        assert_eq!(workers[0].max_model_params, 7_000_000_000);
        assert!(workers[0].supports_bf16);
        assert_eq!(workers[0].status, "available");
    }

    #[test]
    fn worker_register_with_all_fields() {
        let state = test_state();
        let writer = human_writer();
        worker_register(8, 160.0, 70_000_000_000, false, &writer, &state).unwrap();

        let workers = load_all_workers(state.db());
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].gpu_count, 8);
        assert_eq!(workers[0].gpu_memory_gb, 160.0);
        assert_eq!(workers[0].max_model_params, 70_000_000_000);
        assert!(!workers[0].supports_bf16);
        assert!(workers[0].worker_id.starts_with("worker_"));
    }

    #[test]
    fn worker_list_empty() {
        let state = test_state();
        let writer = human_writer();
        worker_list(&None, &None, &writer, &state).unwrap();
    }

    #[test]
    fn worker_list_with_workers() {
        let state = test_state();
        let writer = test_writer();
        worker_register(4, 80.0, 7_000_000_000, true, &writer, &state).unwrap();
        worker_register(2, 40.0, 3_000_000_000, false, &writer, &state).unwrap();

        let writer2 = human_writer();
        worker_list(&None, &None, &writer2, &state).unwrap();

        let workers = load_all_workers(state.db());
        assert_eq!(workers.len(), 2);
    }

    #[test]
    fn worker_list_filters_by_min_gpu() {
        let state = test_state();
        let writer = test_writer();
        worker_register(4, 80.0, 7_000_000_000, true, &writer, &state).unwrap();
        worker_register(2, 40.0, 3_000_000_000, false, &writer, &state).unwrap();

        let filtered = {
            let mut w = load_all_workers(state.db());
            w.retain(|w| w.gpu_count >= 4);
            w
        };
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].gpu_count, 4);
    }

    #[test]
    fn worker_list_filters_by_min_memory() {
        let state = test_state();
        let writer = test_writer();
        worker_register(4, 80.0, 7_000_000_000, true, &writer, &state).unwrap();
        worker_register(2, 40.0, 3_000_000_000, false, &writer, &state).unwrap();

        let filtered = {
            let mut w = load_all_workers(state.db());
            w.retain(|w| w.gpu_memory_gb >= 50.0);
            w
        };
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].gpu_memory_gb, 80.0);
    }

    #[test]
    fn worker_list_combined_filters() {
        let state = test_state();
        let writer = test_writer();
        worker_register(8, 160.0, 70_000_000_000, true, &writer, &state).unwrap();
        worker_register(4, 80.0, 7_000_000_000, true, &writer, &state).unwrap();
        worker_register(2, 24.0, 1_000_000_000, false, &writer, &state).unwrap();

        let filtered = {
            let mut w = load_all_workers(state.db());
            w.retain(|w| w.gpu_count >= 4 && w.gpu_memory_gb >= 50.0);
            w
        };
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn store_and_load_worker() {
        let state = test_state();
        let worker = WorkerInfo {
            worker_id: "worker_abcd1234".to_string(),
            gpu_count: 8,
            gpu_memory_gb: 160.0,
            max_model_params: 70_000_000_000,
            supports_bf16: true,
            status: "available".to_string(),
            registered_at: 1000000,
        };

        store_worker(state.db(), &worker).unwrap();
        let loaded = load_worker(state.db(), "worker_abcd1234").unwrap().unwrap();
        assert_eq!(loaded, worker);
    }

    #[test]
    fn load_worker_not_found() {
        let state = test_state();
        let result = load_worker(state.db(), "worker_nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn coordinator_status_found() {
        let state = test_state();
        let writer = test_writer();
        train_start("llama-3b", "dataset", None, &writer, &state).unwrap();
        let jobs = load_all_jobs(state.db());
        let job_id = jobs[0].job_id.clone();

        let writer2 = test_writer();
        coordinator_status(&job_id, &writer2, &state).unwrap();
    }

    #[test]
    fn coordinator_status_not_found() {
        let state = test_state();
        let writer = test_writer();
        assert!(coordinator_status("train_nonexistent", &writer, &state).is_err());
    }

    #[test]
    fn coordinator_status_with_workers() {
        let state = test_state();
        let writer = test_writer();
        train_start("llama-3b", "dataset", None, &writer, &state).unwrap();
        worker_register(4, 80.0, 7_000_000_000, true, &writer, &state).unwrap();

        let jobs = load_all_jobs(state.db());
        let job_id = jobs[0].job_id.clone();

        let writer2 = human_writer();
        coordinator_status(&job_id, &writer2, &state).unwrap();
    }

    #[test]
    fn worker_register_generates_unique_ids() {
        let state = test_state();
        let writer = test_writer();
        worker_register(4, 80.0, 7_000_000_000, true, &writer, &state).unwrap();
        worker_register(4, 80.0, 7_000_000_000, true, &writer, &state).unwrap();

        let workers = load_all_workers(state.db());
        assert_eq!(workers.len(), 2);
        assert_ne!(workers[0].worker_id, workers[1].worker_id);
    }

    #[test]
    fn workers_and_jobs_coexist() {
        let state = test_state();
        let writer = test_writer();
        train_start("llama-3b", "dataset", None, &writer, &state).unwrap();
        worker_register(4, 80.0, 7_000_000_000, true, &writer, &state).unwrap();

        let jobs = load_all_jobs(state.db());
        let workers = load_all_workers(state.db());
        assert_eq!(jobs.len(), 1);
        assert_eq!(workers.len(), 1);
    }
}
