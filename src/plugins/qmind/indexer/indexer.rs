//! Lazy file indexer implementation
//!
//! Indexes files on-demand as the user navigates directories.
//! Uses .gitignore to determine what to skip.

use crate::plugins::qmind::api::{embeddings::create_embeddings_provider, AIApiConfig, ApiError};
use crate::plugins::qmind::vector::{EntryMetadata, VectorEntry, VectorStore};
use ignore::WalkBuilder;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Configuration for the file indexer
#[derive(Debug, Clone)]
pub struct IndexConfig {
    /// Maximum file size to index content (in bytes)
    pub max_file_size: u64,
    /// File extensions to index content (others get name-only indexing)
    pub content_extensions: HashSet<String>,
    /// Maximum content bytes to use for embedding
    pub max_content_bytes: usize,
    /// Whether to respect .gitignore (default: true)
    pub respect_gitignore: bool,
    /// Whether to include hidden files not in .gitignore (default: true)
    pub include_hidden: bool,
}

impl Default for IndexConfig {
    fn default() -> Self {
        let mut content_extensions = HashSet::new();
        for ext in &[
            "txt",
            "md",
            "rs",
            "py",
            "js",
            "ts",
            "tsx",
            "jsx",
            "go",
            "c",
            "cpp",
            "h",
            "hpp",
            "java",
            "rb",
            "sh",
            "bash",
            "zsh",
            "toml",
            "yaml",
            "yml",
            "json",
            "xml",
            "html",
            "css",
            "scss",
            "sql",
            "dockerfile",
            "makefile",
            "cmake",
            "gradle",
            "swift",
            "kt",
            "scala",
            "clj",
            "ex",
            "exs",
            "erl",
            "hs",
            "ml",
            "lua",
            "php",
            "pl",
            "r",
            "jl",
            "nim",
            "zig",
            "v",
        ] {
            content_extensions.insert(ext.to_string());
        }

        Self {
            max_file_size: 100 * 1024, // 100 KB
            content_extensions,
            max_content_bytes: 4096, // Use first 4KB for embedding
            respect_gitignore: true,
            include_hidden: true, // Include .github, .config, etc. (gitignore handles .git)
        }
    }
}

/// Statistics about the index
#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    /// Total files indexed
    pub files_indexed: usize,
    /// Total directories indexed
    pub dirs_indexed: usize,
    /// Files pending indexing
    pub pending: usize,
    /// Total tokens used
    pub tokens_used: u32,
    /// Index errors
    pub errors: usize,
}

/// Lazy file indexer
pub struct FileIndexer {
    /// API configuration
    config: AIApiConfig,
    /// Index configuration
    index_config: IndexConfig,
    /// Vector store
    store: VectorStore,
    /// Indexed paths (to avoid re-indexing)
    indexed_paths: HashSet<PathBuf>,
    /// Statistics
    stats: IndexStats,
}

impl FileIndexer {
    /// Create a new file indexer
    pub fn new(api_config: AIApiConfig, index_config: IndexConfig) -> Self {
        // Determine dimension based on provider
        let dimension = match api_config.provider {
            crate::plugins::qmind::api::AIProvider::OpenAI => 1536,
            crate::plugins::qmind::api::AIProvider::Anthropic => 64,
        };

        Self {
            config: api_config,
            index_config,
            store: VectorStore::new(dimension),
            indexed_paths: HashSet::new(),
            stats: IndexStats::default(),
        }
    }

    /// Create indexer using environment API keys
    pub fn from_env() -> Self {
        Self::new(AIApiConfig::from_env(), IndexConfig::default())
    }

    /// Check if indexer has API access
    pub fn is_configured(&self) -> bool {
        self.config.is_configured()
    }

    /// Get indexing statistics
    pub fn stats(&self) -> &IndexStats {
        &self.stats
    }

    /// Get the vector store
    pub fn store(&self) -> &VectorStore {
        &self.store
    }

