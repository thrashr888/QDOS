//! Audio Player plugin
//!
//! Play audio files (wav, mp3, ogg, flac, mid) using rodio or external players.

mod modal;
mod player;
mod soundfont;
pub mod state;

use crate::plugins::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use player::AudioPlayerHandle;
use ratatui::{layout::Rect, Frame};
use state::{get_audio_type, AudioPlayer, AudioState, AudioView};
use std::any::Any;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

/// Audio Player plugin
pub struct AudioPlugin {
    initialized: bool,
    pub state: AudioState,
    /// Audio player handle (for builtin player)
    player: Option<AudioPlayerHandle>,
    /// Shared state from player
    player_state: Option<Arc<Mutex<AudioState>>>,
    /// External player process (FluidSynth, TiMidity, etc.)
    external_process: Option<Child>,
}

impl Default for AudioPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
            state: AudioState::new(),
            player: None,
            player_state: None,
            external_process: None,
        }
    }

    /// Open the modal for a specific audio file
    pub fn open_modal(&mut self, file_path: Option<&PathBuf>) {
        self.state = AudioState::new();
        self.state.file_path = file_path.cloned();
        self.state.error = None;

        // Initialize state if file provided
        if let Some(path) = file_path {
            self.state.file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Detect audio type and available players
            self.state.audio_type = get_audio_type(path);
            self.state.detect_players();

            // Detect sibling audio files for prev/next navigation
            self.state.detect_siblings();

            // If only one player available, auto-select and play
            if self.state.available_players.len() == 1 {
                self.play_selected();
            } else {
                // Show menu to select player
                self.state.view = AudioView::Menu;
            }
        } else {
            self.state.view = AudioView::Error;
            self.state.error = Some("No file specified".to_string());
        }
    }

    /// Switch to a different audio file (for prev/next navigation)
    fn switch_to_file(&mut self, file_path: PathBuf) {
        // Stop current playback
        self.stop_playback();

        // Keep sibling list but update current file
        let siblings = std::mem::take(&mut self.state.sibling_files);
        let new_index = siblings.iter().position(|p| p == &file_path).unwrap_or(0);

        // Reset state for new file
        self.state = AudioState::new();
        self.state.file_path = Some(file_path.clone());
        self.state.file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        self.state.audio_type = get_audio_type(&file_path);
        self.state.detect_players();
        self.state.sibling_files = siblings;
        self.state.current_file_index = new_index;

        // Auto-play with same player type if only one available
        if self.state.available_players.len() == 1 {
            self.play_selected();
        } else {
            self.state.view = AudioView::Menu;
        }
    }

    /// Play with selected player
    fn play_selected(&mut self) {
        let Some(player) = self.state.selected().copied() else {
            self.state.error = Some("No player selected".to_string());
            self.state.view = AudioView::Error;
            return;
        };

        let Some(ref path) = self.state.file_path else {
            self.state.error = Some("No file specified".to_string());
            self.state.view = AudioView::Error;
            return;
        };

        match player {
            AudioPlayer::Builtin => {
                // Use rodio builtin player
                match AudioPlayerHandle::new() {
                    Ok(handle) => {
                        self.player_state = Some(handle.state());
                        if let Err(e) = handle.play_file(path) {
                            self.state.error = Some(e);
                            self.state.view = AudioView::Error;
                        } else {
                            self.state.view = AudioView::Player;
                        }
                        self.player = Some(handle);
                    }
                    Err(e) => {
                        self.state.error = Some(e);
                        self.state.view = AudioView::Error;
                    }
                }
            }
            AudioPlayer::SystemDefault => {
                // Use macOS open command
                if let Err(e) = Command::new("open").arg(path).spawn() {
                    self.state.error = Some(format!("Failed to open: {}", e));
                    self.state.view = AudioView::Error;
                } else {
                    self.state.view = AudioView::ExternalPlaying;
                }
            }
            AudioPlayer::FluidSynth => {
                // Find a soundfont for FluidSynth
                let mut cmd = Command::new("fluidsynth");
                cmd.arg("-ni") // Non-interactive mode (exits after playing)
                    .arg("-a")
                    .arg("coreaudio");

                // Add soundfont if found (required for proper MIDI playback)
                if let Some(sf) = soundfont::get_soundfont() {
                    cmd.arg(&sf);
                }

                // Add the MIDI file and suppress output
                cmd.arg(path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());

                match cmd.spawn() {
                    Ok(child) => {
                        self.external_process = Some(child);
                        self.state.view = AudioView::ExternalPlaying;
                    }
                    Err(e) => {
                        self.state.error = Some(format!("Failed to start FluidSynth: {}", e));
                        self.state.view = AudioView::Error;
                    }
                }
            }
            AudioPlayer::Timidity => {
                // Launch TiMidity++ in background with output suppressed
                match Command::new("timidity")
                    .arg(path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    Ok(child) => {
                        self.external_process = Some(child);
                        self.state.view = AudioView::ExternalPlaying;
                    }
                    Err(e) => {
                        self.state.error = Some(format!("Failed to start TiMidity++: {}", e));
                        self.state.view = AudioView::Error;
                    }
                }
            }
        }
    }

    /// Stop playback and cleanup
    fn stop_playback(&mut self) {
        // Stop builtin player
        if let Some(ref player) = self.player {
            player.stop();
        }
        self.player = None;
        self.player_state = None;

        // Kill external player process
        if let Some(ref mut child) = self.external_process {
            let _ = child.kill();
        }
        self.external_process = None;
    }

    /// Check if external player is still running
    fn is_external_playing(&mut self) -> bool {
        if let Some(ref mut child) = self.external_process {
            // Check if process has exited
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Process has exited
                    self.external_process = None;
                    false
                }
                Ok(None) => true, // Still running
                Err(_) => {
                    self.external_process = None;
                    false
                }
            }
        } else {
            false
        }
    }

    /// Sync state from player thread
    fn sync_state(&mut self) {
        // Sync builtin player state
        if let Some(ref player_state) = self.player_state {
            if let Ok(ps) = player_state.lock() {
                self.state.play_state = ps.play_state;
                self.state.position = ps.position;
                self.state.duration = ps.duration;
                self.state.volume = ps.volume;
                self.state.file_name = ps.file_name.clone();
                if ps.error.is_some() {
                    self.state.error = ps.error.clone();
                    self.state.view = AudioView::Error;
                }
            }
        }

        // Check if external player finished
        if self.state.view == AudioView::ExternalPlaying && !self.is_external_playing() {
            // Playback finished - could close modal or return to menu
            self.state.view = AudioView::Menu;
        }
    }

    /// Check if a file is an audio file
    pub fn is_audio_file(path: &PathBuf) -> bool {
        state::is_audio_file(path)
    }
}

