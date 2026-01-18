//! Sound and music types for R-DOS
//!
//! This crate provides procedural chiptune music generation for games and applications.

use rodio::source::SineWave;
use rodio::{OutputStream, Sink, Source};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Musical note frequencies (Hz)
mod notes {
    // Octave 3
    pub const A3: f32 = 220.00;
    pub const B3: f32 = 246.94;
    // Octave 4
    pub const C4: f32 = 261.63;
    pub const D4: f32 = 293.66;
    pub const E4: f32 = 329.63;
    pub const F4: f32 = 349.23;
    pub const G4: f32 = 392.00;
    pub const A4: f32 = 440.00;
    pub const B4: f32 = 493.88;
    // Octave 5
    pub const C5: f32 = 523.25;
    pub const D5: f32 = 587.33;
    pub const E5: f32 = 659.25;
    pub const G5: f32 = 783.99;
    pub const REST: f32 = 0.0;
}

/// Chiptune melody patterns
#[derive(Debug, Clone, Copy)]
pub enum ChiptuneMelody {
    /// Upbeat 8-bit game menu theme (classic)
    GameMenu,
    /// Alternative menu theme - more mellow
    GameMenu2,
    /// Alternative menu theme - energetic
    GameMenu3,
    /// Alternative menu theme - mysterious
    GameMenu4,
    /// Alternative menu theme - triumphant
    GameMenu5,
}

impl ChiptuneMelody {
    /// Get a random menu melody
    pub fn random_menu() -> Self {
        use rand::Rng;
        match rand::thread_rng().gen_range(0..5) {
            0 => ChiptuneMelody::GameMenu,
            1 => ChiptuneMelody::GameMenu2,
            2 => ChiptuneMelody::GameMenu3,
            3 => ChiptuneMelody::GameMenu4,
            _ => ChiptuneMelody::GameMenu5,
        }
    }
}

/// Commands sent to the chiptune thread
enum ChiptuneCommand {
    Play(ChiptuneMelody),
    Stop,
    SetVolume(f32),
    Shutdown,
}

/// Procedural chiptune music player
/// Generates and plays simple retro-style background music
pub struct ChiptuneMusic {
    command_tx: Sender<ChiptuneCommand>,
    playing: Arc<Mutex<bool>>,
    volume: Arc<Mutex<f32>>,
}

