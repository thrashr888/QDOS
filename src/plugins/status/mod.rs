//! Status Plugin for R-DOS
//!
//! Provides system status display (F2 functionality) as a self-contained plugin.

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem};
use crate::ui::components::ModalFrame;
use crate::ui::{COLOR_BG, COLOR_FG, COLOR_GREEN, COLOR_YELLOW};
use crossterm::event::{KeyCode, KeyEvent};
use humansize::{format_size, DECIMAL};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;
use sysinfo::System;

/// System information cached by the plugin
#[derive(Debug, Clone, Default)]
pub struct SystemInfo {
    pub total_memory: u64,
    pub used_memory: u64,
    pub total_swap: u64,
    pub used_swap: u64,
    pub cpu_count: usize,
    pub os_name: String,
    pub os_version: String,
    pub hostname: String,
}

/// Status plugin state
#[derive(Debug, Clone, Default)]
pub struct StatusState {
    /// Cached system info
    pub info: SystemInfo,
    /// Plugin list (id, name, description)
    pub plugins: Vec<(String, String, String)>,
}

/// Status plugin that displays system information
pub struct StatusPlugin {
    /// Whether modal is open
    modal_open: bool,
    /// Current state
    state: StatusState,
}

impl StatusPlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: StatusState::default(),
        }
    }

    /// Refresh system information
    fn refresh_info(&mut self) {
        let mut sys = System::new_all();
        sys.refresh_all();

        self.state.info = SystemInfo {
            total_memory: sys.total_memory(),
            used_memory: sys.used_memory(),
            total_swap: sys.total_swap(),
            used_swap: sys.used_swap(),
            cpu_count: sys.cpus().len(),
            os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
            os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
            hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
        };
    }

    /// Set the plugin list (called by app when opening modal)
    pub fn set_plugins(&mut self, plugins: Vec<(String, String, String)>) {
        self.state.plugins = plugins;
    }

    /// Check if modal is open
    pub fn is_modal_open(&self) -> bool {
        self.modal_open
    }

    /// Open the modal
    pub fn open_modal(&mut self) {
        self.refresh_info();
        self.modal_open = true;
    }

    /// Close the modal
    pub fn close_modal(&mut self) {
        self.modal_open = false;
    }
}

impl Default for StatusPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for StatusPlugin {
    fn id(&self) -> &str {
        "status"
    }

    fn name(&self) -> &str {
        "System Status"
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
            name: "Status".to_string(),
            key: '2', // F2 key
            description: "Display system status information".to_string(),
            priority: 50, // After core features
        })
    }

    fn handle_global_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::F(2) => {
                self.open_modal();
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Any key closes the status modal
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') | KeyCode::F(2) => {
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
        // Calculate centered area (60% width, 50% height)
        let popup_width = (area.width as f32 * 0.6) as u16;
        let popup_height = (area.height as f32 * 0.5) as u16;
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;
        let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        let modal = ModalFrame::new(modal_area, " System Status ")
            .no_footer_separator();
        modal.render_frame(frame);

        let label_style = Style::default().fg(COLOR_GREEN).bg(COLOR_BG);
        let value_style = Style::default().fg(COLOR_FG).bg(COLOR_BG);
        let header_style = Style::default().fg(COLOR_YELLOW).bg(COLOR_BG);
        let dim_style = Style::default()
            .fg(COLOR_FG)
            .bg(COLOR_BG)
            .add_modifier(Modifier::DIM);

        let info = &self.state.info;

        // Render system info rows
        modal.render_row(
            frame,
            0,
            vec![
                Span::styled("Hostname: ", label_style),
                Span::styled(&info.hostname, value_style),
            ],
        );
        modal.render_row(
            frame,
            1,
            vec![
                Span::styled("OS: ", label_style),
                Span::styled(
                    format!("{} {}", info.os_name, info.os_version),
                    value_style,
                ),
            ],
        );
        modal.render_row(
            frame,
            2,
            vec![
                Span::styled("CPUs: ", label_style),
                Span::styled(format!("{}", info.cpu_count), value_style),
            ],
        );
        modal.render_row(frame, 3, vec![]);
        modal.render_row(
            frame,
            4,
            vec![
                Span::styled("Total Memory: ", label_style),
                Span::styled(format_size(info.total_memory, DECIMAL), value_style),
            ],
        );
        modal.render_row(
            frame,
            5,
            vec![
                Span::styled("Used Memory: ", label_style),
                Span::styled(format_size(info.used_memory, DECIMAL), value_style),
            ],
        );
        modal.render_row(
            frame,
            6,
            vec![
                Span::styled("Total Swap: ", label_style),
                Span::styled(format_size(info.total_swap, DECIMAL), value_style),
            ],
        );
        modal.render_row(
            frame,
            7,
            vec![
                Span::styled("Used Swap: ", label_style),
                Span::styled(format_size(info.used_swap, DECIMAL), value_style),
            ],
        );
        modal.render_row(frame, 8, vec![]);
        modal.render_row(
            frame,
            9,
            vec![Span::styled("Registered Plugins:", header_style)],
        );

        // Add registered plugins
        for (i, (id, name, description)) in self.state.plugins.iter().enumerate() {
            modal.render_row(
                frame,
                10 + i as u16,
                vec![
                    Span::styled(format!("  {} ", id), label_style),
                    Span::styled(format!("({}) ", name), value_style),
                    Span::styled(format!("- {}", description), dim_style),
                ],
            );
        }

        modal.render_help(frame, vec![("Any key", "close")]);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "F2 - Open System Status".to_string(),
            "  Shows hostname, OS, CPU, memory info".to_string(),
            "  Lists registered plugins".to_string(),
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
    fn test_status_plugin_creation() {
        let plugin = StatusPlugin::new();
        assert_eq!(plugin.id(), "status");
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_modal_open_close() {
        let mut plugin = StatusPlugin::new();
        plugin.open_modal();
        assert!(plugin.is_modal_open());
        plugin.close_modal();
        assert!(!plugin.is_modal_open());
    }
}
