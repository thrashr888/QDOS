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

## Releasing

1. Update version in `Cargo.toml`
2. Commit, tag (`git tag -a v0.x.x -m "Release notes"`), and push tag
3. GitHub Actions builds multi-platform binaries and creates release
4. Update homebrew tap at `../homebrew-qdos` with new version and SHA256 hashes
