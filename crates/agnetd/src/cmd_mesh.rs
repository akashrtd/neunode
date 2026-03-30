use anyhow::Result;
use neunode_identity::did::did_to_peer_id;

use crate::cli::{Cli, MeshCommands};
use crate::output::OutputWriter;
use crate::state::AppState;

pub async fn execute(cmd: &MeshCommands, cli: &Cli, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        MeshCommands::Start { bootstrap, listen } => {
            mesh_start(bootstrap, listen, &writer, state).await
        }
        MeshCommands::Status => mesh_status(&writer, state),
        MeshCommands::Peers { verbose } => mesh_peers(*verbose, &writer, state),
        MeshCommands::Connect { addr } => mesh_connect(addr, &writer, state),
        MeshCommands::Disconnect { peer_id } => mesh_disconnect(peer_id, &writer),
    }
}

async fn mesh_start(
    bootstrap: &[String],
    listen: &str,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    let keyring = state.require_keyring()?;

    let (ed_bytes, _) = keyring.to_bytes();
    let ed_bytes_fixed: [u8; 32] = ed_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid ed25519 key length"))?;
    let libp2p_keypair = libp2p::identity::Keypair::ed25519_from_bytes(ed_bytes_fixed)
        .map_err(|e| anyhow::anyhow!("failed to create libp2p keypair: {e}"))?;

    let listen_addr: libp2p::Multiaddr =
        listen.parse().map_err(|e| anyhow::anyhow!("invalid listen address '{listen}': {e}"))?;

    let mut node = neunode_p2p::node::P2pNode::new(libp2p_keypair, listen_addr.clone())?;
    node.start(listen_addr.clone())?;

    for addr_str in bootstrap {
        let addr: libp2p::Multiaddr = addr_str
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid bootstrap address '{addr_str}': {e}"))?;
        node.add_bootstrap_peer(addr);
    }

    node.subscribe_all_categories()?;

    if !bootstrap.is_empty() {
        node.bootstrap_dht()?;
    }

    let peer_id = node.local_peer_id().to_string();
    let listeners: Vec<String> = node.listeners().map(|a| a.to_string()).collect();
    let connected = node.connected_peers();
    let topics: Vec<&String> = node.subscribed_topics().iter().collect();

    let bootstrap_display = if bootstrap.is_empty() {
        "none (standalone mode)".to_string()
    } else {
        bootstrap.join(", ")
    };

    let pairs = [
        ("Status", "running"),
        ("Peer ID", peer_id.as_str()),
        ("Listen", listen),
        ("Listeners", &listeners.join(", ")),
        ("Bootstrap", &bootstrap_display),
        ("Connected Peers", &connected.len().to_string()),
        ("Subscribed Topics", &topics.len().to_string()),
        ("DHT Bootstrapped", &(!bootstrap.is_empty()).to_string()),
    ];
    writer.write_key_value_pairs(&pairs);
    writer.write_status("P2P mesh started — press Ctrl+C to stop");

    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            _ = &mut ctrl_c => {
                writer.write_status("Shutting down P2P mesh...");
                break;
            }
            event = node.next_event() => {
                match &event {
                    neunode_p2p::node::NodeEvent::PeerConnected(pid) => {
                        writer.write_status(&format!("Peer connected: {pid}"));
                    }
                    neunode_p2p::node::NodeEvent::PeerDisconnected(pid) => {
                        writer.write_status(&format!("Peer disconnected: {pid}"));
                    }
                    neunode_p2p::node::NodeEvent::GossipsubMessage { source, topic, .. } => {
                        let src = source.map(|p| p.to_string()).unwrap_or_default();
                        writer.write_status(&format!("Message on {topic} from {src}"));
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn mesh_status(writer: &OutputWriter, state: &AppState) -> Result<()> {
    let did_key = state.require_did().ok().and_then(|_| {
        let keyring = state.require_keyring().ok()?;
        let peer_id = did_to_peer_id(&keyring.to_did_key()).ok()?;
        Some(peer_id.to_string())
    });

    let status = serde_json::json!({
        "running": false,
        "local_peer_id": did_key,
        "listen_addresses": [],
        "connected_peers": 0,
        "subscribed_topics": [],
        "dht_bootstrapped": false,
        "note": "mesh not running — use 'mesh start' to begin",
    });
    writer.write_json(&status);
    Ok(())
}

fn mesh_peers(verbose: bool, writer: &OutputWriter, _state: &AppState) -> Result<()> {
    if verbose {
        let info = serde_json::json!({
            "peers": [],
            "count": 0,
            "note": "no peers — mesh not running",
        });
        writer.write_json(&info);
    } else {
        let headers = ["Peer ID", "Address", "Latency", "Status"];
        let rows: Vec<Vec<String>> = Vec::new();
        writer.write_table(&headers, &rows);
        writer.write_status("No connected peers — mesh not running");
    }
    Ok(())
}

fn mesh_connect(addr: &str, writer: &OutputWriter, _state: &AppState) -> Result<()> {
    let _parsed: libp2p::Multiaddr =
        addr.parse().map_err(|e| anyhow::anyhow!("invalid multiaddr '{addr}': {e}"))?;

    writer.write_status(&format!("Connect to {addr} — start mesh first with 'mesh start'"));
    let info = serde_json::json!({
        "action": "connect",
        "address": addr,
        "status": "mesh_not_running",
    });
    writer.write_json(&info);
    Ok(())
}

fn mesh_disconnect(peer_id: &str, writer: &OutputWriter) -> Result<()> {
    writer.write_status(&format!("Disconnect from {peer_id} — start mesh first with 'mesh start'"));
    let info = serde_json::json!({
        "action": "disconnect",
        "peer_id": peer_id,
        "status": "mesh_not_running",
    });
    writer.write_json(&info);
    Ok(())
}