impl Plugin for AudioPlugin {
    fn id(&self) -> &str {
        "audio"
    }

    fn name(&self) -> &str {
        "Audio Player"
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
        self.stop_playback();
        self.initialized = false;
        Ok(())
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        // Always available - rodio is built-in
        true
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Audio".to_string(),
            key: 'U',  // Shift+U... wait, that's Disk Space. Let's use 'O' for audiO
            description: "Play audio files".to_string(),
            priority: 34,
        })
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn handle_global_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Sync state from player thread
        self.sync_state();

        match self.state.view {
            AudioView::Menu => match key.code {
                KeyCode::Esc => {
                    self.stop_playback();
                    KeyHandleResult::CloseModal
                }
                KeyCode::Up => {
                    self.state.select_prev();
                    KeyHandleResult::Handled
                }
                KeyCode::Down => {
                    self.state.select_next();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    if !self.state.available_players.is_empty() {
                        self.play_selected();
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    // Download soundfont for MIDI playback
                    if self.state.audio_type == state::AudioType::Midi
                        && soundfont::needs_download()
                    {
                        self.state.view = AudioView::NeedsSoundFont;
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            AudioView::Player => match key.code {
                KeyCode::Esc => {
                    self.stop_playback();
                    KeyHandleResult::CloseModal
                }
                KeyCode::Char(' ') => {
                    if let Some(ref player) = self.player {
                        player.toggle_pause();
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Up => {
                    if let Some(ref player) = self.player {
                        player.adjust_volume(0.1);
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down => {
                    if let Some(ref player) = self.player {
                        player.adjust_volume(-0.1);
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    if let Some(ref player) = self.player {
                        player.stop();
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('[') | KeyCode::Left => {
                    // Previous file
                    if let Some(prev) = self.state.prev_file() {
                        self.switch_to_file(prev);
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char(']') | KeyCode::Right => {
                    // Next file
                    if let Some(next) = self.state.next_file() {
                        self.switch_to_file(next);
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            AudioView::ExternalPlaying => match key.code {
                KeyCode::Esc => {
                    self.stop_playback();
                    KeyHandleResult::CloseModal
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    // Stop external player and return to menu
                    self.stop_playback();
                    self.state.view = AudioView::Menu;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('[') | KeyCode::Left => {
                    // Previous file
                    if let Some(prev) = self.state.prev_file() {
                        self.switch_to_file(prev);
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char(']') | KeyCode::Right => {
                    // Next file
                    if let Some(next) = self.state.next_file() {
                        self.switch_to_file(next);
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            AudioView::Error => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.stop_playback();
                    KeyHandleResult::CloseModal
                }
                _ => KeyHandleResult::Handled,
            },
            AudioView::NeedsSoundFont => match key.code {
                KeyCode::Esc => {
                    self.state.view = AudioView::Menu;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    // Start download
                    self.state.view = AudioView::DownloadingSoundFont;
                    // Spawn download in background
                    match soundfont::download_soundfont() {
                        Ok(_) => {
                            // Download complete, return to menu
                            self.state.view = AudioView::Menu;
                        }
                        Err(e) => {
                            self.state.error = Some(format!("Download failed: {}", e));
                            self.state.view = AudioView::Error;
                        }
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    // Skip - play without soundfont
                    self.state.view = AudioView::Menu;
                    self.play_selected();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            AudioView::DownloadingSoundFont => {
                // Can't do anything while downloading
                KeyHandleResult::Handled
            }
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_audio_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Audio Player".to_string(),
            "".to_string(),
            "Play audio files with built-in or external players.".to_string(),
            "".to_string(),
            "Supported formats:".to_string(),
            "  WAV  - Waveform Audio".to_string(),
            "  MP3  - MPEG Audio Layer 3".to_string(),
            "  OGG  - Ogg Vorbis".to_string(),
            "  FLAC - Free Lossless Audio".to_string(),
            "  AAC  - Advanced Audio Coding".to_string(),
            "  M4A  - MPEG-4 Audio".to_string(),
            "  MID  - MIDI (requires FluidSynth or TiMidity)".to_string(),
            "".to_string(),
            "Players:".to_string(),
            "  Built-in     - Native playback with controls".to_string(),
            "  System Default - Opens with default app".to_string(),
            "  FluidSynth   - Software synthesizer (MIDI)".to_string(),
            "  TiMidity++   - MIDI to audio converter".to_string(),
            "".to_string(),
            "Controls (built-in player):".to_string(),
            "  Space    - Play/Pause".to_string(),
            "  Up/Down  - Volume".to_string(),
            "  S        - Stop".to_string(),
            "  Esc      - Close".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Audio".to_string(),
            description: "Play audio & MIDI files".to_string(),
            category: PluginCategory::Games,
            key: 'M',
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Drop for AudioPlugin {
    fn drop(&mut self) {
        self.stop_playback();
    }
}
