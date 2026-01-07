//! Media playback module for the Viewer plugin
//!
//! Provides audio playback using rodio in a background thread.

use rodio::{Decoder, OutputStream, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

/// Media file type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    /// Audio files (wav, mp3, ogg, flac)
    Audio,
    /// MIDI files
    Midi,
    /// Video files
    Video,
}

impl MediaType {
    /// Detect media type from file extension
    pub fn from_extension(ext: &str) -> Option<MediaType> {
        match ext.to_lowercase().as_str() {
            // Audio formats supported by rodio
            "wav" | "wave" => Some(MediaType::Audio),
            "mp3" => Some(MediaType::Audio),
            "ogg" | "oga" => Some(MediaType::Audio),
            "flac" => Some(MediaType::Audio),
            "aac" | "m4a" => Some(MediaType::Audio),
            // MIDI
            "mid" | "midi" => Some(MediaType::Midi),
            // Video
            "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg" => {
                Some(MediaType::Video)
            }
            _ => None,
        }
    }

    /// Check if a file path is a media file
    pub fn from_path(path: &PathBuf) -> Option<MediaType> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }
}

/// Playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

/// Media player state (shared across threads)
#[derive(Debug, Default)]
pub struct MediaState {
    /// Current playback state
    pub play_state: PlayState,
    /// Current position in seconds
    pub position: f32,
    /// Total duration in seconds
    pub duration: f32,
    /// Volume (0.0 to 1.0)
    pub volume: f32,
    /// Error message if any
    pub error: Option<String>,
    /// Media type
    pub media_type: Option<MediaType>,
    /// File name being played
    pub file_name: String,
}

impl MediaState {
    pub fn new() -> Self {
        Self {
            volume: 0.8,
            ..Default::default()
        }
    }

