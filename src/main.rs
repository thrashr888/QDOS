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
mod clipboard;
mod config;
mod errors;
mod event;
mod file_ops;
mod plugins;
mod rg;
mod ui;
mod watcher;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

/// CLI subcommand for direct modal access
#[derive(Debug, Clone)]
enum Subcommand {
    /// Open file manager (default)
    Browse(Option<String>),
    /// Open file in Q-EDIT text editor
    Edit(String),
    /// Open file in viewer
    View(String),
    /// Open Git modal
    Git,
    /// Open Beads modal
    Beads,
}

fn print_help() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        r#"rdos - Q-DOS II File Manager v{}

A modern TUI file manager inspired by Q-DOS, with Git and Beads integration.

USAGE:
    rdos [OPTIONS] [PATH]
    rdos edit <FILE>     Open file in Q-EDIT text editor
    rdos view <FILE>     View file contents
    rdos git             Open Git modal
    rdos beads           Open Beads issue tracker

ARGUMENTS:
    [PATH]    Starting directory (defaults to current directory)

SUBCOMMANDS:
    edit <FILE>    Open Q-EDIT text editor with file
    view <FILE>    Open viewer with file
    git            Open Git integration modal
    beads          Open Beads issue tracker modal

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

fn parse_subcommand(args: &[String]) -> Subcommand {
    if args.len() > 1 {
        match args[1].as_str() {
            "edit" => {
                let file = args.get(2).cloned().unwrap_or_else(|| ".".to_string());
                return Subcommand::Edit(file);
            }
            "view" => {
                let file = args.get(2).cloned().unwrap_or_else(|| ".".to_string());
                return Subcommand::View(file);
            }
            "git" => return Subcommand::Git,
            "beads" => return Subcommand::Beads,
            _ => {
                // Might be a path, not a subcommand
                return Subcommand::Browse(Some(args[1].clone()));
            }
        }
    }
    Subcommand::Browse(None)
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

    // Parse subcommand
    let subcommand = parse_subcommand(&args);

    // Get starting directory based on subcommand
    let start_path = match &subcommand {
        Subcommand::Browse(Some(path)) => path.clone(),
        Subcommand::Edit(file) | Subcommand::View(file) => {
            // Use file's parent directory, or current directory
            let path = std::path::PathBuf::from(file);
            if path.is_absolute() {
                path.parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| ".".to_string())
            } else {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            }
        }
        _ => std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string()),
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();

    // Enable Kitty keyboard protocol for enhanced input handling
    // This provides better modifier key detection and removes escape code ambiguity
    // Supported by: Kitty, Ghostty, WezTerm, foot, and other modern terminals
    let keyboard_enhancement = PushKeyboardEnhancementFlags(
        KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS,
    );

    // Note: Mouse capture disabled to allow native terminal text selection
    // Can be re-enabled later as a config option that dynamically enables/disables
    execute!(stdout, EnterAlternateScreen, keyboard_enhancement)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new(&start_path)?;

    // Handle subcommand: open appropriate modal
    match subcommand {
        Subcommand::Edit(file) => {
            let path = std::path::PathBuf::from(&file).canonicalize().ok();
            if let Some(plugin) = app.plugin_manager.qedit_plugin_mut() {
                let _ = plugin.open(path);
            }
            app.plugin_manager.set_active_modal(Some("qedit"));
        }
        Subcommand::View(file) => {
            let path = std::path::PathBuf::from(&file)
                .canonicalize()
                .unwrap_or_else(|_| std::path::PathBuf::from(&file));
            let cwd = app.current_path.clone();
            if let Some(plugin) = app.plugin_manager.viewer_plugin_mut() {
                let _ = plugin.open_file(path, &cwd);
            }
            app.plugin_manager.set_active_modal(Some("viewer"));
        }
        Subcommand::Git => {
            app.plugin_manager.set_active_modal(Some("git"));
        }
        Subcommand::Beads => {
            app.plugin_manager.set_active_modal(Some("beads"));
        }
        Subcommand::Browse(_) => {
            // Default behavior, no modal
        }
    }

    let res = app.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        PopKeyboardEnhancementFlags,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }

    Ok(())
}
