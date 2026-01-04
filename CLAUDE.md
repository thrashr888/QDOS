# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

R-DOS is a retro DOS-style file manager TUI written in Rust, recreating the classic Q-DOS II (1991, Gazelle Systems). It uses ratatui for terminal rendering and crossterm for cross-platform terminal control.

## Build & Development Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Release build (LTO enabled)

# Run
cargo run                      # Run in current directory
cargo run /path/to/dir         # Run in specific directory
./target/release/rdos          # Run release binary

# Quality checks
cargo fmt -- --check           # Check formatting
cargo clippy -- -D warnings    # Lint with warnings as errors
cargo test --verbose           # Run tests (minimal - binary only)
```

## Architecture

```
src/
├── main.rs           # Entry point, terminal setup, main loop
├── app/
│   ├── mod.rs        # Core state machine, event handling, command dispatch
│   └── state.rs      # All state enums (NavItem, SortMode, ViewMode, Modal types)
├── ui/
│   ├── mod.rs        # Main layout rendering (menu bar, file table, stats panel)
│   ├── modals.rs     # Modal dialog rendering (copy, move, erase, rename, find, etc.)
│   └── viewer.rs     # File viewer with ASCII/HEX/Markdown/Image modes
├── plugins/
│   ├── mod.rs        # Plugin trait and manager
│   ├── git/          # Git integration (status, log, diff, commit, push, pull)
│   └── beads/        # Beads issue tracker integration
├── config.rs         # TOML configuration (~/.config/rdos/config.toml)
├── file_ops.rs       # File system operations and utilities
├── errors.rs         # Q-DOS style error messages
└── event.rs          # Terminal event handling
```

### Key Patterns

- **Event-driven TUI**: Main loop in `app/mod.rs` handles keyboard events, dispatches to appropriate handlers based on current state
- **Modal stack**: Dialogs can be nested; `ActiveModal` enum in `state.rs` defines all modal types
- **Plugin system**: Plugins implement traits in `plugins/mod.rs`; currently Git and Beads are built-in
- **State machine**: Navigation state, view modes, and modal states are managed through enums in `state.rs`
- **Idiomatic Rust**: Stick to modern Rust features, best practices, and patterns.
- **Dead code**: Allow dead code to be present in the codebase, but remove it when it's not needed. Old or unused code can live on in our git history.

## Spec & Reference Files

- `spec/SPEC.md` - Detailed feature specification
- `spec/ui.md` - ASCII layout reference for the 80x25 screen
- `spec/strings/` - Organized original Q-DOS strings by feature (for authentic messaging)

## Issue Tracking

This project uses **bd** (beads) for issue tracking.

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

## Code Quality

Write code that is already `cargo fmt` and `cargo clippy` compliant - don't fix formatting after the fact. Before committing, verify with:

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
```

CI runs on every push, so all commits must pass fmt/lint/build checks.

## Session Completion

Before ending a session:

1. File issues for remaining work
2. Verify quality gates pass (`cargo fmt --check`, `cargo clippy`, `cargo test`)
3. Update issue status - close finished work
4. Commit changes and run `bd sync`
5. **Ask before pushing** - do not `git push` without user confirmation

## Plugin Architecture

Plugins are self-contained modules in `src/plugins/` that extend QDOS functionality. See `git/` and `beads/` for reference implementations.

### Plugin Directory Structure

```
src/plugins/myplugin/
├── mod.rs       # Plugin struct, Plugin trait impl, key handlers
├── state.rs     # State types (views, menu items, data structs)
└── ops.rs       # Operations (CLI commands, data parsing, actions)
```

### Plugin Trait Implementation

```rust
pub struct MyPlugin {
    initialized: bool,
    pub modal_state: Option<MyState>,  // Plugin owns its modal state
}

impl Plugin for MyPlugin {
    fn id(&self) -> &str { "myplugin" }
    fn name(&self) -> &str { "My Plugin" }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,   // Provides menu item
            has_keys: true,   // Handles keyboard shortcuts
            has_modal: true,  // Has modal UI
            has_status: true, // Provides status bar content
            has_cli: false,   // Provides CLI arguments
            has_help: true,   // Provides help content
        }
    }

    fn handle_global_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        // Handle key when modal is NOT open
        // Return OpenModal to open plugin's modal
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        // Handle key when modal IS open
        // Return CloseModal to close
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect) {
        // Draw plugin's modal UI
    }
}
```

### Key Patterns

1. **State Ownership**: Plugins own their modal state via `modal_state: Option<State>`
2. **State Re-export**: Re-export state types in `mod.rs` for external use
3. **Ops Module**: Separate operations (CLI calls, parsing) from UI logic
4. **Key Results**: Use `KeyHandleResult` enum for key handling outcomes:
   - `NotHandled` - Let app handle the key
   - `Handled` - Key processed, no further action
   - `OpenModal` - Open plugin's modal
   - `CloseModal` - Close plugin's modal
   - `CloseWithSuccess(msg)` / `CloseWithError(msg)` - Close with message

### Registration

Register plugins in `App::new()`:

```rust
plugin_manager.register(Box::new(MyPlugin::new()));
```

## Releasing

1. Update version in `Cargo.toml`
2. Commit, tag (`git tag -a v0.x.x -m "Release notes"`), and push tag
3. GitHub Actions builds multi-platform binaries and creates release
4. Update homebrew tap at `../homebrew-qdos` with new version and SHA256 hashes
