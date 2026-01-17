//! Q-MAGICK: ImageMagick Wrapper Plugin for QDOS
//!
//! Image manipulation using ImageMagick commands.
//! Features:
//! - Resize images
//! - Convert formats
//! - Rotate and flip
//! - Apply filters (blur, sharpen, grayscale)
//! - Batch processing

use crossterm::event::{KeyCode, KeyEvent};
use qdos_plugin_api::prelude::*;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::process::{Command, Stdio};

/// Image operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageOp {
    #[default]
    Resize,
    Convert,
    Rotate,
    Flip,
    Grayscale,
    Blur,
    Sharpen,
    Negate,
}

impl ImageOp {
    fn as_str(&self) -> &'static str {
        match self {
            ImageOp::Resize => "Resize",
            ImageOp::Convert => "Convert Format",
            ImageOp::Rotate => "Rotate",
            ImageOp::Flip => "Flip",
            ImageOp::Grayscale => "Grayscale",
            ImageOp::Blur => "Blur",
            ImageOp::Sharpen => "Sharpen",
            ImageOp::Negate => "Negate",
        }
    }

    fn next(&self) -> ImageOp {
        match self {
            ImageOp::Resize => ImageOp::Convert,
            ImageOp::Convert => ImageOp::Rotate,
            ImageOp::Rotate => ImageOp::Flip,
            ImageOp::Flip => ImageOp::Grayscale,
            ImageOp::Grayscale => ImageOp::Blur,
            ImageOp::Blur => ImageOp::Sharpen,
            ImageOp::Sharpen => ImageOp::Negate,
            ImageOp::Negate => ImageOp::Resize,
        }
    }

    fn prev(&self) -> ImageOp {
        match self {
            ImageOp::Resize => ImageOp::Negate,
            ImageOp::Convert => ImageOp::Resize,
            ImageOp::Rotate => ImageOp::Convert,
            ImageOp::Flip => ImageOp::Rotate,
            ImageOp::Grayscale => ImageOp::Flip,
            ImageOp::Blur => ImageOp::Grayscale,
            ImageOp::Sharpen => ImageOp::Blur,
            ImageOp::Negate => ImageOp::Sharpen,
        }
    }
}

/// Resize dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResizeOption {
    #[default]
    Half,
    Quarter,
    Double,
    Width800,
    Width1920,
    Custom,
}

impl ResizeOption {
    fn as_str(&self) -> &'static str {
        match self {
            ResizeOption::Half => "50%",
            ResizeOption::Quarter => "25%",
            ResizeOption::Double => "200%",
            ResizeOption::Width800 => "800px wide",
            ResizeOption::Width1920 => "1920px wide",
            ResizeOption::Custom => "Custom",
        }
    }

    fn magick_arg(self) -> String {
        match self {
            ResizeOption::Half => "50%".to_string(),
            ResizeOption::Quarter => "25%".to_string(),
            ResizeOption::Double => "200%".to_string(),
            ResizeOption::Width800 => "800x".to_string(),
            ResizeOption::Width1920 => "1920x".to_string(),
            ResizeOption::Custom => "100%".to_string(),
        }
    }

    fn next(&self) -> ResizeOption {
        match self {
            ResizeOption::Half => ResizeOption::Quarter,
            ResizeOption::Quarter => ResizeOption::Double,
            ResizeOption::Double => ResizeOption::Width800,
            ResizeOption::Width800 => ResizeOption::Width1920,
            ResizeOption::Width1920 => ResizeOption::Custom,
            ResizeOption::Custom => ResizeOption::Half,
        }
    }
}

/// Output format for conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Png,
    Jpg,
    Gif,
    Webp,
    Bmp,
    Tiff,
}

impl OutputFormat {
    fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Png => "PNG",
            OutputFormat::Jpg => "JPEG",
            OutputFormat::Gif => "GIF",
            OutputFormat::Webp => "WebP",
            OutputFormat::Bmp => "BMP",
            OutputFormat::Tiff => "TIFF",
        }
    }

    fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Jpg => "jpg",
            OutputFormat::Gif => "gif",
            OutputFormat::Webp => "webp",
            OutputFormat::Bmp => "bmp",
            OutputFormat::Tiff => "tiff",
        }
    }

    fn next(&self) -> OutputFormat {
        match self {
            OutputFormat::Png => OutputFormat::Jpg,
            OutputFormat::Jpg => OutputFormat::Gif,
            OutputFormat::Gif => OutputFormat::Webp,
            OutputFormat::Webp => OutputFormat::Bmp,
            OutputFormat::Bmp => OutputFormat::Tiff,
            OutputFormat::Tiff => OutputFormat::Png,
        }
    }
}

/// Rotation angle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RotateOption {
    #[default]
    Rotate90,
    Rotate180,
    Rotate270,
}

