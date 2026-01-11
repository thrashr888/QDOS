//! Dependency Manager Plugin
//!
//! Multi-language package manager integration for viewing, installing,
//! updating, and managing project dependencies.

mod detect;
mod modal;
mod ops;
mod state;

pub use state::DepsState;

use crate::app::ThemeColors;
use crate::plugins::{AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, Frame};
use std::any::Any;
use std::path::PathBuf;

use detect::{detect_package_manager, get_project_name};
use modal::draw_deps_modal;
use ops::{
    check_outdated, install_package, list_packages, search_packages, uninstall_package, update_all,
    update_package,
};
use state::ConfirmAction;

/// Dependency Manager plugin
pub struct DepsPlugin {
    state: DepsState,
    modal_open: bool,
}

impl DepsPlugin {
    pub fn new() -> Self {
        Self {
            state: DepsState::new(),
            modal_open: false,
        }
    }

    /// Initialize the plugin for a directory
    fn initialize(&mut self, cwd: &PathBuf) {
        self.state.reset();

        // Detect package manager
        if let Some(pm) = detect_package_manager(cwd) {
            self.state.package_manager = Some(pm);
            self.state.project_name = get_project_name(pm, cwd);

            // Load packages
            self.load_packages(cwd);
        } else {
            self.state.error = Some("No package manager detected".to_string());
        }
    }

    /// Load packages from the current project
    fn load_packages(&mut self, cwd: &PathBuf) {
        if let Some(pm) = self.state.package_manager {
            self.state.set_loading("Loading packages...");

            match list_packages(pm, cwd) {
                Ok(packages) => {
                    self.state.total_count = packages.len();
                    self.state.packages = packages;
                    self.state.clear_loading();

                    // Check for outdated packages in background
                    self.check_outdated(cwd);
                }
                Err(e) => {
                    self.state.clear_loading();
                    self.state.error = Some(e);
                }
            }
        }
    }

    /// Check for outdated packages
    fn check_outdated(&mut self, cwd: &PathBuf) {
        if let Some(pm) = self.state.package_manager {
            match check_outdated(pm, cwd) {
                Ok(outdated) => {
                    // Mark packages as outdated
                    for pkg in &mut self.state.packages {
                        if let Some(out_pkg) = outdated.iter().find(|p| p.name == pkg.name) {
                            pkg.is_outdated = true;
                            pkg.latest_version = out_pkg.latest_version.clone();
                        }
                    }
                    self.state.outdated_count =
                        self.state.packages.iter().filter(|p| p.is_outdated).count();
                }
                Err(_) => {
                    // Silently ignore - outdated checking is optional
                }
            }
        }
    }

    /// Execute install action
    fn do_install(&mut self, name: &str, dev: bool, cwd: &PathBuf) {
        if let Some(pm) = self.state.package_manager {
            self.state.set_loading(&format!("Installing {}...", name));
            self.state.view = state::DepsView::Output;

            match install_package(pm, name, dev, cwd) {
                Ok(output) => {
                    self.state.clear_loading();
                    self.state.command_output = output.lines().map(String::from).collect();
                    self.state.message = Some(format!("Installed {}", name));
                }
                Err(e) => {
                    self.state.clear_loading();
                    self.state.command_output = vec![e.clone()];
                    self.state.error = Some(e);
                }
            }
        }
    }

    /// Execute uninstall action
    fn do_uninstall(&mut self, name: &str, cwd: &PathBuf) {
        if let Some(pm) = self.state.package_manager {
            self.state.set_loading(&format!("Uninstalling {}...", name));
            self.state.view = state::DepsView::Output;

            match uninstall_package(pm, name, cwd) {
                Ok(output) => {
                    self.state.clear_loading();
                    self.state.command_output = output.lines().map(String::from).collect();
                    self.state.message = Some(format!("Uninstalled {}", name));
                    // Remove from list
                    self.state.packages.retain(|p| p.name != name);
                    self.state.total_count = self.state.packages.len();
                }
                Err(e) => {
                    self.state.clear_loading();
                    self.state.command_output = vec![e.clone()];
                    self.state.error = Some(e);
                }
            }
        }
    }

    /// Execute update action
    fn do_update(&mut self, name: &str, cwd: &PathBuf) {
        if let Some(pm) = self.state.package_manager {
            self.state.set_loading(&format!("Updating {}...", name));
            self.state.view = state::DepsView::Output;

            match update_package(pm, name, cwd) {
                Ok(output) => {
                    self.state.clear_loading();
                    self.state.command_output = output.lines().map(String::from).collect();
                    self.state.message = Some(format!("Updated {}", name));
                    // Mark as no longer outdated
                    if let Some(pkg) = self.state.packages.iter_mut().find(|p| p.name == name) {
                        pkg.is_outdated = false;
                        pkg.current_version = pkg.latest_version.clone();
                    }
                    self.state.outdated_count =
                        self.state.packages.iter().filter(|p| p.is_outdated).count();
                }
                Err(e) => {
                    self.state.clear_loading();
                    self.state.command_output = vec![e.clone()];
                    self.state.error = Some(e);
                }
            }
        }
    }

