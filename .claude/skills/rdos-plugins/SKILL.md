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

- **MINIMAL** modifications to `src/app/mod.rs` (import, registration, Apps launcher)
- **MINIMAL** additions to `src/plugins/mod.rs` (module declaration, pub use, accessor method)
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

## Registration Checklist

### 1. `README.md`

Add your plugin to the Apps section in README.md under the appropriate category:

```markdown
### Category Name
| Key | App | Description |
|-----|-----|-------------|
| X | My Plugin | Short description |
```

### 2. `src/plugins/mod.rs`

Add module declaration, pub use, and accessor method:

```rust
// Module declaration (alphabetical order)
pub mod myplugin;

// Public export (alphabetical order)
pub use myplugin::MyPlugin;

// In PluginManager impl - accessor method
pub fn myplugin_plugin_mut(&mut self) -> Option<&mut myplugin::MyPlugin> {
    self.plugins
        .get_mut("myplugin")
        .and_then(|p| p.as_any_mut().downcast_mut::<myplugin::MyPlugin>())
}
```

### 3. `src/app/mod.rs`

Add import and registration:

```rust
// Add to imports (alphabetical order)
use crate::plugins::{MyPlugin, ...};

// Add registration in App::new() (alphabetical order)
plugin_manager.register(Box::new(MyPlugin::new()));
```

### 4. Apps Launcher Integration (REQUIRED)

**Every plugin with `app_entry()` MUST have a handler in `launch_plugin_modal()`.**

The F12 Apps launcher uses `launch_plugin_modal()` to open plugins. Without a handler, you get "Unknown app: myplugin" error.

**Simple plugins** (no special initialization needed):
```rust
// The generic fallback will handle it - no explicit code needed
// Just ensure app_entry() returns the correct id
```

**Plugins needing initialization** (file path, directory, etc.):
```rust
"myplugin" => {
    // Pass context if needed
    if let Some(plugin) = self.plugin_manager.myplugin_plugin_mut() {
        plugin.open_modal(&self.current_path);  // or other initialization
    }
    self.plugin_manager.set_active_modal(Some("myplugin"));
    self.modal = Modal::Plugin("myplugin".to_string());
}
```

Add your handler in alphabetical order within the match statement.

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

## Multi-Backend Plugins

For plugins supporting multiple backends (e.g., SQLite, PostgreSQL, MySQL):

1. **Type enum** for backend selection:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendType {
    TypeA,
    TypeB,
    TypeC,
}
```

2. **Separate operation modules** per backend:
```
src/plugins/myplugin/
├── mod.rs       # Plugin struct, dispatch logic
├── state.rs     # Shared state types
├── modal.rs     # UI rendering
├── backend_a.rs # Backend A operations
├── backend_b.rs # Backend B operations
└── backend_c.rs # Backend C operations
```

3. **Dispatch pattern** in operations:
```rust
fn do_operation(&mut self) {
    let result = match self.state.backend_type {
        Some(BackendType::TypeA) => backend_a::operation(&self.state.config),
        Some(BackendType::TypeB) => backend_b::operation(&self.state.config),
        Some(BackendType::TypeC) => backend_c::operation(&self.state.config),
        None => return,
    };
    // Handle result...
}
```

## Connection Form Pattern

For plugins that need connection configuration (host, port, user, etc.):

1. **Config struct** with URL builder:
```rust
#[derive(Debug, Clone, Default)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

impl ConnectionConfig {
    pub fn new_with_defaults(port: u16, user: &str) -> Self {
        Self {
            host: "localhost".to_string(),
            port,
            user: user.to_string(),
            ..Default::default()
        }
    }
}
```

2. **Field enum** for form navigation:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectField {
    #[default]
    Host,
    Port,
    User,
    Password,
    Database,
}
```

3. **Field navigation and input methods**:
```rust
impl MyState {
    pub fn next_field(&mut self) {
        self.field = match self.field {
            ConnectField::Host => ConnectField::Port,
            // ...cycle through fields
        };
    }

    pub fn insert_char(&mut self, c: char) {
        match self.field {
            ConnectField::Host => self.config.host.push(c),
            ConnectField::Port => {
                if c.is_ascii_digit() {
                    // Parse and update port
                }
            }
            // ...handle each field
        }
    }
}
```

4. **Form rendering** with field highlighting:
```rust
fn draw_form_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    selected: bool,
    is_password: bool,
    colors: &ThemeColors,
) {
    let display = if is_password { "*".repeat(value.len()) } else { value.to_string() };
    let border_color = if selected { colors.yellow() } else { colors.grey() };
    // Render block with border and text
}
```
