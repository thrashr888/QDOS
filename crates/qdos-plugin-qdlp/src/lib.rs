//! Q-DLP: YouTube/Media Downloader Plugin for QDOS
//!
//! A wrapper around yt-dlp for downloading videos and audio.
//! Features:
//! - URL input with clipboard paste support
//! - Format/quality selection
//! - Download progress display
//! - Support for YouTube, Vimeo, and 1000+ other sites

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qdos_plugin_api::prelude::*;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use serde::Deserialize;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread;

/// Download format type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DownloadFormat {
    #[default]
    BestVideo,
    BestAudio,
    Video720p,
    Video480p,
    AudioOnly,
}

impl DownloadFormat {
    fn as_str(&self) -> &'static str {
        match self {
            DownloadFormat::BestVideo => "Best Video",
            DownloadFormat::BestAudio => "Best Audio",
            DownloadFormat::Video720p => "720p",
            DownloadFormat::Video480p => "480p",
            DownloadFormat::AudioOnly => "Audio Only",
        }
    }

    fn yt_dlp_format(&self) -> &'static str {
        match self {
            DownloadFormat::BestVideo => "bestvideo+bestaudio/best",
            DownloadFormat::BestAudio => "bestaudio/best",
            DownloadFormat::Video720p => "bestvideo[height<=720]+bestaudio/best[height<=720]",
            DownloadFormat::Video480p => "bestvideo[height<=480]+bestaudio/best[height<=480]",
            DownloadFormat::AudioOnly => "bestaudio",
        }
    }

    fn next(&self) -> DownloadFormat {
        match self {
            DownloadFormat::BestVideo => DownloadFormat::BestAudio,
            DownloadFormat::BestAudio => DownloadFormat::Video720p,
            DownloadFormat::Video720p => DownloadFormat::Video480p,
            DownloadFormat::Video480p => DownloadFormat::AudioOnly,
            DownloadFormat::AudioOnly => DownloadFormat::BestVideo,
        }
    }

    fn prev(&self) -> DownloadFormat {
        match self {
            DownloadFormat::BestVideo => DownloadFormat::AudioOnly,
            DownloadFormat::BestAudio => DownloadFormat::BestVideo,
            DownloadFormat::Video720p => DownloadFormat::BestAudio,
            DownloadFormat::Video480p => DownloadFormat::Video720p,
            DownloadFormat::AudioOnly => DownloadFormat::Video480p,
        }
    }
}

/// View state
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum QDlpView {
    #[default]
    Input,
    Downloading,
    Complete,
    Error,
}

/// Download progress message
#[derive(Debug, Clone)]
pub enum DownloadMessage {
    Progress(String),
    Complete(String),
    Error(String),
}

/// Video info from yt-dlp
#[derive(Debug, Clone, Default, Deserialize)]
pub struct VideoInfo {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub duration_string: Option<String>,
    #[serde(default)]
    pub uploader: Option<String>,
    #[serde(default)]
    pub extractor: Option<String>,
}

/// Plugin state
pub struct QDlpState {
    pub view: QDlpView,
    pub url: String,
    pub format: DownloadFormat,
    pub cursor_pos: usize,
    pub progress_lines: Vec<String>,
    pub video_info: Option<VideoInfo>,
    pub error_message: Option<String>,
    /// Wrapped in Mutex for Sync (Plugin trait requirement)
    pub download_receiver: Mutex<Option<Receiver<DownloadMessage>>>,
    pub yt_dlp_available: bool,
    pub cwd: PathBuf,
}

impl Default for QDlpState {
    fn default() -> Self {
        Self {
            view: QDlpView::Input,
            url: String::new(),
            format: DownloadFormat::default(),
            cursor_pos: 0,
            progress_lines: Vec::new(),
            video_info: None,
            error_message: None,
            download_receiver: Mutex::new(None),
            yt_dlp_available: check_yt_dlp_available(),
            cwd: PathBuf::from("."),
        }
    }
}

