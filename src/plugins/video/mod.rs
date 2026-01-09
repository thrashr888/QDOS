//! Video Player plugin
//!
//! Play video files using system video players (mpv, VLC, IINA) or inline playback.

mod ascii;
mod ffmpeg;
mod modal;
pub mod state;

use crate::plugins::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{VideoFrame, VideoState, VideoView};
use std::any::Any;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

/// Parse ffmpeg time string (HH:MM:SS.ms) to seconds
fn parse_time_string(time: &str) -> Result<f32, ()> {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() != 3 {
        return Err(());
    }
    let hours: f32 = parts[0].parse().map_err(|_| ())?;
    let minutes: f32 = parts[1].parse().map_err(|_| ())?;
    let seconds: f32 = parts[2].parse().map_err(|_| ())?;
    Ok(hours * 3600.0 + minutes * 60.0 + seconds)
}

/// Shared frame state for thread-safe updates
pub type SharedFrame = Arc<Mutex<Option<VideoFrame>>>;

/// Video Player plugin
pub struct VideoPlugin {
    pub state: VideoState,
    /// Shared frame state updated by background extraction thread
    shared_frame: SharedFrame,
}

impl Default for VideoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoPlugin {
    pub fn new() -> Self {
        Self {
            state: VideoState::new(),
            shared_frame: Arc::new(Mutex::new(None)),
        }
    }

    /// Poll for new frames from background thread (non-blocking)
    pub fn poll_frames(&mut self) {
        // Check if there's a new frame from the background thread
        if let Ok(mut guard) = self.shared_frame.try_lock() {
            if let Some(frame) = guard.take() {
                self.state.inline_state.current_frame += 1;
                self.state.inline_state.position = frame.timestamp;
                self.state.inline_state.current_video_frame = Some(frame);
            }
        }
    }

    /// Open the modal for a specific video file
    pub fn open_modal(&mut self, file_path: Option<&PathBuf>) {
        self.state = VideoState::new();
        // Reset shared frame - create new Arc so background thread stops writing to old one
        self.shared_frame = Arc::new(Mutex::new(None));
        self.state.file_path = file_path.cloned();
        self.state.error = None;

        if let Some(path) = file_path {
            self.state.file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Detect sibling video files for prev/next navigation
            self.state.detect_siblings();
        }

        // Auto-play if file is selected and ffmpeg is available
        if self.state.file_path.is_some() {
            if ffmpeg::is_available() {
                self.start_inline_playback();
            } else {
                self.state.view = VideoView::FfmpegMissing;
            }
        } else {
            self.state.view = VideoView::Menu;
        }
    }

    /// Start inline playback
    fn start_inline_playback(&mut self) {
        if !ffmpeg::is_available() {
            self.state.view = VideoView::FfmpegMissing;
            return;
        }

        self.state.view = VideoView::InlinePlayer;
        self.state.inline_state.play_state = state::PlayState::Playing;

        // Start frame extraction in background
        if let Some(ref path) = self.state.file_path {
            self.start_frame_extraction(path.clone());
        }
    }

    /// Start ffmpeg frame extraction process in background thread
    fn start_frame_extraction(&mut self, path: PathBuf) {
        use ffmpeg_sidecar::command::FfmpegCommand;
        use ffmpeg_sidecar::event::FfmpegEvent;

        // Set frame dimensions (scaled down for terminal display)
        const FRAME_WIDTH: u32 = 160;
        const FRAME_HEIGHT: u32 = 90;
        const TARGET_FPS: u8 = 10;

        self.state.inline_state.target_fps = TARGET_FPS;

        // Clone the shared frame Arc for the background thread
        let shared_frame = Arc::clone(&self.shared_frame);

        // Spawn background thread for frame extraction
        thread::spawn(move || {
            let result = FfmpegCommand::new()
                .input(&*path.to_string_lossy())
                .args([
                    "-vf",
                    &format!("fps={},scale={}:{}", TARGET_FPS, FRAME_WIDTH, FRAME_HEIGHT),
                ])
                .args(["-f", "rawvideo"])
                .args(["-pix_fmt", "rgb24"])
                .args(["-"])
                .spawn();

            let Ok(mut ffmpeg) = result else {
                return;
            };

            let Ok(events) = ffmpeg.iter() else {
                return;
            };

            let mut current_time: f32 = 0.0;
            let frame_duration = 1.0 / TARGET_FPS as f32;

            for event in events {
                match event {
                    FfmpegEvent::OutputFrame(frame) => {
                        let video_frame = VideoFrame {
                            data: frame.data,
                            width: FRAME_WIDTH,
                            height: FRAME_HEIGHT,
                            timestamp: current_time,
                        };
                        current_time += frame_duration;

                        // Update shared frame - stop if we can't get lock (main thread dropped)
                        if let Ok(mut guard) = shared_frame.try_lock() {
                            *guard = Some(video_frame);
                        } else {
                            // Can't lock, main thread probably dropped Arc
                            break;
                        }

                        // Small sleep to match target FPS
                        thread::sleep(std::time::Duration::from_millis(1000 / TARGET_FPS as u64));
                    }
                    FfmpegEvent::Progress(progress) => {
                        if let Ok(secs) = parse_time_string(&progress.time) {
                            current_time = secs;
                        }
                    }
                    FfmpegEvent::Done => break,
                    FfmpegEvent::Error(_) => break,
                    _ => {}
                }
            }
        });
    }

