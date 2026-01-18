//! Natural language command parser
//!
//! Uses LLM to parse natural language into structured file operations.

use crate::api::{
    chat::{create_chat_provider, ChatMessage},
    AIApiConfig, ApiError,
};
use serde::{Deserialize, Serialize};

/// Supported command actions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandAction {
    /// Copy file(s) to destination
    Copy,
    /// Move file(s) to destination
    Move,
    /// Delete file(s)
    Delete,
    /// Rename a file
    Rename,
    /// Create a new file
    CreateFile,
    /// Create a new directory
    CreateDir,
    /// Find files matching a pattern
    Find,
    /// Search file contents
    Search,
    /// View/open a file
    View,
    /// Edit a file
    Edit,
    /// Show file info/properties
    Info,
    /// List directory contents
    List,
    /// Change directory
    ChangeDir,
    /// Sort files by criteria
    Sort,
    /// Unknown/unparseable command
    Unknown,
}

impl CommandAction {
    /// Check if this action is destructive (requires confirmation)
    pub fn is_destructive(&self) -> bool {
        matches!(self, CommandAction::Delete | CommandAction::Move)
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            CommandAction::Copy => "Copy files",
            CommandAction::Move => "Move files",
            CommandAction::Delete => "Delete files",
            CommandAction::Rename => "Rename file",
            CommandAction::CreateFile => "Create file",
            CommandAction::CreateDir => "Create directory",
            CommandAction::Find => "Find files",
            CommandAction::Search => "Search contents",
            CommandAction::View => "View file",
            CommandAction::Edit => "Edit file",
            CommandAction::Info => "File info",
            CommandAction::List => "List directory",
            CommandAction::ChangeDir => "Change directory",
            CommandAction::Sort => "Sort files",
            CommandAction::Unknown => "Unknown command",
        }
    }
}

/// A parsed command with action and parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    /// The action to perform
    pub action: CommandAction,
    /// Target file(s) or pattern
    pub targets: Vec<String>,
    /// Destination path (for copy/move/rename)
    pub destination: Option<String>,
    /// Search pattern or filter
    pub pattern: Option<String>,
    /// Original natural language input
    pub original: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Human-readable explanation
    pub explanation: String,
}

impl ParsedCommand {
    /// Create an unknown/failed parse result
    pub fn unknown(original: String, reason: String) -> Self {
        Self {
            action: CommandAction::Unknown,
            targets: vec![],
            destination: None,
            pattern: None,
            original,
            confidence: 0.0,
            explanation: reason,
        }
    }
}

/// Natural language command parser
pub struct CommandParser {
    config: AIApiConfig,
}

impl CommandParser {
    /// Create a new parser with API configuration
    pub fn new(config: AIApiConfig) -> Self {
        Self { config }
    }

    /// Create parser using environment variables for API keys
    pub fn from_env() -> Self {
        Self::new(AIApiConfig::from_env())
    }

    /// Check if parser is configured (has API key)
    pub fn is_configured(&self) -> bool {
        self.config.is_configured()
    }

    /// Parse a natural language command
    pub fn parse(&self, input: &str) -> Result<ParsedCommand, ApiError> {
        if !self.is_configured() {
            return Ok(ParsedCommand::unknown(
                input.to_string(),
                "No API key configured".to_string(),
            ));
        }

        let provider = create_chat_provider(self.config.clone())?;

        let system_prompt = SYSTEM_PROMPT;
        let user_prompt = format!("Parse this command: {}", input);

        let messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ];

        let response = provider.complete(&messages)?;

        // Parse the JSON response
        self.parse_response(&response.content, input)
    }

    /// Parse the LLM response into a ParsedCommand
    fn parse_response(&self, response: &str, original: &str) -> Result<ParsedCommand, ApiError> {
        // Try to extract JSON from the response
        let json_str = extract_json(response);

        let parsed: ParsedCommandResponse = serde_json::from_str(json_str)
            .map_err(|e| ApiError::ParseError(format!("Failed to parse command: {}", e)))?;

        Ok(ParsedCommand {
            action: parsed.action,
            targets: parsed.targets.unwrap_or_default(),
            destination: parsed.destination,
            pattern: parsed.pattern,
            original: original.to_string(),
            confidence: parsed.confidence.unwrap_or(0.5),
            explanation: parsed.explanation.unwrap_or_default(),
        })
    }
}

