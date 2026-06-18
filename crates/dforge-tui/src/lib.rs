use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
    Frame, Terminal,
};
use std::io;

#[derive(Debug, Clone, PartialEq)]
enum Panel {
    Files,
    Diff,
    Commits,
    Branches,
    Prs,
    Issues,
}

#[derive(Debug, Clone, PartialEq)]
enum Tab {
    Main,
    Search,
}

struct App {
    active_panel: Panel,
    active_tab: Tab,
    branch: String,
    files: Vec<String>,
    commits: Vec<String>,
    branches: Vec<String>,
    prs: Vec<String>,
    issues: Vec<String>,
    diff_lines: Vec<(char, String)>, // ('+'/'-'/' ', content)
    search_query: String,
    search_results: Vec<String>,
    search_mode: bool,
    files_state: ListState,
    commits_state: ListState,
    branches_state: ListState,
    prs_state: ListState,
    issues_state: ListState,
    search_state: ListState,
    status_msg: String,
    peers: u32,
}

impl App {
    fn new() -> Self {
        let mut files_state = ListState::default();
        files_state.select(Some(0));

        Self {
            active_panel: Panel::Files,
            active_tab: Tab::Main,
            branch: "main".to_string(),
            files: vec![
                "  src/main.rs".to_string(),
                "  src/lib.rs".to_string(),
                "M Cargo.toml".to_string(),
                "? README.md".to_string(),
            ],
            commits: vec![
                "a3f2c1d feat: add SSS crypto".to_string(),
                "b7d8911 feat: init repo".to_string(),
                "c2a4451 initial commit".to_string(),
            ],
            branches: vec![
                "* main".to_string(),
                "  dev".to_string(),
                "  feat/search".to_string(),
            ],
            prs: vec![
                "#2 feat: trigram search".to_string(),
                "#1 fix: auth bug".to_string(),
            ],
            issues: vec![
                "● #3 improve push perf".to_string(),
                "● #2 add web UI".to_string(),
                "✓ #1 initial setup".to_string(),
            ],
            diff_lines: vec![
                (' ', "use dforge_crypto::sss;".to_string()),
                ('+', "use dforge_ipfs::IpfsClient;".to_string()),
                ('+', "".to_string()),
                (' ', "fn main() {".to_string()),
                ('-', "    println!(\"hello\");".to_string()),
                ('+', "    node::start().await?;".to_string()),
                (' ', "}".to_string()),
            ],
            search_query: String::new(),
            search_results: Vec::new(),
            search_mode: false,
            files_state,
            commits_state: ListState::default(),
            branches_state: ListState::default(),
            prs_state: ListState::default(),
            issues_state: ListState::default(),
            search_state: ListState::default(),
            status_msg: "Ready — [p]ush [l]pull [c]ommit [b]ranch [/]search [q]uit".to_string(),
            peers: 47,
        }
    }
}

pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if app.search_mode {
                match key.code {
                    KeyCode::Esc => {
                        app.search_mode = false;
                        app.search_query.clear();
                    }
                    KeyCode::Backspace => { app.search_query.pop(); }
                    KeyCode::Char(c) => { app.search_query.push(c); }
                    KeyCode::Enter => {
                        // Perform search
                        app.search_results = vec![
                            format!("src/main.rs:14 | {}", app.search_query),
                            format!("src/lib.rs:7  | fn {}()", app.search_query),
                        ];
                        app.search_mode = false;
                        app.active_tab = Tab::Search;
                    }
                    _ => {}
                }
                continue;
            }

            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),

                // Navigation between panels
                KeyCode::Tab => {
                    app.active_panel = match app.active_panel {
                        Panel::Files    => Panel::Diff,
                        Panel::Diff     => Panel::Commits,
                        Panel::Commits  => Panel::Branches,
                        Panel::Branches => Panel::Prs,
                        Panel::Prs      => Panel::Issues,
                        Panel::Issues   => Panel::Files,
                    };
                }

                // Vim keys
                KeyCode::Char('j') | KeyCode::Down => scroll_down(app),
                KeyCode::Char('k') | KeyCode::Up   => scroll_up(app),

                // Actions
                KeyCode::Char('p') => {
                    app.status_msg = "Pushing to IPFS... (run: dforge push)".to_string();
                }
                KeyCode::Char('l') => {
                    app.status_msg = "Pulling from IPFS... (run: dforge pull)".to_string();
                }
                KeyCode::Char('c') => {
                    app.status_msg = "Enter commit message (run: dforge commit -m \"...\")".to_string();
                }
                KeyCode::Char('b') => {
                    app.active_panel = Panel::Branches;
                }
                KeyCode::Char('r') => {
                    app.active_panel = Panel::Prs;
                }
                KeyCode::Char('i') => {
                    app.active_panel = Panel::Issues;
                }
                KeyCode::Char('/') => {
                    app.search_mode = true;
                    app.search_query.clear();
                    app.active_tab = Tab::Main;
                }
                KeyCode::Char('1') => app.active_tab = Tab::Main,
                KeyCode::Char('2') => app.active_tab = Tab::Search,
                KeyCode::Esc => {
                    app.active_tab = Tab::Main;
                    app.status_msg = "Ready — [p]ush [l]pull [c]ommit [b]ranch [/]search [q]uit".to_string();
                }
                _ => {}
            }
        }
    }
}

