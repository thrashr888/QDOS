//! Apps launcher plugin state types
//!
//! State for the F12 Apps launcher modal.

/// Plugin category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginCategory {
    Files,
    Vcs,
    Tools,
    Games,
    System,
}

impl PluginCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginCategory::Files => "Files",
            PluginCategory::Vcs => "VCS",
            PluginCategory::Tools => "Tools",
            PluginCategory::Games => "Games",
            PluginCategory::System => "System",
        }
    }

    /// Get all categories in display order
    pub fn all() -> &'static [PluginCategory] {
        &[
            PluginCategory::Files,
            PluginCategory::Vcs,
            PluginCategory::Tools,
            PluginCategory::Games,
            PluginCategory::System,
        ]
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
    /// Whether the tool is available on the system
    pub available: bool,
    /// Whether the plugin is enabled in config
    pub enabled: bool,
}

/// Apps launcher state
#[derive(Debug, Clone, Default)]
pub struct AppsState {
    pub apps: Vec<AppEntry>,
    pub selected_index: usize,
    pub filter: String,
    pub scroll_offset: usize,
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
        self.scroll_offset = 0;
    }

    /// Update scroll offset to keep selection visible
    pub fn update_scroll(&mut self, visible_height: usize) {
        // Ensure selected item is visible
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_index - visible_height + 1;
        }
    }

    /// Get apps grouped by category, returns (category, apps) pairs
    pub fn apps_by_category(&self) -> Vec<(PluginCategory, Vec<&AppEntry>)> {
        let filtered = self.filtered_apps();
        let mut result = Vec::new();

        for category in PluginCategory::all() {
            let category_apps: Vec<&AppEntry> = filtered
                .iter()
                .filter(|app| app.category == *category)
                .copied()
                .collect();

            if !category_apps.is_empty() {
                result.push((*category, category_apps));
            }
        }

        result
    }

    /// Get the flat index in filtered list for an item in category view
    pub fn flat_index_for_selection(&self, category_index: usize, item_index: usize) -> usize {
        let by_category = self.apps_by_category();
        let mut flat_idx = 0;

        for (i, (_cat, apps)) in by_category.iter().enumerate() {
            if i == category_index {
                return flat_idx + item_index;
            }
            flat_idx += apps.len();
        }

        0
    }

    /// Convert flat selected_index to (category_index, item_index)
    pub fn category_position(&self) -> Option<(usize, usize)> {
        let by_category = self.apps_by_category();
        let mut flat_idx = 0;

        for (cat_idx, (_cat, apps)) in by_category.iter().enumerate() {
            if self.selected_index < flat_idx + apps.len() {
                return Some((cat_idx, self.selected_index - flat_idx));
            }
            flat_idx += apps.len();
        }

        None
    }
}