    /// Execute update all action
    fn do_update_all(&mut self, cwd: &PathBuf) {
        if let Some(pm) = self.state.package_manager {
            self.state.set_loading("Updating all packages...");
            self.state.view = state::DepsView::Output;

            match update_all(pm, cwd) {
                Ok(output) => {
                    self.state.clear_loading();
                    self.state.command_output = output.lines().map(String::from).collect();
                    self.state.message = Some("Updated all packages".to_string());
                    // Reload packages
                    self.load_packages(cwd);
                }
                Err(e) => {
                    self.state.clear_loading();
                    self.state.command_output = vec![e.clone()];
                    self.state.error = Some(e);
                }
            }
        }
    }

    /// Execute search
    fn do_search(&mut self, query: &str, cwd: &PathBuf) {
        if let Some(pm) = self.state.package_manager {
            self.state.set_loading("Searching...");
            self.state.view = state::DepsView::Search;

            match search_packages(pm, query, cwd) {
                Ok(results) => {
                    self.state.clear_loading();
                    self.state.search_results = results;
                    self.state.selected_result = 0;
                }
                Err(e) => {
                    self.state.clear_loading();
                    self.state.error = Some(e);
                }
            }
        }
    }

    // Key handlers for different views

    fn handle_list_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        // Clear messages on any key
        self.state.message = None;
        self.state.error = None;

