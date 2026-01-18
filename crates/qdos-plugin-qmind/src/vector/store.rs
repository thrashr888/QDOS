//! In-memory vector store with cosine similarity
//!
//! Stores file embeddings and provides semantic search functionality.
//! Supports persistence to disk.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// A single entry in the vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    /// Unique identifier (usually file path)
    pub id: String,
    /// The embedding vector
    pub embedding: Vec<f32>,
    /// Associated metadata
    pub metadata: EntryMetadata,
}

/// Metadata associated with a vector entry
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntryMetadata {
    /// File path
    pub path: PathBuf,
    /// File name
    pub name: String,
    /// File type/extension
    pub file_type: String,
    /// File size in bytes
    pub size: u64,
    /// Last modified timestamp
    pub modified: u64,
    /// Content summary/preview
    pub summary: String,
}

impl VectorEntry {
    /// Create a new vector entry
    pub fn new(id: impl Into<String>, embedding: Vec<f32>, metadata: EntryMetadata) -> Self {
        Self {
            id: id.into(),
            embedding,
            metadata,
        }
    }

    /// Create entry from a file path
    pub fn from_path(path: &Path, embedding: Vec<f32>) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let file_type = path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        Self {
            id: path.to_string_lossy().to_string(),
            embedding,
            metadata: EntryMetadata {
                path: path.to_path_buf(),
                name,
                file_type,
                ..Default::default()
            },
        }
    }
}

/// Search result with similarity score
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The matching entry
    pub entry: VectorEntry,
    /// Cosine similarity score (0.0-1.0)
    pub score: f32,
}

impl SearchResult {
    pub fn new(entry: VectorEntry, score: f32) -> Self {
        Self { entry, score }
    }
}

/// Serializable store data for persistence
#[derive(Serialize, Deserialize)]
struct StoreData {
    /// Embedding dimension
    dimension: usize,
    /// AI provider used for embeddings (e.g., "openai", "anthropic")
    #[serde(default)]
    provider: String,
    /// Embedding model used (e.g., "text-embedding-3-small")
    #[serde(default)]
    embedding_model: String,
    /// Vector entries
    entries: Vec<VectorEntry>,
}

/// In-memory vector store
pub struct VectorStore {
    /// Stored vectors by ID
    entries: HashMap<String, VectorEntry>,
    /// Expected dimension of embeddings
    dimension: usize,
    /// AI provider used for embeddings
    provider: String,
    /// Embedding model used
    embedding_model: String,
}

impl VectorStore {
    /// Create a new vector store with provider info
    pub fn new_with_provider(
        dimension: usize,
        provider: impl Into<String>,
        embedding_model: impl Into<String>,
    ) -> Self {
        Self {
            entries: HashMap::new(),
            dimension,
            provider: provider.into(),
            embedding_model: embedding_model.into(),
        }
    }

    /// Create a new vector store (legacy, uses empty provider/model)
    pub fn new(dimension: usize) -> Self {
        Self::new_with_provider(dimension, "", "")
    }

    /// Get the expected embedding dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Get the AI provider used for this store
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Get the embedding model used for this store
    pub fn embedding_model(&self) -> &str {
        &self.embedding_model
    }

