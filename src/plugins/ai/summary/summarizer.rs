//! File summarizer implementation
//!
//! Uses LLM to generate concise summaries of file contents.

use crate::plugins::ai::api::{
    chat::{create_chat_provider, ChatMessage},
    AIApiConfig, ApiError,
};
use std::fs;
use std::path::Path;

/// A generated file summary
#[derive(Debug, Clone)]
pub struct FileSummary {
    /// Brief one-line description
    pub brief: String,
    /// Longer detailed summary
    pub detailed: String,
    /// Key elements found (functions, classes, etc.)
    pub key_elements: Vec<String>,
    /// File type/language detected
    pub file_type: String,
    /// Tokens used to generate summary
    pub tokens_used: u32,
}

impl Default for FileSummary {
    fn default() -> Self {
        Self {
            brief: String::new(),
            detailed: String::new(),
            key_elements: Vec::new(),
            file_type: String::new(),
            tokens_used: 0,
        }
    }
}

/// File summarizer using LLM
pub struct FileSummarizer {
    config: AIApiConfig,
    /// Maximum content bytes to include in prompt
    max_content_bytes: usize,
}

impl FileSummarizer {
    /// Create a new file summarizer
    pub fn new(config: AIApiConfig) -> Self {
        Self {
            config,
            max_content_bytes: 8192, // 8KB max
        }
    }

    /// Create using environment API keys
    pub fn from_env() -> Self {
        Self::new(AIApiConfig::from_env())
    }

    /// Check if summarizer is configured
    pub fn is_configured(&self) -> bool {
        self.config.is_configured()
    }

    /// Set max content bytes for prompts
    pub fn set_max_content(&mut self, bytes: usize) {
        self.max_content_bytes = bytes;
    }

    /// Summarize a file
    pub fn summarize(&self, path: &Path) -> Result<FileSummary, SummaryError> {
        if !self.is_configured() {
            return Err(SummaryError::NoApiKey);
        }

        // Read file content
        let content = self.read_file_content(path)?;
        let file_type = self.detect_file_type(path);

        // Generate summary using LLM
        self.generate_summary(&content, &file_type, path)
    }

    /// Read file content (limited to max_content_bytes)
    fn read_file_content(&self, path: &Path) -> Result<String, SummaryError> {
        let metadata = fs::metadata(path).map_err(|e| SummaryError::IoError(e.to_string()))?;

        if metadata.len() > 10 * 1024 * 1024 {
            // 10MB limit
            return Err(SummaryError::FileTooLarge(metadata.len()));
        }

        let content = fs::read_to_string(path).map_err(|e| SummaryError::IoError(e.to_string()))?;

        // Truncate to max_content_bytes
        Ok(content
            .chars()
            .take(self.max_content_bytes)
            .collect::<String>())
    }

    /// Detect file type from extension
    fn detect_file_type(&self, path: &Path) -> String {
        path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    }

    /// Generate summary using LLM
    fn generate_summary(
        &self,
        content: &str,
        file_type: &str,
        path: &Path,
    ) -> Result<FileSummary, SummaryError> {
        let provider =
            create_chat_provider(self.config.clone()).map_err(SummaryError::ApiError)?;

        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let system_prompt = r#"You are a code analyst. Summarize files concisely.
Output JSON with these fields:
{
  "brief": "One-line summary (max 80 chars)",
  "detailed": "2-3 sentence description of purpose and contents",
  "key_elements": ["list", "of", "important", "elements"],
  "file_type": "detected type/language"
}
For code files: list main functions, classes, or modules.
For config files: list key settings.
For text/docs: describe the content topic.
Output ONLY valid JSON."#;

        let user_prompt = format!(
            "Summarize this {} file named '{}':\n\n{}",
            if file_type.is_empty() {
                "unknown"
            } else {
                file_type
            },
            file_name,
            content
        );

        let messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ];

        let response = provider.complete(&messages).map_err(SummaryError::ApiError)?;

