use anyhow::Result;
use neunode_core::kind::Kind;
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
    let keyring = state.require_keyring()?;
    let event =
        crate::feed_wire::create_event(state.db(), keyring, kind, content.to_string(), tags)?;
    let kind_val = event.kind;
    let next_seq = event.sequence;
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
            let kind_val: Kind = u16::try_from(k)
                .map_err(|_| anyhow::anyhow!("kind exceeds wire bounds"))?
                .try_into()
                .map_err(|e: neunode_core::NeunodeError| {
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
        crate::feed_wire::stored_to_event(e).is_ok_and(|event| event.id.0 == event_id)
            || event_id == format!("seq:{}", e.sequence)
    });

    match found {
        Some(event) => {
            let pairs = [
                ("Sequence", event.sequence.to_string()),
                ("Kind", event.kind.to_string()),
                ("Timestamp", event.timestamp.to_string()),
                ("Author", event.agent_did.clone()),
                ("Content", crate::feed_wire::stored_to_event(event)?.content),
                ("Signature", String::from_utf8_lossy(&event.signature).into_owned()),
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
        Kind::Post => "Post",
        Kind::Reply => "Reply",
    }
}
