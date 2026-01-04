//! File Operations Plugin for R-DOS
//!
//! Provides file operation modals (Copy, Move, Erase, Rename) as a self-contained plugin.
//! The actual file operations are executed by the app; this plugin manages the UI.

use super::{KeyHandleResult, Plugin, PluginCapabilities};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;

// Colors (DOS-style)
const COLOR_BG: Color = Color::Black;
const COLOR_FG: Color = Color::White;
const COLOR_BLUE: Color = Color::Rgb(0x55, 0x55, 0xFF);
const COLOR_GREEN: Color = Color::Rgb(0x55, 0xFF, 0x55);
const COLOR_YELLOW: Color = Color::Rgb(0xFF, 0xFF, 0x55);
const COLOR_RED: Color = Color::Rgb(0xFF, 0x55, 0x55);

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
                    KeyCode::Char(c) => {
                        state.input.push(c);
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                }
            }
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect) {
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

        // Clear the modal area
        frame.render_widget(Clear, modal_area);

        let title = format!(" {} ", state.operation.name());

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BLUE))
            .style(Style::default().bg(COLOR_BG));

        let mut lines = vec![Line::from("")];

        match state.operation {
            FileOperation::Erase => {
                // Erase confirmation
                let count = state.file_count();
                let file_text = if count == 1 {
                    format!("\"{}\"", state.first_file_name())
                } else {
                    format!("{} files", count)
                };

                lines.push(Line::from(Span::styled(
                    format!("Erase {}?", file_text),
                    Style::default().fg(COLOR_YELLOW),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "This action cannot be undone!",
                    Style::default().fg(COLOR_RED).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Y", Style::default().fg(COLOR_GREEN)),
                    Span::raw("/"),
                    Span::styled("N", Style::default().fg(COLOR_GREEN)),
                    Span::raw(" to confirm or cancel"),
                ]));
            }
            _ => {
                // Copy/Move/Rename with input
                let count = state.file_count();
                let source_text = if count == 1 {
                    format!("\"{}\"", state.first_file_name())
                } else {
                    format!("{} files", count)
                };

                lines.push(Line::from(Span::styled(
                    format!("{} {}", state.operation.verb(), source_text),
                    Style::default().fg(COLOR_GREEN),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{} ", state.operation.prompt()),
                        Style::default().fg(COLOR_BLUE),
                    ),
                    Span::styled(
                        format!("{}█", state.input),
                        Style::default().fg(COLOR_YELLOW).bg(COLOR_RED),
                    ),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
                    Span::raw(" confirm  "),
                    Span::styled("ESC", Style::default().fg(COLOR_BLUE)),
                    Span::raw(" cancel"),
                ]));
            }
        }

        let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, modal_area);
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
