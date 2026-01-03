//! Q-DOS II Configuration Module
//!
//! Handles loading and saving of configuration from ~/.config/rdos/config.toml

use crate::app::{ColorTheme, SortMode};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Main configuration struct
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub plugins: PluginsConfig,
}

/// General settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// File search pattern (default: *.*)
    #[serde(default = "default_search_spec")]
    pub search_spec: String,
    /// Sort method: name, ext, size, date, none
    #[serde(default)]
    pub sort_method: SortMethodConfig,
    /// Sort direction: asc or desc
    #[serde(default)]
    pub sort_direction: SortDirection,
    /// Show hidden files (default: false)
    #[serde(default)]
    pub show_hidden: bool,
    /// Confirm before delete (default: true)
    #[serde(default = "default_true")]
    pub confirm_delete: bool,
    /// Enable mouse support (default: false)
    #[serde(default)]
    pub mouse_support: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            search_spec: default_search_spec(),
            sort_method: SortMethodConfig::default(),
            sort_direction: SortDirection::default(),
            show_hidden: false,
            confirm_delete: true,
            mouse_support: false,
        }
    }
}

/// Display settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisplayConfig {
    /// Color theme name
    #[serde(default)]
    pub theme: ThemeConfig,
    /// Show filenames in uppercase (default: false)
    #[serde(default)]
    pub uppercase_names: bool,
}

/// Editor settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EditorConfig {
    /// Default editor command (uses $EDITOR if not set)
    #[serde(default)]
    pub command: Option<String>,
}

/// Plugin settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginsConfig {
    /// List of explicitly enabled plugin IDs
    /// If empty, all available plugins are enabled by default
    #[serde(default)]
    pub enabled: Vec<String>,
    /// List of explicitly disabled plugin IDs
    /// Takes precedence over enabled list
    #[serde(default)]
    pub disabled: Vec<String>,
    /// Plugin-specific settings as key-value pairs
    /// Keys are plugin IDs, values are plugin-specific config tables
    #[serde(default)]
    pub settings: HashMap<String, PluginSettings>,
}

impl Default for PluginsConfig {
    #[allow(clippy::derivable_impls)]
    fn default() -> Self {
        // Empty enabled list means all plugins are enabled by default
        Self {
            enabled: vec![],
            disabled: vec![],
            settings: HashMap::new(),
        }
    }
}

/// Plugin-specific settings container
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginSettings {
    /// Whether the plugin is enabled (overrides global enabled/disabled lists)
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Plugin-specific key-value settings
    #[serde(flatten)]
    pub options: HashMap<String, toml::Value>,
}

impl PluginsConfig {
    /// Check if a plugin is enabled based on the configuration
    pub fn is_plugin_enabled(&self, plugin_id: &str) -> bool {
        // Check explicit disabled list first (takes precedence)
        if self.disabled.contains(&plugin_id.to_string()) {
            return false;
        }

        // Check plugin-specific settings
        if let Some(settings) = self.settings.get(plugin_id) {
            if let Some(enabled) = settings.enabled {
                return enabled;
            }
        }

        // If enabled list is empty, all plugins are enabled by default
        if self.enabled.is_empty() {
            return true;
        }

        // Check if in enabled list
        self.enabled.contains(&plugin_id.to_string())
    }

    /// Get plugin-specific settings
    #[allow(dead_code)]
    pub fn get_plugin_settings(&self, plugin_id: &str) -> Option<&PluginSettings> {
        self.settings.get(plugin_id)
    }

    /// Get a specific option value for a plugin
    #[allow(dead_code)]
    pub fn get_plugin_option(&self, plugin_id: &str, key: &str) -> Option<&toml::Value> {
        self.settings.get(plugin_id)?.options.get(key)
    }
}

/// Sort method for config serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortMethodConfig {
    #[default]
    Name,
    Ext,
    Size,
    Date,
    None,
}

#[allow(dead_code)]
impl SortMethodConfig {
    pub fn as_str(&self) -> &'static str {
        match self {
            SortMethodConfig::Name => "name",
            SortMethodConfig::Ext => "ext",
            SortMethodConfig::Size => "size",
            SortMethodConfig::Date => "date",
            SortMethodConfig::None => "none",
        }
    }

    pub fn all() -> &'static [SortMethodConfig] {
        &[
            SortMethodConfig::Name,
            SortMethodConfig::Ext,
            SortMethodConfig::Size,
            SortMethodConfig::Date,
            SortMethodConfig::None,
        ]
    }
}

