//! Office Suite Shared Infrastructure
//!
//! Common traits and utilities shared across all office applications.

mod document;
mod formats;
mod saveas;

pub use document::OfficeDocument;

// Format detection utilities (for future use)
#[allow(unused_imports)]
pub use formats::{detect_format, is_document_format, is_spreadsheet_format, FileFormat};

// Save As dialog components (for future use with other office apps)
#[allow(unused_imports)]
pub use saveas::{draw_save_as_modal, handle_save_as_key, tab_complete, SaveAsResult, SaveAsState};
