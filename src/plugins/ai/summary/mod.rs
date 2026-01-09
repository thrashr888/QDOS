//! File summary module for Q-MIND
//!
//! Generates AI-powered summaries of file contents.

mod summarizer;

pub use summarizer::{FileSummarizer, FileSummary, SummaryError};
