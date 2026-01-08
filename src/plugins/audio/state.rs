//! Audio Player plugin state types

use std::path::PathBuf;

/// Audio file type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioType {
    #[default]
    /// Native audio (wav, mp3, ogg, flac) - can be played with rodio
    Native,
    /// MIDI files - need external players
    Midi,
}

/// Available audio players
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioPlayer {
    /// Built-in player using rodio (for native audio only)
    Builtin,
    /// System default (uses macOS `open` command) - works for all types
    SystemDefault,
    /// FluidSynth - software synthesizer (MIDI only)
    FluidSynth,
    /// TiMidity++ - MIDI to audio converter (MIDI only)
    Timidity,
}

impl AudioPlayer {
    pub fn command(&self) -> &'static str {
        match self {
            AudioPlayer::Builtin => "builtin",
            AudioPlayer::SystemDefault => "open",
            AudioPlayer::FluidSynth => "fluidsynth",
            AudioPlayer::Timidity => "timidity",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            AudioPlayer::Builtin => "Built-in Player",
            AudioPlayer::SystemDefault => "System Default",
            AudioPlayer::FluidSynth => "FluidSynth",
            AudioPlayer::Timidity => "TiMidity++",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AudioPlayer::Builtin => "Native playback with controls",
            AudioPlayer::SystemDefault => "Open with default app",
            AudioPlayer::FluidSynth => "Software synthesizer",
            AudioPlayer::Timidity => "MIDI to audio converter",
        }
    }

    /// Check if this player is available
    pub fn is_available(&self) -> bool {
        match self {
            AudioPlayer::Builtin => true, // Always available
            AudioPlayer::SystemDefault => cfg!(target_os = "macos"),
            _ => std::process::Command::new("which")
                .arg(self.command())
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false),
        }
    }

    /// Check if this player supports the given audio type
    pub fn supports(&self, audio_type: AudioType) -> bool {
        match self {
            AudioPlayer::Builtin => audio_type == AudioType::Native,
            AudioPlayer::SystemDefault => true, // Supports everything
            AudioPlayer::FluidSynth | AudioPlayer::Timidity => audio_type == AudioType::Midi,
        }
    }

    /// Get all player variants (FluidSynth first for MIDI priority)
    pub fn all() -> &'static [AudioPlayer] {
        &[
            AudioPlayer::FluidSynth,
            AudioPlayer::Timidity,
            AudioPlayer::Builtin,
            AudioPlayer::SystemDefault,
        ]
    }

    /// Get available players for a given audio type
    pub fn available_for(audio_type: AudioType) -> Vec<AudioPlayer> {
        Self::all()
            .iter()
            .filter(|p| p.is_available() && p.supports(audio_type))
            .copied()
            .collect()
    }
}

/// Playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

/// Current view in the audio plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioView {
    #[default]
    /// Player selection menu
    Menu,
    /// Native player view showing controls (for builtin player)
    Player,
    /// External player launched
    ExternalPlaying,
    /// Soundfont needed for MIDI playback
    NeedsSoundFont,
    /// Downloading soundfont
    DownloadingSoundFont,
    /// Error state
    Error,
}

/// Audio plugin state
#[derive(Debug, Clone, Default)]
pub struct AudioState {
    /// Current view
    pub view: AudioView,
    /// File to play (if any)
    pub file_path: Option<PathBuf>,
    /// File name being played
    pub file_name: String,
    /// Audio type (native or MIDI)
    pub audio_type: AudioType,
    /// Current playback state
    pub play_state: PlayState,
    /// Current position in seconds
    pub position: f32,
    /// Total duration in seconds
    pub duration: f32,
    /// Volume (0.0 to 1.0)
    pub volume: f32,
    /// Error message (if any)
    pub error: Option<String>,
    /// Available players for this file type
    pub available_players: Vec<AudioPlayer>,
    /// Selected player index
    pub selected_player: usize,
    /// Sibling audio files in the same directory
    pub sibling_files: Vec<PathBuf>,
    /// Current file index in sibling_files
    pub current_file_index: usize,
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            volume: 0.8,
            ..Default::default()
        }
    }

    /// Detect available players for the current audio type
    pub fn detect_players(&mut self) {
        self.available_players = AudioPlayer::available_for(self.audio_type);
        self.selected_player = 0;
    }

    /// Get selected player
    pub fn selected(&self) -> Option<&AudioPlayer> {
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

    /// Format position as MM:SS
    pub fn format_position(&self) -> String {
        let mins = (self.position as u32) / 60;
        let secs = (self.position as u32) % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    /// Format duration as MM:SS
    pub fn format_duration(&self) -> String {
        let mins = (self.duration as u32) / 60;
        let secs = (self.duration as u32) % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    /// Get progress as 0.0 to 1.0
    pub fn progress(&self) -> f32 {
        if self.duration > 0.0 {
            (self.position / self.duration).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Detect sibling audio files in the same directory
    pub fn detect_siblings(&mut self) {
        let Some(ref file_path) = self.file_path else {
            return;
        };

        let Some(parent) = file_path.parent() else {
            return;
        };

        // Find all audio files in the directory
        let mut siblings: Vec<PathBuf> = std::fs::read_dir(parent)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_audio_file(p))
            .collect();

        // Sort alphabetically
        siblings.sort();

        // Find current file index
        self.current_file_index = siblings
            .iter()
            .position(|p| p == file_path)
            .unwrap_or(0);

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
            format!("{}/{}", self.current_file_index + 1, self.sibling_files.len())
        }
    }
}

/// Check if a file is an audio file (native or MIDI)
pub fn is_audio_file(path: &PathBuf) -> bool {
    path.extension()
        .map(|ext| {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            matches!(
                ext_lower.as_str(),
                "wav" | "wave" | "mp3" | "ogg" | "oga" | "flac" | "aac" | "m4a" | "mid" | "midi"
            )
        })
        .unwrap_or(false)
}

/// Get audio type from file path
pub fn get_audio_type(path: &PathBuf) -> AudioType {
    path.extension()
        .map(|ext| {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            if ext_lower == "mid" || ext_lower == "midi" {
                AudioType::Midi
            } else {
                AudioType::Native
            }
        })
        .unwrap_or(AudioType::Native)
}
