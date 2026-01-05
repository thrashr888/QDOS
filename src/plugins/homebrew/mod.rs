//! Homebrew plugin
//!
//! Browse and install Homebrew packages on macOS.
//! Accessible via F12 Apps launcher.

mod modal;
pub mod state;

use crate::plugins::{
    KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem, PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{HomebrewState, HomebrewView, PackageCategory, PackageEntry, PackageStatus};
use std::any::Any;
use std::path::PathBuf;
use std::process::Command;

/// Recommended packages for QDOS users
const RECOMMENDED_PACKAGES: &[(&str, &str)] = &[
    ("ripgrep", "Search tool like grep but faster"),
    ("fd", "Fast and user-friendly find alternative"),
    ("bat", "Cat clone with syntax highlighting"),
    ("eza", "Modern replacement for ls"),
    ("fzf", "Fuzzy finder for command line"),
    ("jq", "JSON processor"),
    ("tree", "Display directory tree"),
    ("htop", "Interactive process viewer"),
    ("ncdu", "NCurses disk usage analyzer"),
    ("tmux", "Terminal multiplexer"),
    ("neovim", "Vim-fork focused on extensibility"),
    ("git-delta", "Syntax highlighting for git diffs"),
    ("lazygit", "Terminal UI for git commands"),
    ("jujutsu", "Git-compatible VCS"),
    ("dosbox-x", "DOS emulator with enhancements"),
    ("basic256", "BASIC programming for beginners"),
    ("basicterminal", "Terminal BASIC interpreter"),
];

/// Homebrew plugin for package management
pub struct HomebrewPlugin {
    initialized: bool,
    pub state: HomebrewState,
    /// Package to install after modal closes
    pub install_package: Option<String>,
}

impl Default for HomebrewPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl HomebrewPlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
            state: HomebrewState::new(),
            install_package: None,
        }
    }

    /// Check if Homebrew is available
    fn check_homebrew(&self) -> bool {
        Command::new("brew")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Refresh the package list
    fn refresh_packages(&mut self) {
        self.state.loading = true;
        self.state.error = None;
        self.state.packages.clear();

        // Check Homebrew availability
        self.state.homebrew_available = self.check_homebrew();
        if !self.state.homebrew_available {
            self.state.loading = false;
            return;
        }

        // Get installed packages
        let installed = self.get_installed_packages();

        // Add recommended packages
        for (name, desc) in RECOMMENDED_PACKAGES {
            let status = if installed.contains(&name.to_string()) {
                PackageStatus::Installed
            } else {
                PackageStatus::Available
            };

            self.state.packages.push(PackageEntry {
                name: name.to_string(),
                description: desc.to_string(),
                version: None,
                installed_version: if status == PackageStatus::Installed {
                    self.get_package_version(name)
                } else {
                    None
                },
                category: PackageCategory::Recommended,
                status,
            });
        }

        self.state.loading = false;
    }

    /// Get list of installed packages
    fn get_installed_packages(&self) -> Vec<String> {
        if let Ok(output) = Command::new("brew").args(["list", "--formula"]).output() {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(|s| s.to_string())
                    .collect();
            }
        }
        Vec::new()
    }

    /// Get version of an installed package
    fn get_package_version(&self, name: &str) -> Option<String> {
        if let Ok(output) = Command::new("brew")
            .args(["list", "--versions", name])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Format is "package version"
                return stdout.split_whitespace().nth(1).map(|s| s.to_string());
            }
        }
        None
    }

    /// Search for packages
    fn search_packages(&mut self, query: &str) {
        if query.is_empty() {
            return;
        }

        self.state.loading = true;
        self.state.error = None;

        if let Ok(output) = Command::new("brew")
            .args(["search", "--formula", query])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let installed = self.get_installed_packages();

                // Clear search results but keep recommended
                self.state
                    .packages
                    .retain(|p| p.category == PackageCategory::Recommended);

                for name in stdout.lines().take(20) {
                    let name = name.trim();
                    if name.is_empty() || name.contains("==>") {
                        continue;
                    }

                    // Skip if already in recommended
                    if self
                        .state
                        .packages
                        .iter()
                        .any(|p| p.name == name && p.category == PackageCategory::Recommended)
                    {
                        continue;
                    }

                    let status = if installed.contains(&name.to_string()) {
                        PackageStatus::Installed
                    } else {
                        PackageStatus::Available
                    };

                    self.state.packages.push(PackageEntry {
                        name: name.to_string(),
                        description: "".to_string(),
                        version: None,
                        installed_version: None,
                        category: PackageCategory::SearchResults,
                        status,
                    });
                }
            } else {
                self.state.error = Some("Search failed".to_string());
            }
        } else {
            self.state.error = Some("Could not run brew search".to_string());
        }

        self.state.loading = false;
    }

    /// Take the package to install (consumes it)
    pub fn take_install_package(&mut self) -> Option<String> {
        self.install_package.take()
    }

    /// Open the Homebrew modal (called from Apps launcher)
    pub fn open_modal(&mut self) {
        self.refresh_packages();
        self.state.selected_index = 0;
        self.state.view = HomebrewView::List;
        self.state.clear_search();
        self.install_package = None;
    }
}

