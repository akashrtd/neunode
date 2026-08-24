//! Chain mode integration for `agnetd serve --chain-mode sovereign`.
//!
//! When sovereign mode is enabled, this module:
//! 1. Writes the genesis JSON to a temp file
//! 2. Generates a JWT secret for the Engine API
//! 3. Spawns Reth as a child process
//! 4. Starts the consensus bridge driver in a background task

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{bail, Context, Result};
use tokio::process::{Child, Command};
use tracing::{error, info, warn};

use neunode_consensus_bridge::{MalachiteEvent, MalachiteHandler, MalachiteResponse};
use neunode_engine_api_client::EngineApiClientConfig;

/// Handle to spawned chain processes.
pub struct ChainHandle {
    reth: Option<Child>,
    bridge: Option<tokio::task::JoinHandle<()>>,
    consensus: Option<Child>,
}

impl ChainHandle {
    /// Shut down spawned processes.
    pub fn shutdown(&mut self) {
        if let Some(bridge) = &self.bridge {
            bridge.abort();
        }
        if let Some(ref mut child) = self.consensus {
            let _ = child.start_kill();
            warn!("Malachite process terminated");
        }
        if let Some(ref mut child) = self.reth {
            let _ = child.start_kill();
            warn!("Reth process terminated");
        }
    }
}

impl Drop for ChainHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Start the sovereign L1 chain: spawn Reth + consensus bridge.
///
/// Returns a handle that keeps the spawned processes alive. When the handle
/// is dropped, processes are terminated.
pub async fn start_sovereign_chain(config: crate::cli::ChainModeConfig) -> Result<ChainHandle> {
    // Step 1: Write genesis JSON to temp file.
    let genesis_path = write_genesis_file()?;
    info!(path = %genesis_path.display(), "genesis file written");

    // Step 2: Resolve or generate JWT secret.
    let jwt_path = match &config.jwt_secret_path {
        Some(p) => PathBuf::from(p),
        None => generate_jwt_secret()?,
    };
    info!(path = %jwt_path.display(), "JWT secret ready");

    // Step 3: Resolve Reth binary path.
    let reth_bin = config.reth_path.as_deref().unwrap_or("reth");
    let reth_available = !config.external_engine && which_reth(reth_bin).await;
    if !reth_available && !config.external_engine {
        if config.reth_path.is_some() {
            bail!(
                "Reth binary not found at '{reth_bin}'. Install Reth or set --reth-path to the correct location."
            );
        }
        warn!("Reth binary '{reth_bin}' not found in PATH.");
        warn!("Chain mode requires Reth. Install it from https://paradigmxyz.github.io/reth/");
        warn!("Continuing without Reth - the consensus bridge will start but will not be able to connect to an EL.");
        warn!(
            "To manually start Reth: reth node --chain {} --authrpc.jwtsecret {}",
            genesis_path.display(),
            jwt_path.display()
        );
    }

    // Step 4: Spawn Reth (if available).
    let reth_child = if reth_available {
        Some(spawn_reth(reth_bin, &genesis_path, &jwt_path).await?)
    } else {
        None
    };

    // Step 5: Wait for Engine API to be ready.
    if reth_child.is_some() || config.external_engine {
        info!("Waiting for Reth Engine API to be ready...");
        if !wait_for_engine_api(&config.engine_api_endpoint, &jwt_path, 30).await {
            warn!("Engine API did not become ready within 30s. Starting bridge anyway.");
        } else {
            info!("Engine API is ready.");
        }
    }

    // Step 6: Start consensus bridge in background.
    let engine_config = EngineApiClientConfig {
        endpoint: config.engine_api_endpoint.clone(),
        jwt_secret_path: Some(jwt_path.to_path_buf()),
        jwt_secret: None,
        ..Default::default()
    };

    let wal_path = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .context("could not determine data dir")?
        .join("agnetd")
        .join("consensus-bridge.wal");
    let (bridge, consensus) = match config.consensus_mode {
        crate::cli::ConsensusMode::Single => {
            let block_time = config.block_time;
            let bridge = tokio::spawn(async move {
                if let Err(error) = run_consensus_events(engine_config, wal_path, block_time).await
                {
                    error!(%error, "consensus event bridge exited");
                }
            });
            (Some(bridge), None)
        }
        crate::cli::ConsensusMode::Malachite => {
            let binary = config.malachite_path.as_deref().context(
                "--malachite-path is required when --consensus-mode malachite is selected",
            )?;
            let home = config.malachite_home.as_deref().context(
                "--malachite-home is required when --consensus-mode malachite is selected",
            )?;
            let mut command = Command::new(binary);
            command
                .arg("--home")
                .arg(home)
                .arg("start")
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            if let Some(working_dir) = &config.malachite_working_dir {
                command.current_dir(working_dir);
            }
            let child = command
                .spawn()
                .with_context(|| format!("failed to start Malachite binary at {binary}"))?;
            info!(home, "Malachite BFT validator started");
            (None, Some(child))
        }
    };

    println!("{}  Chain mode: sovereign (Reth + consensus bridge)", console::style("INFO").dim());

    Ok(ChainHandle { reth: reth_child, bridge, consensus })
}

