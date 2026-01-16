//! Dependency Manager plugin state

use serde::{Deserialize, Serialize};

/// Supported package managers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PackageManager {
    #[default]
    Cargo,
    Npm,
    Pnpm,
    Yarn,
    Pip,
    Uv,
    Poetry,
    GoMod,
}

impl PackageManager {
    /// Display name for the package manager
    pub fn name(&self) -> &'static str {
        match self {
            PackageManager::Cargo => "Cargo",
            PackageManager::Npm => "npm",
            PackageManager::Pnpm => "pnpm",
            PackageManager::Yarn => "Yarn",
            PackageManager::Pip => "pip",
            PackageManager::Uv => "uv",
            PackageManager::Poetry => "Poetry",
            PackageManager::GoMod => "Go",
        }
    }

    /// File hint for detection
    pub fn file_hint(&self) -> &'static str {
        match self {
            PackageManager::Cargo => "Cargo.toml",
            PackageManager::Npm | PackageManager::Pnpm | PackageManager::Yarn => "package.json",
            PackageManager::Pip => "requirements.txt",
            PackageManager::Uv | PackageManager::Poetry => "pyproject.toml",
            PackageManager::GoMod => "go.mod",
        }
    }
}

/// View modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DepsView {
    #[default]
    List,
    Outdated,
    Search,
    SearchInput,
    Install,
    Output,
    Confirm,
}

/// A package entry
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageEntry {
    pub name: String,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub is_dev: bool,
    pub is_outdated: bool,
}

/// Search result
#[derive(Debug, Clone, Default)]
pub struct SearchResult {
    pub name: String,
    pub description: String,
    pub version: String,
}

/// Action requiring confirmation
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    Install(String, bool), // (name, is_dev)
    Uninstall(String),
    Update(String),
    UpdateAll,
}

impl std::fmt::Display for ConfirmAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfirmAction::Install(name, dev) => {
                if *dev {
                    write!(f, "Install {} as dev dependency?", name)
                } else {
                    write!(f, "Install {}?", name)
                }
            }
            ConfirmAction::Uninstall(name) => write!(f, "Uninstall {}?", name),
            ConfirmAction::Update(name) => write!(f, "Update {}?", name),
            ConfirmAction::UpdateAll => write!(f, "Update all packages?"),
        }
    }
}

/// Main state container
#[derive(Debug, Clone, Default)]
pub struct DepsState {
    pub view: DepsView,
    pub loading: bool,
    pub loading_message: Option<String>,
    pub error: Option<String>,
    pub message: Option<String>,

    // Detected package manager
    pub package_manager: Option<PackageManager>,
    pub project_name: Option<String>,

    // Packages
    pub packages: Vec<PackageEntry>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub show_dev_only: bool,
    pub show_outdated_only: bool,

    // Search
    pub search_query: String,
    pub search_cursor: usize,
    pub search_results: Vec<SearchResult>,
    pub selected_result: usize,

    // Install input
    pub install_input: String,
    pub install_cursor: usize,
    pub install_as_dev: bool,

    // Command output
    pub command_output: Vec<String>,
    pub output_scroll: usize,

    // Confirmation
    pub confirm_action: Option<ConfirmAction>,

    // Stats
    pub outdated_count: usize,
    pub total_count: usize,
}

impl DepsState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_loading(&mut self, msg: &str) {
        self.loading = true;
        self.loading_message = Some(msg.to_string());
    }

    pub fn clear_loading(&mut self) {
        self.loading = false;
        self.loading_message = None;
    }

    pub fn reset(&mut self) {
        self.view = DepsView::List;
        self.loading = false;
        self.loading_message = None;
        self.error = None;
        self.message = None;
        self.packages.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.search_query.clear();
        self.search_cursor = 0;
        self.search_results.clear();
        self.selected_result = 0;
        self.install_input.clear();
        self.install_cursor = 0;
        self.install_as_dev = false;
        self.command_output.clear();
        self.output_scroll = 0;
        self.confirm_action = None;
        self.outdated_count = 0;
        self.total_count = 0;
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.ensure_visible();
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let max = self.visible_packages().len().saturating_sub(1);
        if self.selected_index < max {
            self.selected_index += 1;
            self.ensure_visible();
        }
    }

    /// Ensure selected item is visible
    fn ensure_visible(&mut self) {
        let visible_lines = 18;
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_lines {
            self.scroll_offset = self.selected_index - visible_lines + 1;
        }
    }

    /// Get visible packages based on filters
    pub fn visible_packages(&self) -> Vec<&PackageEntry> {
        self.packages
            .iter()
            .filter(|p| {
                if self.show_dev_only && !p.is_dev {
                    return false;
                }
                if self.show_outdated_only && !p.is_outdated {
                    return false;
                }
                true
            })
            .collect()
    }

    /// Get selected package
    pub fn selected_package(&self) -> Option<&PackageEntry> {
        self.visible_packages().get(self.selected_index).copied()
    }

    /// Insert character in search input
    pub fn insert_search_char(&mut self, c: char) {
        self.search_query.insert(self.search_cursor, c);
        self.search_cursor += 1;
    }

    /// Backspace in search input
    pub fn backspace_search(&mut self) {
        if self.search_cursor > 0 {
            self.search_cursor -= 1;
            self.search_query.remove(self.search_cursor);
        }
    }

    /// Insert character in install input
    pub fn insert_install_char(&mut self, c: char) {
        self.install_input.insert(self.install_cursor, c);
        self.install_cursor += 1;
    }

    /// Backspace in install input
    pub fn backspace_install(&mut self) {
        if self.install_cursor > 0 {
            self.install_cursor -= 1;
            self.install_input.remove(self.install_cursor);
        }
    }
}
