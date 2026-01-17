# Telnet Plugin (QDOS-bg6)

## Overview
Create a Telnet client plugin for QDOS, enabling connections to remote telnet servers with full ANSI terminal emulation. Reuses the VT100/tui-term infrastructure from the shell plugin.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│ User Input (crossterm events)                           │
└─────────────────┬───────────────────────────────────────┘
                  │
        ┌─────────▼─────────┐
        │  Telnet Plugin    │
        │  TelnetView::     │
        │  Connected        │
        └─────────┬─────────┘
                  │
     ┌────────────┼────────────┐
     │            │            │
┌────▼──────┐ ┌──▼─────────┐ ┌▼────────────┐
│ telnet    │ │  vt100     │ │ TcpStream   │
│ crate     │ │  parser    │ │ (non-block) │
└────┬──────┘ └──┬─────────┘ └─────────────┘
     │           │
     └───────────┼──────────────────┐
                 │                  │
          ┌──────▼──────┐          │
          │   tui-term  │◄─────────┘
          │  (widget)   │
          └──────┬──────┘
                 │
          ┌──────▼────────────┐
          │ ratatui Frame     │
          └───────────────────┘
```

## Dependencies

```toml
# Cargo.toml - add:
telnet = "0.2"
# tui-term already present for shell plugin
```

## Approach

**Extend Shell Plugin** - Add Telnet as views within the existing shell plugin rather than a separate plugin. This keeps terminal/network features together and simplifies the menu integration.

## File Structure

```
src/plugins/shell/
├── mod.rs       # Add Telnet menu item and key handlers (+150 LOC)
├── state.rs     # Add TelnetView variants, TelnetState (+80 LOC)
├── modal.rs     # Add telnet view renderers (+200 LOC)
└── telnet.rs    # NEW: TelnetSession, connection logic (~200 LOC)
```

**Total: ~630 LOC added to shell plugin**

## Implementation Plan

### Phase 1: State Types (state.rs)

```rust
pub enum TelnetView {
    Menu,           // Entry point with options
    ConnectForm,    // Host/port input
    Connecting,     // Loading state
    Connected,      // Active terminal session
    History,        // Past connections
    Bookmarks,      // Saved servers
    Error,          // Connection errors
}

pub enum TelnetMenuItem {
    Connect,    // C - New connection
    History,    // H - View history
    Bookmarks,  // B - Saved servers
}

pub struct TelnetState {
    pub view: TelnetView,
    pub menu_selected: usize,
    pub host_input: String,
    pub port_input: String,
    pub input_field: usize,
    pub history: Vec<HistoryEntry>,
    pub bookmarks: Vec<TelnetBookmark>,
    pub error_message: Option<String>,
}
```

### Phase 2: Session Management (ops.rs)

```rust
pub struct TelnetSession {
    telnet: Telnet,                        // From telnet crate
    parser: Arc<Mutex<vt100::Parser>>,     // Terminal emulation
    pub host: String,
    pub port: u16,
}

impl TelnetSession {
    pub fn connect(host: &str, port: u16, cols: u16, rows: u16) -> Result<Self>;
    pub fn write(&mut self, data: &[u8]) -> Result<()>;
    pub fn try_read(&mut self) -> Result<bool>;  // Non-blocking
    pub fn screen(&self) -> Arc<Mutex<vt100::Parser>>;
    pub fn is_connected(&self) -> bool;
}

// Reuse from shell plugin:
pub use crate::plugins::shell::pty::key_to_bytes;
```

### Phase 3: Plugin Implementation (mod.rs)

```rust
pub struct TelnetPlugin {
    modal_open: bool,
    state: TelnetState,
    session: Option<TelnetSession>,
    terminal_size: (u16, u16),
}

impl Plugin for TelnetPlugin {
    fn id(&self) -> &str { "telnet" }
    fn name(&self) -> &str { "Telnet" }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true,
            has_status: true,
            has_cli: false,
            has_help: true,
        }
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Telnet".to_string(),
            key: 'N',
            description: "Connect to telnet servers".to_string(),
            priority: 60,
        })
    }

    fn tick(&mut self) {
        // Poll session for incoming data (non-blocking)
        if let Some(ref mut session) = self.session {
            let _ = session.try_read();
        }
    }
}
```

Key handlers:
- **Menu**: Arrow keys to select, Enter/key to launch
- **ConnectForm**: Tab between host/port, Enter to connect
- **Connected**: Ctrl+] to escape (traditional telnet), all others forwarded to remote

### Phase 4: Modal Rendering (modal.rs)

- **Menu view**: List of options (Connect, History, Bookmarks)
- **Connect form**: Host and port input fields
- **Connected view**: Full-screen PseudoTerminal widget (same as shell)
- **History/Bookmarks**: Scrollable list with Enter to reconnect

### Phase 5: Integration

1. Add `pub mod telnet;` to `src/plugins/mod.rs`
2. Register in `App::new()`: `plugin_manager.register(Box::new(TelnetPlugin::new()));`
3. Add `telnet = "0.2"` to Cargo.toml

## Key Binding

**Decision**: Add Telnet as 4th option in F6 DOS Command menu

This integrates with existing shell plugin menu:
- C - Command (existing)
- I - Interactive Shell (existing)
- J - Jobs (existing)
- **T - Telnet** (new)

## User Flow

```
Press F6 (DOS Command)
        │
        ▼
┌───────────────────┐
│   DOS COMMAND     │
├───────────────────┤
│   C  Command      │
│   I  Interactive  │
│   J  Jobs         │
│ > T  Telnet       │  ← NEW
└───────────────────┘
        │ T
        ▼
┌───────────────────┐
│   TELNET          │
├───────────────────┤
│ > C  Connect      │
│   H  History      │
│   B  Bookmarks    │
└───────────────────┘
        │ C
        ▼
┌───────────────────┐
│ Host: bbs.example │
│ Port: 23          │
│ [Enter to connect]│
└───────────────────┘
        │ Enter
        ▼
┌───────────────────┐
│ Connected to      │
│ bbs.example:23    │
│                   │
│ (terminal output) │
│                   │
│ Ctrl+] to escape  │
└───────────────────┘
```

## Telnet Protocol Handling

The `telnet` crate handles IAC negotiation:
- Accept ECHO, SUPPRESS-GO-AHEAD
- Respond appropriately to WILL/WONT/DO/DONT
- Terminal type: "xterm-256color"

## Files to Modify

| File | Action |
|------|--------|
| `Cargo.toml` | Add `telnet = "0.2"` |
| `src/plugins/shell/telnet.rs` | Create - TelnetSession struct |
| `src/plugins/shell/state.rs` | Add Telnet menu item, TelnetState |
| `src/plugins/shell/modal.rs` | Add telnet view renderers |
| `src/plugins/shell/mod.rs` | Add telnet handling, menu option |

## Reference Files

- `src/plugins/shell/pty.rs` - Reuse `key_to_bytes()`, VT100 pattern
- `src/plugins/shell/modal.rs` - PseudoTerminal rendering for interactive shell
