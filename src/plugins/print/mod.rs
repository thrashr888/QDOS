//! Print Plugin for R-DOS
//!
//! Provides file printing functionality as a self-contained plugin.

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;
use std::process::Command;

/// Print plugin state
#[derive(Debug, Clone, Default)]
pub struct PrintState {
    /// File path to print
    pub file_path: Option<PathBuf>,
    /// File name for display
    pub file_name: String,
    /// Error message if file can't be printed
    pub error: Option<String>,
}

/// Print plugin that handles file printing
pub struct PrintPlugin {
    /// Whether modal is open
    modal_open: bool,
    /// Print state
    state: PrintState,
}

impl PrintPlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: PrintState::default(),
        }
    }

    /// Check if modal is open
    pub fn is_modal_open(&self) -> bool {
        self.modal_open
    }

    /// Open the modal with a file to print
    pub fn open_modal(&mut self, file_path: PathBuf, file_name: String) {
        self.state = PrintState {
            file_path: Some(file_path),
            file_name,
            error: None,
        };
        self.modal_open = true;
    }

    /// Open the modal with an error
    pub fn open_modal_error(&mut self, error: String) {
        self.state = PrintState {
            file_path: None,
            file_name: String::new(),
            error: Some(error),
        };
        self.modal_open = true;
    }

    /// Close the modal
    pub fn close_modal(&mut self) {
        self.modal_open = false;
        self.state = PrintState::default();
    }

    /// Execute the print command
    fn execute_print(&self) -> Result<(), String> {
        let Some(ref path) = self.state.file_path else {
            return Err("No file to print".to_string());
        };

        match Command::new("lpr").arg(path).spawn() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to print: {}", e)),
        }
    }
}

impl Default for PrintPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for PrintPlugin {
    fn id(&self) -> &str {
        "print"
    }

    fn name(&self) -> &str {
        "Print"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: false, // No global key - triggered via menu
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
        true // Always available
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Print".to_string(),
            key: 'P',
            description: "Print selected file".to_string(),
            priority: 70, // After Theme
        })
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // If there's an error, any key closes
        if self.state.error.is_some() {
            self.close_modal();
            return KeyHandleResult::CloseModal;
        }

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                // Execute print
                match self.execute_print() {
                    Ok(()) => {
                        let name = self.state.file_name.clone();
                        self.close_modal();
                        KeyHandleResult::CloseWithSuccess(format!("Sent {} to printer", name))
                    }
                    Err(e) => {
                        self.close_modal();
                        KeyHandleResult::CloseWithError(e)
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.close_modal();
                KeyHandleResult::CloseModal
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect) {
        // Colors
        let bg = Color::Black;
        let blue = Color::Rgb(0x55, 0x55, 0xFF);
        let yellow = Color::Rgb(0xFF, 0xFF, 0x55);
        let green = Color::Rgb(0x55, 0xFF, 0x55);
        let red = Color::Rgb(0xFF, 0x55, 0x55);
        let white = Color::White;

        // Calculate centered modal area
        let popup_width = 50.min(area.width.saturating_sub(4));
        let popup_height = 8.min(area.height.saturating_sub(4));
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;
        let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Clear the modal area
        frame.render_widget(Clear, modal_area);

        let block = Block::default()
            .title(" Print File ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(blue))
            .style(Style::default().bg(bg));

        let lines = if let Some(ref error) = self.state.error {
            vec![
                Line::from(""),
                Line::from(Span::styled(error, Style::default().fg(red))),
                Line::from(""),
                Line::from(Span::styled(
                    "Press any key to close",
                    Style::default().fg(green),
                )),
            ]
        } else {
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("Print file: ", Style::default().fg(yellow)),
                    Span::styled(&self.state.file_name, Style::default().fg(white)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Y/Enter: Print  N/Esc: Cancel",
                    Style::default().fg(green),
                )),
            ]
        };

        let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, modal_area);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Print - Print Selected File".to_string(),
            "  Sends the selected file to the default printer".to_string(),
            "  Uses the 'lpr' command".to_string(),
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
    fn test_print_plugin_creation() {
        let plugin = PrintPlugin::new();
        assert_eq!(plugin.id(), "print");
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_modal_open_close() {
        let mut plugin = PrintPlugin::new();
        plugin.open_modal(PathBuf::from("/test/file.txt"), "file.txt".to_string());
        assert!(plugin.is_modal_open());
        assert_eq!(plugin.state.file_name, "file.txt");
        plugin.close_modal();
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_error_modal() {
        let mut plugin = PrintPlugin::new();
        plugin.open_modal_error("Cannot print directory".to_string());
        assert!(plugin.is_modal_open());
        assert!(plugin.state.error.is_some());
    }
}
