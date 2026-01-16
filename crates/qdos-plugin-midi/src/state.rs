//! MIDI Player plugin state types
//!
//! State for the MIDI player plugin.

use std::path::PathBuf;

/// Available MIDI players
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiPlayer {
    /// System default (uses macOS `open` command)
    SystemDefault,
    /// FluidSynth - software synthesizer
    FluidSynth,
    /// TiMidity++ - MIDI to WAV converter/player
    Timidity,
}

impl MidiPlayer {
    pub fn command(&self) -> &'static str {
        match self {
            MidiPlayer::SystemDefault => "open",
            MidiPlayer::FluidSynth => "fluidsynth",
            MidiPlayer::Timidity => "timidity",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            MidiPlayer::SystemDefault => "System Default",
            MidiPlayer::FluidSynth => "FluidSynth",
            MidiPlayer::Timidity => "TiMidity++",
        }
    }

    pub fn install_hint(&self) -> &'static str {
        match self {
            MidiPlayer::SystemDefault => "(built-in)",
            MidiPlayer::FluidSynth => "brew install fluid-synth",
            MidiPlayer::Timidity => "brew install timidity",
        }
    }

    /// Ranking for recommendation (lower = better)
    pub fn rank(&self) -> u8 {
        match self {
            MidiPlayer::SystemDefault => 1, // Always available, easiest
            MidiPlayer::FluidSynth => 2,    // Good quality if configured
            MidiPlayer::Timidity => 3,
        }
    }

    /// Check if this player is available
    pub fn is_available(&self) -> bool {
        match self {
            MidiPlayer::SystemDefault => {
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

    /// Get all player variants in ranked order
    pub fn all() -> &'static [MidiPlayer] {
        &[
            MidiPlayer::SystemDefault,
            MidiPlayer::FluidSynth,
            MidiPlayer::Timidity,
        ]
    }
}

/// Current view in the MIDI plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MidiView {
    #[default]
    /// Main menu showing player selection
    Menu,
    /// Playing a MIDI file
    Playing,
    /// Error state
    Error,
}

/// MIDI plugin state
#[derive(Debug, Clone, Default)]
pub struct MidiState {
    /// Current view
    pub view: MidiView,
    /// Available players (detected at init)
    pub available_players: Vec<MidiPlayer>,
    /// Currently selected player index
    pub selected_player: usize,
    /// File to play (if any)
    pub file_path: Option<PathBuf>,
    /// Error message (if any)
    pub error: Option<String>,
    /// Whether currently playing
    pub is_playing: bool,
}

impl MidiState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect available MIDI players
    pub fn detect_players(&mut self) {
        self.available_players = MidiPlayer::all()
            .iter()
            .filter(|p| p.is_available())
            .copied()
            .collect();
        self.selected_player = 0;
    }

    /// Get the currently selected player
    pub fn selected(&self) -> Option<&MidiPlayer> {
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
