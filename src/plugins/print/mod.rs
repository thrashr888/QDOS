//! Print Plugin for R-DOS
//!
//! Provides file printing functionality as a self-contained plugin.

use super::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
};
use crate::ui::components::ModalFrame;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
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

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        // Calculate centered modal area
        let popup_width = 50.min(area.width.saturating_sub(4));
        let popup_height = 8.min(area.height.saturating_sub(4));
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;
        let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Use ModalFrame for consistent styling
        let modal = ModalFrame::themed(modal_area, " PRINT FILE ", colors)
            .no_title_separator()
            .no_footer_separator();
        modal.render_frame(frame);

        // Content
        if let Some(ref error) = self.state.error {
            // Error message
            modal.render_row(
                frame,
                1,
                vec![Span::styled(
                    error.clone(),
                    Style::default().fg(colors.red()).bg(colors.bg()),
                )],
            );

            // Press any key message
            modal.render_row(
                frame,
                3,
                vec![Span::styled(
                    "Press any key to close",
                    Style::default().fg(colors.green()).bg(colors.bg()),
                )],
            );
        } else {
            // Print file prompt
            modal.render_row(
                frame,
                1,
                vec![
                    Span::styled(
                        "Print file: ",
                        Style::default().fg(colors.yellow()).bg(colors.bg()),
                    ),
                    Span::styled(
                        self.state.file_name.clone(),
                        Style::default().fg(colors.fg()).bg(colors.bg()),
                    ),
                ],
            );

            // Help line
            modal.render_help(frame, vec![("Y/Enter", "print"), ("N/Esc", "cancel")]);
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Print - Print Selected File".to_string(),
            "  Sends the selected file to the default printer".to_string(),
            "  Uses the 'lpr' command".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Print".to_string(),
            description: "Print file contents".to_string(),
            category: PluginCategory::Files,
            key: 'R',
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
