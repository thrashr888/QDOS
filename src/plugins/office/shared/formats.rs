//! File Format Detection
//!
//! Shared format detection for office applications.

use std::path::Path;

// =============================================================================
// FILE FORMAT ENUM
// =============================================================================

/// Supported file formats across office applications
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// Comma-separated values
    Csv,
    /// Tab-separated values
    Tsv,
    /// Microsoft Excel spreadsheet
    Xlsx,
    /// Legacy Excel format
    Xls,
    /// Plain text
    Txt,
    /// Markdown
    Markdown,
    /// Rich text format
    Rtf,
    /// Word document
    Docx,
    /// HTML
    Html,
}

impl FileFormat {
    /// Get the primary extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            FileFormat::Csv => "csv",
            FileFormat::Tsv => "tsv",
            FileFormat::Xlsx => "xlsx",
            FileFormat::Xls => "xls",
            FileFormat::Txt => "txt",
            FileFormat::Markdown => "md",
            FileFormat::Rtf => "rtf",
            FileFormat::Docx => "docx",
            FileFormat::Html => "html",
        }
    }

    /// Get all valid extensions for this format
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            FileFormat::Csv => &["csv"],
            FileFormat::Tsv => &["tsv", "tab"],
            FileFormat::Xlsx => &["xlsx"],
            FileFormat::Xls => &["xls"],
            FileFormat::Txt => &["txt", "text"],
            FileFormat::Markdown => &["md", "markdown"],
            FileFormat::Rtf => &["rtf"],
            FileFormat::Docx => &["docx", "doc"],
            FileFormat::Html => &["html", "htm"],
        }
    }

    /// Get display name for this format
    pub fn display_name(&self) -> &'static str {
        match self {
            FileFormat::Csv => "CSV",
            FileFormat::Tsv => "TSV",
            FileFormat::Xlsx => "Excel",
            FileFormat::Xls => "Excel (Legacy)",
            FileFormat::Txt => "Text",
            FileFormat::Markdown => "Markdown",
            FileFormat::Rtf => "RTF",
            FileFormat::Docx => "Word",
            FileFormat::Html => "HTML",
        }
    }

    /// Check if this format supports formulas
    pub fn supports_formulas(&self) -> bool {
        matches!(self, FileFormat::Csv | FileFormat::Xlsx | FileFormat::Xls)
    }
}

// =============================================================================
// FORMAT DETECTION
// =============================================================================

/// Detect file format from path extension
pub fn detect_format(path: &Path) -> Option<FileFormat> {
    let ext = path.extension()?.to_str()?.to_lowercase();

    match ext.as_str() {
        "csv" => Some(FileFormat::Csv),
        "tsv" | "tab" => Some(FileFormat::Tsv),
        "xlsx" => Some(FileFormat::Xlsx),
        "xls" => Some(FileFormat::Xls),
        "txt" | "text" => Some(FileFormat::Txt),
        "md" | "markdown" => Some(FileFormat::Markdown),
        "rtf" => Some(FileFormat::Rtf),
        "docx" | "doc" => Some(FileFormat::Docx),
        "html" | "htm" => Some(FileFormat::Html),
        _ => None,
    }
}

/// Check if a path has a spreadsheet format
pub fn is_spreadsheet_format(path: &Path) -> bool {
    matches!(
        detect_format(path),
        Some(FileFormat::Csv | FileFormat::Tsv | FileFormat::Xlsx | FileFormat::Xls)
    )
}

/// Check if a path has a document format
pub fn is_document_format(path: &Path) -> bool {
    matches!(
        detect_format(path),
        Some(
            FileFormat::Txt
                | FileFormat::Markdown
                | FileFormat::Rtf
                | FileFormat::Docx
                | FileFormat::Html
        )
    )
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_csv() {
        let path = PathBuf::from("test.csv");
        assert_eq!(detect_format(&path), Some(FileFormat::Csv));
    }

    #[test]
    fn test_detect_xlsx() {
        let path = PathBuf::from("test.xlsx");
        assert_eq!(detect_format(&path), Some(FileFormat::Xlsx));
    }

    #[test]
    fn test_detect_markdown() {
        let path = PathBuf::from("README.md");
        assert_eq!(detect_format(&path), Some(FileFormat::Markdown));
    }

    #[test]
    fn test_detect_unknown() {
        let path = PathBuf::from("test.xyz");
        assert_eq!(detect_format(&path), None);
    }

    #[test]
    fn test_is_spreadsheet() {
        assert!(is_spreadsheet_format(&PathBuf::from("data.csv")));
        assert!(is_spreadsheet_format(&PathBuf::from("data.xlsx")));
        assert!(!is_spreadsheet_format(&PathBuf::from("doc.md")));
    }
}