impl Plugin for HomebrewPlugin {
    fn id(&self) -> &str {
        "homebrew"
    }

    fn name(&self) -> &str {
        "Homebrew"
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

    fn init(&mut self, _cwd: &PathBuf) -> Result<(), String> {
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.initialized = false;
        Ok(())
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        // Only available on macOS
        cfg!(target_os = "macos")
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        None // Accessed via F12 Apps launcher, not plugin menu
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn handle_global_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // No global key - accessed via F12 Apps launcher
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            HomebrewView::List => self.handle_list_key(key),
            HomebrewView::Search => self.handle_search_key(key),
            HomebrewView::Details => self.handle_details_key(key),
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_homebrew_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "F7 - Homebrew Packages".to_string(),
            "".to_string(),
            "Browse and install Homebrew packages.".to_string(),
            "".to_string(),
            "Keys:".to_string(),
            "  F7        Open Homebrew modal".to_string(),
            "  ↑↓/jk     Navigate list".to_string(),
            "  Enter     Install selected package".to_string(),
            "  /         Search packages".to_string(),
            "  R         Refresh package list".to_string(),
            "  Esc       Close".to_string(),
            "".to_string(),
            "Status Icons:".to_string(),
            "  *         Installed".to_string(),
            "  ^         Update available".to_string(),
            "  ~         Installing".to_string(),
            "".to_string(),
            "Recommended tools for QDOS:".to_string(),
            "  ripgrep, fd, bat, eza, fzf".to_string(),
            "  jq, tree, htop, ncdu, tmux".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl HomebrewPlugin {
    fn handle_list_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.clear_search();
                KeyHandleResult::CloseModal
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
                // Install selected package
                if let Some(pkg) = self.state.selected_package() {
                    if pkg.status == PackageStatus::Available {
                        self.install_package = Some(pkg.name.clone());
                        return KeyHandleResult::CloseWithSuccess(format!(
                            "homebrew:install:{}",
                            pkg.name
                        ));
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('/') => {
                self.state.view = HomebrewView::Search;
                self.state.search_query.clear();
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.refresh_packages();
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.search_query.pop();
                self.state.selected_index = 0;
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                // Type to filter
                self.state.search_query.push(c);
                self.state.selected_index = 0;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = HomebrewView::List;
                self.state.search_query.clear();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Execute search
                let query = self.state.search_query.clone();
                self.search_packages(&query);
                self.state.view = HomebrewView::List;
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.search_query.pop();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.search_query.push(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_details_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = HomebrewView::List;
                KeyHandleResult::Handled
            }
            KeyCode::Char('i') => {
                // Install
                if let Some(pkg) = self.state.selected_package() {
                    if pkg.status == PackageStatus::Available {
                        self.install_package = Some(pkg.name.clone());
                        return KeyHandleResult::CloseWithSuccess(format!(
                            "homebrew:install:{}",
                            pkg.name
                        ));
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}