    /// Get mutable vector store
    pub fn store_mut(&mut self) -> &mut VectorStore {
        &mut self.store
    }

    /// Index a single file
    pub fn index_file(&mut self, path: &Path) -> Result<bool, IndexError> {
        // Check if already indexed
        if self.indexed_paths.contains(path) {
            return Ok(false);
        }

        // Check if file should be skipped
        if self.should_skip(path) {
            return Ok(false);
        }

        // Get file metadata
        let metadata = fs::metadata(path).map_err(|e| IndexError::IoError(e.to_string()))?;

        if !metadata.is_file() {
            return Ok(false);
        }

        // Generate text for embedding
        let embed_text = self.generate_embed_text(path, &metadata)?;

        // Generate embedding
        let embedding = self.generate_embedding(&embed_text)?;

        // Create metadata
        let entry_meta = EntryMetadata {
            path: path.to_path_buf(),
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            file_type: path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default(),
            size: metadata.len(),
            modified: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
            summary: embed_text.chars().take(200).collect(),
        };

        // Store in vector store
        let entry = VectorEntry::new(path.to_string_lossy().to_string(), embedding, entry_meta);

        self.store
            .upsert(entry)
            .map_err(|e| IndexError::VectorError(e.to_string()))?;

        self.indexed_paths.insert(path.to_path_buf());
        self.stats.files_indexed += 1;

        Ok(true)
    }

    /// Index all files in a directory (non-recursive, for lazy indexing)
    pub fn index_directory(&mut self, dir: &Path) -> Result<usize, IndexError> {
        if !dir.is_dir() {
            return Err(IndexError::NotADirectory(dir.to_string_lossy().to_string()));
        }

        let mut indexed = 0;

        // Use WalkBuilder with max_depth=1 for non-recursive
        let walker = WalkBuilder::new(dir)
            .max_depth(Some(1))
            .hidden(!self.index_config.include_hidden)
            .git_ignore(self.index_config.respect_gitignore)
            .git_global(self.index_config.respect_gitignore)
            .git_exclude(self.index_config.respect_gitignore)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();
            if path.is_file() {
                match self.index_file(path) {
                    Ok(true) => indexed += 1,
                    Ok(false) => {}
                    Err(_) => self.stats.errors += 1,
                }
            }
        }

        self.stats.dirs_indexed += 1;
        Ok(indexed)
    }

    /// Index all files in a directory tree (recursive, respects .gitignore)
    pub fn index_tree(&mut self, dir: &Path) -> Result<usize, IndexError> {
        if !dir.is_dir() {
            return Err(IndexError::NotADirectory(dir.to_string_lossy().to_string()));
        }

        let mut indexed = 0;

        // Use WalkBuilder for gitignore-aware recursive walking
        let walker = WalkBuilder::new(dir)
            .hidden(!self.index_config.include_hidden)
            .git_ignore(self.index_config.respect_gitignore)
            .git_global(self.index_config.respect_gitignore)
            .git_exclude(self.index_config.respect_gitignore)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();

            if path.is_dir() {
                self.stats.dirs_indexed += 1;
                continue;
            }

            if path.is_file() {
                match self.index_file(path) {
                    Ok(true) => indexed += 1,
                    Ok(false) => {}
                    Err(_) => self.stats.errors += 1,
                }
            }
        }

        Ok(indexed)
    }

    /// Check if a path should be skipped (size-based only, gitignore handled by walker)
    fn should_skip(&self, path: &Path) -> bool {
        // Skip files larger than max size
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.len() > self.index_config.max_file_size {
                return true;
            }
        }
        false
    }

    /// Generate text to use for embedding
    fn generate_embed_text(
        &self,
        path: &Path,
        metadata: &fs::Metadata,
    ) -> Result<String, IndexError> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        // Always include name and path context
        let mut text = format!("File: {} ({})", name, ext);

        // Add parent directory context
        if let Some(parent) = path.parent() {
            if let Some(parent_name) = parent.file_name() {
                text.push_str(&format!(" in {}", parent_name.to_string_lossy()));
            }
        }

        // Add content for supported extensions and small files
        if self.index_config.content_extensions.contains(&ext)
            && metadata.len() <= self.index_config.max_file_size
        {
            if let Ok(content) = fs::read_to_string(path) {
                let preview: String = content
                    .chars()
                    .take(self.index_config.max_content_bytes)
                    .collect();
                text.push_str("\n\nContent:\n");
                text.push_str(&preview);
            }
        }

        Ok(text)
    }

    /// Generate embedding for text
    fn generate_embedding(&mut self, text: &str) -> Result<Vec<f32>, IndexError> {
        if !self.is_configured() {
            return Err(IndexError::NoApiKey);
        }

        let provider =
            create_embeddings_provider(self.config.clone()).map_err(IndexError::ApiError)?;

        let response = provider.embed(text).map_err(IndexError::ApiError)?;

        self.stats.tokens_used += response.tokens_used;

        Ok(response.embedding)
    }

    /// Clear the index
    pub fn clear(&mut self) {
        self.store.clear();
        self.indexed_paths.clear();
        self.stats = IndexStats::default();
    }

    /// Check if a file is indexed
    pub fn is_indexed(&self, path: &Path) -> bool {
        self.indexed_paths.contains(path)
    }
}

