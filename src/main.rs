// disk_usage_tui: interactive disk usage analyzer for macOS 🛠️
// Add the following dependencies to your Cargo.toml:
// anyhow = "1.0"
// clap = { version = "4.5", features = ["derive"] }
// crossterm = "0.26"
// humansize = "2.1"
// ignore = "0.4"
// indicatif = "0.17"
// num_cpus = "1.16"
// rayon = "1.8"
// tui = "0.19"
// walkdir = "2.4"

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use clap::{arg, command, Parser};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use humansize::{format_size, DECIMAL};
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};

#[derive(Parser, Debug)]
#[command(version, about = "Disk Usage TUI Analyzer for macOS")]
struct Cli {
    /// Root directory to scan
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Follow symbolic links
    #[arg(long)]
    follow_symlinks: bool,
}

#[derive(Debug, Clone)]
struct DirEntryInfo {
    path: PathBuf,
    size: u64,
    is_dir: bool,
    children: Vec<DirEntryInfo>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = cli.root.canonicalize()?;

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner} Scanning {msg}")?
            .tick_strings(&["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈"]),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(root.display().to_string());

    let tree = build_tree(&root, cli.follow_symlinks, &pb)?;
    pb.finish_and_clear();

    run_tui(tree)?;

    Ok(())
}

fn build_tree(root: &Path, follow_symlinks: bool, _pb: &ProgressBar) -> Result<DirEntryInfo> {
    let mut entries: Vec<(PathBuf, u64, bool)> = WalkBuilder::new(root)
        .follow_links(follow_symlinks)
        .hidden(false)
        .threads(num_cpus::get())
        .build()
        .par_bridge()
        .filter_map(|entry| match entry {
            Ok(dirent) => {
                if dirent.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    let md = dirent.metadata().ok()?;
                    let sz = md.len();
                    Some((dirent.into_path(), sz, false))
                } else {
                    Some((dirent.into_path(), 0, true))
                }
            }
            Err(_) => None,
        })
        .collect();

    entries.sort_by_key(|(p, _, _)| p.clone());

    use std::collections::HashMap;
    let mut sizes: HashMap<PathBuf, u64> = HashMap::new();
    for (path, size, _) in &entries {
        sizes.entry(path.clone()).or_default();
        if *size > 0 {
            sizes.entry(path.clone()).and_modify(|s| *s += *size);
        }
        let mut cur = path.parent();
        while let Some(p) = cur {
            sizes.entry(p.to_path_buf()).or_default();
            sizes.entry(p.to_path_buf()).and_modify(|s| *s += *size);
            cur = p.parent();
        }
    }

    fn build_node(
        path: &Path,
        sizes: &HashMap<PathBuf, u64>,
        is_dir: bool,
        entries: &[(PathBuf, u64, bool)],
    ) -> DirEntryInfo {
        let children_paths: Vec<&(PathBuf, u64, bool)> = entries
            .iter()
            .filter(|(p, _, _)| p.parent() == Some(path))
            .collect();
        let children = children_paths
            .iter()
            .map(|(p, _, isd)| build_node(p, sizes, *isd, entries))
            .collect();
        DirEntryInfo {
            path: path.to_path_buf(),
            size: *sizes.get(path).unwrap_or(&0),
            is_dir,
            children,
        }
    }

    let root_node = build_node(root, &sizes, true, &entries);
    Ok(root_node)
}

#[derive(PartialEq)]
enum SortBy {
    Name,
    Size,
}

impl Default for SortBy {
    fn default() -> Self {
        SortBy::Size
    }
}

fn draw_ui<B: tui::backend::Backend>(
    f: &mut Frame<B>,
    state: &mut TableState,
    stack: &[(DirEntryInfo, usize)],
    sort_by: &SortBy,
) {
    let (node, _) = stack.last().unwrap();

    // Sort children based on current sort criteria
    let mut children = node.children.clone();
    match sort_by {
        SortBy::Name => children.sort_by(|a, b| a.path.file_name().cmp(&b.path.file_name())),
        SortBy::Size => children.sort_by(|a, b| b.size.cmp(&a.size)),
    }

    // Calculate file and directory counts
    let (file_count, dir_count) = children.iter().fold((0, 0), |(files, dirs), child| {
        if child.is_dir {
            (files, dirs + 1)
        } else {
            (files + 1, dirs)
        }
    });

    // Create layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.size());

    // Draw header
    let header = Block::default()
        .borders(Borders::ALL)
        .title(" Disk Usage Analyzer (q to quit)");
    let current_path = node.path.display().to_string();
    let path_text = Paragraph::new(current_path).block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, chunks[0]);
    f.render_widget(path_text, chunks[0]);

    // Draw table
    let header_cells = ["Name", "Size"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::REVERSED))
        .bottom_margin(1);

    let rows = children.iter().enumerate().map(|(i, child)| {
        let is_selected = state.selected() == Some(i);
        let style = if is_selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        let name = child
            .path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());

        let name_style = if child.is_dir {
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        Row::new(vec![name, format_size(child.size, DECIMAL)])
            .style(style)
            .style(name_style)
    });

    let table = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .widths(&[Constraint::Percentage(70), Constraint::Percentage(30)]);

    f.render_stateful_widget(table, chunks[1], state);

    // Draw status bar
    let status = format!(
        "↑/↓: Navigate | Enter: Open | ←: Go Back | s: Toggle Sort | Files: {} | Dirs: {} | Total: {}",
        file_count,
        dir_count,
        format_size(node.size, DECIMAL)
    );
    let status_bar = Paragraph::new(status).block(Block::default().borders(Borders::ALL));
    f.render_widget(status_bar, chunks[2]);
}

fn run_tui(root: DirEntryInfo) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = TableState::default();
    let mut stack: Vec<(DirEntryInfo, usize)> = vec![(root, 0)];
    let mut sort_by = SortBy::default();

    // Initial draw
    terminal.clear()?;

    loop {
        terminal.draw(|f| {
            draw_ui(f, &mut state, &stack, &sort_by);
        })?;

        if crossterm::event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('s') => {
                        sort_by = match sort_by {
                            SortBy::Name => SortBy::Size,
                            SortBy::Size => SortBy::Name,
                        };
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let len = stack.last().unwrap().0.children.len();
                        if len > 0 {
                            state.select(Some(match state.selected() {
                                Some(i) if i + 1 < len => i + 1,
                                _ => 0,
                            }));
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let len = stack.last().unwrap().0.children.len();
                        if len > 0 {
                            state.select(Some(match state.selected() {
                                Some(i) if i > 0 => i - 1,
                                _ => len - 1,
                            }));
                        }
                    }
                    KeyCode::Right | KeyCode::Enter => {
                        if let Some(sel) = state.selected() {
                            let node = &stack.last().unwrap().0.children[sel];
                            if node.is_dir && !node.children.is_empty() {
                                stack.push((node.clone(), 0));
                                state = TableState::default();
                            }
                        }
                    }
                    KeyCode::Left | KeyCode::Backspace => {
                        if stack.len() > 1 {
                            stack.pop();
                            state = TableState::default();
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    // Clean up
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