/// Internal response structure for JSON parsing
#[derive(Debug, Deserialize)]
struct ParsedCommandResponse {
    action: CommandAction,
    targets: Option<Vec<String>>,
    destination: Option<String>,
    pattern: Option<String>,
    confidence: Option<f32>,
    explanation: Option<String>,
}

/// Extract JSON from a response that might have markdown code blocks
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

const SYSTEM_PROMPT: &str = r#"You are a file manager command parser. Parse natural language commands into structured JSON.

Output ONLY valid JSON with these fields:
{
  "action": "copy|move|delete|rename|create_file|create_dir|find|search|view|edit|info|list|change_dir|sort|unknown",
  "targets": ["file1", "file2"],
  "destination": "/path/to/dest",
  "pattern": "*.txt",
  "confidence": 0.9,
  "explanation": "Brief description of what will happen"
}

Rules:
- action is required, others are optional based on command type
- targets: list of files/directories affected
- destination: where to copy/move/rename to
- pattern: glob pattern or search term
- confidence: 0.0-1.0 how confident you are in the parse
- explanation: human-readable description

Examples:
"copy *.txt to backup" -> {"action": "copy", "pattern": "*.txt", "destination": "backup", "confidence": 0.95, "explanation": "Copy all .txt files to backup directory"}
"delete old logs" -> {"action": "delete", "pattern": "*log*", "confidence": 0.7, "explanation": "Delete files containing 'log' in name"}
"find large files" -> {"action": "find", "pattern": "*", "confidence": 0.8, "explanation": "Find large files (size filter will be applied)"}
"rename foo.txt to bar.txt" -> {"action": "rename", "targets": ["foo.txt"], "destination": "bar.txt", "confidence": 0.95, "explanation": "Rename foo.txt to bar.txt"}
"show me what's in documents" -> {"action": "list", "targets": ["documents"], "confidence": 0.9, "explanation": "List contents of documents directory"}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_action_is_destructive() {
        assert!(CommandAction::Delete.is_destructive());
        assert!(CommandAction::Move.is_destructive());
        assert!(!CommandAction::Copy.is_destructive());
        assert!(!CommandAction::View.is_destructive());
    }

    #[test]
    fn test_extract_json_code_block() {
        let response = "Here's the result:\n```json\n{\"action\": \"copy\"}\n```";
        assert_eq!(extract_json(response), "{\"action\": \"copy\"}");
    }

    #[test]
    fn test_extract_json_plain() {
        let response = "{\"action\": \"delete\"}";
        assert_eq!(extract_json(response), "{\"action\": \"delete\"}");
    }

    #[test]
    fn test_extract_json_with_text() {
        let response = "The command is: {\"action\": \"move\"} done.";
        assert_eq!(extract_json(response), "{\"action\": \"move\"}");
    }

    #[test]
    fn test_parsed_command_unknown() {
        let cmd = ParsedCommand::unknown("gibberish".to_string(), "Could not parse".to_string());
        assert_eq!(cmd.action, CommandAction::Unknown);
        assert_eq!(cmd.confidence, 0.0);
    }

    #[test]
    fn test_parse_response() {
        let parser = CommandParser::new(AIApiConfig::default());
        let json = r#"{"action": "copy", "targets": ["file.txt"], "destination": "backup", "confidence": 0.9, "explanation": "Copy file"}"#;

        let result = parser.parse_response(json, "copy file.txt to backup");
        assert!(result.is_ok());

        let cmd = result.unwrap();
        assert_eq!(cmd.action, CommandAction::Copy);
        assert_eq!(cmd.targets, vec!["file.txt"]);
        assert_eq!(cmd.destination, Some("backup".to_string()));
    }
}
