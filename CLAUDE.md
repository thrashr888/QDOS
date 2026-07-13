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

## Code Migration & Refactoring Tools

Prefer these tools for large-scale code changes, renames, and migrations:

```bash
# ripgrep (rg) - Fast code search
rg "old_pattern"               # Find all occurrences
rg -l "pattern"                # List files only
rg -t rust "pattern"           # Search only Rust files

# fastmod - Interactive find-and-replace
fastmod "old_name" "new_name"  # Interactive rename across codebase
fastmod -d src/ "old" "new"    # Limit to directory

# ast-grep - Structural code search and transform
ast-grep -p 'fn $NAME() { $$$BODY }' -l rs  # Find function patterns
ast-grep --rewrite 'new_pattern'             # Transform code structurally
```

**When to use each:**
- **rg**: Finding where code is used, exploring patterns
- **fastmod**: Simple text renames (variables, functions, types)
- **ast-grep**: Structural changes (API migrations, pattern transforms)

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
├── config.rs         # TOML configuration (~/Library/Application Support/rdos/)
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
- **No emoji**: Use ASCII and CP437 extended characters only. Box-drawing (╔═║╗), card suits (♠♥♦♣), and other CP437 characters are acceptable. Avoid non-CP437 Unicode (★, ●, ⚀-⚅) and true emoji (🍒, 🎰). Use ASCII alternatives like `*`, `o`, `#` instead.

## Documentation System

**READ FIRST**: `spec/DEVELOPMENT.md` - **The 5-style documentation system** explaining how we develop features

This project uses a living documentation system with 5 doc types:
1. **Ultra/Plan Mode** - Deep exploration before coding
2. **Beads Issues** - Track multi-session work with dependencies
3. **Evergreen Specs** - How the system works (see below)
4. **Skills** - How agents implement (in `.claude/skills/`)
5. **User Docs** - README files for humans

### Specs & Reference Files

- `spec/DEVELOPMENT.md` - **5-style documentation system** (meta-process for feature development)
- `spec/SPEC.md` - Detailed feature specification
- `spec/PLUGIN.md` - **Plugin development specification** (MUST read before creating plugins)
- `spec/GAMES.md` - Games architecture and patterns
- `spec/OFFICE.md` - Office features specification
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

### Issue Lifecycle Rules

**IMPORTANT**: Follow this order for issue completion:

1. **Code complete** - Implementation is done, tests pass
2. **Commit code** - Create git commit(s) for the work
3. **Wait for user** - DO NOT close issues yet
4. **User tests** - Let user test the implementation
5. **User pushes** - User pushes code to remote
6. **Then close issues** - Only close after code is pushed and tested

**DO NOT** close issues immediately after implementation. The user needs to:
- Review the changes
- Test the functionality
- Push the code to remote

**Epics** should only be closed after:
- All child issues are closed
- The feature has been tested by user
- The code has been pushed to remote

## Version Control

This project uses **jj** (Jujutsu) colocated with git. jj provides:
- Automatic change tracking (no staging area)
- Undo any operation with `jj undo`
- Stable change IDs that survive rebases
- First-class conflict handling

```bash
jj status            # Show working copy status
jj log               # View change history
jj diff              # Show current changes
jj describe -m "..."  # Update change description
jj new               # Start new change
jj undo              # Undo last operation
jj git push          # Push to git remote
```

Use git for remote operations (`git push`, `git pull`) or jj equivalents (`jj git push/fetch`).

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
3. Create commits for completed work (one commit per beads issue recommended)
4. DO NOT close issues - leave that for user after testing/pushing
5. NEVER `bd sync` or `git push` and NEVER ask to do so
6. Only close epics after the release is published or user confirms

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

### CRITICAL: FullScreenView vs ModalFrame

**ModalFrame PANICS on full-screen areas!** This is the #1 source of bugs.

| Component | Use For | Max Size |
|-----------|---------|----------|
| `FullScreenView` | Plugin modals (draw_modal) | Unlimited |
| `ModalFrame` | Small centered dialogs only | 79x23 max |

### Example: Plugin Modal (MUST use FullScreenView)

```rust
use crate::ui::components::FullScreenView;

fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    // ALWAYS use FullScreenView for plugin modals - ModalFrame will PANIC!
    let view = FullScreenView::new(area, " My Plugin ", colors);
    view.render_frame(frame);

    // Render content rows
    view.render_row(frame, 0, vec![Span::styled("Content", style)]);

    // Help footer
    view.render_help(frame, vec![("Enter", "select"), ("Esc", "close")]);
}
```

### Example: Small Centered Dialog (ModalFrame)

Only use ModalFrame for small dialogs - MUST calculate centered area first:

```rust
use crate::ui::components::ModalFrame;

fn draw_confirm_dialog(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    // Calculate centered sub-area (REQUIRED - don't pass full area!)
    let width = area.width.min(55);
    let height = area.height.min(14);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    let modal = ModalFrame::themed(modal_area, " Confirm ", colors);
    modal.render_frame(frame);
    modal.render_help(frame, vec![("Y", "yes"), ("N", "no")]);
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
1. Run **ALL** quality checks - tests are mandatory, not optional:
   ```bash
   cargo fmt -- --check && cargo clippy -- -D warnings && cargo test
   ```
2. Update version in `Cargo.toml` (remove `-dev` suffix)
3. Commit, tag (`git tag -a v0.x.x -m "Release notes"`), and push tag
4. GitHub Actions builds binaries (Linux, macOS ARM, Windows) and creates release
   - **Note**: Intel Mac (x86_64-apple-darwin) is NOT supported due to unreliable GitHub runners
5. Update homebrew tap at `../homebrew-qdos` with new version and SHA256 hash:
   ```bash
   curl -sL https://github.com/thrashr888/QDOS/releases/download/vX.X.X/rdos-macos-aarch64 | shasum -a 256
   ```

### Release Troubleshooting

- If a workflow job fails/cancels, you may need to delete and recreate the tag:
  ```bash
  git push origin :refs/tags/vX.X.X  # Delete remote tag
  git tag -d vX.X.X                   # Delete local tag
  git tag -a vX.X.X -m "Release"      # Recreate tag
  git push origin vX.X.X              # Push new tag
  ```
- Test workflow changes on a separate branch/tag before the real release


<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:7510c1e2 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md for details and anti-patterns.

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
