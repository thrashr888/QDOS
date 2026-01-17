#![allow(dead_code)]
#![allow(clippy::if_same_then_else, clippy::needless_borrow)]
#![allow(clippy::ptr_arg)]

//! Q-MIDI Sequencer Plugin
//!
//! A Cadenza/Mario Paint inspired MIDI sequencer with piano roll editor,
//! multi-track support, and real hardware MIDI output.

mod midi_io;
mod modal;
mod playback;
pub mod state;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qdos_plugin_api::prelude::*;
use ratatui::{layout::Rect, Frame};
use state::{FileAction, QMidiState, QMidiView};
use std::any::Any;
use std::path::PathBuf;

/// Q-MIDI Sequencer plugin
pub struct QMidiPlugin {
    pub state: QMidiState,
    playback: playback::PlaybackEngine,
}

impl Default for QMidiPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl QMidiPlugin {
    pub fn new() -> Self {
        let mut state = QMidiState::new();

        // Refresh MIDI devices
        midi_io::refresh_devices(&mut state);

        // Apply saved config
        midi_io::apply_config(&mut state);

        Self {
            state,
            playback: playback::PlaybackEngine::new(),
        }
    }

    /// Open the modal (optionally with a file)
    pub fn open_modal(&mut self, cwd: &PathBuf, file_path: Option<&PathBuf>) {
        // Refresh devices
        midi_io::refresh_devices(&mut self.state);

        // Load file if provided
        if let Some(path) = file_path {
            if Self::is_midi_file(path) {
                if let Err(e) = midi_io::load_midi_file(&mut self.state, path) {
                    self.state.error = Some(e);
                }
            }
        } else {
            // Reset to new file
            self.state = QMidiState::new();
            midi_io::refresh_devices(&mut self.state);
            midi_io::apply_config(&mut self.state);
        }

        // Try to connect to selected output
        if let Some(port) = &self.state.output_port.clone() {
            let _ = self.playback.connect(port);
        }

        // Set working directory context
        let _ = cwd; // Could use for default save location
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

    /// Handle piano roll key events
    fn handle_piano_roll_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match (key.modifiers, key.code) {
            // Playback
            (KeyModifiers::NONE, KeyCode::Char(' ')) => {
                self.state.toggle_play();
                if self.state.playing {
                    self.playback.start();
                } else {
                    self.playback.stop();
                }
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Char('r') | KeyCode::Char('R')) => {
                self.state.toggle_record();
                if self.state.playing {
                    self.playback.start();
                }
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Char('[')) => {
                self.state.set_loop_start();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Char(']')) => {
                self.state.set_loop_end();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Char('l') | KeyCode::Char('L')) => {
                self.state.toggle_loop();
                KeyHandleResult::Handled
            }

            // Navigation
            (KeyModifiers::NONE, KeyCode::Left) => {
                self.state.cursor_left();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Right) => {
                self.state.cursor_right();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Up) => {
                self.state.cursor_up();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                self.state.cursor_down();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                self.state.scroll_octave_up();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                self.state.scroll_octave_down();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Home) => {
                self.state.goto_start();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Char('+') | KeyCode::Char('=')) => {
                self.state.zoom_in();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Char('-')) => {
                self.state.zoom_out();
                KeyHandleResult::Handled
            }

            // Track selection
            (KeyModifiers::NONE, KeyCode::Char('j')) => {
                self.state.next_track();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) => {
                self.state.prev_track();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Char('m') | KeyCode::Char('M')) => {
                self.state.toggle_mute();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Char('s') | KeyCode::Char('S')) => {
                self.state.toggle_solo();
                KeyHandleResult::Handled
            }

            // Note editing
            (KeyModifiers::NONE, KeyCode::Enter) => {
                self.state.insert_note();
                // Preview the note
                if let Some(track) = self.state.current_track() {
                    self.playback
                        .preview_note(track.channel, self.state.cursor_pitch, 100);
                }
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Delete | KeyCode::Backspace) => {
                self.state.select_at_cursor();
                self.state.delete_selected();
                KeyHandleResult::Handled
            }

            // View switching
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.state.view = self.state.view.next();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Char('t') | KeyCode::Char('T')) => {
                self.state.view = QMidiView::TrackList;
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Char('d') | KeyCode::Char('D')) => {
                midi_io::refresh_devices(&mut self.state);
                self.state.view = QMidiView::MidiDevices;
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::F(1)) => {
                self.state.view = QMidiView::Help;
                KeyHandleResult::Handled
            }

            // File operations
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                if self.state.file_path.is_some() {
                    self.save_file();
                } else {
                    self.state.file_action = FileAction::SaveAs;
                    self.state.view = QMidiView::FileMenu;
                }
                KeyHandleResult::Handled
            }
            (KeyModifiers::CONTROL, KeyCode::Char('o')) => {
                self.state.file_action = FileAction::Open;
                self.state.view = QMidiView::FileMenu;
                KeyHandleResult::Handled
            }
            (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
                self.new_file();
                KeyHandleResult::Handled
            }

