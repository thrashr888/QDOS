//! Video Player plugin state types
//!
//! State for the video player plugin.

use std::path::PathBuf;

/// A video frame with RGB data and dimensions
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: f32,
}

/// Render mode for inline video playback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    /// Use terminal image protocol (Kitty/Sixel/iTerm2) if available
    #[default]
    Image,
    /// Use colored ASCII art rendering
    Ascii,
}

/// Playback state for inline player
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

/// Inline playback state
#[derive(Debug, Default)]
pub struct InlinePlaybackState {
    /// Whether ffmpeg is available
    pub ffmpeg_available: bool,
    /// Current playback state
    pub play_state: PlayState,
    /// Current frame number
    pub current_frame: u64,
    /// Total frames (if known)
    pub total_frames: Option<u64>,
    /// Current position in seconds
    pub position: f32,
    /// Total duration in seconds
    pub duration: f32,
    /// Current frame (latest received)
    pub current_video_frame: Option<VideoFrame>,
    /// Target FPS for playback
    pub target_fps: u8,
    /// Render mode (image protocol vs ASCII)
    pub render_mode: RenderMode,
}

/// Available video players
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoPlayer {
    /// System default (uses macOS `open` command)
    SystemDefault,
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
            VideoPlayer::SystemDefault => "open",
            VideoPlayer::Mpv => "mpv",
            VideoPlayer::Vlc => "vlc",
            VideoPlayer::Iina => "iina",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            VideoPlayer::SystemDefault => "System Default",
            VideoPlayer::Mpv => "mpv",
            VideoPlayer::Vlc => "VLC",
            VideoPlayer::Iina => "IINA",
        }
    }

    pub fn install_hint(&self) -> &'static str {
        match self {
            VideoPlayer::SystemDefault => "(built-in)",
            VideoPlayer::Mpv => "brew install mpv",
            VideoPlayer::Vlc => "brew install vlc",
            VideoPlayer::Iina => "brew install --cask iina",
        }
    }

    /// Ranking for recommendation (lower = better)
    pub fn rank(&self) -> u8 {
        match self {
            VideoPlayer::SystemDefault => 1, // Always available
            VideoPlayer::Mpv => 2,           // Lightweight, terminal-friendly
            VideoPlayer::Iina => 3,          // macOS native, good UI
            VideoPlayer::Vlc => 4,           // Universal, but heavier
        }
    }

    /// Check if this player is available
    pub fn is_available(&self) -> bool {
        match self {
            VideoPlayer::SystemDefault => {
                // Always available on macOS
                cfg!(target_os = "macos")
            }
            _ => std::process::Command::new("which")
                .arg(self.command())
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false),
        }
    }

    /// Get all player variants in ranked order (mpv first as best for terminal use)
    pub fn all() -> &'static [VideoPlayer] {
        &[
            VideoPlayer::Mpv,
            VideoPlayer::Iina,
            VideoPlayer::Vlc,
            VideoPlayer::SystemDefault,
        ]
    }
}

/// Current view in the video plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoView {
    #[default]
    /// Main menu showing player selection
    Menu,
    /// Playing a video file (external player)
    Playing,
    /// Inline video playback
    InlinePlayer,
    /// FFmpeg not found
    FfmpegMissing,
    /// Error state
    Error,
}

/// Video plugin state
#[derive(Debug, Default)]
pub struct VideoState {
    /// Current view
    pub view: VideoView,
    /// Available players (detected at init)
    pub available_players: Vec<VideoPlayer>,
    /// Currently selected player index
    pub selected_player: usize,
    /// File to play (if any)
    pub file_path: Option<PathBuf>,
    /// File name for display
    pub file_name: String,
    /// Error message (if any)
    pub error: Option<String>,
    /// Whether currently playing
    pub is_playing: bool,
    /// Sibling video files in the same directory
    pub sibling_files: Vec<PathBuf>,
    /// Current file index in sibling_files
    pub current_file_index: usize,
    /// Inline playback state
    pub inline_state: InlinePlaybackState,
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

    /// Detect sibling video files in the same directory
    pub fn detect_siblings(&mut self) {
        let Some(ref file_path) = self.file_path else {
            return;
        };

        let Some(parent) = file_path.parent() else {
            return;
        };

        // Find all video files in the directory
        let mut siblings: Vec<PathBuf> = std::fs::read_dir(parent)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_video_file(p))
            .collect();

        // Sort alphabetically
        siblings.sort();

        // Find current file index
        self.current_file_index = siblings.iter().position(|p| p == file_path).unwrap_or(0);

        self.sibling_files = siblings;
    }

    /// Check if there's a previous file
    pub fn has_prev(&self) -> bool {
        self.current_file_index > 0
    }

    /// Check if there's a next file
    pub fn has_next(&self) -> bool {
        !self.sibling_files.is_empty() && self.current_file_index < self.sibling_files.len() - 1
    }

    /// Get previous file path
    pub fn prev_file(&self) -> Option<PathBuf> {
        if self.has_prev() {
            self.sibling_files.get(self.current_file_index - 1).cloned()
        } else {
            None
        }
    }

    /// Get next file path
    pub fn next_file(&self) -> Option<PathBuf> {
        if self.has_next() {
            self.sibling_files.get(self.current_file_index + 1).cloned()
        } else {
            None
        }
    }

    /// Get file position string (e.g., "3/10")
    pub fn file_position(&self) -> String {
        if self.sibling_files.is_empty() {
            String::new()
        } else {
            format!(
                "{}/{}",
                self.current_file_index + 1,
                self.sibling_files.len()
            )
        }
    }

    /// Toggle between image and ASCII render modes
    pub fn toggle_render_mode(&mut self) {
        self.inline_state.render_mode = match self.inline_state.render_mode {
            RenderMode::Image => RenderMode::Ascii,
            RenderMode::Ascii => RenderMode::Image,
        };
    }
}

/// Check if a file is a video file
pub fn is_video_file(path: &PathBuf) -> bool {
    path.extension()
        .map(|ext| {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            matches!(
                ext_lower.as_str(),
                "mp4"
                    | "mkv"
                    | "avi"
                    | "mov"
                    | "wmv"
                    | "flv"
                    | "webm"
                    | "m4v"
                    | "mpg"
                    | "mpeg"
                    | "3gp"
                    | "ogv"
                    | "ts"
                    | "mts"
                    | "vob"
            )
        })
        .unwrap_or(false)
}
