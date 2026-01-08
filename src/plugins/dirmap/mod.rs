//! Directory Map Plugin for R-DOS
//!
//! Provides directory tree navigation (D key) as a self-contained plugin.

mod state;

pub use state::DirMapState;

use super::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
};
use crate::ui::components::FullScreenView;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::any::Any;
use std::fs;
use std::path::PathBuf;

/// Directory Map plugin for tree navigation
pub struct DirMapPlugin {
    /// Whether modal is open
    modal_open: bool,
    /// Directory map state
    state: Option<DirMapState>,
    /// Path to navigate to (set when user selects a directory)
    navigate_to: Option<PathBuf>,
    /// Last error message
    last_error: Option<String>,
}

impl DirMapPlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: None,
            navigate_to: None,
            last_error: None,
        }
    }

    /// Check if modal is open
    pub fn is_modal_open(&self) -> bool {
        self.modal_open
    }

    /// Open the modal with current path
    pub fn open_modal(&mut self, current_path: &PathBuf) {
        self.state = Some(DirMapState::new(current_path));
        self.navigate_to = None;
        self.last_error = None;
        self.modal_open = true;
    }

    /// Close the modal
    pub fn close_modal(&mut self) {
        self.modal_open = false;
        self.state = None;
        self.navigate_to = None;
    }

    /// Get path to navigate to (if any)
    pub fn take_navigate_path(&mut self) -> Option<PathBuf> {
        self.navigate_to.take()
    }

    /// Get last error (if any)
    pub fn take_error(&mut self) -> Option<String> {
        self.last_error.take()
    }
}

impl Default for DirMapPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for DirMapPlugin {
    fn id(&self) -> &str {
        "dirmap"
    }

