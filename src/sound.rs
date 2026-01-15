//! Sound Effects Module
//!
//! Provides programmatic sound generation for system beeps, chimes, and alerts.
//! Uses rodio to generate tones without any embedded audio files.

#![allow(dead_code)] // Many sound types are for future use

use rodio::source::SineWave;
use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Sound effect types
#[derive(Debug, Clone, Copy)]
pub enum SoundType {
    /// Error beep - low tone
    Error,
    /// Success chime - pleasant rising tones
    Success,
    /// Warning alert - attention-getting
    Warning,
    /// Menu click - subtle tick
    Click,
    /// Achievement unlocked - triumphant
    Achievement,
    /// Game over - descending tones
    GameOver,
    /// Level up - ascending tones
    LevelUp,
    /// Notification - gentle ping
    Notify,
    /// Alien contact: Harmonics - melodic greeting in musical scale
    AlienHarmonics,
    /// Alien contact: Geometers - mathematical pattern (Fibonacci intervals)
    AlienGeometers,
    /// Alien contact: Empaths - warm emotional oscillation
    AlienEmpaths,
}

/// Commands sent to the sound thread
#[derive(Debug)]
enum SoundCommand {
    Play(SoundType),
    SetEnabled(bool),
    SetVolume(f32),
    Shutdown,
}

/// Sound effects player handle
/// Manages a background thread for non-blocking sound playback
pub struct SoundEffects {
    command_tx: Sender<SoundCommand>,
    enabled: Arc<Mutex<bool>>,
    volume: Arc<Mutex<f32>>,
}

impl SoundEffects {
    /// Create a new sound effects player
    pub fn new(enabled: bool) -> Self {
        let (command_tx, command_rx) = channel::<SoundCommand>();
        let enabled = Arc::new(Mutex::new(enabled));
        let volume = Arc::new(Mutex::new(0.3)); // Default volume 30%
        let enabled_clone = Arc::clone(&enabled);
        let volume_clone = Arc::clone(&volume);

        // Spawn background thread for sound playback
        thread::spawn(move || {
            // Try to initialize audio output
            let audio = OutputStream::try_default();
            let (_stream, stream_handle) = match audio {
                Ok((stream, handle)) => (stream, handle),
                Err(_) => {
                    // Audio not available - silently ignore commands
                    loop {
                        match command_rx.recv() {
                            Ok(SoundCommand::Shutdown) | Err(_) => break,
                            _ => continue,
                        }
                    }
                    return;
                }
            };

            loop {
                match command_rx.recv() {
                    Ok(SoundCommand::Play(sound_type)) => {
                        let is_enabled = *enabled_clone.lock().unwrap_or_else(|e| e.into_inner());
                        if !is_enabled {
                            continue;
                        }

                        let vol = *volume_clone.lock().unwrap_or_else(|e| e.into_inner());
                        play_sound(&stream_handle, sound_type, vol);
                    }
                    Ok(SoundCommand::SetEnabled(value)) => {
                        if let Ok(mut e) = enabled_clone.lock() {
                            *e = value;
                        }
                    }
                    Ok(SoundCommand::SetVolume(value)) => {
                        if let Ok(mut v) = volume_clone.lock() {
                            *v = value.clamp(0.0, 1.0);
                        }
                    }
                    Ok(SoundCommand::Shutdown) | Err(_) => break,
                }
            }
        });

        Self {
            command_tx,
            enabled,
            volume,
        }
    }

    /// Play a sound effect
    pub fn play(&self, sound_type: SoundType) {
        let _ = self.command_tx.send(SoundCommand::Play(sound_type));
    }

    /// Play error beep
    pub fn error(&self) {
        self.play(SoundType::Error);
    }

    /// Play success chime
    pub fn success(&self) {
        self.play(SoundType::Success);
    }

    /// Play warning alert
    pub fn warning(&self) {
        self.play(SoundType::Warning);
    }

    /// Play menu click
    pub fn click(&self) {
        self.play(SoundType::Click);
    }

