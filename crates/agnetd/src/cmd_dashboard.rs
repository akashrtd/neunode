use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table, Wrap},
    Frame, Terminal,
};

use crate::cli::Cli;
use crate::mesh_handle::MeshStatus;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Dashboard data structs
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct DashboardApp {
    selected_panel: usize,
    scroll_offsets: [u16; 5],
    mesh_status: Option<MeshStatus>,
    token_balances: Vec<TokenBalanceRow>,
    reputation_score: u64,
    feed_events: Vec<FeedEventSummary>,
    active_bounties: Vec<BountySummary>,
    training_jobs: Vec<TrainingJobSummary>,
    should_quit: bool,
    last_refresh: Instant,
}

struct TokenBalanceRow {
    token_type: &'static str,
    balance: u128,
    staked: u128,
}

#[allow(dead_code)]
struct FeedEventSummary {
    kind: u16,
    author: String,
    content_preview: String,
    timestamp: u64,
}

struct BountySummary {
    id: String,
    reward: u64,
    state: String,
}

struct TrainingJobSummary {
    id: String,
    description: String,
    progress: u16,
}

impl DashboardApp {
    fn new() -> Self {
        Self {
            selected_panel: 0,
            scroll_offsets: [0; 5],
            mesh_status: None,
            token_balances: vec![
                TokenBalanceRow { token_type: "Compute", balance: 0, staked: 0 },
                TokenBalanceRow { token_type: "Training", balance: 0, staked: 0 },
                TokenBalanceRow { token_type: "Bandwidth", balance: 0, staked: 0 },
                TokenBalanceRow { token_type: "Storage", balance: 0, staked: 0 },
            ],
            reputation_score: 0,
            feed_events: Vec::new(),
            active_bounties: Vec::new(),
            training_jobs: Vec::new(),
            should_quit: false,
            last_refresh: Instant::now(),
        }
    }

