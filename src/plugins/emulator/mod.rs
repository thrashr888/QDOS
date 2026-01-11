//! Emulator plugin
//!
//! Run DOS executables in DOSBox-X emulator.
//! Supports .EXE, .COM, and .BAT files.

mod modal;
pub mod state;

use crate::app::ThemeColors;
use crate::plugins::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{EmulatorState, EmulatorView};
use std::any::Any;
use std::path::PathBuf;
use std::process::Command;

/// Emulator plugin - run DOS programs in DOSBox-X
pub struct EmulatorPlugin {
    pub state: EmulatorState,
}

impl Default for EmulatorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl EmulatorPlugin {
    pub fn new() -> Self {
        let mut state = EmulatorState::new();
        state.detect_emulators();
        Self { state }
    }

    pub fn open_modal(&mut self, selected_file: Option<&PathBuf>) {
        self.state = EmulatorState::new();
        self.state.detect_emulators();
        self.state.file_path = selected_file.cloned();

        if !self.state.dosbox_available {
            self.state.view = EmulatorView::NotAvailable;
        } else {
            self.state.view = EmulatorView::Menu;
        }
    }

    /// Run the selected file in DOSBox-X
    fn run_in_dosbox(&mut self) {
        let Some(ref path) = self.state.file_path else {
            self.state.error = Some("No file selected".to_string());
            return;
        };

        if !self.state.dosbox_available {
            self.state.error = Some("DOSBox-X is not installed".to_string());
            return;
        }

        self.state.view = EmulatorView::Running;
        self.state.error = None;

        // Get the directory containing the executable
        let working_dir = path.parent().unwrap_or(path);

        // Build DOSBox-X command
        // We use -exit to close DOSBox after the program finishes
        let mut cmd = Command::new("dosbox-x");
        cmd.arg(path).arg("-exit").current_dir(working_dir);

        // Check for a config file in the working directory or parent
        let config_paths = [
            working_dir.join("dosbox-x.conf"),
            working_dir.join("dosbox.conf"),
            working_dir
                .parent()
                .map(|p| p.join("dosbox-x.conf"))
                .unwrap_or_default(),
        ];

        for config_path in &config_paths {
            if config_path.exists() {
                cmd.arg("-conf").arg(config_path);
                break;
            }
        }

        let result = cmd.spawn();

        match result {
            Ok(mut child) => {
                // Wait for DOSBox-X to finish
                match child.wait() {
                    Ok(_status) => {
                        self.state.view = EmulatorView::Menu;
                    }
                    Err(e) => {
                        self.state.error = Some(format!("Error waiting for DOSBox-X: {}", e));
                        self.state.view = EmulatorView::Menu;
                    }
                }
            }
            Err(e) => {
                self.state.error = Some(format!("Failed to start DOSBox-X: {}", e));
                self.state.view = EmulatorView::Menu;
            }
        }
    }

    /// Check if the currently selected file can be run
    fn can_run_selected(&self) -> bool {
        if let Some(ref path) = self.state.file_path {
            self.state.can_run(path).is_some()
        } else {
            false
        }
    }
}

impl Plugin for EmulatorPlugin {
    fn id(&self) -> &str {
        "emulator"
    }

    fn name(&self) -> &str {
        "Emulator"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        // Always show in menu, even if no emulators installed
        true
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Emulator".to_string(),
            key: 'X',
            description: "Run DOS executables in emulator".to_string(),
            priority: 85,
        })
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        _cwd: &PathBuf,
        selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        match key.code {
            KeyCode::Char('x') => {
                self.open_modal(selected_file);
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            EmulatorView::Menu => match key.code {
                KeyCode::Esc => KeyHandleResult::CloseModal,
                KeyCode::Enter => {
                    if self.can_run_selected() {
                        self.run_in_dosbox();
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            EmulatorView::FileSelect => match key.code {
                KeyCode::Esc => {
                    self.state.view = EmulatorView::Menu;
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.state.select_prev();
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.state.select_next();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    if let Some(entry) = self.state.entries.get(self.state.selected).cloned() {
                        self.state.file_path = Some(entry.path);
                        self.run_in_dosbox();
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            EmulatorView::Running => {
                // Can't interact while running
                KeyHandleResult::Handled
            }
            EmulatorView::NotAvailable => match key.code {
                KeyCode::Esc => KeyHandleResult::CloseModal,
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_emulator_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Emulator Plugin".to_string(),
            "".to_string(),
            "Run DOS executables in the DOSBox-X emulator.".to_string(),
            "".to_string(),
            "Supported File Types:".to_string(),
            "  .EXE  DOS executable files".to_string(),
            "  .COM  DOS command files".to_string(),
            "  .BAT  DOS batch files".to_string(),
            "".to_string(),
            "Installation:".to_string(),
            "  brew install dosbox-x".to_string(),
            "".to_string(),
            "Usage:".to_string(),
            "  1. Select a DOS executable in the file list".to_string(),
            "  2. Press 'x' or open from F12 Apps".to_string(),
            "  3. Press Enter to run in DOSBox-X".to_string(),
            "".to_string(),
            "Keys in modal:".to_string(),
            "  Enter   Run selected file".to_string(),
            "  Esc     Close/back".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Emulator".to_string(),
            description: "Run DOS programs in DOSBox-X".to_string(),
            category: PluginCategory::Games,
            key: 'X',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.open_modal(selected_file);
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
