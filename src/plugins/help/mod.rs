//! Help Plugin for R-DOS
//!
//! Provides help system functionality (F1) as a self-contained plugin.

mod state;

pub use state::HelpState;

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem};
use crate::ui::components::ModalFrame;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;

/// Help plugin that displays help topics
pub struct HelpPlugin {
    /// Whether modal is open
    modal_open: bool,
    /// Help state
    state: HelpState,
}

impl HelpPlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: HelpState::new(),
        }
    }

    /// Check if modal is open
    pub fn is_modal_open(&self) -> bool {
        self.modal_open
    }

    /// Open the modal
    pub fn open_modal(&mut self) {
        self.state.current_topic = 0;
        self.state.scroll_offset = 0;
        self.modal_open = true;
    }

    /// Close the modal
    pub fn close_modal(&mut self) {
        self.modal_open = false;
    }

    /// Get visible lines for current view height
    fn visible_lines(&self, height: usize) -> usize {
        height.saturating_sub(4) // Account for border and padding
    }

    /// Get content lines for current topic
    fn current_content_lines(&self) -> Vec<&str> {
        if self.state.current_topic == 0 {
            // Index page - show all topics
            vec![] // Index is handled differently
        } else {
            // Topic page
            let topic_idx = self.state.current_topic - 1;
            if topic_idx < self.state.topics.len() {
                self.state.topics[topic_idx].content.lines().collect()
            } else {
                vec![]
            }
        }
    }

    /// Scroll up
    fn scroll_up(&mut self) {
        if self.state.scroll_offset > 0 {
            self.state.scroll_offset -= 1;
        }
    }

    /// Scroll down
    fn scroll_down(&mut self, visible_height: usize) {
        let max_scroll = if self.state.current_topic == 0 {
            // Index page - scroll through topics
            self.state.topics.len().saturating_sub(visible_height)
        } else {
            // Topic page - scroll through content
            self.current_content_lines()
                .len()
                .saturating_sub(visible_height)
        };
        if self.state.scroll_offset < max_scroll {
            self.state.scroll_offset += 1;
        }
    }

    /// Page up
    fn page_up(&mut self, visible_height: usize) {
        self.state.scroll_offset = self.state.scroll_offset.saturating_sub(visible_height);
    }

    /// Page down
    fn page_down(&mut self, visible_height: usize) {
        let max_scroll = if self.state.current_topic == 0 {
            // Index page - scroll through topics
            self.state.topics.len().saturating_sub(visible_height)
        } else {
            // Topic page - scroll through content
            self.current_content_lines()
                .len()
                .saturating_sub(visible_height)
        };
        self.state.scroll_offset = (self.state.scroll_offset + visible_height).min(max_scroll);
    }

    /// Go to start
    fn go_home(&mut self) {
        self.state.scroll_offset = 0;
    }

    /// Navigate to topic by key
    fn navigate_to_topic(&mut self, key: char) -> bool {
        let key_upper = key.to_ascii_uppercase();
        for (i, topic) in self.state.topics.iter().enumerate() {
            if topic.key == key_upper {
                self.state.current_topic = i + 1;
                self.state.scroll_offset = 0;
                return true;
            }
        }
        false
    }

    /// Go back to index
    fn go_to_index(&mut self) {
        self.state.current_topic = 0;
        self.state.scroll_offset = 0;
    }

    /// Load help topics from plugins
    /// Call this after all plugins are registered
    pub fn load_plugin_help(&mut self, plugin_help: Vec<(String, String, Vec<String>)>) {
        self.state.add_plugin_topics(plugin_help);
    }
}

impl Default for HelpPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for HelpPlugin {
    fn id(&self) -> &str {
        "help"
    }

