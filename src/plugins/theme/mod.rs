//! Theme Plugin for R-DOS
//!
//! Provides color theme selection (Ctrl+T functionality) as a self-contained plugin.

use super::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
};
use crate::app::{ColorTheme, ColorThemeState};
use crate::ui::components::ModalFrame;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;

/// Theme plugin that manages color theme selection
pub struct ThemePlugin {
    /// Whether modal is open
    modal_open: bool,
    /// Theme selection state
    state: Option<ColorThemeState>,
    /// Current app theme (for live preview)
    current_theme: ColorTheme,
}

impl ThemePlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: None,
            current_theme: ColorTheme::Default,
        }
    }

    /// Check if modal is open
    pub fn is_modal_open(&self) -> bool {
        self.modal_open
    }

    /// Open the modal with current theme
    pub fn open_modal(&mut self, current_theme: ColorTheme) {
        self.current_theme = current_theme;
        self.state = Some(ColorThemeState::new(current_theme));
        self.modal_open = true;
    }

    /// Close the modal
    pub fn close_modal(&mut self) {
        self.modal_open = false;
        self.state = None;
    }

    /// Get the currently selected theme (for live preview)
    pub fn selected_theme(&self) -> Option<ColorTheme> {
        self.state.as_ref().map(|s| s.selected_theme())
    }

    /// Get the original theme (for cancel)
    pub fn original_theme(&self) -> Option<ColorTheme> {
        self.state.as_ref().map(|s| s.original_theme)
    }

    /// Set current theme (called by app when theme changes)
    pub fn set_current_theme(&mut self, theme: ColorTheme) {
        self.current_theme = theme;
    }
}

impl Default for ThemePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ThemePlugin {
    fn id(&self) -> &str {
        "theme"
    }

    fn name(&self) -> &str {
        "Color Theme"
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

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true // Always available
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Theme".to_string(),
            key: 'T',
            description: "Change color theme".to_string(),
            priority: 80, // After other features
        })
    }

    fn handle_global_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Ctrl+T opens theme selector
        if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.open_modal(self.current_theme);
            KeyHandleResult::OpenModal
        } else {
            KeyHandleResult::NotHandled
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref mut state) = self.state else {
            return KeyHandleResult::CloseModal;
        };

        match key.code {
            KeyCode::Esc => {
                // Cancel - restore original theme
                let original = state.original_theme;
                self.close_modal();
                KeyHandleResult::CloseWithError(format!("theme:{}", original.name()))
            }
            KeyCode::Enter => {
                // Apply selected theme
                let selected = state.selected_theme();
                self.close_modal();
                KeyHandleResult::CloseWithSuccess(format!("theme:{}", selected.name()))
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected > 0 {
                    state.selected -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected < ColorTheme::ALL.len() - 1 {
                    state.selected += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('1') => {
                state.selected = 0;
                KeyHandleResult::Handled
            }
            KeyCode::Char('2') => {
                state.selected = 1;
                KeyHandleResult::Handled
            }
            KeyCode::Char('3') => {
                state.selected = 2;
                KeyHandleResult::Handled
            }
            KeyCode::Char('4') => {
                state.selected = 3;
                KeyHandleResult::Handled
            }
            KeyCode::Char('5') => {
                state.selected = 4;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        let Some(ref state) = self.state else {
            return;
        };

        // Calculate centered modal area
        let popup_width = 50.min(area.width.saturating_sub(4));
        let popup_height = 15.min(area.height.saturating_sub(4));
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;
        let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Use ModalFrame with theme-aware border (fg color for white borders per SPEC)
        let modal = ModalFrame::themed(modal_area, " Color Theme ", colors);
        modal.render_frame(frame);

        let label_style = Style::default().fg(colors.yellow()).bg(colors.bg());

        modal.render_row(frame, 0, vec![Span::styled("Select a theme:", label_style)]);
        modal.render_row(frame, 1, vec![]);

        for (i, theme) in ColorTheme::ALL.iter().enumerate() {
            let marker = if i == state.selected { "> " } else { "  " };
            let style = if i == state.selected {
                Style::default()
                    .fg(colors.yellow())
                    .bg(colors.bg())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg()).bg(colors.bg())
            };
            let desc_style = Style::default()
                .fg(colors.grey())
                .bg(colors.bg())
                .add_modifier(Modifier::DIM);

            modal.render_row(
                frame,
                2 + i as u16,
                vec![
                    Span::styled(marker, style),
                    Span::styled(
                        format!("{}. ", i + 1),
                        Style::default().fg(colors.cyan()).bg(colors.bg()),
                    ),
                    Span::styled(theme.name(), style),
                    Span::styled(format!(" - {}", theme.description()), desc_style),
                ],
            );
        }

        modal.render_help(
            frame,
            vec![("Enter", "apply"), ("Esc", "cancel"), ("1-5", "select")],
        );
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "           T -- COLOR THEME".to_string(),
            "".to_string(),
            "Purpose:   Change the visual color scheme of R-DOS. Themes".to_string(),
            "           affect all UI elements including the file list,".to_string(),
            "           menus, dialogs, and status bar.".to_string(),
            "".to_string(),
            "To use:    Press Ctrl+T to open the theme selector.".to_string(),
            "".to_string(),
            "Available Themes:".to_string(),
            "  Default    - Classic blue-white Q-DOS II colors".to_string(),
            "  Dark       - Muted colors for low-light environments".to_string(),
            "  Amber      - Warm amber tones (vintage CRT style)".to_string(),
            "  Green      - Monochrome green terminal look".to_string(),
            "  High Con.  - High contrast for accessibility".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  ↑↓       - Navigate theme list".to_string(),
            "  1-5      - Quick select by number".to_string(),
            "  Enter    - Apply selected theme".to_string(),
            "  Esc      - Cancel and restore original theme".to_string(),
            "".to_string(),
            "Tip:       Theme changes preview live as you navigate.".to_string(),
            "           Use Config (Ctrl+S) to save theme permanently.".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Theme".to_string(),
            description: "Color theme settings".to_string(),
            category: PluginCategory::System,
            key: 'T',
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
    fn test_theme_plugin_creation() {
        let plugin = ThemePlugin::new();
        assert_eq!(plugin.id(), "theme");
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_modal_open_close() {
        let mut plugin = ThemePlugin::new();
        plugin.open_modal(ColorTheme::Default);
        assert!(plugin.is_modal_open());
        assert_eq!(plugin.selected_theme(), Some(ColorTheme::Default));
        plugin.close_modal();
        assert!(!plugin.is_modal_open());
    }
}