fn scroll_down(app: &mut App) {
    match app.active_panel {
        Panel::Files => {
            let i = app.files_state.selected().unwrap_or(0);
            app.files_state.select(Some((i + 1).min(app.files.len().saturating_sub(1))));
        }
        Panel::Commits => {
            let i = app.commits_state.selected().unwrap_or(0);
            app.commits_state.select(Some((i + 1).min(app.commits.len().saturating_sub(1))));
        }
        Panel::Branches => {
            let i = app.branches_state.selected().unwrap_or(0);
            app.branches_state.select(Some((i + 1).min(app.branches.len().saturating_sub(1))));
        }
        _ => {}
    }
}

fn scroll_up(app: &mut App) {
    match app.active_panel {
        Panel::Files => {
            let i = app.files_state.selected().unwrap_or(0);
            app.files_state.select(Some(i.saturating_sub(1)));
        }
        Panel::Commits => {
            let i = app.commits_state.selected().unwrap_or(0);
            app.commits_state.select(Some(i.saturating_sub(1)));
        }
        Panel::Branches => {
            let i = app.branches_state.selected().unwrap_or(0);
            app.branches_state.select(Some(i.saturating_sub(1)));
        }
        _ => {}
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    // Header bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),    // main
            Constraint::Length(1), // status bar
        ])
        .split(size);

    // Header: title + node info + tab bar
    render_header(f, app, chunks[0]);

    // Main area: left panels + right diff
    if app.active_tab == Tab::Search {
        render_search(f, app, chunks[1]);
    } else {
        render_main(f, app, chunks[1]);
    }

    // Status bar
    let status = Paragraph::new(app.status_msg.as_str())
        .style(Style::default().fg(Color::Black).bg(Color::DarkGray));
    f.render_widget(status, chunks[2]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .style(Style::default().bg(Color::Black));

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Left: title + branch
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" DecentraForge ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("│ "),
        Span::styled("branch: ", Style::default().fg(Color::DarkGray)),
        Span::styled(&app.branch, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(format!("peers: {}", app.peers), Style::default().fg(Color::Blue)),
    ])).block(block.clone());
    f.render_widget(title, header_chunks[0]);

    // Right: tab bar
    let titles = vec!["[1] Main", "[2] Search"];
    let selected = match app.active_tab { Tab::Main => 0, Tab::Search => 1 };
    let tabs = Tabs::new(titles)
        .block(block)
        .select(selected)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, header_chunks[1]);
}

fn render_main(f: &mut Frame, app: &App, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    // Left column: stacked panels
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30), // files
            Constraint::Percentage(30), // commits
            Constraint::Percentage(40), // branches + prs + issues
        ])
        .split(main_chunks[0]);

    render_panel_files(f, app, left_chunks[0]);
    render_panel_commits(f, app, left_chunks[1]);

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ])
        .split(left_chunks[2]);
    render_panel_branches(f, app, bottom_chunks[0]);
    render_panel_prs(f, app, bottom_chunks[1]);
    render_panel_issues(f, app, bottom_chunks[2]);

    // Right: diff view
    render_diff(f, app, main_chunks[1]);
}