        // Parse the JSON response
        self.parse_summary_response(&response.content, response.input_tokens + response.output_tokens)
    }

    /// Parse LLM response into FileSummary
    fn parse_summary_response(
        &self,
        response: &str,
        tokens: u32,
    ) -> Result<FileSummary, SummaryError> {
        // Extract JSON from response
        let json_str = extract_json(response);

        let parsed: SummaryResponse = serde_json::from_str(json_str)
            .map_err(|e| SummaryError::ParseError(format!("Failed to parse summary: {}", e)))?;

        Ok(FileSummary {
            brief: parsed.brief,
            detailed: parsed.detailed,
            key_elements: parsed.key_elements.unwrap_or_default(),
            file_type: parsed.file_type.unwrap_or_default(),
            tokens_used: tokens,
        })
    }
}

/// Response structure for JSON parsing
#[derive(Debug, serde::Deserialize)]
struct SummaryResponse {
    brief: String,
    detailed: String,
    key_elements: Option<Vec<String>>,
    file_type: Option<String>,
}

/// Extract JSON from response that might have markdown code blocks
fn extract_json(response: &str) -> &str {
    // Look for ```json ... ``` blocks
    if let Some(start) = response.find("```json") {
        let after_marker = &response[start + 7..];
        if let Some(end) = after_marker.find("```") {
            return after_marker[..end].trim();
        }
    }

    // Look for ``` ... ``` blocks
    if let Some(start) = response.find("```") {
        let after_marker = &response[start + 3..];
        if let Some(end) = after_marker.find("```") {
            return after_marker[..end].trim();
        }
    }

    // Look for { ... } directly
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            return &response[start..=end];
        }
    }

    response.trim()
}

/// Errors from summary operations
#[derive(Debug)]
pub enum SummaryError {
    /// No API key configured
    NoApiKey,
    /// IO error reading file
    IoError(String),
    /// File too large
    FileTooLarge(u64),
    /// API error
    ApiError(ApiError),
    /// Failed to parse response
    ParseError(String),
}

impl std::fmt::Display for SummaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SummaryError::NoApiKey => write!(f, "No API key configured"),
            SummaryError::IoError(msg) => write!(f, "IO error: {}", msg),
            SummaryError::FileTooLarge(size) => write!(f, "File too large: {} bytes", size),
            SummaryError::ApiError(e) => write!(f, "API error: {}", e),
            SummaryError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for SummaryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarizer_creation() {
        let summarizer = FileSummarizer::from_env();
        // Will be unconfigured without API key
        assert_eq!(summarizer.max_content_bytes, 8192);
    }

    #[test]
    fn test_detect_file_type() {
        let summarizer = FileSummarizer::new(AIApiConfig::default());
        assert_eq!(
            summarizer.detect_file_type(Path::new("/test/file.rs")),
            "rs"
        );
        assert_eq!(
            summarizer.detect_file_type(Path::new("/test/file.py")),
            "py"
        );
        assert_eq!(
            summarizer.detect_file_type(Path::new("/test/file")),
            ""
        );
    }

    #[test]
    fn test_extract_json_code_block() {
        let response = "Here's the summary:\n```json\n{\"brief\": \"Test\"}\n```";
        assert_eq!(extract_json(response), "{\"brief\": \"Test\"}");
    }

    #[test]
    fn test_extract_json_plain() {
        let response = "{\"brief\": \"Test\", \"detailed\": \"Details\"}";
        assert_eq!(
            extract_json(response),
            "{\"brief\": \"Test\", \"detailed\": \"Details\"}"
        );
    }

    #[test]
    fn test_parse_summary_response() {
        let summarizer = FileSummarizer::new(AIApiConfig::default());
        let json = r#"{"brief": "Test file", "detailed": "A test file for testing", "key_elements": ["test"], "file_type": "txt"}"#;

        let result = summarizer.parse_summary_response(json, 100);
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert_eq!(summary.brief, "Test file");
        assert_eq!(summary.detailed, "A test file for testing");
        assert_eq!(summary.key_elements, vec!["test"]);
        assert_eq!(summary.file_type, "txt");
        assert_eq!(summary.tokens_used, 100);
    }
}
