use anyhow::Result;
use clap::Args;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use serde::Deserialize;
use std::{
    fs,
    io::{self, Stdout},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

#[derive(Args)]
pub struct DashboardArgs {
    /// Auto-refresh interval in seconds (0 = no auto-refresh)
    #[arg(short, long, default_value = "0")]
    pub refresh: u64,
}

#[derive(Default)]
struct MovePackage {
    name: Option<String>,
    version: Option<String>,
    edition: Option<String>,
}

struct DashboardState {
    active_address: String,
    active_env: String,
    balance: String,
    move_pkg: MovePackage,
    last_error: Option<String>,
}

impl DashboardState {
    fn loading() -> Self {
        DashboardState {
            active_address: "Loading…".to_string(),
            active_env: "Loading…".to_string(),
            balance: "Loading…".to_string(),
            move_pkg: MovePackage::default(),
            last_error: None,
        }
    }

    fn load() -> Self {
        let active_address = run_sui_cmd(&["client", "active-address"]);
        let active_env = run_sui_cmd(&["client", "active-env"]);
        let balance = load_balance();
        let move_pkg = load_move_toml();

        DashboardState {
            active_address,
            active_env,
            balance,
            move_pkg,
            last_error: None,
        }
    }
}

// ── Data loaders ─────────────────────────────────────────────────────────────

fn run_sui_cmd(args: &[&str]) -> String {
    let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = Command::new("sui")
            .args(&owned)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "— (sui CLI not found)".to_string());
        let _ = tx.send(result);
    });
    rx.recv_timeout(Duration::from_secs(5))
        .unwrap_or_else(|_| "— (timeout)".to_string())
}

#[derive(Deserialize)]
struct CoinBalance {
    #[serde(rename = "coinType")]
    coin_type: String,
    #[serde(rename = "totalBalance")]
    total_balance: String,
}

fn load_balance() -> String {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = Command::new("sui")
            .args(["client", "balance", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| serde_json::from_str::<Vec<CoinBalance>>(&s).ok())
            .and_then(|coins| {
                coins
                    .into_iter()
                    .find(|c| c.coin_type.contains("SUI"))
                    .map(|c| format_sui_balance(&c.total_balance))
            })
            .unwrap_or_else(|| "—".to_string());
        let _ = tx.send(result);
    });
    rx.recv_timeout(Duration::from_secs(5))
        .unwrap_or_else(|_| "— (timeout)".to_string())
}

fn format_sui_balance(mist: &str) -> String {
    let mist: u128 = mist.parse().unwrap_or(0);
    let sui = mist as f64 / 1_000_000_000.0;
    format!("{:.4} SUI", sui)
}

fn load_move_toml() -> MovePackage {
    let content = match fs::read_to_string("Move.toml") {
        Ok(c) => c,
        Err(_) => return MovePackage::default(),
    };
    let mut pkg = MovePackage::default();
    let mut in_package = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[package]" {
            in_package = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_package = false;
            continue;
        }
        if in_package {
            if let Some(v) = toml_value(trimmed, "name") {
                pkg.name = Some(v);
            } else if let Some(v) = toml_value(trimmed, "version") {
                pkg.version = Some(v);
            } else if let Some(v) = toml_value(trimmed, "edition") {
                pkg.edition = Some(v);
            }
        }
    }
    pkg
}

fn toml_value(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{} =", key);
    if !line.starts_with(&prefix) {
        return None;
    }
    let val = line[prefix.len()..].trim().trim_matches('"');
    Some(val.to_string())
}

// ── TUI entry point ───────────────────────────────────────────────────────────

pub async fn run(args: DashboardArgs) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, args.refresh);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<Stdout>>,
    refresh_secs: u64,
) -> Result<()> {
    // Draw immediately so the UI is visible while data loads
    let mut state = DashboardState::loading();
    terminal.draw(|frame| draw(frame, &state))?;
    state = DashboardState::load();

    let poll_timeout = Duration::from_millis(250);

    loop {
        terminal.draw(|frame| draw(frame, &state))?;

        let auto_refresh = refresh_secs > 0;
        let timeout = if auto_refresh {
            Duration::from_secs(refresh_secs)
        } else {
            poll_timeout
        };

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        state = DashboardState::load();
                    }
                    _ => {}
                }
            }
        } else if auto_refresh {
            state = DashboardState::load();
        }
    }

    Ok(())
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn draw(frame: &mut ratatui::Frame, state: &DashboardState) {
    let area = frame.area();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "  sui-runner ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled("dashboard", Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(title, outer[0]);

    // Body: two columns
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[1]);

    // Left — Wallet
    let wallet_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Address",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(&state.active_address, Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Network",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(&state.active_env, Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Balance",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(&state.balance, Style::default().fg(Color::Cyan)),
        ]),
    ];
    let wallet = Paragraph::new(wallet_lines).block(
        Block::default()
            .title(" Wallet ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(wallet, body[0]);

    // Right — Move Package
    let pkg = &state.move_pkg;
    let project_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Name     ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                pkg.name.as_deref().unwrap_or("—"),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Version  ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                pkg.version.as_deref().unwrap_or("—"),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Edition  ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                pkg.edition.as_deref().unwrap_or("—"),
                Style::default().fg(Color::Green),
            ),
        ]),
    ];
    let proj_widget = Paragraph::new(project_lines).block(
        Block::default()
            .title(" Move Package ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(proj_widget, body[1]);

    // Footer
    let footer_text = if let Some(err) = &state.last_error {
        Line::from(Span::styled(
            format!("  Error: {}", err),
            Style::default().fg(Color::Red),
        ))
    } else {
        Line::from(vec![
            Span::styled(
                "  [r]",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" refresh  "),
            Span::styled(
                "[q]",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" quit"),
        ])
    };
    let footer = Paragraph::new(footer_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(footer, outer[2]);
}
