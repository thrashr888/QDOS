//! TaskPaper format parser
//!
//! Parses TaskPaper files into a tree of TaskNodes.
//! Format reference: https://www.taskpaper.com/guide/

use chrono::Local;
use std::collections::HashMap;

/// Type of node in the TaskPaper document
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    /// A project (ends with colon)
    Project,
    /// A task (starts with dash)
    Task,
    /// A note (plain text, no special prefix)
    Note,
}

/// A tag with optional value: @tag or @tag(value)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    pub value: Option<String>,
}

impl Tag {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: None,
        }
    }

    pub fn with_value(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: Some(value.into()),
        }
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Some(v) => write!(f, "@{}({})", self.name, v),
            None => write!(f, "@{}", self.name),
        }
    }
}

/// A single node in the TaskPaper document
#[derive(Debug, Clone)]
pub struct TaskNode {
    /// Line number in the original document (0-indexed)
    pub line_number: usize,
    /// Indentation level (number of tabs)
    pub indent_level: usize,
    /// Type of node
    pub node_type: NodeType,
    /// The text content (without prefix/suffix)
    pub content: String,
    /// Original full line text
    pub raw_line: String,
    /// Tags extracted from the line
    pub tags: Vec<Tag>,
    /// Whether this node is folded (children hidden)
    pub folded: bool,
    /// Whether this node is visible (not filtered out)
    pub visible: bool,
}

impl TaskNode {
    /// Check if this task is marked as done
    pub fn is_done(&self) -> bool {
        self.tags.iter().any(|t| t.name == "done")
    }

    /// Get the @done tag's date value if present
    pub fn done_date(&self) -> Option<&str> {
        self.tags
            .iter()
            .find(|t| t.name == "done")
            .and_then(|t| t.value.as_deref())
    }

    /// Check if node has a specific tag
    pub fn has_tag(&self, name: &str) -> bool {
        self.tags.iter().any(|t| t.name == name)
    }

    /// Get tag value if present
    pub fn get_tag_value(&self, name: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|t| t.name == name)
            .and_then(|t| t.value.as_deref())
    }
}

/// Parsed TaskPaper document
#[derive(Debug, Clone)]
pub struct TaskPaperDocument {
    /// All nodes in document order
    pub nodes: Vec<TaskNode>,
    /// Map from line number to node index
    line_to_index: HashMap<usize, usize>,
}

impl TaskPaperDocument {
    /// Parse a TaskPaper document from text
    pub fn parse(content: &str) -> Self {
        let mut nodes = Vec::new();
        let mut line_to_index = HashMap::new();

        for (line_number, line) in content.lines().enumerate() {
            let node = parse_line(line_number, line);
            line_to_index.insert(line_number, nodes.len());
            nodes.push(node);
        }

        Self {
            nodes,
            line_to_index,
        }
    }

    /// Get node by line number
    pub fn get_by_line(&self, line_number: usize) -> Option<&TaskNode> {
        self.line_to_index
            .get(&line_number)
            .and_then(|idx| self.nodes.get(*idx))
    }

    /// Get mutable node by line number
    pub fn get_by_line_mut(&mut self, line_number: usize) -> Option<&mut TaskNode> {
        self.line_to_index
            .get(&line_number)
            .and_then(|idx| self.nodes.get_mut(*idx))
    }

    /// Toggle @done tag on a task
    pub fn toggle_done(&mut self, line_number: usize) -> Option<String> {
        let idx = *self.line_to_index.get(&line_number)?;
        let node = self.nodes.get_mut(idx)?;

        if node.node_type != NodeType::Task {
            return None;
        }

        let new_line = if node.is_done() {
            // Remove @done tag
            remove_done_tag(&node.raw_line)
        } else {
            // Add @done tag with today's date
            let today = Local::now().format("%Y-%m-%d").to_string();
            add_done_tag(&node.raw_line, &today)
        };

        // Re-parse the modified line
        let new_node = parse_line(line_number, &new_line);
        self.nodes[idx] = new_node;

        Some(new_line)
    }

    /// Get all visible nodes (respecting fold state)
    pub fn visible_nodes(&self) -> Vec<&TaskNode> {
        let mut result = Vec::new();
        let mut skip_until_indent: Option<usize> = None;

        for node in &self.nodes {
            // Skip children of folded nodes
            if let Some(indent) = skip_until_indent {
                if node.indent_level > indent {
                    continue;
                } else {
                    skip_until_indent = None;
                }
            }

            if !node.visible {
                continue;
            }

            result.push(node);

            // If this node is folded, skip its children
            if node.folded {
                skip_until_indent = Some(node.indent_level);
            }
        }

        result
    }