    /// Format position as MM:SS
    pub fn format_position(&self) -> String {
        let mins = (self.position as u32) / 60;
        let secs = (self.position as u32) % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    /// Format duration as MM:SS
    pub fn format_duration(&self) -> String {
        let mins = (self.duration as u32) / 60;
        let secs = (self.duration as u32) % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    /// Get progress as 0.0 to 1.0
    pub fn progress(&self) -> f32 {
        if self.duration > 0.0 {
            (self.position / self.duration).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// Commands that can be sent to the audio thread
#[derive(Debug)]
pub enum AudioCommand {
    Play(PathBuf),
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    Shutdown,
}

/// Audio player handle (Send + Sync safe)
/// This just holds the command sender and shared state
pub struct AudioPlayerHandle {
    /// Command sender to the audio thread
    command_tx: Sender<AudioCommand>,
    /// Shared state for UI updates
    state: Arc<Mutex<MediaState>>,
}

impl AudioPlayerHandle {
    /// Create a new audio player with a background thread
    pub fn new() -> Result<Self, String> {
        let (command_tx, command_rx) = channel::<AudioCommand>();
        let state = Arc::new(Mutex::new(MediaState::new()));
        let state_clone = Arc::clone(&state);

        // Spawn the audio thread
        thread::spawn(move || {
            // Initialize audio output on this thread
            let (_stream, stream_handle) = match OutputStream::try_default() {
                Ok(s) => s,
                Err(e) => {
                    if let Ok(mut s) = state_clone.lock() {
                        s.error = Some(format!("Failed to open audio: {}", e));
                    }
                    return;
                }
            };

            let mut sink: Option<Sink> = None;

            loop {
                match command_rx.recv() {
                    Ok(AudioCommand::Play(path)) => {
                        // Stop any existing playback
                        if let Some(s) = sink.take() {
                            s.stop();
                        }

                        // Open and decode the file
                        let file = match File::open(&path) {
                            Ok(f) => f,
                            Err(e) => {
                                if let Ok(mut s) = state_clone.lock() {
                                    s.error = Some(format!("Failed to open file: {}", e));
                                    s.play_state = PlayState::Stopped;
                                }
                                continue;
                            }
                        };

                        let source = match Decoder::new(BufReader::new(file)) {
                            Ok(s) => s,
                            Err(e) => {
                                if let Ok(mut s) = state_clone.lock() {
                                    s.error = Some(format!("Failed to decode audio: {}", e));
                                    s.play_state = PlayState::Stopped;
                                }
                                continue;
                            }
                        };

                        let duration = source
                            .total_duration()
                            .map(|d| d.as_secs_f32())
                            .unwrap_or(0.0);

                        // Create new sink and start playback
                        let new_sink = match Sink::try_new(&stream_handle) {
                            Ok(s) => s,
                            Err(e) => {
                                if let Ok(mut s) = state_clone.lock() {
                                    s.error = Some(format!("Failed to create audio sink: {}", e));
                                    s.play_state = PlayState::Stopped;
                                }
                                continue;
                            }
                        };

                        // Get volume from state
                        let volume = state_clone.lock().map(|s| s.volume).unwrap_or(0.8);
                        new_sink.set_volume(volume);
                        new_sink.append(source);

                        // Update state
                        if let Ok(mut s) = state_clone.lock() {
                            s.play_state = PlayState::Playing;
                            s.position = 0.0;
                            s.duration = duration;
                            s.error = None;
                            s.media_type = Some(MediaType::Audio);
                            s.file_name = path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                        }

                        sink = Some(new_sink);
                    }
                    Ok(AudioCommand::Pause) => {
                        if let Some(ref s) = sink {
                            s.pause();
                            if let Ok(mut state) = state_clone.lock() {
                                state.play_state = PlayState::Paused;
                            }
                        }
                    }
                    Ok(AudioCommand::Resume) => {
                        if let Some(ref s) = sink {
                            s.play();
                            if let Ok(mut state) = state_clone.lock() {
                                state.play_state = PlayState::Playing;
                            }
                        }
                    }
                    Ok(AudioCommand::Stop) => {
                        if let Some(s) = sink.take() {
                            s.stop();
                        }
                        if let Ok(mut state) = state_clone.lock() {
                            state.play_state = PlayState::Stopped;
                            state.position = 0.0;
                        }
                    }
                    Ok(AudioCommand::SetVolume(vol)) => {
                        let vol = vol.clamp(0.0, 1.0);
                        if let Some(ref s) = sink {
                            s.set_volume(vol);
                        }
                        if let Ok(mut state) = state_clone.lock() {
                            state.volume = vol;
                        }
                    }
                    Ok(AudioCommand::Shutdown) => {
                        if let Some(s) = sink.take() {
                            s.stop();
                        }
                        break;
                    }
                    Err(_) => {
                        // Channel closed, exit thread
                        break;
                    }
                }

                // Check if playback finished
                if let Some(ref s) = sink {
                    if s.empty() {
                        if let Ok(mut state) = state_clone.lock() {
                            state.play_state = PlayState::Stopped;
                            state.position = state.duration;
                        }
                    }
                }
            }
        });

        Ok(Self { command_tx, state })
    }

    /// Get shared state reference
    pub fn state(&self) -> Arc<Mutex<MediaState>> {
        Arc::clone(&self.state)
    }

    /// Play a file
    pub fn play_file(&self, path: &PathBuf) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::Play(path.clone()))
            .map_err(|e| format!("Failed to send play command: {}", e))
    }

    /// Toggle play/pause
    pub fn toggle_pause(&self) {
        let is_paused = self
            .state
            .lock()
            .map(|s| s.play_state == PlayState::Paused)
            .unwrap_or(false);

        if is_paused {
            let _ = self.command_tx.send(AudioCommand::Resume);
        } else {
            let _ = self.command_tx.send(AudioCommand::Pause);
        }
    }

    /// Stop playback
    pub fn stop(&self) {
        let _ = self.command_tx.send(AudioCommand::Stop);
    }

    /// Set volume (0.0 to 1.0)
    pub fn set_volume(&self, volume: f32) {
        let _ = self.command_tx.send(AudioCommand::SetVolume(volume));
    }

    /// Adjust volume by delta
    pub fn adjust_volume(&self, delta: f32) {
        let current = self.state.lock().map(|s| s.volume).unwrap_or(0.8);
        self.set_volume(current + delta);
    }
}

impl Drop for AudioPlayerHandle {
    fn drop(&mut self) {
        let _ = self.command_tx.send(AudioCommand::Shutdown);
    }
}

// AudioPlayerHandle is Send + Sync because it only contains Send types
unsafe impl Send for AudioPlayerHandle {}
unsafe impl Sync for AudioPlayerHandle {}
