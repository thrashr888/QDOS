//! Command Palette plugin
//!
//! Spotlight-style command palette for quick access to apps, commands, files, and calculator.

#![allow(clippy::ptr_arg)]

mod calc;
mod fuzzy;
mod modal;
pub mod state;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qdos_plugin_api::prelude::*;
use ratatui::{layout::Rect, Frame};
use state::{PaletteAction, PaletteResult, PaletteState};
use std::any::Any;
use std::path::PathBuf;

/// App info for palette results (simplified from AppEntry)
#[derive(Debug, Clone)]
pub struct PaletteApp {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// Command Palette plugin
pub struct PalettePlugin {
    pub state: PaletteState,
    /// Available apps (populated from plugin manager)
    pub apps: Vec<PaletteApp>,
    /// Current working directory for file search
    cwd: PathBuf,
    /// Result of executing a command (plugin ID to launch, or NavItem to execute)
    pub pending_action: Option<PaletteAction>,
}

impl Default for PalettePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl PalettePlugin {
    pub fn new() -> Self {
        Self {
            state: PaletteState::new(),
            apps: Vec::new(),
            cwd: PathBuf::new(),
            pending_action: None,
        }
    }

    /// Set available apps (called from App when opening palette)
    pub fn set_apps(&mut self, apps: Vec<PaletteApp>) {
        self.apps = apps;
    }

    /// Update results based on current input
    pub fn update_results(&mut self) {
        let query = &self.state.input;
        let mut results = Vec::new();

        // Check for calculator expression first
        if calc::looks_like_expression(query) {
            if let Some(value) = calc::evaluate(query) {
                results.push(PaletteResult::calculator(value));
            }
        }

        // Match commands (NavItem)
        for item in NavItem::ALL.iter() {
            if let Some(score) =
                fuzzy::fuzzy_score_multi(query, &[item.as_str(), item.description()])
            {
                if score > 0 || query.is_empty() {
                    results.push(PaletteResult::command(*item, score));
                }
            }
        }

        // Match apps
        for app in &self.apps {
            if let Some(score) =
                fuzzy::fuzzy_score_multi(query, &[&app.name, &app.description, &app.id])
            {
                if score > 0 || query.is_empty() {
                    results.push(PaletteResult::app(
                        app.id.clone(),
                        app.name.clone(),
                        app.description.clone(),
                        score,
                    ));
                }
            }
        }

        // Match files in current directory (simple name matching)
        if !query.is_empty() && query.len() >= 2 {
            if let Ok(entries) = std::fs::read_dir(&self.cwd) {
                for entry in entries.filter_map(|e| e.ok()).take(20) {
                    let path = entry.path();
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    if let Some(score) = fuzzy::fuzzy_score(query, &name) {
                        if score > 0 {
                            results.push(PaletteResult::file(path, score));
                        }
                    }
                }
            }
        }

        self.state.set_results(results);
    }

    /// Execute the selected result
    fn execute_selected(&mut self) -> KeyHandleResult {
        if let Some(result) = self.state.selected_result() {
            let action = result.action.clone();
            match action {
                PaletteAction::CopyToClipboard(ref text) => {
                    let msg = format!("Copied: {}", text);
                    self.pending_action = Some(action);
                    KeyHandleResult::CloseWithSuccess(msg)
                }
                PaletteAction::OpenPlugin(ref id) => {
                    let msg = format!("launch:{}", id);
                    self.pending_action = Some(action);
                    KeyHandleResult::CloseWithSuccess(msg)
                }
                PaletteAction::ExecuteCommand(ref item) => {
                    let msg = format!("command:{}", item.as_str());
                    self.pending_action = Some(action);
                    KeyHandleResult::CloseWithSuccess(msg)
                }
                PaletteAction::NavigateFile(ref path) => {
                    let result = if path.is_dir() {
                        KeyHandleResult::NavigateToDir(path.clone())
                    } else {
                        KeyHandleResult::NavigateToFile(path.clone())
                    };
                    self.pending_action = Some(action);
                    result
                }
            }
        } else {
            KeyHandleResult::Handled
        }
    }
}

impl Plugin for PalettePlugin {
    fn id(&self) -> &str {
        "palette"
    }

    fn name(&self) -> &str {
        "Command Palette"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: false, // Not in plugin menu - accessed via TAB
            has_keys: true,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true // Always available
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        None // Accessed via TAB, not plugin menu
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        match key.code {
            KeyCode::Tab => {
                // Open Command Palette
                self.state.reset();
                self.cwd = cwd.clone();
                self.pending_action = None;
                self.update_results();
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.reset();
                KeyHandleResult::CloseModal
            }
            KeyCode::Enter => self.execute_selected(),
            KeyCode::Up => {
                self.state.select_prev();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.state.select_next();
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.state.cursor_home();
                } else {
                    self.state.cursor_left();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.state.cursor_end();
                } else {
                    self.state.cursor_right();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                self.state.cursor_home();
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                self.state.cursor_end();
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.backspace();
                self.update_results();
                KeyHandleResult::Handled
            }
            KeyCode::Delete => {
                self.state.delete();
                self.update_results();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.insert_char(c);
                self.update_results();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_palette_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "TAB - Command Palette".to_string(),
            "".to_string(),
            "Quick access to commands, apps, files, and calculator.".to_string(),
            "".to_string(),
            "Keys:".to_string(),
            "  TAB       Open palette".to_string(),
            "  Type      Filter results".to_string(),
            "  ↑↓        Navigate results".to_string(),
            "  Enter     Execute selected".to_string(),
            "  Esc       Close".to_string(),
            "".to_string(),
            "Calculator:".to_string(),
            "  Type math expressions like 2+2, (3*4)/2, 2^8".to_string(),
            "  Press Enter to copy result".to_string(),
        ]
    }

    fn launch(&mut self, cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.state.reset();
        self.cwd = cwd.clone();
        self.pending_action = None;
        self.update_results();
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Self-registration for automatic plugin discovery
inventory::submit! {
    PluginRegistration::new("palette", || Box::new(PalettePlugin::new()))
}