    /// Serialize document back to string (for saving)
    pub fn serialize(&self) -> String {
        self.nodes
            .iter()
            .map(|n| n.raw_line.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Filter nodes by tag
    pub fn filter_by_tag(&mut self, tag_name: &str) {
        for node in &mut self.nodes {
            node.visible = node.has_tag(tag_name);
        }
    }

    /// Show all nodes
    pub fn show_all(&mut self) {
        for node in &mut self.nodes {
            node.visible = true;
        }
    }

    /// Toggle fold state of a node
    pub fn toggle_fold(&mut self, line_number: usize) {
        if let Some(idx) = self.line_to_index.get(&line_number) {
            if let Some(node) = self.nodes.get_mut(*idx) {
                node.folded = !node.folded;
            }
        }
    }
}

/// Parse a single line into a TaskNode
fn parse_line(line_number: usize, line: &str) -> TaskNode {
    let raw_line = line.to_string();

    // Count leading tabs for indent level
    let indent_level = line.chars().take_while(|c| *c == '\t').count();
    let trimmed = line.trim_start_matches('\t');

    // Determine node type and extract content
    let (node_type, content) = if trimmed.ends_with(':') && !trimmed.contains('@') {
        // Project: ends with colon (but not a tag value)
        (NodeType::Project, trimmed.trim_end_matches(':').to_string())
    } else if trimmed.starts_with("- ") {
        // Task: starts with dash
        (NodeType::Task, trimmed[2..].to_string())
    } else if trimmed.starts_with('-') && trimmed.len() > 1 {
        // Task without space after dash
        (NodeType::Task, trimmed[1..].trim_start().to_string())
    } else {
        // Note: everything else
        (NodeType::Note, trimmed.to_string())
    };

    // Extract tags
    let tags = parse_tags(&content);

    TaskNode {
        line_number,
        indent_level,
        node_type,
        content,
        raw_line,
        tags,
        folded: false,
        visible: true,
    }
}

/// Parse tags from content string
fn parse_tags(content: &str) -> Vec<Tag> {
    let mut tags = Vec::new();
    let mut in_tag = false;
    let mut tag_name = String::new();
    let mut tag_value = String::new();
    let mut in_value = false;
    let mut paren_depth = 0;

    for c in content.chars() {
        if !in_tag {
            if c == '@' {
                in_tag = true;
                tag_name.clear();
                tag_value.clear();
                in_value = false;
            }
        } else if in_value {
            if c == '(' {
                paren_depth += 1;
                tag_value.push(c);
            } else if c == ')' {
                if paren_depth > 0 {
                    paren_depth -= 1;
                    tag_value.push(c);
                } else {
                    // End of tag value
                    tags.push(Tag::with_value(&tag_name, &tag_value));
                    in_tag = false;
                    in_value = false;
                }
            } else {
                tag_value.push(c);
            }
        } else if c == '(' {
            in_value = true;
        } else if c.is_alphanumeric() || c == '-' || c == '_' {
            tag_name.push(c);
        } else {
            // End of tag name
            if !tag_name.is_empty() {
                tags.push(Tag::new(&tag_name));
            }
            in_tag = false;
            // Check if this is start of new tag
            if c == '@' {
                in_tag = true;
                tag_name.clear();
            }
        }
    }

    // Handle tag at end of string
    if in_tag && !tag_name.is_empty() {
        if in_value {
            tags.push(Tag::with_value(&tag_name, &tag_value));
        } else {
            tags.push(Tag::new(&tag_name));
        }
    }

    tags
}

/// Add @done tag with date to a line
fn add_done_tag(line: &str, date: &str) -> String {
    let trimmed = line.trim_end();
    format!("{} @done({})", trimmed, date)
}

/// Remove @done tag from a line
fn remove_done_tag(line: &str) -> String {
    // Match @done or @done(...)
    let mut result = String::new();
    let mut chars = line.chars().peekable();
    let mut skip_until_close = false;
    let mut _at_done_start = 0;

    while let Some(c) = chars.next() {
        if skip_until_close {
            if c == ')' {
                skip_until_close = false;
            }
            continue;
        }

        if c == '@' {
            // Check if this is @done
            let remaining: String = std::iter::once(c).chain(chars.clone()).collect();
            if remaining.starts_with("@done") {
                // Skip @done
                for _ in 0..4 {
                    chars.next();
                }
                // Check for (value)
                if chars.peek() == Some(&'(') {
                    skip_until_close = true;
                    chars.next(); // skip '('
                }
                // Skip trailing space before @done
                if result.ends_with(' ') {
                    result.pop();
                }
                continue;
            }
        }
        result.push(c);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_project() {
        let node = parse_line(0, "My Project:");
        assert_eq!(node.node_type, NodeType::Project);
        assert_eq!(node.content, "My Project");
        assert_eq!(node.indent_level, 0);
    }

    #[test]
    fn test_parse_task() {
        let node = parse_line(0, "\t- Buy milk @today");
        assert_eq!(node.node_type, NodeType::Task);
        assert_eq!(node.content, "Buy milk @today");
        assert_eq!(node.indent_level, 1);
        assert!(node.has_tag("today"));
    }

    #[test]
    fn test_parse_task_with_done() {
        let node = parse_line(0, "- Done task @done(2024-01-15)");
        assert!(node.is_done());
        assert_eq!(node.done_date(), Some("2024-01-15"));
    }

    #[test]
    fn test_parse_note() {
        let node = parse_line(0, "\t\tThis is a note");
        assert_eq!(node.node_type, NodeType::Note);
        assert_eq!(node.indent_level, 2);
    }

    #[test]
    fn test_parse_tags() {
        let tags = parse_tags("Task @priority(high) @today @due(2024-01-20)");
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].name, "priority");
        assert_eq!(tags[0].value, Some("high".to_string()));
        assert_eq!(tags[1].name, "today");
        assert_eq!(tags[1].value, None);
    }

    #[test]
    fn test_toggle_done() {
        let mut doc = TaskPaperDocument::parse("- Task 1\n- Task 2 @done(2024-01-01)");

        // Add done to first task
        let new_line = doc.toggle_done(0).unwrap();
        assert!(new_line.contains("@done("));

        // Remove done from second task
        let new_line = doc.toggle_done(1).unwrap();
        assert!(!new_line.contains("@done"));
    }

    #[test]
    fn test_remove_done_tag() {
        assert_eq!(remove_done_tag("- Task @done"), "- Task");
        assert_eq!(remove_done_tag("- Task @done(2024-01-15)"), "- Task");
        assert_eq!(
            remove_done_tag("- Task @priority(high) @done(2024-01-15) @today"),
            "- Task @priority(high) @today"
        );
    }
}
