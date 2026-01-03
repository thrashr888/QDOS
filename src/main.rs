// Allow specific clippy lints that would require significant refactoring
#![allow(clippy::ptr_arg)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::manual_strip)]
#![allow(clippy::wrong_self_convention)]
#![allow(clippy::manual_flatten)]
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::lines_filter_map_ok)]

mod app;
mod config;
mod errors;
mod event;
mod file_ops;
mod plugins;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

fn print_help() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        r#"rdos - Q-DOS II File Manager v{}

A modern TUI file manager inspired by Q-DOS, with Git and Beads integration.

USAGE:
    rdos [OPTIONS] [PATH]

ARGUMENTS:
    [PATH]    Starting directory (defaults to current directory)

OPTIONS:
    -h, --help       Show this help message
    -v, --version    Show version information

KEYBOARD SHORTCUTS:
    F1-F8           Quick navigation menu items
    G               Open Git menu
    B               Open Beads issue tracker
    V               View selected file
    Enter           Enter directory / execute action
    Space           Tag/untag file
    Tab             Toggle directory panels
    Esc             Cancel / close modal
    Q / Ctrl+C      Quit

For more information, see the in-app help (F10 or ?).

Repository: https://github.com/thrashr888/QDOS"#,
        version
    );
}

fn print_version() {
    let version = env!("CARGO_PKG_VERSION");
    println!("rdos v{}", version);
}

#[tokio::main]
async fn main() -> Result<()> {
    // Check for help/version flags
    let args: Vec<String> = std::env::args().collect();
    for arg in &args[1..] {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            "-v" | "--version" => {
                print_version();
                return Ok(());
            }
            _ => {}
        }
    }

    // Get starting directory from args or use current directory
    let start_path = args.get(1).cloned().unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new(&start_path)?;
    let res = app.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }

    Ok(())
}
