mod core;
mod ui;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::{arg, command, Parser};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::CrosstermBackend,
    Terminal,
};

use crate::{
    core::build_tree,
    ui::{
        app::App,
        event::{self, Action, Events},
        ui::draw_ui,
    },
};

#[derive(Parser, Debug)]
#[command(version, about = "Disk Usage TUI Analyzer")]
struct Cli {
    /// Root directory to scan
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Follow symbolic links
    #[arg(long)]
    follow_symlinks: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = cli.root.canonicalize()?;

    // Setup progress bar
    let pb = indicatif::ProgressBar::new_spinner();
    pb.set_style(
        indicatif::ProgressStyle::with_template("{spinner} Scanning {msg}")?
            .tick_strings(&["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈"]),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(root.display().to_string());

    // Build directory tree
    let tree = build_tree(&root, cli.follow_symlinks, &pb)?;
    pb.finish_and_clear();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and event handler
    let mut app = App::new(tree);
    let events = Events::new(Duration::from_millis(100));

    // Main event loop
    terminal.clear()?;
    loop {
        // Draw UI
        terminal.draw(|f| draw_ui(f, &app))?;

        // Handle events
        match events.next()? {
            event::Event::Input(key) => {
                if let Some(action) = event::handle_key_event(key.code) {
                    match action {
                        Action::Quit => break,
                        Action::ToggleSort => app.toggle_sort(),
                        Action::MoveSelection(delta) => app.move_selection(delta),
                        Action::NavigateIn => {
                            app.navigate_into();
                        }
                        Action::NavigateOut => {
                            app.navigate_out();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
