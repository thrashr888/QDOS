//! QDCONFIG Plugin for R-DOS
//!
//! Provides startup configuration (Ctrl+S) as a self-contained plugin.

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem};
use crate::app::{ColorTheme, SortMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
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
const COLOR_GREY: Color = Color::Rgb(0x80, 0x80, 0x80);

/// Configuration fields
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QdconfigField {
    SearchSpec,
    SortMethod,
    SortDirection,
    ShowHidden,
    ConfirmDelete,
    Editor,
    ColorTheme,
    MouseSupport,
    UppercaseNames,
    AutoRefresh,
}

impl QdconfigField {
    pub const ALL: [QdconfigField; 10] = [
        QdconfigField::SearchSpec,
        QdconfigField::SortMethod,
        QdconfigField::SortDirection,
        QdconfigField::ShowHidden,
        QdconfigField::ConfirmDelete,
        QdconfigField::Editor,
        QdconfigField::ColorTheme,
        QdconfigField::MouseSupport,
        QdconfigField::UppercaseNames,
        QdconfigField::AutoRefresh,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            QdconfigField::SearchSpec => "Search Specification",
            QdconfigField::SortMethod => "Sort Method",
            QdconfigField::SortDirection => "Sort Direction",
            QdconfigField::ShowHidden => "Show Hidden Files",
            QdconfigField::ConfirmDelete => "Confirm Delete",
            QdconfigField::Editor => "Default Editor",
            QdconfigField::ColorTheme => "Color Theme",
            QdconfigField::MouseSupport => "Mouse Support",
            QdconfigField::UppercaseNames => "Uppercase Names",
            QdconfigField::AutoRefresh => "Auto-Refresh (sec)",
        }
    }
}

/// Configuration state
#[derive(Debug, Clone)]
pub struct QdconfigState {
    /// Currently selected field
    pub selected: usize,
    /// Editing mode (for text input fields)
    pub editing: bool,
    /// Input buffer for text fields
    pub input_buffer: String,
    /// Search specification
    pub search_spec: String,
    /// Sort method (0=name, 1=ext, 2=size, 3=date, 4=none)
    pub sort_method: usize,
    /// Sort direction (true=asc, false=desc)
    pub sort_asc: bool,
    /// Show hidden files
    pub show_hidden: bool,
    /// Confirm before delete
    pub confirm_delete: bool,
    /// Editor command (None = use $EDITOR)
    pub editor: Option<String>,
    /// Color theme index
    pub theme_index: usize,
    /// Mouse support enabled
    pub mouse_support: bool,
    /// Show filenames in uppercase
    pub uppercase_names: bool,
    /// Auto-refresh interval in seconds (0 = disabled)
    pub auto_refresh_interval: u64,
    /// Original theme for cancel restore
    original_theme_index: usize,
}

impl QdconfigState {
    pub fn new(
        search_spec: String,
        sort_mode: SortMode,
        show_hidden: bool,
        confirm_delete: bool,
        editor: Option<String>,
        color_theme: ColorTheme,
        mouse_support: bool,
        uppercase_names: bool,
        auto_refresh_interval: u64,
    ) -> Self {
        // Convert SortMode to method + direction
        let (sort_method, sort_asc) = match sort_mode {
            SortMode::NameAsc => (0, true),
            SortMode::NameDesc => (0, false),
            SortMode::ExtAsc => (1, true),
            SortMode::ExtDesc => (1, false),
            SortMode::SizeAsc => (2, true),
            SortMode::SizeDesc => (2, false),
            SortMode::DateAsc => (3, true),
            SortMode::DateDesc => (3, false),
            SortMode::None => (4, true),
        };

        // Get theme index
        let theme_index = ColorTheme::ALL
            .iter()
            .position(|&t| t == color_theme)
            .unwrap_or(0);

        Self {
            selected: 0,
            editing: false,
            input_buffer: String::new(),
            search_spec,
            sort_method,
            sort_asc,
            show_hidden,
            confirm_delete,
            editor,
            theme_index,
            mouse_support,
            uppercase_names,
            auto_refresh_interval,
            original_theme_index: theme_index,
        }
    }

    pub fn cycle_auto_refresh(&mut self) {
        // Cycle through: 0 (off), 1, 2, 5, 10, 30, 60 seconds
        self.auto_refresh_interval = match self.auto_refresh_interval {
            0 => 1,
            1 => 2,
            2 => 5,
            5 => 10,
            10 => 30,
            30 => 60,
            _ => 0,
        };
    }

