//! Games-specific configuration
//!
//! Stores game data (leaderboards, clicker state, stats, achievements) separately
//! from the main app config.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Games configuration data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GamesConfig {
    /// Whether sounds are enabled
    pub play_sounds: bool,
    /// Leaderboards data (base64 encoded JSON)
    #[serde(default)]
    pub leaderboards: Option<String>,
    /// Clicker state (base64 encoded JSON)
    #[serde(default)]
    pub clicker_state: Option<String>,
    /// Player stats (base64 encoded JSON)
    #[serde(default)]
    pub player_stats: Option<String>,
    /// Achievements (base64 encoded JSON)
    #[serde(default)]
    pub achievements: Option<String>,
}

impl GamesConfig {
    /// Get the config file path
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("rdos").join("games.json"))
    }

    /// Load config from file
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path().ok_or("Could not determine config directory")?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    /// Save config to file
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("Could not determine config directory")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())
    }

    /// Decode leaderboards from base64 JSON
    pub fn get_leaderboards<T: for<'de> Deserialize<'de> + Default>(&self) -> T {
        self.leaderboards
            .as_ref()
            .and_then(|s| {
                let decoded = base64::engine::general_purpose::STANDARD.decode(s).ok()?;
                serde_json::from_slice(&decoded).ok()
            })
            .unwrap_or_default()
    }

    /// Encode leaderboards to base64 JSON
    pub fn set_leaderboards<T: Serialize>(&mut self, leaderboards: &T) {
        if let Ok(json) = serde_json::to_vec(leaderboards) {
            self.leaderboards = Some(base64::engine::general_purpose::STANDARD.encode(&json));
        }
    }

    /// Decode clicker state from base64 JSON
    pub fn get_clicker_state<T: for<'de> Deserialize<'de> + Default>(&self) -> T {
        self.clicker_state
            .as_ref()
            .and_then(|s| {
                let decoded = base64::engine::general_purpose::STANDARD.decode(s).ok()?;
                serde_json::from_slice(&decoded).ok()
            })
            .unwrap_or_default()
    }

    /// Encode clicker state to base64 JSON
    pub fn set_clicker_state<T: Serialize>(&mut self, state: &T) {
        if let Ok(json) = serde_json::to_vec(state) {
            self.clicker_state = Some(base64::engine::general_purpose::STANDARD.encode(&json));
        }
    }

    /// Decode player stats from base64 JSON
    pub fn get_player_stats<T: for<'de> Deserialize<'de> + Default>(&self) -> T {
        self.player_stats
            .as_ref()
            .and_then(|s| {
                let decoded = base64::engine::general_purpose::STANDARD.decode(s).ok()?;
                serde_json::from_slice(&decoded).ok()
            })
            .unwrap_or_default()
    }

    /// Encode player stats to base64 JSON
    pub fn set_player_stats<T: Serialize>(&mut self, stats: &T) {
        if let Ok(json) = serde_json::to_vec(stats) {
            self.player_stats = Some(base64::engine::general_purpose::STANDARD.encode(&json));
        }
    }

    /// Decode achievements from base64 JSON
    pub fn get_achievements<T: for<'de> Deserialize<'de> + Default>(&self) -> T {
        self.achievements
            .as_ref()
            .and_then(|s| {
                let decoded = base64::engine::general_purpose::STANDARD.decode(s).ok()?;
                serde_json::from_slice(&decoded).ok()
            })
            .unwrap_or_default()
    }

    /// Encode achievements to base64 JSON
    pub fn set_achievements<T: Serialize>(&mut self, achievements: &T) {
        if let Ok(json) = serde_json::to_vec(achievements) {
            self.achievements = Some(base64::engine::general_purpose::STANDARD.encode(&json));
        }
    }
}
