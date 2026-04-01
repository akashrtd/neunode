use anyhow::Result;
use neunode_core::kind::Kind;
use neunode_core::types::Hash256;
use neunode_storage::feed_store::StoredEvent;

use crate::cli::{FeedCommands, GlobalArgs};
use crate::output::OutputWriter;
use crate::state::AppState;

pub fn execute(cmd: &FeedCommands, args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(args.output);
    match cmd {
        FeedCommands::Post { kind, content, tags } => {
            feed_post(*kind, content, tags, state, &writer)
        }
        FeedCommands::List { kind, author, limit } => {
            feed_list(*kind, author.as_deref(), *limit, state, &writer)
        }
        FeedCommands::Subscribe { kind } => feed_subscribe(*kind, &writer, state),
        FeedCommands::Show { event_id } => feed_show(event_id, state, &writer),
    }
}

fn feed_post(
    kind: u32,
    content: &str,
    tags: &[String],
    state: &AppState,
    writer: &OutputWriter,
) -> Result<()> {
    let kind_val = (kind as u16)
        .try_into()
        .map_err(|e: neunode_core::NeunodeError| anyhow::anyhow!("invalid kind {}: {e}", kind))?;

    let keyring = state.require_keyring()?;
    let did = state.require_did()?;

    let store = state.feed_store();
    let latest_seq = store.latest_sequence(&did.0)?;
    let next_seq = if latest_seq == 0 { 1 } else { latest_seq + 1 };

    let prev_hash = if latest_seq == 0 {
        Hash256("0".to_string())
    } else {
        match store.get(&did.0, latest_seq)? {
            Some(prev) => {
                let prev_event = neunode_feed::event::FeedEvent::new(
                    Kind::AgentMetadata,
                    did.clone(),
                    prev.sequence,
                    Hash256(
                        std::str::from_utf8(&prev.prev_hash)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|_| "0".to_string()),
                    ),
                    prev.payload.iter().map(|&b| b as char).collect::<String>(),
                )?;
                prev_event.compute_hash()?
            }
            None => Hash256("0".to_string()),
        }
    };

    let prev_hash_bytes = prev_hash.0.as_bytes().to_vec();

    let mut event = neunode_feed::event::FeedEvent::new(
        kind_val,
        did.clone(),
        next_seq,
        prev_hash,
        content.to_string(),
    )?;

    let parsed_tags: Vec<neunode_feed::event::EventTag> = tags
        .iter()
        .map(|t| {
            let parts: Vec<&str> = t.splitn(2, '=').collect();
            neunode_feed::event::EventTag {
                key: parts.first().unwrap_or(&"").to_string(),
                value: parts.get(1).unwrap_or(&"").to_string(),
            }
        })
        .collect();
    event.tags = parsed_tags;

    event.validate()?;

    let (ed_bytes, _) = keyring.to_bytes();
    let ed_bytes_fixed: [u8; 32] = ed_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid ed25519 key length"))?;
    event.sign(&ed_bytes_fixed)?;

    let stored = StoredEvent {
        kind: kind_val.as_u16(),
        timestamp: event.timestamp,
        agent_did: did.0.clone(),
        sequence: next_seq,
        prev_hash: prev_hash_bytes,
        payload: content.as_bytes().to_vec(),
        signature: event.signature.as_ref().map(|s| s.0.as_bytes().to_vec()).unwrap_or_default(),
    };
    store.append(&stored)?;

    let kind_name = kind_name(kind_val);
    let topic = kind_val.gossipsub_topic();
    let kind_display = format!("{} ({})", kind, kind_name);
    let event_id_str = event.id.to_string();
    let author_str = event.author.to_string();
    let schema = kind_val.schema_nsid();

    let pairs = [
        ("Event ID", event_id_str.as_str()),
        ("Kind", kind_display.as_str()),
        ("Author", author_str.as_str()),
        ("Sequence", &next_seq.to_string()),
        ("Topic", topic),
        ("Schema", schema),
    ];
    writer.write_key_value_pairs(&pairs);
    writer.write_status(&format!("Event posted to {topic} (signed, persisted to DB)"));
    Ok(())
}

fn feed_list(
    kind: Option<u32>,
    author: Option<&str>,
    limit: usize,
    state: &AppState,
    writer: &OutputWriter,
) -> Result<()> {
    let did = match author {
        Some(a) => a.to_string(),
        None => state.require_did()?.0.clone(),
    };

    let store = state.feed_store();
    let events = store.get_all(&did)?;

    let filtered: Vec<StoredEvent> = events
        .into_iter()
        .filter(|e| kind.is_none_or(|k| e.kind == k as u16))
        .take(limit)
        .collect();

    if filtered.is_empty() {
        writer.write_status("No events found");
    } else {
        let headers = ["Seq", "Kind", "Timestamp", "Author"];
        let rows: Vec<Vec<String>> = filtered
            .iter()
            .map(|e| {
                vec![
                    e.sequence.to_string(),
                    e.kind.to_string(),
                    e.timestamp.to_string(),
                    e.agent_did.clone(),
                ]
            })
            .collect();
        writer.write_table(&headers, &rows);
    }
    Ok(())
}

