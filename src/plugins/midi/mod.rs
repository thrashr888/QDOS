//! MIDI Player plugin
//!
//! Play MIDI files using system MIDI players (FluidSynth, TiMidity++).

mod modal;
pub mod state;

use crate::plugins::{
    KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem, PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{MidiState, MidiView};
use std::any::Any;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// MIDI Player plugin
pub struct MidiPlugin {
    initialized: bool,
    pub state: MidiState,
    /// Currently playing process (if any)
    player_process: Option<Child>,
}

impl Default for MidiPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MidiPlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
            state: MidiState::new(),
            player_process: None,
        }
    }

    /// Open the modal for a specific .mid file
    pub fn open_modal(&mut self, file_path: Option<&PathBuf>) {
        self.state.detect_players();
        self.state.file_path = file_path.cloned();
        self.state.view = MidiView::Menu;
        self.state.error = None;

        // Auto-play if only one player and a file is selected
        if self.state.available_players.len() == 1 && self.state.file_path.is_some() {
            self.play_midi();
        }
    }

    /// Play the MIDI file with the selected player
    fn play_midi(&mut self) {
        let Some(player) = self.state.selected().copied() else {
            self.state.error = Some("No player selected".to_string());
            self.state.view = MidiView::Error;
            return;
        };

        let Some(file_path) = &self.state.file_path else {
            self.state.error = Some("No file selected".to_string());
            self.state.view = MidiView::Error;
            return;
        };

        self.state.view = MidiView::Playing;
        self.state.is_playing = true;

        // Build command based on player
        let result = match player {
            state::MidiPlayer::SystemDefault => {
                // Use macOS `open` to play with default MIDI app
                Command::new("open").arg(file_path).spawn()
            }
            state::MidiPlayer::FluidSynth => {
                // FluidSynth needs a soundfont - try common locations
                let soundfont = find_soundfont();
                if let Some(sf) = soundfont {
                    Command::new("fluidsynth")
                        .args(["-a", "coreaudio", "-m", "coremidi", "-ni"])
                        .arg(&sf)
                        .arg(file_path)
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "No soundfont (.sf2) found. Download one from: https://musical-artifacts.com/artifacts?formats=sf2",
                    ))
                }
            }
            state::MidiPlayer::Timidity => Command::new("timidity")
                .arg(file_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn(),
        };

        match result {
            Ok(child) => {
                self.player_process = Some(child);
            }
            Err(e) => {
                self.state.error = Some(format!("Failed to play: {}", e));
                self.state.view = MidiView::Error;
                self.state.is_playing = false;
            }
        }
    }

    /// Stop the currently playing MIDI
    fn stop_playing(&mut self) {
        if let Some(mut child) = self.player_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.state.is_playing = false;
    }

    /// Check if a file is a MIDI file
    pub fn is_midi_file(path: &PathBuf) -> bool {
        path.extension()
            .map(|ext| {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                ext_lower == "mid" || ext_lower == "midi"
            })
            .unwrap_or(false)
    }
}

/// Find a soundfont for FluidSynth
fn find_soundfont() -> Option<String> {
    let common_paths = [
        "/opt/homebrew/share/soundfonts/default.sf2",
        "/opt/homebrew/share/fluid-synth/default.sf2",
        "/usr/local/share/soundfonts/default.sf2",
        "/usr/share/soundfonts/default.sf2",
        "/usr/share/sounds/sf2/FluidR3_GM.sf2",
    ];

    for path in common_paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    // Try to find any .sf2 file in common locations
    for dir in [
        "/opt/homebrew/share/soundfonts",
        "/opt/homebrew/share/fluid-synth",
        "/usr/local/share/soundfonts",
        "/usr/share/soundfonts",
        "/usr/share/sounds/sf2",
    ] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry
                    .path()
                    .extension()
                    .map(|e| e == "sf2")
                    .unwrap_or(false)
                {
                    return Some(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }

    None
}

impl Plugin for MidiPlugin {
    fn id(&self) -> &str {
        "midi"
    }

    fn name(&self) -> &str {
        "MIDI Player"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: false,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn init(&mut self, _cwd: &PathBuf) -> Result<(), String> {
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.stop_playing();
        self.initialized = false;
        Ok(())
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        // Available if any MIDI player is installed
        state::MidiPlayer::all().iter().any(|p| p.is_available())
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "MIDI".to_string(),
            key: 'M',
            description: "Play MIDI files".to_string(),
            priority: 35,
        })
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn handle_global_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            MidiView::Menu => match key.code {
                KeyCode::Esc => {
                    self.stop_playing();
                    KeyHandleResult::CloseModal
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.state.select_prev();
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.state.select_next();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char('p') | KeyCode::Char('P') => {
                    if self.state.file_path.is_some() && !self.state.available_players.is_empty() {
                        self.play_midi();
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            MidiView::Playing => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.stop_playing();
                    self.state.view = MidiView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            MidiView::Error => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.state.view = MidiView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_midi_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "MIDI Player".to_string(),
            "".to_string(),
            "Play MIDI files using system MIDI players.".to_string(),
            "".to_string(),
            "Supported players:".to_string(),
            "  FluidSynth - Software synthesizer".to_string(),
            "  TiMidity++ - MIDI to WAV converter".to_string(),
            "".to_string(),
            "To install:".to_string(),
            "  brew install fluid-synth".to_string(),
            "  brew install timidity".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Drop for MidiPlugin {
    fn drop(&mut self) {
        self.stop_playing();
    }
}
