//! Q-MIDI State Management
//!
//! Core state types for the MIDI sequencer.

use std::path::PathBuf;

/// Current view in Q-MIDI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QMidiView {
    #[default]
    PianoRoll,
    EventList,
    TrackList,
    MidiDevices,
    FileMenu,
    Help,
}

impl QMidiView {
    /// Cycle to next main view
    pub fn next(&self) -> Self {
        match self {
            QMidiView::PianoRoll => QMidiView::EventList,
            QMidiView::EventList => QMidiView::TrackList,
            QMidiView::TrackList => QMidiView::PianoRoll,
            QMidiView::MidiDevices => QMidiView::PianoRoll,
            QMidiView::FileMenu => QMidiView::PianoRoll,
            QMidiView::Help => QMidiView::PianoRoll,
        }
    }
}

/// A single MIDI note
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub start_tick: u32,
    pub duration: u32,
    pub pitch: u8,
    pub velocity: u8,
}

impl Note {
    pub fn new(start_tick: u32, duration: u32, pitch: u8, velocity: u8) -> Self {
        Self {
            start_tick,
            duration,
            pitch,
            velocity,
        }
    }

    /// Get end tick
    pub fn end_tick(&self) -> u32 {
        self.start_tick + self.duration
    }

    /// Get note name (e.g., "C4", "F#5")
    pub fn name(&self) -> String {
        let octave = (self.pitch / 12) as i8 - 1;
        let note_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let note_idx = (self.pitch % 12) as usize;
        format!("{}{}", note_names[note_idx], octave)
    }
}

/// A MIDI track
#[derive(Debug, Clone)]
pub struct Track {
    pub name: String,
    pub channel: u8,
    pub notes: Vec<Note>,
    pub muted: bool,
    pub solo: bool,
    pub volume: u8,
    pub pan: u8,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            name: "Track".to_string(),
            channel: 0,
            notes: Vec::new(),
            muted: false,
            solo: false,
            volume: 100,
            pan: 64, // Center
        }
    }
}

impl Track {
    pub fn new(name: &str, channel: u8) -> Self {
        Self {
            name: name.to_string(),
            channel,
            ..Default::default()
        }
    }

    /// Add a note to the track
    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
        self.notes.sort_by_key(|n| n.start_tick);
    }

    /// Remove a note by index
    pub fn remove_note(&mut self, index: usize) -> Option<Note> {
        if index < self.notes.len() {
            Some(self.notes.remove(index))
        } else {
            None
        }
    }

    /// Get notes in a tick range
    pub fn notes_in_range(&self, start: u32, end: u32) -> Vec<(usize, &Note)> {
        self.notes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.start_tick < end && n.end_tick() > start)
            .collect()
    }

    /// Get notes at a specific pitch
    pub fn notes_at_pitch(&self, pitch: u8) -> Vec<(usize, &Note)> {
        self.notes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.pitch == pitch)
            .collect()
    }
}

/// File menu action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileAction {
    #[default]
    New,
    Open,
    Save,
    SaveAs,
}

impl FileAction {
    pub fn all() -> &'static [FileAction] {
        &[
            FileAction::New,
            FileAction::Open,
            FileAction::Save,
            FileAction::SaveAs,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            FileAction::New => "New",
            FileAction::Open => "Open",
            FileAction::Save => "Save",
            FileAction::SaveAs => "Save As",
        }
    }
}

/// Main Q-MIDI state
#[derive(Debug, Clone)]
pub struct QMidiState {
    pub view: QMidiView,
    pub tracks: Vec<Track>,
    pub current_track: usize,
    pub tempo: u16,
    pub time_signature: (u8, u8),
    pub ppqn: u16,

    // Playback
    pub playing: bool,
    pub recording: bool,
    pub position: u32,
    pub loop_enabled: bool,
    pub loop_start: Option<u32>,
    pub loop_end: Option<u32>,

    // Piano roll view
    pub scroll_x: u32,
    pub scroll_y: u8,
    pub zoom_x: u8,
    pub cursor_tick: u32,
    pub cursor_pitch: u8,
    pub selected_notes: Vec<usize>,

