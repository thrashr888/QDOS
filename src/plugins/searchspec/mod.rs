//! Search Specification Plugin for R-DOS
//!
//! Provides search specification (F7) functionality as a self-contained plugin.

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;

// Colors
const COLOR_BG: Color = Color::Black;
const COLOR_FG: Color = Color::White;
const COLOR_BLUE: Color = Color::Rgb(0x55, 0x55, 0xFF);
const COLOR_GREEN: Color = Color::Rgb(0x55, 0xFF, 0x55);
const COLOR_YELLOW: Color = Color::Rgb(0xFF, 0xFF, 0x55);
const COLOR_RED: Color = Color::Rgb(0xFF, 0x55, 0x55);
const COLOR_GREY: Color = Color::Rgb(0x80, 0x80, 0x80);

/// Search specification state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSpecState {
    pub pattern: String,
    pub phase: u8,
    pub attrs: [bool; 6],
    pub selected_attr: usize,
}

impl SearchSpecState {
    pub fn new(current_spec: &str) -> Self {
        Self {
            pattern: current_spec.to_string(),
            phase: 0,
            attrs: [true, true, false, false, false, false],
            selected_attr: 0,
        }
    }

    pub fn attr_name(index: usize) -> &'static str {
        match index {
            0 => "NORM",
            1 => "DIR ",
            2 => "HID ",
            3 => "SYS ",
            4 => "R/O ",
            5 => "ARC ",
            _ => "????",
        }
    }

    pub fn toggle_current(&mut self) {
        self.attrs[self.selected_attr] = !self.attrs[self.selected_attr];
    }
}

/// Search Specification plugin
pub struct SearchSpecPlugin {
    /// Whether modal is open
    modal_open: bool,
    /// Search spec state
    state: Option<SearchSpecState>,
    /// Result pattern (set when applied)
    result_pattern: Option<String>,
}

impl SearchSpecPlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: None,
            result_pattern: None,
        }
    }

    /// Check if modal is open
    pub fn is_modal_open(&self) -> bool {
        self.modal_open
    }

    /// Open the modal with current search spec
    pub fn open_modal(&mut self, current_spec: &str) {
        self.state = Some(SearchSpecState::new(current_spec));
        self.result_pattern = None;
        self.modal_open = true;
    }

    /// Close the modal
    pub fn close_modal(&mut self) {
        self.modal_open = false;
        self.state = None;
    }

    /// Get the result pattern (if applied)
    pub fn take_result(&mut self) -> Option<String> {
        self.result_pattern.take()
    }
}

impl Default for SearchSpecPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for SearchSpecPlugin {
    fn id(&self) -> &str {
        "searchspec"
    }

