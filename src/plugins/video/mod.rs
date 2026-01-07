//! Video Player plugin
//!
//! Play video files using system video players (mpv, VLC, IINA).

mod modal;
pub mod state;

use crate::plugins::{
    KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem, PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{VideoState, VideoView};
use std::any::Any;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Video Player plugin
pub struct VideoPlugin {
    initialized: bool,
    pub state: VideoState,
}

impl Default for VideoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoPlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
            state: VideoState::new(),
        }
    }

    /// Open the modal for a specific video file
    pub fn open_modal(&mut self, file_path: Option<&PathBuf>) {
        self.state.detect_players();
        self.state.file_path = file_path.cloned();
        self.state.view = VideoView::Menu;
        self.state.error = None;

        // Auto-play if only one player and a file is selected
        if self.state.available_players.len() == 1 && self.state.file_path.is_some() {
            self.play_video();
        }
    }

    /// Play the video file with the selected player
    fn play_video(&mut self) {
        let Some(player) = self.state.selected().copied() else {
            self.state.error = Some("No player selected".to_string());
            self.state.view = VideoView::Error;
            return;
        };

        let Some(file_path) = &self.state.file_path else {
            self.state.error = Some("No file selected".to_string());
            self.state.view = VideoView::Error;
            return;
        };

        self.state.view = VideoView::Playing;
        self.state.is_playing = true;

        // Build command based on player
        let result = match player {
            state::VideoPlayer::Mpv => Command::new("mpv")
                .arg(file_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn(),
            state::VideoPlayer::Vlc => Command::new("vlc")
                .arg(file_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn(),
            state::VideoPlayer::Iina => Command::new("iina")
                .arg("--pip")
                .arg(file_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn(),
        };

        match result {
            Ok(_) => {
                // Video launched successfully - we don't track the process
                // since video players manage their own windows
                self.state.is_playing = false;
            }
            Err(e) => {
                self.state.error = Some(format!("Failed to play: {}", e));
                self.state.view = VideoView::Error;
                self.state.is_playing = false;
            }
        }
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

    fn init(&mut self, _cwd: &PathBuf) -> Result<(), String> {
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.initialized = false;
        Ok(())
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
                KeyCode::Up | KeyCode::Char('k') => {
                    self.state.select_prev();
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.state.select_next();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char('p') | KeyCode::Char('P') => {
                    if self.state.file_path.is_some() && !self.state.available_players.is_empty() {
                        self.play_video();
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            VideoView::Playing => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.state.view = VideoView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            VideoView::Error => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.state.view = VideoView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_video_modal(frame, area, &self.state, colors);
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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