    // Event list view
    pub event_scroll: usize,
    pub event_selected: usize,

    // Track list view
    pub track_scroll: usize,

    // Device selection
    pub device_scroll: usize,
    pub device_selected: usize,

    // File menu
    pub file_action: FileAction,
    pub file_input: String,
    pub file_cursor: usize,

    // File state
    pub file_path: Option<PathBuf>,
    pub modified: bool,

    // MIDI devices
    pub output_port: Option<String>,
    pub available_outputs: Vec<String>,
    pub input_port: Option<String>,
    pub available_inputs: Vec<String>,

    // Error/status
    pub error: Option<String>,
    pub status_message: Option<String>,
}

impl Default for QMidiState {
    fn default() -> Self {
        Self::new()
    }
}

impl QMidiState {
    pub fn new() -> Self {
        let mut state = Self {
            view: QMidiView::PianoRoll,
            tracks: Vec::new(),
            current_track: 0,
            tempo: 120,
            time_signature: (4, 4),
            ppqn: 480,

            playing: false,
            recording: false,
            position: 0,
            loop_enabled: false,
            loop_start: None,
            loop_end: None,

            scroll_x: 0,
            scroll_y: 60, // Middle C area
            zoom_x: 4,
            cursor_tick: 0,
            cursor_pitch: 60, // Middle C
            selected_notes: Vec::new(),

            event_scroll: 0,
            event_selected: 0,

            track_scroll: 0,

            device_scroll: 0,
            device_selected: 0,

            file_action: FileAction::New,
            file_input: String::new(),
            file_cursor: 0,

            file_path: None,
            modified: false,

            output_port: None,
            available_outputs: Vec::new(),
            input_port: None,
            available_inputs: Vec::new(),

            error: None,
            status_message: None,
        };

        // Add default track
        state.tracks.push(Track::new("Track 1", 0));

        state
    }

    /// Get display name for title bar
    pub fn display_name(&self) -> String {
        if let Some(path) = &self.file_path {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".to_string())
        } else {
            "Untitled".to_string()
        }
    }

    /// Get current track
    pub fn current_track(&self) -> Option<&Track> {
        self.tracks.get(self.current_track)
    }

    /// Get current track mutably
    pub fn current_track_mut(&mut self) -> Option<&mut Track> {
        self.tracks.get_mut(self.current_track)
    }

    /// Select next track
    pub fn next_track(&mut self) {
        if !self.tracks.is_empty() {
            self.current_track = (self.current_track + 1) % self.tracks.len();
        }
    }

    /// Select previous track
    pub fn prev_track(&mut self) {
        if !self.tracks.is_empty() {
            self.current_track = self
                .current_track
                .checked_sub(1)
                .unwrap_or(self.tracks.len() - 1);
        }
    }

    /// Add a new track
    pub fn add_track(&mut self) {
        let num = self.tracks.len() + 1;
        let channel = (num.min(16) - 1) as u8;
        self.tracks
            .push(Track::new(&format!("Track {}", num), channel));
        self.modified = true;
    }

    /// Delete current track
    pub fn delete_track(&mut self) {
        if self.tracks.len() > 1 {
            self.tracks.remove(self.current_track);
            if self.current_track >= self.tracks.len() {
                self.current_track = self.tracks.len() - 1;
            }
            self.modified = true;
        }
    }

    /// Toggle mute on current track
    pub fn toggle_mute(&mut self) {
        if let Some(track) = self.current_track_mut() {
            track.muted = !track.muted;
        }
    }

    /// Toggle solo on current track
    pub fn toggle_solo(&mut self) {
        if let Some(track) = self.current_track_mut() {
            track.solo = !track.solo;
        }
    }

    // =========================================================================
    // PIANO ROLL NAVIGATION
    // =========================================================================

    /// Move cursor left (earlier in time)
    pub fn cursor_left(&mut self) {
        let step = self.ticks_per_step();
        self.cursor_tick = self.cursor_tick.saturating_sub(step);
        self.ensure_cursor_visible();
    }

