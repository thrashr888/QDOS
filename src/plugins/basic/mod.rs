//! BASIC Runner plugin
//!
//! Run BASIC programs (.bas files) using available interpreters.
//! Supports bas55, pc-basic, bwbasic, and cbmbasic.

mod modal;
pub mod state;

use crate::app::ThemeColors;
use crate::plugins::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{BasicState, BasicView};
use std::any::Any;
use std::path::PathBuf;
use std::process::Command;

/// BASIC Runner plugin
pub struct BasicPlugin {
    pub state: BasicState,
}

impl Default for BasicPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl BasicPlugin {
    pub fn new() -> Self {
        Self {
            state: BasicState::new(),
        }
    }

    /// Open the modal for a specific .bas file
    pub fn open_modal(&mut self, file_path: Option<&PathBuf>) {
        self.state.detect_interpreters();
        self.state.file_path = file_path.cloned();
        self.state.view = BasicView::Menu;
        self.state.output.clear();
        self.state.error = None;
        self.state.scroll_offset = 0;

        // Auto-run if only one interpreter and a file is selected
        if self.state.available_interpreters.len() == 1 && self.state.file_path.is_some() {
            self.run_program();
        }
    }

    /// Run the BASIC program with the selected interpreter
    fn run_program(&mut self) {
        let Some(interpreter) = self.state.selected().copied() else {
            self.state.error = Some("No interpreter selected".to_string());
            self.state.view = BasicView::Error;
            return;
        };

        let Some(file_path) = &self.state.file_path else {
            self.state.error = Some("No file selected".to_string());
            self.state.view = BasicView::Error;
            return;
        };

        self.state.view = BasicView::Running;
        self.state.is_running = true;

        // Build command based on interpreter
        let output = match interpreter {
            state::BasicInterpreter::Bas55 => Command::new("bas55").arg(file_path).output(),
            state::BasicInterpreter::PcBasic => Command::new("pc-basic")
                .arg("--run")
                .arg(file_path)
                .arg("--quit")
                .output(),
            state::BasicInterpreter::BwBasic => Command::new("bwbasic").arg(file_path).output(),
            state::BasicInterpreter::CbmBasic => Command::new("cbmbasic").arg(file_path).output(),
        };

        self.state.is_running = false;

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);

                self.state.output.clear();

                if !stdout.is_empty() {
                    for line in stdout.lines() {
                        self.state.output.push(strip_ansi_codes(line));
                    }
                }

                if !stderr.is_empty() {
                    if !self.state.output.is_empty() {
                        self.state.output.push(String::new());
                        self.state.output.push("--- Errors ---".to_string());
                    }
                    for line in stderr.lines() {
                        self.state.output.push(strip_ansi_codes(line));
                    }
                }

                if self.state.output.is_empty() {
                    self.state.output.push("(No output)".to_string());
                }

                self.state.scroll_offset = 0;
                self.state.view = BasicView::Output;
            }
            Err(e) => {
                self.state.error = Some(format!("Failed to run: {}", e));
                self.state.view = BasicView::Error;
            }
        }
    }

    /// Check if a file is a BASIC program
    pub fn is_basic_file(path: &PathBuf) -> bool {
        path.extension()
            .map(|ext| {
                let ext = ext.to_string_lossy().to_lowercase();
                ext == "bas" || ext == "basic"
            })
            .unwrap_or(false)
    }
}

impl Plugin for BasicPlugin {
    fn id(&self) -> &str {
        "basic"
    }

    fn name(&self) -> &str {
        "BASIC Runner"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: false, // No global key - accessed via F12 Apps or file action
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        // Available if any interpreter is installed
        !self.state.available_interpreters.is_empty()
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "BASIC".to_string(),
            key: 'B',
            description: "Run BASIC programs".to_string(),
            priority: 75,
        })
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn handle_global_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // No global key binding
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            BasicView::Menu => match key.code {
                KeyCode::Esc => KeyHandleResult::CloseModal,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.state.select_prev();
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.state.select_next();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char('r') | KeyCode::Char('R') => {
                    if self.state.file_path.is_some()
                        && !self.state.available_interpreters.is_empty()
                    {
                        self.run_program();
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            BasicView::Running => {
                // Can't interrupt while running
                KeyHandleResult::Handled
            }
            BasicView::Output => match key.code {
                KeyCode::Esc => {
                    self.state.view = BasicView::Menu;
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.state.scroll_up();
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.state.scroll_down(20); // Approximate visible lines
                    KeyHandleResult::Handled
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.run_program();
                    KeyHandleResult::Handled
                }
                KeyCode::PageUp => {
                    for _ in 0..10 {
                        self.state.scroll_up();
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::PageDown => {
                    for _ in 0..10 {
                        self.state.scroll_down(20);
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            BasicView::Error => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.state.view = BasicView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_basic_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "BASIC Runner Plugin".to_string(),
            "".to_string(),
            "Run BASIC programs (.bas files) using available interpreters.".to_string(),
            "".to_string(),
            "Supported Interpreters:".to_string(),
            "  bas55    - Minimal ANSI BASIC (Ecma-55)".to_string(),
            "  pc-basic - GW-BASIC/BASICA compatible".to_string(),
            "  bwbasic  - Bywater BASIC interpreter".to_string(),
            "  cbmbasic - Commodore 64 BASIC".to_string(),
            "".to_string(),
            "Installation:".to_string(),
            "  brew install bas55".to_string(),
            "  brew install cbmbasic".to_string(),
            "".to_string(),
            "Usage:".to_string(),
            "  1. Select a .bas file in the file list".to_string(),
            "  2. Open BASIC Runner from F12 Apps".to_string(),
            "  3. Select an interpreter and press Enter to run".to_string(),
            "".to_string(),
            "Keys in modal:".to_string(),
            "  ↑↓/jk   Select interpreter".to_string(),
            "  Enter/r Run program".to_string(),
            "  Esc     Close/back".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "BASIC".to_string(),
            description: "Run BASIC programs".to_string(),
            category: PluginCategory::Games,
            key: 'I',
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Strip ANSI escape codes from a string
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ESC and following sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                              // Skip until we hit a letter (end of sequence)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else if c.is_control() && c != '\n' && c != '\t' {
            // Skip other control characters
        } else {
            result.push(c);
        }
    }

    result
}
