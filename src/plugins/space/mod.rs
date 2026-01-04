//! Space Plugin for R-DOS
//!
//! Provides disk space display functionality as a self-contained plugin.

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem};
use crate::ui::components::ModalFrame;
use crate::ui::{COLOR_BG, COLOR_CYAN, COLOR_GREEN, COLOR_YELLOW};
use crossterm::event::{KeyCode, KeyEvent};
use humansize::{format_size, DECIMAL};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;
use sysinfo::Disks;

/// Space plugin state
#[derive(Debug, Clone, Default)]
pub struct SpaceState {
    /// Disk name
    pub disk_name: String,
    /// Total space in bytes
    pub total: u64,
    /// Available space in bytes
    pub available: u64,
    /// Current path for context
    pub current_path: PathBuf,
}

/// Space plugin that displays disk space information
pub struct SpacePlugin {
    /// Whether modal is open
    modal_open: bool,
    /// Current state
    state: SpaceState,
}

impl SpacePlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: SpaceState::default(),
        }
    }

    /// Refresh disk space information for the given path
    fn refresh_info(&mut self, path: &PathBuf) {
        self.state.current_path = path.clone();

        // Get disk name from path
        self.state.disk_name = path
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());

        // Get disk space
        let disks = Disks::new_with_refreshed_list();
        for disk in disks.iter() {
            let mount_point = disk.mount_point();
            if path.starts_with(mount_point) {
                self.state.total = disk.total_space();
                self.state.available = disk.available_space();
                return;
            }
        }

        // Fallback - use root disk
        if let Some(disk) = disks.iter().next() {
            self.state.total = disk.total_space();
            self.state.available = disk.available_space();
        }
    }

    /// Check if modal is open
    pub fn is_modal_open(&self) -> bool {
        self.modal_open
    }

    /// Open the modal
    pub fn open_modal(&mut self, path: &PathBuf) {
        self.refresh_info(path);
        self.modal_open = true;
    }

    /// Close the modal
    pub fn close_modal(&mut self) {
        self.modal_open = false;
    }

    /// Format size in short form (e.g., "1.5 GB")
    fn format_size_short(bytes: u64) -> String {
        format_size(bytes, DECIMAL)
    }
}

impl Default for SpacePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for SpacePlugin {
    fn id(&self) -> &str {
        "space"
    }

    fn name(&self) -> &str {
        "Disk Space"
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
        true // Always available
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Space".to_string(),
            key: 'S',
            description: "Show disk space usage".to_string(),
            priority: 60, // After Status
        })
    }

    fn handle_global_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            // F11 or 'S' for Space - using a function key to avoid conflict with 's' sort
            KeyCode::F(11) => {
                self.open_modal(cwd);
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Any key closes the space modal
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') | KeyCode::F(11) => {
                self.close_modal();
                KeyHandleResult::CloseModal
            }
            _ => {
                self.close_modal();
                KeyHandleResult::CloseModal
            }
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect) {
        // Calculate centered modal area
        let popup_width = 50.min(area.width.saturating_sub(4));
        let popup_height = 12.min(area.height.saturating_sub(4));
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;
        let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        let title = format!(" Space On Disk {} ", self.state.disk_name);
        let modal = ModalFrame::new(modal_area, &title).no_footer_separator();
        modal.render_frame(frame);

        let used = self.state.total.saturating_sub(self.state.available);
        let used_percent = if self.state.total > 0 {
            used as f64 / self.state.total as f64 * 100.0
        } else {
            0.0
        };

        let label_style = Style::default().fg(COLOR_YELLOW).bg(COLOR_BG);
        let value_style = Style::default().fg(COLOR_CYAN).bg(COLOR_BG);

        // Content rows
        modal.render_row(frame, 0, vec![]); // Empty row
        modal.render_row(
            frame,
            1,
            vec![
                Span::styled("Total space:      ", label_style),
                Span::styled(Self::format_size_short(self.state.total), value_style),
            ],
        );
        modal.render_row(frame, 2, vec![]); // Empty row
        modal.render_row(
            frame,
            3,
            vec![
                Span::styled("Total used:       ", label_style),
                Span::styled(
                    format!("{} ({:.1}%)", Self::format_size_short(used), used_percent),
                    value_style,
                ),
            ],
        );
        modal.render_row(frame, 4, vec![]); // Empty row
        modal.render_row(
            frame,
            5,
            vec![
                Span::styled("Total available:  ", label_style),
                Span::styled(Self::format_size_short(self.state.available), value_style),
            ],
        );
        modal.render_row(frame, 6, vec![]); // Empty row
        modal.render_row(
            frame,
            7,
            vec![Span::styled(
                "Press any key to continue",
                Style::default().fg(COLOR_GREEN).bg(COLOR_BG),
            )],
        );
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "F11 - Show Disk Space".to_string(),
            "  Displays total, used, and available disk space".to_string(),
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
    fn test_space_plugin_creation() {
        let plugin = SpacePlugin::new();
        assert_eq!(plugin.id(), "space");
        assert!(!plugin.is_modal_open());
    }
}
