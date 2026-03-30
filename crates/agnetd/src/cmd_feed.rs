use anyhow::Result;
use neunode_core::kind::Kind;
use neunode_core::types::{Did, Hash256};

use crate::cli::{Cli, FeedCommands};
use crate::config::CliConfig;
use crate::output::OutputWriter;

pub fn execute(cmd: &FeedCommands, cli: &Cli, _config: &mut CliConfig) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        FeedCommands::Post { kind, content, tags } => feed_post(*kind, content, tags, cli, &writer),
        FeedCommands::List { kind, author, limit } => {
            feed_list(*kind, author.as_deref(), *limit, &writer)
        }
        FeedCommands::Subscribe { kind } => feed_subscribe(*kind, &writer),
        FeedCommands::Show { event_id } => feed_show(event_id, &writer),
    }
}

fn feed_post(
    kind: u32,
    content: &str,
    tags: &[String],
    cli: &Cli,
    writer: &OutputWriter,
) -> Result<()> {
    let kind_val = (kind as u16)
        .try_into()
        .map_err(|e: neunode_core::NeunodeError| anyhow::anyhow!("invalid kind {}: {e}", kind))?;

    let author_did = cli.identity.as_deref().map(|s| Did(s.to_string())).unwrap_or_else(|| {
        Did("did:neunode:0x0000000000000000000000000000000000000000".to_string())
    });

    let mut event = neunode_feed::event::FeedEvent::new(
        kind_val,
        author_did,
        0,
        Hash256("0".to_string()),
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
        ("Sequence", &event.sequence.to_string()),
        ("Topic", topic),
        ("Schema", schema),
    ];
    writer.write_key_value_pairs(&pairs);
    writer.write_status(&format!("Event posted to {topic} (unsigned — Phase 1 MVP)"));
    Ok(())
}

fn feed_list(
    kind: Option<u32>,
    author: Option<&str>,
    limit: usize,
    writer: &OutputWriter,
) -> Result<()> {
    let kind_display = kind.map(|k| format!("{} ({})", k, k)).unwrap_or_else(|| "all".to_string());
    let author_display = author.unwrap_or("all").to_string();

    let info = serde_json::json!({
        "kind_filter": kind_display,
        "author_filter": author_display,
        "limit": limit,
        "events": [],
        "note": "Phase 1 MVP — feed storage not yet connected",
    });
    writer.write_json(&info);
    Ok(())
}

fn feed_subscribe(kind: Option<u32>, writer: &OutputWriter) -> Result<()> {
    let topic = match kind {
        Some(k) => {
            let kind_val: Kind =
                (k as u16).try_into().map_err(|e: neunode_core::NeunodeError| {
                    anyhow::anyhow!("invalid kind {k}: {e}")
                })?;
            kind_val.gossipsub_topic().to_string()
        }
        None => "neunode/*".to_string(),
    };

    writer.write_status(&format!("Subscribed to {topic}"));
    writer.write_warning("Streaming not yet available in Phase 1 MVP");
    let info = serde_json::json!({
        "topic": topic,
        "status": "subscribed",
        "streaming": false,
    });
    writer.write_json(&info);
    Ok(())
}

fn feed_show(event_id: &str, writer: &OutputWriter) -> Result<()> {
    let info = serde_json::json!({
        "event_id": event_id,
        "status": "not_found",
        "note": "Phase 1 MVP — event lookup not yet connected to storage",
    });
    writer.write_json(&info);
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