    /// Switch to a different video file (for prev/next navigation)
    fn switch_to_file(&mut self, file_path: PathBuf) {
        // Stop current frame extraction by creating new Arc (old thread will stop)
        self.shared_frame = Arc::new(Mutex::new(None));

        // Keep sibling list and render mode
        let siblings = std::mem::take(&mut self.state.sibling_files);
        let new_index = siblings.iter().position(|p| p == &file_path).unwrap_or(0);
        let render_mode = self.state.inline_state.render_mode;

        // Reset state for new file but preserve render mode
        self.state = VideoState::new();
        self.state.inline_state.render_mode = render_mode;
        self.state.file_path = Some(file_path.clone());
        self.state.file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        self.state.sibling_files = siblings;
        self.state.current_file_index = new_index;

        // Auto-play inline
        self.start_inline_playback();
    }

    /// Check if a file is a video file
    pub fn is_video_file(path: &PathBuf) -> bool {
        path.extension()
            .map(|ext| {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                matches!(
                    ext_lower.as_str(),
                    "mp4"
                        | "mkv"
                        | "avi"
                        | "mov"
                        | "wmv"
                        | "flv"
                        | "webm"
                        | "m4v"
                        | "mpg"
                        | "mpeg"
                        | "3gp"
                        | "ogv"
                        | "ts"
                        | "mts"
                        | "vob"
                )
            })
            .unwrap_or(false)
    }
}

impl Plugin for VideoPlugin {
    fn id(&self) -> &str {
        "video"
    }

    fn name(&self) -> &str {
        "Video Player"
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
        // Available if any video player is installed
        state::VideoPlayer::all().iter().any(|p| p.is_available())
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Video".to_string(),
            key: 'V',
            description: "Play video files".to_string(),
            priority: 36,
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
            VideoView::Menu => match key.code {
                KeyCode::Esc => KeyHandleResult::CloseModal,
                KeyCode::Enter | KeyCode::Char('p') | KeyCode::Char('P') => {
                    if self.state.file_path.is_some() {
                        self.start_inline_playback();
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            VideoView::Playing => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    KeyHandleResult::CloseModal
                }
                KeyCode::Char('[') | KeyCode::Left => {
                    if let Some(prev) = self.state.prev_file() {
                        self.switch_to_file(prev);
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char(']') | KeyCode::Right => {
                    if let Some(next) = self.state.next_file() {
                        self.switch_to_file(next);
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            VideoView::InlinePlayer => match key.code {
                KeyCode::Esc => KeyHandleResult::CloseModal,
                KeyCode::Char(' ') => {
                    // Toggle play/pause
                    self.state.inline_state.play_state = match self.state.inline_state.play_state {
                        state::PlayState::Playing => state::PlayState::Paused,
                        state::PlayState::Paused => state::PlayState::Playing,
                        state::PlayState::Stopped => state::PlayState::Playing,
                    };
                    KeyHandleResult::Handled
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    // Toggle between image and ASCII render modes
                    self.state.toggle_render_mode();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('[') | KeyCode::Left => {
                    if let Some(prev) = self.state.prev_file() {
                        self.switch_to_file(prev);
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char(']') | KeyCode::Right => {
                    if let Some(next) = self.state.next_file() {
                        self.switch_to_file(next);
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            VideoView::FfmpegMissing => match key.code {
                KeyCode::Esc | KeyCode::Enter => KeyHandleResult::CloseModal,
                _ => KeyHandleResult::Handled,
            },
            VideoView::Error => match key.code {
                KeyCode::Esc | KeyCode::Enter => KeyHandleResult::CloseModal,
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_video_modal(frame, area, &self.state, colors);
    }

    fn tick(&mut self) {
        // Poll for new frames from background thread during inline playback
        if self.state.view == VideoView::InlinePlayer
            && self.state.inline_state.play_state == state::PlayState::Playing
        {
            self.poll_frames();
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Video Player".to_string(),
            "".to_string(),
            "Play video files using system video players.".to_string(),
            "".to_string(),
            "Supported players:".to_string(),
            "  mpv  - Lightweight, terminal-friendly".to_string(),
            "  IINA - Modern macOS media player".to_string(),
            "  VLC  - Cross-platform media player".to_string(),
            "".to_string(),
            "To install:".to_string(),
            "  brew install mpv".to_string(),
            "  brew install --cask iina".to_string(),
            "  brew install vlc".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Video".to_string(),
            description: "Play video files".to_string(),
            category: PluginCategory::Games,
            key: 'Z',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.open_modal(selected_file);
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