    fn name(&self) -> &str {
        "Search Specification"
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
            name: "SrchSpec".to_string(),
            key: '7', // F7 key
            description: "Set search specification".to_string(),
            priority: 35, // After DirMap
        })
    }

    fn handle_global_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // F7 opens search spec
        if key.code == KeyCode::F(7) {
            // Note: We'll get the current spec from app when modal opens
            self.open_modal("*.*");
            KeyHandleResult::OpenModal
        } else {
            KeyHandleResult::NotHandled
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref mut state) = self.state else {
            return KeyHandleResult::CloseModal;
        };

        match state.phase {
            0 => {
                // Phase 0: Pattern input
                match key.code {
                    KeyCode::Enter => {
                        // Move to attribute selection phase
                        state.phase = 1;
                        KeyHandleResult::Handled
                    }
                    KeyCode::Esc => {
                        self.close_modal();
                        KeyHandleResult::CloseModal
                    }
                    KeyCode::Backspace => {
                        state.pattern.pop();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char(c) => {
                        state.pattern.push(c);
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                }
            }
            1 => {
                // Phase 1: Attribute selection
                match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        if state.selected_attr > 0 {
                            state.selected_attr -= 1;
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if state.selected_attr < 5 {
                            state.selected_attr += 1;
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char(' ') => {
                        state.toggle_current();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Enter => {
                        // Apply the search specification
                        self.result_pattern = Some(state.pattern.clone());
                        self.close_modal();
                        KeyHandleResult::CloseWithSuccess("searchspec:applied".to_string())
                    }
                    KeyCode::Esc => {
                        // Go back to pattern phase
                        state.phase = 0;
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                }
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect) {
        let Some(ref state) = self.state else {
            return;
        };

        // Calculate centered modal area
        let popup_width = 60.min(area.width.saturating_sub(4));
        let popup_height = 12.min(area.height.saturating_sub(4));
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;
        let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Clear the modal area
        frame.render_widget(Clear, modal_area);

        let title = if state.phase == 0 {
            " Set Search Specification "
        } else {
            " Search Attributes "
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BLUE))
            .style(Style::default().bg(COLOR_BG));

        let mut lines = vec![Line::from("")];

        if state.phase == 0 {
            // Phase 0: Pattern input
            lines.push(Line::from(Span::styled(
                "Enter file search specification:",
                Style::default().fg(COLOR_GREEN),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Pattern: ", Style::default().fg(COLOR_BLUE)),
                Span::styled(
                    &state.pattern,
                    Style::default().fg(COLOR_YELLOW).bg(COLOR_RED),
                ),
                Span::styled("█", Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Examples: *.*  *.txt  *.rs  config.*",
                Style::default().fg(COLOR_GREY),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
                Span::raw(" next  "),
                Span::styled("ESC", Style::default().fg(COLOR_BLUE)),
                Span::raw(" cancel"),
            ]));
        } else {
            // Phase 1: Attribute selection
            lines.push(Line::from(vec![
                Span::styled("Pattern: ", Style::default().fg(COLOR_GREEN)),
                Span::styled(&state.pattern, Style::default().fg(COLOR_FG)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Select which file types to display:",
                Style::default().fg(COLOR_GREEN),
            )));
            lines.push(Line::from(""));

            // Build attribute bar
            let mut attr_spans: Vec<Span> = vec![Span::raw("  ")];
            for i in 0..6 {
                let name = SearchSpecState::attr_name(i);
                let is_on = state.attrs[i];
                let is_selected = i == state.selected_attr;

                let style = if is_selected {
                    Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)
                } else if is_on {
                    Style::default().fg(COLOR_GREEN)
                } else {
                    Style::default().fg(COLOR_GREY)
                };

                let indicator = if is_on { " ✓ " } else { "   " };
                attr_spans.push(Span::styled(format!("[{}{}]", name, indicator), style));
                attr_spans.push(Span::raw(" "));
            }
            lines.push(Line::from(attr_spans));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("←→", Style::default().fg(COLOR_BLUE)),
                Span::raw(" select  "),
                Span::styled("SPACE", Style::default().fg(COLOR_BLUE)),
                Span::raw(" toggle  "),
                Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
                Span::raw(" apply  "),
                Span::styled("ESC", Style::default().fg(COLOR_BLUE)),
                Span::raw(" back"),
            ]));
        }

        let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, modal_area);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "F7 - Set Search Specification".to_string(),
            "  Filter files by pattern (*.txt, *.rs, etc.)".to_string(),
            "  Toggle file type attributes".to_string(),
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
    fn test_searchspec_plugin_creation() {
        let plugin = SearchSpecPlugin::new();
        assert_eq!(plugin.id(), "searchspec");
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_modal_open_close() {
        let mut plugin = SearchSpecPlugin::new();
        plugin.open_modal("*.txt");
        assert!(plugin.is_modal_open());
        assert_eq!(plugin.state.as_ref().unwrap().pattern, "*.txt");
        plugin.close_modal();
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_phase_transition() {
        let mut plugin = SearchSpecPlugin::new();
        plugin.open_modal("*.*");
        assert_eq!(plugin.state.as_ref().unwrap().phase, 0);

        // Transition to phase 1
        if let Some(ref mut state) = plugin.state {
            state.phase = 1;
        }
        assert_eq!(plugin.state.as_ref().unwrap().phase, 1);
    }
}
