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

use anyhow::{Context, Result};
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
    widgets::{Block, Borders, Row, Table, TableState},
    Terminal,
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

fn build_tree(root: &Path, follow_symlinks: bool, pb: &ProgressBar) -> Result<DirEntryInfo> {
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

fn run_tui(root: DirEntryInfo) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = TableState::default();
    let mut stack: Vec<(DirEntryInfo, usize)> = vec![(root, 0)];

    loop {
        terminal.draw(|f| {
            let (node, _) = stack.last().unwrap();
            let rows: Vec<Row> = node
                .children
                .iter()
                .map(|child| {
                    Row::new(vec![
                        child
                            .path
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "/".to_string()),
                        format_size(child.size, DECIMAL),
                    ])
                })
                .collect();
            let table = Table::new(rows)
                .header(Row::new(vec!["Name", "Size"]).bottom_margin(1))
                .block(
                    Block::default()
                        .title(node.path.display().to_string())
                        .borders(Borders::ALL),
                )
                .widths(&[
                    tui::layout::Constraint::Percentage(70),
                    tui::layout::Constraint::Percentage(30),
                ]);
            f.render_stateful_widget(table, f.size(), &mut state);
        })?;

        if crossterm::event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => state.select(Some(match state.selected() {
                        Some(i) if i + 1 < stack.last().unwrap().0.children.len() => i + 1,
                        _ => 0,
                    })),
                    KeyCode::Up => state.select(Some(match state.selected() {
                        Some(i) if i > 0 => i - 1,
                        _ => stack.last().unwrap().0.children.len() - 1,
                    })),
                    KeyCode::Enter => {
                        if let Some(sel) = state.selected() {
                            let node = &stack.last().unwrap().0.children[sel];
                            if node.is_dir && !node.children.is_empty() {
                                stack.push((node.clone(), 0));
                                state = TableState::default();
                            }
                        }
                    }
                    KeyCode::Backspace => {
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

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
