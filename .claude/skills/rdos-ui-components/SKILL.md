---
name: rdos-ui-components
description: UI components and rendering for R-DOS. Use when rendering modals, dialogs, lists, or any UI. CRITICAL - explains ModalFrame vs FullScreenView to prevent panics.
---

# R-DOS UI Components

## CRITICAL: ModalFrame vs FullScreenView

**This is the #1 source of bugs. ModalFrame PANICS on full-screen areas!**

| Component | Use For | Max Size |
|-----------|---------|----------|
| `FullScreenView` | Full-screen plugin views | Unlimited |
| `ModalFrame` | Small centered dialogs | 79x23 max |

### FullScreenView (Full-screen views)

```rust
use crate::ui::components::FullScreenView;

fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " TITLE ", colors);
    view.render_frame(frame);
    let content = view.content_area();

    // Render content...

    view.render_help(frame, vec![("Esc", "close")]);
}
```

### ModalFrame (Small dialogs only)

```rust
use crate::ui::components::ModalFrame;

fn draw_dialog(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    // MUST calculate centered sub-area first!
    let width = area.width.min(55);
    let height = area.height.min(14);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    let modal = ModalFrame::themed(modal_area, " CONFIRM ", colors);
    modal.render_frame(frame);
    modal.render_help(frame, vec![("Y", "yes"), ("N", "no")]);
}
```

## Available Components

```rust
use crate::ui::components::{
    ModalFrame,       // Small centered dialogs (max 79x23)
    FullScreenView,   // Full-screen plugin views
    MessageModal,     // Error/success/info/warning
    ProgressBar,      // Q-DOS style progress
    ScrollableList,   // Selection with scrolling
    Table,            // Columnar data
    InputField,       // Text input
    ConfirmDialog,    // Y/N prompts
    TabBar,           // Horizontal tab navigation
    TabState,         // Tab navigation state management
    LogViewer,        // Streaming log viewer with auto-scroll
    LogViewerState,   // Log viewer state management
    LogStatus,        // Running/Success/Failed status
};
```

## TabBar Component

For plugins with multiple views (Containers/Images/Volumes/Networks):

```rust
use crate::ui::components::TabBar;

// Define tabs and render
let tabs = vec!["Containers", "Images", "Volumes", "Networks"];
let tab_bar = TabBar::new(&tabs, selected_index);
let spans = tab_bar.render(&colors);
view.render_row(frame, 0, spans);

// Navigation with TabState
let mut state = TabState::new(4);  // 4 tabs
state.next();  // Tab key
state.prev();  // Shift+Tab
```

## LogViewer Component

For streaming command output (builds, terraform plan/apply):

```rust
use crate::ui::components::{LogViewer, LogViewerState, LogStatus};

// Create state
let mut state = LogViewerState::new().with_visible_height(18);

// Add lines (typically from background process)
state.push_line("Step 1/5: Starting...");
state.push_line("Downloading dependencies...");

// Render
let viewer = LogViewer::new(&state)
    .title("Build Output")
    .status(LogStatus::Running);
viewer.render(frame, &view, &colors);

// Handle scroll keys
state.scroll_up();     // Up arrow
state.scroll_down();   // Down arrow
state.page_up(18);     // PageUp
state.page_down(18);   // PageDown
state.scroll_to_top(); // Home
state.scroll_to_bottom(); // End (re-enables following)

// Get help items based on status
let help = viewer.help_items();
view.render_help(frame, help);
```

### LogStatus Values

- `LogStatus::Idle` - Ready, not running
- `LogStatus::Running` - In progress (yellow)
- `LogStatus::Success` - Completed successfully (green)
- `LogStatus::Failed` - Command failed (red)

## Theme Colors

**NEVER hardcode colors.** Use `ThemeColors`:

```rust
fn draw(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    colors.fg()      // White - text, borders
    colors.bg()      // Default background
    colors.blue()    // Headers, menu items
    colors.green()   // Help text, key hints
    colors.red()     // Errors, selection bg
    colors.yellow()  // Selected text, titles
    colors.grey()    // Disabled, hidden files
    colors.cyan()    // Accents
}
```

## Selection Highlighting

```rust
let style = if is_selected {
    Style::default().fg(colors.yellow()).bg(colors.red())
} else {
    Style::default().fg(colors.fg())
};
```

## Q-DOS II Screen Layout

```
Row 0:    Title bar
Row 1:    ═══════════════════════════ (separator)
Row 2:    Context info
Row 3-N:  Content area (scrollable)
Row N-1:  ═══════════════════════════ (separator)
Row N:    Help row (keybindings)
```

## Border Characters

Double-line borders: `╔═╗║╚╝╠╣╦╩╬`

## Minimum Size

All layouts must work at 80x25 terminal size.
