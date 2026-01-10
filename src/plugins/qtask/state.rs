//! Q-TASK plugin state

use super::parser::TaskPaperDocument;
use std::path::PathBuf;

/// Current view/mode of the plugin
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum QTaskView {
    /// Viewing/editing the document
    #[default]
    Document,
    /// Filter input mode
    Filter,
    /// Help overlay
    Help,
}

/// State for the Q-TASK plugin
#[derive(Debug, Default)]
pub struct QTaskState {
    /// Current view
    pub view: QTaskView,
    /// Path to the current file
    pub file_path: Option<PathBuf>,
    /// Parsed document
    pub document: Option<TaskPaperDocument>,
    /// Currently selected line index (in visible nodes)
    pub selected_index: usize,
    /// Scroll offset for display
    pub scroll_offset: usize,
    /// Filter input text
    pub filter_text: String,
    /// Active filter (if any)
    pub active_filter: Option<String>,
    /// Whether document has unsaved changes
    pub modified: bool,
    /// Error message to display
    pub error: Option<String>,
    /// Success message to display
    pub message: Option<String>,
}

impl QTaskState {
    /// Create a new state
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a file
    pub fn load_file(&mut self, path: PathBuf) -> Result<(), String> {
        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;

        self.document = Some(TaskPaperDocument::parse(&content));
        self.file_path = Some(path);
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.modified = false;
        self.active_filter = None;
        self.error = None;

        Ok(())
    }

    /// Save the current file
    pub fn save(&mut self) -> Result<(), String> {
        let path = self.file_path.as_ref().ok_or("No file loaded")?;
        let doc = self.document.as_ref().ok_or("No document loaded")?;

        let content = doc.serialize();
        std::fs::write(path, content).map_err(|e| format!("Failed to save: {}", e))?;

        self.modified = false;
        self.message = Some("Saved".to_string());

        Ok(())
    }

    /// Get visible node count
    pub fn visible_count(&self) -> usize {
        self.document
            .as_ref()
            .map(|d| d.visible_nodes().len())
            .unwrap_or(0)
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.ensure_visible();
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let max = self.visible_count().saturating_sub(1);
        if self.selected_index < max {
            self.selected_index += 1;
            self.ensure_visible();
        }
    }

    /// Ensure selected item is visible
    fn ensure_visible(&mut self) {
        // Assume ~20 visible lines
        let visible_lines = 18;
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_lines {
            self.scroll_offset = self.selected_index - visible_lines + 1;
        }
    }

    /// Toggle done on selected task
    pub fn toggle_done(&mut self) {
        if let Some(doc) = &mut self.document {
            let visible = doc.visible_nodes();
            if let Some(node) = visible.get(self.selected_index) {
                let line_num = node.line_number;
                if doc.toggle_done(line_num).is_some() {
                    self.modified = true;
                }
            }
        }
    }

    /// Toggle fold on selected node
    pub fn toggle_fold(&mut self) {
        if let Some(doc) = &mut self.document {
            let visible = doc.visible_nodes();
            if let Some(node) = visible.get(self.selected_index) {
                let line_num = node.line_number;
                doc.toggle_fold(line_num);
            }
        }
    }

    /// Apply tag filter
    pub fn apply_filter(&mut self, tag: &str) {
        if let Some(doc) = &mut self.document {
            doc.filter_by_tag(tag);
            self.active_filter = Some(tag.to_string());
            self.selected_index = 0;
            self.scroll_offset = 0;
        }
    }

    /// Clear filter
    pub fn clear_filter(&mut self) {
        if let Some(doc) = &mut self.document {
            doc.show_all();
            self.active_filter = None;
        }
    }

    /// Get selected node's line number
    pub fn selected_line_number(&self) -> Option<usize> {
        self.document.as_ref().and_then(|doc| {
            doc.visible_nodes()
                .get(self.selected_index)
                .map(|n| n.line_number)
        })
    }
}
