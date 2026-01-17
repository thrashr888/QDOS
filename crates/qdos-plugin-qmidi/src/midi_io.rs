//! Q-MIDI File and Hardware I/O
//!
//! MIDI file parsing (midly) and hardware device I/O (midir).

use crate::state::{Note, QMidiState, Track};
use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use std::fs;
use std::path::Path;

/// MIDI output connection wrapper
pub struct MidiConnection {
    #[allow(dead_code)]
    connection: MidiOutputConnection,
}

impl MidiConnection {
    /// Send a MIDI message
    pub fn send(&mut self, message: &[u8]) -> Result<(), String> {
        self.connection
            .send(message)
            .map_err(|e| format!("Failed to send MIDI: {}", e))
    }

    /// Send note on
    pub fn note_on(&mut self, channel: u8, pitch: u8, velocity: u8) -> Result<(), String> {
        self.send(&[0x90 | (channel & 0x0F), pitch & 0x7F, velocity & 0x7F])
    }

    /// Send note off
    pub fn note_off(&mut self, channel: u8, pitch: u8) -> Result<(), String> {
        self.send(&[0x80 | (channel & 0x0F), pitch & 0x7F, 0])
    }

    /// Send all notes off for a channel
    pub fn all_notes_off(&mut self, channel: u8) -> Result<(), String> {
        // CC 123 = All Notes Off
        self.send(&[0xB0 | (channel & 0x0F), 123, 0])
    }

    /// Panic - send all notes off on all channels
    pub fn panic(&mut self) -> Result<(), String> {
        for ch in 0..16 {
            self.all_notes_off(ch)?;
        }
        Ok(())
    }
}

/// Enumerate available MIDI output ports
pub fn enumerate_outputs() -> Vec<String> {
    let Ok(midi_out) = MidiOutput::new("Q-MIDI") else {
        return Vec::new();
    };

    midi_out
        .ports()
        .iter()
        .filter_map(|port| midi_out.port_name(port).ok())
        .collect()
}

/// Enumerate available MIDI input ports
pub fn enumerate_inputs() -> Vec<String> {
    let Ok(midi_in) = midir::MidiInput::new("Q-MIDI") else {
        return Vec::new();
    };

    midi_in
        .ports()
        .iter()
        .filter_map(|port| midi_in.port_name(port).ok())
        .collect()
}

/// Connect to a MIDI output by name
pub fn connect_output(port_name: &str) -> Result<MidiConnection, String> {
    let midi_out =
        MidiOutput::new("Q-MIDI").map_err(|e| format!("Failed to create MIDI output: {}", e))?;

    let ports = midi_out.ports();
    let port: &MidiOutputPort = ports
        .iter()
        .find(|p| {
            midi_out
                .port_name(p)
                .map(|n| n == port_name)
                .unwrap_or(false)
        })
        .ok_or_else(|| format!("MIDI port not found: {}", port_name))?;

    let connection = midi_out
        .connect(port, "Q-MIDI Output")
        .map_err(|e| format!("Failed to connect to MIDI port: {}", e))?;

    Ok(MidiConnection { connection })
}

