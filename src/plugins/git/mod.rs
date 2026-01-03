//! Git Plugin for R-DOS
//!
//! Provides Git integration as a plugin with self-contained operations.

pub mod ops;

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem, PluginStatusInfo};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;
use std::process::Command;

/// Git plugin that provides version control integration
pub struct GitPlugin {
    /// Whether the plugin is initialized
    initialized: bool,
    /// Cached info about whether we're in a git repo
    is_repo: bool,
    /// Current branch name
    branch: String,
    /// Number of staged files
    staged: usize,
    /// Number of modified files
    modified: usize,
    /// Commits ahead of remote
    ahead: u32,
    /// Commits behind remote
    behind: u32,
}

impl GitPlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
            is_repo: false,
            branch: String::new(),
            staged: 0,
            modified: 0,
            ahead: 0,
            behind: 0,
        }
    }

    /// Check if a directory is a git repository
    fn check_is_repo(&self, cwd: &PathBuf) -> bool {
        Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(cwd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Update cached git status
    fn refresh_status(&mut self, cwd: &PathBuf) {
        self.is_repo = self.check_is_repo(cwd);
        if !self.is_repo {
            self.branch = String::new();
            self.staged = 0;
            self.modified = 0;
            self.ahead = 0;
            self.behind = 0;
            return;
        }

        // Get branch name
        if let Ok(output) = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(cwd)
            .output()
        {
            self.branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }

        // Get status counts
        if let Ok(output) = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(cwd)
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            self.staged = 0;
            self.modified = 0;
            for line in stdout.lines() {
                if line.len() >= 2 {
                    let first = line.chars().next().unwrap_or(' ');
                    let second = line.chars().nth(1).unwrap_or(' ');
                    if first != ' ' && first != '?' {
                        self.staged += 1;
                    }
                    if second != ' ' {
                        self.modified += 1;
                    }
                }
            }
        }

        // Get ahead/behind counts
        if let Ok(output) = Command::new("git")
            .args(["rev-list", "--left-right", "--count", "@{u}...HEAD"])
            .current_dir(cwd)
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = stdout.split_whitespace().collect();
                if parts.len() == 2 {
                    self.behind = parts[0].parse().unwrap_or(0);
                    self.ahead = parts[1].parse().unwrap_or(0);
                }
            }
        }
    }
}

impl Default for GitPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for GitPlugin {
    fn id(&self) -> &str {
        "git"
    }

    fn name(&self) -> &str {
        "Git Integration"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: false, // Uses existing Modal::Git for now
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
        self.check_is_repo(cwd)
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Git".to_string(),
            key: 'G',
            description: "Git version control menu".to_string(),
            priority: 10, // Show early in menu
        })
    }

    fn status_info(&self, cwd: &PathBuf) -> Option<PluginStatusInfo> {
        if !self.check_is_repo(cwd) {
            return None;
        }

        // Build status text similar to existing format
        let mut parts = Vec::new();
        if self.ahead > 0 || self.behind > 0 {
            parts.push(format!("↑{}↓{}", self.ahead, self.behind));
        }
        if self.staged > 0 {
            parts.push(format!("+{}", self.staged));
        }
        if self.modified > 0 {
            parts.push(format!("!{}", self.modified));
        }

        let text = if parts.is_empty() {
            self.branch.clone()
        } else {
            format!("{} {}", parts.join(" "), self.branch)
        };

        Some(PluginStatusInfo { text, active: true })
    }

    fn handle_global_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // The 'G' key is handled by the main app for now
        // This could be moved here in the future
        match key.code {
            KeyCode::Char('g') | KeyCode::Char('G') => {
                // For now, let the main app handle this
                KeyHandleResult::NotHandled
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Modal handling is done by the existing GitState
        KeyHandleResult::NotHandled
    }

    fn draw_modal(&self, _frame: &mut Frame, _area: Rect) {
        // Modal drawing is done by the existing draw_git_modal
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "G - Open Git menu".to_string(),
            "  Status - View file changes".to_string(),
            "  Log - View commit history".to_string(),
            "  Branches - Manage branches".to_string(),
            "  Stash - Manage stashed changes".to_string(),
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
    fn test_git_plugin_creation() {
        let plugin = GitPlugin::new();
        assert_eq!(plugin.id(), "git");
        assert_eq!(plugin.name(), "Git Integration");
        assert!(plugin.capabilities().has_menu);
        assert!(plugin.capabilities().has_status);
    }

    #[test]
    fn test_git_plugin_menu_item() {
        let plugin = GitPlugin::new();
        let menu = plugin.menu_item().unwrap();
        assert_eq!(menu.key, 'G');
        assert_eq!(menu.name, "Git");
    }
}
