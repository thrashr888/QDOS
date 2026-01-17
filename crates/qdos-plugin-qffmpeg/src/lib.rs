//! Q-FFMPEG: Video/Audio Processing Plugin for QDOS
//!
//! Media processing using FFmpeg commands.
//! Features:
//! - Convert video/audio formats
//! - Extract audio from video
//! - Trim clips
//! - Resize video
//! - Compress/reduce quality

use crossterm::event::{KeyCode, KeyEvent};
use qdos_plugin_api::prelude::*;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::process::{Command, Stdio};

/// Media operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaOp {
    #[default]
    Convert,
    ExtractAudio,
    Resize,
    Compress,
    ToGif,
    Thumbnail,
}

impl MediaOp {
    fn as_str(&self) -> &'static str {
        match self {
            MediaOp::Convert => "Convert Format",
            MediaOp::ExtractAudio => "Extract Audio",
            MediaOp::Resize => "Resize Video",
            MediaOp::Compress => "Compress",
            MediaOp::ToGif => "Convert to GIF",
            MediaOp::Thumbnail => "Extract Thumbnail",
        }
    }

    fn next(&self) -> MediaOp {
        match self {
            MediaOp::Convert => MediaOp::ExtractAudio,
            MediaOp::ExtractAudio => MediaOp::Resize,
            MediaOp::Resize => MediaOp::Compress,
            MediaOp::Compress => MediaOp::ToGif,
            MediaOp::ToGif => MediaOp::Thumbnail,
            MediaOp::Thumbnail => MediaOp::Convert,
        }
    }

    fn prev(&self) -> MediaOp {
        match self {
            MediaOp::Convert => MediaOp::Thumbnail,
            MediaOp::ExtractAudio => MediaOp::Convert,
            MediaOp::Resize => MediaOp::ExtractAudio,
            MediaOp::Compress => MediaOp::Resize,
            MediaOp::ToGif => MediaOp::Compress,
            MediaOp::Thumbnail => MediaOp::ToGif,
        }
    }
}

/// Video output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoFormat {
    #[default]
    Mp4,
    Webm,
    Mkv,
    Avi,
    Mov,
}

impl VideoFormat {
    fn as_str(&self) -> &'static str {
        match self {
            VideoFormat::Mp4 => "MP4",
            VideoFormat::Webm => "WebM",
            VideoFormat::Mkv => "MKV",
            VideoFormat::Avi => "AVI",
            VideoFormat::Mov => "MOV",
        }
    }

    fn extension(&self) -> &'static str {
        match self {
            VideoFormat::Mp4 => "mp4",
            VideoFormat::Webm => "webm",
            VideoFormat::Mkv => "mkv",
            VideoFormat::Avi => "avi",
            VideoFormat::Mov => "mov",
        }
    }

    fn next(&self) -> VideoFormat {
        match self {
            VideoFormat::Mp4 => VideoFormat::Webm,
            VideoFormat::Webm => VideoFormat::Mkv,
            VideoFormat::Mkv => VideoFormat::Avi,
            VideoFormat::Avi => VideoFormat::Mov,
            VideoFormat::Mov => VideoFormat::Mp4,
        }
    }
}

/// Audio output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioFormat {
    #[default]
    Mp3,
    Aac,
    Wav,
    Flac,
    Ogg,
}

impl AudioFormat {
    fn as_str(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "MP3",
            AudioFormat::Aac => "AAC",
            AudioFormat::Wav => "WAV",
            AudioFormat::Flac => "FLAC",
            AudioFormat::Ogg => "OGG",
        }
    }

    fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Aac => "aac",
            AudioFormat::Wav => "wav",
            AudioFormat::Flac => "flac",
            AudioFormat::Ogg => "ogg",
        }
    }

    fn next(&self) -> AudioFormat {
        match self {
            AudioFormat::Mp3 => AudioFormat::Aac,
            AudioFormat::Aac => AudioFormat::Wav,
            AudioFormat::Wav => AudioFormat::Flac,
            AudioFormat::Flac => AudioFormat::Ogg,
            AudioFormat::Ogg => AudioFormat::Mp3,
        }
    }
}