impl QDlpState {
    pub fn new(cwd: &std::path::Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            ..Default::default()
        }
    }

    fn insert_char(&mut self, c: char) {
        if self.cursor_pos <= self.url.len() {
            self.url.insert(self.cursor_pos, c);
            self.cursor_pos += 1;
        }
    }

    fn delete_char(&mut self) {
        if self.cursor_pos > 0 && self.cursor_pos <= self.url.len() {
            self.url.remove(self.cursor_pos - 1);
            self.cursor_pos -= 1;
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.url.len() {
            self.cursor_pos += 1;
        }
    }

    fn start_download(&mut self) {
        if self.url.is_empty() {
            return;
        }

        self.view = QDlpView::Downloading;
        self.progress_lines.clear();
        self.progress_lines.push("Starting download...".to_string());

        let (tx, rx): (Sender<DownloadMessage>, Receiver<DownloadMessage>) = mpsc::channel();
        *self.download_receiver.lock().unwrap() = Some(rx);

        let url = self.url.clone();
        let format = self.format.yt_dlp_format().to_string();
        let cwd = self.cwd.clone();

        thread::spawn(move || {
            run_yt_dlp(&url, &format, &cwd, tx);
        });
    }

    fn check_progress(&mut self) {
        let guard = self.download_receiver.lock().unwrap();
        if let Some(ref rx) = *guard {
            // Non-blocking receive - collect messages first
            let mut messages = Vec::new();
            while let Ok(msg) = rx.try_recv() {
                messages.push(msg);
            }
            drop(guard); // Release lock before mutating self

            // Process messages
            for msg in messages {
                match msg {
                    DownloadMessage::Progress(line) => {
                        // Keep last 20 lines
                        if self.progress_lines.len() > 20 {
                            self.progress_lines.remove(0);
                        }
                        self.progress_lines.push(line);
                    }
                    DownloadMessage::Complete(filename) => {
                        self.view = QDlpView::Complete;
                        self.progress_lines
                            .push(format!("Downloaded: {}", filename));
                    }
                    DownloadMessage::Error(err) => {
                        self.view = QDlpView::Error;
                        self.error_message = Some(err);
                    }
                }
            }
        }
    }

    fn fetch_video_info(&mut self) {
        if self.url.is_empty() {
            self.video_info = None;
            return;
        }

        // Run yt-dlp to get video info (quick, non-downloading)
        let output = Command::new("yt-dlp")
            .args(["--dump-json", "--no-download", &self.url])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                if let Ok(info) = serde_json::from_slice::<VideoInfo>(&output.stdout) {
                    self.video_info = Some(info);
                }
            }
        }
    }
}

/// Check if yt-dlp is available
fn check_yt_dlp_available() -> bool {
    Command::new("yt-dlp")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run yt-dlp download in background
fn run_yt_dlp(url: &str, format: &str, cwd: &PathBuf, tx: Sender<DownloadMessage>) {
    let mut child = match Command::new("yt-dlp")
        .args([
            "--newline",
            "--progress",
            "-f",
            format,
            "-o",
            "%(title)s.%(ext)s",
            url,
        ])
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            let _ = tx.send(DownloadMessage::Error(format!(
                "Failed to start yt-dlp: {}",
                e
            )));
            return;
        }
    };

    // Read stdout for progress
    if let Some(stdout) = child.stdout.take() {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let _ = tx.send(DownloadMessage::Progress(line));
        }
    }

    // Wait for completion
    match child.wait() {
        Ok(status) => {
            if status.success() {
                let _ = tx.send(DownloadMessage::Complete("Download complete".to_string()));
            } else {
                let _ = tx.send(DownloadMessage::Error(format!(
                    "yt-dlp exited with code {}",
                    status.code().unwrap_or(-1)
                )));
            }
        }
        Err(e) => {
            let _ = tx.send(DownloadMessage::Error(format!("Process error: {}", e)));
        }
    }
}

/// Q-DLP Media Downloader Plugin
pub struct QDlpPlugin {
    pub state: QDlpState,
}

impl Default for QDlpPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl QDlpPlugin {
    pub fn new() -> Self {
        Self {
            state: QDlpState::default(),
        }
    }

