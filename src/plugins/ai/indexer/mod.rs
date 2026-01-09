//! Lazy file indexer for Q-MIND
//!
//! Incrementally indexes files as the user navigates,
//! generating embeddings for semantic search.

mod indexer;

pub use indexer::{FileIndexer, IndexConfig, IndexStats};
