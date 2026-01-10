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

        // Help footer
        let help = vec![
            ("Space", "toggle done"),
            ("Tab", "fold"),
            ("/", "filter"),
            ("Ctrl+S", "save"),
            ("?", "help"),
            ("Esc", "close"),
        ];
        view.render_help(frame, help);
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