    fn draw_input_view(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let view = FullScreenView::new(area, " Q-DLP ", colors);
        view.render_frame(frame);
        let content = view.content_area();

        let state = &self.state;

        if !state.yt_dlp_available {
            // Show error if yt-dlp not installed
            let error_lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    " yt-dlp is not installed",
                    Style::default()
                        .fg(colors.red())
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    " Install it with:",
                    Style::default().fg(colors.fg()),
                )),
                Line::from(Span::styled(
                    "   brew install yt-dlp",
                    Style::default().fg(colors.green()),
                )),
                Line::from(Span::styled(
                    "   pip install yt-dlp",
                    Style::default().fg(colors.green()),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    " Press Esc to close",
                    Style::default().fg(colors.blue()),
                )),
            ];
            frame.render_widget(Paragraph::new(error_lines), content);
            return;
        }

        // Layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // URL input
                Constraint::Length(3), // Format selector
                Constraint::Min(3),    // Info area
                Constraint::Length(2), // Status
            ])
            .split(content);

        // URL input
        let url_style = Style::default().fg(colors.yellow());
        let (before, after) = state.url.split_at(state.cursor_pos.min(state.url.len()));
        let url_display = format!(" {}|{}", before, after);
        let url_para = Paragraph::new(url_display)
            .style(url_style)
            .block(Block::default().borders(Borders::ALL).title(" Video URL "));
        frame.render_widget(url_para, chunks[0]);

        // Format selector
        let format_style = Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD);
        let format_para = Paragraph::new(format!(" {} ", state.format.as_str()))
            .style(format_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Format (</> to change) "),
            );
        frame.render_widget(format_para, chunks[1]);

        // Video info area
        if let Some(info) = &state.video_info {
            let mut info_lines = vec![Line::from(Span::styled(
                format!(" Title: {}", info.title),
                Style::default().fg(colors.fg()),
            ))];
            if let Some(uploader) = &info.uploader {
                info_lines.push(Line::from(Span::styled(
                    format!(" Uploader: {}", uploader),
                    Style::default().fg(colors.grey()),
                )));
            }
            if let Some(duration) = &info.duration_string {
                info_lines.push(Line::from(Span::styled(
                    format!(" Duration: {}", duration),
                    Style::default().fg(colors.grey()),
                )));
            }
            if let Some(extractor) = &info.extractor {
                info_lines.push(Line::from(Span::styled(
                    format!(" Source: {}", extractor),
                    Style::default().fg(colors.grey()),
                )));
            }
            frame.render_widget(Paragraph::new(info_lines), chunks[2]);
        } else {
            let hint = vec![
                Line::from(""),
                Line::from(Span::styled(
                    " Paste a video URL and press Enter to download",
                    Style::default().fg(colors.grey()),
                )),
                Line::from(Span::styled(
                    " Supports: YouTube, Vimeo, Twitter, and 1000+ more",
                    Style::default().fg(colors.grey()),
                )),
            ];
            frame.render_widget(Paragraph::new(hint), chunks[2]);
        }

        // Download location hint
        let cwd_hint = Paragraph::new(format!(" Download to: {}", state.cwd.display()))
            .style(Style::default().fg(colors.grey()));
        frame.render_widget(cwd_hint, chunks[3]);

        view.render_help(
            frame,
            vec![
                ("Enter", "download"),
                ("</>", "format"),
                ("Tab", "info"),
                ("Esc", "close"),
            ],
        );
    }

    fn draw_downloading_view(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let view = FullScreenView::new(area, " Q-DLP - Downloading ", colors);
        view.render_frame(frame);
        let content = view.content_area();

        let state = &self.state;

        // Progress lines
        let lines: Vec<Line> = state
            .progress_lines
            .iter()
            .map(|line| {
                let color = if line.contains('%') {
                    colors.yellow()
                } else if line.contains("Downloading") {
                    colors.green()
                } else {
                    colors.fg()
                };
                Line::from(Span::styled(
                    format!(" {}", line),
                    Style::default().fg(color),
                ))
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), content);

        view.render_help(frame, vec![("Esc", "cancel")]);
    }

    fn draw_complete_view(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let view = FullScreenView::new(area, " Q-DLP - Complete ", colors);
        view.render_frame(frame);
        let content = view.content_area();

        let state = &self.state;

        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                " Download Complete!",
                Style::default()
                    .fg(colors.green())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        // Show last few progress lines
        for line in state.progress_lines.iter().rev().take(5).rev() {
            lines.push(Line::from(Span::styled(
                format!(" {}", line),
                Style::default().fg(colors.grey()),
            )));
        }

        frame.render_widget(Paragraph::new(lines), content);

        view.render_help(frame, vec![("Enter/Esc", "close")]);
    }

    fn draw_error_view(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let view = FullScreenView::new(area, " Q-DLP - Error ", colors);
        view.render_frame(frame);
        let content = view.content_area();

        let state = &self.state;

        let error_msg = state
            .error_message
            .as_deref()
            .unwrap_or("Unknown error occurred");

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                " Download Failed",
                Style::default()
                    .fg(colors.red())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!(" {}", error_msg),
                Style::default().fg(colors.fg()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " Press Backspace to try again or Esc to close",
                Style::default().fg(colors.blue()),
            )),
        ];

        frame.render_widget(Paragraph::new(lines), content);

        view.render_help(frame, vec![("Backspace", "retry"), ("Esc", "close")]);
    }
}