/// Sort direction for config serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

#[allow(dead_code)]
impl SortDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            SortDirection::Asc => "asc",
            SortDirection::Desc => "desc",
        }
    }
}

/// Theme configuration for serialization
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeConfig {
    #[default]
    Default,
    Monochrome,
    Blue,
    Green,
    Amber,
}

#[allow(dead_code)]
impl ThemeConfig {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeConfig::Default => "default",
            ThemeConfig::Monochrome => "monochrome",
            ThemeConfig::Blue => "blue",
            ThemeConfig::Green => "green",
            ThemeConfig::Amber => "amber",
        }
    }

    pub fn all() -> &'static [ThemeConfig] {
        &[
            ThemeConfig::Default,
            ThemeConfig::Monochrome,
            ThemeConfig::Blue,
            ThemeConfig::Green,
            ThemeConfig::Amber,
        ]
    }
}

impl From<ColorTheme> for ThemeConfig {
    fn from(theme: ColorTheme) -> Self {
        match theme {
            ColorTheme::Default => ThemeConfig::Default,
            ColorTheme::Monochrome => ThemeConfig::Monochrome,
            ColorTheme::Blue => ThemeConfig::Blue,
            ColorTheme::Green => ThemeConfig::Green,
            ColorTheme::Amber => ThemeConfig::Amber,
        }
    }
}

impl From<ThemeConfig> for ColorTheme {
    fn from(config: ThemeConfig) -> Self {
        match config {
            ThemeConfig::Default => ColorTheme::Default,
            ThemeConfig::Monochrome => ColorTheme::Monochrome,
            ThemeConfig::Blue => ColorTheme::Blue,
            ThemeConfig::Green => ColorTheme::Green,
            ThemeConfig::Amber => ColorTheme::Amber,
        }
    }
}

// Default value helpers for serde
fn default_search_spec() -> String {
    "*.*".to_string()
}

fn default_true() -> bool {
    true
}