    fn name(&self) -> &str {
        "Directory Map"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
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

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "DirMap".to_string(),
            key: 'D',
            description: "Show directory tree".to_string(),
            priority: 30, // After Help, Git, Beads
        })
    }

    fn handle_global_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        // D key opens directory map (but not lowercase d which might be delete)
        if key.code == KeyCode::Char('D') {
            self.open_modal(cwd);
            KeyHandleResult::OpenModal
        } else {
            KeyHandleResult::NotHandled
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref mut state) = self.state else {
            return KeyHandleResult::CloseModal;
        };

        // Handle delete confirmation mode
        if let Some(ref path_to_delete) = state.confirm_delete.clone() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => match fs::remove_dir(path_to_delete) {
                    Ok(()) => {
                        let parent_idx = if state.selected_index > 0 {
                            state.selected_index - 1
                        } else {
                            0
                        };
                        state.confirm_delete = None;
                        state.rebuild_flat_list();
                        state.selected_index =
                            parent_idx.min(state.flat_list.len().saturating_sub(1));
                    }
                    Err(e) => {
                        state.confirm_delete = None;
                        self.last_error = Some(format!("Cannot remove directory: {}", e));
                        self.close_modal();
                        return KeyHandleResult::CloseWithError(format!(
                            "Cannot remove directory: {}",
                            e
                        ));
                    }
                },
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    state.confirm_delete = None;
                }
                _ => {}
            }
            return KeyHandleResult::Handled;
        }

        // Handle input mode (for make directory)
        if state.input_mode.is_some() {
            match key.code {
                KeyCode::Enter => {
                    let dir_name = state.input_buffer.clone();
                    if !dir_name.is_empty() {
                        if let Some(parent_path) = state.selected_path() {
                            let new_dir = parent_path.join(&dir_name);
                            match fs::create_dir(&new_dir) {
                                Ok(()) => {
                                    state.input_mode = None;
                                    state.input_buffer.clear();
                                    // Reload children of selected node
                                    state.toggle_expand(state.selected_index);
                                    state.toggle_expand(state.selected_index);
                                }
                                Err(e) => {
                                    state.input_mode = None;
                                    state.input_buffer.clear();
                                    self.last_error =
                                        Some(format!("Failed to create directory: {}", e));
                                    self.close_modal();
                                    return KeyHandleResult::CloseWithError(format!(
                                        "Failed to create directory: {}",
                                        e
                                    ));
                                }
                            }
                        }
                    }
                    state.input_mode = None;
                    state.input_buffer.clear();
                }
                KeyCode::Esc => {
                    state.input_mode = None;
                    state.input_buffer.clear();
                }
                KeyCode::Backspace => {
                    state.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    state.input_buffer.push(c);
                }
                _ => {}
            }
            return KeyHandleResult::Handled;
        }

        // Normal navigation mode
        match key.code {
            KeyCode::Esc => {
                self.close_modal();
                KeyHandleResult::CloseModal
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_index > 0 {
                    state.selected_index -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_index + 1 < state.flat_list.len() {
                    state.selected_index += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                // Navigate to directory or expand
                if let Some((_, _, expanded, has_children)) =
                    state.flat_list.get(state.selected_index)
                {
                    if *has_children && !*expanded {
                        state.toggle_expand(state.selected_index);
                    } else if let Some(path) = state.selected_path() {
                        // Navigate to the selected directory
                        self.navigate_to = Some(path);
                        self.close_modal();
                        // Use special message format for navigation
                        return KeyHandleResult::CloseWithSuccess("dirmap:navigate".to_string());
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                // Collapse if expanded, otherwise go to parent
                if let Some((_, _, expanded, _)) = state.flat_list.get(state.selected_index) {
                    if *expanded {
                        state.toggle_expand(state.selected_index);
                    } else if state.selected_index > 0 {
                        // Find parent (look for item with depth - 1)
                        let current_depth = state
                            .flat_list
                            .get(state.selected_index)
                            .map(|(_, d, _, _)| *d)
                            .unwrap_or(0);
                        if current_depth > 0 {
                            for i in (0..state.selected_index).rev() {
                                if let Some((_, d, _, _)) = state.flat_list.get(i) {
                                    if *d < current_depth {
                                        state.selected_index = i;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                // Make directory mode
                state.input_mode = Some("New directory name".to_string());
                state.input_buffer.clear();
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') => {
                // Request delete confirmation (lowercase d only in modal)
                if let Some(path) = state.selected_path() {
                    // Don't allow deleting root
                    if path != state.root.path {
                        state.confirm_delete = Some(path);
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                state.selected_index = 0;
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                state.selected_index = state.flat_list.len().saturating_sub(1);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        let Some(ref state) = self.state else {
            return;
        };

        // Create full-screen view
        let view = FullScreenView::new(area, " DIRECTORY MAP - Tree View ", colors);
        view.render_frame(frame);

        // Tree content area
        let content_area = view.content_area();
        let visible_height = content_area.height as usize;

        // Calculate scroll position to keep selected item visible
        let tree_area = content_area;
        let scroll_offset = if state.selected_index >= visible_height {
            state.selected_index - visible_height + 1
        } else {
            0
        };

        // Render tree lines
        let mut lines: Vec<Line> = Vec::new();
        for (i, (path, depth, expanded, has_children)) in state
            .flat_list
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_height)
        {
            let is_selected = i == state.selected_index;

            // Build the tree line with indentation and expand/collapse indicator
            let indent = "  ".repeat(*depth);
            let indicator = if *has_children {
                if *expanded {
                    "▼ "
                } else {
                    "▶ "
                }
            } else {
                "  "
            };

            // Get the directory name (last component of path)
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());

            let line_text = format!("{}{}{}", indent, indicator, name);

            let style = if is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(colors.fg())
            };

            // Pad to full width for selection highlighting
            let padded = format!("{:<width$}", line_text, width = tree_area.width as usize);
            lines.push(Line::from(Span::styled(padded, style)));
        }

        frame.render_widget(Paragraph::new(lines), tree_area);

        // Help/input line - use render_footer for custom content
        if let Some(ref path) = state.confirm_delete {
            let dir_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());
            view.render_footer(
                frame,
                vec![Span::styled(
                    format!(" Delete '{}'? (Y)es / (N)o / ESC", dir_name),
                    Style::default().fg(colors.yellow()),
                )],
            );
        } else if let Some(ref mode) = state.input_mode {
            view.render_footer(
                frame,
                vec![Span::styled(
                    format!(" {}: {}█", mode, state.input_buffer),
                    Style::default().fg(colors.green()),
                )],
            );
        } else {
            view.render_help(
                frame,
                vec![
                    ("↑↓", "Navigate"),
                    ("Enter/→", "Expand"),
                    ("←/Backspace", "Collapse"),
                    ("M", "Make Dir"),
                    ("d", "Delete Dir"),
                    ("ESC", "Close"),
                ],
            );
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "D - Open Directory Map".to_string(),
            "  Navigate directory tree".to_string(),
            "  M: Create directory, d: Delete directory".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Dir Map".to_string(),
            description: "Directory tree view".to_string(),
            category: PluginCategory::Files,
            key: 'D',
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
    fn test_dirmap_plugin_creation() {
        let plugin = DirMapPlugin::new();
        assert_eq!(plugin.id(), "dirmap");
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_modal_open_close() {
        let mut plugin = DirMapPlugin::new();
        plugin.open_modal(&PathBuf::from("/tmp"));
        assert!(plugin.is_modal_open());
        plugin.close_modal();
        assert!(!plugin.is_modal_open());
    }
}
