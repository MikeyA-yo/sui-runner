use anyhow::Result;
use clap::Args;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use duct::cmd;
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
    time::Duration,
};

#[derive(Args)]
pub struct DashboardArgs {
    /// Auto-refresh interval in seconds (0 = no auto-refresh)
    #[arg(short, long, default_value = "0")]
    pub refresh: u64,
}

#[derive(Deserialize, Default)]
struct ProjectConfig {
    name: Option<String>,
    network: Option<String>,
    version: Option<String>,
}

struct DashboardState {
    active_address: String,
    active_env: String,
    project: ProjectConfig,
    last_error: Option<String>,
}

impl DashboardState {
    fn load() -> Self {
        let project = fs::read_to_string("sui-runner.json")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let active_address = cmd!("sui", "client", "active-address")
            .read()
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "— (sui CLI not found)".to_string());

        let active_env = cmd!("sui", "client", "active-env")
            .read()
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "— (sui CLI not found)".to_string());

        DashboardState {
            active_address,
            active_env,
            project,
            last_error: None,
        }
    }
}

pub async fn run(args: DashboardArgs) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, args.refresh);

    // Always restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<Stdout>>,
    refresh_secs: u64,
) -> Result<()> {
    let mut state = DashboardState::load();
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

fn draw(frame: &mut ratatui::Frame, state: &DashboardState) {
    let area = frame.area();

    // Outer vertical split: header | body | footer
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),    // body
            Constraint::Length(3), // footer
        ])
        .split(area);

    // ── Header ──────────────────────────────────────────────────────────────
    let title = Paragraph::new(Line::from(vec![
        Span::styled("  sui-runner ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("dashboard", Style::default().fg(Color::White)),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
    frame.render_widget(title, outer[0]);

    // ── Body: two columns ───────────────────────────────────────────────────
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[1]);

    // Left — Wallet
    let wallet_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Address  ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(&state.active_address, Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Network  ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(&state.active_env, Style::default().fg(Color::Green)),
        ]),
    ];
    let wallet = Paragraph::new(wallet_lines).block(
        Block::default()
            .title(" Wallet ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(wallet, body[0]);

    // Right — Project
    let project = &state.project;
    let project_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Name     ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(
                project.name.as_deref().unwrap_or("—"),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Network  ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(
                project.network.as_deref().unwrap_or("—"),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Version  ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(
                project.version.as_deref().unwrap_or("—"),
                Style::default().fg(Color::White),
            ),
        ]),
    ];
    let proj_widget = Paragraph::new(project_lines).block(
        Block::default()
            .title(" Project (sui-runner.json) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(proj_widget, body[1]);

    // ── Footer ───────────────────────────────────────────────────────────────
    let footer_text = if let Some(err) = &state.last_error {
        Line::from(Span::styled(format!("  Error: {}", err), Style::default().fg(Color::Red)))
    } else {
        Line::from(vec![
            Span::styled("  [r]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" refresh  "),
            Span::styled("[q]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
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
