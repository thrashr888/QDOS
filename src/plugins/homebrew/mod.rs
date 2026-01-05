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
use state::{
    ConfirmAction, HomebrewState, HomebrewTab, HomebrewView, PackageCategory, PackageEntry,
    PackageInfo, PackageStatus,
};
use std::any::Any;
use std::path::PathBuf;
use std::process::Command;

/// Recommended packages for QDOS users
const RECOMMENDED_PACKAGES: &[(&str, &str)] = &[
    // Essential tools
    ("git", "Distributed version control system"),
    ("ripgrep", "Search tool like grep but faster"),
    ("fd", "Fast and user-friendly find alternative"),
    ("fzf", "Fuzzy finder for command line"),
    ("jq", "JSON processor"),
    ("tree", "Display directory tree"),
    // System tools
    ("htop", "Interactive process viewer"),
    ("ncdu", "NCurses disk usage analyzer"),
    ("tmux", "Terminal multiplexer"),
    // Development tools
    ("neovim", "Vim-fork focused on extensibility"),
    ("git-delta", "Syntax highlighting for git diffs"),
    ("lazygit", "Terminal UI for git commands"),
    ("jujutsu", "Git-compatible VCS"),
    // Retro/DOS tools
    ("dosbox-x", "DOS emulator with enhancements"),
    ("basic256", "BASIC programming for beginners"),
    ("basicterminal", "Terminal BASIC interpreter"),
];

