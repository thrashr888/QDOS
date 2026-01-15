//! Office Document Trait
//!
//! Common interface for all office document types.

use std::path::Path;

/// Common trait for all office document types
pub trait OfficeDocument {
    /// File extensions this document type supports
    fn extensions() -> &'static [&'static str]
    where
        Self: Sized;

    /// Create a new empty document
    fn new_document() -> Self
    where
        Self: Sized;

    /// Load document from file
    fn load(path: &Path) -> Result<Self, String>
    where
        Self: Sized;

    /// Save document to file
    fn save(&self, path: &Path) -> Result<(), String>;

    /// Check if document has unsaved changes
    fn is_modified(&self) -> bool;

    /// Get display name for the document
    fn display_name(&self) -> String;
}