        match (key.code, key.modifiers) {
            // Navigation
            (KeyCode::Up | KeyCode::Char('k'), _) => {
                self.state.move_up();
                KeyHandleResult::Handled
            }
            (KeyCode::Down | KeyCode::Char('j'), _) => {
                self.state.move_down();
                KeyHandleResult::Handled
            }
            (KeyCode::Home | KeyCode::Char('g'), _) => {
                self.state.selected_index = 0;
                self.state.scroll_offset = 0;
                KeyHandleResult::Handled
            }
            (KeyCode::End | KeyCode::Char('G'), _) => {
                let max = self.state.visible_packages().len().saturating_sub(1);
                self.state.selected_index = max;
                KeyHandleResult::Handled
            }
            (KeyCode::PageUp, _) => {
                for _ in 0..10 {
                    self.state.move_up();
                }
                KeyHandleResult::Handled
            }
            (KeyCode::PageDown, _) => {
                for _ in 0..10 {
                    self.state.move_down();
                }
                KeyHandleResult::Handled
            }

            // Install
            (KeyCode::Char('i'), KeyModifiers::NONE) => {
                self.state.install_input.clear();
                self.state.install_cursor = 0;
                self.state.install_as_dev = false;
                self.state.view = state::DepsView::Install;
                KeyHandleResult::Handled
            }

            // Uninstall
            (KeyCode::Char('d') | KeyCode::Char('x'), KeyModifiers::NONE) => {
                if let Some(pkg) = self.state.selected_package() {
                    self.state.confirm_action = Some(ConfirmAction::Uninstall(pkg.name.clone()));
                    self.state.view = state::DepsView::Confirm;
                }
                KeyHandleResult::Handled
            }

            // Update
            (KeyCode::Char('u'), KeyModifiers::NONE) => {
                if let Some(pkg) = self.state.selected_package() {
                    if pkg.is_outdated {
                        self.state.confirm_action = Some(ConfirmAction::Update(pkg.name.clone()));
                        self.state.view = state::DepsView::Confirm;
                    }
                }
                KeyHandleResult::Handled
            }

            // Update all
            (KeyCode::Char('U'), KeyModifiers::SHIFT) => {
                if self.state.outdated_count > 0 {
                    self.state.confirm_action = Some(ConfirmAction::UpdateAll);
                    self.state.view = state::DepsView::Confirm;
                }
                KeyHandleResult::Handled
            }

            // Toggle outdated filter
            (KeyCode::Char('o'), KeyModifiers::NONE) => {
                self.state.show_outdated_only = !self.state.show_outdated_only;
                self.state.selected_index = 0;
                self.state.scroll_offset = 0;
                if self.state.show_outdated_only {
                    self.state.view = state::DepsView::Outdated;
                } else {
                    self.state.view = state::DepsView::List;
                }
                KeyHandleResult::Handled
            }

            // Toggle dev filter
            (KeyCode::Char('v'), KeyModifiers::NONE) => {
                self.state.show_dev_only = !self.state.show_dev_only;
                self.state.selected_index = 0;
                self.state.scroll_offset = 0;
                KeyHandleResult::Handled
            }

            // Search
            (KeyCode::Char('/'), _) => {
                self.state.search_query.clear();
                self.state.search_cursor = 0;
                self.state.view = state::DepsView::SearchInput;
                KeyHandleResult::Handled
            }

            // Refresh
            (KeyCode::Char('r'), KeyModifiers::NONE) => {
                self.load_packages(cwd);
                KeyHandleResult::Handled
            }

            // Tab to switch views
            (KeyCode::Tab, _) => {
                self.state.show_outdated_only = !self.state.show_outdated_only;
                self.state.selected_index = 0;
                self.state.scroll_offset = 0;
                if self.state.show_outdated_only {
                    self.state.view = state::DepsView::Outdated;
                } else {
                    self.state.view = state::DepsView::List;
                }
                KeyHandleResult::Handled
            }

            // Close
            (KeyCode::Esc | KeyCode::Char('q'), _) => {
                self.modal_open = false;
                KeyHandleResult::CloseModal
            }

            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_search_input_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = state::DepsView::List;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if !self.state.search_query.is_empty() {
                    let query = self.state.search_query.clone();
                    self.do_search(&query, cwd);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.backspace_search();
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                if self.state.search_cursor > 0 {
                    self.state.search_cursor -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                if self.state.search_cursor < self.state.search_query.len() {
                    self.state.search_cursor += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.insert_search_char(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_search_results_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = state::DepsView::List;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.selected_result > 0 {
                    self.state.selected_result -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.state.selected_result < self.state.search_results.len().saturating_sub(1) {
                    self.state.selected_result += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if let Some(result) = self.state.search_results.get(self.state.selected_result) {
                    let name = result.name.clone();
                    self.do_install(&name, false, cwd);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('/') => {
                self.state.search_query.clear();
                self.state.search_cursor = 0;
                self.state.view = state::DepsView::SearchInput;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_install_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = state::DepsView::List;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if !self.state.install_input.is_empty() {
                    let name = self.state.install_input.clone();
                    let dev = self.state.install_as_dev;
                    self.do_install(&name, dev, cwd);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                self.state.install_as_dev = !self.state.install_as_dev;
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.backspace_install();
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                if self.state.install_cursor > 0 {
                    self.state.install_cursor -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                if self.state.install_cursor < self.state.install_input.len() {
                    self.state.install_cursor += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.insert_install_char(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_output_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                self.state.command_output.clear();
                self.state.output_scroll = 0;
                self.state.view = state::DepsView::List;
                // Reload packages after operation
                self.load_packages(cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.output_scroll > 0 {
                    self.state.output_scroll -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.state.output_scroll < self.state.command_output.len().saturating_sub(1) {
                    self.state.output_scroll += 1;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_confirm_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if let Some(action) = self.state.confirm_action.take() {
                    match action {
                        ConfirmAction::Install(name, dev) => {
                            self.do_install(&name, dev, cwd);
                        }
                        ConfirmAction::Uninstall(name) => {
                            self.do_uninstall(&name, cwd);
                        }
                        ConfirmAction::Update(name) => {
                            self.do_update(&name, cwd);
                        }
                        ConfirmAction::UpdateAll => {
                            self.do_update_all(cwd);
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.state.confirm_action = None;
                self.state.view = state::DepsView::List;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}

impl Default for DepsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for DepsPlugin {
    fn id(&self) -> &str {
        "deps"
    }

    fn name(&self) -> &str {
        "Dependencies"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_modal: true,
            has_menu: true,
            ..Default::default()
        }
    }

    fn is_available(&self, cwd: &PathBuf) -> bool {
        detect_package_manager(cwd).is_some()
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: "deps".to_string(),
            name: "Dependencies".to_string(),
            description: "Package manager integration".to_string(),
            category: PluginCategory::Tools,
            key: 'Y',
        })
    }

    fn launch(&mut self, cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.initialize(cwd);
        self.modal_open = true;
        Ok(())
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        if key.code == KeyCode::Char('y') || key.code == KeyCode::Char('Y') {
            self.initialize(cwd);
            self.modal_open = true;
            return KeyHandleResult::OpenModal;
        }
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            state::DepsView::List | state::DepsView::Outdated => self.handle_list_key(key, cwd),
            state::DepsView::SearchInput => self.handle_search_input_key(key, cwd),
            state::DepsView::Search => self.handle_search_results_key(key, cwd),
            state::DepsView::Install => self.handle_install_key(key, cwd),
            state::DepsView::Output => self.handle_output_key(key, cwd),
            state::DepsView::Confirm => self.handle_confirm_key(key, cwd),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        draw_deps_modal(frame, area, &self.state, colors);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use state::PackageManager;

    #[test]
    fn test_plugin_creation() {
        let plugin = DepsPlugin::new();
        assert_eq!(plugin.id(), "deps");
        assert_eq!(plugin.name(), "Dependencies");
    }

    #[test]
    fn test_package_manager_names() {
        assert_eq!(PackageManager::Cargo.name(), "Cargo");
        assert_eq!(PackageManager::Npm.name(), "npm");
        assert_eq!(PackageManager::GoMod.name(), "Go");
    }
}