/// Packages from custom taps (format: tap, formula, description)
const TAP_PACKAGES: &[(&str, &str, &str)] =
    &[("thrashr888/qdos", "beads", "Git-native issue tracker")];

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
        self.state.set_loading("Checking Homebrew...");
        self.state.error = None;
        self.state.packages.clear();

        // Check Homebrew availability
        self.state.homebrew_available = self.check_homebrew();
        if !self.state.homebrew_available {
            self.state.clear_loading();
            return;
        }

        self.state.set_loading("Loading installed packages...");

        // Get installed packages with versions
        let installed = self.get_installed_packages_with_versions();

        // Get outdated packages
        self.state.set_loading("Checking for updates...");
        let outdated = self.get_outdated_packages();
        self.state.outdated_count = outdated.len();

        // Add recommended packages
        for (name, desc) in RECOMMENDED_PACKAGES {
            let is_installed = installed.iter().any(|(n, _)| n == name);
            let is_outdated = outdated.contains(&name.to_string());
            let status = if is_outdated {
                PackageStatus::Outdated
            } else if is_installed {
                PackageStatus::Installed
            } else {
                PackageStatus::Available
            };

            self.state.packages.push(PackageEntry {
                name: name.to_string(),
                description: desc.to_string(),
                version: None,
                installed_version: installed
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, v)| v.clone()),
                category: PackageCategory::Recommended,
                status,
            });
        }

        // Add tap packages (like beads)
        for (tap, formula, desc) in TAP_PACKAGES {
            let full_name = format!("{}/{}", tap, formula);
            let is_installed = installed.iter().any(|(n, _)| n == *formula);
            let is_outdated = outdated.contains(&formula.to_string());
            let status = if is_outdated {
                PackageStatus::Outdated
            } else if is_installed {
                PackageStatus::Installed
            } else {
                PackageStatus::Available
            };

            self.state.packages.push(PackageEntry {
                name: full_name,
                description: desc.to_string(),
                version: None,
                installed_version: installed
                    .iter()
                    .find(|(n, _)| n == *formula)
                    .map(|(_, v)| v.clone()),
                category: PackageCategory::Recommended,
                status,
            });
        }

        // Add all installed packages to Installed tab
        for (name, version) in &installed {
            // Skip if already in recommended
            if self
                .state
                .packages
                .iter()
                .any(|p| p.name == *name || p.name.ends_with(&format!("/{}", name)))
            {
                continue;
            }

            let is_outdated = outdated.contains(name);
            self.state.packages.push(PackageEntry {
                name: name.clone(),
                description: String::new(),
                version: None,
                installed_version: Some(version.clone()),
                category: PackageCategory::Installed,
                status: if is_outdated {
                    PackageStatus::Outdated
                } else {
                    PackageStatus::Installed
                },
            });
        }

        self.state.clear_loading();
    }

    /// Get list of installed packages with versions
    fn get_installed_packages_with_versions(&self) -> Vec<(String, String)> {
        if let Ok(output) = Command::new("brew")
            .args(["list", "--versions", "--formula"])
            .output()
        {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter_map(|line| {
                        let mut parts = line.split_whitespace();
                        let name = parts.next()?.to_string();
                        let version = parts.next().unwrap_or("").to_string();
                        Some((name, version))
                    })
                    .collect();
            }
        }
        Vec::new()
    }

    /// Get list of outdated packages
    fn get_outdated_packages(&self) -> Vec<String> {
        if let Ok(output) = Command::new("brew")
            .args(["outdated", "--formula"])
            .output()
        {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(|s| s.split_whitespace().next().unwrap_or(s).to_string())
                    .collect();
            }
        }
        Vec::new()
    }

    /// Get detailed info for a package
    fn get_package_info(&self, name: &str) -> Option<PackageInfo> {
        let output = Command::new("brew")
            .args(["info", "--json=v2", name])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        // Parse JSON manually (avoiding serde dependency)
        self.parse_package_info(&json_str, name)
    }

    /// Parse package info from brew info JSON output
    fn parse_package_info(&self, json: &str, name: &str) -> Option<PackageInfo> {
        // Simple JSON parsing without serde
        let mut info = PackageInfo {
            name: name.to_string(),
            ..Default::default()
        };

        // Extract version from "stable": "x.x.x"
        if let Some(start) = json.find("\"stable\":") {
            let after = &json[start + 10..];
            // Skip whitespace and find the opening quote
            let trimmed = after.trim_start();
            if trimmed.starts_with('"') {
                if let Some(end) = trimmed[1..].find('"') {
                    info.version = trimmed[1..end + 1].to_string();
                }
            }
        }

        // Extract description from "desc": "..."
        if let Some(start) = json.find("\"desc\":") {
            let after = &json[start + 7..];
            let trimmed = after.trim_start();
            if trimmed.starts_with('"') {
                if let Some(end) = trimmed[1..].find('"') {
                    info.description = trimmed[1..end + 1].to_string();
                }
            }
        }

        // Extract homepage from "homepage": "..."
        if let Some(start) = json.find("\"homepage\":") {
            let after = &json[start + 11..];
            let trimmed = after.trim_start();
            if trimmed.starts_with('"') {
                if let Some(end) = trimmed[1..].find('"') {
                    info.homepage = trimmed[1..end + 1].to_string();
                }
            }
        }

        // Check if installed - look for "installed": [ followed by { (non-empty array)
        // The pattern "installed": [\n        { indicates installed
        if let Some(start) = json.find("\"installed\":") {
            let after = &json[start + 12..];
            let trimmed = after.trim_start();
            // Check if array is non-empty (starts with [ and has content before ])
            if trimmed.starts_with('[') {
                let array_content = &trimmed[1..].trim_start();
                info.installed = array_content.starts_with('{');

                // Extract installed version if present
                if info.installed {
                    if let Some(ver_start) = array_content.find("\"version\":") {
                        let ver_after = &array_content[ver_start + 10..];
                        let ver_trimmed = ver_after.trim_start();
                        if ver_trimmed.starts_with('"') {
                            if let Some(end) = ver_trimmed[1..].find('"') {
                                info.installed_version = Some(ver_trimmed[1..end + 1].to_string());
                            }
                        }
                    }
                }
            }
        }

        Some(info)
    }

    /// Run brew update and show output
    fn run_brew_update(&mut self) {
        self.state.set_loading("Running brew update...");
        self.state.last_command = Some("brew update".to_string());

        // Run brew update and capture output
        match Command::new("brew").arg("update").output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let mut result = String::new();
                if !stdout.is_empty() {
                    result.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !result.is_empty() {
                        result.push('\n');
                    }
                    result.push_str(&stderr);
                }
                if result.is_empty() {
                    result = "Already up-to-date.".to_string();
                }
                self.state.command_output = Some(result);
            }
            Err(e) => {
                self.state.command_output = Some(format!("Error: {}", e));
            }
        }

        self.state.clear_loading();
        self.state.output_scroll = 0;
        self.state.view = HomebrewView::Output;
    }

    /// Run a brew command (install/uninstall/upgrade) and show output
    fn run_brew_command(&mut self, args: &[&str]) {
        if args.is_empty() {
            return;
        }

        let cmd_str = format!("brew {}", args.join(" "));
        self.state.set_loading(&format!("Running {}...", cmd_str));
        self.state.last_command = Some(cmd_str);

        // Run the command and capture output
        match Command::new("brew").args(args).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let mut result = String::new();
                if !stdout.is_empty() {
                    result.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !result.is_empty() {
                        result.push('\n');
                    }
                    result.push_str(&stderr);
                }
                if result.is_empty() {
                    result = "Command completed successfully.".to_string();
                }
                self.state.command_output = Some(result);
            }
            Err(e) => {
                self.state.command_output = Some(format!("Error: {}", e));
            }
        }

        self.state.clear_loading();
        self.state.output_scroll = 0;
        self.state.view = HomebrewView::Output;
    }

    /// Search for packages
    fn search_packages(&mut self, query: &str) {
        if query.is_empty() {
            return;
        }

        self.state.set_loading("Searching...");
        self.state.error = None;

        if let Ok(output) = Command::new("brew")
            .args(["search", "--formula", query])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let installed = self.get_installed_packages_with_versions();
                let installed_names: Vec<&str> =
                    installed.iter().map(|(n, _)| n.as_str()).collect();

                // Clear search results but keep recommended and installed
                self.state
                    .packages
                    .retain(|p| p.category != PackageCategory::SearchResults);

                for name in stdout.lines().take(30) {
                    let name = name.trim();
                    if name.is_empty() || name.contains("==>") {
                        continue;
                    }

                    let status = if installed_names.contains(&name) {
                        PackageStatus::Installed
                    } else {
                        PackageStatus::Available
                    };

                    self.state.packages.push(PackageEntry {
                        name: name.to_string(),
                        description: String::new(),
                        version: None,
                        installed_version: installed
                            .iter()
                            .find(|(n, _)| n == name)
                            .map(|(_, v)| v.clone()),
                        category: PackageCategory::SearchResults,
                        status,
                    });
                }

                // Switch to Search tab to show results
                self.state.tab = HomebrewTab::Search;
            } else {
                self.state.error = Some("Search failed".to_string());
            }
        } else {
            self.state.error = Some("Could not run brew search".to_string());
        }

        self.state.clear_loading();
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
            HomebrewView::SearchInput => self.handle_search_input_key(key),
            HomebrewView::Info => self.handle_info_key(key),
            HomebrewView::Confirm => self.handle_confirm_key(key),
            HomebrewView::Output => self.handle_output_key(key),
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_homebrew_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Homebrew Packages".to_string(),
            "".to_string(),
            "Browse, search, and manage Homebrew packages.".to_string(),
            "Access via F12 Apps launcher.".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  ↑↓/jk     Navigate package list".to_string(),
            "  Tab/←→    Switch tabs (Recommended/Installed/Search)".to_string(),
            "  Enter/i   View package info".to_string(),
            "  Esc       Close/back".to_string(),
            "".to_string(),
            "Actions:".to_string(),
            "  /         Search Homebrew packages".to_string(),
            "  r         Refresh package list".to_string(),
            "  u         Update Homebrew (brew update)".to_string(),
            "  g         Upgrade selected package".to_string(),
            "  G         Upgrade ALL outdated packages".to_string(),
            "  x/d       Uninstall selected package".to_string(),
            "".to_string(),
            "Status Icons:".to_string(),
            "  *         Installed".to_string(),
            "  ^         Update available".to_string(),
            "  ~         Installing".to_string(),
            "".to_string(),
            "Tabs:".to_string(),
            "  Recommended - QDOS essentials".to_string(),
            "  Installed   - All installed packages".to_string(),
            "  Search      - Search results".to_string(),
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
            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_prev();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next();
                KeyHandleResult::Handled
            }
            // Tab switching
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                self.state.next_tab();
                KeyHandleResult::Handled
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                self.state.prev_tab();
                KeyHandleResult::Handled
            }
            // Info view
            KeyCode::Enter | KeyCode::Char('i') => {
                if let Some(pkg) = self.state.selected_package() {
                    let pkg_name = pkg.name.clone();
                    // Get just the formula name for tap packages
                    let lookup_name = if pkg_name.contains('/') {
                        pkg_name.split('/').next_back().unwrap_or(&pkg_name)
                    } else {
                        &pkg_name
                    };
                    self.state.package_info = self.get_package_info(lookup_name);
                    self.state.view = HomebrewView::Info;
                }
                KeyHandleResult::Handled
            }
            // Search
            KeyCode::Char('/') => {
                self.state.view = HomebrewView::SearchInput;
                self.state.search_query.clear();
                KeyHandleResult::Handled
            }
            // Refresh
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.refresh_packages();
                KeyHandleResult::Handled
            }
            // Update homebrew (runs synchronously and refreshes)
            KeyCode::Char('u') => {
                self.run_brew_update();
                KeyHandleResult::Handled
            }
            // Upgrade selected package
            KeyCode::Char('g') => {
                if let Some(pkg) = self.state.selected_package() {
                    if pkg.status == PackageStatus::Outdated
                        || pkg.status == PackageStatus::Installed
                    {
                        self.state.confirm_action = Some(ConfirmAction::Upgrade(pkg.name.clone()));
                        self.state.view = HomebrewView::Confirm;
                    }
                }
                KeyHandleResult::Handled
            }
            // Upgrade all (Shift+G)
            KeyCode::Char('G') => {
                if self.state.outdated_count > 0 {
                    self.state.confirm_action = Some(ConfirmAction::UpgradeAll);
                    self.state.view = HomebrewView::Confirm;
                }
                KeyHandleResult::Handled
            }
            // Uninstall
            KeyCode::Char('x') | KeyCode::Char('d') => {
                if let Some(pkg) = self.state.selected_package() {
                    if pkg.status == PackageStatus::Installed
                        || pkg.status == PackageStatus::Outdated
                    {
                        self.state.confirm_action =
                            Some(ConfirmAction::Uninstall(pkg.name.clone()));
                        self.state.view = HomebrewView::Confirm;
                    }
                }
                KeyHandleResult::Handled
            }
            // Filter to outdated only
            KeyCode::Char('o') => {
                self.state.show_outdated_only = !self.state.show_outdated_only;
                self.state.selected_index = 0;
                KeyHandleResult::Handled
            }
            // Quick filter (just type)
            KeyCode::Backspace => {
                self.state.search_query.pop();
                self.state.selected_index = 0;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_search_input_key(&mut self, key: KeyEvent) -> KeyHandleResult {
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
                // Clear search query after search (results are in SearchResults category)
                self.state.search_query.clear();
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

    fn handle_info_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.state.view = HomebrewView::List;
                self.state.package_info = None;
                KeyHandleResult::Handled
            }
            // Install from info view
            KeyCode::Enter => {
                if let Some(ref info) = self.state.package_info {
                    if !info.installed {
                        self.state.confirm_action = Some(ConfirmAction::Install(info.name.clone()));
                        self.state.view = HomebrewView::Confirm;
                    }
                }
                KeyHandleResult::Handled
            }
            // Upgrade from info view
            KeyCode::Char('g') => {
                if let Some(ref info) = self.state.package_info {
                    if info.installed {
                        self.state.confirm_action = Some(ConfirmAction::Upgrade(info.name.clone()));
                        self.state.view = HomebrewView::Confirm;
                    }
                }
                KeyHandleResult::Handled
            }
            // Uninstall from info view
            KeyCode::Char('x') | KeyCode::Char('d') => {
                if let Some(ref info) = self.state.package_info {
                    if info.installed {
                        self.state.confirm_action =
                            Some(ConfirmAction::Uninstall(info.name.clone()));
                        self.state.view = HomebrewView::Confirm;
                    }
                }
                KeyHandleResult::Handled
            }
            // Open homepage
            KeyCode::Char('h') => {
                if let Some(ref info) = self.state.package_info {
                    // Use brew home to open the homepage URL
                    let _ = std::process::Command::new("brew")
                        .arg("home")
                        .arg(&info.name)
                        .spawn();
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                self.state.confirm_action = None;
                self.state.view = HomebrewView::List;
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(action) = self.state.confirm_action.take() {
                    // Run the command - it will switch to Output view
                    match action {
                        ConfirmAction::Install(pkg) => {
                            self.run_brew_command(&["install", &pkg]);
                        }
                        ConfirmAction::Uninstall(pkg) => {
                            self.run_brew_command(&["uninstall", &pkg]);
                        }
                        ConfirmAction::Upgrade(pkg) => {
                            self.run_brew_command(&["upgrade", &pkg]);
                        }
                        ConfirmAction::UpgradeAll => {
                            self.run_brew_command(&["upgrade"]);
                        }
                        ConfirmAction::Update => {
                            self.run_brew_update();
                        }
                    }
                    // View is now Output, not List
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_output_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            // Scroll up
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.output_scroll > 0 {
                    self.state.output_scroll -= 1;
                }
                KeyHandleResult::Handled
            }
            // Scroll down
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.output_scroll += 1;
                KeyHandleResult::Handled
            }
            // Any other key: refresh packages and go back to list
            _ => {
                self.state.command_output = None;
                self.state.last_command = None;
                self.refresh_packages();
                self.state.view = HomebrewView::List;
                KeyHandleResult::Handled
            }
        }
    }
}
