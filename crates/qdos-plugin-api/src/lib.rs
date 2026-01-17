//! QDOS Plugin API
//!
//! This crate defines the core plugin trait and types for QDOS plugins.
//! All plugins must implement the [`Plugin`] trait and can use the types
//! defined here to integrate with the QDOS host application.
//!
// Allow &PathBuf in trait definitions for backward compatibility with existing plugins
// Future: migrate to &Path
#![allow(clippy::ptr_arg)]
//!
//! # Example
//!
//! ```ignore
//! use qdos_plugin_api::prelude::*;
//!
//! pub struct MyPlugin {
//!     // plugin state
//! }
//!
//! impl Plugin for MyPlugin {
//!     fn id(&self) -> &str { "myplugin" }
//!     fn name(&self) -> &str { "My Plugin" }
//!     // ... implement other required methods
//! }
//! ```

pub mod ui;

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;

// Re-export commonly used types from ratatui and crossterm
pub use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
pub use ratatui::layout::Rect as LayoutRect;
pub use ratatui::style::{Color, Modifier, Style};
pub use ratatui::text::{Line, Span, Text};
pub use ratatui::widgets::{Block, Borders, Paragraph};

// =============================================================================
// THEME COLORS
// =============================================================================

/// RGB color values for a theme
///
/// Plugins receive this from the host to ensure consistent styling.
#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub background: (u8, u8, u8),
    pub foreground: (u8, u8, u8),
    pub blue: (u8, u8, u8),
    pub green: (u8, u8, u8),
    pub red: (u8, u8, u8),
    pub yellow: (u8, u8, u8),
    pub grey: (u8, u8, u8),
    pub cyan: (u8, u8, u8),
    pub magenta: (u8, u8, u8),
}

impl ThemeColors {
    /// Get background color (Reset for terminal default)
    pub fn bg(&self) -> Color {
        if self.background == (0, 0, 0) {
            Color::Reset
        } else {
            let (r, g, b) = self.background;
            Color::Rgb(r, g, b)
        }
    }

    /// Get foreground color
    pub fn fg(&self) -> Color {
        let (r, g, b) = self.foreground;
        Color::Rgb(r, g, b)
    }

    /// Get blue color
    pub fn blue(&self) -> Color {
        let (r, g, b) = self.blue;
        Color::Rgb(r, g, b)
    }

    /// Get green color
    pub fn green(&self) -> Color {
        let (r, g, b) = self.green;
        Color::Rgb(r, g, b)
    }

    /// Get red color
    pub fn red(&self) -> Color {
        let (r, g, b) = self.red;
        Color::Rgb(r, g, b)
    }

    /// Get yellow color
    pub fn yellow(&self) -> Color {
        let (r, g, b) = self.yellow;
        Color::Rgb(r, g, b)
    }

    /// Get grey color
    pub fn grey(&self) -> Color {
        let (r, g, b) = self.grey;
        Color::Rgb(r, g, b)
    }

    /// Get cyan color
    pub fn cyan(&self) -> Color {
        let (r, g, b) = self.cyan;
        Color::Rgb(r, g, b)
    }

    /// Get magenta color
    pub fn magenta(&self) -> Color {
        let (r, g, b) = self.magenta;
        Color::Rgb(r, g, b)
    }

    /// Adapt colors for a light terminal background
    /// Darkens colors that would be hard to read on light backgrounds
    pub fn for_light_terminal(&self) -> ThemeColors {
        ThemeColors {
            // Use light background, dark foreground
            background: (240, 240, 240),
            foreground: (30, 30, 30),
            // Darken accent colors for visibility on light backgrounds
            blue: Self::darken(self.blue, 0.4),
            green: Self::darken(self.green, 0.5),
            red: self.red, // Red is typically already visible
            yellow: Self::darken(self.yellow, 0.3),
            grey: (100, 100, 100),
            cyan: Self::darken(self.cyan, 0.4),
            magenta: Self::darken(self.magenta, 0.3),
        }
    }

    /// Darken an RGB color by a factor (0.0 = no change, 1.0 = black)
    fn darken(color: (u8, u8, u8), factor: f32) -> (u8, u8, u8) {
        let f = 1.0 - factor.clamp(0.0, 1.0);
        (
            (color.0 as f32 * f) as u8,
            (color.1 as f32 * f) as u8,
            (color.2 as f32 * f) as u8,
        )
    }
}

impl Default for ThemeColors {
    fn default() -> Self {
        // DOS-style default colors
        Self {
            background: (0, 0, 0),
            foreground: (255, 255, 255),
            blue: (0, 0, 170),
            green: (0, 170, 0),
            red: (170, 0, 0),
            yellow: (255, 255, 85),
            grey: (170, 170, 170),
            cyan: (0, 170, 170),
            magenta: (170, 0, 170),
        }
    }
}

// =============================================================================
// PLUGIN CAPABILITIES
// =============================================================================

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

// =============================================================================
// MENU AND STATUS
// =============================================================================

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

