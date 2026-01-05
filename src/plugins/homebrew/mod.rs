//! Homebrew plugin
//!
//! Browse and install Homebrew packages on macOS.
//! Accessible via F12 Apps launcher.
//!
//! Uses the `homebrew` crate for CLI operations.

mod modal;
pub mod state;

use crate::plugins::{
    KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem, PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use homebrew::Homebrew;
use ratatui::{layout::Rect, Frame};
use state::{
    ConfirmAction, HomebrewState, HomebrewTab, HomebrewView, PackageCategory, PackageEntry,
    PackageInfo, PackageStatus,
};
use std::any::Any;
use std::path::PathBuf;

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
    /// Homebrew CLI wrapper
    brew: Option<Homebrew>,
}

impl Default for HomebrewPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl HomebrewPlugin {
    pub fn new() -> Self {
        // Try to create Homebrew instance, but don't fail if not available
        let brew = Homebrew::new().ok();

        Self {
            initialized: false,
            state: HomebrewState::new(),
            install_package: None,
            brew,
        }
    }

    /// Check if Homebrew is available
    fn check_homebrew(&self) -> bool {
        self.brew.as_ref().is_some_and(|b| b.is_available())
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

        let brew = match &self.brew {
            Some(b) => b,
            None => {
                self.state.clear_loading();
                return;
            }
        };

        self.state.set_loading("Loading installed packages...");

        // Get installed packages with versions using the crate
        let installed: Vec<(String, String)> = brew
            .list()
            .map(|pkgs| pkgs.into_iter().map(|p| (p.name, p.version)).collect())
            .unwrap_or_default();

        // Get outdated packages using the crate
        self.state.set_loading("Checking for updates...");
        let outdated = brew.outdated().unwrap_or_default();
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

    /// Get detailed info for a package using the crate
    fn get_package_info(&self, name: &str) -> Option<PackageInfo> {
        let brew = self.brew.as_ref()?;

        // Extract package name from tap/formula format
        let pkg_name = name.split('/').next_back().unwrap_or(name);

        match brew.info(pkg_name) {
            Ok(pkg) => Some(PackageInfo {
                name: pkg.name,
                version: pkg.version,
                description: pkg.description,
                homepage: pkg.homepage,
                installed: pkg.installed,
                installed_version: pkg.installed_version,
                dependencies: pkg.dependencies,
                caveats: pkg.caveats,
            }),
            Err(_) => None,
        }
    }

    /// Run brew update and show output
    fn run_brew_update(&mut self) {
        self.state.set_loading("Running brew update...");
        self.state.last_command = Some("brew update".to_string());

        let result = match &self.brew {
            Some(brew) => brew.update(),
            None => {
                self.state.command_output = Some("Homebrew not available".to_string());
                self.state.output_scroll = 0;
                self.state.view = HomebrewView::Output;
                self.state.clear_loading();
                return;
            }
        };

        match result {
            Ok(output) => {
                self.state.command_output = Some(output.combined());
            }
            Err(e) => {
                self.state.command_output = Some(format!("Error: {}", e));
            }
        }

        self.state.output_scroll = 0;
        self.state.view = HomebrewView::Output;
        self.state.clear_loading();
    }

    /// Run a brew command and show output
    fn run_brew_command(&mut self, command: &str, pkg_name: &str) {
        self.state
            .set_loading(&format!("Running brew {} {}...", command, pkg_name));
        self.state.last_command = Some(format!("brew {} {}", command, pkg_name));

        let result = match &self.brew {
            Some(brew) => match command {
                "install" => brew.install(pkg_name),
                "uninstall" => brew.uninstall(pkg_name),
                "upgrade" => brew.upgrade(pkg_name),
                _ => {
                    self.state.command_output = Some(format!("Unknown command: {}", command));
                    self.state.output_scroll = 0;
                    self.state.view = HomebrewView::Output;
                    self.state.clear_loading();
                    return;
                }
            },
            None => {
                self.state.command_output = Some("Homebrew not available".to_string());
                self.state.output_scroll = 0;
                self.state.view = HomebrewView::Output;
                self.state.clear_loading();
                return;
            }
        };

        match result {
            Ok(output) => {
                self.state.command_output = Some(output.combined());
            }
            Err(e) => {
                self.state.command_output = Some(format!("Error: {}", e));
            }
        }

        self.state.output_scroll = 0;
        self.state.view = HomebrewView::Output;
        self.state.clear_loading();
    }

    /// Run brew upgrade all
    fn run_brew_upgrade_all(&mut self) {
        self.state.set_loading("Running brew upgrade...");
        self.state.last_command = Some("brew upgrade".to_string());

        let result = match &self.brew {
            Some(brew) => brew.upgrade_all(),
            None => {
                self.state.command_output = Some("Homebrew not available".to_string());
                self.state.output_scroll = 0;
                self.state.view = HomebrewView::Output;
                self.state.clear_loading();
                return;
            }
        };

        match result {
            Ok(output) => {
                self.state.command_output = Some(output.combined());
            }
            Err(e) => {
                self.state.command_output = Some(format!("Error: {}", e));
            }
        }

        self.state.output_scroll = 0;
        self.state.view = HomebrewView::Output;
        self.state.clear_loading();
    }

    /// Search for packages using the crate
    fn search_packages(&mut self, query: &str) {
        self.state
            .set_loading(&format!("Searching for '{}'...", query));

        let brew = match &self.brew {
            Some(b) => b,
            None => {
                self.state.clear_loading();
                return;
            }
        };

        // Get search results from the crate
        let results = match brew.search(query) {
            Ok(r) => r,
            Err(_) => {
                self.state.error = Some("Search failed".to_string());
                self.state.clear_loading();
                return;
            }
        };

        // Get currently installed packages to show status
        let installed: Vec<String> = brew
            .list()
            .map(|pkgs| pkgs.into_iter().map(|p| p.name).collect())
            .unwrap_or_default();

        // Remove old search results
        self.state
            .packages
            .retain(|p| p.category != PackageCategory::SearchResults);

        // Add search results
        for result in results {
            // Skip casks for now
            if result.is_cask {
                continue;
            }

            let is_installed = installed.contains(&result.name);
            self.state.packages.push(PackageEntry {
                name: result.name.clone(),
                description: String::new(), // Search doesn't give descriptions
                version: None,
                installed_version: None,
                category: PackageCategory::SearchResults,
                status: if is_installed {
                    PackageStatus::Installed
                } else {
                    PackageStatus::Available
                },
            });
        }

        // Switch to search tab
        self.state.tab = HomebrewTab::Search;
        self.state.selected_index = 0;
        self.state.clear_loading();
    }

    /// Take the install package name (for external use)
    pub fn take_install_package(&mut self) -> Option<String> {
        self.install_package.take()
    }

    /// Open the modal
    pub fn open_modal(&mut self) {
        self.state.view = HomebrewView::List;
        self.state.tab = HomebrewTab::Recommended;
        self.state.selected_index = 0;
        self.refresh_packages();
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
        // Only available on macOS where Homebrew typically exists
        cfg!(target_os = "macos")
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Homebrew".to_string(),
            key: 'H',
            description: "Homebrew package manager".to_string(),
            priority: 55, // After Shell
        })
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn handle_global_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Homebrew plugin doesn't have a global key - accessed via F12 Apps launcher
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
            "Homebrew Package Manager".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  Tab        Switch tabs (Recommended/Installed/Search)".to_string(),
            "  ↑/↓        Navigate packages".to_string(),
            "  Enter      Install/view info".to_string(),
            "  i          View package info".to_string(),
            "".to_string(),
            "Actions:".to_string(),
            "  /          Search packages".to_string(),
            "  u          Update Homebrew".to_string(),
            "  g          Upgrade package".to_string(),
            "  G          Upgrade all packages".to_string(),
            "  x          Uninstall package".to_string(),
            "  o          Toggle outdated filter".to_string(),
            "  r          Refresh package list".to_string(),
            "".to_string(),
            "Info View:".to_string(),
            "  h          Open homepage in browser".to_string(),
            "  Enter      Install (if not installed)".to_string(),
            "  g          Upgrade (if installed)".to_string(),
            "  x          Uninstall (if installed)".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Key handlers
impl HomebrewPlugin {
    fn handle_list_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => KeyHandleResult::CloseModal,

            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_prev();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next();
                KeyHandleResult::Handled
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                self.state.next_tab();
                KeyHandleResult::Handled
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                self.state.prev_tab();
                KeyHandleResult::Handled
            }

