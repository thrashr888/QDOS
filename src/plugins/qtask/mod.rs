//! Q-TASK: TaskPaper Integration Plugin
//!
//! Treats .taskpaper files as interactive project management interfaces.
//! Features syntax highlighting, smart editing, folding, and filtering.

mod parser;
mod state;

pub use parser::{NodeType, TaskNode};
pub use state::{QTaskState, QTaskView};

use crate::app::ThemeColors;
use crate::plugins::{AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory};
use crate::ui::components::FullScreenView;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    Frame,
};
use std::any::Any;
use std::path::PathBuf;

/// Q-TASK plugin for TaskPaper file editing
pub struct QTaskPlugin {
    state: QTaskState,
    modal_open: bool,
}

impl QTaskPlugin {
    pub fn new() -> Self {
        Self {
            state: QTaskState::new(),
            modal_open: false,
        }
    }

    /// Open a TaskPaper file
    pub fn open_file(&mut self, path: PathBuf) -> Result<(), String> {
        self.state.load_file(path)?;
        self.modal_open = true;
        Ok(())
    }

    /// Check if plugin handles this file extension
    pub fn handles_extension(ext: &str) -> bool {
        ext.eq_ignore_ascii_case("taskpaper")
    }

    /// Check if modal is open
    pub fn is_modal_open(&self) -> bool {
        self.modal_open
    }

    /// Render a single node with syntax highlighting
    fn render_node<'a>(
        &self,
        node: &'a TaskNode,
        is_selected: bool,
        colors: &ThemeColors,
    ) -> Line<'a> {
        let mut spans = Vec::new();

        // Indent
        let indent = "  ".repeat(node.indent_level);
        spans.push(Span::raw(indent));

        // Fold indicator
        if node.folded {
            spans.push(Span::styled("▶ ", Style::default().fg(colors.grey())));
        } else if node.node_type == NodeType::Project {
            spans.push(Span::styled("▼ ", Style::default().fg(colors.grey())));
        }

        // Style based on node type
        let base_style = match node.node_type {
            NodeType::Project => Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
            NodeType::Task => {
                if node.is_done() {
                    Style::default()
                        .fg(colors.grey())
                        .add_modifier(Modifier::CROSSED_OUT)
                } else {
                    Style::default().fg(colors.fg())
                }
            }
            NodeType::Note => Style::default().fg(colors.grey()),
        };

        // Prefix for tasks
        if node.node_type == NodeType::Task {
            let checkbox = if node.is_done() { "[x] " } else { "[ ] " };
            spans.push(Span::styled(checkbox, base_style));
        }

        // Content with highlighted tags
        let content = &node.content;
        let mut last_end = 0;

        // Find and highlight tags
        for (i, c) in content.char_indices() {
            if c == '@' {
                // Add text before tag
                if i > last_end {
                    spans.push(Span::styled(&content[last_end..i], base_style));
                }

                // Find end of tag
                let tag_start = i;
                let mut tag_end = i + 1;
                let chars: Vec<char> = content[i + 1..].chars().collect();
                let mut j = 0;

                // Tag name
                while j < chars.len()
                    && (chars[j].is_alphanumeric() || chars[j] == '-' || chars[j] == '_')
                {
                    tag_end += chars[j].len_utf8();
                    j += 1;
                }

                // Tag value in parentheses
                if j < chars.len() && chars[j] == '(' {
                    tag_end += 1;
                    j += 1;
                    let mut depth = 1;
                    while j < chars.len() && depth > 0 {
                        if chars[j] == '(' {
                            depth += 1;
                        } else if chars[j] == ')' {
                            depth -= 1;
                        }
                        tag_end += chars[j].len_utf8();
                        j += 1;
                    }
                }

                // Style the tag
                let tag_text = &content[tag_start..tag_end];
                let tag_style = if tag_text.starts_with("@done") {
                    Style::default().fg(colors.green())
                } else if tag_text.starts_with("@today") || tag_text.starts_with("@priority") {
                    Style::default().fg(colors.yellow())
                } else if tag_text.starts_with("@due") {
                    Style::default().fg(colors.red())
                } else {
                    Style::default().fg(colors.cyan())
                };
                spans.push(Span::styled(tag_text.to_string(), tag_style));
                last_end = tag_end;
            }
        }

        // Add remaining text
        if last_end < content.len() {
            spans.push(Span::styled(&content[last_end..], base_style));
        }

        // Project suffix
        if node.node_type == NodeType::Project {
            spans.push(Span::styled(":", base_style));
        }

        // Selection highlight
        if is_selected {
            // Wrap all spans with selection background
            for span in &mut spans {
                let mut style = span.style;
                style = style.bg(colors.red());
                *span = Span::styled(span.content.clone(), style);
            }
        }

        Line::from(spans)
    }
}

impl Default for QTaskPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for QTaskPlugin {
    fn id(&self) -> &str {
        "qtask"
    }

