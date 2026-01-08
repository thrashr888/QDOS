//! File Operations Plugin for R-DOS
//!
//! Provides file operation modals (Copy, Move, Erase, Rename) as a self-contained plugin.
//! The actual file operations are executed by the app; this plugin manages the UI.

pub mod modal;

use super::{AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory};
use crate::ui::components::ModalFrame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;
use std::any::Any;
use std::fs;
use std::path::PathBuf;

/// File operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOperation {
    Copy,
    Move,
    Erase,
    Rename,
}

impl FileOperation {
    pub fn name(&self) -> &'static str {
        match self {
            FileOperation::Copy => "Copy",
            FileOperation::Move => "Move",
            FileOperation::Erase => "Erase",
            FileOperation::Rename => "Rename",
        }
    }

    pub fn verb(&self) -> &'static str {
        match self {
            FileOperation::Copy => "Copy",
            FileOperation::Move => "Move",
            FileOperation::Erase => "Erase",
            FileOperation::Rename => "Rename",
        }
    }

    pub fn prompt(&self) -> &'static str {
        match self {
            FileOperation::Copy => "Copy to:",
            FileOperation::Move => "Move to:",
            FileOperation::Erase => "Erase file(s)?",
            FileOperation::Rename => "New name:",
        }
    }
}

/// File operation state
#[derive(Debug, Clone)]
pub struct FileOpsState {
    /// The operation type
    pub operation: FileOperation,
    /// Files to operate on
    pub files: Vec<PathBuf>,
    /// Input buffer (for destination path or new name)
    pub input: String,
    /// Whether operation is confirmed
    pub confirmed: bool,
}