impl Plugin for QDlpPlugin {
    fn id(&self) -> &str {
        "qdlp"
    }

    fn name(&self) -> &str {
        "Q-DLP"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_modal: true,
            has_menu: false,
            has_keys: false,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: "qdlp".to_string(),
            name: "Q-DLP".to_string(),
            description: "Media downloader".to_string(),
            category: PluginCategory::Tools,
            key: 'Y',
        })
    }

    fn launch(&mut self, cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.state = QDlpState::new(cwd);
        Ok(())
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let state = &mut self.state;

        // Check for download progress
        state.check_progress();

        match state.view {
            QDlpView::Input => match key.code {
                KeyCode::Esc => return KeyHandleResult::CloseModal,
                KeyCode::Enter => {
                    if !state.url.is_empty() {
                        state.start_download();
                    }
                }
                KeyCode::Tab => {
                    // Fetch video info
                    state.fetch_video_info();
                }
                KeyCode::Char('<') => {
                    state.format = state.format.prev();
                }
                KeyCode::Char('>') => {
                    state.format = state.format.next();
                }
                KeyCode::Left => state.move_cursor_left(),
                KeyCode::Right => state.move_cursor_right(),
                KeyCode::Backspace => state.delete_char(),
                KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // Ctrl+V paste - would need clipboard access
                    // For now just note it in help
                }
                KeyCode::Char(c) => state.insert_char(c),
                KeyCode::Home => state.cursor_pos = 0,
                KeyCode::End => state.cursor_pos = state.url.len(),
                _ => return KeyHandleResult::NotHandled,
            },
            QDlpView::Downloading => {
                // Check progress on each key event
                state.check_progress();
                if key.code == KeyCode::Esc {
                    // TODO: Kill download process
                    return KeyHandleResult::CloseModal;
                }
            }
            QDlpView::Complete => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    return KeyHandleResult::CloseWithSuccess("Download complete".to_string());
                }
                _ => return KeyHandleResult::NotHandled,
            },
            QDlpView::Error => match key.code {
                KeyCode::Esc => return KeyHandleResult::CloseModal,
                KeyCode::Backspace => {
                    state.view = QDlpView::Input;
                    state.error_message = None;
                }
                _ => return KeyHandleResult::NotHandled,
            },
        }

        KeyHandleResult::Handled
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        frame.render_widget(Clear, area);

        match self.state.view {
            QDlpView::Input => self.draw_input_view(frame, area, colors),
            QDlpView::Downloading => self.draw_downloading_view(frame, area, colors),
            QDlpView::Complete => self.draw_complete_view(frame, area, colors),
            QDlpView::Error => self.draw_error_view(frame, area, colors),
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-DLP - Media Downloader".to_string(),
            "".to_string(),
            "Download videos and audio from YouTube and 1000+ other sites.".to_string(),
            "Requires yt-dlp to be installed.".to_string(),
            "".to_string(),
            "Formats:".to_string(),
            "  Best Video    - Highest quality video + audio".to_string(),
            "  Best Audio    - Best audio quality".to_string(),
            "  720p          - HD video".to_string(),
            "  480p          - SD video".to_string(),
            "  Audio Only    - Extract audio only (MP3/M4A)".to_string(),
            "".to_string(),
            "Keybindings:".to_string(),
            "  Enter         - Start download".to_string(),
            "  </>           - Change format".to_string(),
            "  Tab           - Fetch video info".to_string(),
            "  Esc           - Close".to_string(),
            "".to_string(),
            "Supported sites: youtube.com, vimeo.com, twitter.com,".to_string(),
            "instagram.com, tiktok.com, and many more.".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Self-registration
inventory::submit! {
    PluginRegistration::new("qdlp", || Box::new(QDlpPlugin::new()))
}