impl ChiptuneMusic {
    /// Create a new chiptune music player
    pub fn new() -> Self {
        let (command_tx, command_rx) = channel::<ChiptuneCommand>();
        let playing = Arc::new(Mutex::new(false));
        let volume = Arc::new(Mutex::new(0.15)); // Quieter for background music
        let playing_clone = Arc::clone(&playing);
        let volume_clone = Arc::clone(&volume);

        thread::spawn(move || {
            // Try to initialize audio output
            let audio = OutputStream::try_default();
            let (_stream, stream_handle) = match audio {
                Ok((stream, handle)) => (stream, handle),
                Err(_) => {
                    // Audio not available - silently ignore commands
                    loop {
                        match command_rx.recv() {
                            Ok(ChiptuneCommand::Shutdown) | Err(_) => break,
                            _ => continue,
                        }
                    }
                    return;
                }
            };

            let mut current_sink: Option<Sink> = None;

            loop {
                // Non-blocking check for commands with timeout
                match command_rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(ChiptuneCommand::Play(melody)) => {
                        // Stop any currently playing music
                        if let Some(sink) = current_sink.take() {
                            sink.stop();
                        }

                        *playing_clone.lock().unwrap_or_else(|e| e.into_inner()) = true;
                        let vol = *volume_clone.lock().unwrap_or_else(|e| e.into_inner());

                        // Create new sink and start melody
                        if let Ok(sink) = Sink::try_new(&stream_handle) {
                            sink.set_volume(vol);
                            play_chiptune_melody(&sink, melody, &playing_clone, &volume_clone);
                            current_sink = Some(sink);
                        }
                    }
                    Ok(ChiptuneCommand::Stop) => {
                        *playing_clone.lock().unwrap_or_else(|e| e.into_inner()) = false;
                        if let Some(sink) = current_sink.take() {
                            sink.stop();
                        }
                    }
                    Ok(ChiptuneCommand::SetVolume(vol)) => {
                        *volume_clone.lock().unwrap_or_else(|e| e.into_inner()) =
                            vol.clamp(0.0, 1.0);
                        if let Some(ref sink) = current_sink {
                            sink.set_volume(vol.clamp(0.0, 1.0));
                        }
                    }
                    Ok(ChiptuneCommand::Shutdown)
                    | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Check if we should continue playing
                        let is_playing = *playing_clone.lock().unwrap_or_else(|e| e.into_inner());
                        if is_playing {
                            // Check if the sink is empty and needs refilling
                            if let Some(ref sink) = current_sink {
                                if sink.empty() {
                                    let vol =
                                        *volume_clone.lock().unwrap_or_else(|e| e.into_inner());
                                    sink.set_volume(vol);
                                    // Restart the melody (loop)
                                    play_chiptune_melody(
                                        sink,
                                        ChiptuneMelody::GameMenu,
                                        &playing_clone,
                                        &volume_clone,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        });

        Self {
            command_tx,
            playing,
            volume,
        }
    }

    /// Start playing a melody
    pub fn play(&self, melody: ChiptuneMelody) {
        let _ = self.command_tx.send(ChiptuneCommand::Play(melody));
    }

    /// Stop playing
    pub fn stop(&self) {
        let _ = self.command_tx.send(ChiptuneCommand::Stop);
    }

    /// Check if music is playing
    pub fn is_playing(&self) -> bool {
        *self.playing.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Set volume (0.0 - 1.0)
    pub fn set_volume(&self, volume: f32) {
        let _ = self
            .command_tx
            .send(ChiptuneCommand::SetVolume(volume.clamp(0.0, 1.0)));
    }

    /// Get current volume
    pub fn volume(&self) -> f32 {
        *self.volume.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Default for ChiptuneMusic {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ChiptuneMusic {
    fn drop(&mut self) {
        let _ = self.command_tx.send(ChiptuneCommand::Shutdown);
    }
}

// ChiptuneMusic is Send + Sync because it only contains Send types
unsafe impl Send for ChiptuneMusic {}
unsafe impl Sync for ChiptuneMusic {}

/// Play a chiptune melody on the given sink
fn play_chiptune_melody(
    sink: &Sink,
    melody: ChiptuneMelody,
    playing: &Arc<Mutex<bool>>,
    _volume: &Arc<Mutex<f32>>,
) {
    use notes::*;

    // Note duration in milliseconds
    let note_ms = 120u64;
    let half_note = note_ms / 2;

    match melody {
        ChiptuneMelody::GameMenu => {
            // Upbeat 8-bit game menu theme
            // Pattern: Simple arpeggio-based melody that loops
            let pattern: &[(f32, u64)] = &[
                // Bar 1: C major arpeggio up
                (C4, note_ms),
                (E4, note_ms),
                (G4, note_ms),
                (C5, note_ms),
                // Bar 2: Descend with passing tones
                (B4, half_note),
                (A4, half_note),
                (G4, note_ms),
                (E4, note_ms),
                // Bar 3: F major feel
                (F4, note_ms),
                (A4, note_ms),
                (C5, note_ms),
                (A4, note_ms),
                // Bar 4: Back to G
                (G4, note_ms),
                (B4, note_ms),
                (D5, note_ms),
                (G5, half_note),
                (REST, half_note),
                // Bar 5: Echo pattern
                (E5, half_note),
                (D5, half_note),
                (C5, note_ms),
                (G4, note_ms),
                // Bar 6: Resolution
                (A4, note_ms),
                (G4, note_ms),
                (E4, note_ms),
                (C4, note_ms * 2),
                // Short rest before loop
                (REST, note_ms),
            ];

            for &(freq, duration) in pattern {
                // Check if we should stop
                if !*playing.lock().unwrap_or_else(|e| e.into_inner()) {
                    return;
                }

                if freq > 0.0 {
                    // Square-wave-like sound using harmonics (more 8-bit feel)
                    let fundamental = SineWave::new(freq)
                        .take_duration(Duration::from_millis(duration))
                        .amplify(0.5);
                    // Add slight harmonics for chiptune character
                    let harmonic = SineWave::new(freq * 2.0)
                        .take_duration(Duration::from_millis(duration))
                        .amplify(0.15);

                    sink.append(fundamental);
                    // Mix in the harmonic by appending at same time (rodio mixes)
                    sink.append(harmonic);
                } else {
                    // Rest - append silence
                    let silence = SineWave::new(0.0).take_duration(Duration::from_millis(duration));
                    sink.append(silence);
                }
            }
        }
        ChiptuneMelody::GameMenu2 => {
            // Mellow menu theme - slower, more ambient
            let pattern: &[(f32, u64)] = &[
                (C4, note_ms * 2),
                (E4, note_ms * 2),
                (G4, note_ms * 2),
                (REST, note_ms),
                (A4, note_ms * 2),
                (G4, note_ms * 2),
                (E4, note_ms * 2),
                (REST, note_ms),
                (F4, note_ms * 2),
                (A4, note_ms * 2),
                (G4, note_ms * 2),
                (E4, note_ms * 2),
                (D4, note_ms * 2),
                (C4, note_ms * 2),
                (REST, note_ms * 2),
            ];
            play_pattern(sink, pattern, playing);
        }
        ChiptuneMelody::GameMenu3 => {
            // Energetic menu theme - faster, more rhythmic
            let pattern: &[(f32, u64)] = &[
                (C4, half_note),
                (C4, half_note),
                (G4, half_note),
                (G4, half_note),
                (A4, half_note),
                (A4, half_note),
                (G4, note_ms),
                (REST, half_note),
                (F4, half_note),
                (F4, half_note),
                (E4, half_note),
                (E4, half_note),
                (D4, half_note),
                (D4, half_note),
                (C4, note_ms),
                (REST, half_note),
                (E4, half_note),
                (G4, half_note),
                (C5, half_note),
                (G4, half_note),
                (E4, half_note),
                (C4, note_ms),
                (REST, note_ms),
            ];
            play_pattern(sink, pattern, playing);
        }
        ChiptuneMelody::GameMenu4 => {
            // Mysterious menu theme - minor key
            let pattern: &[(f32, u64)] = &[
                (A3, note_ms),
                (C4, note_ms),
                (E4, note_ms),
                (A4, note_ms * 2),
                (REST, half_note),
                (G4, note_ms),
                (E4, note_ms),
                (C4, note_ms),
                (D4, note_ms * 2),
                (REST, half_note),
                (E4, note_ms),
                (F4, note_ms),
                (E4, note_ms),
                (D4, note_ms),
                (C4, note_ms),
                (B3, note_ms),
                (A3, note_ms * 2),
                (REST, note_ms),
            ];
            play_pattern(sink, pattern, playing);
        }
        ChiptuneMelody::GameMenu5 => {
            // Triumphant menu theme - fanfare style
            let pattern: &[(f32, u64)] = &[
                (C4, note_ms),
                (E4, note_ms),
                (G4, note_ms),
                (C5, note_ms * 2),
                (REST, half_note),
                (B4, half_note),
                (C5, note_ms),
                (G4, note_ms),
                (E4, note_ms),
                (C4, note_ms * 2),
                (REST, half_note),
                (D4, note_ms),
                (E4, note_ms),
                (F4, note_ms),
                (G4, note_ms),
                (A4, note_ms),
                (B4, note_ms),
                (C5, note_ms * 2),
                (G5, note_ms),
                (E5, note_ms),
                (C5, note_ms * 2),
                (REST, note_ms),
            ];
            play_pattern(sink, pattern, playing);
        }
    }
}

/// Helper to play a pattern
fn play_pattern(sink: &Sink, pattern: &[(f32, u64)], playing: &Arc<Mutex<bool>>) {
    for &(freq, duration) in pattern {
        if !*playing.lock().unwrap_or_else(|e| e.into_inner()) {
            return;
        }
        if freq > 0.0 {
            let fundamental = SineWave::new(freq)
                .take_duration(Duration::from_millis(duration))
                .amplify(0.5);
            let harmonic = SineWave::new(freq * 2.0)
                .take_duration(Duration::from_millis(duration))
                .amplify(0.15);
            sink.append(fundamental);
            sink.append(harmonic);
        } else {
            let silence = SineWave::new(0.0).take_duration(Duration::from_millis(duration));
            sink.append(silence);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_melody_random() {
        // Just ensure random_menu doesn't panic
        let _melody = ChiptuneMelody::random_menu();
    }

    #[test]
    fn test_chiptune_music_new() {
        // This may fail on CI without audio, but should not panic
        let music = ChiptuneMusic::new();
        assert!(!music.is_playing());
    }
}
