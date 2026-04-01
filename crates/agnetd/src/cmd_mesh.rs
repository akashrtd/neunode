use anyhow::Result;
use neunode_core::kind::Kind;
use neunode_core::types::Hash256;
use neunode_feed::event::FeedEvent;
use neunode_identity::did::did_to_peer_id;

use crate::cli::{GlobalArgs, MeshCommands};
use crate::output::OutputWriter;
use crate::state::AppState;

pub async fn execute(cmd: &MeshCommands, args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(args.output);
    match cmd {
        MeshCommands::Start { bootstrap, listen } => {
            mesh_start(bootstrap, listen, &writer, state).await
        }
        MeshCommands::Status => mesh_status(&writer, state),
        MeshCommands::Peers { verbose } => mesh_peers(*verbose, &writer, state),
        MeshCommands::Connect { addr } => mesh_connect(addr, &writer, state),
        MeshCommands::Disconnect { peer_id } => mesh_disconnect(peer_id, &writer, state),
    }
}

// ---------------------------------------------------------------------------
// mesh start — spawn background task, enter interactive stdin loop
// ---------------------------------------------------------------------------

async fn mesh_start(
    bootstrap: &[String],
    listen: &str,
    writer: &OutputWriter,
    state: &mut AppState,
) -> Result<()> {
    let keyring = state.require_keyring()?;

    // Convert ed25519 private key bytes → libp2p Keypair.
    // libp2p ed25519_from_bytes takes a 32-byte seed (private key only).
    let (ed_bytes, _) = keyring.to_bytes();
    let ed_bytes_fixed: [u8; 32] = ed_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid ed25519 key length"))?;
    let libp2p_keypair = libp2p::identity::Keypair::ed25519_from_bytes(ed_bytes_fixed)
        .map_err(|e| anyhow::anyhow!("failed to create libp2p keypair: {e}"))?;

    let listen_addr: libp2p::Multiaddr =
        listen.parse().map_err(|e| anyhow::anyhow!("invalid listen address '{listen}': {e}"))?;

    let bootstrap_addrs: Vec<libp2p::Multiaddr> = bootstrap
        .iter()
        .map(|a| {
            a.parse::<libp2p::Multiaddr>()
                .map_err(|e| anyhow::anyhow!("invalid bootstrap address '{a}': {e}"))
        })
        .collect::<Result<Vec<_>>>()?;

    let subscribe_all = true;
    let handle = crate::mesh_handle::spawn_mesh_task(
        libp2p_keypair,
        listen_addr,
        bootstrap_addrs,
        subscribe_all,
        state.db().clone(),
    )?;

    let peer_id = handle.local_peer_id.to_string();
    state.set_mesh_handle(handle);

    let bootstrap_display = if bootstrap.is_empty() {
        "none (standalone mode)".to_string()
    } else {
        bootstrap.join(", ")
    };

    let pairs = [
        ("Status", "running"),
        ("Peer ID", peer_id.as_str()),
        ("Listen", listen),
        ("Bootstrap", &bootstrap_display),
        ("Subscribed", "all categories"),
    ];
    writer.write_key_value_pairs(&pairs);
    writer.write_status("P2P mesh started — type 'help' for available commands");

    // Interactive stdin loop — use blocking stdin read in a spawned task
    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    tokio::task::spawn_blocking(move || {
        use std::io::{BufRead, Write};
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        loop {
            print!("> ");
            let _ = stdout.flush();
            let mut buf = String::new();
            match stdin.lock().read_line(&mut buf) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let trimmed = buf.trim_end().to_string();
                    if stdin_tx.send(trimmed).is_err() {
                        break; // receiver dropped
                    }
                }
                Err(_) => break,
            }
        }
    });

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                if let Some(h) = state.mesh_handle() {
                    let _ = h.shutdown();
                }
                writer.write_status("Mesh stopped");
                break;
            }
            line = stdin_rx.recv() => {
                match line {
                    Some(line) => handle_stdin_command(&line, writer, state),
                    None => break, // stdin closed
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Interactive stdin command dispatcher
// ---------------------------------------------------------------------------

fn handle_stdin_command(line: &str, writer: &OutputWriter, state: &AppState) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }

    let (cmd, args) = match parse_stdin_line(line) {
        Some(parsed) => parsed,
        None => return,
    };

    match cmd.as_str() {
        "post" => handle_stdin_post(&args, writer, state),
        "status" => handle_stdin_status(writer, state),
        "peers" => handle_stdin_peers(writer, state),
        "connect" => handle_stdin_connect(&args, writer, state),
        "help" => print_help(writer),
        "quit" | "exit" => {
            if let Some(h) = state.mesh_handle() {
                let _ = h.shutdown();
            }
            writer.write_status("Shutting down...");
        }
        _ => {
            writer.write_error(&format!("unknown command: {cmd} — type 'help' for commands"));
        }
    }
}