            // Actions
            KeyCode::Enter => {
                if let Some(pkg) = self.state.selected_package() {
                    let pkg_name = pkg.name.clone();
                    if pkg.status == PackageStatus::Installed
                        || pkg.status == PackageStatus::Outdated
                    {
                        // Show info for installed packages
                        self.state.package_info = self.get_package_info(&pkg_name);
                        self.state.view = HomebrewView::Info;
                    } else {
                        // Confirm install for available packages
                        self.state.confirm_action = Some(ConfirmAction::Install(pkg_name));
                        self.state.view = HomebrewView::Confirm;
                    }
                }
                KeyHandleResult::Handled
            }

            // Info view
            KeyCode::Char('i') => {
                if let Some(pkg) = self.state.selected_package() {
                    let pkg_name = pkg.name.clone();
                    self.state.package_info = self.get_package_info(&pkg_name);
                    self.state.view = HomebrewView::Info;
                }
                KeyHandleResult::Handled
            }

            // Search
            KeyCode::Char('/') => {
                self.state.search_query.clear();
                self.state.view = HomebrewView::SearchInput;
                KeyHandleResult::Handled
            }

            // Update
            KeyCode::Char('u') => {
                self.state.confirm_action = Some(ConfirmAction::Update);
                self.state.view = HomebrewView::Confirm;
                KeyHandleResult::Handled
            }

