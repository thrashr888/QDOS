//! Reusable UI Components
//!
//! Common UI components for consistent modal and widget rendering.
//!
//! ## Available Components
//!
//! - [`ModalFrame`] - Double-line border modal with title and help row
//! - [`FullScreenView`] - Full-screen layout with title, separators, content, footer
//! - [`MessageModal`] - Simple message modal (error, success, info, warning)
//! - [`ProgressBar`] - Q-DOS style progress indicator (bar, arrow, spinner)
//! - [`ScrollableList`] - Scrollable list with selection highlighting
//! - [`ListState`] - State management for list navigation
//! - [`Table`] - Column-based table with alignment
//! - [`InputField`] - Text input with cursor and editing support
//! - [`ConfirmDialog`] - Yes/No confirmation dialog
//! - [`colors`] - Status color helpers for consistent theming
//!
//! ## Usage
//!
//! ```ignore
//! use crate::ui::components::{
//!     ModalFrame, ScrollableList, ListState,
//!     Table, Column, InputField, ConfirmDialog, ConfirmResult
//! };
//! use crate::ui::components::colors;
//!
//! // Create a modal frame
//! let modal = ModalFrame::themed(area, " TITLE ", &colors);
//! modal.render_frame(frame);
//!
//! // Render a scrollable list inside
//! let list = ScrollableList::new(&items, state.selected, visible_height);
//! list.render(frame, content_area, colors, |item, selected, style| {
//!     vec![Span::styled(item.name.clone(), style)]
//! });
//!
//! // Render a table
//! let table = Table::new(vec![
//!     Column::new("ID", 10).left(),
//!     Column::new("Name", 20).left(),
//!     Column::new("Size", 12).right(),
//! ]);
//! table.render_header(frame, area, y, &colors);
//! table.render_row(frame, area, y + 1, &["123", "file.txt", "1.2 KB"], false, &colors);
//!
//! // Use an input field
//! let mut input = InputField::with_content("/path/to/file");
//! input.insert('x');
//! input.render(frame, area, &colors, true);
//!
//! // Create a confirmation dialog
//! let dialog = ConfirmDialog::new("Delete 5 files?")
//!     .with_warning("This cannot be undone.")
//!     .yes_label("Delete")
//!     .no_label("Cancel");
//! dialog.render(frame, area, &colors);
//!
//! // Use status colors
//! let color = colors::git_status_color('M', &theme_colors);  // Yellow for modified
//! let color = colors::priority_color(1, &theme_colors);      // Red for P1
//! ```

// Allow unused for component library - public API for plugins
#[allow(dead_code)]
pub mod colors;
#[allow(dead_code)]
mod confirm;
#[allow(dead_code)]
mod input;
#[allow(dead_code)]
mod list;
mod modal;
#[allow(dead_code)]
mod screen;
#[allow(dead_code)]
mod message;
#[allow(dead_code)]
mod progress;
#[allow(dead_code)]
mod table;

#[allow(unused_imports)]
pub use confirm::{ConfirmDialog, ConfirmResult};
#[allow(unused_imports)]
pub use message::{MessageModal, MessageType};
#[allow(unused_imports)]
pub use input::InputField;
#[allow(unused_imports)]
pub use list::{truncate_with_ellipsis, ListState, ScrollableList};
pub use modal::ModalFrame;
#[allow(unused_imports)]
pub use progress::{ProgressBar, ProgressStyle};
#[allow(unused_imports)]
pub use screen::FullScreenView;
#[allow(unused_imports)]
pub use table::{Align, Column, Table};
