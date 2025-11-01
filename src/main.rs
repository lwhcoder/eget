// src/main.rs
mod log;
mod app;

use crate::app::{App, InputMode};
use crate::log::{load_log, mark_as_removed};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::io::{self, stdout};
use std::process::Command;

fn main() -> anyhow::Result<()> {
    // load data
    let entries = load_log();
    let app = App::new(entries);

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // main loop
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, LeaveAlternateScreen)?;

    if let Err(err) = res {
        eprintln!("error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<()> 
{
    loop {
        terminal.draw(|f| ui(f, &app))?;

        // input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        InputMode::Normal => {
                            match key.code {
                                KeyCode::Char('q') => return Ok(()),
                                KeyCode::Down | KeyCode::Char('j') => app.next(),
                                KeyCode::Up | KeyCode::Char('k') => app.prev(),
                                KeyCode::Char('/') => {
                                    app.input_mode = InputMode::Filter;
                                    app.filter_input.clear();
                                }
                                KeyCode::Char('d') => {
                                    if let Some(entry) = app.current() {
                                        let path = entry.path.clone();
                                        // Attempt to remove the binary
                                        if std::fs::remove_file(&path).is_ok() {
                                            // Mark as removed in log
                                            let _ = mark_as_removed(&path);
                                            // Reload entries
                                            let entries = load_log();
                                            app = App::new(entries);
                                        }
                                    }
                                }
                                KeyCode::Char('u') | KeyCode::Char('r') => {
                                    // Reinstall/update selected tool
                                    if let Some(entry) = app.current() {
                                        let repo = entry.repo.clone();
                                        // Exit TUI temporarily
                                        disable_raw_mode()?;
                                        execute!(io::stdout(), LeaveAlternateScreen)?;
                                        
                                        // Run eget command
                                        println!("Running: eget {}", repo);
                                        let status = Command::new("eget")
                                            .arg(&repo)
                                            .status();
                                        
                                        match status {
                                            Ok(s) if s.success() => {
                                                println!("✓ Successfully updated {}", repo);
                                            }
                                            Ok(s) => {
                                                println!("✗ eget exited with status: {}", s);
                                            }
                                            Err(e) => {
                                                println!("✗ Failed to run eget: {}", e);
                                            }
                                        }
                                        
                                        println!("\nPress Enter to continue...");
                                        let mut input = String::new();
                                        let _ = io::stdin().read_line(&mut input);
                                        
                                        // Re-enter TUI
                                        enable_raw_mode()?;
                                        execute!(io::stdout(), EnterAlternateScreen)?;
                                        
                                        // Reload entries
                                        let entries = load_log();
                                        app = App::new(entries);
                                    }
                                }
                                _ => {}
                            }
                        }
                        InputMode::Filter => {
                            match key.code {
                                KeyCode::Enter => {
                                    app.input_mode = InputMode::Normal;
                                    app.apply_filter();
                                }
                                KeyCode::Esc => {
                                    app.input_mode = InputMode::Normal;
                                    app.filter_input.clear();
                                    app.apply_filter();
                                }
                                KeyCode::Char(c) => {
                                    app.filter_input.push(c);
                                    app.apply_filter();
                                }
                                KeyCode::Backspace => {
                                    app.filter_input.pop();
                                    app.apply_filter();
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.size();

    // Create main layout
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(size);

    // Split main area: left list, right details
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(65),
            Constraint::Percentage(35),
        ])
        .split(main_chunks[0]);

    // left list items
    let visible_entries = app.visible_entries();
    let items: Vec<ListItem> = visible_entries.iter().enumerate().map(|(i, e)| {
        let name = std::path::Path::new(&e.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?");

        let status = if e.removed {
            " [REMOVED]"
        } else if !std::path::Path::new(&e.path).exists() {
            " [MISSING]"
        } else {
            ""
        };

        let line = format!(
            "{:3} │ {:20} │ {:30} │ {:>8}{}",
            i,
            name.chars().take(20).collect::<String>(),
            e.repo.chars().take(30).collect::<String>(),
            e.size_human(),
            status,
        );

        let mut style = Style::default();
        if e.removed || !std::path::Path::new(&e.path).exists() {
            style = style.fg(Color::DarkGray);
        }

        ListItem::new(line).style(style)
    }).collect();

    let list_title = if app.input_mode == InputMode::Filter {
        format!("Installed via eget ({} filtered)", visible_entries.len())
    } else {
        format!("Installed via eget ({} total)", visible_entries.len())
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(list_title))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ratatui::widgets::ListState::default();
    if !visible_entries.is_empty() {
        list_state.select(Some(app.selected));
    }

    f.render_stateful_widget(list, chunks[0], &mut list_state);

    // right detail panel
    let detail_text = if let Some(curr) = app.current() {
        let exists = std::path::Path::new(&curr.path).exists();
        let status = if curr.removed {
            "Status: REMOVED"
        } else if !exists {
            "Status: MISSING"
        } else {
            "Status: Installed"
        };

        format!(
            "Binary: {}\n\nRepo: {}\n\nSize: {}\n\nInstalled: {}\n\n{}\n\n\
            Keys:\n\
            [j/k] navigate\n\
            [/] filter\n\
            [d] delete binary\n\
            [u/r] update/reinstall\n\
            [q] quit",
            curr.path,
            curr.repo,
            curr.size_human(),
            curr.timestamp.format("%Y-%m-%d %H:%M:%S"),
            status,
        )
    } else {
        "No entries".into()
    };

    let detail = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title("Details"))
        .wrap(Wrap { trim: true });

    f.render_widget(detail, chunks[1]);

    // Bottom status/filter bar
    let status_text = match app.input_mode {
        InputMode::Normal => {
            "Press [/] to filter, [q] to quit".to_string()
        }
        InputMode::Filter => {
            format!("Filter: {} (Enter to apply, Esc to cancel)", app.filter_input)
        }
    };

    let status_style = if app.input_mode == InputMode::Filter {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let status = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL))
        .style(status_style);

    f.render_widget(status, main_chunks[1]);
}