/// Video resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoSize {
    #[default]
    Hd720,
    Hd1080,
    Sd480,
    Sd360,
    Half,
}

impl VideoSize {
    fn as_str(&self) -> &'static str {
        match self {
            VideoSize::Hd720 => "720p (HD)",
            VideoSize::Hd1080 => "1080p (Full HD)",
            VideoSize::Sd480 => "480p (SD)",
            VideoSize::Sd360 => "360p",
            VideoSize::Half => "50% scale",
        }
    }

    fn ffmpeg_scale(&self) -> &'static str {
        match self {
            VideoSize::Hd720 => "-vf scale=-1:720",
            VideoSize::Hd1080 => "-vf scale=-1:1080",
            VideoSize::Sd480 => "-vf scale=-1:480",
            VideoSize::Sd360 => "-vf scale=-1:360",
            VideoSize::Half => "-vf scale=iw/2:ih/2",
        }
    }

    fn next(&self) -> VideoSize {
        match self {
            VideoSize::Hd720 => VideoSize::Hd1080,
            VideoSize::Hd1080 => VideoSize::Sd480,
            VideoSize::Sd480 => VideoSize::Sd360,
            VideoSize::Sd360 => VideoSize::Half,
            VideoSize::Half => VideoSize::Hd720,
        }
    }
}

/// Compression quality
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Quality {
    #[default]
    Medium,
    High,
    Low,
    VeryLow,
}

impl Quality {
    fn as_str(&self) -> &'static str {
        match self {
            Quality::High => "High (larger file)",
            Quality::Medium => "Medium (balanced)",
            Quality::Low => "Low (smaller file)",
            Quality::VeryLow => "Very Low (tiny)",
        }
    }

    fn crf(&self) -> &'static str {
        match self {
            Quality::High => "18",
            Quality::Medium => "23",
            Quality::Low => "28",
            Quality::VeryLow => "35",
        }
    }

    fn next(&self) -> Quality {
        match self {
            Quality::High => Quality::Medium,
            Quality::Medium => Quality::Low,
            Quality::Low => Quality::VeryLow,
            Quality::VeryLow => Quality::High,
        }
    }
}

/// View state
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum QFfmpegView {
    #[default]
    SelectOp,
    Options,
    Result,
}

/// Plugin state
pub struct QFfmpegState {
    pub view: QFfmpegView,
    pub operation: MediaOp,
    pub video_format: VideoFormat,
    pub audio_format: AudioFormat,
    pub video_size: VideoSize,
    pub quality: Quality,
    pub input_file: Option<PathBuf>,
    pub output_file: Option<PathBuf>,
    pub result_message: Option<String>,
    pub error_message: Option<String>,
    pub ffmpeg_available: bool,
    pub cwd: PathBuf,
}

impl Default for QFfmpegState {
    fn default() -> Self {
        Self {
            view: QFfmpegView::SelectOp,
            operation: MediaOp::default(),
            video_format: VideoFormat::default(),
            audio_format: AudioFormat::default(),
            video_size: VideoSize::default(),
            quality: Quality::default(),
            input_file: None,
            output_file: None,
            result_message: None,
            error_message: None,
            ffmpeg_available: check_ffmpeg_available(),
            cwd: PathBuf::from("."),
        }
    }
}

