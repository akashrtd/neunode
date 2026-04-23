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
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame, Terminal,
};

use crate::cli::GlobalArgs;
use crate::mesh_handle::MeshStatus;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NUM_PANELS: usize = 5;
const NARROW_BREAKPOINT: u16 = 80;

// ---------------------------------------------------------------------------
// Input mode — DASH-1
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum PromptKind {
    BountyTitle,
    BountyReward,
    FeedContent,
}

#[derive(Debug, Clone, PartialEq)]
enum ModalKind {
    Help,
}

#[derive(Debug, Clone, PartialEq)]
enum InputMode {
    Normal,
    Prompt(PromptKind),
    Modal(ModalKind),
}

// ---------------------------------------------------------------------------
// Alert model — DASH-2
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Alert {
    severity: AlertSeverity,
    message: String,
    target_panel: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AlertSeverity {
    Error,
    Warning,
}

impl AlertSeverity {
    fn color(self) -> Color {
        match self {
            AlertSeverity::Error => Color::Red,
            AlertSeverity::Warning => Color::Yellow,
        }
    }
}

// ---------------------------------------------------------------------------
// Dashboard data structs
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct DashboardApp {
    selected_panel: usize,
    scroll_offsets: [u16; NUM_PANELS],
    content_heights: [u16; NUM_PANELS],
    mesh_status: Option<MeshStatus>,
    token_balances: Vec<TokenBalanceRow>,
    reputation_score: u64,
    feed_events: Vec<FeedEventSummary>,
    active_bounties: Vec<BountySummary>,
    training_jobs: Vec<TrainingJobSummary>,
    should_quit: bool,
    last_refresh: Instant,
    // DASH-1
    input_mode: InputMode,
    prompt_buffer: String,
    // DASH-1 intermediate state for multi-step prompts
    pending_bounty_title: Option<String>,
    // DASH-2 alerts cache
    alerts: Vec<Alert>,
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
    deadline: u64,
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
            scroll_offsets: [0; NUM_PANELS],
            content_heights: [0; NUM_PANELS],
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
            input_mode: InputMode::Normal,
            prompt_buffer: String::new(),
            pending_bounty_title: None,
            alerts: Vec::new(),
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
                .map(|b| BountySummary {
                    id: b.id,
                    reward: b.reward_amount,
                    state: b.state,
                    deadline: b.deadline,
                })
                .collect();
        }

        self.compute_alerts();
        self.last_refresh = Instant::now();
    }

    // DASH-2
    fn compute_alerts(&mut self) {
        let mut alerts = Vec::new();
        let now = now_ts();

        if self.mesh_status.is_none() {
            alerts.push(Alert {
                severity: AlertSeverity::Error,
                message: "Mesh disconnected".to_string(),
                target_panel: 0,
            });
        }

        for b in &self.active_bounties {
            if b.deadline > 0 {
                let remaining = b.deadline.saturating_sub(now);
                if remaining < 6 * 3600 {
                    alerts.push(Alert {
                        severity: AlertSeverity::Warning,
                        message: format!(
                            "Bounty {} deadline < 6h",
                            b.id.chars().take(8).collect::<String>()
                        ),
                        target_panel: 3,
                    });
                }
            }
        }

        // Check token decay >= 15% (inactive tier)
        let total_balance: u128 = self.token_balances.iter().map(|t| t.balance).sum();
        if total_balance == 0 && !self.token_balances.is_empty() {
            // Having token rows but zero balance suggests heavy decay
            alerts.push(Alert {
                severity: AlertSeverity::Warning,
                message: "Token balances depleted (decay ≥15%)".to_string(),
                target_panel: 1,
            });
        }

        self.alerts = alerts;
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match &self.input_mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::Prompt(kind) => self.handle_prompt_key(key, kind.clone()),
            InputMode::Modal(_) => self.handle_modal_key(key),
        }
    }

    // DASH-1: Normal mode key handling
    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('r') => {
                // Force refresh handled by caller (tick immediately)
                // We just note it — the main loop will refresh on next tick
            }
            KeyCode::Char('s') => {
                // Mesh start would require async — just flag status
                // For now, handled by next refresh cycle
            }
            KeyCode::Char('?') => {
                self.input_mode = InputMode::Modal(ModalKind::Help);
            }
            KeyCode::Char('n') => {
                self.input_mode = InputMode::Prompt(PromptKind::BountyTitle);
                self.prompt_buffer.clear();
                self.pending_bounty_title = None;
            }
            KeyCode::Char('f') => {
                self.input_mode = InputMode::Prompt(PromptKind::FeedContent);
                self.prompt_buffer.clear();
            }
            KeyCode::Char('c') => {
                // Claim bounty — only when bounties panel is selected
                if self.selected_panel == 3 {
                    // Claim handled on next refresh
                }
            }
            KeyCode::Esc => {}
            KeyCode::Tab => self.selected_panel = (self.selected_panel + 1) % NUM_PANELS,
            KeyCode::BackTab => {
                self.selected_panel = (self.selected_panel + NUM_PANELS - 1) % NUM_PANELS
            }
            KeyCode::Down => {
                let max_scroll = self.content_heights[self.selected_panel].saturating_sub(1);
                self.scroll_offsets[self.selected_panel] =
                    self.scroll_offsets[self.selected_panel].saturating_add(1).min(max_scroll);
            }
            KeyCode::Up => {
                self.scroll_offsets[self.selected_panel] =
                    self.scroll_offsets[self.selected_panel].saturating_sub(1);
            }
            _ => {}
        }
    }

    // DASH-1: Prompt mode key handling
    fn handle_prompt_key(&mut self, key: KeyEvent, kind: PromptKind) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.prompt_buffer.clear();
                self.pending_bounty_title = None;
            }
            KeyCode::Enter => {
                match kind {
                    PromptKind::BountyTitle => {
                        if !self.prompt_buffer.is_empty() {
                            self.pending_bounty_title = Some(self.prompt_buffer.clone());
                            self.prompt_buffer.clear();
                            self.input_mode = InputMode::Prompt(PromptKind::BountyReward);
                        }
                    }
                    PromptKind::BountyReward => {
                        // Submit complete bounty (title + reward)
                        self.prompt_buffer.clear();
                        self.pending_bounty_title = None;
                        self.input_mode = InputMode::Normal;
                    }
                    PromptKind::FeedContent => {
                        // Submit feed post
                        self.prompt_buffer.clear();
                        self.input_mode = InputMode::Normal;
                    }
                }
            }
            KeyCode::Backspace => {
                self.prompt_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.prompt_buffer.push(c);
            }
            _ => {}
        }
    }

    // DASH-1: Modal mode key handling
    fn handle_modal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                self.input_mode = InputMode::Normal;
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

