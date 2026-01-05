//! QDCONFIG state types
//!
//! Configuration field definitions and state management.

use crate::app::{ColorTheme, SortMode};

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
    /// Currently selected field (index into selectable items only)
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
    /// Registered plugins list (id, name, description)
    pub plugins: Vec<(String, String, String)>,
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
        plugins: Vec<(String, String, String)>,
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
            plugins,
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

    /// Get the currently selected config field, or None if a plugin is selected
    pub fn current_field(&self) -> Option<QdconfigField> {
        if self.selected < QdconfigField::ALL.len() {
            Some(QdconfigField::ALL[self.selected])
        } else {
            None // A plugin is selected, not a config field
        }
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

    /// Reload state from a config file
    pub fn reload_from_config(&mut self, config: &crate::config::Config) {
        // Update search spec
        self.search_spec = config.general.search_spec.clone();

        // Update sort mode
        let sort_mode = config.to_sort_mode();
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
        self.sort_method = sort_method;
        self.sort_asc = sort_asc;

        // Update general settings
        self.show_hidden = config.general.show_hidden;
        self.confirm_delete = config.general.confirm_delete;
        self.mouse_support = config.general.mouse_support;
        self.auto_refresh_interval = config.general.auto_refresh_interval;

        // Update editor
        self.editor = config.editor.command.clone();

        // Update display settings
        self.uppercase_names = config.display.uppercase_names;

        // Update theme
        let color_theme: ColorTheme = config.display.theme.clone().into();
        self.theme_index = ColorTheme::ALL
            .iter()
            .position(|&t| t == color_theme)
            .unwrap_or(0);
        self.original_theme_index = self.theme_index;

        // Clear editing state
        self.editing = false;
        self.input_buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_names() {
        assert_eq!(QdconfigField::SearchSpec.name(), "Search Specification");
        assert_eq!(QdconfigField::ColorTheme.name(), "Color Theme");
    }

    #[test]
    fn test_state_new() {
        let state = QdconfigState::new(
            "*.*".to_string(),
            SortMode::NameAsc,
            false,
            true,
            None,
            ColorTheme::Default,
            false,
            false,
            5,
            Vec::new(),
        );
        assert_eq!(state.selected, 0);
        assert_eq!(state.sort_method, 0);
        assert!(state.sort_asc);
        assert!(!state.show_hidden);
        assert!(state.confirm_delete);
    }

    #[test]
    fn test_sort_method_cycling() {
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
            Vec::new(),
        );

        assert_eq!(state.sort_method_name(), "Name");
        state.cycle_sort_method();
        assert_eq!(state.sort_method_name(), "Extension");
        state.cycle_sort_method();
        assert_eq!(state.sort_method_name(), "Size");
    }

    #[test]
    fn test_theme_cycling() {
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
            Vec::new(),
        );

        let initial_theme = state.theme();
        state.cycle_theme();
        assert_ne!(state.theme(), initial_theme);
    }

    #[test]
    fn test_auto_refresh_cycling() {
        let mut state = QdconfigState::new(
            "*.*".to_string(),
            SortMode::NameAsc,
            false,
            true,
            None,
            ColorTheme::Default,
            false,
            false,
            0,
            Vec::new(),
        );

        assert_eq!(state.auto_refresh_interval, 0);
        state.cycle_auto_refresh();
        assert_eq!(state.auto_refresh_interval, 1);
        state.cycle_auto_refresh();
        assert_eq!(state.auto_refresh_interval, 2);
    }

    #[test]
    fn test_sort_mode_conversion() {
        let state = QdconfigState::new(
            "*.*".to_string(),
            SortMode::SizeDesc,
            false,
            true,
            None,
            ColorTheme::Default,
            false,
            false,
            5,
            Vec::new(),
        );

        assert_eq!(state.sort_mode(), SortMode::SizeDesc);
    }
}