impl Config {
    /// Get the config file path (~/.config/rdos/config.toml)
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("rdos");
        Ok(config_dir.join("config.toml"))
    }

    /// Load config from file, or return defaults if file doesn't exist
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Create config directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Convert config sort settings to SortMode
    pub fn to_sort_mode(&self) -> SortMode {
        match (self.general.sort_method, self.general.sort_direction) {
            (SortMethodConfig::Name, SortDirection::Asc) => SortMode::NameAsc,
            (SortMethodConfig::Name, SortDirection::Desc) => SortMode::NameDesc,
            (SortMethodConfig::Ext, SortDirection::Asc) => SortMode::ExtAsc,
            (SortMethodConfig::Ext, SortDirection::Desc) => SortMode::ExtDesc,
            (SortMethodConfig::Size, SortDirection::Asc) => SortMode::SizeAsc,
            (SortMethodConfig::Size, SortDirection::Desc) => SortMode::SizeDesc,
            (SortMethodConfig::Date, SortDirection::Asc) => SortMode::DateAsc,
            (SortMethodConfig::Date, SortDirection::Desc) => SortMode::DateDesc,
            (SortMethodConfig::None, _) => SortMode::None,
        }
    }

    /// Update config from SortMode
    pub fn from_sort_mode(&mut self, mode: SortMode) {
        match mode {
            SortMode::NameAsc => {
                self.general.sort_method = SortMethodConfig::Name;
                self.general.sort_direction = SortDirection::Asc;
            }
            SortMode::NameDesc => {
                self.general.sort_method = SortMethodConfig::Name;
                self.general.sort_direction = SortDirection::Desc;
            }
            SortMode::ExtAsc => {
                self.general.sort_method = SortMethodConfig::Ext;
                self.general.sort_direction = SortDirection::Asc;
            }
            SortMode::ExtDesc => {
                self.general.sort_method = SortMethodConfig::Ext;
                self.general.sort_direction = SortDirection::Desc;
            }
            SortMode::SizeAsc => {
                self.general.sort_method = SortMethodConfig::Size;
                self.general.sort_direction = SortDirection::Asc;
            }
            SortMode::SizeDesc => {
                self.general.sort_method = SortMethodConfig::Size;
                self.general.sort_direction = SortDirection::Desc;
            }
            SortMode::DateAsc => {
                self.general.sort_method = SortMethodConfig::Date;
                self.general.sort_direction = SortDirection::Asc;
            }
            SortMode::DateDesc => {
                self.general.sort_method = SortMethodConfig::Date;
                self.general.sort_direction = SortDirection::Desc;
            }
            SortMode::None => {
                self.general.sort_method = SortMethodConfig::None;
            }
        }
    }

    /// Get the editor command (config value or $EDITOR)
    #[allow(dead_code)]
    pub fn editor_command(&self) -> String {
        self.editor
            .command
            .clone()
            .or_else(|| std::env::var("EDITOR").ok())
            .unwrap_or_else(|| "vi".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.general.search_spec, "*.*");
        assert!(config.general.confirm_delete);
        assert!(!config.general.show_hidden);
        assert_eq!(config.display.theme, ThemeConfig::Default);
    }

    #[test]
    fn test_sort_mode_conversion() {
        let mut config = Config::default();

        config.from_sort_mode(SortMode::SizeDesc);
        assert_eq!(config.general.sort_method, SortMethodConfig::Size);
        assert_eq!(config.general.sort_direction, SortDirection::Desc);

        assert_eq!(config.to_sort_mode(), SortMode::SizeDesc);
    }

    #[test]
    fn test_theme_conversion() {
        let theme = ColorTheme::Blue;
        let config: ThemeConfig = theme.into();
        assert_eq!(config, ThemeConfig::Blue);

        let back: ColorTheme = config.into();
        assert_eq!(back, ColorTheme::Blue);
    }

    #[test]
    fn test_toml_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[general]"));
        assert!(toml_str.contains("[display]"));
    }

    #[test]
    fn test_plugin_config_default() {
        let config = PluginsConfig::default();
        // All plugins enabled by default when lists are empty
        assert!(config.is_plugin_enabled("git"));
        assert!(config.is_plugin_enabled("beads"));
        assert!(config.is_plugin_enabled("any_plugin"));
    }

    #[test]
    fn test_plugin_config_disabled_list() {
        let config = PluginsConfig {
            enabled: vec![],
            disabled: vec!["git".to_string()],
            settings: HashMap::new(),
        };
        assert!(!config.is_plugin_enabled("git"));
        assert!(config.is_plugin_enabled("beads"));
    }

    #[test]
    fn test_plugin_config_enabled_list() {
        let config = PluginsConfig {
            enabled: vec!["git".to_string()],
            disabled: vec![],
            settings: HashMap::new(),
        };
        assert!(config.is_plugin_enabled("git"));
        assert!(!config.is_plugin_enabled("beads"));
    }

    #[test]
    fn test_plugin_config_disabled_takes_precedence() {
        let config = PluginsConfig {
            enabled: vec!["git".to_string()],
            disabled: vec!["git".to_string()],
            settings: HashMap::new(),
        };
        // Disabled takes precedence
        assert!(!config.is_plugin_enabled("git"));
    }

    #[test]
    fn test_plugin_specific_enabled_override() {
        let mut settings = HashMap::new();
        settings.insert(
            "git".to_string(),
            PluginSettings {
                enabled: Some(false),
                options: HashMap::new(),
            },
        );
        let config = PluginsConfig {
            enabled: vec![],
            disabled: vec![],
            settings,
        };
        // Plugin-specific enabled=false overrides default
        assert!(!config.is_plugin_enabled("git"));
    }

    #[test]
    fn test_plugin_config_toml_parsing() {
        let toml_str = r#"
            [plugins]
            disabled = ["experimental"]

            [plugins.settings.git]
            enabled = true
            show_branch = true

            [plugins.settings.beads]
            auto_sync = false
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.plugins.is_plugin_enabled("git"));
        assert!(!config.plugins.is_plugin_enabled("experimental"));

        let git_settings = config.plugins.get_plugin_settings("git").unwrap();
        assert_eq!(git_settings.enabled, Some(true));
        assert_eq!(
            git_settings.options.get("show_branch"),
            Some(&toml::Value::Boolean(true))
        );
    }
}