    /// Check if this store was created with a specific provider
    pub fn is_provider(&self, provider: &str) -> bool {
        self.provider.eq_ignore_ascii_case(provider)
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Add or update an entry
    pub fn upsert(&mut self, entry: VectorEntry) -> Result<(), VectorStoreError> {
        if entry.embedding.len() != self.dimension {
            return Err(VectorStoreError::DimensionMismatch {
                expected: self.dimension,
                got: entry.embedding.len(),
            });
        }

        self.entries.insert(entry.id.clone(), entry);
        Ok(())
    }

    /// Remove an entry by ID
    pub fn remove(&mut self, id: &str) -> Option<VectorEntry> {
        self.entries.remove(id)
    }

    /// Get an entry by ID
    pub fn get(&self, id: &str) -> Option<&VectorEntry> {
        self.entries.get(id)
    }

    /// Check if an entry exists
    pub fn contains(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    /// Search for similar vectors
    pub fn search(
        &self,
        query: &[f32],
        top_k: usize,
    ) -> Result<Vec<SearchResult>, VectorStoreError> {
        if query.len() != self.dimension {
            return Err(VectorStoreError::DimensionMismatch {
                expected: self.dimension,
                got: query.len(),
            });
        }

        let mut results: Vec<SearchResult> = self
            .entries
            .values()
            .map(|entry| {
                let score = cosine_similarity(query, &entry.embedding);
                SearchResult::new(entry.clone(), score)
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top_k results
        results.truncate(top_k);

        Ok(results)
    }

    /// Search with a minimum similarity threshold
    pub fn search_threshold(
        &self,
        query: &[f32],
        min_score: f32,
        max_results: usize,
    ) -> Result<Vec<SearchResult>, VectorStoreError> {
        let results = self.search(query, max_results)?;
        Ok(results
            .into_iter()
            .filter(|r| r.score >= min_score)
            .collect())
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get all entry IDs
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(|s| s.as_str())
    }

    /// Filter entries by path prefix
    pub fn entries_in_dir(&self, dir: &Path) -> Vec<&VectorEntry> {
        self.entries
            .values()
            .filter(|e| e.metadata.path.starts_with(dir))
            .collect()
    }

    /// Save store to a file
    pub fn save(&self, path: &Path) -> Result<(), VectorStoreError> {
        use std::io::Write;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| VectorStoreError::IoError(e.to_string()))?;
        }

        let data = StoreData {
            dimension: self.dimension,
            provider: self.provider.clone(),
            embedding_model: self.embedding_model.clone(),
            entries: self.entries.values().cloned().collect(),
        };

        // Write to a temp file first, then rename (atomic write)
        let temp_path = path.with_extension("json.tmp");
        let file =
            fs::File::create(&temp_path).map_err(|e| VectorStoreError::IoError(e.to_string()))?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &data)
            .map_err(|e| VectorStoreError::IoError(e.to_string()))?;

        // Ensure all data is flushed to disk
        writer
            .flush()
            .map_err(|e| VectorStoreError::IoError(e.to_string()))?;
        writer
            .into_inner()
            .map_err(|e| VectorStoreError::IoError(e.to_string()))?
            .sync_all()
            .map_err(|e| VectorStoreError::IoError(e.to_string()))?;

        // Atomic rename
        fs::rename(&temp_path, path).map_err(|e| VectorStoreError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Load store from a file
    pub fn load(path: &Path) -> Result<Self, VectorStoreError> {
        let file = fs::File::open(path).map_err(|e| VectorStoreError::IoError(e.to_string()))?;
        let reader = BufReader::new(file);
        let data: StoreData = serde_json::from_reader(reader)
            .map_err(|e| VectorStoreError::IoError(e.to_string()))?;

        let mut store =
            Self::new_with_provider(data.dimension, data.provider, data.embedding_model);
        for entry in data.entries {
            store.entries.insert(entry.id.clone(), entry);
        }

        Ok(store)
    }

    /// Get default index path (~/Library/Application Support/rdos/qmind-index.json)
    pub fn default_index_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("rdos").join("qmind-index.json"))
    }
}

/// Vector store errors
#[derive(Debug)]
pub enum VectorStoreError {
    /// Embedding dimension doesn't match store dimension
    DimensionMismatch { expected: usize, got: usize },
    /// IO error during save/load
    IoError(String),
}

impl std::fmt::Display for VectorStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VectorStoreError::DimensionMismatch { expected, got } => {
                write!(f, "Dimension mismatch: expected {}, got {}", expected, got)
            }
            VectorStoreError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for VectorStoreError {}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let norm = norm_a.sqrt() * norm_b.sqrt();
    if norm == 0.0 {
        0.0
    } else {
        dot / norm
    }
}

/// Normalize a vector to unit length
#[allow(dead_code)]
fn normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_store() {
        let store = VectorStore::new(128);
        assert_eq!(store.dimension(), 128);
        assert!(store.is_empty());
    }

    #[test]
    fn test_upsert_and_get() {
        let mut store = VectorStore::new(3);
        let entry = VectorEntry::new("test", vec![1.0, 0.0, 0.0], EntryMetadata::default());

        store.upsert(entry.clone()).unwrap();
        assert_eq!(store.len(), 1);
        assert!(store.contains("test"));

        let retrieved = store.get("test").unwrap();
        assert_eq!(retrieved.embedding, vec![1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_dimension_mismatch() {
        let mut store = VectorStore::new(3);
        let entry = VectorEntry::new("test", vec![1.0, 0.0], EntryMetadata::default());

        let result = store.upsert(entry);
        assert!(matches!(
            result,
            Err(VectorStoreError::DimensionMismatch { .. })
        ));
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_search() {
        let mut store = VectorStore::new(3);

        store
            .upsert(VectorEntry::new(
                "a",
                vec![1.0, 0.0, 0.0],
                EntryMetadata::default(),
            ))
            .unwrap();
        store
            .upsert(VectorEntry::new(
                "b",
                vec![0.9, 0.1, 0.0],
                EntryMetadata::default(),
            ))
            .unwrap();
        store
            .upsert(VectorEntry::new(
                "c",
                vec![0.0, 1.0, 0.0],
                EntryMetadata::default(),
            ))
            .unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = store.search(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].entry.id, "a"); // Most similar
        assert_eq!(results[1].entry.id, "b"); // Second most similar
    }

    #[test]
    fn test_search_threshold() {
        let mut store = VectorStore::new(3);

        store
            .upsert(VectorEntry::new(
                "a",
                vec![1.0, 0.0, 0.0],
                EntryMetadata::default(),
            ))
            .unwrap();
        store
            .upsert(VectorEntry::new(
                "b",
                vec![0.0, 1.0, 0.0],
                EntryMetadata::default(),
            ))
            .unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = store.search_threshold(&query, 0.5, 10).unwrap();

        assert_eq!(results.len(), 1); // Only "a" meets threshold
        assert_eq!(results[0].entry.id, "a");
    }

    #[test]
    fn test_remove() {
        let mut store = VectorStore::new(3);
        store
            .upsert(VectorEntry::new(
                "test",
                vec![1.0, 0.0, 0.0],
                EntryMetadata::default(),
            ))
            .unwrap();

        assert!(store.contains("test"));
        store.remove("test");
        assert!(!store.contains("test"));
    }

    #[test]
    fn test_from_path() {
        let path = Path::new("/home/user/documents/test.txt");
        let entry = VectorEntry::from_path(path, vec![1.0, 0.0, 0.0]);

        assert_eq!(entry.metadata.name, "test.txt");
        assert_eq!(entry.metadata.file_type, "txt");
        assert_eq!(entry.metadata.path, path);
    }
}