    async fn refresh_data(&mut self, state: &AppState) {
        if let Some(handle) = state.mesh_handle() {
            self.mesh_status = handle.status().await.ok();
        } else {
            self.mesh_status = None;
        }

        if let Ok(did) = state.require_did() {
            let store = state.token_store();
            if let Ok(balances) = store.get_all_balances(&did.0) {
                let types = ["Compute", "Training", "Bandwidth", "Storage"];
                for (i, bal) in balances.iter().enumerate() {
                    if i < self.token_balances.len() {
                        self.token_balances[i].balance = bal.balance;
                        self.token_balances[i].staked = bal.staked;
                    }
                }
                let _ = types;
            }

            let feed_store = state.feed_store();
            let latest = feed_store.latest_sequence(&did.0).unwrap_or(0);
            if latest > 0 {
                let from = latest.saturating_sub(20);
                if let Ok(events) = feed_store.get_range(&did.0, from, 20) {
                    self.feed_events = events
                        .into_iter()
                        .map(|e| {
                            let preview =
                                String::from_utf8_lossy(&e.payload).chars().take(60).collect();
                            FeedEventSummary {
                                kind: e.kind,
                                author: e.agent_did,
                                content_preview: preview,
                                timestamp: e.timestamp,
                            }
                        })
                        .collect();
                }
            }
        }

        let bounty_store = state.bounty_store();
        if let Ok(all_bounties) = bounty_store.list_all() {
            self.active_bounties = all_bounties
                .into_iter()
                .filter(|b| b.state != "Cancelled" && b.state != "Paid")
                .take(20)
                .map(|b| BountySummary { id: b.id, reward: b.reward_amount, state: b.state })
                .collect();
        }

        self.last_refresh = Instant::now();
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Tab => self.selected_panel = (self.selected_panel + 1) % 5,
            KeyCode::BackTab => self.selected_panel = (self.selected_panel + 4) % 5,
            KeyCode::Down => {
                self.scroll_offsets[self.selected_panel] =
                    self.scroll_offsets[self.selected_panel].saturating_add(1);
            }
            KeyCode::Up => {
                self.scroll_offsets[self.selected_panel] =
                    self.scroll_offsets[self.selected_panel].saturating_sub(1);
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Terminal guard — restores terminal state on drop (panic-safe)
// ---------------------------------------------------------------------------

struct TerminalGuard;

impl TerminalGuard {
    fn init() -> Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub async fn execute(_cli: &Cli, state: &mut AppState) -> Result<()> {
    let _guard = TerminalGuard::init()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = DashboardApp::new();
    app.refresh_data(state).await;

    let tick_rate = Duration::from_secs(2);

    loop {
        terminal.draw(|f| render(f, &app))?;

        let timeout = tick_rate.saturating_sub(app.last_refresh.elapsed());
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }

        if app.should_quit {
            break;
        }

        if app.last_refresh.elapsed() >= tick_rate {
            app.refresh_data(state).await;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render(f: &mut Frame, app: &DashboardApp) {
    let size = f.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(1)])
        .split(size);

    render_header(f, vertical[0], app);
    render_content(f, vertical[1], app);
    render_status_bar(f, vertical[2]);
}

fn render_header(f: &mut Frame, area: Rect, app: &DashboardApp) {
    let peer_id: String = app
        .mesh_status
        .as_ref()
        .map(|s| s.local_peer_id.chars().take(16).collect())
        .unwrap_or_else(|| "not connected".into());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " Neunode Dashboard ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(format!("Peer: {}…", peer_id), Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("[q]uit  [Tab]panel  [↑↓]scroll", Style::default().fg(Color::DarkGray)),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).style(Style::default().fg(Color::DarkGray)));
    f.render_widget(header, area);
}

fn render_content(f: &mut Frame, area: Rect, app: &DashboardApp) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(35),
            Constraint::Percentage(30),
        ])
        .split(area);

    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    render_mesh_health(f, top_cols[0], app, 0);
    render_token_balances(f, top_cols[1], app, 1);

    let mid_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    render_feed_activity(f, mid_cols[0], app, 2);
    render_active_bounties(f, mid_cols[1], app, 3);

    render_training_progress(f, rows[2], app, 4);
}

fn panel_block(title: &str, selected: bool) -> Block<'_> {
    let border_color = if selected { Color::Cyan } else { Color::DarkGray };
    Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title_style(
            Style::default()
                .fg(if selected { Color::Cyan } else { Color::White })
                .add_modifier(Modifier::BOLD),
        )
}

