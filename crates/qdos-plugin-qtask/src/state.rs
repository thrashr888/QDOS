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
    /// Adding a new task
    NewTask,
    /// Adding a new project
    NewProject,
    /// Editing current line
    EditLine,
    /// Confirm delete
    ConfirmDelete,
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
    /// Edit buffer for new/edit modes
    pub edit_buffer: String,
    /// Cursor position in edit buffer
    pub edit_cursor: usize,
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

    /// Fold (collapse) selected node
    pub fn fold(&mut self) {
        if let Some(doc) = &mut self.document {
            let visible = doc.visible_nodes();
            if let Some(node) = visible.get(self.selected_index) {
                let line_num = node.line_number;
                doc.set_folded(line_num, true);
            }
        }
    }

    /// Unfold (expand) selected node
    pub fn unfold(&mut self) {
        if let Some(doc) = &mut self.document {
            let visible = doc.visible_nodes();
            if let Some(node) = visible.get(self.selected_index) {
                let line_num = node.line_number;
                doc.set_folded(line_num, false);
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

    /// Start new task mode
    pub fn start_new_task(&mut self) {
        self.edit_buffer.clear();
        self.edit_cursor = 0;
        self.view = QTaskView::NewTask;
    }

    /// Start new project mode
    pub fn start_new_project(&mut self) {
        self.edit_buffer.clear();
        self.edit_cursor = 0;
        self.view = QTaskView::NewProject;
    }

    /// Start edit mode for current line
    pub fn start_edit(&mut self) {
        if let Some(doc) = &self.document {
            let visible = doc.visible_nodes();
            if let Some(node) = visible.get(self.selected_index) {
                // Load current content into buffer
                self.edit_buffer = node.content.clone();
                self.edit_cursor = self.edit_buffer.len();
                self.view = QTaskView::EditLine;
            }
        }
    }

    /// Start delete confirmation
    pub fn start_delete(&mut self) {
        self.view = QTaskView::ConfirmDelete;
    }

    /// Cancel edit mode
    pub fn cancel_edit(&mut self) {
        self.edit_buffer.clear();
        self.edit_cursor = 0;
        self.view = QTaskView::Document;
    }

    /// Get the indent level for new items (based on current selection)
    fn get_insert_indent(&self) -> usize {
        self.document
            .as_ref()
            .and_then(|doc| {
                doc.visible_nodes()
                    .get(self.selected_index)
                    .map(|n| n.indent_level)
            })
            .unwrap_or(0)
    }

    /// Get the line number to insert after
    fn get_insert_line(&self) -> usize {
        self.document
            .as_ref()
            .and_then(|doc| {
                doc.visible_nodes()
                    .get(self.selected_index)
                    .map(|n| n.line_number)
            })
            .unwrap_or(0)
    }

    /// Confirm adding new task
    pub fn confirm_new_task(&mut self) {
        if self.edit_buffer.is_empty() {
            self.cancel_edit();
            return;
        }

        let indent = self.get_insert_indent();
        let after_line = self.get_insert_line();
        let task_line = format!("{}- {}", "\t".repeat(indent), self.edit_buffer);

        if let Some(doc) = &mut self.document {
            doc.insert_line(after_line + 1, &task_line);
            self.modified = true;
            self.selected_index += 1;
        }

        self.cancel_edit();
    }

    /// Confirm adding new project
    pub fn confirm_new_project(&mut self) {
        if self.edit_buffer.is_empty() {
            self.cancel_edit();
            return;
        }

        let after_line = self.get_insert_line();
        let project_line = format!("{}:", self.edit_buffer);

        if let Some(doc) = &mut self.document {
            // Add blank line before project if not at start
            if after_line > 0 {
                doc.insert_line(after_line + 1, "");
                doc.insert_line(after_line + 2, &project_line);
            } else {
                doc.insert_line(after_line + 1, &project_line);
            }
            self.modified = true;
        }

        self.cancel_edit();
    }

    /// Confirm editing current line
    pub fn confirm_edit(&mut self) {
        if let Some(line_num) = self.selected_line_number() {
            if let Some(doc) = &mut self.document {
                doc.update_content(line_num, &self.edit_buffer);
                self.modified = true;
            }
        }
        self.cancel_edit();
    }

    /// Delete selected item
    pub fn delete_selected(&mut self) {
        if let Some(line_num) = self.selected_line_number() {
            if let Some(doc) = &mut self.document {
                doc.delete_line(line_num);
                self.modified = true;
                // Adjust selection if needed
                let max = self.visible_count().saturating_sub(1);
                if self.selected_index > max {
                    self.selected_index = max;
                }
            }
        }
        self.view = QTaskView::Document;
    }

    /// Insert character at cursor
    pub fn insert_char(&mut self, c: char) {
        self.edit_buffer.insert(self.edit_cursor, c);
        self.edit_cursor += 1;
    }

    /// Delete character before cursor
    pub fn backspace(&mut self) {
        if self.edit_cursor > 0 {
            self.edit_cursor -= 1;
            self.edit_buffer.remove(self.edit_cursor);
        }
    }

    /// Delete character at cursor
    pub fn delete_char(&mut self) {
        if self.edit_cursor < self.edit_buffer.len() {
            self.edit_buffer.remove(self.edit_cursor);
        }
    }

    /// Move cursor left
    pub fn cursor_left(&mut self) {
        if self.edit_cursor > 0 {
            self.edit_cursor -= 1;
        }
    }

    /// Move cursor right
    pub fn cursor_right(&mut self) {
        if self.edit_cursor < self.edit_buffer.len() {
            self.edit_cursor += 1;
        }
    }

    /// Move cursor to start
    pub fn cursor_home(&mut self) {
        self.edit_cursor = 0;
    }

    /// Move cursor to end
    pub fn cursor_end(&mut self) {
        self.edit_cursor = self.edit_buffer.len();
    }
}