    pub fn current_field(&self) -> QdconfigField {
        QdconfigField::ALL[self.selected]
    }

    pub fn sort_method_name(&self) -> &'static str {
        match self.sort_method {
            0 => "Name",
            1 => "Extension",
            2 => "Size",
            3 => "Date",
            _ => "None",
        }
    }

    pub fn cycle_sort_method(&mut self) {
        self.sort_method = (self.sort_method + 1) % 5;
    }

    pub fn toggle_sort_direction(&mut self) {
        self.sort_asc = !self.sort_asc;
    }

    pub fn cycle_theme(&mut self) {
        self.theme_index = (self.theme_index + 1) % ColorTheme::ALL.len();
    }

    pub fn theme(&self) -> ColorTheme {
        ColorTheme::ALL[self.theme_index]
    }

    pub fn original_theme(&self) -> ColorTheme {
        ColorTheme::ALL[self.original_theme_index]
    }

    pub fn sort_mode(&self) -> SortMode {
        match (self.sort_method, self.sort_asc) {
            (0, true) => SortMode::NameAsc,
            (0, false) => SortMode::NameDesc,
            (1, true) => SortMode::ExtAsc,
            (1, false) => SortMode::ExtDesc,
            (2, true) => SortMode::SizeAsc,
            (2, false) => SortMode::SizeDesc,
            (3, true) => SortMode::DateAsc,
            (3, false) => SortMode::DateDesc,
            _ => SortMode::None,
        }
    }
}

/// QDCONFIG plugin for startup configuration
pub struct QdconfigPlugin {
    /// Whether modal is open
    modal_open: bool,
    /// Configuration state
    state: Option<QdconfigState>,
    /// Result state (set when applied/saved)
    result_state: Option<QdconfigState>,
    /// Whether settings were saved (vs just applied)
    settings_saved: bool,
}

impl QdconfigPlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: None,
            result_state: None,
            settings_saved: false,
        }
    }

    /// Check if modal is open
    pub fn is_modal_open(&self) -> bool {
        self.modal_open
    }

    /// Open the modal with current settings
    pub fn open_modal(
        &mut self,
        search_spec: String,
        sort_mode: SortMode,
        show_hidden: bool,
        confirm_delete: bool,
        editor: Option<String>,
        color_theme: ColorTheme,
        mouse_support: bool,
        uppercase_names: bool,
        auto_refresh_interval: u64,
    ) {
        self.state = Some(QdconfigState::new(
            search_spec,
            sort_mode,
            show_hidden,
            confirm_delete,
            editor,
            color_theme,
            mouse_support,
            uppercase_names,
            auto_refresh_interval,
        ));
        self.result_state = None;
        self.settings_saved = false;
        self.modal_open = true;
    }

    /// Close the modal
    pub fn close_modal(&mut self) {
        self.modal_open = false;
        self.state = None;
    }

    /// Get the result state (if applied)
    pub fn take_result(&mut self) -> Option<QdconfigState> {
        self.result_state.take()
    }

    /// Check if settings were saved to disk
    pub fn was_saved(&self) -> bool {
        self.settings_saved
    }

    /// Get current preview theme (for live preview)
    pub fn preview_theme(&self) -> Option<ColorTheme> {
        self.state.as_ref().map(|s| s.theme())
    }
}

impl Default for QdconfigPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for QdconfigPlugin {
    fn id(&self) -> &str {
        "qdconfig"
    }

