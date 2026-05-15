use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use askama::Template;
use axum::extract::{
    ws::{Message, WebSocket},
    Query, State, WebSocketUpgrade,
};
use axum::response::{Html, IntoResponse, Sse};
use axum::routing::{get, post};
use axum::Router;
use futures::stream::Stream;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio_stream::wrappers::BroadcastStream;

use crate::cli::GlobalArgs;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Shared server state
// ---------------------------------------------------------------------------

pub struct ServerState {
    pub db: Arc<neunode_storage::db::NeunodeDb>,
    pub active_did: Option<String>,
    pub mesh_handle: Option<crate::mesh_handle::MeshHandle>,
    pub feed_tx: tokio::sync::broadcast::Sender<FeedEventUpdate>,
}

#[derive(Clone, serde::Serialize)]
#[allow(dead_code)]
pub struct FeedEventUpdate {
    pub kind: u16,
    pub author_did: String,
    pub author_short: String,
    pub kind_label: String,
    pub preview: String,
    pub time_ago: String,
}

// ---------------------------------------------------------------------------
// Template structs
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub page: &'static str,
    pub online: bool,
    pub node_id: String,
    pub latency: String,
    pub uptime: String,
    pub load: String,
    pub peer_count: usize,
    pub agent_count: usize,
    pub total_events: usize,
    pub active_bounties: usize,
    pub total_compute: String,
    pub connected_peers: usize,
    pub recent_events: Vec<FeedEventView>,
    pub system_logs: Vec<LogEntry>,
    pub has_mesh_data: bool,
    pub graph_data: String,
}

pub struct FeedEventView {
    pub author_short: String,
    pub kind_label: String,
    pub time_ago: String,
    pub preview: String,
}

pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

#[derive(Template)]
#[template(path = "partials/token_balances.html")]
pub struct TokenBalancesPartial {
    pub balances: Vec<TokenBalanceView>,
}

pub struct TokenBalanceView {
    pub label: String,
    pub balance: String,
}

#[derive(Template)]
#[template(path = "partials/bounty_list.html")]
pub struct BountyListPartial {
    pub bounties: Vec<BountyView>,
}

pub struct BountyView {
    pub id: String,
    pub title: String,
    pub state: String,
    pub state_class: String,
    pub reward: String,
    pub time_ago: String,
}

#[derive(Template)]
#[template(path = "partials/feed_event.html")]
pub struct FeedEventPartial {
    pub event: FeedEventView,
}

#[derive(Template)]
#[template(path = "partials/feed_events.html")]
pub struct FeedEventsPartial {
    pub events: Vec<FeedEventView>,
}

#[derive(Template)]
#[template(path = "partials/mesh_peers.html")]
pub struct MeshPeersPartial {
    pub peers: Vec<String>,
}

#[derive(Template)]
#[template(path = "feed.html")]
pub struct FeedTemplate {
    pub page: &'static str,
    pub online: bool,
    pub node_id: String,
    pub latency: String,
    pub uptime: String,
    pub load: String,
    pub peer_count: usize,
    pub agent_count: usize,
    pub total_events: usize,
    pub bounty_events: usize,
    pub training_events: usize,
    pub post_events: usize,
}

#[derive(Template)]
#[template(path = "bounties.html")]
pub struct BountiesTemplate {
    pub page: &'static str,
    pub online: bool,
    pub node_id: String,
    pub latency: String,
    pub uptime: String,
    pub load: String,
    pub peer_count: usize,
    pub agent_count: usize,
    pub total_bounties: usize,
}

#[derive(Template)]
#[template(path = "tokens.html")]
pub struct TokensTemplate {
    pub page: &'static str,
    pub online: bool,
    pub node_id: String,
    pub latency: String,
    pub uptime: String,
    pub load: String,
    pub peer_count: usize,
    pub agent_count: usize,
    pub balances: Vec<TokenCardView>,
    pub activity_level: String,
    pub decay_rate: String,
}

pub struct TokenCardView {
    pub label: String,
    pub balance: String,
    pub staked: String,
    pub decay_epoch: String,
    pub balance_u128: u128,
}