    /// Move cursor right (later in time)
    pub fn cursor_right(&mut self) {
        let step = self.ticks_per_step();
        self.cursor_tick += step;
        self.ensure_cursor_visible();
    }

    /// Move cursor up (higher pitch)
    pub fn cursor_up(&mut self) {
        if self.cursor_pitch < 127 {
            self.cursor_pitch += 1;
            self.ensure_cursor_visible();
        }
    }

    /// Move cursor down (lower pitch)
    pub fn cursor_down(&mut self) {
        if self.cursor_pitch > 0 {
            self.cursor_pitch -= 1;
            self.ensure_cursor_visible();
        }
    }

    /// Get ticks per cursor step based on zoom
    fn ticks_per_step(&self) -> u32 {
        (self.ppqn / 4) as u32 // 16th notes
    }

    /// Ensure cursor is visible in viewport
    fn ensure_cursor_visible(&mut self) {
        // Horizontal scroll
        let visible_ticks = self.visible_ticks();
        if self.cursor_tick < self.scroll_x {
            self.scroll_x = self.cursor_tick;
        } else if self.cursor_tick >= self.scroll_x + visible_ticks {
            self.scroll_x = self.cursor_tick - visible_ticks + self.ticks_per_step();
        }

        // Vertical scroll
        let visible_pitches = 12; // Approximate
        if self.cursor_pitch < self.scroll_y {
            self.scroll_y = self.cursor_pitch;
        } else if self.cursor_pitch >= self.scroll_y + visible_pitches {
            self.scroll_y = self.cursor_pitch - visible_pitches + 1;
        }
    }

    /// Get number of visible ticks based on zoom
    fn visible_ticks(&self) -> u32 {
        // Approximate based on typical width
        (self.ppqn as u32 * 4) / self.zoom_x as u32
    }

    /// Scroll up by octave
    pub fn scroll_octave_up(&mut self) {
        self.scroll_y = self.scroll_y.saturating_add(12).min(127);
        self.cursor_pitch = self.cursor_pitch.saturating_add(12).min(127);
    }

    /// Scroll down by octave
    pub fn scroll_octave_down(&mut self) {
        self.scroll_y = self.scroll_y.saturating_sub(12);
        self.cursor_pitch = self.cursor_pitch.saturating_sub(12);
    }

    /// Go to start
    pub fn goto_start(&mut self) {
        self.cursor_tick = 0;
        self.scroll_x = 0;
    }

    /// Zoom in (more detail)
    pub fn zoom_in(&mut self) {
        if self.zoom_x < 16 {
            self.zoom_x += 1;
        }
    }

    /// Zoom out (less detail)
    pub fn zoom_out(&mut self) {
        if self.zoom_x > 1 {
            self.zoom_x -= 1;
        }
    }

    // =========================================================================
    // NOTE EDITING
    // =========================================================================

    /// Insert a note at cursor position
    pub fn insert_note(&mut self) {
        let note = Note::new(
            self.cursor_tick,
            self.ticks_per_step(),
            self.cursor_pitch,
            100, // Default velocity
        );
        if let Some(track) = self.current_track_mut() {
            track.add_note(note);
            self.modified = true;
        }
    }

    /// Delete selected notes
    pub fn delete_selected(&mut self) {
        if self.selected_notes.is_empty() {
            return;
        }

        // Clone and sort indices before borrowing track mutably
        let mut indices: Vec<usize> = self.selected_notes.clone();
        indices.sort_by(|a, b| b.cmp(a));

        if let Some(track) = self.current_track_mut() {
            for idx in indices {
                track.remove_note(idx);
            }
        }
        self.selected_notes.clear();
        self.modified = true;
    }

    /// Select note at cursor position
    pub fn select_at_cursor(&mut self) {
        if let Some(track) = self.current_track() {
            for (idx, note) in track.notes.iter().enumerate() {
                if note.pitch == self.cursor_pitch
                    && note.start_tick <= self.cursor_tick
                    && note.end_tick() > self.cursor_tick
                {
                    if !self.selected_notes.contains(&idx) {
                        self.selected_notes.push(idx);
                    }
                    return;
                }
            }
        }
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected_notes.clear();
    }