impl RotateOption {
    fn as_str(&self) -> &'static str {
        match self {
            RotateOption::Rotate90 => "90 degrees",
            RotateOption::Rotate180 => "180 degrees",
            RotateOption::Rotate270 => "270 degrees",
        }
    }

    fn degrees(&self) -> i32 {
        match self {
            RotateOption::Rotate90 => 90,
            RotateOption::Rotate180 => 180,
            RotateOption::Rotate270 => 270,
        }
    }

    fn next(&self) -> RotateOption {
        match self {
            RotateOption::Rotate90 => RotateOption::Rotate180,
            RotateOption::Rotate180 => RotateOption::Rotate270,
            RotateOption::Rotate270 => RotateOption::Rotate90,
        }
    }
}

/// Flip direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlipOption {
    #[default]
    Horizontal,
    Vertical,
    Both,
}

impl FlipOption {
    fn as_str(&self) -> &'static str {
        match self {
            FlipOption::Horizontal => "Horizontal",
            FlipOption::Vertical => "Vertical",
            FlipOption::Both => "Both",
        }
    }

    fn next(&self) -> FlipOption {
        match self {
            FlipOption::Horizontal => FlipOption::Vertical,
            FlipOption::Vertical => FlipOption::Both,
            FlipOption::Both => FlipOption::Horizontal,
        }
    }
}

/// View state
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum QMagickView {
    #[default]
    SelectOp,
    Options,
    Result,
}

/// Plugin state
pub struct QMagickState {
    pub view: QMagickView,
    pub operation: ImageOp,
    pub resize_option: ResizeOption,
    pub output_format: OutputFormat,
    pub rotate_option: RotateOption,
    pub flip_option: FlipOption,
    pub input_file: Option<PathBuf>,
    pub output_file: Option<PathBuf>,
    pub result_message: Option<String>,
    pub error_message: Option<String>,
    pub magick_available: bool,
    pub cwd: PathBuf,
}

impl Default for QMagickState {
    fn default() -> Self {
        Self {
            view: QMagickView::SelectOp,
            operation: ImageOp::default(),
            resize_option: ResizeOption::default(),
            output_format: OutputFormat::default(),
            rotate_option: RotateOption::default(),
            flip_option: FlipOption::default(),
            input_file: None,
            output_file: None,
            result_message: None,
            error_message: None,
            magick_available: check_magick_available(),
            cwd: PathBuf::from("."),
        }
    }
}

impl QMagickState {
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
            self.view = QMagickView::Result;
            return;
        };

        // Determine output filename
        let input_stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let input_ext = input.extension().and_then(|s| s.to_str()).unwrap_or("png");

        let (output_name, magick_args) = match self.operation {
            ImageOp::Resize => {
                let suffix = match self.resize_option {
                    ResizeOption::Half => "_50pct",
                    ResizeOption::Quarter => "_25pct",
                    ResizeOption::Double => "_200pct",
                    ResizeOption::Width800 => "_800w",
                    ResizeOption::Width1920 => "_1920w",
                    ResizeOption::Custom => "_resized",
                };
                let output = format!("{}{}.{}", input_stem, suffix, input_ext);
                let args = vec!["-resize".to_string(), self.resize_option.magick_arg()];
                (output, args)
            }
            ImageOp::Convert => {
                let output = format!("{}.{}", input_stem, self.output_format.extension());
                (output, vec![])
            }
            ImageOp::Rotate => {
                let output = format!(
                    "{}_rot{}.{}",
                    input_stem,
                    self.rotate_option.degrees(),
                    input_ext
                );
                let args = vec![
                    "-rotate".to_string(),
                    self.rotate_option.degrees().to_string(),
                ];
                (output, args)
            }
            ImageOp::Flip => {
                let (suffix, args) = match self.flip_option {
                    FlipOption::Horizontal => ("_floph", vec!["-flop".to_string()]),
                    FlipOption::Vertical => ("_flipv", vec!["-flip".to_string()]),
                    FlipOption::Both => {
                        ("_flipboth", vec!["-flip".to_string(), "-flop".to_string()])
                    }
                };
                let output = format!("{}{}.{}", input_stem, suffix, input_ext);
                (output, args)
            }
            ImageOp::Grayscale => {
                let output = format!("{}_gray.{}", input_stem, input_ext);
                let args = vec!["-colorspace".to_string(), "Gray".to_string()];
                (output, args)
            }
            ImageOp::Blur => {
                let output = format!("{}_blur.{}", input_stem, input_ext);
                let args = vec!["-blur".to_string(), "0x3".to_string()];
                (output, args)
            }
            ImageOp::Sharpen => {
                let output = format!("{}_sharp.{}", input_stem, input_ext);
                let args = vec!["-sharpen".to_string(), "0x2".to_string()];
                (output, args)
            }
            ImageOp::Negate => {
                let output = format!("{}_neg.{}", input_stem, input_ext);
                let args = vec!["-negate".to_string()];
                (output, args)
            }
        };

        let output_path = self.cwd.join(&output_name);
        self.output_file = Some(output_path.clone());

        // Build magick command
        let mut cmd = Command::new("magick");
        cmd.arg(input);
        for arg in magick_args {
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
                    self.view = QMagickView::Result;
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    self.error_message = Some(format!("ImageMagick error: {}", stderr.trim()));
                    self.view = QMagickView::Result;
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to run magick: {}", e));
                self.view = QMagickView::Result;
            }
        }
    }
}