async fn run_consensus_events(
    engine_config: EngineApiClientConfig,
    wal_path: PathBuf,
    block_time: u64,
) -> Result<()> {
    let engine = neunode_engine_api_client::EngineApiClient::new(engine_config).await?;
    let handler = MalachiteHandler::open(engine, neunode_chain_spec::DEPLOYER_ADDRESS, wal_path)?;
    let (sender, receiver) = tokio::sync::mpsc::channel(32);
    tokio::spawn(async move {
        if let Err(error) = handler.run(receiver).await {
            error!(%error, "Malachite handler stopped");
        }
    });

    let (reply, response) = tokio::sync::oneshot::channel();
    sender.send(MalachiteEvent::ConsensusReady { reply }).await?;
    let MalachiteResponse::Ready { mut height } = response.await?? else {
        bail!("unexpected consensus-ready response")
    };
    info!(height, "consensus event bridge ready");

    loop {
        sender
            .send(MalachiteEvent::StartedRound {
                height,
                round: 0,
                proposer: "single-node-validator".to_string(),
            })
            .await?;
        let (reply, response) = tokio::sync::oneshot::channel();
        sender.send(MalachiteEvent::GetValue { height, round: 0, reply }).await?;
        let MalachiteResponse::Proposal(proposal) = response.await?? else {
            bail!("unexpected proposal response")
        };
        let (reply, response) = tokio::sync::oneshot::channel();
        sender
            .send(MalachiteEvent::ValidationRequest { height, round: 0, proposal, reply })
            .await?;
        if !matches!(response.await??, MalachiteResponse::Validity(true)) {
            bail!("execution layer rejected proposed block at height {height}")
        }
        let (reply, response) = tokio::sync::oneshot::channel();
        sender
            .send(MalachiteEvent::Decided {
                height,
                round: 0,
                certificate: format!("single-node:{height}").into_bytes(),
                reply,
            })
            .await?;
        let MalachiteResponse::Finalized { block_hash, .. } = response.await?? else {
            bail!("unexpected finalization response")
        };
        info!(height, %block_hash, "consensus decision finalized");
        height += 1;
        tokio::time::sleep(tokio::time::Duration::from_secs(block_time)).await;
    }
}

fn write_genesis_file() -> Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .context("could not determine local data directory")?
        .join("agnetd");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("genesis.json");
    std::fs::write(&path, neunode_chain_spec::neunode_genesis_json())
        .with_context(|| format!("failed to write genesis to {}", path.display()))?;
    Ok(path)
}

fn generate_jwt_secret() -> Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .context("could not determine local data directory")?
        .join("agnetd");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("jwt.hex");

    // Generate a random 32-byte secret using the OS RNG.
    let mut random_bytes = [0u8; 32];
    use std::io::Read;
    let mut f = std::fs::File::open("/dev/urandom").or_else(|_| std::fs::File::open("/dev/random"));
    match &mut f {
        Ok(file) => {
            file.read_exact(&mut random_bytes).context("failed to read random bytes")?;
        }
        Err(_) => {
            // Fallback for non-Unix systems: use system time + process info as entropy.
            // This is weaker than /dev/urandom but acceptable for devnet/testnet.
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            for (i, byte) in random_bytes.iter_mut().enumerate() {
                *byte = ((seed.wrapping_mul((i as u128 + 1) * 2654435761)) >> ((i % 16) * 4)) as u8;
            }
            warn!(
                "Using fallback entropy for JWT secret (no /dev/urandom). For devnet/testnet only."
            );
        }
    }
    let hex_secret = hex::encode(random_bytes);

    std::fs::write(&path, hex_secret)
        .with_context(|| format!("failed to write JWT secret to {}", path.display()))?;
    Ok(path)
}

async fn which_reth(reth_bin: &str) -> bool {
    std::process::Command::new(reth_bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn spawn_reth(reth_bin: &str, genesis_path: &Path, jwt_path: &Path) -> Result<Child> {
    let data_dir = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .context("could not determine data dir")?
        .join("agnetd")
        .join("reth");

    let child = Command::new(reth_bin)
        .arg("node")
        .arg("--chain")
        .arg(genesis_path)
        .arg("--authrpc.jwtsecret")
        .arg(jwt_path)
        .arg("--datadir")
        .arg(&data_dir)
        .arg("--http")
        .arg("--http.api")
        .arg("eth,net,web3,debug")
        .arg("--http.port")
        .arg("8545")
        .arg("--authrpc.port")
        .arg("8551")
        .arg("--dev") // Single-node mode in Reth itself
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("failed to spawn Reth process")?;

    info!(bin = reth_bin, datadir = %data_dir.display(), "Reth spawned");
    Ok(child)
}

async fn wait_for_engine_api(endpoint: &str, jwt_path: &Path, timeout_secs: u64) -> bool {
    let config = EngineApiClientConfig {
        endpoint: endpoint.to_string(),
        jwt_secret_path: Some(jwt_path.to_path_buf()),
        jwt_secret: None,
        ..Default::default()
    };

    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout_secs);

    loop {
        if tokio::time::Instant::now() > deadline {
            return false;
        }

        if let Ok(client) = neunode_engine_api_client::EngineApiClient::new(config.clone()).await {
            let methods = vec![
                "engine_newPayloadV3".to_string(),
                "engine_forkchoiceUpdatedV3".to_string(),
                "engine_getPayloadV3".to_string(),
            ];
            if client.exchange_capabilities(methods).await.is_ok() {
                return true;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
