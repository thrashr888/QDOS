//! Apps launcher plugin state types
//!
//! State for the F12 Apps launcher modal.

/// Plugin category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginCategory {
    Files,
    Vcs,
    Tools,
    System,
}

impl PluginCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginCategory::Files => "Files",
            PluginCategory::Vcs => "VCS",
            PluginCategory::Tools => "Tools",
            PluginCategory::System => "System",
        }
    }
}

/// An app entry in the launcher
#[derive(Debug, Clone)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: PluginCategory,
    pub key: char,
    pub available: bool,
}

/// Apps launcher state
#[derive(Debug, Clone, Default)]
pub struct AppsState {
    pub apps: Vec<AppEntry>,
    pub selected_index: usize,
    pub filter: String,
}

impl AppsState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get filtered apps based on search filter
    pub fn filtered_apps(&self) -> Vec<&AppEntry> {
        if self.filter.is_empty() {
            self.apps.iter().collect()
        } else {
            let filter_lower = self.filter.to_lowercase();
            self.apps
                .iter()
                .filter(|app| {
                    app.name.to_lowercase().contains(&filter_lower)
                        || app.description.to_lowercase().contains(&filter_lower)
                })
                .collect()
        }
    }

    /// Get currently selected app
    pub fn selected_app(&self) -> Option<&AppEntry> {
        let filtered = self.filtered_apps();
        filtered.get(self.selected_index).copied()
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        let max = self.filtered_apps().len().saturating_sub(1);
        if self.selected_index < max {
            self.selected_index += 1;
        }
    }

    /// Clear filter and reset selection
    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.selected_index = 0;
    }
}