fn handle_stdin_post(args: &[&str], writer: &OutputWriter, state: &AppState) {
    if args.len() < 2 {
        writer.write_error("usage: post <kind_u16> <content>");
        return;
    }

    let kind: u16 = match args[0].parse() {
        Ok(k) => k,
        Err(_) => {
            writer.write_error(&format!("invalid kind number: {}", args[0]));
            return;
        }
    };
    let kind_val: Kind = match kind.try_into() {
        Ok(k) => k,
        Err(_) => {
            writer.write_error(&format!("unknown kind: {kind}"));
            return;
        }
    };

    let content = args[1..].join(" ");

    let keyring = match state.require_keyring() {
        Ok(kr) => kr,
        Err(e) => {
            writer.write_error(&e.to_string());
            return;
        }
    };
    let did = match state.require_did() {
        Ok(d) => d,
        Err(e) => {
            writer.write_error(&e.to_string());
            return;
        }
    };

    // Get latest sequence from feed store
    let store = state.feed_store();
    let latest_seq = match store.latest_sequence(&did.0) {
        Ok(seq) => seq,
        Err(e) => {
            writer.write_error(&format!("failed to query latest sequence: {e}"));
            return;
        }
    };
    let next_seq = if latest_seq == 0 { 1 } else { latest_seq + 1 };

    let prev_hash = if latest_seq == 0 {
        Hash256("0".to_string())
    } else {
        match store.get(&did.0, latest_seq) {
            Ok(Some(prev)) => {
                // Reconstruct a minimal event to compute hash
                let prev_content = std::str::from_utf8(&prev.payload).unwrap_or("").to_string();
                let prev_prev_hash_str =
                    std::str::from_utf8(&prev.prev_hash).unwrap_or("0").to_string();
                match FeedEvent::new(
                    Kind::AgentMetadata, // kind doesn't matter for hash — only prev event's full serialization matters
                    did.clone(),
                    prev.sequence,
                    Hash256(prev_prev_hash_str),
                    prev_content,
                ) {
                    Ok(prev_event) => {
                        prev_event.compute_hash().unwrap_or_else(|_| Hash256("0".to_string()))
                    }
                    Err(_) => Hash256("0".to_string()),
                }
            }
            _ => Hash256("0".to_string()),
        }
    };

    // Create and sign event
    let mut event = match FeedEvent::new(kind_val, did.clone(), next_seq, prev_hash, content) {
        Ok(e) => e,
        Err(e) => {
            writer.write_error(&format!("event creation failed: {e}"));
            return;
        }
    };

    if let Err(e) = event.validate() {
        writer.write_error(&format!("validation failed: {e}"));
        return;
    }

    let (ed_bytes, _) = keyring.to_bytes();
    let ed_bytes_fixed: [u8; 32] = match ed_bytes.as_slice().try_into() {
        Ok(arr) => arr,
        Err(_) => {
            writer.write_error("invalid ed25519 key length");
            return;
        }
    };
    if let Err(e) = event.sign(&ed_bytes_fixed) {
        writer.write_error(&format!("signing failed: {e}"));
        return;
    }

    // Store locally
    let stored = crate::feed_wire::feed_event_to_stored(&event);
    if let Err(e) = state.feed_store().append(&stored) {
        writer.write_error(&format!("store failed: {e}"));
        return;
    }

    // Publish to mesh
    if let Some(handle) = state.mesh_handle() {
        match crate::feed_wire::serialize_feed_event(&event) {
            Ok(bytes) => {
                let topic = kind_val.gossipsub_topic();
                if let Err(e) = handle.publish(topic, &bytes) {
                    writer.write_error(&format!("publish failed: {e}"));
                }
            }
            Err(e) => writer.write_error(&format!("serialize failed: {e}")),
        }
    }

    writer.write_status(&format!(
        "Event {} posted to {} (signed, persisted)",
        event.id,
        kind_val.gossipsub_topic()
    ));
}

