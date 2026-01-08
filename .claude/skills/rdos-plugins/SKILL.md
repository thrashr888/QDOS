---
name: rdos-plugins
description: Plugin development for R-DOS. Use when creating new plugins, implementing Plugin trait, handling keys, or structuring plugin code. Reference spec/PLUGIN.md for complete specification.
---

# R-DOS Plugin Development

See `spec/PLUGIN.md` for the complete plugin specification.

## Plugin Structure

```
src/plugins/myplugin/
├── mod.rs      # Plugin struct, Plugin trait impl
├── state.rs    # State types, enums
├── modal.rs    # Modal rendering
└── ops.rs      # Business logic (optional)
```

## Self-Containment Rules

- **ZERO** modifications to `src/app/mod.rs`
- **MINIMAL** additions to `src/plugins/mod.rs` (registration only)
- **NO** plugin-specific code in `src/ui/`
- All state, operations, UI in plugin directory

## Plugin Trait

```rust
impl Plugin for MyPlugin {
    fn id(&self) -> &str { "myplugin" }
    fn name(&self) -> &str { "My Plugin" }

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

    fn handle_global_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        // Called when NO modal is open
        match key.code {
            KeyCode::Char('m') => {
                self.open_modal(cwd);
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        // Called when THIS plugin's modal is open
        match key.code {
            KeyCode::Esc => KeyHandleResult::CloseModal,
            KeyCode::Enter => {
                // Do action
                KeyHandleResult::CloseWithSuccess("Done".to_string())
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        // Use FullScreenView for full-screen, ModalFrame for dialogs
    }
}
```

## Key Handling

### Conventions

| Key Type | Examples | Use Case |
|----------|----------|----------|
| F1-F12 | F7 | Primary plugin actions |
| Letters | G, B | Menu shortcuts |
| Navigation | ↑↓←→ | List navigation |
| Actions | Enter, Space, Esc | Confirm, toggle, cancel |

### Avoid Ctrl Conflicts

```rust
// WRONG - catches Ctrl+T too
KeyCode::Char('t') => { ... }

// RIGHT - exclude Ctrl modifier
KeyCode::Char('t') if !key.modifiers.contains(KeyModifiers::CONTROL) => { ... }
```

## State Pattern

```rust
// state.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MyView {
    #[default]
    Menu,
    Detail,
    Error,
}

#[derive(Debug, Clone, Default)]
pub struct MyState {
    pub view: MyView,
    pub items: Vec<Item>,
    pub selected: usize,
    pub error: Option<String>,
}

impl MyState {
    pub fn select_next(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1) % self.items.len();
        }
    }
    pub fn select_prev(&mut self) {
        if !self.items.is_empty() {
            self.selected = self.selected.checked_sub(1).unwrap_or(self.items.len() - 1);
        }
    }
}
```

## Registration

In `src/plugins/mod.rs`:

1. Add module declaration and pub use:
```rust
pub mod myplugin;
pub use myplugin::MyPlugin;
```

2. Add accessor method (if plugin needs direct access):
```rust
pub fn myplugin_plugin_mut(&mut self) -> Option<&mut myplugin::MyPlugin> {
    self.plugins
        .get_mut("myplugin")
        .and_then(|p| p.as_any_mut().downcast_mut::<myplugin::MyPlugin>())
}
```

In `src/app/mod.rs`:

1. Add import:
```rust
use crate::plugins::{MyPlugin, ...};
```

2. Register plugin:
```rust
plugin_manager.register(Box::new(MyPlugin::new()));
```

3. Add Apps launcher handler (in `launch_plugin_modal`):
```rust
"myplugin" => {
    if let Some(plugin) = self.plugin_manager.myplugin_plugin_mut() {
        plugin.open_modal(...);
    }
    self.plugin_manager.set_active_modal(Some("myplugin"));
    self.modal = Modal::Plugin("myplugin".to_string());
}
```

## Apps Launcher (F12)

To appear in the Apps launcher, implement `app_entry()`:

```rust
fn app_entry(&self) -> Option<AppEntry> {
    Some(AppEntry {
        id: "myplugin".to_string(),      // Must match plugin.id()
        name: "My Plugin".to_string(),
        description: "Short description".to_string(),
        category: PluginCategory::Tools,  // Tools, Media, System, etc.
        key: 'M',                         // Keyboard shortcut in Apps
    })
}
```

## Thread Safety (Send + Sync)

The `Plugin` trait requires `Send + Sync`. Avoid storing non-thread-safe types in plugin structs.

**Problem**: External libraries may have types that aren't thread-safe (e.g., `rusqlite::Connection` uses `RefCell`).

**Solution**: Don't store connections persistently. Open/close for each operation:

```rust
// WRONG - won't compile
pub struct MyPlugin {
    conn: Option<Connection>,  // Connection is not Send+Sync
}

// RIGHT - stateless operations
pub struct MyPlugin {
    state: MyState,  // Only store serializable state
}

// Operations open connection on-demand
pub fn get_data(path: &Path) -> Result<Data, String> {
    let conn = open_connection(path)?;
    // use conn...
}
```

## File-Based Plugins

For plugins that work with files (databases, media, etc.):

1. Store the file path in state, not an open handle
2. Provide `is_*_file()` helper for file type detection
3. Use the file path to open resources on-demand

```rust
impl MyPlugin {
    pub fn is_my_file(path: &PathBuf) -> bool {
        path.extension()
            .map(|ext| matches!(ext.to_string_lossy().to_lowercase().as_str(), "xyz" | "abc"))
            .unwrap_or(false)
    }
}
```

## Auto-Play Pattern

Skip menu when only one option:

```rust
if self.state.available_players.len() == 1 {
    self.play();
} else {
    self.state.view = MyView::Menu;
}
```