            // Exit
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.playback.stop();
                if let Err(e) = midi_io::save_config(&self.state) {
                    self.state.error = Some(e);
                }
                KeyHandleResult::CloseModal
            }

            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle drum sequencer key events
    fn handle_drum_sequencer_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match (key.modifiers, key.code) {
            // Playback
            (KeyModifiers::NONE, KeyCode::Char(' ')) => {
                self.state.toggle_play();
                if self.state.playing {
                    self.playback.start();
                } else {
                    self.playback.stop();
                    self.state.drum_playing_step = 0;
                }
                KeyHandleResult::Handled
            }

            // Navigation
            (KeyModifiers::NONE, KeyCode::Left) => {
                self.state.drum_cursor_left();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Right) => {
                self.state.drum_cursor_right();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Up) => {
                self.state.drum_cursor_up();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                self.state.drum_cursor_down();
                KeyHandleResult::Handled
            }

            // Toggle hit
            (KeyModifiers::NONE, KeyCode::Enter) | (KeyModifiers::NONE, KeyCode::Char('x')) => {
                self.state.drum_toggle_hit();
                // Preview the drum sound
                let drum = &state::DRUM_SOUNDS[self.state.drum_cursor_sound];
                self.playback.preview_note(9, drum.note, 100); // Channel 10 (9 in 0-indexed)
                KeyHandleResult::Handled
            }

            // Clear
            (KeyModifiers::NONE, KeyCode::Char('c') | KeyCode::Char('C'))
                if !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.state.drum_clear_row();
                KeyHandleResult::Handled
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.state.drum_clear_pattern();
                KeyHandleResult::Handled
            }

            // Preset patterns
            (KeyModifiers::NONE, KeyCode::Char('1')) => {
                self.state.drum_load_preset(0); // Empty
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Char('2')) => {
                self.state.drum_load_preset(1); // Rock beat
                KeyHandleResult::Handled
            }

            // View switching
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.state.view = self.state.view.next();
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::Char('d') | KeyCode::Char('D')) => {
                midi_io::refresh_devices(&mut self.state);
                self.state.view = QMidiView::MidiDevices;
                KeyHandleResult::Handled
            }
            (KeyModifiers::NONE, KeyCode::F(1)) => {
                self.state.view = QMidiView::Help;
                KeyHandleResult::Handled
            }

            // Exit
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.playback.stop();
                self.state.drum_playing_step = 0;
                if let Err(e) = midi_io::save_config(&self.state) {
                    self.state.error = Some(e);
                }
                KeyHandleResult::CloseModal
            }

            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle event list key events
    fn handle_event_list_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.event_prev();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.event_next();
                KeyHandleResult::Handled
            }
            KeyCode::Delete | KeyCode::Backspace => {
                // Delete selected event
                let idx = self.state.event_selected;
                if let Some(track) = self.state.current_track_mut() {
                    track.remove_note(idx);
                    self.state.modified = true;
                    if self.state.event_selected >= self.state.event_count() {
                        self.state.event_selected = self.state.event_count().saturating_sub(1);
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                self.state.view = QMidiView::PianoRoll;
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.state.view = QMidiView::PianoRoll;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle track list key events
    fn handle_track_list_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.prev_track();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.next_track();
                KeyHandleResult::Handled
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.state.add_track();
                KeyHandleResult::Handled
            }
            KeyCode::Delete | KeyCode::Backspace => {
                self.state.delete_track();
                KeyHandleResult::Handled
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.state.toggle_mute();
                KeyHandleResult::Handled
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.state.toggle_solo();
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                self.state.view = QMidiView::PianoRoll;
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.state.view = QMidiView::PianoRoll;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle MIDI devices key events
    fn handle_devices_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.device_prev();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.device_next();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.state.select_device();
                // Try to connect
                if let Some(port) = &self.state.output_port.clone() {
                    match self.playback.connect(port) {
                        Ok(()) => {
                            self.state.status_message = Some(format!("Connected: {}", port));
                        }
                        Err(e) => {
                            self.state.error = Some(e);
                        }
                    }
                }
                // Save config
                let _ = midi_io::save_config(&self.state);
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                midi_io::refresh_devices(&mut self.state);
                KeyHandleResult::Handled
            }
            KeyCode::Tab | KeyCode::Esc => {
                self.state.view = QMidiView::PianoRoll;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle file menu key events
    fn handle_file_menu_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.file_action_prev();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.file_action_next();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                match self.state.file_action {
                    FileAction::New => {
                        self.new_file();
                        self.state.view = QMidiView::PianoRoll;
                    }
                    FileAction::Save => {
                        self.save_file();
                        self.state.view = QMidiView::PianoRoll;
                    }
                    FileAction::SaveAs | FileAction::Open => {
                        // For now, use hardcoded filename - proper file picker would be nice
                        if self.state.file_action == FileAction::Open {
                            // Look for .mid files in cwd
                            if let Ok(entries) = std::fs::read_dir(cwd) {
                                for entry in entries.flatten() {
                                    let path = entry.path();
                                    if Self::is_midi_file(&path) {
                                        let _ = midi_io::load_midi_file(&mut self.state, &path);
                                        break;
                                    }
                                }
                            }
                        } else {
                            // Save As
                            let path = cwd.join("untitled.mid");
                            if midi_io::save_midi_file(&self.state, &path).is_ok() {
                                self.state.file_path = Some(path);
                                self.state.modified = false;
                            }
                        }
                        self.state.view = QMidiView::PianoRoll;
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.state.view = QMidiView::PianoRoll;
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                // Input for filename
                self.state.file_input.push(c);
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.file_input.pop();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Create a new file
    fn new_file(&mut self) {
        self.state = QMidiState::new();
        midi_io::refresh_devices(&mut self.state);
        midi_io::apply_config(&mut self.state);
    }

    /// Save current file
    fn save_file(&mut self) {
        if let Some(path) = self.state.file_path.clone() {
            match midi_io::save_midi_file(&self.state, &path) {
                Ok(()) => {
                    self.state.modified = false;
                    self.state.status_message = Some("Saved".to_string());
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
        }
    }
}

impl Plugin for QMidiPlugin {
    fn id(&self) -> &str {
        "qmidi"
    }

    fn name(&self) -> &str {
        "Q-MIDI"
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

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Q-MIDI".to_string(),
            key: 'Q',
            description: "MIDI Sequencer".to_string(),
            priority: 30,
        })
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: "qmidi".to_string(),
            name: "Q-MIDI".to_string(),
            description: "MIDI sequencer - Cadenza/Mario Paint inspired".to_string(),
            category: PluginCategory::Tools,
            key: 'M',
        })
    }

    fn handle_global_key(
        &mut self,
        _key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            QMidiView::PianoRoll => self.handle_piano_roll_key(key),
            QMidiView::DrumSequencer => self.handle_drum_sequencer_key(key),
            QMidiView::EventList => self.handle_event_list_key(key),
            QMidiView::TrackList => self.handle_track_list_key(key),
            QMidiView::MidiDevices => self.handle_devices_key(key),
            QMidiView::FileMenu => self.handle_file_menu_key(key, cwd),
            QMidiView::Help => match key.code {
                KeyCode::Esc | KeyCode::F(1) => {
                    self.state.view = QMidiView::PianoRoll;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn tick(&mut self) {
        // Advance playback
        self.playback.tick(&mut self.state);
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_qmidi_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-MIDI Sequencer".to_string(),
            "".to_string(),
            "A Cadenza-inspired MIDI sequencer with:".to_string(),
            "  - Piano roll editor".to_string(),
            "  - Multi-track support".to_string(),
            "  - Real hardware MIDI output".to_string(),
            "".to_string(),
            "Open .mid files or create new sequences.".to_string(),
            "Connect to MIDI devices to trigger".to_string(),
            "external synths and drum machines.".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Drop for QMidiPlugin {
    fn drop(&mut self) {
        self.playback.stop();
    }
}

inventory::submit! { PluginRegistration::new("qmidi", || Box::new(QMidiPlugin::new())) }