fn feed_subscribe(kind: Option<u32>, writer: &OutputWriter, state: &mut AppState) -> Result<()> {
    let topic_filter = match kind {
        Some(k) => {
            let kind_val: Kind =
                (k as u16).try_into().map_err(|e: neunode_core::NeunodeError| {
                    anyhow::anyhow!("invalid kind {k}: {e}")
                })?;
            Some(kind_val.gossipsub_topic().to_string())
        }
        None => None,
    };

    let event_rx = state.mesh_handle.as_mut().and_then(|h| h.take_event_stream());

    match event_rx {
        Some(mut rx) => {
            writer.write_status("Subscribed — streaming events (Ctrl+C to stop)");
            loop {
                match rx.blocking_recv() {
                    Some(event) => {
                        let matches = match &topic_filter {
                            Some(tf) => event.kind.gossipsub_topic() == tf.as_str(),
                            None => true,
                        };
                        if matches {
                            let pairs = [
                                ("Event ID", event.id.to_string()),
                                (
                                    "Kind",
                                    format!("{} ({})", event.kind.as_u16(), kind_name(event.kind)),
                                ),
                                ("Author", event.author.0.clone()),
                                ("Sequence", event.sequence.to_string()),
                                ("Timestamp", event.timestamp.to_string()),
                                ("Content", event.content.chars().take(200).collect::<String>()),
                            ];
                            writer.write_key_value_pairs(
                                &pairs.iter().map(|(k, v)| (*k, v.as_str())).collect::<Vec<_>>(),
                            );
                        }
                    }
                    None => {
                        writer.write_status("Event stream ended");
                        break;
                    }
                }
            }
        }
        None => {
            writer.write_warning("Mesh not running — start mesh first for live streaming");
            let info = serde_json::json!({
                "status": "mesh_not_running",
                "hint": "run 'agnetd mesh start' first",
            });
            writer.write_json(&info);
        }
    }
    Ok(())
}

fn feed_show(event_id: &str, state: &AppState, writer: &OutputWriter) -> Result<()> {
    let did = state.require_did()?;
    let store = state.feed_store();

    let events = store.get_all(&did.0)?;
    let found = events.iter().find(|e| {
        let id_hex = hex::encode(&e.signature);
        id_hex.contains(event_id) || event_id.starts_with(&format!("seq:{}", e.sequence))
    });

    match found {
        Some(event) => {
            let pairs = [
                ("Sequence", event.sequence.to_string()),
                ("Kind", event.kind.to_string()),
                ("Timestamp", event.timestamp.to_string()),
                ("Author", event.agent_did.clone()),
                ("Content", std::str::from_utf8(&event.payload).unwrap_or("(binary)").to_string()),
                ("Signature", hex::encode(&event.signature)),
            ];
            writer.write_key_value_pairs(
                &pairs.iter().map(|(k, v)| (*k, v.as_str())).collect::<Vec<_>>(),
            );
        }
        None => {
            let info = serde_json::json!({
                "event_id": event_id,
                "status": "not_found",
            });
            writer.write_json(&info);
        }
    }
    Ok(())
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::AgentMetadata => "AgentMetadata",
        Kind::CapabilityUpdate => "CapabilityUpdate",
        Kind::ReputationChange => "ReputationChange",
        Kind::IdentityRotation => "IdentityRotation",
        Kind::Lifecycle => "Lifecycle",
        Kind::BountyPost => "BountyPost",
        Kind::BountyClaim => "BountyClaim",
        Kind::BountySubmit => "BountySubmit",
        Kind::BountyReview => "BountyReview",
        Kind::BountyDispute => "BountyDispute",
        Kind::BountyResolved => "BountyResolved",
        Kind::EscrowDeposit => "EscrowDeposit",
        Kind::EscrowRelease => "EscrowRelease",
        Kind::EscrowRefund => "EscrowRefund",
        Kind::JobSubmit => "JobSubmit",
        Kind::Checkpoint => "Checkpoint",
        Kind::TrainingResult => "TrainingResult",
        Kind::GradientUpdate => "GradientUpdate",
        Kind::EvalScore => "EvalScore",
        Kind::Attest => "Attest",
        Kind::CounterAttest => "CounterAttest",
        Kind::DisputeInit => "DisputeInit",
        Kind::VerificationResult => "VerificationResult",
        Kind::ModelAnnounce => "ModelAnnounce",
        Kind::ServeOffer => "ServeOffer",
        Kind::ServeResult => "ServeResult",
        Kind::BenchmarkClaim => "BenchmarkClaim",
        Kind::Proposal => "Proposal",
        Kind::Vote => "Vote",
        Kind::Delegate => "Delegate",
        Kind::ParameterChange => "ParameterChange",
    }
}