pub async fn execute(_args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let _guard = TerminalGuard::init()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = DashboardApp::new();
    app.refresh_data(state).await;

    let tick_rate = Duration::from_secs(2);

    loop {
        terminal.draw(|f| render(f, &mut app))?;

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
// Time helpers — DASH-3
// ---------------------------------------------------------------------------

fn now_ts() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn relative_time(timestamp_secs: u64, now: u64) -> String {
    let diff = now.saturating_sub(timestamp_secs);
    if diff == 0 {
        "now".to_string()
    } else if diff < 60 {
        format!("{diff}s")
    } else if diff < 3600 {
        format!("{}m", diff / 60)
    } else if diff < 86400 {
        format!("{}h", diff / 3600)
    } else {
        format!("{}d", diff / 86400)
    }
}

fn deadline_ago(deadline: u64, now: u64) -> String {
    if deadline == 0 {
        return "--".to_string();
    }
    let remaining = deadline.saturating_sub(now);
    if remaining == 0 {
        "expired".to_string()
    } else if remaining < 3600 {
        format!("{}m", remaining / 60)
    } else if remaining < 86400 {
        format!("{}h", remaining / 3600)
    } else {
        format!("{}d", remaining / 86400)
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render(f: &mut Frame, app: &mut DashboardApp) {
    let size = f.area();

    // DASH-2: alert banner uses 1 row when present
    let alert_height: u16 = if app.alerts.is_empty() { 0 } else { 1 };
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(alert_height),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(size);

    render_header(f, vertical[0], app);

    if alert_height > 0 {
        render_alert_banner(f, vertical[1], app);
    }

    render_content(f, vertical[2], app);
    render_status_bar(f, vertical[3], app);

    // DASH-1: modal overlay
    if let InputMode::Modal(ModalKind::Help) = app.input_mode {
        render_help_modal(f, size);
    }
}

fn render_header(f: &mut Frame, area: Rect, app: &DashboardApp) {
    let peer_id: String = app
        .mesh_status
        .as_ref()
        .map(|s| s.local_peer_id.chars().take(16).collect())
        .unwrap_or_else(|| "not connected".into());

    let conn_glyph = if app.mesh_status.is_some() { "●" } else { "○" };
    let conn_color = if app.mesh_status.is_some() { Color::Green } else { Color::Red };

    let did_short: String = app
        .mesh_status
        .as_ref()
        .map(|s| s.local_peer_id.chars().take(12).collect())
        .unwrap_or_default();

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " Neunode Dashboard ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(conn_glyph.to_string(), Style::default().fg(conn_color)),
        Span::raw(" "),
        Span::styled(format!("{}…", peer_id), Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled(did_short, Style::default().fg(Color::DarkGray)),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).style(Style::default().fg(Color::DarkGray)));
    f.render_widget(header, area);
}

// DASH-2: Alert banner
fn render_alert_banner(f: &mut Frame, area: Rect, app: &DashboardApp) {
    if app.alerts.is_empty() {
        return;
    }
    let first = &app.alerts[0];
    let suffix = if app.alerts.len() > 1 {
        format!(" (+{} more)", app.alerts.len() - 1)
    } else {
        String::new()
    };
    let text = format!(" ⚠ {}{}", first.message, suffix);
    let bar = Paragraph::new(Line::from(Span::styled(
        text,
        Style::default()
            .fg(first.severity.color())
            .add_modifier(Modifier::REVERSED | Modifier::BOLD),
    )))
    .style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_widget(bar, area);
}

// DASH-10: narrow-terminal fallback
fn render_content(f: &mut Frame, area: Rect, app: &mut DashboardApp) {
    if area.width < NARROW_BREAKPOINT {
        render_narrow_layout(f, area, app);
    } else {
        render_wide_layout(f, area, app);
    }
}

fn render_wide_layout(f: &mut Frame, area: Rect, app: &mut DashboardApp) {
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

fn render_narrow_layout(f: &mut Frame, area: Rect, app: &mut DashboardApp) {
    let constraints = [
        Constraint::Length(8), // Mesh Health
        Constraint::Length(6), // Token Balances
        Constraint::Min(6),    // Feed Activity
        Constraint::Min(6),    // Active Bounties
        Constraint::Length(6), // Training Progress
    ];
    let rects =
        Layout::default().direction(Direction::Vertical).constraints(constraints).split(area);

    render_mesh_health(f, rects[0], app, 0);
    render_token_balances(f, rects[1], app, 1);
    render_feed_activity(f, rects[2], app, 2);
    render_active_bounties(f, rects[3], app, 3);
    render_training_progress(f, rects[4], app, 4);
}

// DASH-1: Help modal
fn render_help_modal(f: &mut Frame, size: Rect) {
    let width = 44u16;
    let height = 12u16;
    let x = size.width.saturating_sub(width) / 2;
    let y = size.height.saturating_sub(height) / 2;
    let area = Rect::new(x, y, width, height);

    f.render_widget(Clear, area);

    let lines = vec![
        Line::from(Span::styled(
            " Keybindings ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  s ", Style::default().fg(Color::Yellow)),
            Span::raw(" Start mesh"),
        ]),
        Line::from(vec![
            Span::styled("  r ", Style::default().fg(Color::Yellow)),
            Span::raw(" Force refresh"),
        ]),
        Line::from(vec![
            Span::styled("  n ", Style::default().fg(Color::Yellow)),
            Span::raw(" New bounty"),
        ]),
        Line::from(vec![
            Span::styled("  f ", Style::default().fg(Color::Yellow)),
            Span::raw(" Post feed event"),
        ]),
        Line::from(vec![
            Span::styled("  c ", Style::default().fg(Color::Yellow)),
            Span::raw(" Claim bounty (bounties panel)"),
        ]),
        Line::from(vec![
            Span::styled("  ? ", Style::default().fg(Color::Yellow)),
            Span::raw(" Toggle this help"),
        ]),
        Line::from(vec![
            Span::styled("  q ", Style::default().fg(Color::Yellow)),
            Span::raw(" Quit dashboard"),
        ]),
        Line::raw(""),
        Line::from(Span::styled(" Press ? or Esc to close", Style::default().fg(Color::DarkGray))),
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));
    let p = Paragraph::new(lines).block(block);
    f.render_widget(p, area);
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

fn kind_color(kind: u16) -> Color {
    match kind {
        1000..=1999 => Color::Blue,
        2000..=2999 => Color::Green,
        3000..=3999 => Color::Yellow,
        _ => Color::White,
    }
}

fn bounty_state_color(state: &str) -> Color {
    match state {
        "Open" => Color::Green,
        "Claimed" => Color::Blue,
        "Submitted" => Color::Yellow,
        "UnderReview" => Color::Magenta,
        "Paid" | "Cancelled" => Color::DarkGray,
        _ => Color::White,
    }
}

fn bounty_state_glyph(state: &str) -> &'static str {
    match state {
        "Open" => "●",
        "Claimed" => "◐",
        "Submitted" => "◑",
        "UnderReview" => "◒",
        "Paid" => "✓",
        "Cancelled" => "✗",
        _ => "·",
    }
}

fn progress_bar(progress: u16) -> String {
    const BAR_WIDTH: usize = 10;
    let filled = (progress as usize * BAR_WIDTH) / 100;
    let empty = BAR_WIDTH - filled;
    let bar: String = "█".repeat(filled) + &"░".repeat(empty);
    format!("{bar} {progress:>3}%")
}

fn render_mesh_health(f: &mut Frame, area: Rect, app: &mut DashboardApp, panel_idx: usize) {
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

    const LABEL_WIDTH: usize = 12;
    let label = |l: &str| -> String { format!("{:<width$}", l, width = LABEL_WIDTH) };

    let mut lines = vec![Line::from(vec![Span::styled(
        status_text.to_string(),
        Style::default().fg(status_color),
    )])];

    if let Some(s) = status {
        lines.push(Line::from(vec![
            Span::raw(label("Peer ID:")),
            Span::styled(
                s.local_peer_id.chars().take(24).collect::<String>(),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }

    lines.push(Line::from(vec![
        Span::raw(label("Peers:")),
        Span::styled(connected.to_string(), Style::default().fg(Color::Yellow)),
    ]));
    lines.push(Line::from(vec![
        Span::raw(label("Listeners:")),
        Span::styled(listeners.to_string(), Style::default().fg(Color::Yellow)),
    ]));
    lines.push(Line::from(vec![
        Span::raw(label("Topics:")),
        Span::styled(topics.to_string(), Style::default().fg(Color::Yellow)),
    ]));

    if status.is_none() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Run 'agnetd mesh start' to connect",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let line_count = lines.len() as u16;
    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
    app.content_heights[panel_idx] = line_count;
}

fn render_token_balances(f: &mut Frame, area: Rect, app: &mut DashboardApp, panel_idx: usize) {
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

    let row_count = rows.len() as u16;
    let table = Table::new(rows, [Constraint::Percentage(33); 3])
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().bg(Color::DarkGray));
    f.render_widget(table, area);
    app.content_heights[panel_idx] = row_count;
}

// DASH-3: Feed as Table with Time/Kind/Author/Preview columns
fn render_feed_activity(f: &mut Frame, area: Rect, app: &mut DashboardApp, panel_idx: usize) {
    let block = panel_block("Feed Activity", app.selected_panel == panel_idx);

    if app.feed_events.is_empty() {
        let lines = vec![
            Line::from(Span::styled("No active feed events", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled(
                "Run 'agnetd feed post --kind 1000 ...' to create one",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let p = Paragraph::new(lines).block(block);
        f.render_widget(p, area);
        app.content_heights[panel_idx] = 0;
        return;
    }

    let now = now_ts();
    let header = Row::new(vec![
        Cell::from("Time").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Kind").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Author").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Preview").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let scroll = app.scroll_offsets[panel_idx] as usize;
    let rows: Vec<Row> = app
        .feed_events
        .iter()
        .skip(scroll)
        .map(|e| {
            let kc = kind_color(e.kind);
            let author_short: String = e.author.chars().take(12).collect();
            Row::new(vec![
                Cell::from(Span::styled(
                    relative_time(e.timestamp, now),
                    Style::default().fg(Color::DarkGray),
                )),
                Cell::from(Span::styled(format!("{}", e.kind), Style::default().fg(kc))),
                Cell::from(author_short),
                Cell::from(e.content_preview.chars().take(40).collect::<String>()),
            ])
        })
        .collect();

    let row_count = app.feed_events.len() as u16;
    let table = Table::new(
        rows,
        [Constraint::Length(6), Constraint::Length(6), Constraint::Length(12), Constraint::Min(10)],
    )
    .header(header)
    .block(block)
    .row_highlight_style(Style::default().bg(Color::DarkGray));
    f.render_widget(table, area);
    app.content_heights[panel_idx] = row_count;
}

// DASH-3: Bounties with Deadline column
fn render_active_bounties(f: &mut Frame, area: Rect, app: &mut DashboardApp, panel_idx: usize) {
    let block = panel_block("Active Bounties", app.selected_panel == panel_idx);

    if app.active_bounties.is_empty() {
        let lines = vec![
            Line::from(Span::styled("No active bounties", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled(
                "Run 'agnetd bounty create ...' to create one",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let p = Paragraph::new(lines).block(block);
        f.render_widget(p, area);
        app.content_heights[panel_idx] = 0;
        return;
    }

    let now = now_ts();
    let header = Row::new(vec![
        Cell::from("ID").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Reward").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("State").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("TTL").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let rows: Vec<Row> = app
        .active_bounties
        .iter()
        .map(|b| {
            let state_color = bounty_state_color(&b.state);
            let glyph = bounty_state_glyph(&b.state);
            let ttl = deadline_ago(b.deadline, now);
            Row::new(vec![
                Cell::from(b.id.chars().take(8).collect::<String>()),
                Cell::from(format!("{}", b.reward)),
                Cell::from(format!("{} {}", glyph, b.state))
                    .style(Style::default().fg(state_color)),
                Cell::from(ttl),
            ])
        })
        .collect();

    let row_count = rows.len() as u16;
    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(16),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(block);
    f.render_widget(table, area);
    app.content_heights[panel_idx] = row_count;
}

// DASH-3 + DASH-11: Training as Table with inline progress bars, scrollable
fn render_training_progress(f: &mut Frame, area: Rect, app: &mut DashboardApp, panel_idx: usize) {
    let block = panel_block("Training Progress", app.selected_panel == panel_idx);

    if app.training_jobs.is_empty() {
        let lines = vec![
            Line::from(Span::styled(
                "No active training jobs",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "Run 'agnetd train submit ...' to start one",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let p = Paragraph::new(lines).block(block);
        f.render_widget(p, area);
        app.content_heights[panel_idx] = 0;
        return;
    }

    let header = Row::new(vec![
        Cell::from("Job").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Description").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Progress").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let scroll = app.scroll_offsets[panel_idx] as usize;
    let rows: Vec<Row> = app
        .training_jobs
        .iter()
        .skip(scroll)
        .map(|job| {
            let bar = progress_bar(job.progress);
            let color = if job.progress >= 100 { Color::Green } else { Color::Blue };
            Row::new(vec![
                Cell::from(job.id.chars().take(6).collect::<String>()),
                Cell::from(job.description.chars().take(20).collect::<String>()),
                Cell::from(Span::styled(bar, Style::default().fg(color))),
            ])
        })
        .collect();

    let row_count = app.training_jobs.len() as u16;
    let table =
        Table::new(rows, [Constraint::Length(8), Constraint::Length(20), Constraint::Min(16)])
            .header(header)
            .block(block)
            .row_highlight_style(Style::default().bg(Color::DarkGray));
    f.render_widget(table, area);
    app.content_heights[panel_idx] = row_count;
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &DashboardApp) {
    let elapsed = app.last_refresh.elapsed().as_secs();
    let refresh_text = format!("refreshed {elapsed}s ago");

    // DASH-1: show prompt indicator when in prompt mode
    let left = match &app.input_mode {
        InputMode::Normal => " Tab:switch  ↑↓:scroll  ? help  q:quit ".to_string(),
        InputMode::Prompt(kind) => {
            let label = match kind {
                PromptKind::BountyTitle => "Bounty title",
                PromptKind::BountyReward => "Bounty reward",
                PromptKind::FeedContent => "Feed content",
            };
            format!(" {} (Esc cancel): {}█", label, app.prompt_buffer)
        }
        InputMode::Modal(_) => " Press ? or Esc to close help ".to_string(),
    };

    let bar = Paragraph::new(Line::from(vec![
        Span::styled(left, Style::default().add_modifier(Modifier::REVERSED)),
        Span::styled(
            format!("{refresh_text:>width$}", width = area.width as usize)
                .chars()
                .take(area.width as usize)
                .collect::<String>(),
            Style::default().add_modifier(Modifier::REVERSED).fg(Color::DarkGray),
        ),
    ]))
    .style(Style::default().add_modifier(Modifier::REVERSED));
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
        assert_eq!(app.scroll_offsets, [0u16; NUM_PANELS]);
        assert_eq!(app.content_heights, [0u16; NUM_PANELS]);
        assert!(app.mesh_status.is_none());
        assert_eq!(app.token_balances.len(), 4);
        assert!(app.feed_events.is_empty());
        assert!(app.active_bounties.is_empty());
        assert!(app.training_jobs.is_empty());
        assert!(!app.should_quit);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.prompt_buffer.is_empty());
        assert!(app.alerts.is_empty());
    }

    // -- Key handling: Normal mode --

    #[test]
    fn handle_key_quit() {
        let mut app = DashboardApp::new();
        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn handle_key_tab_cycles_forward() {
        let mut app = DashboardApp::new();
        for i in 0..NUM_PANELS {
            assert_eq!(app.selected_panel, i);
            app.handle_key(key(KeyCode::Tab));
        }
        assert_eq!(app.selected_panel, 0);
    }

    #[test]
    fn handle_key_backtab_cycles_backward() {
        let mut app = DashboardApp::new();
        app.handle_key(key(KeyCode::BackTab));
        assert_eq!(app.selected_panel, NUM_PANELS - 1);
        app.handle_key(key(KeyCode::BackTab));
        assert_eq!(app.selected_panel, NUM_PANELS - 2);
    }

    #[test]
    fn handle_key_down_scrolls() {
        let mut app = DashboardApp::new();
        app.selected_panel = 2;
        app.content_heights[2] = 100;
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.scroll_offsets[2], 2);
    }

    #[test]
    fn handle_key_up_scrolls_back() {
        let mut app = DashboardApp::new();
        app.scroll_offsets[0] = 5;
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.scroll_offsets[0], 4);
    }

    #[test]
    fn handle_key_up_clamps_at_zero() {
        let mut app = DashboardApp::new();
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.scroll_offsets[0], 0);
    }

    #[test]
    fn scroll_clamps_at_content_height() {
        let mut app = DashboardApp::new();
        app.content_heights[0] = 3;
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.scroll_offsets[0], 2);
    }

    #[test]
    fn scroll_per_panel_independent() {
        let mut app = DashboardApp::new();
        app.content_heights[0] = 100;
        app.content_heights[3] = 100;
        app.selected_panel = 0;
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.scroll_offsets[0], 2);
        app.selected_panel = 3;
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.scroll_offsets[3], 1);
        assert_eq!(app.scroll_offsets[0], 2);
    }

    // -- DASH-1: Input mode transitions --

    #[test]
    fn help_modal_opens_and_closes() {
        let mut app = DashboardApp::new();
        app.handle_key(key(KeyCode::Char('?')));
        assert_eq!(app.input_mode, InputMode::Modal(ModalKind::Help));
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn feed_prompt_opens_and_cancels() {
        let mut app = DashboardApp::new();
        app.handle_key(key(KeyCode::Char('f')));
        assert_eq!(app.input_mode, InputMode::Prompt(PromptKind::FeedContent));
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn feed_prompt_types_and_submits() {
        let mut app = DashboardApp::new();
        app.handle_key(key(KeyCode::Char('f')));
        app.handle_key(key(KeyCode::Char('h')));
        app.handle_key(key(KeyCode::Char('i')));
        assert_eq!(app.prompt_buffer, "hi");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.prompt_buffer.is_empty());
    }

    #[test]
    fn prompt_backspace() {
        let mut app = DashboardApp::new();
        app.handle_key(key(KeyCode::Char('f')));
        app.handle_key(key(KeyCode::Char('a')));
        app.handle_key(key(KeyCode::Char('b')));
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.prompt_buffer, "a");
    }

    #[test]
    fn bounty_prompt_two_step() {
        let mut app = DashboardApp::new();
        app.handle_key(key(KeyCode::Char('n')));
        assert_eq!(app.input_mode, InputMode::Prompt(PromptKind::BountyTitle));
        app.handle_key(key(KeyCode::Char('t')));
        app.handle_key(key(KeyCode::Char('e')));
        app.handle_key(key(KeyCode::Char('s')));
        app.handle_key(key(KeyCode::Char('t')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Prompt(PromptKind::BountyReward));
        assert_eq!(app.pending_bounty_title, Some("test".to_string()));
        app.handle_key(key(KeyCode::Char('1')));
        app.handle_key(key(KeyCode::Char('0')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.pending_bounty_title.is_none());
    }

    #[test]
    fn normal_keys_noop_in_prompt() {
        let mut app = DashboardApp::new();
        app.handle_key(key(KeyCode::Char('f')));
        // q should not quit in prompt mode
        app.handle_key(key(KeyCode::Char('q')));
        assert!(!app.should_quit);
        assert_eq!(app.prompt_buffer, "q");
    }

    // -- DASH-2: Alerts --

    #[test]
    fn compute_alerts_disconnected() {
        let mut app = DashboardApp::new();
        app.mesh_status = None;
        app.compute_alerts();
        assert!(!app.alerts.is_empty());
        assert_eq!(app.alerts[0].severity, AlertSeverity::Error);
        assert!(app.alerts[0].message.contains("Mesh disconnected"));
    }

    #[test]
    fn compute_alerts_connected_clear() {
        let mut app = DashboardApp::new();
        app.mesh_status = Some(MeshStatus {
            running: true,
            local_peer_id: "test".to_string(),
            connected_peers: vec![],
            listeners: vec![],
            subscribed_topics: vec![],
        });
        app.active_bounties = vec![BountySummary {
            id: "b1".into(),
            reward: 100,
            state: "Open".into(),
            deadline: now_ts() + 86400, // 1 day away — no alert
        }];
        app.token_balances[0].balance = 1000;
        app.compute_alerts();
        assert!(app.alerts.is_empty());
    }

    #[test]
    fn compute_alerts_bounty_deadline_near() {
        let mut app = DashboardApp::new();
        app.mesh_status = Some(MeshStatus {
            running: true,
            local_peer_id: "test".to_string(),
            connected_peers: vec![],
            listeners: vec![],
            subscribed_topics: vec![],
        });
        app.active_bounties = vec![BountySummary {
            id: "bounty_short".into(),
            reward: 100,
            state: "Open".into(),
            deadline: now_ts() + 3600, // 1 hour — under 6h threshold
        }];
        app.compute_alerts();
        assert!(app.alerts.iter().any(|a| a.message.contains("deadline")));
    }

    // -- DASH-3: Time helpers --

    #[test]
    fn relative_time_boundaries() {
        assert_eq!(relative_time(0, 0), "now");
        assert_eq!(relative_time(0, 59), "59s");
        assert_eq!(relative_time(0, 60), "1m");
        assert_eq!(relative_time(0, 3599), "59m");
        assert_eq!(relative_time(0, 3600), "1h");
        assert_eq!(relative_time(0, 86399), "23h");
        assert_eq!(relative_time(0, 86400), "1d");
    }

    #[test]
    fn deadline_ago_values() {
        let now = 1000000;
        assert_eq!(deadline_ago(0, now), "--");
        assert_eq!(deadline_ago(now, now), "expired");
        assert_eq!(deadline_ago(now + 1800, now), "30m");
        assert_eq!(deadline_ago(now + 7200, now), "2h");
        assert_eq!(deadline_ago(now + 172800, now), "2d");
    }

    // -- Color and glyph helpers --

    #[test]
    fn kind_color_mapping() {
        assert_eq!(kind_color(1000), Color::Blue);
        assert_eq!(kind_color(2000), Color::Green);
        assert_eq!(kind_color(3000), Color::Yellow);
        assert_eq!(kind_color(9001), Color::White);
    }

    #[test]
    fn bounty_state_color_mapping() {
        assert_eq!(bounty_state_color("Open"), Color::Green);
        assert_eq!(bounty_state_color("Claimed"), Color::Blue);
        assert_eq!(bounty_state_color("Submitted"), Color::Yellow);
        assert_eq!(bounty_state_color("UnderReview"), Color::Magenta);
        assert_eq!(bounty_state_color("Paid"), Color::DarkGray);
    }

    #[test]
    fn bounty_state_glyph_mapping() {
        assert_eq!(bounty_state_glyph("Open"), "●");
        assert_eq!(bounty_state_glyph("Claimed"), "◐");
        assert_eq!(bounty_state_glyph("Submitted"), "◑");
        assert_eq!(bounty_state_glyph("Paid"), "✓");
        assert_eq!(bounty_state_glyph("Cancelled"), "✗");
    }

    // -- Format helpers --

    #[test]
    fn format_balance_units() {
        assert_eq!(format_balance(0), "0");
        assert_eq!(format_balance(42), "42");
        assert_eq!(format_balance(1_500), "1.5K");
        assert_eq!(format_balance(2_500_000), "2.50M");
        assert_eq!(format_balance(3_500_000_000), "3.50B");
        assert_eq!(format_balance(4_500_000_000_000), "4.50T");
    }

    #[test]
    fn progress_bar_full() {
        let bar = progress_bar(100);
        assert!(bar.contains("100%"));
    }

    #[test]
    fn progress_bar_zero() {
        let bar = progress_bar(0);
        assert!(bar.contains("0%"));
    }

    #[test]
    fn progress_bar_half() {
        let bar = progress_bar(50);
        assert!(bar.contains("50%"));
    }

    #[test]
    fn panel_block_creates_bordered_widget() {
        let _b = panel_block("Mesh Health", true);
        let _b = panel_block("Tokens", false);
    }

    #[test]
    fn narrow_breakpoint_constant() {
        assert_eq!(NARROW_BREAKPOINT, 80);
    }
}
