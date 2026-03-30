use anyhow::Result;

use crate::cli::{Cli, MeshCommands};
use crate::config::CliConfig;
use crate::output::OutputWriter;

pub fn execute(cmd: &MeshCommands, cli: &Cli, _config: &mut CliConfig) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        MeshCommands::Start { bootstrap, listen } => mesh_start(bootstrap, listen, &writer),
        MeshCommands::Status => mesh_status(&writer),
        MeshCommands::Peers { verbose } => mesh_peers(*verbose, &writer),
        MeshCommands::Connect { addr } => mesh_connect(addr, &writer),
        MeshCommands::Disconnect { peer_id } => mesh_disconnect(peer_id, &writer),
    }
}

fn mesh_start(bootstrap: &[String], listen: &str, writer: &OutputWriter) -> Result<()> {
    let bootstrap_display = if bootstrap.is_empty() {
        "none (standalone mode)".to_string()
    } else {
        bootstrap.join(", ")
    };

    let pairs = [
        ("Status", "starting"),
        ("Listen", listen),
        ("Bootstrap", &bootstrap_display),
        ("Mesh degree", "6"),
        ("Protocol", "libp2p (Gossipsub + KadDHT)"),
    ];
    writer.write_key_value_pairs(&pairs);
    writer.write_warning("P2P mesh is not yet started in Phase 1 MVP — this is a dry run");
    Ok(())
}

fn mesh_status(writer: &OutputWriter) -> Result<()> {
    let status = serde_json::json!({
        "running": false,
        "local_peer_id": null,
        "listen_addresses": [],
        "connected_peers": 0,
        "subscribed_topics": [],
        "dht_bootstrapped": false,
        "note": "Phase 1 MVP — mesh not yet started",
    });
    writer.write_json(&status);
    Ok(())
}

fn mesh_peers(verbose: bool, writer: &OutputWriter) -> Result<()> {
    if verbose {
        let info = serde_json::json!({
            "peers": [],
            "count": 0,
            "note": "Phase 1 MVP — no peers connected yet",
        });
        writer.write_json(&info);
    } else {
        let headers = ["Peer ID", "Address", "Latency", "Status"];
        let rows: Vec<Vec<String>> = Vec::new();
        writer.write_table(&headers, &rows);
        writer.write_status("No connected peers (Phase 1 MVP)");
    }
    Ok(())
}

fn mesh_connect(addr: &str, writer: &OutputWriter) -> Result<()> {
    writer.write_status(&format!("Would connect to {addr} (Phase 1 MVP — dry run)"));
    let info = serde_json::json!({
        "action": "connect",
        "address": addr,
        "status": "dry_run",
        "note": "Phase 1 MVP — connection not yet implemented",
    });
    writer.write_json(&info);
    Ok(())
}

fn mesh_disconnect(peer_id: &str, writer: &OutputWriter) -> Result<()> {
    writer.write_status(&format!("Would disconnect from {peer_id} (Phase 1 MVP — dry run)"));
    let info = serde_json::json!({
        "action": "disconnect",
        "peer_id": peer_id,
        "status": "dry_run",
    });
    writer.write_json(&info);
    Ok(())
}
