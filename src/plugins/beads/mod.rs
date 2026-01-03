//! Beads Plugin for R-DOS
//!
//! Provides Beads issue tracker integration as a plugin with self-contained operations.

pub mod ops;
pub mod state;

// Re-export state types for external use
#[allow(unused_imports)]
pub use state::{
    BeadsActivityEntry, BeadsComment, BeadsIssue, BeadsMenuItem, BeadsState, BeadsStats,
    BeadsSubIssue, BeadsView,
};

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem, PluginStatusInfo};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;
use std::process::Command;

/// Beads plugin that provides issue tracking integration
pub struct BeadsPlugin {
    /// Whether the plugin is initialized
    initialized: bool,
    /// Cached info about whether we're in a beads project
    is_beads: bool,
    /// Number of open issues
    open_count: u32,
    /// Number of in-progress issues
    in_progress_count: u32,
    /// Number of ready issues (no blockers)
    ready_count: u32,
    /// Modal state when beads modal is open (plugin owns this state)
    pub modal_state: Option<BeadsState>,
}

impl BeadsPlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
            is_beads: false,
            open_count: 0,
            in_progress_count: 0,
            ready_count: 0,
            modal_state: None,
        }
    }

    /// Open the beads modal with fresh state
    pub fn open_modal(&mut self, cwd: &PathBuf) {
        let is_beads = self.check_is_beads(cwd);
        self.modal_state = Some(BeadsState::new(is_beads));
    }

    /// Close the beads modal
    pub fn close_modal(&mut self) {
        self.modal_state = None;
    }

    /// Get mutable reference to modal state
    pub fn modal_state_mut(&mut self) -> Option<&mut BeadsState> {
        self.modal_state.as_mut()
    }

    /// Check if a directory has beads initialized
    fn check_is_beads(&self, cwd: &PathBuf) -> bool {
        cwd.join(".beads").exists()
    }

    /// Update cached beads status
    fn refresh_status(&mut self, cwd: &PathBuf) {
        self.is_beads = self.check_is_beads(cwd);
        if !self.is_beads {
            self.open_count = 0;
            self.in_progress_count = 0;
            self.ready_count = 0;
            return;
        }

        // Get stats using bd stats command
        if let Ok(output) = Command::new("bd")
            .args(["stats", "--json"])
            .current_dir(cwd)
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse JSON for counts - simplified parsing
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    self.open_count = json["open"].as_u64().unwrap_or(0) as u32;
                    self.in_progress_count = json["in_progress"].as_u64().unwrap_or(0) as u32;
                    self.ready_count = json["ready"].as_u64().unwrap_or(0) as u32;
                }
            }
        }
    }
}

impl Default for BeadsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for BeadsPlugin {
    fn id(&self) -> &str {
        "beads"
    }

    fn name(&self) -> &str {
        "Beads Issue Tracker"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true, // Plugin owns modal state
            has_status: true,
            has_cli: false,
            has_help: true,
        }
    }

    fn init(&mut self, cwd: &PathBuf) -> Result<(), String> {
        self.refresh_status(cwd);
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.initialized = false;
        Ok(())
    }

    fn is_available(&self, cwd: &PathBuf) -> bool {
        self.check_is_beads(cwd)
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Beads".to_string(),
            key: 'B',
            description: "Beads issue tracker menu".to_string(),
            priority: 20, // Show after Git
        })
    }

    fn status_info(&self, cwd: &PathBuf) -> Option<PluginStatusInfo> {
        if !self.check_is_beads(cwd) {
            return None;
        }

        // Build status text similar to existing format: "bd: ○19 ●3 ✓12"
        let mut parts = Vec::new();
        if self.open_count > 0 {
            parts.push(format!("○{}", self.open_count));
        }
        if self.in_progress_count > 0 {
            parts.push(format!("●{}", self.in_progress_count));
        }
        if self.ready_count > 0 {
            parts.push(format!("✓{}", self.ready_count));
        }

        let text = if parts.is_empty() {
            "bd: ✓".to_string() // All clear
        } else {
            format!("bd: {}", parts.join(" "))
        };

        Some(PluginStatusInfo { text, active: true })
    }

    fn handle_global_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Char('b') | KeyCode::Char('B') => {
                // Open beads modal with plugin-owned state
                self.open_modal(cwd);
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Modal handling is done by the existing BeadsState
        KeyHandleResult::NotHandled
    }

    fn draw_modal(&self, _frame: &mut Frame, _area: Rect) {
        // Modal drawing is done by the existing draw_beads_modal
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "B - Open Beads menu".to_string(),
            "  Issues - View and manage issues".to_string(),
            "  Create - Create new issue".to_string(),
            "  Ready - Show issues ready to work".to_string(),
            "  Stats - View project statistics".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beads_plugin_creation() {
        let plugin = BeadsPlugin::new();
        assert_eq!(plugin.id(), "beads");
        assert_eq!(plugin.name(), "Beads Issue Tracker");
        assert!(plugin.capabilities().has_menu);
        assert!(plugin.capabilities().has_status);
    }

    #[test]
    fn test_beads_plugin_menu_item() {
        let plugin = BeadsPlugin::new();
        let menu = plugin.menu_item().unwrap();
        assert_eq!(menu.key, 'B');
        assert_eq!(menu.name, "Beads");
    }
}
