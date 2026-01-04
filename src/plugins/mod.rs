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

pub mod beads;
pub mod dirmap;
pub mod git;
pub mod help;
pub mod print;
pub mod qdconfig;
pub mod searchspec;
pub mod space;
pub mod status;
pub mod theme;

pub use beads::BeadsPlugin;
pub use dirmap::DirMapPlugin;
pub use git::GitPlugin;
pub use help::HelpPlugin;
pub use print::PrintPlugin;
pub use qdconfig::QdconfigPlugin;
pub use searchspec::SearchSpecPlugin;
pub use space::SpacePlugin;
pub use status::StatusPlugin;
pub use theme::ThemePlugin;

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

    /// Initialize the plugin
    fn init(&mut self, cwd: &PathBuf) -> Result<(), String>;

    /// Shutdown the plugin
    fn shutdown(&mut self) -> Result<(), String>;

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
    fn handle_global_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    /// Handle a key event (when plugin modal is open)
    fn handle_modal_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    /// Draw the plugin's modal
    fn draw_modal(&self, _frame: &mut Frame, _area: Rect) {}

    /// Get help content lines
    fn help_content(&self) -> Vec<String> {
        vec![]
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

    /// Initialize all enabled plugins
    pub fn init_all(&mut self, cwd: &PathBuf) -> Result<(), String> {
        for plugin in self.plugins.values_mut() {
            plugin.init(cwd)?;
        }
        Ok(())
    }

    /// Shutdown all plugins
    pub fn shutdown_all(&mut self) -> Result<(), String> {
        for plugin in self.plugins.values_mut() {
            plugin.shutdown()?;
        }
        Ok(())
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

    /// Handle a global key event
    pub fn handle_global_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        for plugin in self.plugins.values_mut() {
            let result = plugin.handle_global_key(key, cwd);
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
    pub fn draw_modal(&self, frame: &mut Frame, area: Rect) {
        if let Some(plugin) = self.active_modal() {
            plugin.draw_modal(frame, area);
        }
    }

    /// Get mutable reference to BeadsPlugin for key handling delegation
    pub fn beads_plugin_mut(&mut self) -> Option<&mut beads::BeadsPlugin> {
        self.plugins
            .get_mut("beads")
            .and_then(|p| p.as_any_mut().downcast_mut::<beads::BeadsPlugin>())
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
        self.plugins
            .get_mut("searchspec")
            .and_then(|p| p.as_any_mut().downcast_mut::<searchspec::SearchSpecPlugin>())
    }

    /// Get mutable reference to QdconfigPlugin
    pub fn qdconfig_plugin_mut(&mut self) -> Option<&mut qdconfig::QdconfigPlugin> {
        self.plugins
            .get_mut("qdconfig")
            .and_then(|p| p.as_any_mut().downcast_mut::<qdconfig::QdconfigPlugin>())
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
