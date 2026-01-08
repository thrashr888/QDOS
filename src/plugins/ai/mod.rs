//! AI Assistant plugin
//!
//! Monitors AI coding assistant CLI tools (Claude Code, OpenAI Codex, Gemini CLI).
//! Displays usage stats and configuration from local data files.

mod modal;
pub mod ops;
pub mod state;

use crate::plugins::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{AIMenuItem, AIState, AIView};
use std::any::Any;
use std::path::PathBuf;

// Re-export types for external use
#[allow(unused_imports)]
pub use state::{AIProvider, ClaudeStatus, CodexStatus, CopilotStatus, CursorStatus, GeminiStatus};

/// AI Assistant plugin for monitoring AI CLI tools
pub struct AIPlugin {
    initialized: bool,
    pub state: AIState,
    /// Whether data is currently being loaded (for lazy loading)
    loading: bool,
}

impl Default for AIPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl AIPlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
            state: AIState::new(),
            loading: false,
        }
    }

    /// Start loading - sets loading flag for lazy load on next tick
    pub fn start_loading(&mut self) {
        self.loading = true;
    }

    /// Check if currently loading
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Refresh status from all providers
    fn refresh_status(&mut self) {
        let (claude, codex, gemini, cursor, copilot) = ops::refresh_all_status();
        self.state.claude = claude;
        self.state.codex = codex;
        self.state.gemini = gemini;
        self.state.cursor = cursor;
        self.state.copilot = copilot;
        self.loading = false;
    }
}

impl Plugin for AIPlugin {
    fn id(&self) -> &str {
        "ai"
    }

    fn name(&self) -> &str {
        "AI Assistant"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true,
            has_status: true,
            has_cli: false,
            has_help: true,
        }
    }

    fn init(&mut self, _cwd: &PathBuf) -> Result<(), String> {
        self.refresh_status();
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.initialized = false;
        Ok(())
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        // AI plugin is always available (shows install info if tools not found)
        true
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "AI".to_string(),
            key: 'I',
            description: "Monitor AI coding assistant CLI tools".to_string(),
            priority: 55,
        })
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        // Show status for the first available provider
        if self.state.claude.available {
            let text = if let Some(ref today) = self.state.claude.today {
                format!("Claude {} msgs", today.message_count)
            } else {
                "Claude ✓".to_string()
            };
            return Some(PluginStatusInfo { text, active: true });
        }

        if self.state.codex.available {
            let model = self
                .state
                .codex
                .model
                .as_ref()
                .map(|m| m.split('-').take(2).collect::<Vec<_>>().join("-"))
                .unwrap_or_else(|| "ready".to_string());
            return Some(PluginStatusInfo {
                text: format!("Codex {}", model),
                active: true,
            });
        }

        if self.state.gemini.available {
            return Some(PluginStatusInfo {
                text: "Gemini ✓".to_string(),
                active: true,
            });
        }

        if self.state.cursor.available {
            let text = if self.state.cursor.code_generations > 0 {
                format!("Cursor {} gens", self.state.cursor.code_generations)
            } else {
                "Cursor ✓".to_string()
            };
            return Some(PluginStatusInfo { text, active: true });
        }

        if self.state.copilot.available {
            let text = if let Some(ref user) = self.state.copilot.github_user {
                format!("Copilot @{}", user)
            } else {
                "Copilot ✓".to_string()
            };
            return Some(PluginStatusInfo { text, active: true });
        }

        // No AI tools installed
        None
    }

    fn handle_global_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // No global key - AI plugin is accessed through plugin menu system
        // TODO: Add to F-key menu when Plugin Launcher (QDOS-f5ap) is implemented
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            AIView::Overview => match key.code {
                KeyCode::Esc => KeyHandleResult::CloseModal,
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.state.menu_index > 0 {
                        self.state.menu_index -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.state.menu_index < AIMenuItem::ALL.len() - 1 {
                        self.state.menu_index += 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    // Navigate to selected view
                    self.state.view = match AIMenuItem::ALL[self.state.menu_index] {
                        AIMenuItem::Claude => AIView::Claude,
                        AIMenuItem::Codex => AIView::Codex,
                        AIMenuItem::Gemini => AIView::Gemini,
                        AIMenuItem::Cursor => AIView::Cursor,
                        AIMenuItem::Copilot => AIView::Copilot,
                    };
                    KeyHandleResult::Handled
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.state.view = AIView::Claude;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    self.state.view = AIView::Codex;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('g') | KeyCode::Char('G') => {
                    self.state.view = AIView::Gemini;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('u') | KeyCode::Char('U') => {
                    self.state.view = AIView::Cursor;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    self.state.view = AIView::Copilot;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.refresh_status();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            AIView::Claude | AIView::Codex | AIView::Gemini | AIView::Cursor | AIView::Copilot => {
                match key.code {
                    KeyCode::Esc => {
                        self.state.view = AIView::Overview;
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        self.refresh_status();
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                }
            }
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_ai_modal(frame, area, &self.state, self.loading, colors);
    }

    fn tick(&mut self) {
        // Lazy load data on first tick after modal opens
        if self.loading {
            self.refresh_status();
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "AI Assistant - Monitor AI coding tools".to_string(),
            "".to_string(),
            "Supported tools:".to_string(),
            "  Claude Code (~/.claude/)".to_string(),
            "  OpenAI Codex (~/.codex/)".to_string(),
            "  Gemini CLI   (~/.gemini/)".to_string(),
            "  Cursor IDE   (~/.cursor/)".to_string(),
            "  GitHub Copilot (gh auth)".to_string(),
            "".to_string(),
            "Keys:".to_string(),
            "  C       Claude Code status".to_string(),
            "  X       Codex status".to_string(),
            "  G       Gemini status".to_string(),
            "  U       Cursor status".to_string(),
            "  P       Copilot status".to_string(),
            "  R       Refresh status".to_string(),
            "  Esc     Close/Back".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "AI Assistant".to_string(),
            description: "Monitor AI coding tools".to_string(),
            category: PluginCategory::Tools,
            key: 'A',
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