    fn name(&self) -> &str {
        "Help System"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: false, // Help doesn't provide help content to itself
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
            name: "Help".to_string(),
            key: '1', // F1 key
            description: "Display help information".to_string(),
            priority: 10, // First in menu
        })
    }

    fn handle_global_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::F(1) => {
                self.open_modal();
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Assume a reasonable visible height for scrolling
        let visible_height = 20;

        match key.code {
            KeyCode::Esc => {
                if self.state.current_topic == 0 {
                    // On index page, close help
                    self.close_modal();
                    KeyHandleResult::CloseModal
                } else {
                    // On topic page, go back to index
                    self.go_to_index();
                    KeyHandleResult::Handled
                }
            }
            KeyCode::Up => {
                self.scroll_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.scroll_down(visible_height);
                KeyHandleResult::Handled
            }
            KeyCode::PageUp => {
                self.page_up(visible_height);
                KeyHandleResult::Handled
            }
            KeyCode::PageDown => {
                self.page_down(visible_height);
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                self.go_home();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if self.state.current_topic > 0 {
                    // On topic page, go back to index
                    self.go_to_index();
                }
                KeyHandleResult::Handled
            }
            KeyCode::F(1) => {
                self.close_modal();
                KeyHandleResult::CloseModal
            }
            KeyCode::Char(c) => {
                if self.state.current_topic == 0 {
                    // On index page, navigate to topic
                    self.navigate_to_topic(c);
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        // Calculate centered modal area (80% width, 80% height)
        let popup_width = ((area.width as f32) * 0.8) as u16;
        let popup_height = ((area.height as f32) * 0.8) as u16;
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;
        let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        if self.state.current_topic == 0 {
            // Index page
            self.draw_index(frame, modal_area, colors);
        } else {
            // Topic page
            self.draw_topic(frame, modal_area, colors);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl HelpPlugin {
    fn draw_index(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        let modal = ModalFrame::themed(area, " R-DOS Help ", colors).no_footer_separator();
        modal.render_frame(frame);

        let label_style = Style::default().fg(colors.yellow()).bg(colors.bg());
        let key_style = Style::default()
            .fg(colors.cyan())
            .bg(colors.bg())
            .add_modifier(Modifier::BOLD);
        let title_style = Style::default().fg(colors.fg()).bg(colors.bg());

        modal.render_row(
            frame,
            0,
            vec![Span::styled(
                "Select a topic by pressing its key:",
                label_style,
            )],
        );
        modal.render_row(frame, 1, vec![]);

        // Calculate visible height for topics (modal height - borders - title - instruction rows - help)
        let visible_height = (area.height as usize).saturating_sub(8); // 2 border + 1 title sep + 2 instruction + 1 help sep + 2 help
        let total_topics = self.state.topics.len();

        // Calculate scroll range
        let start = self.state.scroll_offset;
        let end = (start + visible_height).min(total_topics);

        for (i, topic) in self
            .state
            .topics
            .iter()
            .enumerate()
            .skip(start)
            .take(end - start)
        {
            let row = (i - start) as u16 + 2;
            modal.render_row(
                frame,
                row,
                vec![
                    Span::styled(format!("  {} ", topic.key), key_style),
                    Span::styled(&topic.title, title_style),
                ],
            );
        }

        // Show scroll indicator if needed
        let scroll_info = if total_topics > visible_height {
            format!(" [{}-{}/{}]", start + 1, end, total_topics)
        } else {
            String::new()
        };

        modal.render_help(
            frame,
            vec![
                ("↑↓", &format!("scroll{}", scroll_info)),
                ("ESC", "close help"),
            ],
        );
    }

    fn draw_topic(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        let topic_idx = self.state.current_topic - 1;
        let topic = &self.state.topics[topic_idx];

        let title = format!(" {} ", topic.title);
        let modal = ModalFrame::themed(area, &title, colors).no_footer_separator();
        modal.render_frame(frame);

        let content_style = Style::default().fg(colors.fg()).bg(colors.bg());

        // Get content lines
        let content_lines: Vec<&str> = topic.content.lines().collect();
        let visible_height = (area.height as usize).saturating_sub(6); // Account for border, title, separator, help
        let start = self.state.scroll_offset;
        let end = (start + visible_height).min(content_lines.len());

        for (i, line) in content_lines[start..end].iter().enumerate() {
            modal.render_row(frame, i as u16, vec![Span::styled(*line, content_style)]);
        }

        // Show scroll indicator if needed
        let total_lines = content_lines.len();
        let scroll_info = if total_lines > visible_height {
            format!(" [Line {}-{} of {}]", start + 1, end, total_lines)
        } else {
            String::new()
        };

        modal.render_help(
            frame,
            vec![
                ("ESC/Enter", "back to index"),
                ("Up/Down", &format!("scroll{}", scroll_info)),
            ],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_plugin_creation() {
        let plugin = HelpPlugin::new();
        assert_eq!(plugin.id(), "help");
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_modal_open_close() {
        let mut plugin = HelpPlugin::new();
        plugin.open_modal();
        assert!(plugin.is_modal_open());
        plugin.close_modal();
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_topic_navigation() {
        let mut plugin = HelpPlugin::new();
        plugin.open_modal();
        assert_eq!(plugin.state.current_topic, 0); // Index

        // Navigate to a topic
        plugin.navigate_to_topic('I');
        assert_eq!(plugin.state.current_topic, 1); // Introduction

        // Go back to index
        plugin.go_to_index();
        assert_eq!(plugin.state.current_topic, 0);
    }
}