    /// Play achievement sound
    pub fn achievement(&self) {
        self.play(SoundType::Achievement);
    }

    /// Play game over sound
    pub fn game_over(&self) {
        self.play(SoundType::GameOver);
    }

    /// Play level up sound
    pub fn level_up(&self) {
        self.play(SoundType::LevelUp);
    }

    /// Play notification ping
    pub fn notify(&self) {
        self.play(SoundType::Notify);
    }

    /// Play alien contact sound - Harmonics (melodic)
    pub fn alien_harmonics(&self) {
        self.play(SoundType::AlienHarmonics);
    }

    /// Play alien contact sound - Geometers (mathematical)
    pub fn alien_geometers(&self) {
        self.play(SoundType::AlienGeometers);
    }

    /// Play alien contact sound - Empaths (emotional)
    pub fn alien_empaths(&self) {
        self.play(SoundType::AlienEmpaths);
    }

    /// Check if sounds are enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Enable or disable sounds
    pub fn set_enabled(&self, enabled: bool) {
        if let Ok(mut e) = self.enabled.lock() {
            *e = enabled;
        }
        let _ = self.command_tx.send(SoundCommand::SetEnabled(enabled));
    }

    /// Get current volume (0.0 - 1.0)
    pub fn volume(&self) -> f32 {
        *self.volume.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Set volume (0.0 - 1.0)
    pub fn set_volume(&self, volume: f32) {
        let vol = volume.clamp(0.0, 1.0);
        if let Ok(mut v) = self.volume.lock() {
            *v = vol;
        }
        let _ = self.command_tx.send(SoundCommand::SetVolume(vol));
    }
}

impl Drop for SoundEffects {
    fn drop(&mut self) {
        let _ = self.command_tx.send(SoundCommand::Shutdown);
    }
}

// SoundEffects is Send + Sync because it only contains Send types
unsafe impl Send for SoundEffects {}
unsafe impl Sync for SoundEffects {}

/// Play a sound effect on the given stream handle
fn play_sound(stream_handle: &OutputStreamHandle, sound_type: SoundType, volume: f32) {
    let sink = match Sink::try_new(stream_handle) {
        Ok(s) => s,
        Err(_) => return,
    };

    sink.set_volume(volume);

    match sound_type {
        SoundType::Error => {
            // Low buzzy tone - 200Hz for 150ms
            let source = SineWave::new(200.0)
                .take_duration(Duration::from_millis(150))
                .amplify(0.8);
            sink.append(source);
        }
        SoundType::Success => {
            // Rising two-tone chime - C5 then E5
            let tone1 = SineWave::new(523.25) // C5
                .take_duration(Duration::from_millis(100))
                .amplify(0.6);
            let tone2 = SineWave::new(659.25) // E5
                .take_duration(Duration::from_millis(150))
                .amplify(0.6);
            sink.append(tone1);
            sink.append(tone2);
        }
        SoundType::Warning => {
            // Two quick beeps - 440Hz
            let beep1 = SineWave::new(440.0)
                .take_duration(Duration::from_millis(80))
                .amplify(0.7);
            let silence = SineWave::new(0.0).take_duration(Duration::from_millis(50));
            let beep2 = SineWave::new(440.0)
                .take_duration(Duration::from_millis(80))
                .amplify(0.7);
            sink.append(beep1);
            sink.append(silence);
            sink.append(beep2);
        }
        SoundType::Click => {
            // Very short tick - 1000Hz for 10ms
            let source = SineWave::new(1000.0)
                .take_duration(Duration::from_millis(10))
                .amplify(0.3);
            sink.append(source);
        }
        SoundType::Achievement => {
            // Triumphant fanfare - ascending arpeggio C-E-G-C
            let notes = [(523.25, 80), (659.25, 80), (783.99, 80), (1046.50, 200)];
            for (freq, ms) in notes {
                let tone = SineWave::new(freq)
                    .take_duration(Duration::from_millis(ms))
                    .amplify(0.5);
                sink.append(tone);
            }
        }
        SoundType::GameOver => {
            // Sad descending tones
            let notes = [(392.0, 150), (349.23, 150), (329.63, 150), (293.66, 300)];
            for (freq, ms) in notes {
                let tone = SineWave::new(freq)
                    .take_duration(Duration::from_millis(ms))
                    .amplify(0.5);
                sink.append(tone);
            }
        }
        SoundType::LevelUp => {
            // Quick ascending tones
            let notes = [(440.0, 60), (554.37, 60), (659.25, 60), (880.0, 120)];
            for (freq, ms) in notes {
                let tone = SineWave::new(freq)
                    .take_duration(Duration::from_millis(ms))
                    .amplify(0.5);
                sink.append(tone);
            }
        }
        SoundType::Notify => {
            // Gentle ping - single tone with decay feel
            let source = SineWave::new(880.0) // A5
                .take_duration(Duration::from_millis(100))
                .amplify(0.4);
            sink.append(source);
        }
        SoundType::AlienHarmonics => {
            // Melodic greeting - ascending major scale (do-re-mi-fa-sol-la-ti-do)
            // The Harmonics communicate through musical tones and resonance
            let notes = [
                (261.63, 80),  // C4 - do
                (293.66, 80),  // D4 - re
                (329.63, 80),  // E4 - mi
                (349.23, 80),  // F4 - fa
                (392.00, 80),  // G4 - sol
                (440.00, 80),  // A4 - la
                (493.88, 80),  // B4 - ti
                (523.25, 150), // C5 - do (held longer)
            ];
            for (freq, ms) in notes {
                let tone = SineWave::new(freq)
                    .take_duration(Duration::from_millis(ms))
                    .amplify(0.4);
                sink.append(tone);
            }
        }
        SoundType::AlienGeometers => {
            // Mathematical pattern - Fibonacci intervals (1, 1, 2, 3, 5, 8...)
            // The Geometers express ideas through mathematical patterns
            let base_freq = 330.0; // E4
            let fib_ratios = [1.0, 1.0, 2.0, 3.0, 5.0, 8.0];
            for ratio in fib_ratios {
                let freq = base_freq * (1.0 + ratio * 0.1); // Subtle interval changes
                let tone = SineWave::new(freq)
                    .take_duration(Duration::from_millis(100))
                    .amplify(0.5);
                sink.append(tone);
                // Brief silence between mathematical "symbols"
                let silence = SineWave::new(0.0).take_duration(Duration::from_millis(30));
                sink.append(silence);
            }
        }
        SoundType::AlienEmpaths => {
            // Warm emotional oscillation - gentle rising and falling
            // The Empaths share emotions directly through wavelengths
            let pattern = [
                (350.0, 120), // Warm low
                (400.0, 100), // Rising...
                (450.0, 80),  // Peak emotion
                (420.0, 90),  // Gentle fall
                (380.0, 100), // Settling
                (400.0, 150), // Rest in peace/contentment
            ];
            for (freq, ms) in pattern {
                let tone = SineWave::new(freq)
                    .take_duration(Duration::from_millis(ms))
                    .amplify(0.35);
                sink.append(tone);
            }
        }
    }

    // Detach so it plays without blocking
    sink.detach();
}

// =============================================================================
// PROCEDURAL CHIPTUNE BACKGROUND MUSIC
// =============================================================================

/// Musical note frequencies (Hz) in the 4th octave
mod notes {
    pub const C4: f32 = 261.63;
    pub const D4: f32 = 293.66;
    pub const E4: f32 = 329.63;
    pub const F4: f32 = 349.23;
    pub const G4: f32 = 392.00;
    pub const A4: f32 = 440.00;
    pub const B4: f32 = 493.88;
    pub const C5: f32 = 523.25;
    pub const D5: f32 = 587.33;
    pub const E5: f32 = 659.25;
    pub const G5: f32 = 783.99;
    pub const REST: f32 = 0.0;
}

/// Chiptune melody patterns
#[derive(Debug, Clone, Copy)]
pub enum ChiptuneMelody {
    /// Upbeat 8-bit game menu theme
    GameMenu,
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
    }
}