impl FileOpsState {
    pub fn new(operation: FileOperation, files: Vec<PathBuf>, default_input: String) -> Self {
        Self {
            operation,
            files,
            input: default_input,
            confirmed: false,
        }
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn first_file_name(&self) -> String {
        self.files
            .first()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    }
}

/// File operations result
#[derive(Debug, Clone)]
pub struct FileOpsResult {
    pub operation: FileOperation,
    pub files: Vec<PathBuf>,
    pub destination: Option<String>,
}

/// File Operations plugin
pub struct FileOpsPlugin {
    /// Whether modal is open
    modal_open: bool,
    /// Current operation state
    state: Option<FileOpsState>,
    /// Result (set when operation is confirmed)
    result: Option<FileOpsResult>,
}

impl FileOpsPlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: None,
            result: None,
        }
    }

    /// Check if modal is open
    pub fn is_modal_open(&self) -> bool {
        self.modal_open
    }

    /// Open modal for a file operation
    pub fn open_modal(
        &mut self,
        operation: FileOperation,
        files: Vec<PathBuf>,
        default_input: String,
    ) {
        self.state = Some(FileOpsState::new(operation, files, default_input));
        self.result = None;
        self.modal_open = true;
    }

    /// Close the modal
    pub fn close_modal(&mut self) {
        self.modal_open = false;
        self.state = None;
    }

    /// Take the result (if operation was confirmed)
    pub fn take_result(&mut self) -> Option<FileOpsResult> {
        self.result.take()
    }

    /// Tab completion for paths
    fn tab_complete(partial: &str) -> Option<String> {
        let path = PathBuf::from(partial);

        // Determine the directory to search and the prefix to match
        let (search_dir, prefix) = if partial.ends_with('/') || partial.ends_with('\\') {
            (path.clone(), String::new())
        } else if let Some(parent) = path.parent() {
            let file_name = path.file_name()?.to_string_lossy().to_string();
            (parent.to_path_buf(), file_name)
        } else {
            (PathBuf::from("."), partial.to_string())
        };

        // Read directory and find matches (directories only for copy/move destination)
        let entries = fs::read_dir(&search_dir).ok()?;
        let mut matches: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let full_path = search_dir.join(&name);
                // Only match directories for destination paths
                if full_path.is_dir() && name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    let mut result = full_path.to_string_lossy().to_string();
                    if !result.ends_with('/') {
                        result.push('/');
                    }
                    Some(result)
                } else {
                    None
                }
            })
            .collect();

        matches.sort();

        // Return first match, or find common prefix if multiple matches
        if matches.len() == 1 {
            Some(matches.remove(0))
        } else if matches.len() > 1 {
            // Find common prefix among all matches
            let first = &matches[0];
            let mut common_len = first.len();
            for m in &matches[1..] {
                common_len = first
                    .chars()
                    .zip(m.chars())
                    .take_while(|(a, b)| a == b)
                    .count()
                    .min(common_len);
            }
            if common_len > partial.len() {
                Some(first[..common_len].to_string())
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Default for FileOpsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for FileOpsPlugin {
    fn id(&self) -> &str {
        "fileops"
    }

    fn name(&self) -> &str {
        "File Operations"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: false, // Operations are triggered from main nav menu
            has_keys: false, // Keys are handled by main app nav
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn init(&mut self, _cwd: &PathBuf) -> Result<(), String> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true
    }

    fn handle_modal_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        _cwd: &PathBuf,
    ) -> KeyHandleResult {
        use crossterm::event::KeyCode;

        let Some(ref mut state) = self.state else {
            return KeyHandleResult::CloseModal;
        };

        match state.operation {
            FileOperation::Erase => {
                // Erase is just Y/N confirmation
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.result = Some(FileOpsResult {
                            operation: FileOperation::Erase,
                            files: state.files.clone(),
                            destination: None,
                        });
                        self.close_modal();
                        KeyHandleResult::CloseWithSuccess("fileops:erase".to_string())
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.close_modal();
                        KeyHandleResult::CloseModal
                    }
                    _ => KeyHandleResult::Handled,
                }
            }
            FileOperation::Copy | FileOperation::Move | FileOperation::Rename => {
                // These need text input
                match key.code {
                    KeyCode::Enter => {
                        if !state.input.is_empty() {
                            let msg = match state.operation {
                                FileOperation::Copy => "fileops:copy",
                                FileOperation::Move => "fileops:move",
                                FileOperation::Rename => "fileops:rename",
                                _ => "fileops:done",
                            };
                            self.result = Some(FileOpsResult {
                                operation: state.operation,
                                files: state.files.clone(),
                                destination: Some(state.input.clone()),
                            });
                            self.close_modal();
                            KeyHandleResult::CloseWithSuccess(msg.to_string())
                        } else {
                            KeyHandleResult::Handled
                        }
                    }
                    KeyCode::Esc => {
                        self.close_modal();
                        KeyHandleResult::CloseModal
                    }
                    KeyCode::Backspace => {
                        state.input.pop();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Tab => {
                        // Tab completion for Copy/Move destination paths
                        if matches!(state.operation, FileOperation::Copy | FileOperation::Move) {
                            if let Some(completed) = Self::tab_complete(&state.input) {
                                state.input = completed;
                            }
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char(c) => {
                        state.input.push(c);
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                }
            }
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        let Some(ref state) = self.state else {
            return;
        };

        // Calculate modal size
        let popup_width = 60.min(area.width.saturating_sub(4));
        let popup_height = match state.operation {
            FileOperation::Erase => 8,
            _ => 10,
        }
        .min(area.height.saturating_sub(4));
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;
        let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        let title = format!(" {} ", state.operation.name());
        let modal = ModalFrame::themed(modal_area, &title, colors)
            .no_title_separator()
            .no_footer_separator();
        modal.render_frame(frame);

        let label_style = Style::default().fg(colors.green()).bg(colors.bg());
        let input_style = Style::default().fg(colors.yellow()).bg(colors.red());
        let warning_style = Style::default()
            .fg(colors.red())
            .bg(colors.bg())
            .add_modifier(Modifier::BOLD);

        match state.operation {
            FileOperation::Erase => {
                // Erase confirmation
                let count = state.file_count();
                let file_text = if count == 1 {
                    format!("\"{}\"", state.first_file_name())
                } else {
                    format!("{} files", count)
                };

                modal.render_row(
                    frame,
                    1,
                    vec![Span::styled(
                        format!("Erase {}?", file_text),
                        Style::default().fg(colors.yellow()).bg(colors.bg()),
                    )],
                );
                modal.render_row(frame, 2, vec![]);
                modal.render_row(
                    frame,
                    3,
                    vec![Span::styled("This action cannot be undone!", warning_style)],
                );

                modal.render_help(frame, vec![("Y/N", "confirm or cancel")]);
            }
            _ => {
                // Copy/Move/Rename with input
                let count = state.file_count();
                let source_text = if count == 1 {
                    format!("\"{}\"", state.first_file_name())
                } else {
                    format!("{} files", count)
                };

                modal.render_row(
                    frame,
                    1,
                    vec![Span::styled(
                        format!("{} {}", state.operation.verb(), source_text),
                        label_style,
                    )],
                );
                modal.render_row(frame, 2, vec![]);
                modal.render_row(
                    frame,
                    3,
                    vec![
                        Span::styled(format!("{} ", state.operation.prompt()), label_style),
                        Span::styled(format!("{}█", state.input), input_style),
                    ],
                );

                // Show Tab hint for Copy/Move (path completion), not for Rename
                let help = if matches!(state.operation, FileOperation::Copy | FileOperation::Move) {
                    vec![("Tab", "complete"), ("Enter", "confirm"), ("ESC", "cancel")]
                } else {
                    vec![("Enter", "confirm"), ("ESC", "cancel")]
                };
                modal.render_help(frame, help);
            }
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "File Operations".to_string(),
            "  Copy - Copy files to another location".to_string(),
            "  Move - Move files to another location".to_string(),
            "  Erase - Delete files".to_string(),
            "  Rename - Rename files".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "File Ops".to_string(),
            description: "Copy, move, delete files".to_string(),
            category: PluginCategory::Files,
            key: 'O',
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fileops_plugin_creation() {
        let plugin = FileOpsPlugin::new();
        assert_eq!(plugin.id(), "fileops");
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_modal_open_close() {
        let mut plugin = FileOpsPlugin::new();
        plugin.open_modal(
            FileOperation::Copy,
            vec![PathBuf::from("/tmp/test.txt")],
            "/tmp/dest/".to_string(),
        );
        assert!(plugin.is_modal_open());
        plugin.close_modal();
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_erase_operation() {
        let state = FileOpsState::new(
            FileOperation::Erase,
            vec![PathBuf::from("/tmp/test.txt")],
            String::new(),
        );
        assert_eq!(state.operation, FileOperation::Erase);
        assert_eq!(state.file_count(), 1);
    }
}
