//! Q-MIND: AI Intelligence Layer plugin
//!
//! Provides semantic search and natural language commands for QDOS.
//! Press `?` to open the command palette from anywhere.

mod modal;
mod state;

use crate::plugins::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{QMindState, QMindView};
use std::any::Any;
use std::path::PathBuf;


/// Q-MIND AI Intelligence Layer plugin
pub struct QMindPlugin {
    pub state: QMindState,
    /// Whether data is currently being loaded
    loading: bool,
}

impl Default for QMindPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl QMindPlugin {
    pub fn new() -> Self {
        Self {
            state: QMindState::new(),
            loading: false,
        }
    }

    /// Start loading
    pub fn start_loading(&mut self) {
        self.loading = true;
    }

    /// Check if currently loading
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Initialize Q-MIND (check API availability, etc.)
    fn initialize(&mut self) {
        self.state.check_api_availability();
        self.loading = false;
    }
}

impl Plugin for QMindPlugin {
    fn id(&self) -> &str {
        "qmind"
    }

    fn name(&self) -> &str {
        "Q-MIND"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: false,       // Not in old menu system
            has_keys: true,        // Has ? global key
            has_modal: true,
            has_status: true,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        // Q-MIND is always available (shows setup info if no API key)
        true
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        // No menu item - accessed via ? key or App Launcher
        None
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        if self.state.api_available {
            Some(PluginStatusInfo {
                text: format!("Q-MIND {} files", self.state.indexed_count),
                active: true,
            })
        } else {
            None
        }
    }

    fn handle_global_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // ? key opens command palette directly
        if let KeyCode::Char('?') = key.code {
            self.state.view = QMindView::CommandPalette;
            self.state.command_input.reset();
            return KeyHandleResult::OpenModal;
        }
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            QMindView::Overview => match key.code {
                KeyCode::Esc => KeyHandleResult::CloseModal,
                KeyCode::Char('?') | KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.state.view = QMindView::CommandPalette;
                    self.state.command_input.reset();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.state.view = QMindView::SemanticSearch;
                    self.state.search_input.reset();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.state.view = QMindView::IndexStatus;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            QMindView::CommandPalette => {
                match key.code {
                    KeyCode::Esc => {
                        self.state.view = QMindView::Overview;
                        KeyHandleResult::Handled
                    }
                    KeyCode::Enter => {
                        // TODO: Parse and execute command
                        KeyHandleResult::Handled
                    }
                    KeyCode::Backspace => {
                        self.state.command_input.backspace();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Delete => {
                        self.state.command_input.delete();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Left => {
                        self.state.command_input.cursor_left();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Right => {
                        self.state.command_input.cursor_right();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Home => {
                        self.state.command_input.cursor_home();
                        KeyHandleResult::Handled
                    }
                    KeyCode::End => {
                        self.state.command_input.cursor_end();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char(c) => {
                        self.state.command_input.insert_char(c);
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                }
            }
            QMindView::SemanticSearch => {
                match key.code {
                    KeyCode::Esc => {
                        self.state.view = QMindView::Overview;
                        KeyHandleResult::Handled
                    }
                    KeyCode::Enter => {
                        // TODO: Execute semantic search
                        KeyHandleResult::Handled
                    }
                    KeyCode::Backspace => {
                        self.state.search_input.backspace();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Delete => {
                        self.state.search_input.delete();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Left => {
                        self.state.search_input.cursor_left();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Right => {
                        self.state.search_input.cursor_right();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char(c) => {
                        self.state.search_input.insert_char(c);
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                }
            }
            QMindView::IndexStatus => match key.code {
                KeyCode::Esc => {
                    self.state.view = QMindView::Overview;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    // TODO: Refresh/rebuild index
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            QMindView::FileSummary => match key.code {
                KeyCode::Esc => {
                    self.state.view = QMindView::Overview;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_qmind_modal(frame, area, &self.state, self.loading, colors);
    }

    fn tick(&mut self) {
        if self.loading {
            self.initialize();
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-MIND - AI Intelligence Layer".to_string(),
            "".to_string(),
            "Natural language commands and semantic search".to_string(),
            "for your files.".to_string(),
            "".to_string(),
            "Global Keys:".to_string(),
            "  ?       Open command palette (from anywhere)".to_string(),
            "".to_string(),
            "Features:".to_string(),
            "  C       Command palette (natural language)".to_string(),
            "  S       Semantic search".to_string(),
            "  I       Index status".to_string(),
            "  Esc     Close/Back".to_string(),
            "".to_string(),
            "Examples:".to_string(),
            "  'copy *.txt to backup'".to_string(),
            "  'find that config file for rust'".to_string(),
            "  'delete old log files'".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Q-MIND".to_string(),
            description: "AI-powered commands & search".to_string(),
            category: PluginCategory::Tools,
            key: '?',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.start_loading();
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