            // Upgrade
            KeyCode::Char('g') => {
                if let Some(pkg) = self.state.selected_package() {
                    if pkg.status == PackageStatus::Outdated
                        || pkg.status == PackageStatus::Installed
                    {
                        let pkg_name = pkg.name.clone();
                        self.state.confirm_action = Some(ConfirmAction::Upgrade(pkg_name));
                        self.state.view = HomebrewView::Confirm;
                    }
                }
                KeyHandleResult::Handled
            }

            // Upgrade all
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
                        let pkg_name = pkg.name.clone();
                        self.state.confirm_action = Some(ConfirmAction::Uninstall(pkg_name));
                        self.state.view = HomebrewView::Confirm;
                    }
                }
                KeyHandleResult::Handled
            }

            // Toggle outdated filter
            KeyCode::Char('o') => {
                self.state.show_outdated_only = !self.state.show_outdated_only;
                self.state.selected_index = 0;
                KeyHandleResult::Handled
            }

            // Refresh
            KeyCode::Char('r') => {
                self.refresh_packages();
                KeyHandleResult::Handled
            }

            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_search_input_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.search_query.clear();
                self.state.view = HomebrewView::List;
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
                    if let Some(brew) = &self.brew {
                        let _ = brew.home(&info.name);
                    }
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
                    match action {
                        ConfirmAction::Install(name) => {
                            self.run_brew_command("install", &name);
                        }
                        ConfirmAction::Uninstall(name) => {
                            self.run_brew_command("uninstall", &name);
                        }
                        ConfirmAction::Upgrade(name) => {
                            self.run_brew_command("upgrade", &name);
                        }
                        ConfirmAction::UpgradeAll => {
                            self.run_brew_upgrade_all();
                        }
                        ConfirmAction::Update => {
                            self.run_brew_update();
                        }
                    }
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
            // Any other key - go back and refresh
            _ => {
                self.state.command_output = None;
                self.state.last_command = None;
                self.state.output_scroll = 0;
                self.state.view = HomebrewView::List;
                // Refresh packages after command
                self.refresh_packages();
                KeyHandleResult::Handled
            }
        }
    }
}