    // =========================================================================
    // PLAYBACK
    // =========================================================================

    /// Toggle play/stop
    pub fn toggle_play(&mut self) {
        self.playing = !self.playing;
        if !self.playing {
            self.recording = false;
        }
    }

    /// Toggle recording
    pub fn toggle_record(&mut self) {
        self.recording = !self.recording;
        if self.recording && !self.playing {
            self.playing = true;
        }
    }

    /// Set loop start at current position
    pub fn set_loop_start(&mut self) {
        self.loop_start = Some(self.cursor_tick);
        if let Some(end) = self.loop_end {
            if end <= self.cursor_tick {
                self.loop_end = None;
            }
        }
    }

    /// Set loop end at current position
    pub fn set_loop_end(&mut self) {
        self.loop_end = Some(self.cursor_tick);
        if let Some(start) = self.loop_start {
            if start >= self.cursor_tick {
                self.loop_start = None;
            }
        }
    }

    /// Toggle loop mode
    pub fn toggle_loop(&mut self) {
        self.loop_enabled = !self.loop_enabled;
    }

    /// Rewind to start
    pub fn rewind(&mut self) {
        self.position = if self.loop_enabled {
            self.loop_start.unwrap_or(0)
        } else {
            0
        };
    }

    // =========================================================================
    // EVENT LIST
    // =========================================================================

    /// Get total events for current track
    pub fn event_count(&self) -> usize {
        self.current_track().map(|t| t.notes.len()).unwrap_or(0)
    }

    /// Select next event
    pub fn event_next(&mut self) {
        let count = self.event_count();
        if count > 0 && self.event_selected < count - 1 {
            self.event_selected += 1;
        }
    }

    /// Select previous event
    pub fn event_prev(&mut self) {
        if self.event_selected > 0 {
            self.event_selected -= 1;
        }
    }

    // =========================================================================
    // DEVICE SELECTION
    // =========================================================================

    /// Select next device
    pub fn device_next(&mut self) {
        if !self.available_outputs.is_empty()
            && self.device_selected < self.available_outputs.len() - 1
        {
            self.device_selected += 1;
        }
    }

    /// Select previous device
    pub fn device_prev(&mut self) {
        if self.device_selected > 0 {
            self.device_selected -= 1;
        }
    }

    /// Confirm device selection
    pub fn select_device(&mut self) {
        if let Some(port) = self.available_outputs.get(self.device_selected) {
            self.output_port = Some(port.clone());
            self.status_message = Some(format!("Selected: {}", port));
        }
    }

    // =========================================================================
    // FILE MENU
    // =========================================================================

    /// Next file action
    pub fn file_action_next(&mut self) {
        let actions = FileAction::all();
        let current = actions
            .iter()
            .position(|a| *a == self.file_action)
            .unwrap_or(0);
        self.file_action = actions[(current + 1) % actions.len()];
    }

    /// Previous file action
    pub fn file_action_prev(&mut self) {
        let actions = FileAction::all();
        let current = actions
            .iter()
            .position(|a| *a == self.file_action)
            .unwrap_or(0);
        self.file_action = actions[(current + actions.len() - 1) % actions.len()];
    }

    // =========================================================================
    // POSITION FORMATTING
    // =========================================================================

    /// Format position as measure:beat:tick
    pub fn format_position(&self, tick: u32) -> String {
        let ticks_per_beat = self.ppqn as u32;
        let beats_per_measure = self.time_signature.0 as u32;
        let ticks_per_measure = ticks_per_beat * beats_per_measure;

        let measure = tick / ticks_per_measure + 1;
        let beat = (tick % ticks_per_measure) / ticks_per_beat + 1;
        let subtick = tick % ticks_per_beat;

        format!("{:03}:{:02}:{:03}", measure, beat, subtick)
    }
}
