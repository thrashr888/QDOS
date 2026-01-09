//! Plugin system for R-DOS
//!
//! This module provides a plugin architecture that allows extending R-DOS
//! with custom functionality. Plugins can:
//! - Add menu items
//! - Handle keyboard shortcuts
//! - Render custom modals
//! - Provide status bar content
//! - Add CLI arguments
//! - Provide help content

// Allow dead code until plugins are fully integrated
#![allow(dead_code)]

pub mod ai;
pub mod apps;
pub mod attribute;
pub mod audio;
pub mod basic;
pub mod beads;
pub mod clipboard;
pub mod cloud;
pub mod database;
pub mod dirmap;
pub mod drives;
pub mod dropbox;
pub mod fileops;
pub mod find;
pub mod gdrive;
pub mod git;
pub mod help;
pub mod homebrew;
pub mod icloud;
pub mod jj;
pub mod midi;
pub mod model3d;
pub mod print;
pub mod proc;
pub mod qdconfig;
pub mod qedit;
pub mod qmind;
pub mod searchspec;
pub mod sftp;
pub mod shell;
pub mod space;
pub mod status;
pub mod theme;
pub mod video;
pub mod viewer;

pub use ai::AIPlugin;
pub use apps::AppsPlugin;
pub use audio::AudioPlugin;
pub use basic::BasicPlugin;
pub use beads::BeadsPlugin;
pub use database::DatabasePlugin;
pub use dirmap::DirMapPlugin;
pub use drives::DrivesPlugin;
pub use dropbox::DropboxPlugin;
pub use fileops::FileOpsPlugin;
pub use gdrive::GDrivePlugin;
pub use git::GitPlugin;
pub use help::HelpPlugin;
pub use homebrew::HomebrewPlugin;
pub use icloud::ICloudPlugin;
pub use jj::JjPlugin;
pub use midi::MidiPlugin;
pub use model3d::Model3dPlugin;
pub use print::PrintPlugin;
pub use proc::ProcPlugin;
pub use qdconfig::QdconfigPlugin;
pub use qedit::QEditPlugin;
pub use qmind::QMindPlugin;
pub use searchspec::SearchSpecPlugin;
pub use sftp::SftpPlugin;
pub use shell::ShellPlugin;
pub use space::SpacePlugin;
pub use status::StatusPlugin;
pub use theme::ThemePlugin;
pub use video::VideoPlugin;
pub use viewer::ViewerPlugin;

use crate::config::PluginsConfig;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;
use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;

/// Plugin capability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PluginCapabilities {
    /// Plugin provides a menu item
    pub has_menu: bool,
    /// Plugin handles keyboard shortcuts
    pub has_keys: bool,
    /// Plugin provides a modal UI
    pub has_modal: bool,
    /// Plugin provides status bar content
    pub has_status: bool,
    /// Plugin provides CLI arguments
    pub has_cli: bool,
    /// Plugin provides help content
    pub has_help: bool,
}

/// Menu item provided by a plugin
#[derive(Debug, Clone)]
pub struct PluginMenuItem {
    /// Display name in menu
    pub name: String,
    /// Keyboard shortcut key
    pub key: char,
    /// Description shown in menu
    pub description: String,
    /// Priority for ordering (lower = earlier)
    pub priority: i32,
}

/// Status bar info provided by a plugin
#[derive(Debug, Clone, Default)]
pub struct PluginStatusInfo {
    /// Short status text (max ~20 chars)
    pub text: String,
    /// Whether plugin is active/enabled
    pub active: bool,
}

/// Plugin category for Apps launcher organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginCategory {
    Files,
    Vcs,
    Tools,
    Games,
    System,
}

impl PluginCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginCategory::Files => "Files",
            PluginCategory::Vcs => "VCS",
            PluginCategory::Tools => "Tools",
            PluginCategory::Games => "Games",
            PluginCategory::System => "System",
        }
    }

    /// Get all categories in display order
    pub fn all() -> &'static [PluginCategory] {
        &[
            PluginCategory::Files,
            PluginCategory::Vcs,
            PluginCategory::Tools,
            PluginCategory::Games,
            PluginCategory::System,
        ]
    }
}