/// Check if ImageMagick is available
fn check_magick_available() -> bool {
    Command::new("magick")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Q-MAGICK ImageMagick Plugin
pub struct QMagickPlugin {
    pub state: QMagickState,
}

impl Default for QMagickPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl QMagickPlugin {
    pub fn new() -> Self {
        Self {
            state: QMagickState::default(),
        }
    }

    fn draw_select_op_view(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let view = FullScreenView::new(area, " Q-MAGICK ", colors);
        view.render_frame(frame);
        let content = view.content_area();

        let state = &self.state;

        if !state.magick_available {
            let error_lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    " ImageMagick is not installed",
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
                    "   brew install imagemagick",
                    Style::default().fg(colors.green()),
                )),
                Line::from(Span::styled(
                    "   apt install imagemagick",
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
            " No image selected".to_string()
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
            ImageOp::Resize => "Scale image to different dimensions",
            ImageOp::Convert => "Convert to another image format",
            ImageOp::Rotate => "Rotate image by 90/180/270 degrees",
            ImageOp::Flip => "Flip image horizontally or vertically",
            ImageOp::Grayscale => "Convert to grayscale (black & white)",
            ImageOp::Blur => "Apply gaussian blur filter",
            ImageOp::Sharpen => "Sharpen image edges",
            ImageOp::Negate => "Invert colors (negative)",
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
        let view = FullScreenView::new(area, " Q-MAGICK - Options ", colors);
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
            ImageOp::Resize => ("Size", state.resize_option.as_str()),
            ImageOp::Convert => ("Format", state.output_format.as_str()),
            ImageOp::Rotate => ("Angle", state.rotate_option.as_str()),
            ImageOp::Flip => ("Direction", state.flip_option.as_str()),
            _ => ("N/A", "Press Enter to apply"),
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
        let view = FullScreenView::new(area, " Q-MAGICK - Result ", colors);
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

impl Plugin for QMagickPlugin {
    fn id(&self) -> &str {
        "qmagick"
    }

    fn name(&self) -> &str {
        "Q-MAGICK"
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
            id: "qmagick".to_string(),
            name: "Q-MAGICK".to_string(),
            description: "Image manipulation".to_string(),
            category: PluginCategory::Tools,
            key: 'K',
        })
    }

    fn launch(&mut self, cwd: &PathBuf, selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.state = QMagickState::new(cwd, selected_file);
        Ok(())
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let state = &mut self.state;

        match state.view {
            QMagickView::SelectOp => match key.code {
                KeyCode::Esc => return KeyHandleResult::CloseModal,
                KeyCode::Char('<') => {
                    state.operation = state.operation.prev();
                }
                KeyCode::Char('>') => {
                    state.operation = state.operation.next();
                }
                KeyCode::Enter => {
                    if state.input_file.is_some() && state.magick_available {
                        // Operations without options go straight to execute
                        if matches!(
                            state.operation,
                            ImageOp::Grayscale | ImageOp::Blur | ImageOp::Sharpen | ImageOp::Negate
                        ) {
                            state.execute_operation();
                        } else {
                            state.view = QMagickView::Options;
                        }
                    }
                }
                _ => return KeyHandleResult::NotHandled,
            },
            QMagickView::Options => match key.code {
                KeyCode::Esc => return KeyHandleResult::CloseModal,
                KeyCode::Backspace => {
                    state.view = QMagickView::SelectOp;
                }
                KeyCode::Char('<') | KeyCode::Char('>') => match state.operation {
                    ImageOp::Resize => {
                        state.resize_option = state.resize_option.next();
                    }
                    ImageOp::Convert => {
                        state.output_format = state.output_format.next();
                    }
                    ImageOp::Rotate => {
                        state.rotate_option = state.rotate_option.next();
                    }
                    ImageOp::Flip => {
                        state.flip_option = state.flip_option.next();
                    }
                    _ => {}
                },
                KeyCode::Enter => {
                    state.execute_operation();
                }
                _ => return KeyHandleResult::NotHandled,
            },
            QMagickView::Result => match key.code {
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
            QMagickView::SelectOp => self.draw_select_op_view(frame, area, colors),
            QMagickView::Options => self.draw_options_view(frame, area, colors),
            QMagickView::Result => self.draw_result_view(frame, area, colors),
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-MAGICK - Image Manipulation".to_string(),
            "".to_string(),
            "Transform images using ImageMagick.".to_string(),
            "Requires ImageMagick to be installed.".to_string(),
            "".to_string(),
            "Operations:".to_string(),
            "  Resize     - Scale to different dimensions".to_string(),
            "  Convert    - Change format (PNG, JPEG, etc.)".to_string(),
            "  Rotate     - Rotate by 90/180/270 degrees".to_string(),
            "  Flip       - Mirror horizontally/vertically".to_string(),
            "  Grayscale  - Convert to black & white".to_string(),
            "  Blur       - Apply gaussian blur".to_string(),
            "  Sharpen    - Enhance edges".to_string(),
            "  Negate     - Invert colors".to_string(),
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
    PluginRegistration::new("qmagick", || Box::new(QMagickPlugin::new()))
}
