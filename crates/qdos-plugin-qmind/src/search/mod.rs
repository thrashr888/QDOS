//! Semantic search for Q-MIND
//!
//! Provides natural language file search using embeddings.

mod semantic;

// SemanticSearchResult is re-exported for external use
#[allow(unused_imports)]
pub use semantic::{SemanticSearch, SemanticSearchResult};
