//! Homebrew plugin state types
//!
//! State for the Homebrew modal showing packages and recommendations.

/// Tab selection for the main view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HomebrewTab {
    #[default]
    /// Recommended packages for QDOS
    Recommended,
    /// All installed packages
    Installed,
    /// Search results
    Search,
}

impl HomebrewTab {
    pub fn as_str(&self) -> &'static str {
        match self {
            HomebrewTab::Recommended => "Recommended",
            HomebrewTab::Installed => "Installed",
            HomebrewTab::Search => "Search",
        }
    }

    pub fn all() -> &'static [HomebrewTab] {
        &[
            HomebrewTab::Recommended,
            HomebrewTab::Installed,
            HomebrewTab::Search,
        ]
    }

    pub fn next(&self) -> Self {
        match self {
            HomebrewTab::Recommended => HomebrewTab::Installed,
            HomebrewTab::Installed => HomebrewTab::Search,
            HomebrewTab::Search => HomebrewTab::Recommended,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            HomebrewTab::Recommended => HomebrewTab::Search,
            HomebrewTab::Installed => HomebrewTab::Recommended,
            HomebrewTab::Search => HomebrewTab::Installed,
        }
    }
}

/// Package category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageCategory {
    /// Recommended tools for QDOS
    Recommended,
    /// Currently installed packages
    Installed,
    /// Search results
    SearchResults,
}

impl PackageCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            PackageCategory::Recommended => "Recommended",
            PackageCategory::Installed => "Installed",
            PackageCategory::SearchResults => "Search Results",
        }
    }
}

/// Package status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageStatus {
    /// Available for install
    Available,
    /// Currently installed
    Installed,
    /// Has update available
    Outdated,
    /// Installing in progress
    Installing,
}

impl PackageStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            PackageStatus::Available => " ",
            PackageStatus::Installed => "*",
            PackageStatus::Outdated => "^",
            PackageStatus::Installing => "~",
        }
    }
}

/// A Homebrew package entry
#[derive(Debug, Clone)]
pub struct PackageEntry {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub installed_version: Option<String>,
    pub category: PackageCategory,
    pub status: PackageStatus,
}

impl PackageEntry {
    /// Check if package has update available
    pub fn has_update(&self) -> bool {
        if let (Some(ver), Some(inst)) = (&self.version, &self.installed_version) {
            ver != inst
        } else {
            false
        }
    }
}

/// Homebrew modal view mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HomebrewView {
    #[default]
    /// List view (tabbed: Recommended/Installed/Search)
    List,
    /// Search input mode
    SearchInput,
    /// Package info/details view
    Info,
    /// Confirm action (install/uninstall/upgrade)
    Confirm,
    /// Show command output
    Output,
}

/// Action to confirm
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    Install(String),
    Uninstall(String),
    Upgrade(String),
    UpgradeAll,
    Update,
}

impl ConfirmAction {
    pub fn message(&self) -> String {
        match self {
            ConfirmAction::Install(pkg) => format!("Install {}?", pkg),
            ConfirmAction::Uninstall(pkg) => format!("Uninstall {}?", pkg),
            ConfirmAction::Upgrade(pkg) => format!("Upgrade {}?", pkg),
            ConfirmAction::UpgradeAll => "Upgrade all outdated packages?".to_string(),
            ConfirmAction::Update => "Update Homebrew package list?".to_string(),
        }
    }

    pub fn command(&self) -> String {
        match self {
            ConfirmAction::Install(pkg) => format!("brew install {}", pkg),
            ConfirmAction::Uninstall(pkg) => format!("brew uninstall {}", pkg),
            ConfirmAction::Upgrade(pkg) => format!("brew upgrade {}", pkg),
            ConfirmAction::UpgradeAll => "brew upgrade".to_string(),
            ConfirmAction::Update => "brew update".to_string(),
        }
    }
}

/// Package info from `brew info`
#[derive(Debug, Clone, Default)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub homepage: String,
    pub installed: bool,
    pub installed_version: Option<String>,
    pub dependencies: Vec<String>,
    pub caveats: Option<String>,
}