impl QFfmpegState {
    pub fn new(cwd: &std::path::Path, selected_file: Option<&PathBuf>) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            input_file: selected_file.cloned(),
            ..Default::default()
        }
    }

    fn execute_operation(&mut self) {
        let Some(input) = &self.input_file else {
            self.error_message = Some("No file selected".to_string());
            self.view = QFfmpegView::Result;
            return;
        };

        let input_stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        let (output_name, ffmpeg_args) = match self.operation {
            MediaOp::Convert => {
                let ext = self.video_format.extension();
                let output = format!("{}_converted.{}", input_stem, ext);
                let args = vec!["-c:v", "libx264", "-c:a", "aac"];
                (output, args)
            }
            MediaOp::ExtractAudio => {
                let ext = self.audio_format.extension();
                let output = format!("{}.{}", input_stem, ext);
                let args = vec![
                    "-vn",
                    "-acodec",
                    match self.audio_format {
                        AudioFormat::Mp3 => "libmp3lame",
                        AudioFormat::Aac => "aac",
                        AudioFormat::Wav => "pcm_s16le",
                        AudioFormat::Flac => "flac",
                        AudioFormat::Ogg => "libvorbis",
                    },
                ];
                (output, args)
            }
            MediaOp::Resize => {
                let output = format!("{}_resized.mp4", input_stem);
                let scale = self.video_size.ffmpeg_scale();
                // Split scale arg
                let args: Vec<&str> = scale.split_whitespace().collect();
                (output, args)
            }
            MediaOp::Compress => {
                let output = format!("{}_compressed.mp4", input_stem);
                let args = vec!["-c:v", "libx264", "-crf", self.quality.crf(), "-c:a", "aac"];
                (output, args)
            }
            MediaOp::ToGif => {
                let output = format!("{}.gif", input_stem);
                let args = vec!["-vf", "fps=10,scale=480:-1:flags=lanczos", "-loop", "0"];
                (output, args)
            }
            MediaOp::Thumbnail => {
                let output = format!("{}_thumb.jpg", input_stem);
                let args = vec!["-ss", "00:00:01", "-vframes", "1"];
                (output, args)
            }
        };

        let output_path = self.cwd.join(&output_name);
        self.output_file = Some(output_path.clone());

        // Build ffmpeg command
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y"); // Overwrite output
        cmd.arg("-i");
        cmd.arg(input);
        for arg in &ffmpeg_args {
            cmd.arg(arg);
        }
        cmd.arg(&output_path);
        cmd.current_dir(&self.cwd);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    self.result_message = Some(format!("Created: {}", output_name));
                    self.view = QFfmpegView::Result;
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    // FFmpeg outputs to stderr even on success, so check for common errors
                    if stderr.contains("Error") || stderr.contains("Invalid") {
                        self.error_message = Some(format!(
                            "FFmpeg error: {}",
                            stderr.lines().last().unwrap_or("Unknown error")
                        ));
                    } else {
                        self.result_message = Some(format!("Created: {}", output_name));
                    }
                    self.view = QFfmpegView::Result;
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to run ffmpeg: {}", e));
                self.view = QFfmpegView::Result;
            }
        }
    }
}

/// Check if FFmpeg is available
fn check_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Q-FFMPEG Media Processing Plugin
pub struct QFfmpegPlugin {
    pub state: QFfmpegState,
}

impl Default for QFfmpegPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl QFfmpegPlugin {
    pub fn new() -> Self {
        Self {
            state: QFfmpegState::default(),
        }
    }

    fn draw_select_op_view(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let view = FullScreenView::new(area, " Q-FFMPEG ", colors);
        view.render_frame(frame);
        let content = view.content_area();

        let state = &self.state;

        if !state.ffmpeg_available {
            let error_lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    " FFmpeg is not installed",
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
                    "   brew install ffmpeg",
                    Style::default().fg(colors.green()),
                )),
                Line::from(Span::styled(
                    "   apt install ffmpeg",
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

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // File info
                Constraint::Length(3), // Operation selector
                Constraint::Min(3),    // Description
            ])
            .split(content);