fn handle_stdin_status(writer: &OutputWriter, state: &AppState) {
    if let Some(_handle) = state.mesh_handle() {
        // Mesh is running — show static info (async status query not possible from sync stdin)
        writer.write_status("Mesh is running — use 'agnetd mesh status' for full details");
    } else {
        writer.write_status("Mesh not running");
    }
}

fn handle_stdin_peers(writer: &OutputWriter, state: &AppState) {
    if state.mesh_handle().is_some() {
        writer.write_status("Mesh is running — use 'agnetd mesh peers' for peer list");
    } else {
        writer.write_status("Mesh not running");
    }
}

fn handle_stdin_connect(args: &[&str], writer: &OutputWriter, state: &AppState) {
    if args.is_empty() {
        writer.write_error("usage: connect <multiaddr>");
        return;
    }
    let addr_str = args[0];
    match addr_str.parse::<libp2p::Multiaddr>() {
        Ok(addr) => {
            if let Some(handle) = state.mesh_handle() {
                match handle.dial(addr) {
                    Ok(()) => writer.write_status(&format!("Dialing {addr_str}...")),
                    Err(e) => writer.write_error(&format!("dial failed: {e}")),
                }
            } else {
                writer.write_error("Mesh not running");
            }
        }
        Err(e) => writer.write_error(&format!("invalid multiaddr '{addr_str}': {e}")),
    }
}

fn print_help(writer: &OutputWriter) {
    let pairs = [
        ("post <kind> <text>", "Create and publish a feed event"),
        ("status", "Show mesh status"),
        ("peers", "List connected peers"),
        ("connect <addr>", "Dial a peer"),
        ("help", "Show this help"),
        ("quit", "Shutdown mesh and exit"),
    ];
    writer.write_key_value_pairs(&pairs);
}

// ---------------------------------------------------------------------------
// Parse stdin line → (command, args)
// ---------------------------------------------------------------------------

/// Parse a stdin line into (command, arguments).
/// Returns `None` for empty/whitespace-only lines.
fn parse_stdin_line(line: &str) -> Option<(String, Vec<&str>)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let cmd = parts[0].to_lowercase();
    Some((cmd, parts[1..].to_vec()))
}

// ---------------------------------------------------------------------------
// Mesh subcommands (called from CLI dispatch, NOT stdin)
// ---------------------------------------------------------------------------

fn mesh_status(writer: &OutputWriter, state: &AppState) -> Result<()> {
    if let Some(handle) = state.mesh_handle() {
        // status() is async — use block_on since we're in a sync context within tokio RT
        let rt = tokio::runtime::Handle::current();
        let status = rt.block_on(handle.status())?;

        let pairs = [
            ("Status", "running"),
            ("Peer ID", status.local_peer_id.as_str()),
            ("Listeners", &status.listeners.join(", ")),
            ("Connected Peers", &status.connected_peers.len().to_string()),
            ("Subscribed Topics", &status.subscribed_topics.join(", ")),
        ];
        writer.write_key_value_pairs(&pairs);
    } else {
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
            "note": "mesh not running — use 'agnetd mesh start' to begin",
        });
        writer.write_json(&status);
    }
    Ok(())
}