/// Homebrew plugin state
#[derive(Debug, Clone, Default)]
pub struct HomebrewState {
    /// All packages (recommended + installed + search results)
    pub packages: Vec<PackageEntry>,
    /// Currently selected index in filtered list
    pub selected_index: usize,
    /// Current view mode
    pub view: HomebrewView,
    /// Current tab in list view
    pub tab: HomebrewTab,
    /// Search/filter query
    pub search_query: String,
    /// Loading indicator
    pub loading: bool,
    /// Loading message
    pub loading_message: Option<String>,
    /// Error message
    pub error: Option<String>,
    /// Whether Homebrew is available
    pub homebrew_available: bool,
    /// Package info for Info view
    pub package_info: Option<PackageInfo>,
    /// Pending action to confirm
    pub confirm_action: Option<ConfirmAction>,
    /// Outdated packages count
    pub outdated_count: usize,
    /// Filter to show only outdated packages
    pub show_outdated_only: bool,
    /// Command that was run (for output view)
    pub last_command: Option<String>,
    /// Output from last command (for output view)
    pub command_output: Option<String>,
    /// Scroll position in output view
    pub output_scroll: usize,
}

impl HomebrewState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the currently selected package
    pub fn selected_package(&self) -> Option<&PackageEntry> {
        self.filtered_packages().get(self.selected_index).copied()
    }

    /// Get filtered packages based on current tab and search query
    pub fn filtered_packages(&self) -> Vec<&PackageEntry> {
        let base: Vec<&PackageEntry> = match self.tab {
            HomebrewTab::Recommended => self
                .packages
                .iter()
                .filter(|p| p.category == PackageCategory::Recommended)
                .collect(),
            HomebrewTab::Installed => self
                .packages
                .iter()
                .filter(|p| {
                    p.status == PackageStatus::Installed || p.status == PackageStatus::Outdated
                })
                .collect(),
            HomebrewTab::Search => self
                .packages
                .iter()
                .filter(|p| p.category == PackageCategory::SearchResults)
                .collect(),
        };

        // Apply outdated filter if enabled
        let filtered: Vec<&PackageEntry> = if self.show_outdated_only {
            base.into_iter()
                .filter(|p| p.status == PackageStatus::Outdated)
                .collect()
        } else {
            base
        };

        // Apply search filter if present
        if self.search_query.is_empty() {
            filtered
        } else {
            let query = self.search_query.to_lowercase();
            filtered
                .into_iter()
                .filter(|p| {
                    p.name.to_lowercase().contains(&query)
                        || p.description.to_lowercase().contains(&query)
                })
                .collect()
        }
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        let max = self.filtered_packages().len().saturating_sub(1);
        if self.selected_index < max {
            self.selected_index += 1;
        }
    }

    /// Switch to next tab
    pub fn next_tab(&mut self) {
        self.tab = self.tab.next();
        self.selected_index = 0;
        // Clear search when switching tabs (except to Search tab)
        if self.tab != HomebrewTab::Search {
            self.search_query.clear();
        }
    }

    /// Switch to previous tab
    pub fn prev_tab(&mut self) {
        self.tab = self.tab.prev();
        self.selected_index = 0;
        // Clear search when switching tabs (except to Search tab)
        if self.tab != HomebrewTab::Search {
            self.search_query.clear();
        }
    }

    /// Clear search
    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.selected_index = 0;
    }

    /// Set loading state with message
    pub fn set_loading(&mut self, message: &str) {
        self.loading = true;
        self.loading_message = Some(message.to_string());
    }

    /// Clear loading state
    pub fn clear_loading(&mut self) {
        self.loading = false;
        self.loading_message = None;
    }

    /// Count packages by status
    pub fn count_installed(&self) -> usize {
        self.packages
            .iter()
            .filter(|p| p.status == PackageStatus::Installed || p.status == PackageStatus::Outdated)
            .count()
    }
}