/// Errors from indexing operations
#[derive(Debug)]
pub enum IndexError {
    /// IO error
    IoError(String),
    /// Path is not a directory
    NotADirectory(String),
    /// No API key configured
    NoApiKey,
    /// API error
    ApiError(ApiError),
    /// Vector store error
    VectorError(String),
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::IoError(msg) => write!(f, "IO error: {}", msg),
            IndexError::NotADirectory(path) => write!(f, "Not a directory: {}", path),
            IndexError::NoApiKey => write!(f, "No API key configured"),
            IndexError::ApiError(e) => write!(f, "API error: {}", e),
            IndexError::VectorError(msg) => write!(f, "Vector store error: {}", msg),
        }
    }
}

impl std::error::Error for IndexError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IndexConfig::default();
        assert!(config.content_extensions.contains("rs"));
        assert!(config.content_extensions.contains("py"));
        assert!(config.respect_gitignore);
        assert!(config.include_hidden); // Hidden files included (gitignore handles .git)
    }

    #[test]
    fn test_indexer_creation() {
        let indexer = FileIndexer::new(AIApiConfig::default(), IndexConfig::default());
        assert!(!indexer.is_configured());
        assert_eq!(indexer.stats().files_indexed, 0);
    }

    #[test]
    fn test_should_skip_large_files() {
        let indexer = FileIndexer::new(AIApiConfig::default(), IndexConfig::default());
        // should_skip now only checks file size, gitignore handled by walker
        // Small files should not be skipped
        assert!(!indexer.should_skip(Path::new("Cargo.toml")));
    }

    #[test]
    fn test_generate_embed_text() {
        let indexer = FileIndexer::new(AIApiConfig::default(), IndexConfig::default());
        let path = Path::new("/project/src/main.rs");
        let metadata = fs::metadata(".").unwrap(); // Use cwd as dummy

        let text = indexer.generate_embed_text(path, &metadata).unwrap();
        assert!(text.contains("main.rs"));
        assert!(text.contains("rs"));
    }

    #[test]
    fn test_clear() {
        let mut indexer = FileIndexer::new(AIApiConfig::default(), IndexConfig::default());
        indexer.stats.files_indexed = 10;
        indexer.indexed_paths.insert(PathBuf::from("/test"));

        indexer.clear();
        assert_eq!(indexer.stats.files_indexed, 0);
        assert!(indexer.indexed_paths.is_empty());
    }
}
