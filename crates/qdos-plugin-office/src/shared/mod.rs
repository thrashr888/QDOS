//! Office Suite Shared Infrastructure
//!
//! Common traits and utilities shared across all office applications.

mod document;
mod formats;
pub mod html;
pub mod network;
mod saveas;

pub use document::OfficeDocument;

// Format detection utilities (for future use)
#[allow(unused_imports)]
pub use formats::{detect_format, is_document_format, is_spreadsheet_format, FileFormat};

// HTML parsing utilities
#[allow(unused_imports)]
pub use html::HtmlDocument;

// Reader mode content extraction
pub use html::extract_reader_content;

// Network utilities for Q-WEB and Q-MAIL
#[allow(unused_imports)]
pub use network::{parse_url, HttpClient, Response, UrlParts};

// Save As dialog components (for future use with other office apps)
#[allow(unused_imports)]
pub use saveas::{draw_save_as_modal, handle_save_as_key, tab_complete, SaveAsResult, SaveAsState};
