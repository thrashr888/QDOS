//! Video Player plugin state types
//!
//! State for the video player plugin.

use std::path::PathBuf;

/// Available video players
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoPlayer {
    /// mpv - powerful media player
    Mpv,
    /// VLC - cross-platform media player
    Vlc,
    /// IINA - modern macOS media player
    Iina,
}

impl VideoPlayer {
    pub fn command(&self) -> &'static str {
        match self {
            VideoPlayer::Mpv => "mpv",
            VideoPlayer::Vlc => "vlc",
            VideoPlayer::Iina => "iina",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            VideoPlayer::Mpv => "mpv",
            VideoPlayer::Vlc => "VLC",
            VideoPlayer::Iina => "IINA",
        }
    }

    pub fn install_hint(&self) -> &'static str {
        match self {
            VideoPlayer::Mpv => "brew install mpv",
            VideoPlayer::Vlc => "brew install vlc",
            VideoPlayer::Iina => "brew install --cask iina",
        }
    }

    /// Ranking for recommendation (lower = better)
    pub fn rank(&self) -> u8 {
        match self {
            VideoPlayer::Mpv => 1,  // Preferred - terminal-friendly, lightweight
            VideoPlayer::Iina => 2, // macOS native, good UI
            VideoPlayer::Vlc => 3,  // Universal, but heavier
        }
    }

    /// Check if this player is available
    pub fn is_available(&self) -> bool {
        std::process::Command::new("which")
            .arg(self.command())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get all player variants in ranked order
    pub fn all() -> &'static [VideoPlayer] {
        &[VideoPlayer::Mpv, VideoPlayer::Iina, VideoPlayer::Vlc]
    }
}

/// Current view in the video plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoView {
    #[default]
    /// Main menu showing player selection
    Menu,
    /// Playing a video file
    Playing,
    /// Error state
    Error,
}

/// Video plugin state
#[derive(Debug, Clone, Default)]
pub struct VideoState {
    /// Current view
    pub view: VideoView,
    /// Available players (detected at init)
    pub available_players: Vec<VideoPlayer>,
    /// Currently selected player index
    pub selected_player: usize,
    /// File to play (if any)
    pub file_path: Option<PathBuf>,
    /// Error message (if any)
    pub error: Option<String>,
    /// Whether currently playing
    pub is_playing: bool,
}

impl VideoState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect available video players
    pub fn detect_players(&mut self) {
        self.available_players = VideoPlayer::all()
            .iter()
            .filter(|p| p.is_available())
            .copied()
            .collect();
        self.selected_player = 0;
    }

    /// Get the currently selected player
    pub fn selected(&self) -> Option<&VideoPlayer> {
        self.available_players.get(self.selected_player)
    }

    /// Select next player
    pub fn select_next(&mut self) {
        if !self.available_players.is_empty() {
            self.selected_player = (self.selected_player + 1) % self.available_players.len();
        }
    }

    /// Select previous player
    pub fn select_prev(&mut self) {
        if !self.available_players.is_empty() {
            self.selected_player = self
                .selected_player
                .checked_sub(1)
                .unwrap_or(self.available_players.len() - 1);
        }
    }
}
