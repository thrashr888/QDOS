//! Vector store for Q-MIND semantic search
//!
//! Provides in-memory vector storage with cosine similarity search.

mod store;

pub use store::{EntryMetadata, SearchResult, VectorEntry, VectorStore};