    fn name(&self) -> &str {
        "Q-TASK"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_modal: true,
            ..Default::default()
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: "qtask".to_string(),
            name: "Q-TASK".to_string(),
            description: "TaskPaper project manager".to_string(),
            category: PluginCategory::Tools,
            key: 'T',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, selected_file: Option<&PathBuf>) -> Result<(), String> {
        // If a .taskpaper file is selected, open it
        if let Some(path) = selected_file {
            if Self::handles_extension(
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or_default(),
            ) {
                self.open_file(path.clone())?;
                return Ok(());
            }
        }

        // No valid file selected
        Err("Select a .taskpaper file first, then press V to view".to_string())
    }

    fn handle_global_key(
        &mut self,
        _key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        // Q-TASK doesn't have a global hotkey - it's opened via file handler
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Clear messages on any key
        self.state.message = None;
        self.state.error = None;

        match self.state.view {
            QTaskView::Document => self.handle_document_key(key),
            QTaskView::Filter => self.handle_filter_key(key),
            QTaskView::Help => self.handle_help_key(key),
            QTaskView::NewTask | QTaskView::NewProject | QTaskView::EditLine => {
                self.handle_edit_key(key)
            }
            QTaskView::ConfirmDelete => self.handle_delete_confirm_key(key),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let title = match &self.state.file_path {
            Some(p) => format!(
                " Q-TASK: {}{} ",
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Untitled".to_string()),
                if self.state.modified { " *" } else { "" }
            ),
            None => " Q-TASK ".to_string(),
        };

        let view = FullScreenView::new(area, &title, colors);
        view.render_frame(frame);

        // Get visible nodes
        let visible_nodes: Vec<_> = self
            .state
            .document
            .as_ref()
            .map(|d| d.visible_nodes())
            .unwrap_or_default();

        // Render nodes
        let content_height = view.content_height() as usize;
        let start = self.state.scroll_offset;
        let end = (start + content_height).min(visible_nodes.len());

        for (i, node) in visible_nodes[start..end].iter().enumerate() {
            let is_selected = start + i == self.state.selected_index;
            let line = self.render_node(node, is_selected, colors);
            view.render_row(frame, i as u16, line.spans);
        }

        // Render edit input or delete confirmation if in those modes
        match self.state.view {
            QTaskView::NewTask | QTaskView::NewProject | QTaskView::EditLine => {
                // Show input field at bottom of content area
                let prompt = match self.state.view {
                    QTaskView::NewTask => "New task: ",
                    QTaskView::NewProject => "New project: ",
                    QTaskView::EditLine => "Edit: ",
                    _ => "",
                };

                let row = content_height.saturating_sub(1) as u16;

                // Render input with cursor
                let mut spans = vec![Span::styled(prompt, Style::default().fg(colors.cyan()))];

                // Split buffer at cursor for visual cursor
                let before = &self.state.edit_buffer[..self.state.edit_cursor];
                let cursor_char = self
                    .state
                    .edit_buffer
                    .chars()
                    .nth(self.state.edit_cursor)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| " ".to_string());
                let after = if self.state.edit_cursor < self.state.edit_buffer.len() {
                    &self.state.edit_buffer[self.state.edit_cursor + 1..]
                } else {
                    ""
                };

                spans.push(Span::raw(before.to_string()));
                spans.push(Span::styled(
                    cursor_char,
                    Style::default().fg(colors.bg()).bg(colors.fg()),
                ));
                spans.push(Span::raw(after.to_string()));

                view.render_row(frame, row, spans);

                // Help for edit mode
                let help = vec![("Enter", "confirm"), ("Esc", "cancel")];
                view.render_help(frame, help);
            }
            QTaskView::ConfirmDelete => {
                // Show delete confirmation
                let row = content_height.saturating_sub(1) as u16;
                let spans = vec![
                    Span::styled("Delete this item? ", Style::default().fg(colors.red())),
                    Span::styled("(Y/N)", Style::default().fg(colors.yellow())),
                ];
                view.render_row(frame, row, spans);

                let help = vec![("Y", "delete"), ("N", "cancel")];
                view.render_help(frame, help);
            }
            QTaskView::Filter => {
                // Show filter input
                let row = content_height.saturating_sub(1) as u16;
                let spans = vec![
                    Span::styled("Filter by tag: @", Style::default().fg(colors.cyan())),
                    Span::raw(self.state.filter_text.clone()),
                    Span::styled("_", Style::default().fg(colors.fg()).bg(colors.fg())),
                ];
                view.render_row(frame, row, spans);

                let help = vec![("Enter", "apply"), ("Esc", "cancel")];
                view.render_help(frame, help);
            }
            _ => {
                // Status line with filter info
                let mut status_parts = Vec::new();
                if let Some(filter) = &self.state.active_filter {
                    status_parts.push(format!("Filter: @{}", filter));
                }
                status_parts.push(format!(
                    "{}/{}",
                    self.state.selected_index + 1,
                    visible_nodes.len()
                ));

                if let Some(msg) = &self.state.message {
                    status_parts.push(msg.clone());
                }
                if let Some(err) = &self.state.error {
                    status_parts.push(format!("Error: {}", err));
                }

                // Help footer for document mode
                let help = vec![
                    ("n", "new task"),
                    ("N", "new project"),
                    ("e", "edit"),
                    ("d", "delete"),
                    ("Space", "done"),
                    ("?", "help"),
                ];
                view.render_help(frame, help);
            }
        }
    }
}