fn mesh_peers(verbose: bool, writer: &OutputWriter, state: &AppState) -> Result<()> {
    if let Some(handle) = state.mesh_handle() {
        let rt = tokio::runtime::Handle::current();
        let peers = rt.block_on(handle.peers())?;

        if verbose {
            let info = serde_json::json!({
                "peers": peers.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
                "count": peers.len(),
            });
            writer.write_json(&info);
        } else {
            let headers = ["Peer ID"];
            let rows: Vec<Vec<String>> = peers.iter().map(|p| vec![p.to_string()]).collect();
            writer.write_table(&headers, &rows);
            writer.write_status(&format!("{} peer(s) connected", peers.len()));
        }
    } else if verbose {
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

fn mesh_connect(addr: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    if let Some(handle) = state.mesh_handle() {
        let parsed: libp2p::Multiaddr =
            addr.parse().map_err(|e| anyhow::anyhow!("invalid multiaddr '{addr}': {e}"))?;
        handle.dial(parsed)?;
        writer.write_status(&format!("Dialing {addr}..."));
    } else {
        writer.write_status(&format!("Connect to {addr} — start mesh first with 'mesh start'"));
        let info = serde_json::json!({
            "action": "connect",
            "address": addr,
            "status": "mesh_not_running",
        });
        writer.write_json(&info);
    }
    Ok(())
}

fn mesh_disconnect(peer_id: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    let handle = state.require_mesh_handle()?;
    let pid: libp2p::PeerId =
        peer_id.parse().map_err(|e| anyhow::anyhow!("invalid peer ID '{peer_id}': {e}"))?;
    handle.disconnect(pid)?;
    writer.write_status(&format!("Disconnected from {peer_id}"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_post_command() {
        let result = parse_stdin_line("post 1000 hello world");
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, "post");
        assert_eq!(args, vec!["1000", "hello", "world"]);
    }

    #[test]
    fn parse_status_command() {
        let result = parse_stdin_line("status");
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, "status");
        assert!(args.is_empty());
    }

    #[test]
    fn parse_empty_line() {
        assert!(parse_stdin_line("").is_none());
    }

    #[test]
    fn parse_whitespace_only() {
        assert!(parse_stdin_line("   ").is_none());
    }

    #[test]
    fn parse_help_command() {
        let result = parse_stdin_line("help");
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, "help");
        assert!(args.is_empty());
    }

    #[test]
    fn parse_quit_command() {
        let result = parse_stdin_line("quit");
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, "quit");
        assert!(args.is_empty());
    }

    #[test]
    fn parse_exit_alias() {
        let result = parse_stdin_line("exit");
        let (cmd, _args) = result.unwrap();
        assert_eq!(cmd, "exit");
    }

    #[test]
    fn parse_case_insensitive() {
        let result = parse_stdin_line("STATUS");
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, "status");
        assert!(args.is_empty());
    }

    #[test]
    fn parse_connect_with_multiaddr() {
        let result = parse_stdin_line("connect /ip4/1.2.3.4/tcp/4001/p2p/QmABC");
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, "connect");
        assert_eq!(args[0], "/ip4/1.2.3.4/tcp/4001/p2p/QmABC");
    }

    #[test]
    fn parse_post_with_json_content() {
        let result = parse_stdin_line(r#"post 1000 {"title":"test"}"#);
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, "post");
        assert_eq!(args[0], "1000");
        // JSON gets split by whitespace — stdin post joins args back
        assert!(args.len() >= 2);
    }

    #[test]
    fn parse_single_connect_arg() {
        let result = parse_stdin_line("connect /ip4/127.0.0.1/tcp/41000");
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, "connect");
        assert_eq!(args, vec!["/ip4/127.0.0.1/tcp/41000"]);
    }
}