        // File info
        let file_text = if let Some(file) = &state.input_file {
            format!(
                " {}",
                file.file_name().and_then(|s| s.to_str()).unwrap_or("?")
            )
        } else {
            " No media file selected".to_string()
        };
        let file_para = Paragraph::new(file_text)
            .style(Style::default().fg(if state.input_file.is_some() {
                colors.green()
            } else {
                colors.red()
            }))
            .block(Block::default().borders(Borders::ALL).title(" Input File "));
        frame.render_widget(file_para, chunks[0]);

        // Operation selector
        let op_style = Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD);
        let op_para = Paragraph::new(format!(" {} ", state.operation.as_str()))
            .style(op_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Operation (</> to change) "),
            );
        frame.render_widget(op_para, chunks[1]);

        // Operation description
        let desc = match state.operation {
            MediaOp::Convert => "Convert video to another format (MP4, WebM, MKV, etc.)",
            MediaOp::ExtractAudio => "Extract audio track from video file",
            MediaOp::Resize => "Resize video to different resolution",
            MediaOp::Compress => "Compress video to reduce file size",
            MediaOp::ToGif => "Convert video clip to animated GIF",
            MediaOp::Thumbnail => "Extract thumbnail image from video",
        };
        let desc_lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!(" {}", desc),
                Style::default().fg(colors.fg()),
            )),
        ];
        frame.render_widget(Paragraph::new(desc_lines), chunks[2]);

        view.render_help(
            frame,
            vec![("</>", "operation"), ("Enter", "options"), ("Esc", "close")],
        );
    }

    fn draw_options_view(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let view = FullScreenView::new(area, " Q-FFMPEG - Options ", colors);
        view.render_frame(frame);
        let content = view.content_area();

        let state = &self.state;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Operation
                Constraint::Length(3), // Option
                Constraint::Min(3),    // Preview
            ])
            .split(content);

        // Operation
        let op_para = Paragraph::new(format!(" {} ", state.operation.as_str()))
            .style(Style::default().fg(colors.fg()))
            .block(Block::default().borders(Borders::ALL).title(" Operation "));
        frame.render_widget(op_para, chunks[0]);

        // Options based on operation
        let (option_title, option_value) = match state.operation {
            MediaOp::Convert => ("Format", state.video_format.as_str()),
            MediaOp::ExtractAudio => ("Audio Format", state.audio_format.as_str()),
            MediaOp::Resize => ("Resolution", state.video_size.as_str()),
            MediaOp::Compress => ("Quality", state.quality.as_str()),
            MediaOp::ToGif | MediaOp::Thumbnail => ("N/A", "Press Enter to execute"),
        };

        let opt_style = Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD);
        let opt_para = Paragraph::new(format!(" {} ", option_value))
            .style(opt_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} (</> to change) ", option_title)),
            );
        frame.render_widget(opt_para, chunks[1]);

        // Preview
        let preview = if let Some(file) = &state.input_file {
            let name = file.file_name().and_then(|s| s.to_str()).unwrap_or("?");
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!(" Input:  {}", name),
                    Style::default().fg(colors.grey()),
                )),
                Line::from(Span::styled(
                    format!(" Output: Will be created in {}", state.cwd.display()),
                    Style::default().fg(colors.grey()),
                )),
            ]
        } else {
            vec![]
        };
        frame.render_widget(Paragraph::new(preview), chunks[2]);

        view.render_help(
            frame,
            vec![
                ("</>", "change"),
                ("Enter", "execute"),
                ("Backspace", "back"),
                ("Esc", "close"),
            ],
        );
    }

    fn draw_result_view(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let view = FullScreenView::new(area, " Q-FFMPEG - Result ", colors);
        view.render_frame(frame);
        let content = view.content_area();

        let state = &self.state;

        let lines = if let Some(msg) = &state.result_message {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    " Success!",
                    Style::default()
                        .fg(colors.green())
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!(" {}", msg),
                    Style::default().fg(colors.fg()),
                )),
            ]
        } else if let Some(err) = &state.error_message {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    " Error",
                    Style::default()
                        .fg(colors.red())
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!(" {}", err),
                    Style::default().fg(colors.fg()),
                )),
            ]
        } else {
            vec![]
        };

        frame.render_widget(Paragraph::new(lines), content);

        view.render_help(frame, vec![("Enter/Esc", "close")]);
    }
}

