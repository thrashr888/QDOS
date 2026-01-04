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
│   ├── components/   # Reusable UI component library
│   └── viewer.rs     # File viewer with ASCII/HEX/Markdown/Image modes
├── plugins/          # Plugin modules (git, beads, proc, qedit, etc.)
├── config.rs         # TOML configuration (~/.config/rdos/config.toml)
├── file_ops.rs       # File system operations and utilities
├── errors.rs         # Q-DOS style error messages
└── event.rs          # Terminal event handling
```

### Key Patterns

- **Event-driven TUI**: Main loop in `app/mod.rs` handles keyboard events, dispatches to appropriate handlers based on current state
- **Modal stack**: Dialogs can be nested; `ActiveModal` enum in `state.rs` defines all modal types
- **Plugin system**: Plugins implement traits in `plugins/mod.rs`; see `spec/PLUGIN.md` for complete guide
- **Component library**: Reusable UI components in `src/ui/components/` for consistent rendering
- **State machine**: Navigation state, view modes, and modal states are managed through enums in `state.rs`
- **Idiomatic Rust**: Stick to modern Rust features, best practices, and patterns
- **Dead code**: Allow dead code to be present in the codebase, but remove it when it's not needed

## Spec & Reference Files

- `spec/SPEC.md` - Detailed feature specification
- `spec/PLUGIN.md` - **Plugin development specification** (MUST read before creating plugins)
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
4. One commit per beads issue
5. NEVER `bd sync` or `git push` and NEVER ask to do so
6. Only close a release epic after the release is published

## UI Component Library

All modal and plugin UI should use components from `src/ui/components/`. See the module documentation for full API.

### Available Components

```rust
use crate::ui::components::{
    ModalFrame,       // Double-line border modal with title and help
    FullScreenView,   // Full-screen layout with separators
    MessageModal,     // Error/success/info/warning modals
    ProgressBar,      // Q-DOS style progress (bar/arrow/spinner)
    ScrollableList,   // Selection, scrolling, highlighting
    Table,            // Column specs, headers, alignment
    InputField,       // Text input with cursor
    ConfirmDialog,    // Y/N confirmation prompts
};
use crate::ui::components::colors; // Status color helpers
```

### Theme Colors

Use `ThemeColors` from `app.colors()` - never hardcode colors:

```rust
fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    colors.fg()      // White - primary text, borders
    colors.bg()      // Terminal default - backgrounds
    colors.blue()    // DOS Blue - headers, menu items
    colors.green()   // DOS Green - help text, key hints
    colors.red()     // DOS Red - selection background, errors
    colors.yellow()  // DOS Yellow - selected text, titles
    colors.grey()    // Grey - hidden files, disabled items
    colors.cyan()    // Cyan - git added files
}
```

### Example: Plugin Modal with Components

```rust
use crate::ui::components::{ModalFrame, ScrollableList};

fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    // Use ModalFrame for consistent double-line borders
    let modal = ModalFrame::themed(area, " My Plugin ", colors);
    modal.render_frame(frame);

    // Use ScrollableList for item selection
    let list = ScrollableList::new(&self.items, self.selected, visible_height);
    list.render(frame, modal.content_area(), colors, |item, selected, style| {
        vec![Span::styled(&item.name, style)]
    });

    // Use built-in help rendering
    modal.render_help(frame, vec![("Enter", "select"), ("Esc", "close")]);
}
```

## Plugin Architecture

**See `spec/PLUGIN.md` for the complete plugin development guide**, including:
- File structure (mod.rs, state.rs, ops.rs, modal.rs)
- Plugin trait implementation
- Key handling patterns
- UI/UX conventions
- Testing requirements

### Quick Reference

```rust
impl Plugin for MyPlugin {
    fn id(&self) -> &str { "myplugin" }
    fn name(&self) -> &str { "My Plugin" }

    fn handle_global_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        // Return OpenModal, Handled, or NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        // Return CloseModal, CloseWithSuccess(msg), CloseWithError(msg), or Handled
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        // Use ModalFrame or FullScreenView from component library
    }
}
```

Register plugins in `App::new()`:

```rust
plugin_manager.register(Box::new(MyPlugin::new()));
```

## Releasing

0. Batch issues and epics into release epics
1. Run quality checks: `cargo fmt --check && cargo clippy && cargo test`
2. Update version in `Cargo.toml` (remove `-dev` suffix)
3. Commit, tag (`git tag -a v0.x.x -m "Release notes"`), and push tag
4. GitHub Actions builds multi-platform binaries and creates release
5. Update homebrew tap at `../homebrew-qdos` with new version and SHA256 hashes:
   ```bash
   curl -sL https://github.com/thrashr888/QDOS/releases/download/vX.X.X/rdos-macos-aarch64 | shasum -a 256
   curl -sL https://github.com/thrashr888/QDOS/releases/download/vX.X.X/rdos-macos-x86_64 | shasum -a 256
   ```