    fn name(&self) -> &str {
        "Configuration"
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
            name: "Config".to_string(),
            key: 'S', // Ctrl+S
            description: "Configure startup options".to_string(),
            priority: 40, // After SearchSpec
        })
    }

    fn handle_global_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Ctrl+S opens configuration
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            // We'll get the settings synced from app when modal opens
            // Open with defaults for now - app will re-sync in handle_plugin_result
            self.state = Some(QdconfigState::new(
                "*.*".to_string(),
                SortMode::NameAsc,
                false,
                true,
                None,
                ColorTheme::Default,
                false,
                false,
                5, // default auto-refresh
            ));
            self.result_state = None;
            self.settings_saved = false;
            self.modal_open = true;
            KeyHandleResult::OpenModal
        } else {
            KeyHandleResult::NotHandled
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref mut state) = self.state else {
            return KeyHandleResult::CloseModal;
        };

        if state.editing {
            // Handle text input mode
            match key.code {
                KeyCode::Enter => {
                    // Apply the edited value
                    let current_field = state.current_field();
                    match current_field {
                        QdconfigField::SearchSpec => {
                            state.search_spec = state.input_buffer.clone();
                        }
                        QdconfigField::Editor => {
                            if state.input_buffer.is_empty() || state.input_buffer == "$EDITOR" {
                                state.editor = None;
                            } else {
                                state.editor = Some(state.input_buffer.clone());
                            }
                        }
                        _ => {}
                    }
                    state.editing = false;
                    state.input_buffer.clear();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    state.editing = false;
                    state.input_buffer.clear();
                    KeyHandleResult::Handled
                }
                KeyCode::Backspace => {
                    state.input_buffer.pop();
                    KeyHandleResult::Handled
                }
                KeyCode::Char(c) => {
                    state.input_buffer.push(c);
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        } else {
            // Handle navigation/selection mode
            match key.code {
                KeyCode::Esc => {
                    // Restore original theme on cancel
                    let original_theme = state.original_theme();
                    self.close_modal();
                    KeyHandleResult::CloseWithError(format!("theme:{}", original_theme.name()))
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if state.selected > 0 {
                        state.selected -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.selected < QdconfigField::ALL.len() - 1 {
                        state.selected += 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Toggle or edit based on field type
                    let current_field = state.current_field();
                    match current_field {
                        QdconfigField::SearchSpec | QdconfigField::Editor => {
                            // Enter editing mode
                            state.editing = true;
                            match current_field {
                                QdconfigField::SearchSpec => {
                                    state.input_buffer = state.search_spec.clone();
                                }
                                QdconfigField::Editor => {
                                    state.input_buffer = state
                                        .editor
                                        .clone()
                                        .unwrap_or_else(|| "$EDITOR".to_string());
                                }
                                _ => {}
                            }
                        }
                        QdconfigField::SortMethod => {
                            state.cycle_sort_method();
                        }
                        QdconfigField::SortDirection => {
                            state.toggle_sort_direction();
                        }
                        QdconfigField::ShowHidden => {
                            state.show_hidden = !state.show_hidden;
                        }
                        QdconfigField::ConfirmDelete => {
                            state.confirm_delete = !state.confirm_delete;
                        }
                        QdconfigField::ColorTheme => {
                            state.cycle_theme();
                            // Live preview handled by returning Handled
                            // App checks preview_theme() in handle_plugin_result
                        }
                        QdconfigField::MouseSupport => {
                            state.mouse_support = !state.mouse_support;
                        }
                        QdconfigField::UppercaseNames => {
                            state.uppercase_names = !state.uppercase_names;
                        }
                        QdconfigField::AutoRefresh => {
                            state.cycle_auto_refresh();
                        }
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    // Save configuration
                    self.result_state = Some(state.clone());
                    self.settings_saved = true;
                    self.close_modal();
                    KeyHandleResult::CloseWithSuccess("qdconfig:saved".to_string())
                }
                _ => KeyHandleResult::Handled,
            }
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect) {
        let Some(ref state) = self.state else {
            return;
        };

        // Clear the entire area
        frame.render_widget(Clear, area);

        // Layout: title, separator, content, separator, help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Length(1), // Separator
                Constraint::Min(12),   // Content
                Constraint::Length(1), // Separator
                Constraint::Length(1), // Help line
            ])
            .split(area);

        // Title
        let title = " R-DOS STARTUP CONFIGURATION ";
        frame.render_widget(
            Paragraph::new(Span::styled(
                title,
                Style::default()
                    .fg(COLOR_FG)
                    .add_modifier(Modifier::BOLD),
            )),
            chunks[0],
        );

        // Separator
        let sep = "═".repeat(area.width as usize);
        frame.render_widget(
            Paragraph::new(Span::styled(sep.clone(), Style::default().fg(COLOR_FG))),
            chunks[1],
        );

        // Content area
        let content_area = chunks[2];
        let mut lines: Vec<Line> = vec![Line::from("")];

        for (i, field) in QdconfigField::ALL.iter().enumerate() {
            let is_selected = i == state.selected;
            let is_editing = is_selected && state.editing;

            // Get field name and value
            let name = field.name();
            let value = match field {
                QdconfigField::SearchSpec => {
                    if is_editing {
                        format!("{}█", state.input_buffer)
                    } else {
                        state.search_spec.clone()
                    }
                }
                QdconfigField::SortMethod => state.sort_method_name().to_string(),
                QdconfigField::SortDirection => {
                    if state.sort_asc {
                        "Ascending".to_string()
                    } else {
                        "Descending".to_string()
                    }
                }
                QdconfigField::ShowHidden => {
                    if state.show_hidden {
                        "Yes".to_string()
                    } else {
                        "No".to_string()
                    }
                }
                QdconfigField::ConfirmDelete => {
                    if state.confirm_delete {
                        "Yes".to_string()
                    } else {
                        "No".to_string()
                    }
                }
                QdconfigField::Editor => {
                    if is_editing {
                        format!("{}█", state.input_buffer)
                    } else {
                        state
                            .editor
                            .clone()
                            .unwrap_or_else(|| "$EDITOR".to_string())
                    }
                }
                QdconfigField::ColorTheme => state.theme().name().to_string(),
                QdconfigField::MouseSupport => {
                    if state.mouse_support {
                        "Yes".to_string()
                    } else {
                        "No".to_string()
                    }
                }
                QdconfigField::UppercaseNames => {
                    if state.uppercase_names {
                        "Yes".to_string()
                    } else {
                        "No".to_string()
                    }
                }
                QdconfigField::AutoRefresh => {
                    if state.auto_refresh_interval == 0 {
                        "Off".to_string()
                    } else {
                        format!("{} sec", state.auto_refresh_interval)
                    }
                }
            };

            // Style based on selection
            let line_style = if is_selected {
                Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)
            } else {
                Style::default().fg(COLOR_FG).bg(COLOR_BG)
            };

            let name_style = if is_selected {
                Style::default()
                    .fg(COLOR_YELLOW)
                    .bg(COLOR_RED)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(COLOR_BLUE)
            };

            let value_style = if is_editing {
                Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)
            } else if is_selected {
                Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)
            } else {
                Style::default().fg(COLOR_GREEN)
            };

            // Format as "  Field Name:        Value"
            let padded_name = format!("  {:<22}", format!("{}:", name));
            let padded_value = format!("{:<20}", value);

            lines.push(Line::from(vec![
                Span::styled(padded_name, name_style),
                Span::styled(padded_value, value_style),
                Span::styled(
                    " ".repeat(area.width.saturating_sub(44) as usize),
                    line_style,
                ),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Settings will be saved to ~/.config/rdos/config.toml",
            Style::default().fg(COLOR_GREY),
        )));

        frame.render_widget(Paragraph::new(lines), content_area);

        // Bottom separator
        frame.render_widget(
            Paragraph::new(Span::styled(sep, Style::default().fg(COLOR_FG))),
            chunks[3],
        );

        // Help line
        let help_text = if state.editing {
            "Type value, Enter to confirm, ESC to cancel"
        } else {
            "↑↓ select  Enter/Space toggle  S save  ESC close"
        };
        frame.render_widget(
            Paragraph::new(Span::styled(help_text, Style::default().fg(COLOR_GREEN))),
            chunks[4],
        );
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Ctrl+S - Configuration".to_string(),
            "  Configure startup options".to_string(),
            "  S: Save settings to config file".to_string(),
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
    fn test_qdconfig_plugin_creation() {
        let plugin = QdconfigPlugin::new();
        assert_eq!(plugin.id(), "qdconfig");
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_modal_open_close() {
        let mut plugin = QdconfigPlugin::new();
        plugin.open_modal(
            "*.*".to_string(),
            SortMode::NameAsc,
            false,
            true,
            None,
            ColorTheme::Default,
            false,
            false,
            5,
        );
        assert!(plugin.is_modal_open());
        plugin.close_modal();
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_field_cycling() {
        let mut state = QdconfigState::new(
            "*.*".to_string(),
            SortMode::NameAsc,
            false,
            true,
            None,
            ColorTheme::Default,
            false,
            false,
            5,
        );

        // Test sort method cycling
        assert_eq!(state.sort_method_name(), "Name");
        state.cycle_sort_method();
        assert_eq!(state.sort_method_name(), "Extension");

        // Test theme cycling
        let initial_theme = state.theme();
        state.cycle_theme();
        assert_ne!(state.theme(), initial_theme);
    }
}
