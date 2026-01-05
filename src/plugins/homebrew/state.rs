//! Homebrew plugin state types
//!
//! State for the Homebrew modal showing packages and recommendations.

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

    pub fn all() -> Vec<PackageCategory> {
        vec![
            PackageCategory::Recommended,
            PackageCategory::Installed,
            PackageCategory::SearchResults,
        ]
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
    /// List view
    List,
    /// Search mode
    Search,
    /// Package details
    Details,
}

/// Homebrew plugin state
#[derive(Debug, Clone, Default)]
pub struct HomebrewState {
    pub packages: Vec<PackageEntry>,
    pub selected_index: usize,
    pub view: HomebrewView,
    pub search_query: String,
    pub loading: bool,
    pub error: Option<String>,
    pub homebrew_available: bool,
}

impl HomebrewState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the currently selected package
    pub fn selected_package(&self) -> Option<&PackageEntry> {
        self.filtered_packages().get(self.selected_index).copied()
    }

    /// Get filtered packages based on current view
    pub fn filtered_packages(&self) -> Vec<&PackageEntry> {
        if self.search_query.is_empty() {
            self.packages.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.packages
                .iter()
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

    /// Clear search
    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.selected_index = 0;
    }
}
