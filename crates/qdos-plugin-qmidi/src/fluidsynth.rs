//! FluidSynth software synthesizer integration
//!
//! Provides software MIDI playback using FluidSynth when no hardware MIDI is available.

use crate::soundfont;
use crate::state::QMidiState;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// Check if FluidSynth is available on the system
pub fn is_available() -> bool {
    Command::new("fluidsynth")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// FluidSynth playback handle
pub struct FluidSynthPlayer {
    /// Running FluidSynth process
    process: Option<Child>,
    /// Path to soundfont being used
    soundfont_path: Option<PathBuf>,
    /// Temp MIDI file path
    temp_midi_path: Option<PathBuf>,
}

impl Default for FluidSynthPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl FluidSynthPlayer {
    pub fn new() -> Self {
        Self {
            process: None,
            soundfont_path: soundfont::find_soundfont(),
            temp_midi_path: None,
        }
    }

    /// Check if FluidSynth and soundfont are available
    pub fn is_ready(&self) -> bool {
        is_available() && self.soundfont_path.is_some()
    }

    /// Get the soundfont path
    pub fn soundfont(&self) -> Option<&PathBuf> {
        self.soundfont_path.as_ref()
    }

    /// Set custom soundfont path
    pub fn set_soundfont(&mut self, path: PathBuf) {
        if path.exists() {
            self.soundfont_path = Some(path);
        }
    }

    /// Refresh soundfont detection
    pub fn refresh_soundfont(&mut self) {
        self.soundfont_path = soundfont::find_soundfont();
    }

    /// Check if currently playing
    pub fn is_playing(&mut self) -> bool {
        if let Some(ref mut process) = self.process {
            // Try to check if process is still running
            match process.try_wait() {
                Ok(Some(_)) => {
                    self.process = None; // Process has exited, clean up
                    false
                }
                Ok(None) => true, // Still running
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Play a MIDI file using FluidSynth
    pub fn play_file(&mut self, midi_path: &PathBuf) -> Result<(), String> {
        self.stop();

        let sf_path = self
            .soundfont_path
            .as_ref()
            .ok_or("No soundfont available")?;

        // Start FluidSynth in non-interactive mode
        let child = Command::new("fluidsynth")
            .arg("-a")
            .arg("coreaudio") // macOS audio driver (use "pulseaudio" or "alsa" on Linux)
            .arg("-n") // No shell
            .arg("-i") // No interactive mode
            .arg("-q") // Quiet
            .arg(sf_path)
            .arg(midi_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start FluidSynth: {}", e))?;

        self.process = Some(child);
        Ok(())
    }

    /// Play from Q-MIDI state by exporting to temp MIDI file
    pub fn play_state(&mut self, state: &QMidiState) -> Result<(), String> {
        self.stop();

        // Export state to temp MIDI file
        let temp_path = self.export_to_temp_midi(state)?;
        self.temp_midi_path = Some(temp_path.clone());

        self.play_file(&temp_path)
    }

    /// Stop playback
    pub fn stop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }

        // Clean up temp file
        if let Some(path) = self.temp_midi_path.take() {
            let _ = std::fs::remove_file(path);
        }
    }

    /// Export Q-MIDI state to a temporary MIDI file
    fn export_to_temp_midi(&self, state: &QMidiState) -> Result<PathBuf, String> {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("qmidi_temp_{}.mid", std::process::id()));

        let midi_data = self.create_midi_data(state)?;

        let mut file = std::fs::File::create(&temp_path)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

        file.write_all(&midi_data)
            .map_err(|e| format!("Failed to write MIDI data: {}", e))?;

        Ok(temp_path)
    }

    /// Create MIDI file data from Q-MIDI state
    fn create_midi_data(&self, state: &QMidiState) -> Result<Vec<u8>, String> {
        let mut data = Vec::new();

        // MIDI header: "MThd" + length (6) + format (1) + num tracks + ppqn
        data.extend_from_slice(b"MThd");
        data.extend_from_slice(&6u32.to_be_bytes()); // Header length
        data.extend_from_slice(&1u16.to_be_bytes()); // Format 1 (multi-track)
        data.extend_from_slice(&(state.tracks.len() as u16 + 1).to_be_bytes()); // +1 for tempo track
        data.extend_from_slice(&state.ppqn.to_be_bytes());

        // Tempo track
        let tempo_track = self.create_tempo_track(state);
        data.extend_from_slice(&tempo_track);

        // Note tracks
        for track in &state.tracks {
            if track.muted {
                continue;
            }

            let track_data = self.create_note_track(track, state);
            data.extend_from_slice(&track_data);
        }

        Ok(data)
    }

    /// Create tempo track
    fn create_tempo_track(&self, state: &QMidiState) -> Vec<u8> {
        let mut track = Vec::new();

        // Track header
        track.extend_from_slice(b"MTrk");

        let mut events = Vec::new();

        // Set tempo (microseconds per beat)
        let usec_per_beat = 60_000_000u32 / state.tempo as u32;
        events.push(0x00); // Delta time
        events.push(0xFF); // Meta event
        events.push(0x51); // Tempo
        events.push(0x03); // Length
        events.push((usec_per_beat >> 16) as u8);
        events.push((usec_per_beat >> 8) as u8);
        events.push(usec_per_beat as u8);

        // Time signature
        events.push(0x00); // Delta time
        events.push(0xFF); // Meta event
        events.push(0x58); // Time signature
        events.push(0x04); // Length
        events.push(state.time_signature.0); // Numerator
        events.push(state.time_signature.1.trailing_zeros() as u8); // Denominator as power of 2
        events.push(24); // Clocks per click
        events.push(8); // 32nds per quarter

        // End of track
        events.push(0x00);
        events.push(0xFF);
        events.push(0x2F);
        events.push(0x00);

        // Track length
        track.extend_from_slice(&(events.len() as u32).to_be_bytes());
        track.extend_from_slice(&events);

        track
    }

    /// Create a note track
    fn create_note_track(&self, track: &crate::state::Track, _state: &QMidiState) -> Vec<u8> {
        let mut data = Vec::new();

        // Track header
        data.extend_from_slice(b"MTrk");

        let mut events = Vec::new();
        let channel = track.channel & 0x0F;

        // Collect all note events and sort by time
        let mut note_events: Vec<(u32, bool, u8, u8)> = Vec::new(); // (tick, is_on, pitch, velocity)

        for note in &track.notes {
            note_events.push((note.start_tick, true, note.pitch, note.velocity));
            note_events.push((note.end_tick(), false, note.pitch, 0));
        }

        note_events.sort_by_key(|(tick, is_on, _, _)| (*tick, !*is_on)); // Note-offs before note-ons at same tick

        // Convert to MIDI events with delta times
        let mut last_tick = 0u32;

        for (tick, is_on, pitch, velocity) in note_events {
            let delta = tick.saturating_sub(last_tick);
            last_tick = tick;

            // Write variable-length delta time
            Self::write_var_len(&mut events, delta);

            if is_on {
                events.push(0x90 | channel); // Note on
                events.push(pitch);
                events.push(velocity);
            } else {
                events.push(0x80 | channel); // Note off
                events.push(pitch);
                events.push(0);
            }
        }

        // End of track
        events.push(0x00);
        events.push(0xFF);
        events.push(0x2F);
        events.push(0x00);

        // Track length
        data.extend_from_slice(&(events.len() as u32).to_be_bytes());
        data.extend_from_slice(&events);

        data
    }

    /// Write variable-length quantity
    fn write_var_len(buffer: &mut Vec<u8>, mut value: u32) {
        if value == 0 {
            buffer.push(0);
            return;
        }

        let mut bytes = Vec::new();
        while value > 0 {
            bytes.push((value & 0x7F) as u8);
            value >>= 7;
        }

        bytes.reverse();
        for (i, byte) in bytes.iter().enumerate() {
            if i < bytes.len() - 1 {
                buffer.push(byte | 0x80); // Continuation bit
            } else {
                buffer.push(*byte);
            }
        }
    }

    /// Play a single note preview (uses a quick MIDI file)
    pub fn preview_note(&mut self, channel: u8, pitch: u8, velocity: u8) -> Result<(), String> {
        // For note preview, we create a minimal MIDI file with one note
        // This is a simple approach; a more sophisticated version would use FluidSynth's shell mode

        let sf_path = self
            .soundfont_path
            .as_ref()
            .ok_or("No soundfont available")?;

        // Create a minimal MIDI with one short note
        let mut data = Vec::new();

        // Header
        data.extend_from_slice(b"MThd");
        data.extend_from_slice(&6u32.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes()); // Format 0
        data.extend_from_slice(&1u16.to_be_bytes()); // 1 track
        data.extend_from_slice(&480u16.to_be_bytes()); // PPQN

        // Track
        data.extend_from_slice(b"MTrk");
        let mut track = Vec::new();

        // Tempo (120 BPM = 500000 usec/beat)
        track.extend_from_slice(&[0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20]);

        // Note on
        track.push(0x00);
        track.push(0x90 | (channel & 0x0F));
        track.push(pitch);
        track.push(velocity);

        // Note off after 1/4 second (240 ticks at 480 PPQN, 120 BPM)
        track.push(0x81); // 240 in variable length (0x81 0x70)
        track.push(0x70);
        track.push(0x80 | (channel & 0x0F));
        track.push(pitch);
        track.push(0x00);

        // End of track
        track.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);

        data.extend_from_slice(&(track.len() as u32).to_be_bytes());
        data.extend_from_slice(&track);

        // Write to temp file
        let temp_path =
            std::env::temp_dir().join(format!("qmidi_preview_{}.mid", std::process::id()));
        std::fs::write(&temp_path, &data)
            .map_err(|e| format!("Failed to write preview file: {}", e))?;

        // Play (don't save process handle - let it complete on its own)
        let _ = Command::new("fluidsynth")
            .arg("-a")
            .arg("coreaudio")
            .arg("-n")
            .arg("-i")
            .arg("-q")
            .arg(sf_path)
            .arg(&temp_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        // Schedule cleanup (crude but works)
        let temp_path_clone = temp_path.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = std::fs::remove_file(temp_path_clone);
        });

        Ok(())
    }
}

impl Drop for FluidSynthPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fluidsynth_availability() {
        // Just check the function runs - may or may not be available
        let _ = is_available();
    }

    #[test]
    fn test_player_creation() {
        let mut player = FluidSynthPlayer::new();
        assert!(!player.is_playing());
    }

    #[test]
    fn test_var_len_encoding() {
        let mut buf = Vec::new();
        FluidSynthPlayer::write_var_len(&mut buf, 0);
        assert_eq!(buf, vec![0x00]);

        buf.clear();
        FluidSynthPlayer::write_var_len(&mut buf, 127);
        assert_eq!(buf, vec![0x7F]);

        buf.clear();
        FluidSynthPlayer::write_var_len(&mut buf, 128);
        assert_eq!(buf, vec![0x81, 0x00]);

        buf.clear();
        FluidSynthPlayer::write_var_len(&mut buf, 480);
        assert_eq!(buf, vec![0x83, 0x60]);
    }
}
