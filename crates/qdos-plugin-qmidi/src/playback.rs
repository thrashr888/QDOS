//! Q-MIDI Playback Engine
//!
//! Tick-based sequencer with MIDI output.

use crate::midi_io::MidiConnection;
use crate::state::QMidiState;
use std::time::{Duration, Instant};

/// Playback engine for Q-MIDI
pub struct PlaybackEngine {
    /// MIDI output connection
    connection: Option<MidiConnection>,
    /// Last tick time
    last_tick_time: Option<Instant>,
    /// Accumulated time for sub-tick precision
    accumulated_time: Duration,
    /// Active notes (channel, pitch) for note-off tracking
    active_notes: Vec<(u8, u8)>,
}

impl Default for PlaybackEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybackEngine {
    pub fn new() -> Self {
        Self {
            connection: None,
            last_tick_time: None,
            accumulated_time: Duration::ZERO,
            active_notes: Vec::new(),
        }
    }

    /// Connect to a MIDI output
    pub fn connect(&mut self, port_name: &str) -> Result<(), String> {
        self.disconnect();
        self.connection = Some(crate::midi_io::connect_output(port_name)?);
        Ok(())
    }

    /// Disconnect from MIDI output
    pub fn disconnect(&mut self) {
        if let Some(conn) = &mut self.connection {
            let _ = conn.panic();
        }
        self.connection = None;
        self.active_notes.clear();
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    /// Start playback
    pub fn start(&mut self) {
        self.last_tick_time = Some(Instant::now());
        self.accumulated_time = Duration::ZERO;
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.last_tick_time = None;
        self.all_notes_off();
    }

    /// Send all notes off
    pub fn all_notes_off(&mut self) {
        if let Some(conn) = &mut self.connection {
            for (channel, pitch) in self.active_notes.drain(..) {
                let _ = conn.note_off(channel, pitch);
            }
        }
    }

    /// Tick the playback engine - call this from the main loop
    /// Returns the new position if it changed
    pub fn tick(&mut self, state: &mut QMidiState) -> bool {
        if !state.playing {
            return false;
        }

        let Some(conn) = &mut self.connection else {
            return false;
        };

        let Some(last_time) = self.last_tick_time else {
            self.last_tick_time = Some(Instant::now());
            return false;
        };

        let now = Instant::now();
        let elapsed = now.duration_since(last_time);
        self.last_tick_time = Some(now);

        // Calculate ticks from elapsed time
        let usec_per_beat = 60_000_000u64 / state.tempo as u64;
        let usec_per_tick = usec_per_beat / state.ppqn as u64;
        let elapsed_usec = elapsed.as_micros() as u64 + self.accumulated_time.as_micros() as u64;
        let ticks_to_advance = elapsed_usec / usec_per_tick;
        self.accumulated_time = Duration::from_micros(elapsed_usec % usec_per_tick);

        if ticks_to_advance == 0 {
            return false;
        }

        let old_position = state.position;
        let new_position = state.position + ticks_to_advance as u32;

        // Check for notes to play in this time range
        for track in &state.tracks {
            if track.muted {
                continue;
            }

            // Check if any tracks have solo
            let any_solo = state.tracks.iter().any(|t| t.solo);
            if any_solo && !track.solo {
                continue;
            }

            for note in &track.notes {
                // Note on
                if note.start_tick >= old_position && note.start_tick < new_position {
                    let _ = conn.note_on(track.channel, note.pitch, note.velocity);
                    self.active_notes.push((track.channel, note.pitch));
                }

                // Note off
                let end = note.end_tick();
                if end >= old_position && end < new_position {
                    let _ = conn.note_off(track.channel, note.pitch);
                    self.active_notes
                        .retain(|(ch, p)| !(*ch == track.channel && *p == note.pitch));
                }
            }
        }

        // Update position
        state.position = new_position;

        // Handle looping
        if state.loop_enabled {
            if let (Some(start), Some(end)) = (state.loop_start, state.loop_end) {
                if state.position >= end {
                    self.all_notes_off();
                    state.position = start;
                }
            }
        }

        true
    }

    /// Play a single note immediately (for preview/audition)
    pub fn preview_note(&mut self, channel: u8, pitch: u8, velocity: u8) {
        if let Some(conn) = &mut self.connection {
            let _ = conn.note_on(channel, pitch, velocity);
            self.active_notes.push((channel, pitch));
        }
    }

    /// Stop a preview note
    pub fn stop_preview(&mut self, channel: u8, pitch: u8) {
        if let Some(conn) = &mut self.connection {
            let _ = conn.note_off(channel, pitch);
            self.active_notes
                .retain(|(ch, p)| !(*ch == channel && *p == pitch));
        }
    }
}

impl Drop for PlaybackEngine {
    fn drop(&mut self) {
        self.stop();
        self.disconnect();
    }
}