#[derive(Template)]
#[template(path = "analytics.html")]
pub struct AnalyticsTemplate {
    pub page: &'static str,
    pub online: bool,
    pub node_id: String,
    pub latency: String,
    pub uptime: String,
    pub load: String,
    pub peer_count: usize,
    pub agent_count: usize,
    pub activity_chart_data: String,
    pub bounty_chart_data: String,
    pub token_chart_data: String,
    pub kind_chart_data: String,
}

#[derive(Template)]
#[template(path = "mesh.html")]
pub struct MeshTemplate {
    pub page: &'static str,
    pub online: bool,
    pub node_id: String,
    pub latency: String,
    pub uptime: String,
    pub load: String,
    pub peer_count: usize,
    pub agent_count: usize,
    pub local_peer_id: String,
    pub topic_count: usize,
    pub listeners: Vec<String>,
    pub topics: Vec<String>,
}

#[derive(Template)]
#[template(path = "agents.html")]
pub struct AgentsTemplate {
    pub page: &'static str,
    pub online: bool,
    pub node_id: String,
    pub latency: String,
    pub uptime: String,
    pub load: String,
    pub peer_count: usize,
    pub agent_count: usize,
    pub has_agent: bool,
    pub agent_did: String,
    pub reputation_grade: String,
    pub stake_score: String,
    pub attest_score: String,
    pub activity_score: String,
    pub verify_score: String,
    pub tenure_score: String,
    pub token_balances: Vec<TokenBalanceView>,
    pub agent_events: Vec<FeedEventView>,
    pub agent_bounties: Vec<BountyView>,
}

// ---------------------------------------------------------------------------
// Query params
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
pub struct BountyFilter {
    pub state: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Deserialize, Default)]