/// App entry for the F12 Apps launcher
#[derive(Debug, Clone)]
pub struct AppEntry {
    /// Plugin ID (must match plugin.id())
    pub id: String,
    /// Display name
    pub name: String,
    /// Short description
    pub description: String,
    /// Category for grouping
    pub category: PluginCategory,
    /// Keyboard shortcut key (A-Z)
    pub key: char,
}

/// Result of handling a key event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyHandleResult {
    /// Key was not handled by this plugin
    NotHandled,
    /// Key was handled, continue processing
    Handled,
    /// Key was handled, open plugin's modal
    OpenModal,
    /// Key was handled, close current modal
    CloseModal,
    /// Key was handled, close modal and show success message
    CloseWithSuccess(String),
    /// Key was handled, close modal and show error message
    CloseWithError(String),
    /// Key was handled, request file list refresh
    RefreshFiles,
}

/// The core Plugin trait that all plugins must implement
pub trait Plugin: Send + Sync {
    /// Unique identifier for this plugin
    fn id(&self) -> &str;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Plugin capabilities
    fn capabilities(&self) -> PluginCapabilities;

    /// Check if plugin is available in current directory
    fn is_available(&self, cwd: &PathBuf) -> bool;

    /// Get menu item if plugin provides one
    fn menu_item(&self) -> Option<PluginMenuItem> {
        None
    }

    /// Get status bar info
    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    /// Handle a key event (when plugin modal is not open)
    fn handle_global_key(
        &mut self,
        _key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    /// Handle a key event (when plugin modal is open)
    fn handle_modal_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    /// Handle tick event for animations/auto-refresh (called every 100ms when modal is open)
    fn tick(&mut self) {}

    /// Draw the plugin's modal
    fn draw_modal(&self, _frame: &mut Frame, _area: Rect, _colors: &crate::app::ThemeColors) {}

    /// Get help content lines
    fn help_content(&self) -> Vec<String> {
        vec![]
    }

    /// Get app entry for F12 Apps launcher.
    /// Return Some(AppEntry) if this plugin should appear in the Apps launcher.
    /// The `id` should match the plugin's `id()` method.
    fn app_entry(&self) -> Option<AppEntry> {
        None
    }

    /// Launch the plugin from the Apps launcher (F12).
    /// Called when user selects this plugin from the Apps menu.
    /// Plugins should initialize their state here.
    ///
    /// Parameters:
    /// - `cwd`: Current working directory
    /// - `selected_file`: Currently selected file path (if any)
    ///
    /// Returns true if the plugin was successfully launched and its modal should open.
    /// Returns false if the plugin cannot be launched (will show error).
    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        Ok(())
    }

