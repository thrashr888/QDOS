//! Semantic search implementation
//!
//! Combines embeddings API and vector store for natural language file search.

use crate::plugins::ai::api::{embeddings::create_embeddings_provider, AIApiConfig, ApiError};
use crate::plugins::ai::indexer::FileIndexer;
use crate::plugins::ai::vector::SearchResult;
use std::path::{Path, PathBuf};

/// Result from semantic search
#[derive(Debug, Clone)]
pub struct SemanticSearchResult {
    /// File path
    pub path: PathBuf,
    /// File name
    pub name: String,
    /// Similarity score (0.0-1.0)
    pub score: f32,
    /// Content summary/preview
    pub summary: String,
    /// File type/extension
    pub file_type: String,
}

impl SemanticSearchResult {
    /// Create from vector search result
    fn from_search_result(result: SearchResult) -> Self {
        Self {
            path: result.entry.metadata.path.clone(),
            name: result.entry.metadata.name.clone(),
            score: result.score,
            summary: result.entry.metadata.summary.clone(),
            file_type: result.entry.metadata.file_type.clone(),
        }
    }
}

/// Semantic file search
pub struct SemanticSearch {
    /// API configuration
    config: AIApiConfig,
    /// File indexer with vector store
    indexer: FileIndexer,
    /// Minimum similarity score for results
    min_score: f32,
    /// Maximum results to return
    max_results: usize,
}

impl SemanticSearch {
    /// Create a new semantic search instance
    pub fn new(config: AIApiConfig) -> Self {
        use crate::plugins::ai::indexer::IndexConfig;

        Self {
            indexer: FileIndexer::new(config.clone(), IndexConfig::default()),
            config,
            min_score: 0.3,
            max_results: 20,
        }
    }

    /// Create using environment API keys
    pub fn from_env() -> Self {
        Self::new(AIApiConfig::from_env())
    }

    /// Check if search is configured (has API key)
    pub fn is_configured(&self) -> bool {
        self.config.is_configured()
    }

    /// Set minimum similarity score (0.0-1.0)
    pub fn set_min_score(&mut self, score: f32) {
        self.min_score = score.clamp(0.0, 1.0);
    }

    /// Set maximum results
    pub fn set_max_results(&mut self, max: usize) {
        self.max_results = max;
    }

    /// Index a directory for searching
    pub fn index_directory(&mut self, dir: &Path) -> Result<usize, SearchError> {
        self.indexer
            .index_directory(dir)
            .map_err(|e| SearchError::IndexError(e.to_string()))
    }

    /// Index a single file
    pub fn index_file(&mut self, path: &Path) -> Result<bool, SearchError> {
        self.indexer
            .index_file(path)
            .map_err(|e| SearchError::IndexError(e.to_string()))
    }

    /// Search files with a natural language query
    pub fn search(&self, query: &str) -> Result<Vec<SemanticSearchResult>, SearchError> {
        if !self.is_configured() {
            return Err(SearchError::NoApiKey);
        }

        if self.indexer.store().is_empty() {
            return Ok(vec![]); // No files indexed yet
        }

        // Generate embedding for query
        let query_embedding = self.generate_query_embedding(query)?;

        // Search vector store
        let results = self
            .indexer
            .store()
            .search_threshold(&query_embedding, self.min_score, self.max_results)
            .map_err(|e| SearchError::VectorError(e.to_string()))?;

        // Convert to semantic search results
        Ok(results
            .into_iter()
            .map(SemanticSearchResult::from_search_result)
            .collect())
    }

    /// Search within a specific directory
    pub fn search_in_dir(
        &self,
        query: &str,
        dir: &Path,
    ) -> Result<Vec<SemanticSearchResult>, SearchError> {
        let results = self.search(query)?;
        Ok(results
            .into_iter()
            .filter(|r| r.path.starts_with(dir))
            .collect())
    }

    /// Generate embedding for a search query
    fn generate_query_embedding(&self, query: &str) -> Result<Vec<f32>, SearchError> {
        let provider = create_embeddings_provider(self.config.clone())
            .map_err(|e| SearchError::ApiError(e))?;

        let response = provider
            .embed(query)
            .map_err(|e| SearchError::ApiError(e))?;

        Ok(response.embedding)
    }

    /// Get indexer statistics
    pub fn stats(&self) -> &crate::plugins::ai::indexer::IndexStats {
        self.indexer.stats()
    }

    /// Clear the search index
    pub fn clear(&mut self) {
        self.indexer.clear();
    }

    /// Get number of indexed files
    pub fn indexed_count(&self) -> usize {
        self.indexer.store().len()
    }
}

/// Errors from semantic search operations
#[derive(Debug)]
pub enum SearchError {
    /// No API key configured
    NoApiKey,
    /// API error
    ApiError(ApiError),
    /// Indexing error
    IndexError(String),
    /// Vector store error
    VectorError(String),
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::NoApiKey => write!(f, "No API key configured for semantic search"),
            SearchError::ApiError(e) => write!(f, "API error: {}", e),
            SearchError::IndexError(msg) => write!(f, "Index error: {}", msg),
            SearchError::VectorError(msg) => write!(f, "Vector error: {}", msg),
        }
    }
}

impl std::error::Error for SearchError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_search_creation() {
        let search = SemanticSearch::from_env();
        // Will be unconfigured without API key in env
        assert_eq!(search.indexed_count(), 0);
    }

    #[test]
    fn test_set_min_score() {
        let mut search = SemanticSearch::new(AIApiConfig::default());
        search.set_min_score(0.5);
        assert!((search.min_score - 0.5).abs() < 0.001);

        // Test clamping
        search.set_min_score(1.5);
        assert!((search.min_score - 1.0).abs() < 0.001);

        search.set_min_score(-0.5);
        assert!((search.min_score - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_search_empty_index() {
        let search = SemanticSearch::new(AIApiConfig::default());
        // Should return empty results with unconfigured API
        let result = search.search("test query");
        assert!(result.is_err() || result.unwrap().is_empty());
    }

    #[test]
    fn test_clear() {
        let mut search = SemanticSearch::new(AIApiConfig::default());
        search.clear();
        assert_eq!(search.indexed_count(), 0);
    }
}