pub struct FeedFilter {
    pub kind: Option<String>,
    pub author: Option<String>,
    pub mine: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct AgentQuery {
    pub did: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct FeedPostForm {
    pub kind: String,
    pub content: String,
    #[serde(default)]
    pub tags: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct BountyCreateForm {
    pub title: String,
    pub description: String,
    pub reward: u64,
    #[serde(default = "default_token")]
    pub token: String,
    #[serde(default = "default_claim_deadline")]
    pub claim_deadline: u64,
    #[serde(default = "default_work_deadline")]
    pub work_deadline: u64,
}

fn default_token() -> String {
    "compute".to_string()
}
fn default_claim_deadline() -> u64 {
    72
}
fn default_work_deadline() -> u64 {
    168
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn kind_label(kind: u16) -> String {
    match kind {
        0 => "METADATA".to_string(),
        1 => "CAPABILITY".to_string(),
        1000 => "BOUNTY_CREATED".to_string(),
        1001 => "BOUNTY_CLAIMED".to_string(),
        1002 => "BOUNTY_SUBMITTED".to_string(),
        1003 => "BOUNTY_REVIEWED".to_string(),
        1004 => "BOUNTY_DISPUTED".to_string(),
        1005 => "BOUNTY_RESOLVED".to_string(),
        1100 => "ESCROW_DEPOSIT".to_string(),
        1101 => "ESCROW_RELEASE".to_string(),
        2000 => "TRAINING_START".to_string(),
        2001 => "TRAINING_PROGRESS".to_string(),
        2002 => "TRAINING_COMPLETE".to_string(),
        3000 => "ATTESTATION".to_string(),
        3001 => "COUNTER_ATTEST".to_string(),
        4000 => "MODEL_PUBLISHED".to_string(),
        4001 => "SERVE_OFFER".to_string(),
        5000 => "GOV_PROPOSAL".to_string(),
        5001 => "GOV_VOTE".to_string(),
        9001 => "POST".to_string(),
        9002 => "REPLY".to_string(),
        _ => format!("KIND_{}", kind),
    }
}

fn truncate_did(did: &str) -> String {
    if did.len() > 20 {
        format!("{}...{}", &did[..12], &did[did.len() - 4..])
    } else {
        did.to_string()
    }
}

fn time_ago(unix_ts: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let diff = now.saturating_sub(unix_ts);
    if diff < 60 {
        format!("{diff}s")
    } else if diff < 3600 {
        format!("{diff}m", diff = diff / 60)
    } else if diff < 86400 {
        format!("{diff}h", diff = diff / 3600)
    } else {
        format!("{diff}d", diff = diff / 86400)
    }
}

fn format_u128(val: u128) -> String {
    if val >= 1_000_000_000_000 {
        format!("{:.1}T", val as f64 / 1_000_000_000_000.0)
    } else if val >= 1_000_000_000 {
        format!("{:.1}B", val as f64 / 1_000_000_000.0)
    } else if val >= 1_000_000 {
        format!("{:.1}M", val as f64 / 1_000_000.0)
    } else if val >= 1_000 {
        format!("{:.1}K", val as f64 / 1_000.0)
    } else {
        val.to_string()
    }
}

fn stored_to_view(e: &neunode_storage::feed_store::StoredEvent) -> FeedEventView {
    let preview = String::from_utf8_lossy(&e.payload).chars().take(80).collect();
    FeedEventView {
        author_short: truncate_did(&e.agent_did),
        kind_label: kind_label(e.kind),
        time_ago: time_ago(e.timestamp),
        preview,
    }
}

/// Build common status bar fields from mesh state.
async fn status_bar(state: &ServerState) -> (bool, String, usize) {
    if let Some(ref mesh) = state.mesh_handle {
        match mesh.status().await {
            Ok(s) => (true, truncate_did(&s.local_peer_id), s.connected_peers.len()),
            Err(_) => (false, "offline".to_string(), 0),
        }
    } else {
        (false, "offline".to_string(), 0)
    }
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

async fn dashboard_handler(State(state): State<Arc<ServerState>>) -> Html<String> {
    let feed_store = neunode_storage::feed_store::FeedStore::new(&state.db);
    let bounty_store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    let token_store = neunode_storage::token_store::TokenStore::new(&state.db);

    let all_events = feed_store.get_all("").unwrap_or_default();
    let recent_events: Vec<_> = all_events.iter().rev().take(20).map(stored_to_view).collect();

    let bounties = bounty_store.list_all().unwrap_or_default();
    let active_bounties =
        bounties.iter().filter(|b| b.state != "Paid" && b.state != "Cancelled").count();

    let total_compute = if let Some(ref did) = state.active_did {
        token_store
            .get_balance(did, neunode_storage::token_store::TOKEN_COMPUTE)
            .map(|b| format_u128(b.balance))
            .unwrap_or_else(|_| "0".to_string())
    } else {
        "0".to_string()
    };

    let (online, node_id, peer_count) = status_bar(&state).await;

    let (has_mesh_data, graph_data, connected_peers) = if let Some(ref mesh) = state.mesh_handle {
        match mesh.status().await {
            Ok(status) => {
                let peers = status.connected_peers.len();
                let nodes: Vec<_> = std::iter::once(serde_json::json!({
                    "name": status.local_peer_id,
                    "symbolSize": 12,
                    "itemStyle": {"color": "#22D3EE"}
                }))
                .chain(status.connected_peers.iter().map(|p| {
                    serde_json::json!({
                        "name": p, "symbolSize": 6, "itemStyle": {"color": "#4A90D9"}
                    })
                }))
                .collect();
                let links: Vec<_> = status
                    .connected_peers
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "source": status.local_peer_id, "target": p
                        })
                    })
                    .collect();
                (peers > 0, serde_json::json!({"nodes": nodes, "links": links}).to_string(), peers)
            }
            Err(_) => (false, "{}".to_string(), 0),
        }
    } else {
        (false, "{}".to_string(), 0)
    };

    let tpl = DashboardTemplate {
        page: "dashboard",
        online,
        node_id,
        latency: "--".to_string(),
        uptime: "--".to_string(),
        load: "--".to_string(),
        peer_count,
        agent_count: 0,
        total_events: all_events.len(),
        active_bounties,
        total_compute,
        connected_peers,
        recent_events,
        system_logs: vec![],
        has_mesh_data,
        graph_data,
    };
    Html(tpl.render().unwrap_or_default())
}

async fn feed_page_handler(State(state): State<Arc<ServerState>>) -> Html<String> {
    let feed_store = neunode_storage::feed_store::FeedStore::new(&state.db);
    let all_events = feed_store.get_all("").unwrap_or_default();

    let (online, node_id, peer_count) = status_bar(&state).await;

    let tpl = FeedTemplate {
        page: "feed",
        online,
        node_id,
        latency: "--".to_string(),
        uptime: "--".to_string(),
        load: "--".to_string(),
        peer_count,
        agent_count: 0,
        total_events: all_events.len(),
        bounty_events: all_events.iter().filter(|e| (1000..=1999).contains(&e.kind)).count(),
        training_events: all_events.iter().filter(|e| (2000..=2999).contains(&e.kind)).count(),
        post_events: all_events.iter().filter(|e| e.kind == 9001).count(),
    };
    Html(tpl.render().unwrap_or_default())
}

async fn feed_events_partial(
    State(state): State<Arc<ServerState>>,
    Query(filter): Query<FeedFilter>,
) -> Html<String> {
    let feed_store = neunode_storage::feed_store::FeedStore::new(&state.db);
    let all_events = feed_store.get_all("").unwrap_or_default();

    let kind_filter: Option<u16> = filter.kind.as_deref().and_then(|k| k.parse().ok());
    let mine_did = if filter.mine.is_some() { state.active_did.clone() } else { None };

    let events: Vec<_> = all_events
        .iter()
        .rev()
        .filter(|e| kind_filter.is_none_or(|k| e.kind == k))
        .filter(|e| {
            filter.author.as_deref().is_none_or(|a| e.agent_did.contains(a))
                || mine_did.as_deref().is_none_or(|d| e.agent_did == d)
        })
        .take(50)
        .map(stored_to_view)
        .collect();

    let tpl = FeedEventsPartial { events };
    Html(tpl.render().unwrap_or_default())
}

async fn feed_post_handler(
    State(state): State<Arc<ServerState>>,
    axum::Form(form): axum::Form<FeedPostForm>,
) -> Html<String> {
    let kind: u16 = form.kind.parse().unwrap_or(9001);
    let content = form.content.trim();
    if content.is_empty() {
        return Html(
            "<div style='color:var(--accent-red);font-size:12px;'>Content cannot be empty.</div>"
                .to_string(),
        );
    }

    let did = match &state.active_did {
        Some(d) => d.clone(),
        None => {
            return Html(
                "<div style='color:var(--accent-red);font-size:12px;'>No active identity.</div>"
                    .to_string(),
            );
        }
    };

    let feed_store = neunode_storage::feed_store::FeedStore::new(&state.db);
    let latest_seq = feed_store.latest_sequence(&did).unwrap_or(0);
    let next_seq = if latest_seq == 0 { 1 } else { latest_seq + 1 };
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let stored = neunode_storage::feed_store::StoredEvent {
        kind,
        timestamp: now_ts,
        agent_did: did.clone(),
        sequence: next_seq,
        prev_hash: vec![0u8; 32],
        payload: content.as_bytes().to_vec(),
        signature: vec![],
    };
    feed_store.append(&stored).ok();

    // Broadcast to SSE subscribers
    let _ = state.feed_tx.send(FeedEventUpdate {
        kind,
        author_did: did.clone(),
        author_short: truncate_did(&did),
        kind_label: kind_label(kind),
        preview: content.chars().take(80).collect(),
        time_ago: "now".to_string(),
    });

    Html("<div style='color:var(--accent-green);font-size:12px;'>Posted.</div>".to_string())
}

async fn bounties_page_handler(State(state): State<Arc<ServerState>>) -> Html<String> {
    let bounty_store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    let bounties = bounty_store.list_all().unwrap_or_default();
    let (online, node_id, peer_count) = status_bar(&state).await;

    let tpl = BountiesTemplate {
        page: "bounties",
        online,
        node_id,
        latency: "--".to_string(),
        uptime: "--".to_string(),
        load: "--".to_string(),
        peer_count,
        agent_count: 0,
        total_bounties: bounties.len(),
    };
    Html(tpl.render().unwrap_or_default())
}

async fn bounty_create_handler(
    State(state): State<Arc<ServerState>>,
    axum::Form(form): axum::Form<BountyCreateForm>,
) -> Html<String> {
    let did = match &state.active_did {
        Some(d) => d.clone(),
        None => {
            return Html(
                "<div style='color:var(--accent-red);font-size:12px;'>No active identity.</div>"
                    .to_string(),
            );
        }
    };

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let bounty_id = format!("bnty_{}", hex::encode(&now_ts.to_be_bytes()[..4]));
    let bounty = neunode_storage::bounty_store::BountyData {
        id: bounty_id.clone(),
        state: "Open".to_string(),
        requester_did: did,
        provider_did: None,
        reward_amount: form.reward,
        reward_token_type: 0x01,
        deadline: now_ts + form.work_deadline * 3600,
        created_at: now_ts,
        escrow_deposited: form.reward,
        title: form.title,
        description: form.description,
        claim_deadline: now_ts + form.claim_deadline * 3600,
        work_deadline: now_ts + form.work_deadline * 3600,
        review_deadline: now_ts + (form.work_deadline + 72) * 3600,
        artifact_hash: None,
        bond: None,
    };

    let bounty_store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    match bounty_store.put(&bounty) {
        Ok(()) => Html(format!(
            "<div style='color:var(--accent-green);font-size:12px;'>Bounty created: {}</div>",
            bounty_id
        )),
        Err(e) => {
            Html(format!("<div style='color:var(--accent-red);font-size:12px;'>Error: {e}</div>"))
        }
    }
}

async fn tokens_page_handler(State(state): State<Arc<ServerState>>) -> Html<String> {
    let token_store = neunode_storage::token_store::TokenStore::new(&state.db);
    let (online, node_id, peer_count) = status_bar(&state).await;

    let labels = ["nCompute", "nTrain", "nBandwidth", "nStorage"];
    let types = [
        neunode_storage::token_store::TOKEN_COMPUTE,
        neunode_storage::token_store::TOKEN_TRAINING,
        neunode_storage::token_store::TOKEN_BANDWIDTH,
        neunode_storage::token_store::TOKEN_STORAGE,
    ];

    let balances: Vec<TokenCardView> = if let Some(ref did) = state.active_did {
        types
            .iter()
            .zip(labels.iter())
            .map(|(&tt, label)| {
                let bal = token_store.get_balance(did, tt).unwrap_or_default();
                TokenCardView {
                    label: label.to_string(),
                    balance: format_u128(bal.balance),
                    staked: format_u128(bal.staked),
                    decay_epoch: bal.last_decay_epoch.to_string(),
                    balance_u128: bal.balance,
                }
            })
            .collect()
    } else {
        labels
            .iter()
            .map(|label| TokenCardView {
                label: label.to_string(),
                balance: "0".to_string(),
                staked: "0".to_string(),
                decay_epoch: "0".to_string(),
                balance_u128: 0,
            })
            .collect()
    };

    let tpl = TokensTemplate {
        page: "tokens",
        online,
        node_id,
        latency: "--".to_string(),
        uptime: "--".to_string(),
        load: "--".to_string(),
        peer_count,
        agent_count: 0,
        balances,
        activity_level: "Active".to_string(),
        decay_rate: "0%".to_string(),
    };
    Html(tpl.render().unwrap_or_default())
}

async fn analytics_page_handler(State(state): State<Arc<ServerState>>) -> Html<String> {
    let feed_store = neunode_storage::feed_store::FeedStore::new(&state.db);
    let all_events = feed_store.get_all("").unwrap_or_default();
    let (online, node_id, peer_count) = status_bar(&state).await;

    // Activity chart: last 7 days
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let day_secs: u64 = 86400;
    let mut activity_data = vec![0u64; 7];
    for e in &all_events {
        let day_index = ((now_ts.saturating_sub(e.timestamp)) / day_secs) as usize;
        if day_index < 7 {
            activity_data[6 - day_index] += 1;
        }
    }
    let activity_labels: Vec<String> = (0..7).rev().map(|i| format!("{}d ago", i)).collect();
    let activity_chart = serde_json::json!({
        "xAxis": {"type": "category", "data": activity_labels, "axisLabel": {"color": "#5C6078"}},
        "yAxis": {"type": "value", "axisLabel": {"color": "#5C6078"}, "splitLine": {"lineStyle": {"color": "#1E2130"}}},
        "series": [{"data": activity_data, "type": "bar", "itemStyle": {"color": "#4A90D9"}}],
        "grid": {"left": 40, "right": 10, "top": 10, "bottom": 30},
        "tooltip": {"trigger": "axis"}
    }).to_string();

    // Bounty completion
    let bounty_store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    let bounties = bounty_store.list_all().unwrap_or_default();
    let open = bounties.iter().filter(|b| b.state == "Open").count();
    let claimed = bounties.iter().filter(|b| b.state == "Claimed").count();
    let completed = bounties.iter().filter(|b| b.state == "Paid" || b.state == "Accepted").count();
    let rejected = bounties.iter().filter(|b| b.state == "Rejected").count();
    let bounty_chart = serde_json::json!({
        "xAxis": {"type": "category", "data": ["Open", "Claimed", "Completed", "Rejected"], "axisLabel": {"color": "#5C6078"}},
        "yAxis": {"type": "value", "axisLabel": {"color": "#5C6078"}, "splitLine": {"lineStyle": {"color": "#1E2130"}}},
        "series": [{"data": [
            {"value": open, "itemStyle": {"color": "#4A90D9"}},
            {"value": claimed, "itemStyle": {"color": "#22D3EE"}},
            {"value": completed, "itemStyle": {"color": "#34D399"}},
            {"value": rejected, "itemStyle": {"color": "#F87171"}}
        ], "type": "bar"}],
        "grid": {"left": 40, "right": 10, "top": 10, "bottom": 30},
        "tooltip": {"trigger": "axis"}
    }).to_string();

    // Token distribution
    let token_chart = serde_json::json!({
        "series": [{"type": "pie", "radius": ["40%", "70%"], "data": [
            {"name": "nCompute", "value": all_events.len().max(1), "itemStyle": {"color": "#4A90D9"}},
            {"name": "nTrain", "value": all_events.len().max(1) / 2, "itemStyle": {"color": "#22D3EE"}},
            {"name": "nBandwidth", "value": all_events.len().max(1) / 3, "itemStyle": {"color": "#34D399"}},
            {"name": "nStorage", "value": all_events.len().max(1) / 4, "itemStyle": {"color": "#A78BFA"}}
        ], "label": {"color": "#8B8FA3"}}],
        "tooltip": {"trigger": "item"}
    }).to_string();

    // Kind distribution
    let mut kind_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for e in &all_events {
        let cat = match e.kind {
            0..=99 => "System".to_string(),
            1000..=1999 => "Bounty".to_string(),
            2000..=2999 => "Training".to_string(),
            3000..=3999 => "Attestation".to_string(),
            4000..=4999 => "Inference".to_string(),
            5000..=5999 => "Governance".to_string(),
            9000..=9999 => "Custom".to_string(),
            _ => "Unknown".to_string(),
        };
        *kind_counts.entry(cat).or_insert(0) += 1;
    }
    let kind_data: Vec<_> = kind_counts
        .into_iter()
        .map(|(name, value)| {
            let color = match name.as_str() {
                "Bounty" => "#4A90D9",
                "Training" => "#22D3EE",
                "Attestation" => "#34D399",
                "Inference" => "#A78BFA",
                "Governance" => "#FBBF24",
                "Custom" => "#F87171",
                _ => "#5C6078",
            };
            serde_json::json!({"name": name, "value": value, "itemStyle": {"color": color}})
        })
        .collect();
    let kind_chart = serde_json::json!({
        "series": [{"type": "pie", "radius": ["40%", "70%"], "data": kind_data, "label": {"color": "#8B8FA3"}}],
        "tooltip": {"trigger": "item"}
    }).to_string();

    let tpl = AnalyticsTemplate {
        page: "analytics",
        online,
        node_id,
        latency: "--".to_string(),
        uptime: "--".to_string(),
        load: "--".to_string(),
        peer_count,
        agent_count: 0,
        activity_chart_data: activity_chart,
        bounty_chart_data: bounty_chart,
        token_chart_data: token_chart,
        kind_chart_data: kind_chart,
    };
    Html(tpl.render().unwrap_or_default())
}

async fn mesh_page_handler(State(state): State<Arc<ServerState>>) -> Html<String> {
    let (online, node_id, peer_count) = status_bar(&state).await;

    let (local_peer_id, listeners, topics, topic_count) = if let Some(ref mesh) = state.mesh_handle
    {
        match mesh.status().await {
            Ok(s) => {
                let topic_count = s.subscribed_topics.len();
                (truncate_did(&s.local_peer_id), s.listeners, s.subscribed_topics, topic_count)
            }
            Err(_) => ("--".to_string(), vec![], vec![], 0),
        }
    } else {
        ("--".to_string(), vec![], vec![], 0)
    };

    let tpl = MeshTemplate {
        page: "mesh",
        online,
        node_id,
        latency: "--".to_string(),
        uptime: "--".to_string(),
        load: "--".to_string(),
        peer_count,
        agent_count: 0,
        local_peer_id,
        topic_count,
        listeners,
        topics,
    };
    Html(tpl.render().unwrap_or_default())
}

async fn mesh_peers_partial(State(state): State<Arc<ServerState>>) -> Html<String> {
    let peers = if let Some(ref mesh) = state.mesh_handle {
        mesh.status().await.map(|s| s.connected_peers).unwrap_or_default()
    } else {
        vec![]
    };
    let tpl = MeshPeersPartial { peers };
    Html(tpl.render().unwrap_or_default())
}

async fn agents_page_handler(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<AgentQuery>,
) -> Html<String> {
    let (online, node_id, peer_count) = status_bar(&state).await;
    let did = query.did.unwrap_or_default();

    if did.is_empty() {
        let tpl = AgentsTemplate {
            page: "agents",
            online,
            node_id,
            latency: "--".to_string(),
            uptime: "--".to_string(),
            load: "--".to_string(),
            peer_count,
            agent_count: 0,
            has_agent: false,
            agent_did: String::new(),
            reputation_grade: "--".to_string(),
            stake_score: "--".to_string(),
            attest_score: "--".to_string(),
            activity_score: "--".to_string(),
            verify_score: "--".to_string(),
            tenure_score: "--".to_string(),
            token_balances: vec![],
            agent_events: vec![],
            agent_bounties: vec![],
        };
        return Html(tpl.render().unwrap_or_default());
    }

    let feed_store = neunode_storage::feed_store::FeedStore::new(&state.db);
    let token_store = neunode_storage::token_store::TokenStore::new(&state.db);
    let bounty_store = neunode_storage::bounty_store::BountyStore::new(&state.db);

    let agent_events: Vec<_> = feed_store
        .get_all(&did)
        .unwrap_or_default()
        .iter()
        .rev()
        .take(20)
        .map(stored_to_view)
        .collect();

    let event_count = agent_events.len();
    let grade = match event_count {
        0..=5 => "D",
        6..=20 => "C",
        21..=50 => "B",
        51..=100 => "B+",
        101..=500 => "A",
        _ => "A+",
    };

    let token_balances = {
        let labels = ["nCompute", "nTrain", "nBandwidth", "nStorage"];
        let types = [
            neunode_storage::token_store::TOKEN_COMPUTE,
            neunode_storage::token_store::TOKEN_TRAINING,
            neunode_storage::token_store::TOKEN_BANDWIDTH,
            neunode_storage::token_store::TOKEN_STORAGE,
        ];
        types
            .iter()
            .zip(labels.iter())
            .map(|(&tt, label)| {
                let bal = token_store.get_balance(&did, tt).unwrap_or_default();
                TokenBalanceView { label: label.to_string(), balance: format_u128(bal.balance) }
            })
            .collect()
    };

    let agent_bounties: Vec<_> = bounty_store
        .list_all()
        .unwrap_or_default()
        .iter()
        .filter(|b| b.requester_did == did || b.provider_did.as_deref() == Some(&did))
        .take(20)
        .map(|b| BountyView {
            id: b.id.clone(),
            title: if b.title.is_empty() { truncate_did(&b.id) } else { b.title.clone() },
            state: b.state.clone(),
            state_class: b.state.to_lowercase(),
            reward: format!("{} nC", b.reward_amount),
            time_ago: time_ago(b.created_at),
        })
        .collect();

    let tpl = AgentsTemplate {
        page: "agents",
        online,
        node_id,
        latency: "--".to_string(),
        uptime: "--".to_string(),
        load: "--".to_string(),
        peer_count,
        agent_count: 0,
        has_agent: true,
        agent_did: did,
        reputation_grade: grade.to_string(),
        stake_score: "0.0".to_string(),
        attest_score: "0.0".to_string(),
        activity_score: "0.0".to_string(),
        verify_score: "0.0".to_string(),
        tenure_score: "0.0".to_string(),
        token_balances,
        agent_events,
        agent_bounties,
    };
    Html(tpl.render().unwrap_or_default())
}

async fn token_balances_partial(State(state): State<Arc<ServerState>>) -> Html<String> {
    let token_store = neunode_storage::token_store::TokenStore::new(&state.db);

    let balances = if let Some(ref did) = state.active_did {
        match token_store.get_all_balances(did) {
            Ok(bals) => {
                let labels = ["nCompute", "nTrain", "nBandwidth", "nStorage"];
                labels
                    .iter()
                    .zip(bals.iter())
                    .map(|(label, b)| TokenBalanceView {
                        label: label.to_string(),
                        balance: format_u128(b.balance),
                    })
                    .collect()
            }
            Err(_) => vec![],
        }
    } else {
        vec![TokenBalanceView { label: "No identity".to_string(), balance: "-".to_string() }]
    };

    let tpl = TokenBalancesPartial { balances };
    Html(tpl.render().unwrap_or_default())
}

async fn bounty_list_partial(
    State(state): State<Arc<ServerState>>,
    Query(filter): Query<BountyFilter>,
) -> Html<String> {
    let bounty_store = neunode_storage::bounty_store::BountyStore::new(&state.db);
    let limit = filter.limit.unwrap_or(20);

    let bounties = bounty_store.list_all().unwrap_or_default();
    let filtered: Vec<_> = bounties
        .iter()
        .filter(|b| filter.state.as_deref().is_none_or(|s| b.state == s))
        .take(limit)
        .map(|b| BountyView {
            id: b.id.clone(),
            title: if b.title.is_empty() { truncate_did(&b.id) } else { b.title.clone() },
            state: b.state.clone(),
            state_class: b.state.to_lowercase(),
            reward: format!("{} nC", b.reward_amount),
            time_ago: time_ago(b.created_at),
        })
        .collect();

    let tpl = BountyListPartial { bounties: filtered };
    Html(tpl.render().unwrap_or_default())
}

async fn feed_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| feed_ws_client(socket, state))
}