/// Check if FluidSynth is available for software synthesis
pub fn has_fluidsynth() -> bool {
    std::process::Command::new("fluidsynth")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get helpful message when no MIDI devices found
pub fn no_devices_help() -> Vec<String> {
    let mut help = vec![
        "No MIDI output devices found.".to_string(),
        "".to_string(),
        "To enable MIDI playback:".to_string(),
    ];

    #[cfg(target_os = "macos")]
    {
        help.push("".to_string());
        help.push("macOS Options:".to_string());
        help.push("  1. Enable IAC Driver:".to_string());
        help.push("     - Open Audio MIDI Setup".to_string());
        help.push("     - Window > Show MIDI Studio".to_string());
        help.push("     - Double-click IAC Driver".to_string());
        help.push("     - Check 'Device is online'".to_string());
        help.push("".to_string());
        help.push("  2. Install FluidSynth:".to_string());
        help.push("     brew install fluid-synth".to_string());
    }

    #[cfg(target_os = "linux")]
    {
        help.push("".to_string());
        help.push("Linux Options:".to_string());
        help.push("  1. Install TiMidity++ or FluidSynth".to_string());
        help.push("  2. Start a software synthesizer".to_string());
    }

    if has_fluidsynth() {
        help.push("".to_string());
        help.push("FluidSynth detected! Software synth available.".to_string());
    }

    help
}

/// Refresh device lists in state
pub fn refresh_devices(state: &mut QMidiState) {
    state.available_outputs = enumerate_outputs();
    state.available_inputs = enumerate_inputs();

    // If current output port is no longer available, clear it
    if let Some(port) = &state.output_port {
        if !state.available_outputs.contains(port) {
            state.output_port = None;
        }
    }

    // Auto-select first output if none selected
    if state.output_port.is_none() && !state.available_outputs.is_empty() {
        state.output_port = Some(state.available_outputs[0].clone());
        state.device_selected = 0;
    }
}

// =============================================================================
// MIDI FILE I/O
// =============================================================================

/// Load a MIDI file into state
pub fn load_midi_file(state: &mut QMidiState, path: &Path) -> Result<(), String> {
    let data = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let smf = Smf::parse(&data).map_err(|e| format!("Failed to parse MIDI: {}", e))?;

    // Extract timing info
    match smf.header.timing {
        Timing::Metrical(ppqn) => {
            state.ppqn = ppqn.as_int();
        }
        Timing::Timecode(fps, subframe) => {
            // Convert to approximate PPQN
            state.ppqn = (fps.as_int() as u16) * (subframe as u16);
        }
    }

    // Clear existing tracks
    state.tracks.clear();

    // Parse tracks
    for (track_idx, track) in smf.tracks.iter().enumerate() {
        let mut new_track = Track::new(&format!("Track {}", track_idx + 1), track_idx as u8 % 16);
        let mut current_tick: u32 = 0;
        let mut active_notes: Vec<(u8, u32)> = Vec::new(); // (pitch, start_tick)

        for event in track {
            current_tick += event.delta.as_int();

            match event.kind {
                TrackEventKind::Meta(MetaMessage::TrackName(name)) => {
                    if let Ok(s) = std::str::from_utf8(name) {
                        new_track.name = s.to_string();
                    }
                }
                TrackEventKind::Meta(MetaMessage::Tempo(tempo)) => {
                    // Convert microseconds per beat to BPM
                    let usec = tempo.as_int();
                    if usec > 0 {
                        state.tempo = (60_000_000 / usec) as u16;
                    }
                }
                TrackEventKind::Meta(MetaMessage::TimeSignature(num, denom_power, _, _)) => {
                    state.time_signature = (num, 1 << denom_power);
                }
                TrackEventKind::Midi { channel, message } => {
                    new_track.channel = channel.as_int();

                    match message {
                        MidiMessage::NoteOn { key, vel } => {
                            let pitch = key.as_int();
                            let velocity = vel.as_int();

                            if velocity > 0 {
                                // Note on - store start time
                                active_notes.push((pitch, current_tick));
                            } else {
                                // Note on with velocity 0 = note off
                                if let Some(pos) =
                                    active_notes.iter().position(|(p, _)| *p == pitch)
                                {
                                    let (_, start) = active_notes.remove(pos);
                                    let duration = current_tick.saturating_sub(start);
                                    new_track.add_note(Note::new(start, duration, pitch, 100));
                                }
                            }
                        }
                        MidiMessage::NoteOff { key, .. } => {
                            let pitch = key.as_int();
                            if let Some(pos) = active_notes.iter().position(|(p, _)| *p == pitch) {
                                let (_, start) = active_notes.remove(pos);
                                let duration = current_tick.saturating_sub(start);
                                new_track.add_note(Note::new(start, duration, pitch, 100));
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Only add tracks with notes
        if !new_track.notes.is_empty() || track_idx == 0 {
            state.tracks.push(new_track);
        }
    }

    // Ensure at least one track
    if state.tracks.is_empty() {
        state.tracks.push(Track::new("Track 1", 0));
    }

    state.file_path = Some(path.to_path_buf());
    state.modified = false;
    state.current_track = 0;
    state.position = 0;
    state.cursor_tick = 0;

    Ok(())
}

/// Save state to a MIDI file
pub fn save_midi_file(state: &QMidiState, path: &Path) -> Result<(), String> {
    let mut tracks: Vec<Vec<TrackEvent<'static>>> = Vec::new();

    // First track: tempo and time signature
    let mut meta_track: Vec<TrackEvent<'static>> = Vec::new();

    // Tempo
    let usec_per_beat = 60_000_000 / state.tempo as u32;
    meta_track.push(TrackEvent {
        delta: 0.into(),
        kind: TrackEventKind::Meta(MetaMessage::Tempo(usec_per_beat.into())),
    });

    // Time signature
    let denom_power = match state.time_signature.1 {
        1 => 0,
        2 => 1,
        4 => 2,
        8 => 3,
        16 => 4,
        _ => 2,
    };
    meta_track.push(TrackEvent {
        delta: 0.into(),
        kind: TrackEventKind::Meta(MetaMessage::TimeSignature(
            state.time_signature.0,
            denom_power,
            24, // MIDI clocks per metronome tick
            8,  // 32nd notes per MIDI quarter note
        )),
    });

    // End of track
    meta_track.push(TrackEvent {
        delta: 0.into(),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });

    tracks.push(meta_track);

    // Note tracks
    for track in &state.tracks {
        let mut midi_track: Vec<TrackEvent<'static>> = Vec::new();
        let mut events: Vec<(u32, bool, u8, u8)> = Vec::new(); // (tick, is_on, pitch, velocity)

        // Convert notes to events
        for note in &track.notes {
            events.push((note.start_tick, true, note.pitch, note.velocity));
            events.push((note.end_tick(), false, note.pitch, 0));
        }

        // Sort by tick
        events.sort_by_key(|(tick, is_on, _, _)| (*tick, !*is_on)); // Note offs before note ons at same tick

        // Convert to delta times
        let mut last_tick = 0u32;
        for (tick, is_on, pitch, velocity) in events {
            let delta = tick.saturating_sub(last_tick);
            last_tick = tick;

            let message = if is_on {
                MidiMessage::NoteOn {
                    key: pitch.into(),
                    vel: velocity.into(),
                }
            } else {
                MidiMessage::NoteOff {
                    key: pitch.into(),
                    vel: 0.into(),
                }
            };

            midi_track.push(TrackEvent {
                delta: delta.into(),
                kind: TrackEventKind::Midi {
                    channel: track.channel.into(),
                    message,
                },
            });
        }

        // End of track
        midi_track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        tracks.push(midi_track);
    }

    let smf = Smf {
        header: Header {
            format: Format::Parallel,
            timing: Timing::Metrical(state.ppqn.into()),
        },
        tracks,
    };

    smf.save(path)
        .map_err(|e| format!("Failed to save MIDI: {}", e))
}

// =============================================================================
// CONFIG PERSISTENCE
// =============================================================================

use std::io::Write;

/// Config file structure for Q-MIDI settings
#[derive(Debug, Default)]
pub struct QMidiConfig {
    pub last_output_port: Option<String>,
    pub last_input_port: Option<String>,
    pub default_tempo: u16,
    pub default_ppqn: u16,
}

impl QMidiConfig {
    /// Load config from file
    pub fn load() -> Self {
        let config_path = config_path();
        if !config_path.exists() {
            return Self::default();
        }

        let Ok(content) = fs::read_to_string(&config_path) else {
            return Self::default();
        };

        let mut config = Self::default();

        for line in content.lines() {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }
            let key = parts[0].trim();
            let value = parts[1].trim();

            match key {
                "output_port" => config.last_output_port = Some(value.to_string()),
                "input_port" => config.last_input_port = Some(value.to_string()),
                "tempo" => config.default_tempo = value.parse().unwrap_or(120),
                "ppqn" => config.default_ppqn = value.parse().unwrap_or(480),
                _ => {}
            }
        }

        config
    }

    /// Save config to file
    pub fn save(&self) -> Result<(), String> {
        let config_path = config_path();

        // Ensure directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }

        let mut file = fs::File::create(&config_path)
            .map_err(|e| format!("Failed to create config: {}", e))?;

        if let Some(port) = &self.last_output_port {
            writeln!(file, "output_port={}", port)
                .map_err(|e| format!("Failed to write config: {}", e))?;
        }
        if let Some(port) = &self.last_input_port {
            writeln!(file, "input_port={}", port)
                .map_err(|e| format!("Failed to write config: {}", e))?;
        }
        if self.default_tempo > 0 {
            writeln!(file, "tempo={}", self.default_tempo)
                .map_err(|e| format!("Failed to write config: {}", e))?;
        }
        if self.default_ppqn > 0 {
            writeln!(file, "ppqn={}", self.default_ppqn)
                .map_err(|e| format!("Failed to write config: {}", e))?;
        }

        Ok(())
    }
}

/// Get config file path
fn config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("rdos")
        .join("qmidi.conf")
}

/// Apply config to state
pub fn apply_config(state: &mut QMidiState) {
    let config = QMidiConfig::load();

    if let Some(port) = &config.last_output_port {
        if state.available_outputs.contains(port) {
            state.output_port = Some(port.clone());
            state.device_selected = state
                .available_outputs
                .iter()
                .position(|p| p == port)
                .unwrap_or(0);
        }
    }

    if config.default_tempo > 0 {
        state.tempo = config.default_tempo;
    }
    if config.default_ppqn > 0 {
        state.ppqn = config.default_ppqn;
    }
}

/// Save current settings to config
pub fn save_config(state: &QMidiState) -> Result<(), String> {
    let config = QMidiConfig {
        last_output_port: state.output_port.clone(),
        last_input_port: state.input_port.clone(),
        default_tempo: state.tempo,
        default_ppqn: state.ppqn,
    };
    config.save()
}