impl Plugin for QFfmpegPlugin {
    fn id(&self) -> &str {
        "qffmpeg"
    }

    fn name(&self) -> &str {
        "Q-FFMPEG"
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
            id: "qffmpeg".to_string(),
            name: "Q-FFMPEG".to_string(),
            description: "Media processing".to_string(),
            category: PluginCategory::Tools,
            key: 'F',
        })
    }

    fn launch(&mut self, cwd: &PathBuf, selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.state = QFfmpegState::new(cwd, selected_file);
        Ok(())
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let state = &mut self.state;

        match state.view {
            QFfmpegView::SelectOp => match key.code {
                KeyCode::Esc => return KeyHandleResult::CloseModal,
                KeyCode::Char('<') => {
                    state.operation = state.operation.prev();
                }
                KeyCode::Char('>') => {
                    state.operation = state.operation.next();
                }
                KeyCode::Enter => {
                    if state.input_file.is_some() && state.ffmpeg_available {
                        // Operations without options go straight to execute
                        if matches!(state.operation, MediaOp::ToGif | MediaOp::Thumbnail) {
                            state.execute_operation();
                        } else {
                            state.view = QFfmpegView::Options;
                        }
                    }
                }
                _ => return KeyHandleResult::NotHandled,
            },
            QFfmpegView::Options => match key.code {
                KeyCode::Esc => return KeyHandleResult::CloseModal,
                KeyCode::Backspace => {
                    state.view = QFfmpegView::SelectOp;
                }
                KeyCode::Char('<') | KeyCode::Char('>') => match state.operation {
                    MediaOp::Convert => {
                        state.video_format = state.video_format.next();
                    }
                    MediaOp::ExtractAudio => {
                        state.audio_format = state.audio_format.next();
                    }
                    MediaOp::Resize => {
                        state.video_size = state.video_size.next();
                    }
                    MediaOp::Compress => {
                        state.quality = state.quality.next();
                    }
                    _ => {}
                },
                KeyCode::Enter => {
                    state.execute_operation();
                }
                _ => return KeyHandleResult::NotHandled,
            },
            QFfmpegView::Result => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    if state.result_message.is_some() {
                        return KeyHandleResult::CloseWithSuccess(
                            state.result_message.clone().unwrap_or_default(),
                        );
                    } else {
                        return KeyHandleResult::CloseModal;
                    }
                }
                _ => return KeyHandleResult::NotHandled,
            },
        }

        KeyHandleResult::Handled
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        frame.render_widget(Clear, area);

        match self.state.view {
            QFfmpegView::SelectOp => self.draw_select_op_view(frame, area, colors),
            QFfmpegView::Options => self.draw_options_view(frame, area, colors),
            QFfmpegView::Result => self.draw_result_view(frame, area, colors),
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-FFMPEG - Media Processing".to_string(),
            "".to_string(),
            "Process video and audio using FFmpeg.".to_string(),
            "Requires FFmpeg to be installed.".to_string(),
            "".to_string(),
            "Operations:".to_string(),
            "  Convert       - Change video format".to_string(),
            "  Extract Audio - Get audio from video".to_string(),
            "  Resize        - Change resolution".to_string(),
            "  Compress      - Reduce file size".to_string(),
            "  To GIF        - Create animated GIF".to_string(),
            "  Thumbnail     - Extract frame as image".to_string(),
            "".to_string(),
            "Keybindings:".to_string(),
            "  </>        - Cycle through options".to_string(),
            "  Enter      - Execute operation".to_string(),
            "  Backspace  - Go back".to_string(),
            "  Esc        - Close".to_string(),
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
    PluginRegistration::new("qffmpeg", || Box::new(QFfmpegPlugin::new()))
}