fn render_mesh_health(f: &mut Frame, area: Rect, app: &DashboardApp, panel_idx: usize) {
    let block = panel_block("Mesh Health", app.selected_panel == panel_idx);

    let status = app.mesh_status.as_ref();
    let connected = status.map(|s| s.connected_peers.len()).unwrap_or(0);
    let listeners = status.map(|s| s.listeners.len()).unwrap_or(0);
    let topics = status.map(|s| s.subscribed_topics.len()).unwrap_or(0);

    let (status_text, status_color) = if status.is_some() {
        ("● Connected", Color::Green)
    } else {
        ("○ Not Connected", Color::Red)
    };

    let mut lines = vec![Line::from(vec![Span::styled(
        status_text.to_string(),
        Style::default().fg(status_color),
    )])];

    if let Some(s) = status {
        lines.push(Line::from(vec![
            Span::raw("Peer ID:  "),
            Span::styled(
                s.local_peer_id.chars().take(24).collect::<String>(),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }

    lines.push(Line::from(vec![
        Span::raw("Peers:    "),
        Span::styled(connected.to_string(), Style::default().fg(Color::Yellow)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("Listeners: "),
        Span::styled(listeners.to_string(), Style::default().fg(Color::Yellow)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("Topics:   "),
        Span::styled(topics.to_string(), Style::default().fg(Color::Yellow)),
    ]));

    if status.is_none() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Run 'agnetd mesh start' to connect",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn render_token_balances(f: &mut Frame, area: Rect, app: &DashboardApp, panel_idx: usize) {
    let block = panel_block("Token Balances", app.selected_panel == panel_idx);

    let header = Row::new(vec![
        Cell::from("Token").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Balance").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Staked").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let rows: Vec<Row> = app
        .token_balances
        .iter()
        .map(|tb| {
            Row::new(vec![
                Cell::from(tb.token_type),
                Cell::from(format_balance(tb.balance)),
                Cell::from(format_balance(tb.staked)),
            ])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Percentage(33); 3])
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().bg(Color::DarkGray));
    f.render_widget(table, area);
}

fn render_feed_activity(f: &mut Frame, area: Rect, app: &DashboardApp, panel_idx: usize) {
    let block = panel_block("Feed Activity", app.selected_panel == panel_idx);

    if app.feed_events.is_empty() {
        let p =
            Paragraph::new("No events").block(block).style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
        return;
    }

    let lines: Vec<Line> = app
        .feed_events
        .iter()
        .map(|e| {
            let kind_color = match e.kind {
                1000..=1999 => Color::Cyan,
                2000..=2999 => Color::Green,
                3000..=3999 => Color::Yellow,
                _ => Color::White,
            };
            Line::from(vec![
                Span::styled(format!("[{}] ", e.kind), Style::default().fg(kind_color)),
                Span::raw(&e.content_preview),
            ])
        })
        .collect();

    let p = Paragraph::new(lines).block(block).scroll((app.scroll_offsets[panel_idx], 0));
    f.render_widget(p, area);
}

fn render_active_bounties(f: &mut Frame, area: Rect, app: &DashboardApp, panel_idx: usize) {
    let block = panel_block("Active Bounties", app.selected_panel == panel_idx);

    if app.active_bounties.is_empty() {
        let p = Paragraph::new("No active bounties")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("ID").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Reward").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("State").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let rows: Vec<Row> = app
        .active_bounties
        .iter()
        .map(|b| {
            let state_color = match b.state.as_str() {
                "Open" => Color::Green,
                "Claimed" => Color::Cyan,
                "Submitted" => Color::Magenta,
                "UnderReview" => Color::Yellow,
                _ => Color::White,
            };
            Row::new(vec![
                Cell::from(b.id.chars().take(8).collect::<String>()),
                Cell::from(format!("{}", b.reward)),
                Cell::from(b.state.as_str()).style(Style::default().fg(state_color)),
            ])
        })
        .collect();

    let table =
        Table::new(rows, [Constraint::Length(10), Constraint::Length(12), Constraint::Length(14)])
            .header(header)
            .block(block);
    f.render_widget(table, area);
}

fn render_training_progress(f: &mut Frame, area: Rect, app: &DashboardApp, panel_idx: usize) {
    let block = panel_block("Training Progress", app.selected_panel == panel_idx);

    if app.training_jobs.is_empty() {
        let p = Paragraph::new("No active training jobs")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
        return;
    }

    let inner = block.inner(area);
    f.render_widget(block, area);

    let constraints: Vec<Constraint> =
        app.training_jobs.iter().map(|_| Constraint::Length(2)).collect();
    let rects =
        Layout::default().direction(Direction::Vertical).constraints(constraints).split(inner);

    for (i, job) in app.training_jobs.iter().enumerate() {
        if i >= rects.len() {
            break;
        }
        let label =
            format!("Job {}: {}", job.id.chars().take(6).collect::<String>(), job.description);
        let gauge = Gauge::default()
            .block(Block::default().title(label))
            .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
            .ratio(job.progress as f64 / 100.0)
            .label(format!("{}%", job.progress));
        f.render_widget(gauge, rects[i]);
    }
}

fn render_status_bar(f: &mut Frame, area: Rect) {
    let bar = Paragraph::new(Line::from(vec![Span::styled(
        " Tab:switch  ↑↓:scroll  q:quit ",
        Style::default().fg(Color::Black).bg(Color::DarkGray),
    )]));
    f.render_widget(bar, area);
}

fn format_balance(value: u128) -> String {
    if value >= 1_000_000_000_000 {
        format!("{:.2}T", value as f64 / 1_000_000_000_000.0)
    } else if value >= 1_000_000_000 {
        format!("{:.2}B", value as f64 / 1_000_000_000.0)
    } else if value >= 1_000_000 {
        format!("{:.2}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}K", value as f64 / 1_000.0)
    } else {
        format!("{}", value)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn dashboard_parse_alias() {
        use clap::Parser;
        let cli = crate::cli::Cli::try_parse_from(["agnetd", "dashboard"]).expect("parse");
        assert!(matches!(cli.command, crate::cli::Commands::Dashboard));
    }

    #[test]
    fn dashboard_parse_short_alias() {
        use clap::Parser;
        let cli = crate::cli::Cli::try_parse_from(["agnetd", "d"]).expect("parse alias");
        assert!(matches!(cli.command, crate::cli::Commands::Dashboard));
    }

    #[test]
    fn app_new_initializes_defaults() {
        let app = DashboardApp::new();
        assert_eq!(app.selected_panel, 0);
        assert_eq!(app.scroll_offsets, [0u16; 5]);
        assert!(app.mesh_status.is_none());
        assert_eq!(app.token_balances.len(), 4);
        assert_eq!(app.token_balances[0].token_type, "Compute");
        assert_eq!(app.token_balances[1].token_type, "Training");
        assert_eq!(app.token_balances[2].token_type, "Bandwidth");
        assert_eq!(app.token_balances[3].token_type, "Storage");
        assert!(app.feed_events.is_empty());
        assert!(app.active_bounties.is_empty());
        assert!(app.training_jobs.is_empty());
        assert!(!app.should_quit);
    }

    #[test]
    fn handle_key_quit() {
        let mut app = DashboardApp::new();
        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn handle_key_escape() {
        let mut app = DashboardApp::new();
        app.handle_key(key(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn handle_key_tab_cycles_forward() {
        let mut app = DashboardApp::new();
        assert_eq!(app.selected_panel, 0);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.selected_panel, 1);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.selected_panel, 2);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.selected_panel, 3);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.selected_panel, 4);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.selected_panel, 0);
    }

    #[test]
    fn handle_key_backtab_cycles_backward() {
        let mut app = DashboardApp::new();
        app.selected_panel = 0;
        app.handle_key(key(KeyCode::BackTab));
        assert_eq!(app.selected_panel, 4);
        app.handle_key(key(KeyCode::BackTab));
        assert_eq!(app.selected_panel, 3);
    }

    #[test]
    fn handle_key_down_scrolls() {
        let mut app = DashboardApp::new();
        app.selected_panel = 2;
        assert_eq!(app.scroll_offsets[2], 0);
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.scroll_offsets[2], 1);
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.scroll_offsets[2], 2);
    }

    #[test]
    fn handle_key_up_scrolls_back() {
        let mut app = DashboardApp::new();
        app.selected_panel = 0;
        app.scroll_offsets[0] = 5;
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.scroll_offsets[0], 4);
    }

    #[test]
    fn handle_key_up_clamps_at_zero() {
        let mut app = DashboardApp::new();
        app.selected_panel = 0;
        app.scroll_offsets[0] = 0;
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.scroll_offsets[0], 0);
    }

    #[test]
    fn handle_key_other_noop() {
        let mut app = DashboardApp::new();
        app.handle_key(key(KeyCode::Char('a')));
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('x')));
        assert_eq!(app.selected_panel, 0);
        assert!(!app.should_quit);
    }

    #[test]
    fn format_balance_units() {
        assert_eq!(format_balance(0), "0");
        assert_eq!(format_balance(42), "42");
        assert_eq!(format_balance(999), "999");
        assert_eq!(format_balance(1_500), "1.5K");
        assert_eq!(format_balance(2_500_000), "2.50M");
        assert_eq!(format_balance(3_500_000_000), "3.50B");
        assert_eq!(format_balance(4_500_000_000_000), "4.50T");
    }

    #[test]
    fn panel_block_creates_bordered_widget() {
        let _b = panel_block("Mesh Health", true);
    }

    #[test]
    fn panel_block_creates_bordered_widget_unselected() {
        let _b = panel_block("Tokens", false);
    }

    #[test]
    fn scroll_per_panel_independent() {
        let mut app = DashboardApp::new();
        app.selected_panel = 0;
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.scroll_offsets[0], 2);

        app.selected_panel = 3;
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.scroll_offsets[3], 1);
        assert_eq!(app.scroll_offsets[0], 2);
    }
}