fn render_panel_files(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Files;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app.files.iter().map(|f| {
        let color = if f.starts_with('M') { Color::Yellow }
            else if f.starts_with('?') { Color::Red }
            else { Color::White };
        ListItem::new(f.as_str()).style(Style::default().fg(color))
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Files").border_style(border_style))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    let mut state = app.files_state.clone();
    f.render_stateful_widget(list, area, &mut state);
}

fn render_panel_commits(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Commits;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app.commits.iter().map(|c| {
        let (hash, rest) = c.split_at(7.min(c.len()));
        ListItem::new(Line::from(vec![
            Span::styled(hash, Style::default().fg(Color::Yellow)),
            Span::raw(rest),
        ]))
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Commits").border_style(border_style))
        .highlight_style(Style::default().bg(Color::DarkGray));

    let mut state = app.commits_state.clone();
    f.render_stateful_widget(list, area, &mut state);
}

fn render_panel_branches(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Branches;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app.branches.iter().map(|b| {
        let color = if b.starts_with('*') { Color::Green } else { Color::White };
        ListItem::new(b.as_str()).style(Style::default().fg(color))
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Branch").border_style(border_style));

    let mut state = app.branches_state.clone();
    f.render_stateful_widget(list, area, &mut state);
}

fn render_panel_prs(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Prs;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app.prs.iter().map(|p| {
        ListItem::new(p.as_str()).style(Style::default().fg(Color::Magenta))
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("PRs").border_style(border_style));

    let mut state = app.prs_state.clone();
    f.render_stateful_widget(list, area, &mut state);
}

fn render_panel_issues(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Issues;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app.issues.iter().map(|i| {
        let color = if i.starts_with('●') { Color::Green } else { Color::DarkGray };
        ListItem::new(i.as_str()).style(Style::default().fg(color))
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Issues").border_style(border_style));

    let mut state = app.issues_state.clone();
    f.render_stateful_widget(list, area, &mut state);
}

fn render_diff(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Diff;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let search_bar = if app.search_mode {
        format!(" Search: {}█", app.search_query)
    } else {
        String::new()
    };

    let lines: Vec<Line> = app.diff_lines.iter().enumerate().map(|(i, (kind, content))| {
        let (prefix, color) = match kind {
            '+' => ("+", Color::Green),
            '-' => ("-", Color::Red),
            _   => (" ", Color::White),
        };
        Line::from(vec![
            Span::styled(format!("{:4} ", i + 1), Style::default().fg(Color::DarkGray)),
            Span::styled(prefix, Style::default().fg(color)),
            Span::styled(content.as_str(), Style::default().fg(color)),
        ])
    }).collect();

    let title = if search_bar.is_empty() { "Diff" } else { "Diff (searching)" };
    let diff = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title).border_style(border_style));

    f.render_widget(diff, area);

    // Overlay search bar at bottom of diff panel
    if app.search_mode {
        let search_area = Rect {
            x: area.x + 1,
            y: area.y + area.height - 2,
            width: area.width - 2,
            height: 1,
        };
        let search = Paragraph::new(search_bar.as_str())
            .style(Style::default().fg(Color::Yellow).bg(Color::Black));
        f.render_widget(search, search_area);
    }
}

fn render_search(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let query_display = format!(" Query: '{}' — {} results", app.search_query, app.search_results.len());
    let query_para = Paragraph::new(query_display.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Code Search"));
    f.render_widget(query_para, chunks[0]);

    let items: Vec<ListItem> = app.search_results.iter().map(|r| {
        let parts: Vec<&str> = r.splitn(2, " | ").collect();
        if parts.len() == 2 {
            ListItem::new(Line::from(vec![
                Span::styled(parts[0], Style::default().fg(Color::Cyan)),
                Span::raw(" │ "),
                Span::raw(parts[1]),
            ]))
        } else {
            ListItem::new(r.as_str())
        }
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Results"))
        .highlight_style(Style::default().bg(Color::DarkGray));

    let mut state = app.search_state.clone();
    f.render_stateful_widget(list, chunks[1], &mut state);
}
