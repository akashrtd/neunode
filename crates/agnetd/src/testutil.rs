use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crate::cli::OutputFormat;
use crate::config::CliConfig;
use crate::output::OutputWriter;
use crate::state::AppState;
use neunode_identity::keyring::Keyring;

pub fn json_writer() -> OutputWriter {
    OutputWriter::new(OutputFormat::Json)
}

pub fn human_writer() -> OutputWriter {
    OutputWriter::new(OutputFormat::Human)
}

pub fn test_state() -> AppState {
    static TEST_ID: AtomicU64 = AtomicU64::new(0);
    let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("agnetd_test_{}_{}", std::process::id(), id));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db = neunode_storage::db::NeunodeDb::open(&dir).unwrap();

    let kr = Keyring::generate();
    let did = kr.to_did();

    let config_path = dir.join("config.toml");

    AppState {
        db: Arc::new(db),
        config: CliConfig::load(config_path.to_str()).unwrap(),
        active_keyring: Some(kr),
        active_did: Some(did),
        mesh_handle: None,
    }
}