async fn feed_ws_client(socket: WebSocket, state: Arc<ServerState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.feed_tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(update) = rx.recv().await {
            let json = match serde_json::to_string(&update) {
                Ok(j) => j,
                Err(_) => continue,
            };
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
}

async fn feed_sse_handler(
    State(state): State<Arc<ServerState>>,
) -> Sse<impl Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    let rx = state.feed_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| {
        let result = match msg {
            Ok(update) => {
                let event = FeedEventView {
                    author_short: update.author_short,
                    kind_label: update.kind_label,
                    time_ago: update.time_ago,
                    preview: update.preview,
                };
                let partial = FeedEventPartial { event };
                let html = partial.render().unwrap_or_default();
                Some(Ok(axum::response::sse::Event::default().event("feed-event").data(html)))
            }
            Err(_) => None,
        };
        std::future::ready(result)
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(30))
            .text("keep-alive"),
    )
}

// ---------------------------------------------------------------------------
// Serve command entry point
// ---------------------------------------------------------------------------

pub async fn execute(port: u16, _args: &GlobalArgs, app_state: &mut AppState) -> Result<()> {
    let (feed_tx, _) = tokio::sync::broadcast::channel(256);

    let server_state = Arc::new(ServerState {
        db: Arc::clone(&app_state.db),
        active_did: app_state.active_did.as_ref().map(|d| d.0.clone()),
        mesh_handle: app_state.mesh_handle.take(),
        feed_tx,
    });

    let app = Router::new()
        // Pages
        .route("/", get(dashboard_handler))
        .route("/feed", get(feed_page_handler))
        .route("/bounties", get(bounties_page_handler))
        .route("/tokens", get(tokens_page_handler))
        .route("/analytics", get(analytics_page_handler))
        .route("/mesh", get(mesh_page_handler))
        .route("/agents", get(agents_page_handler))
        // Partials (HTMX)
        .route("/partials/token-balances", get(token_balances_partial))
        .route("/partials/bounty-list", get(bounty_list_partial))
        .route("/partials/feed-events", get(feed_events_partial))
        .route("/partials/mesh-peers", get(mesh_peers_partial))
        // API endpoints
        .route("/api/feed/post", post(feed_post_handler))
        .route("/api/bounties/create", post(bounty_create_handler))
        // SSE stream
        .route("/events/stream", get(feed_sse_handler))
        // WebSocket stream
        .route("/ws/feed", get(feed_ws_handler))
        .with_state(server_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("{}  neunode dashboard → http://127.0.0.1:{}", console::style("INFO").dim(), port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
