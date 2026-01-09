//! File summary module for Q-MIND
//!
//! Generates AI-powered summaries of file contents.

mod summarizer;

// SummaryError is re-exported for external use
#[allow(unused_imports)]
pub use summarizer::{FileSummarizer, FileSummary, SummaryError};