    /// Get plugin state as Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Get mutable plugin state as Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Plugin manager that handles plugin registration and lifecycle
pub struct PluginManager {
    /// Registered plugins by ID
    plugins: HashMap<String, Box<dyn Plugin>>,
    /// Order of plugins for menu display
    plugin_order: Vec<String>,
    /// Currently active plugin modal (if any)
    active_modal: Option<String>,
    /// Plugin configuration
    config: PluginsConfig,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_order: Vec::new(),
            active_modal: None,
            config: PluginsConfig::default(),
        }
    }

    /// Create a new plugin manager with config
    pub fn with_config(config: PluginsConfig) -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_order: Vec::new(),
            active_modal: None,
            config,
        }
    }

    /// Update the plugin configuration
    pub fn set_config(&mut self, config: PluginsConfig) {
        self.config = config;
    }

    /// Get the current plugin configuration
    pub fn config(&self) -> &PluginsConfig {
        &self.config
    }

    /// Check if a plugin is enabled by config
    pub fn is_plugin_enabled(&self, id: &str) -> bool {
        self.config.is_plugin_enabled(id)
    }

    /// Register a plugin (respects config enabled status)
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let id = plugin.id().to_string();
        // Only register if enabled in config
        if self.config.is_plugin_enabled(&id) {
            self.plugin_order.push(id.clone());
            self.plugins.insert(id, plugin);
        }
    }

    /// Register a plugin unconditionally (ignores config)
    pub fn register_always(&mut self, plugin: Box<dyn Plugin>) {
        let id = plugin.id().to_string();
        self.plugin_order.push(id.clone());
        self.plugins.insert(id, plugin);
    }

    /// Get all registered plugins
    pub fn plugins(&self) -> impl Iterator<Item = &dyn Plugin> {
        self.plugin_order
            .iter()
            .filter_map(|id| self.plugins.get(id).map(|p| p.as_ref()))
    }

    /// Get a plugin by ID
    pub fn get(&self, id: &str) -> Option<&dyn Plugin> {
        self.plugins.get(id).map(|p| p.as_ref())
    }

    /// Get a mutable plugin by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Box<dyn Plugin>> {
        self.plugins.get_mut(id)
    }

    /// Get plugins that provide menu items
    pub fn menu_plugins(&self) -> Vec<(&dyn Plugin, PluginMenuItem)> {
        let mut items: Vec<_> = self
            .plugins()
            .filter(|p| p.capabilities().has_menu)
            .filter_map(|p| p.menu_item().map(|m| (p, m)))
            .collect();
        items.sort_by_key(|(_, m)| m.priority);
        items
    }

    /// Get plugins that provide status bar info
    pub fn status_plugins(&self, cwd: &PathBuf) -> Vec<(&dyn Plugin, PluginStatusInfo)> {
        self.plugins()
            .filter(|p| p.capabilities().has_status && p.is_available(cwd))
            .filter_map(|p| p.status_info(cwd).map(|s| (p, s)))
            .collect()
    }

    /// Collect app entries from all registered plugins for the Apps launcher
    pub fn collect_app_entries(&self, _cwd: &PathBuf) -> Vec<apps::state::AppEntry> {
        self.plugins()
            .filter_map(|p| {
                p.app_entry().map(|entry| {
                    let enabled = self.config.is_plugin_enabled(&entry.id);
                    apps::state::AppEntry {
                        id: entry.id,
                        name: entry.name,
                        description: entry.description,
                        category: match entry.category {
                            PluginCategory::Files => apps::state::PluginCategory::Files,
                            PluginCategory::Vcs => apps::state::PluginCategory::Vcs,
                            PluginCategory::Tools => apps::state::PluginCategory::Tools,
                            PluginCategory::Games => apps::state::PluginCategory::Games,
                            PluginCategory::System => apps::state::PluginCategory::System,
                        },
                        key: entry.key,
                        // Apps launcher shows all plugins as available - runtime availability
                        // is checked when the plugin is actually launched
                        available: true,
                        enabled,
                    }
                })
            })
            .collect()
    }

    /// Handle a global key event
    pub fn handle_global_key(
        &mut self,
        key: KeyEvent,
        cwd: &PathBuf,
        selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        for plugin in self.plugins.values_mut() {
            let result = plugin.handle_global_key(key, cwd, selected_file);
            if result != KeyHandleResult::NotHandled {
                if result == KeyHandleResult::OpenModal {
                    self.active_modal = Some(plugin.id().to_string());
                }
                return result;
            }
        }
        KeyHandleResult::NotHandled
    }

    /// Handle a modal key event
    pub fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        if let Some(ref id) = self.active_modal.clone() {
            if let Some(plugin) = self.plugins.get_mut(id) {
                let result = plugin.handle_modal_key(key, cwd);
                if result == KeyHandleResult::CloseModal {
                    self.active_modal = None;
                }
                return result;
            }
        }
        KeyHandleResult::NotHandled
    }

    /// Get the currently active modal plugin
    pub fn active_modal(&self) -> Option<&dyn Plugin> {
        self.active_modal
            .as_ref()
            .and_then(|id| self.plugins.get(id).map(|p| p.as_ref()))
    }

    /// Set the active modal by plugin ID
    pub fn set_active_modal(&mut self, id: Option<&str>) {
        self.active_modal = id.map(|s| s.to_string());
    }

    /// Check if a modal is currently open
    pub fn has_active_modal(&self) -> bool {
        self.active_modal.is_some()
    }

    /// Draw the active modal
    pub fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        if let Some(plugin) = self.active_modal() {
            plugin.draw_modal(frame, area, colors);
        }
    }

    /// Call tick on active modal plugin (for auto-refresh etc.)
    pub fn tick_active_modal(&mut self) {
        if let Some(ref id) = self.active_modal.clone() {
            if let Some(plugin) = self.plugins.get_mut(id) {
                plugin.tick();
            }
        }
    }

    /// Launch a plugin by ID from the Apps launcher.
    /// Calls the plugin's launch() method and sets it as the active modal.
    /// Returns Ok(plugin_id) on success, Err(message) on failure.
    pub fn launch_plugin(
        &mut self,
        plugin_id: &str,
        cwd: &PathBuf,
        selected_file: Option<&PathBuf>,
    ) -> Result<String, String> {
        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            plugin.launch(cwd, selected_file)?;
            self.active_modal = Some(plugin_id.to_string());
            Ok(plugin_id.to_string())
        } else {
            Err(format!("Unknown plugin: {}", plugin_id))
        }
    }

    /// Get mutable reference to AIPlugin
    pub fn ai_plugin_mut(&mut self) -> Option<&mut ai::AIPlugin> {
        self.plugins
            .get_mut("ai")
            .and_then(|p| p.as_any_mut().downcast_mut::<ai::AIPlugin>())
    }

    /// Get mutable reference to BeadsPlugin for key handling delegation
    pub fn beads_plugin_mut(&mut self) -> Option<&mut beads::BeadsPlugin> {
        self.plugins
            .get_mut("beads")
            .and_then(|p| p.as_any_mut().downcast_mut::<beads::BeadsPlugin>())
    }

    /// Get mutable reference to GitPlugin for key handling delegation
    pub fn git_plugin_mut(&mut self) -> Option<&mut git::GitPlugin> {
        self.plugins
            .get_mut("git")
            .and_then(|p| p.as_any_mut().downcast_mut::<git::GitPlugin>())
    }

    /// Get mutable reference to StatusPlugin
    pub fn status_plugin_mut(&mut self) -> Option<&mut status::StatusPlugin> {
        self.plugins
            .get_mut("status")
            .and_then(|p| p.as_any_mut().downcast_mut::<status::StatusPlugin>())
    }

    /// Get mutable reference to ThemePlugin
    pub fn theme_plugin_mut(&mut self) -> Option<&mut theme::ThemePlugin> {
        self.plugins
            .get_mut("theme")
            .and_then(|p| p.as_any_mut().downcast_mut::<theme::ThemePlugin>())
    }

    /// Get mutable reference to SpacePlugin
    pub fn space_plugin_mut(&mut self) -> Option<&mut space::SpacePlugin> {
        self.plugins
            .get_mut("space")
            .and_then(|p| p.as_any_mut().downcast_mut::<space::SpacePlugin>())
    }

    /// Get mutable reference to ProcPlugin
    pub fn proc_plugin_mut(&mut self) -> Option<&mut proc::ProcPlugin> {
        self.plugins
            .get_mut("proc")
            .and_then(|p| p.as_any_mut().downcast_mut::<proc::ProcPlugin>())
    }

    /// Collect help content from all plugins with has_help capability
    /// Returns (plugin_id, plugin_name, help_lines) for each plugin
    pub fn collect_plugin_help(&self) -> Vec<(String, String, Vec<String>)> {
        self.plugins()
            .filter(|p| p.capabilities().has_help)
            .map(|p| (p.id().to_string(), p.name().to_string(), p.help_content()))
            .filter(|(_, _, content)| !content.is_empty())
            .collect()
    }

    /// Get mutable reference to HelpPlugin
    pub fn help_plugin_mut(&mut self) -> Option<&mut help::HelpPlugin> {
        self.plugins
            .get_mut("help")
            .and_then(|p| p.as_any_mut().downcast_mut::<help::HelpPlugin>())
    }

    /// Get mutable reference to PrintPlugin
    pub fn print_plugin_mut(&mut self) -> Option<&mut print::PrintPlugin> {
        self.plugins
            .get_mut("print")
            .and_then(|p| p.as_any_mut().downcast_mut::<print::PrintPlugin>())
    }

    /// Get mutable reference to DirMapPlugin
    pub fn dirmap_plugin_mut(&mut self) -> Option<&mut dirmap::DirMapPlugin> {
        self.plugins
            .get_mut("dirmap")
            .and_then(|p| p.as_any_mut().downcast_mut::<dirmap::DirMapPlugin>())
    }

    /// Get mutable reference to SearchSpecPlugin
    pub fn searchspec_plugin_mut(&mut self) -> Option<&mut searchspec::SearchSpecPlugin> {
        self.plugins.get_mut("searchspec").and_then(|p| {
            p.as_any_mut()
                .downcast_mut::<searchspec::SearchSpecPlugin>()
        })
    }

    /// Get mutable reference to QdconfigPlugin
    pub fn qdconfig_plugin_mut(&mut self) -> Option<&mut qdconfig::QdconfigPlugin> {
        self.plugins
            .get_mut("qdconfig")
            .and_then(|p| p.as_any_mut().downcast_mut::<qdconfig::QdconfigPlugin>())
    }

    /// Get mutable reference to FileOpsPlugin
    pub fn fileops_plugin_mut(&mut self) -> Option<&mut fileops::FileOpsPlugin> {
        self.plugins
            .get_mut("fileops")
            .and_then(|p| p.as_any_mut().downcast_mut::<fileops::FileOpsPlugin>())
    }

    /// Get mutable reference to ViewerPlugin
    pub fn viewer_plugin_mut(&mut self) -> Option<&mut viewer::ViewerPlugin> {
        self.plugins
            .get_mut("viewer")
            .and_then(|p| p.as_any_mut().downcast_mut::<viewer::ViewerPlugin>())
    }

    /// Get reference to ViewerPlugin
    pub fn viewer_plugin(&self) -> Option<&viewer::ViewerPlugin> {
        self.plugins
            .get("viewer")
            .and_then(|p| p.as_any().downcast_ref::<viewer::ViewerPlugin>())
    }

    /// Get mutable reference to QEditPlugin
    pub fn qedit_plugin_mut(&mut self) -> Option<&mut qedit::QEditPlugin> {
        self.plugins
            .get_mut("qedit")
            .and_then(|p| p.as_any_mut().downcast_mut::<qedit::QEditPlugin>())
    }

    /// Get mutable reference to JjPlugin
    pub fn jj_plugin_mut(&mut self) -> Option<&mut jj::JjPlugin> {
        self.plugins
            .get_mut("jj")
            .and_then(|p| p.as_any_mut().downcast_mut::<jj::JjPlugin>())
    }

    /// Get mutable reference to AppsPlugin
    pub fn apps_plugin_mut(&mut self) -> Option<&mut apps::AppsPlugin> {
        self.plugins
            .get_mut("apps")
            .and_then(|p| p.as_any_mut().downcast_mut::<apps::AppsPlugin>())
    }

    /// Get mutable reference to DrivesPlugin
    pub fn drives_plugin_mut(&mut self) -> Option<&mut drives::DrivesPlugin> {
        self.plugins
            .get_mut("drives")
            .and_then(|p| p.as_any_mut().downcast_mut::<drives::DrivesPlugin>())
    }

    /// Get mutable reference to HomebrewPlugin
    pub fn homebrew_plugin_mut(&mut self) -> Option<&mut homebrew::HomebrewPlugin> {
        self.plugins
            .get_mut("homebrew")
            .and_then(|p| p.as_any_mut().downcast_mut::<homebrew::HomebrewPlugin>())
    }

    /// Get mutable reference to BasicPlugin
    pub fn basic_plugin_mut(&mut self) -> Option<&mut basic::BasicPlugin> {
        self.plugins
            .get_mut("basic")
            .and_then(|p| p.as_any_mut().downcast_mut::<basic::BasicPlugin>())
    }

    /// Get mutable reference to MidiPlugin
    pub fn midi_plugin_mut(&mut self) -> Option<&mut midi::MidiPlugin> {
        self.plugins
            .get_mut("midi")
            .and_then(|p| p.as_any_mut().downcast_mut::<midi::MidiPlugin>())
    }

    /// Get mutable reference to VideoPlugin
    pub fn video_plugin_mut(&mut self) -> Option<&mut video::VideoPlugin> {
        self.plugins
            .get_mut("video")
            .and_then(|p| p.as_any_mut().downcast_mut::<video::VideoPlugin>())
    }

    /// Get mutable reference to AudioPlugin
    pub fn audio_plugin_mut(&mut self) -> Option<&mut audio::AudioPlugin> {
        self.plugins
            .get_mut("audio")
            .and_then(|p| p.as_any_mut().downcast_mut::<audio::AudioPlugin>())
    }

    /// Get mutable reference to Model3dPlugin
    pub fn model3d_plugin_mut(&mut self) -> Option<&mut model3d::Model3dPlugin> {
        self.plugins
            .get_mut("model3d")
            .and_then(|p| p.as_any_mut().downcast_mut::<model3d::Model3dPlugin>())
    }

    /// Get mutable reference to ShellPlugin
    pub fn shell_plugin_mut(&mut self) -> Option<&mut shell::ShellPlugin> {
        self.plugins
            .get_mut("shell")
            .and_then(|p| p.as_any_mut().downcast_mut::<shell::ShellPlugin>())
    }

    /// Get mutable reference to DatabasePlugin
    pub fn database_plugin_mut(&mut self) -> Option<&mut database::DatabasePlugin> {
        self.plugins
            .get_mut("database")
            .and_then(|p| p.as_any_mut().downcast_mut::<database::DatabasePlugin>())
    }

    /// Get mutable reference to DropboxPlugin
    pub fn dropbox_plugin_mut(&mut self) -> Option<&mut dropbox::DropboxPlugin> {
        self.plugins
            .get_mut("dropbox")
            .and_then(|p| p.as_any_mut().downcast_mut::<dropbox::DropboxPlugin>())
    }

    /// Get mutable reference to ICloudPlugin
    pub fn icloud_plugin_mut(&mut self) -> Option<&mut icloud::ICloudPlugin> {
        self.plugins
            .get_mut("icloud")
            .and_then(|p| p.as_any_mut().downcast_mut::<icloud::ICloudPlugin>())
    }

    /// Get mutable reference to GDrivePlugin
    pub fn gdrive_plugin_mut(&mut self) -> Option<&mut gdrive::GDrivePlugin> {
        self.plugins
            .get_mut("gdrive")
            .and_then(|p| p.as_any_mut().downcast_mut::<gdrive::GDrivePlugin>())
    }

    /// Get mutable reference to SftpPlugin
    pub fn sftp_plugin_mut(&mut self) -> Option<&mut sftp::SftpPlugin> {
        self.plugins
            .get_mut("sftp")
            .and_then(|p| p.as_any_mut().downcast_mut::<sftp::SftpPlugin>())
    }

    /// Get mutable reference to QMindPlugin
    pub fn qmind_plugin_mut(&mut self) -> Option<&mut qmind::QMindPlugin> {
        self.plugins
            .get_mut("qmind")
            .and_then(|p| p.as_any_mut().downcast_mut::<qmind::QMindPlugin>())
    }

    /// Get list of registered plugins with their info (id, name, description)
    pub fn plugin_list(&self) -> Vec<(String, String, String)> {
        self.plugins()
            .map(|p| {
                let menu_item = p.menu_item();
                let description = menu_item
                    .as_ref()
                    .map(|m| m.description.clone())
                    .unwrap_or_else(|| "No description".to_string());
                (p.id().to_string(), p.name().to_string(), description)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
