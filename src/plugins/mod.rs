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
pub mod git;

pub use beads::BeadsPlugin;
pub use git::GitPlugin;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        id: String,
        initialized: bool,
    }

    impl TestPlugin {
        fn new(id: &str) -> Self {
            Self {
                id: id.to_string(),
                initialized: false,
            }
        }
    }

    impl Plugin for TestPlugin {
        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            "Test Plugin"
        }

        fn capabilities(&self) -> PluginCapabilities {
            PluginCapabilities {
                has_menu: true,
                ..Default::default()
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
            true
        }

        fn menu_item(&self) -> Option<PluginMenuItem> {
            Some(PluginMenuItem {
                name: "Test".to_string(),
                key: 'T',
                description: "Test plugin".to_string(),
                priority: 100,
            })
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_plugin_manager() {
        let mut manager = PluginManager::new();
        manager.register(Box::new(TestPlugin::new("test")));

        assert!(manager.get("test").is_some());
        assert!(manager.get("nonexistent").is_none());

        let menu_items = manager.menu_plugins();
        assert_eq!(menu_items.len(), 1);
        assert_eq!(menu_items[0].1.name, "Test");
    }

    #[test]
    fn test_plugin_manager_with_config() {
        // Create a config that disables the "test" plugin
        let config = PluginsConfig {
            enabled: vec![],
            disabled: vec!["test".to_string()],
            settings: HashMap::new(),
        };

        let mut manager = PluginManager::with_config(config);
        manager.register(Box::new(TestPlugin::new("test")));

        // Plugin should not be registered because it's disabled
        assert!(manager.get("test").is_none());
    }

    #[test]
    fn test_plugin_manager_register_always() {
        // Create a config that disables the "test" plugin
        let config = PluginsConfig {
            enabled: vec![],
            disabled: vec!["test".to_string()],
            settings: HashMap::new(),
        };

        let mut manager = PluginManager::with_config(config);
        // register_always ignores config
        manager.register_always(Box::new(TestPlugin::new("test")));

        // Plugin should be registered because we used register_always
        assert!(manager.get("test").is_some());
    }
}