// =============================================================================
// APP LAUNCHER
// =============================================================================

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

// =============================================================================
// KEY HANDLING
// =============================================================================

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
    /// Key was handled, navigate to a file (close modal and select file)
    NavigateToFile(PathBuf),
    /// Key was handled, navigate to a directory (close modal and enter directory)
    NavigateToDir(PathBuf),
}

// =============================================================================
// SOUND EVENTS
// =============================================================================

/// Sound events that plugins can emit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundEvent {
    /// Achievement unlocked
    Achievement,
    /// Game over
    GameOver,
    /// Level up / success
    LevelUp,
    /// Click / selection
    Click,
    /// Error
    Error,
    /// Success
    Success,
    /// Alien contact: Harmonics - melodic greeting
    AlienHarmonics,
    /// Alien contact: Geometers - mathematical pattern
    AlienGeometers,
    /// Alien contact: Empaths - emotional oscillation
    AlienEmpaths,
}

// =============================================================================
// PLUGIN TRAIT
// =============================================================================

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

    /// Handle a mouse event (when plugin modal is open)
    /// Parameters:
    /// - `column`: Mouse X position (0-based from left)
    /// - `row`: Mouse Y position (0-based from top)
    /// - `kind`: Type of mouse event (down, up, drag, scroll)
    /// - `button`: Which button (left, right, middle)
    fn handle_modal_mouse(
        &mut self,
        _column: u16,
        _row: u16,
        _kind: MouseEventKind,
        _button: MouseButton,
    ) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    /// Handle tick event for animations/auto-refresh (called every 100ms when modal is open)
    fn tick(&mut self) {}

    /// Drain pending sound events (called after tick)
    fn drain_sound_events(&mut self) -> Vec<SoundEvent> {
        Vec::new()
    }

    /// Draw the plugin's modal
    fn draw_modal(&self, _frame: &mut Frame, _area: Rect, _colors: &ThemeColors) {}

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
    /// Returns Ok(()) if the plugin was successfully launched and its modal should open.
    /// Returns Err(message) if the plugin cannot be launched (will show error).
    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        Ok(())
    }

    /// Get plugin state as Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Get mutable plugin state as Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// =============================================================================
// PLUGIN REGISTRATION
// =============================================================================

/// A plugin registration entry for self-registration.
///
/// Plugins use `inventory::submit!` to register themselves at compile time.
/// The host application collects all registrations and instantiates plugins.
///
/// # Example
///
/// ```ignore
/// use qdos_plugin_api::{PluginRegistration, inventory};
///
/// pub struct MyPlugin { /* ... */ }
/// impl Plugin for MyPlugin { /* ... */ }
///
/// // Self-registration: this runs at startup
/// inventory::submit! {
///     PluginRegistration::new("myplugin", || Box::new(MyPlugin::new()))
/// }
/// ```
pub struct PluginRegistration {
    /// Plugin ID (must match Plugin::id())
    pub id: &'static str,
    /// Factory function to create a new plugin instance
    pub create: fn() -> Box<dyn Plugin>,
}

impl PluginRegistration {
    /// Create a new plugin registration
    pub const fn new(id: &'static str, create: fn() -> Box<dyn Plugin>) -> Self {
        Self { id, create }
    }

    /// Create a plugin instance
    pub fn instantiate(&self) -> Box<dyn Plugin> {
        (self.create)()
    }
}

// Register PluginRegistration with inventory for distributed collection
inventory::collect!(PluginRegistration);

/// Collect all registered plugins from inventory.
///
/// This function gathers all plugins that were registered via `inventory::submit!`
/// and returns them as boxed trait objects.
///
/// # Example
///
/// ```ignore
/// use qdos_plugin_api::collect_plugins;
///
/// let plugins = collect_plugins();
/// for plugin in plugins {
///     println!("Found plugin: {}", plugin.id());
/// }
/// ```
pub fn collect_plugins() -> Vec<Box<dyn Plugin>> {
    inventory::iter::<PluginRegistration>
        .into_iter()
        .map(|reg| reg.instantiate())
        .collect()
}

/// Get plugin registrations without instantiating.
///
/// Useful for checking what plugins are available without creating instances.
pub fn plugin_registrations() -> impl Iterator<Item = &'static PluginRegistration> {
    inventory::iter::<PluginRegistration>.into_iter()
}

// =============================================================================
// PRELUDE
// =============================================================================

/// Convenience re-exports for plugin authors
pub mod prelude {
    pub use super::{collect_plugins, plugin_registrations};
    pub use super::{
        AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
        PluginRegistration, PluginStatusInfo, SoundEvent, ThemeColors,
    };
    pub use super::{Color, KeyCode, KeyModifiers, Modifier, Span, Style};
    // UI components
    pub use super::ui::{FullScreenView, ModalFrame, TabBar, TabState};
    pub use crossterm::event::KeyEvent;
    pub use inventory;
    pub use ratatui::layout::Rect;
    pub use ratatui::Frame;
    pub use std::any::Any;
    pub use std::path::PathBuf;
}
