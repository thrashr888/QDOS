# R-DOS Plugin Specification

A comprehensive guide for developing plugins that extend R-DOS functionality while maintaining consistency with Q-DOS II aesthetics and modern Rust best practices.

---

## Table of Contents

1. [Design Principles](#1-design-principles)
2. [Plugin Architecture](#2-plugin-architecture)
3. [File Structure](#3-file-structure)
4. [The Plugin Trait](#4-the-plugin-trait)
5. [UI/UX Guidelines](#5-uiux-guidelines)
6. [Key Handling](#6-key-handling)
7. [Modal Rendering](#7-modal-rendering)
8. [Status Bar Integration](#8-status-bar-integration)
9. [Help System Integration](#9-help-system-integration)
10. [Menu Integration](#10-menu-integration)
11. [Lifecycle Hooks](#11-lifecycle-hooks)
12. [Background Jobs](#12-background-jobs)
13. [Testing](#13-testing)
14. [Configuration](#14-configuration)
15. [Future: Dynamic Loading](#15-future-dynamic-loading)

---

## 1. Design Principles

### 1.1 Q-DOS II Authenticity

Plugins MUST adhere to the Q-DOS II visual language and interaction patterns:

- **Double-line borders** (╔═╗║╚╝╠╣) for all modal dialogs
- **80x25 terminal design** - layouts should work at minimum dimensions
- **DOS color palette** - use theme colors, not arbitrary colors
- **Function key shortcuts** - prefer F1-F12 for primary actions
- **Menu-driven navigation** - single letter shortcuts, clear hierarchy
- **Confirmation dialogs** - all destructive actions require confirmation
- **Progress indicators** - show operation status with `====>` style arrows

### 1.2 Self-Containment

Plugins MUST be self-contained within their plugin directory:

```
src/plugins/myplugin/
├── mod.rs      # Plugin struct, trait impl, public API
├── state.rs    # State types, data structures
├── ops.rs      # Operations, CLI calls, business logic
├── modal.rs    # Modal rendering (optional, can be in mod.rs)
└── tests.rs    # Unit tests
```

**Rules:**
- **ZERO** modifications to `src/app/mod.rs` for plugin logic
- **MINIMAL** additions to `src/plugins/mod.rs` (registration only)
- **NO** plugin-specific code in `src/ui/` (use shared components)
- All state, operations, and UI rendering contained in plugin directory
- Future: plugins will be dynamically loaded from separate repositories

### 1.3 Modern Rust Patterns

- Use `Result<T, E>` for fallible operations
- Prefer `Option<T>` over sentinel values
- Implement `Default` for state types
- Use `#[derive(...)]` for common traits
- Document public APIs with `///` doc comments
- Handle all `clippy` warnings

---

## 2. Plugin Architecture

### 2.1 Component Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         PluginManager                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │  Plugin A   │  │  Plugin B   │  │  Plugin C   │   ...        │
│  │  (Git)      │  │  (Beads)    │  │  (Q-EDIT)   │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
└─────────────────────────────────────────────────────────────────┘
         │                  │                  │
         ▼                  ▼                  ▼
    ┌─────────┐        ┌─────────┐        ┌─────────┐
    │ State   │        │ State   │        │ State   │
    │ Ops     │        │ Ops     │        │ Ops     │
    │ Modal   │        │ Modal   │        │ Modal   │
    └─────────┘        └─────────┘        └─────────┘
```

### 2.2 Data Flow

```
User Input (KeyEvent)
        │
        ▼
    App::handle_key()
        │
        ├──► handle_plugin_key() ──► PluginManager
        │                                  │
        │                    ┌─────────────┴─────────────┐
        │                    ▼                           ▼
        │           handle_global_key()         handle_modal_key()
        │           (modal NOT open)            (modal IS open)
        │                    │                           │
        │                    ▼                           ▼
        │           KeyHandleResult              KeyHandleResult
        │                    │                           │
        └────────────────────┴───────────────────────────┘
                             │
                             ▼
                    App state update
```

---

## 3. File Structure

### 3.1 Required Files

Every plugin MUST have at minimum:

```rust
// src/plugins/myplugin/mod.rs
pub struct MyPlugin { ... }
impl Plugin for MyPlugin { ... }
```

### 3.2 Recommended Structure

For non-trivial plugins:

```
src/plugins/myplugin/
├── mod.rs       # Plugin struct, Plugin trait, key handlers
├── state.rs     # All state types and enums
├── ops.rs       # Business logic, external command calls
├── modal.rs     # Modal rendering functions (if complex)
└── tests.rs     # Unit tests
```

### 3.3 Module Organization

**mod.rs** - The plugin's public interface:
```rust
//! MyPlugin - Brief description
//!
//! Extended description of what this plugin does.

mod state;
mod ops;

pub use state::{MyState, MyView};

use crate::plugins::{
    Plugin, PluginCapabilities, PluginMenuItem,
    PluginStatusInfo, KeyHandleResult,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;

pub struct MyPlugin {
    initialized: bool,
    pub modal_state: Option<MyState>,
}

impl MyPlugin {
    pub fn new() -> Self { ... }
    pub fn open_modal(&mut self, cwd: &PathBuf) { ... }

    // Private helpers
    fn render_list_view(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) { ... }
}

impl Default for MyPlugin { ... }

impl Plugin for MyPlugin { ... }
```

**state.rs** - State types:
```rust
//! State types for MyPlugin

/// The main state container
#[derive(Debug, Clone, Default)]
pub struct MyState {
    pub view: MyView,
    pub selected_index: usize,
    pub items: Vec<MyItem>,
    pub error: Option<String>,
}

/// View/screen variants within the plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MyView {
    #[default]
    List,
    Detail,
    Create,
    Edit,
}

/// Data item type
#[derive(Debug, Clone)]
pub struct MyItem {
    pub id: String,
    pub name: String,
    // ...
}

impl MyState {
    pub fn new() -> Self { Self::default() }

    // State manipulation methods
    pub fn select_next(&mut self) { ... }
    pub fn select_prev(&mut self) { ... }
}
```

**ops.rs** - Operations and business logic:
```rust
//! Operations for MyPlugin

use super::state::{MyState, MyItem};
use std::path::Path;
use std::process::Command;

/// Load items from external source
pub fn load_items(state: &mut MyState, cwd: &Path) {
    let output = Command::new("mytool")
        .args(["list", "--json"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            // Parse and populate state.items
        }
        Ok(o) => {
            state.error = Some(String::from_utf8_lossy(&o.stderr).to_string());
        }
        Err(e) => {
            state.error = Some(format!("Failed to run mytool: {}", e));
        }
    }
}

/// Execute an action
pub fn do_action(item: &MyItem, cwd: &Path) -> Result<String, String> {
    // ...
}
```

---

## 4. The Plugin Trait

### 4.1 Required Methods

```rust
pub trait Plugin: Send + Sync {
    /// Unique identifier (lowercase, no spaces)
    fn id(&self) -> &str;

    /// Human-readable display name
    fn name(&self) -> &str;

    /// Declare plugin capabilities
    fn capabilities(&self) -> PluginCapabilities;

    /// Initialize when app starts
    fn init(&mut self, cwd: &PathBuf) -> Result<(), String>;

    /// Cleanup when app exits
    fn shutdown(&mut self) -> Result<(), String>;

    /// Check if plugin is relevant for current directory
    fn is_available(&self, cwd: &PathBuf) -> bool;

    /// Required for downcasting
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

### 4.2 Optional Methods

```rust
pub trait Plugin: Send + Sync {
    // ... required methods ...

    /// Menu item (if has_menu capability)
    fn menu_item(&self) -> Option<PluginMenuItem> { None }

    /// Status bar info (if has_status capability)
    fn status_info(&self, cwd: &PathBuf) -> Option<PluginStatusInfo> { None }

    /// Handle keys when modal is NOT open
    fn handle_global_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    /// Handle keys when modal IS open
    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    /// Called every ~100ms when modal is open (for animations/refresh)
    fn tick(&mut self) {}

    /// Render the plugin's modal
    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {}

    /// Help content lines
    fn help_content(&self) -> Vec<String> { vec![] }
}
```

### 4.3 Capabilities

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct PluginCapabilities {
    pub has_menu: bool,    // Provides menu item
    pub has_keys: bool,    // Handles keyboard shortcuts
    pub has_modal: bool,   // Has modal UI
    pub has_status: bool,  // Provides status bar content
    pub has_cli: bool,     // Provides CLI arguments (future)
    pub has_help: bool,    // Provides help content
}
```

### 4.4 KeyHandleResult

```rust
pub enum KeyHandleResult {
    NotHandled,              // Pass to next handler
    Handled,                 // Consumed, no further action
    OpenModal,               // Open this plugin's modal
    CloseModal,              // Close current modal
    CloseWithSuccess(String),// Close + show success message
    CloseWithError(String),  // Close + show error message
    RefreshFiles,            // Request file list refresh
}
```

---

## 5. UI/UX Guidelines

### 5.1 Color Palette

Use theme colors from `ThemeColors`, NEVER hardcoded RGB values:

| Color Method | Use Case |
|--------------|----------|
| `colors.bg()` | Background (transparent/black) |
| `colors.fg()` | Primary text, borders |
| `colors.blue()` | Headers, menu items, directories |
| `colors.green()` | Help text, key hints, descriptions |
| `colors.red()` | Selection background, errors |
| `colors.yellow()` | Selected text, tagged items, titles |
| `colors.grey()` | Disabled items, hidden files |
| `colors.cyan()` | Added/new items (git) |

### 5.2 Box Drawing Characters

**Double-line borders** for modals (REQUIRED):
```
╔═══════════════════════════════════════╗  ← Top
║        Modal Title                    ║  ← Title
╠═══════════════════════════════════════╣  ← Separator
║  Content goes here                    ║  ← Content
╠═══════════════════════════════════════╣  ← Footer sep
║  F1 Help  Enter confirm  Esc cancel   ║  ← Help row
╚═══════════════════════════════════════╝  ← Bottom
```

**Single-line borders** for internal separators only:
```
─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼
```

### 5.3 Modal Layout Patterns

**Full-screen modal** (most plugins):
```rust
fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    // Use full terminal area
    let modal_area = Rect::new(0, 0, area.width, area.height);

    // Use ModalFrame component
    let modal = ModalFrame::themed(modal_area, " MY PLUGIN ", colors);
    modal.render_frame(frame);

    // Render content
    modal.render_row(frame, 0, vec![Span::raw("Content...")]);

    // Render help
    modal.render_help(frame, vec![
        ("Enter", "confirm"),
        ("Esc", "cancel"),
    ]);
}
```

**Centered dialog** (confirmations, inputs):
```rust
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let dialog = centered_rect(50, 10, area);
    let modal = ModalFrame::themed(dialog, " CONFIRM ", colors);
    modal.render_frame(frame);
    // ...
}
```

### 5.4 Standard Screen Structure

Follow the Q-DOS II screen layout:

```
Row 0:    Title bar (plugin name)
Row 1:    Separator ═══════════════════════════════════════════
Row 2:    Context info (path, branch, etc.)
Row 3:    Blank or secondary info
Row 4-N:  Main content area (scrollable list, form, etc.)
Row N-2:  Separator ═══════════════════════════════════════════
Row N-1:  Help row (keybindings)
Row N:    Bottom border
```

### 5.5 Selection Highlighting

**Selected items**: Yellow text on red background
```rust
let style = if is_selected {
    Style::default().fg(colors.yellow()).bg(colors.red())
} else {
    Style::default().fg(colors.fg()).bg(colors.bg())
};
```

### 5.6 Progress Indicators

Use Q-DOS II style progress arrows:
```
Copying FILE.TXT ====> /destination/path
Processing item 3 of 10 [████████░░░░░░░░] 30%
```

### 5.7 Error Messages

Display errors in the Q-DOS II style:
```
╔═══════════════════════════════════════╗
║            *** ERROR ***              ║
╠═══════════════════════════════════════╣
║                                       ║
║  Unable to open file: permission      ║
║  denied                               ║
║                                       ║
╠═══════════════════════════════════════╣
║         Press any key to continue     ║
╚═══════════════════════════════════════╝
```

---

## 6. Key Handling

### 6.1 Key Binding Conventions

| Key Type | Examples | Use Case |
|----------|----------|----------|
| Function keys | F1-F12 | Primary plugin actions |
| Letters | G, B, S | Menu shortcuts (first letter) |
| Ctrl+Letter | Ctrl+T, Ctrl+S | Global shortcuts |
| Alt+Key | Alt-F9 | Alternate actions |
| Navigation | ↑↓←→, PgUp/Dn | List navigation |
| Actions | Enter, Space, Esc | Confirm, toggle, cancel |

### 6.2 Global Key Handler

Called when NO modal is open:

```rust
fn handle_global_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
    match key.code {
        // Single key to open modal
        KeyCode::Char('m') | KeyCode::Char('M') => {
            self.open_modal(cwd);
            KeyHandleResult::OpenModal
        }
        // Ctrl+key combination
        KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            self.open_modal(cwd);
            KeyHandleResult::OpenModal
        }
        // Function key
        KeyCode::F(7) => {
            self.open_modal(cwd);
            KeyHandleResult::OpenModal
        }
        _ => KeyHandleResult::NotHandled,
    }
}
```

### 6.3 Modal Key Handler

Called when THIS plugin's modal is open:

```rust
fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
    let state = match &mut self.modal_state {
        Some(s) => s,
        None => return KeyHandleResult::CloseModal,
    };

    match key.code {
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            state.select_prev();
            KeyHandleResult::Handled
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.select_next();
            KeyHandleResult::Handled
        }

        // Actions
        KeyCode::Enter => {
            match self.execute_action(cwd) {
                Ok(msg) => KeyHandleResult::CloseWithSuccess(msg),
                Err(e) => KeyHandleResult::CloseWithError(e),
            }
        }

        // View switching
        KeyCode::Tab => {
            state.cycle_view();
            KeyHandleResult::Handled
        }

        // Close
        KeyCode::Esc => {
            self.modal_state = None;
            KeyHandleResult::CloseModal
        }

        _ => KeyHandleResult::NotHandled,
    }
}
```

### 6.4 Avoiding Key Conflicts

1. Check existing plugins for conflicts before choosing keys
2. Prefer function keys (F1-F12) for unique plugin actions
3. Use Ctrl+Letter for global shortcuts sparingly
4. Document all keybindings in help content
5. When using letters, check if CONTROL modifier is pressed:

```rust
// WRONG - catches both 't' and Ctrl+T
KeyCode::Char('t') | KeyCode::Char('T') => { ... }

// RIGHT - only plain 't', not Ctrl+T
KeyCode::Char('t') | KeyCode::Char('T')
    if !key.modifiers.contains(KeyModifiers::CONTROL) => { ... }
```

---

## 7. Modal Rendering

### 7.1 Using ModalFrame Component

The `ModalFrame` component provides consistent modal styling:

```rust
use crate::ui::components::ModalFrame;

fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    // Full screen modal
    let modal_area = Rect::new(0, 0, area.width, area.height);

    // Create themed modal frame
    let modal = ModalFrame::themed(modal_area, " MY PLUGIN ", colors)
        // Optional: customize separators
        // .no_title_separator()
        // .no_footer_separator()
        ;

    // Render the frame (borders, title, separators)
    modal.render_frame(frame);

    // Render content rows (0-indexed from after title separator)
    modal.render_row(frame, 0, vec![
        Span::styled("Label: ", Style::default().fg(colors.blue())),
        Span::styled("Value", Style::default().fg(colors.fg())),
    ]);

    modal.render_row(frame, 2, vec![
        Span::styled("Another row", Style::default().fg(colors.fg())),
    ]);

    // Render help row at bottom
    modal.render_help(frame, vec![
        ("↑↓", "navigate"),
        ("Enter", "select"),
        ("Esc", "close"),
    ]);
}
```

### 7.2 View Switching

For plugins with multiple views:

```rust
fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let state = match &self.modal_state {
        Some(s) => s,
        None => return,
    };

    match state.view {
        MyView::List => self.draw_list_view(frame, area, colors, state),
        MyView::Detail => self.draw_detail_view(frame, area, colors, state),
        MyView::Create => self.draw_create_view(frame, area, colors, state),
    }
}

fn draw_list_view(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors, state: &MyState) {
    let modal = ModalFrame::themed(area, " MY PLUGIN - LIST ", colors);
    modal.render_frame(frame);
    // ...
}
```

### 7.3 Scrollable Lists

For lists longer than the visible area:

```rust
fn draw_list(&self, frame: &mut Frame, modal: &ModalFrame, colors: &ThemeColors, state: &MyState) {
    let visible_height = modal.content_height() as usize;
    let total_items = state.items.len();

    // Calculate scroll offset to keep selection visible
    let scroll_offset = if state.selected_index >= visible_height {
        state.selected_index - visible_height + 1
    } else {
        0
    };

    for (i, item) in state.items.iter().enumerate().skip(scroll_offset).take(visible_height) {
        let is_selected = i == state.selected_index;
        let style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg()).bg(colors.bg())
        };

        let row = (i - scroll_offset) as u16;
        modal.render_row(frame, row, vec![
            Span::styled(&item.name, style),
        ]);
    }

    // Show scroll indicator if needed
    if total_items > visible_height {
        let indicator = format!(" [{}/{}] ", state.selected_index + 1, total_items);
        // Render in corner
    }
}
```

### 7.4 Input Fields

For text input:

```rust
fn draw_input_field(
    frame: &mut Frame,
    modal: &ModalFrame,
    row: u16,
    label: &str,
    value: &str,
    cursor_pos: usize,
    colors: &ThemeColors,
    is_focused: bool,
) {
    let cursor_char = if is_focused { "_" } else { "" };
    let display_value = if is_focused {
        format!("{}{}", value, cursor_char)
    } else {
        value.to_string()
    };

    modal.render_row(frame, row, vec![
        Span::styled(label, Style::default().fg(colors.green()).bg(colors.bg())),
        Span::styled(&display_value, Style::default().fg(colors.fg()).bg(colors.bg())),
    ]);
}
```

---

## 8. Status Bar Integration

### 8.1 Providing Status Info

Plugins can contribute to the status bar:

```rust
fn capabilities(&self) -> PluginCapabilities {
    PluginCapabilities {
        has_status: true,
        ..Default::default()
    }
}

fn status_info(&self, cwd: &PathBuf) -> Option<PluginStatusInfo> {
    // Only show if relevant
    if !self.is_available(cwd) {
        return None;
    }

    // Get current status
    let count = self.get_item_count(cwd);

    Some(PluginStatusInfo {
        text: format!("[MY: {}]", count),  // Keep short! ~15-20 chars max
        active: count > 0,
    })
}
```

### 8.2 Status Text Guidelines

- **Maximum ~20 characters** - status bar space is limited
- **Use brackets** for visual grouping: `[GIT: main ↑2]`
- **Show counts** when relevant: `[BEADS: 5 open]`
- **Use symbols** for state: `↑` up, `↓` down, `✓` done, `✗` error

---

## 9. Help System Integration

### 9.1 Providing Help Content

Plugins can contribute help topics to the Help system (F1). When a plugin has `has_help: true`, its `help_content()` is automatically collected and displayed in the Help menu.

```rust
fn capabilities(&self) -> PluginCapabilities {
    PluginCapabilities {
        has_help: true,
        ..Default::default()
    }
}

fn help_content(&self) -> Vec<String> {
    vec![
        "G - Open Git menu".to_string(),
        "  Status - View file changes".to_string(),
        "  Log - View commit history".to_string(),
        "  Branches - Manage branches".to_string(),
    ]
}
```

### 9.2 Help Content Guidelines

Follow the Q-DOS II help format (see `spec/help.txt`):

- **Title line**: Centered command name (e.g., `"        G -- GIT VERSION CONTROL"`)
- **Purpose**: Brief description of what it does
- **To use**: How to access the feature
- **Key concepts**: Important terms or status bar info
- **Navigation**: Keyboard shortcuts within the modal
- **Common workflow**: Step-by-step usage guide
- **Tip**: Helpful hints (optional)

**Example help content:**
```rust
fn help_content(&self) -> Vec<String> {
    vec![
        "           J -- JUJUTSU VCS".to_string(),
        "".to_string(),
        "Purpose:   Jujutsu (jj) is a modern version control system that".to_string(),
        "           tracks changes automatically without staging.".to_string(),
        "".to_string(),
        "To use:    Press J to open the Jujutsu menu. Only available in".to_string(),
        "           directories with a .jj folder (jj repositories).".to_string(),
        "".to_string(),
        "Key concepts:".to_string(),
        "  - Changes: Like commits, but mutable until pushed".to_string(),
        "  - Working copy (@): Your current change, auto-updated".to_string(),
        "".to_string(),
        "Navigation:".to_string(),
        "  Tab       Switch between views".to_string(),
        "  Enter     Select item or confirm action".to_string(),
        "  Esc       Go back or close".to_string(),
        "".to_string(),
        "Tip: Use Operations > Undo to reverse any jj action.".to_string(),
    ]
}
```

**Don't** just list menu items - that duplicates the modal UI.

### 9.3 How It Works

1. During app initialization, `PluginManager.collect_plugin_help()` gathers content from all plugins with `has_help: true`
2. The collected content is passed to `HelpPlugin.load_plugin_help()`
3. Each plugin's help becomes a selectable topic in the Help menu
4. Topics are assigned keyboard shortcuts automatically (J, L, N, O, P, Q, S, T, U, W, X, Y, Z)
5. Plugins with existing hardcoded topics (Git, Beads) are not duplicated

---

## 10. Menu Integration

### 10.1 Providing Menu Items

```rust
fn capabilities(&self) -> PluginCapabilities {
    PluginCapabilities {
        has_menu: true,
        ..Default::default()
    }
}

fn menu_item(&self) -> Option<PluginMenuItem> {
    Some(PluginMenuItem {
        name: "MyPlugin".to_string(),      // Menu display name
        key: 'M',                           // Shortcut key (uppercase)
        description: "Do something useful".to_string(),
        priority: 50,                       // Lower = earlier in menu
    })
}
```

### 10.2 Priority Guidelines

| Priority | Plugins |
|----------|---------|
| 10-19 | Core (Help, Status) |
| 20-29 | File operations (View, Copy, Move) |
| 30-39 | Navigation (DirMap, SearchSpec) |
| 40-49 | Configuration (Qdconfig) |
| 50-59 | Features (Git, Beads, Shell) |
| 60-79 | Utilities (Space, Print) |
| 80-99 | Extras (Theme, Proc) |
| 90+ | Editors (Q-EDIT) |

---

## 11. Lifecycle Hooks

### 11.1 Initialization

Called when the app starts:

```rust
fn init(&mut self, cwd: &PathBuf) -> Result<(), String> {
    // Check for required tools
    if !self.check_tool_available() {
        // Don't fail - just note unavailability
        self.tool_available = false;
        return Ok(());
    }

    // Load initial data
    self.load_config()?;
    self.initialized = true;

    Ok(())
}
```

### 11.2 Shutdown

Called when the app exits:

```rust
fn shutdown(&mut self) -> Result<(), String> {
    // Save state if needed
    if let Some(state) = &self.modal_state {
        self.save_state(state)?;
    }

    // Cleanup resources
    self.cleanup_temp_files()?;

    Ok(())
}
```

### 11.3 Availability Check

Called to determine if plugin is relevant:

```rust
fn is_available(&self, cwd: &PathBuf) -> bool {
    // Example: Git plugin only available in git repos
    cwd.join(".git").exists()

    // Example: Always available
    // true

    // Example: Check for tool
    // self.tool_available
}
```

### 11.4 Tick (Auto-refresh)

Called ~every 100ms when modal is open:

```rust
fn tick(&mut self) {
    // Example: Auto-refresh data every 5 seconds
    if let Some(state) = &mut self.modal_state {
        state.tick_count += 1;
        if state.tick_count >= 50 {  // 50 * 100ms = 5 seconds
            state.tick_count = 0;
            self.refresh_data();
        }
    }
}
```

---

## 12. Background Jobs

### 12.1 Long-Running Operations

For operations that take time, show progress:

```rust
pub struct MyState {
    pub operation_in_progress: bool,
    pub progress: f32,  // 0.0 to 1.0
    pub progress_message: String,
}

fn tick(&mut self) {
    if let Some(state) = &mut self.modal_state {
        if state.operation_in_progress {
            // Poll for completion
            if let Some(result) = self.check_operation_status() {
                state.operation_in_progress = false;
                // Handle result
            }
        }
    }
}
```

### 12.2 Async Operations (Future)

Currently, plugins run synchronously. Future versions may support:

```rust
// FUTURE API - not yet implemented
async fn execute_async(&mut self, cwd: &PathBuf) -> Result<(), String> {
    let result = tokio::spawn(async {
        // Long-running work
    }).await?;
    Ok(())
}
```

### 12.3 External Process Execution

For running external commands:

```rust
use std::process::Command;

pub fn run_tool(args: &[&str], cwd: &Path) -> Result<String, String> {
    let output = Command::new("mytool")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to execute: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
```

---

## 13. Testing

### 13.1 Test File Location

```
src/plugins/myplugin/tests.rs
```

### 13.2 Unit Tests

```rust
// src/plugins/myplugin/tests.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_navigation() {
        let mut state = MyState::new();
        state.items = vec![
            MyItem { id: "1".into(), name: "Item 1".into() },
            MyItem { id: "2".into(), name: "Item 2".into() },
        ];

        assert_eq!(state.selected_index, 0);
        state.select_next();
        assert_eq!(state.selected_index, 1);
        state.select_next();
        assert_eq!(state.selected_index, 1); // Stays at end
    }

    #[test]
    fn test_plugin_capabilities() {
        let plugin = MyPlugin::new();
        let caps = plugin.capabilities();

        assert!(caps.has_modal);
        assert!(caps.has_keys);
    }
}
```

### 13.3 Integration Tests

For testing with real external tools:

```rust
#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_with_real_tool() {
    let tmpdir = tempfile::tempdir().unwrap();
    let cwd = tmpdir.path().to_path_buf();

    // Setup test environment
    std::fs::write(cwd.join("test.txt"), "content").unwrap();

    // Test operations
    let result = ops::do_action(&cwd);
    assert!(result.is_ok());
}
```

---

## 14. Configuration

### 14.1 Plugin Enable/Disable

Users can enable/disable plugins in `~/.config/rdos/config.toml`:

```toml
[plugins]
# Disable specific plugins
disable = ["print", "attribute"]

# Or enable only specific plugins (overrides disable)
# enable = ["git", "beads", "help"]
```

### 14.2 Plugin-Specific Config

If your plugin needs configuration:

```toml
[plugins.myplugin]
option1 = "value"
option2 = true
```

Access in plugin:

```rust
// Future API - configuration system TBD
fn init(&mut self, cwd: &PathBuf) -> Result<(), String> {
    // Currently: read from environment or plugin-specific file
    if let Ok(val) = std::env::var("MYPLUGIN_OPTION1") {
        self.config.option1 = val;
    }
    Ok(())
}
```

---

## 15. Future: Dynamic Loading

### 15.1 Preparation

Plugins are being designed for future dynamic loading:

1. **Self-contained** - All code in plugin directory
2. **No core dependencies** - Don't modify app/mod.rs
3. **Stable trait** - Plugin trait is the contract
4. **Serializable config** - Use TOML/JSON for settings

### 15.2 Future Plugin Distribution

```bash
# Future: Install from registry
rdos plugin install myplugin

# Future: Install from GitHub
rdos plugin install github.com/user/rdos-myplugin

# Future: List installed plugins
rdos plugin list
```

### 15.3 Plugin Manifest (Future)

```toml
# plugin.toml
[plugin]
id = "myplugin"
name = "My Plugin"
version = "1.0.0"
description = "Does something useful"
author = "Your Name"
license = "MIT"

[dependencies]
rdos = ">=0.5.0"
external_tools = ["mytool"]

[capabilities]
has_menu = true
has_modal = true
has_status = true
```

---

## Appendix A: Complete Plugin Example

```rust
// src/plugins/example/mod.rs

//! Example Plugin
//!
//! A minimal but complete plugin implementation.

mod state;

pub use state::ExampleState;

use crate::app::ThemeColors;
use crate::plugins::{
    KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem, PluginStatusInfo,
};
use crate::ui::components::ModalFrame;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;

pub struct ExamplePlugin {
    initialized: bool,
    pub modal_state: Option<ExampleState>,
}

impl ExamplePlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
            modal_state: None,
        }
    }

    pub fn open_modal(&mut self) {
        self.modal_state = Some(ExampleState::new());
    }
}

impl Default for ExamplePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ExamplePlugin {
    fn id(&self) -> &str {
        "example"
    }

    fn name(&self) -> &str {
        "Example"
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
        Ok(())
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Example".to_string(),
            key: 'X',
            description: "Example plugin demonstration".to_string(),
            priority: 100,
        })
    }

    fn handle_global_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.open_modal();
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.modal_state = None;
                KeyHandleResult::CloseModal
            }
            KeyCode::Enter => {
                KeyHandleResult::CloseWithSuccess("Action completed!".to_string())
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let modal_area = Rect::new(0, 0, area.width, area.height);
        let modal = ModalFrame::themed(modal_area, " EXAMPLE PLUGIN ", colors);
        modal.render_frame(frame);

        modal.render_row(
            frame,
            1,
            vec![Span::styled(
                "This is an example plugin.",
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );

        modal.render_row(
            frame,
            3,
            vec![Span::styled(
                "Press Enter to confirm or Esc to cancel.",
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );

        modal.render_help(frame, vec![("Enter", "confirm"), ("Esc", "cancel")]);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "EXAMPLE PLUGIN".to_string(),
            "".to_string(),
            "This is an example plugin that demonstrates".to_string(),
            "the plugin architecture.".to_string(),
            "".to_string(),
            "Keys:".to_string(),
            "  X      - Open example plugin".to_string(),
            "  Enter  - Confirm action".to_string(),
            "  Esc    - Close plugin".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
```

```rust
// src/plugins/example/state.rs

//! State types for Example Plugin

#[derive(Debug, Clone, Default)]
pub struct ExampleState {
    pub message: String,
}

impl ExampleState {
    pub fn new() -> Self {
        Self {
            message: "Hello from Example Plugin!".to_string(),
        }
    }
}
```

---

## Appendix B: Existing Plugin Reference

| Plugin | ID | Keys | Priority | Capabilities |
|--------|-----|------|----------|--------------|
| Help | help | F1 | 10 | menu, keys, modal, help |
| Status | status | F2 | 20 | menu, keys, modal |
| Viewer | viewer | F3 | 30 | menu, keys, modal |
| DirMap | dirmap | D | 30 | menu, keys, modal |
| SearchSpec | searchspec | F7 | 35 | menu, keys, modal |
| Qdconfig | qdconfig | Ctrl+S | 40 | menu, keys, modal |
| Git | git | G | 50 | menu, keys, modal, status |
| Beads | beads | B | 55 | menu, keys, modal, status |
| Shell | shell | F6 | 55 | menu, keys, modal |
| Space | space | F11 | 60 | menu, keys, modal |
| Print | print | P | 70 | menu, keys, modal |
| Theme | theme | Ctrl+T | 80 | menu, keys, modal |
| Q-EDIT | qedit | F9, Alt-F9 | 90 | menu, keys, modal, help |
| Proc | proc | F12 | 110 | menu, keys, modal |

---

## Appendix C: UI Component Roadmap

Planned shared components for `src/ui/components/`:

| Component | Status | Description |
|-----------|--------|-------------|
| ModalFrame | ✅ Done | Double-line border modal with title/help |
| ScrollableList | Planned | Scrollable list with selection |
| InputField | Planned | Text input with cursor |
| ConfirmDialog | Planned | Yes/No confirmation |
| ProgressBar | Planned | Q-DOS style progress indicator |
| Table | Planned | Column-aligned data table |
| Menu | Planned | Submenu with keyboard nav |
| Tabs | Planned | Tab bar for view switching |

---

*Last updated: 2026-01-04*
*R-DOS Version: 0.4.0+*
