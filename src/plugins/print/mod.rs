//! Print Plugin for R-DOS
//!
//! Provides file printing functionality as a self-contained plugin.

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem};
use crate::ui::{COLOR_BG, COLOR_BLUE, COLOR_FG, COLOR_GREEN, COLOR_RED, COLOR_YELLOW};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Clear;
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
        // Calculate centered modal area
        let popup_width = 50.min(area.width.saturating_sub(4));
        let popup_height = 8.min(area.height.saturating_sub(4));
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;
        let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Clear the modal area
        frame.render_widget(Clear, modal_area);

        let buf = frame.buffer_mut();
        let style_border = Style::default().fg(COLOR_BLUE).bg(COLOR_BG);
        let style_title = Style::default().fg(COLOR_YELLOW).bg(COLOR_BLUE);
        let style_bg = Style::default().fg(COLOR_FG).bg(COLOR_BG);
        let w = popup_width as usize;

        // Draw double-line border box
        // Top border
        let top = format!("╔{}╗", "═".repeat(w.saturating_sub(2)));
        buf.set_string(popup_x, popup_y, &top, style_border);

        // Title (centered on top border)
        let title = " Print File ";
        let title_x = popup_x + (popup_width.saturating_sub(title.len() as u16)) / 2;
        buf.set_string(title_x, popup_y, title, style_title);

        // Middle rows with side borders
        for row in 1..(popup_height - 1) {
            let y = popup_y + row;
            buf.set_string(popup_x, y, "║", style_border);
            let fill = " ".repeat(w.saturating_sub(2));
            buf.set_string(popup_x + 1, y, &fill, style_bg);
            buf.set_string(popup_x + popup_width - 1, y, "║", style_border);
        }

        // Bottom border
        let bottom = format!("╚{}╝", "═".repeat(w.saturating_sub(2)));
        buf.set_string(popup_x, popup_y + popup_height - 1, &bottom, style_border);

        // Content
        if let Some(ref error) = self.state.error {
            // Error message
            let error_style = Style::default().fg(COLOR_RED).bg(COLOR_BG);
            let error_x = popup_x + 2;
            buf.set_string(error_x, popup_y + 2, error, error_style);

            // Press any key message
            let help = "Press any key to close";
            let help_style = Style::default().fg(COLOR_GREEN).bg(COLOR_BG);
            buf.set_string(error_x, popup_y + 4, help, help_style);
        } else {
            // Print file prompt
            let label_style = Style::default().fg(COLOR_YELLOW).bg(COLOR_BG);
            let file_style = Style::default().fg(COLOR_FG).bg(COLOR_BG);
            let content_x = popup_x + 2;
            buf.set_string(content_x, popup_y + 2, "Print file: ", label_style);
            buf.set_string(content_x + 12, popup_y + 2, &self.state.file_name, file_style);

            // Help line
            let help = "Y/Enter: Print  N/Esc: Cancel";
            let help_style = Style::default().fg(COLOR_GREEN).bg(COLOR_BG);
            buf.set_string(content_x, popup_y + 4, help, help_style);
        }
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