impl QTaskPlugin {
    fn handle_document_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match (key.code, key.modifiers) {
            // Navigation
            (KeyCode::Up | KeyCode::Char('k'), _) => {
                self.state.move_up();
                KeyHandleResult::Handled
            }
            (KeyCode::Down | KeyCode::Char('j'), _) => {
                self.state.move_down();
                KeyHandleResult::Handled
            }
            (KeyCode::Home | KeyCode::Char('g'), _) => {
                self.state.selected_index = 0;
                self.state.scroll_offset = 0;
                KeyHandleResult::Handled
            }
            (KeyCode::End | KeyCode::Char('G'), _) => {
                let max = self.state.visible_count().saturating_sub(1);
                self.state.selected_index = max;
                KeyHandleResult::Handled
            }
            (KeyCode::PageUp, _) => {
                for _ in 0..10 {
                    self.state.move_up();
                }
                KeyHandleResult::Handled
            }
            (KeyCode::PageDown, _) => {
                for _ in 0..10 {
                    self.state.move_down();
                }
                KeyHandleResult::Handled
            }

            // Toggle done
            (KeyCode::Char(' '), _) => {
                self.state.toggle_done();
                KeyHandleResult::Handled
            }

            // Fold/unfold
            (KeyCode::Tab, _) => {
                self.state.toggle_fold();
                KeyHandleResult::Handled
            }

            // Filter
            (KeyCode::Char('/'), _) => {
                self.state.view = QTaskView::Filter;
                self.state.filter_text.clear();
                KeyHandleResult::Handled
            }

            // Clear filter
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.state.clear_filter();
                KeyHandleResult::Handled
            }

            // Save
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                if let Err(e) = self.state.save() {
                    self.state.error = Some(e);
                }
                KeyHandleResult::Handled
            }

            // Help
            (KeyCode::Char('?'), _) => {
                self.state.view = QTaskView::Help;
                KeyHandleResult::Handled
            }

            // New task
            (KeyCode::Char('n'), KeyModifiers::NONE) => {
                self.state.start_new_task();
                KeyHandleResult::Handled
            }

            // New project
            (KeyCode::Char('N'), KeyModifiers::SHIFT) => {
                self.state.start_new_project();
                KeyHandleResult::Handled
            }

            // Edit current line
            (KeyCode::Char('e'), KeyModifiers::NONE) | (KeyCode::Enter, _) => {
                self.state.start_edit();
                KeyHandleResult::Handled
            }

            // Delete
            (KeyCode::Char('d'), KeyModifiers::NONE) | (KeyCode::Delete, _) => {
                self.state.start_delete();
                KeyHandleResult::Handled
            }

            // Close
            (KeyCode::Esc, _) | (KeyCode::Char('q'), _) => {
                self.modal_open = false;
                KeyHandleResult::CloseModal
            }

            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = QTaskView::Document;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if !self.state.filter_text.is_empty() {
                    let filter = self.state.filter_text.clone();
                    self.state.apply_filter(&filter);
                }
                self.state.view = QTaskView::Document;
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.filter_text.pop();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.filter_text.push(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter => {
                self.state.view = QTaskView::Document;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.cancel_edit();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                match self.state.view {
                    QTaskView::NewTask => self.state.confirm_new_task(),
                    QTaskView::NewProject => self.state.confirm_new_project(),
                    QTaskView::EditLine => self.state.confirm_edit(),
                    _ => {}
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.backspace();
                KeyHandleResult::Handled
            }
            KeyCode::Delete => {
                self.state.delete_char();
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                self.state.cursor_left();
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                self.state.cursor_right();
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                self.state.cursor_home();
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                self.state.cursor_end();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.insert_char(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_delete_confirm_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.state.delete_selected();
                KeyHandleResult::Handled
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.state.view = QTaskView::Document;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handles_extension() {
        assert!(QTaskPlugin::handles_extension("taskpaper"));
        assert!(QTaskPlugin::handles_extension("TASKPAPER"));
        assert!(!QTaskPlugin::handles_extension("txt"));
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = QTaskPlugin::new();
        assert_eq!(plugin.id(), "qtask");
        assert_eq!(plugin.name(), "Q-TASK");
        assert!(!plugin.is_modal_open());
    }
}
