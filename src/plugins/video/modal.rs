//! Video Player plugin modal rendering
//!
//! UI for the video player plugin.

use super::state::{PlayState, RenderMode, VideoFrame, VideoState, VideoView};
use crate::app::ThemeColors;
use crate::ui::components::{FullScreenView, ModalFrame};
use image::{ImageBuffer, Rgb};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;
use std::sync::{Mutex, OnceLock};

// Lazy-loaded image picker (detects Kitty/Sixel/iTerm2 protocols)
static IMAGE_PICKER: OnceLock<Mutex<Option<Picker>>> = OnceLock::new();

/// Get or initialize the image picker with terminal protocol detection
fn get_image_picker() -> &'static Mutex<Option<Picker>> {
    IMAGE_PICKER.get_or_init(|| {
        let picker = Picker::from_query_stdio().ok();
        Mutex::new(picker)
    })
}

/// Create an image protocol from raw RGB frame data
fn frame_to_image_protocol(video_frame: &VideoFrame) -> Option<StatefulProtocol> {
    // Convert raw RGB bytes to image::DynamicImage
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_raw(
        video_frame.width,
        video_frame.height,
        video_frame.data.clone(),
    )?;

    let dyn_img = image::DynamicImage::ImageRgb8(img);

    // Create protocol from picker
    if let Ok(mut guard) = get_image_picker().lock() {
        if let Some(ref mut picker) = *guard {
            return Some(picker.new_resize_protocol(dyn_img));
        }
    }
    None
}

/// Calculate a centered modal area from the full screen area
fn centered_modal_area(area: Rect) -> Rect {
    let width = area.width.min(55);
    let height = area.height.min(14);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

pub fn draw_video_modal(frame: &mut Frame, area: Rect, state: &VideoState, colors: &ThemeColors) {
    match state.view {
        VideoView::Menu => draw_menu(frame, area, state, colors),
        VideoView::Playing => draw_playing(frame, area, state, colors),
        VideoView::InlinePlayer => draw_inline_player(frame, area, state, colors),
        VideoView::FfmpegMissing => draw_ffmpeg_missing(frame, area, state, colors),
        VideoView::Error => draw_error(frame, area, state, colors),
    }
}

fn draw_menu(frame: &mut Frame, area: Rect, _state: &VideoState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " Video Player ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    // Simple message - this view is only shown when no file is selected
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "No video file selected",
            Style::default().fg(colors.yellow()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Select a video file to play",
            Style::default().fg(colors.grey()),
        )),
    ];

    let para = Paragraph::new(text);
    frame.render_widget(para, content_area);

    modal.render_help(frame, vec![("Esc", "close")]);
}

fn draw_playing(frame: &mut Frame, area: Rect, state: &VideoState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " Video - Playing ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    let player_name = state
        .selected()
        .map(|p| p.name())
        .unwrap_or("Unknown player");

    // File position indicator
    let position_str = state.file_position();

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("Playing: {}", state.file_name),
                Style::default().fg(colors.cyan()),
            ),
            if !position_str.is_empty() {
                Span::styled(format!("  [{}]", position_str), Style::default().fg(colors.grey()))
            } else {
                Span::raw("")
            },
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("Using: {}", player_name),
            Style::default().fg(colors.grey()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Video will open in external player",
            Style::default().fg(colors.yellow()),
        )),
    ];

    let para = Paragraph::new(text);
    frame.render_widget(para, content_area);

    // Build help with prev/next if available
    let mut help = vec![];
    if state.has_prev() || state.has_next() {
        help.push(("[/]", "prev/next"));
    }
    help.push(("Esc", "close"));
    modal.render_help(frame, help);
}

fn draw_error(frame: &mut Frame, area: Rect, state: &VideoState, colors: &ThemeColors) {
    // Use wider modal for error messages
    let width = area.width.min(60);
    let height = area.height.min(14);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    let modal = ModalFrame::themed(modal_area, " Video - Error ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    let error_msg = state.error.as_deref().unwrap_or("Unknown error");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Error playing video:",
            Style::default().fg(colors.red()),
        )),
        Line::from(""),
        Line::from(Span::styled(error_msg, Style::default().fg(colors.fg()))),
    ];

    let para = Paragraph::new(text).wrap(Wrap { trim: true });
    frame.render_widget(para, content_area);

    modal.render_help(frame, vec![("Enter/Esc", "back")]);
}

/// Draw inline video player view with image protocol or ASCII art
fn draw_inline_player(frame: &mut Frame, area: Rect, state: &VideoState, colors: &ThemeColors) {
    // Use FullScreenView for video playback (not ModalFrame - that's for small dialogs)
    let play_symbol = match state.inline_state.play_state {
        PlayState::Playing => "▶",
        PlayState::Paused => "⏸",
        PlayState::Stopped => "⏹",
    };

    let mode_str = match state.inline_state.render_mode {
        RenderMode::Image => "IMG",
        RenderMode::Ascii => "ASCII",
    };

    // Add frame counter and position to title
    let position_str = if state.inline_state.position > 0.0 {
        format!(
            " {:.1}s - Frame {} ",
            state.inline_state.position,
            state.inline_state.current_frame
        )
    } else {
        String::new()
    };

    let title = format!(
        " {} [{}] [{}]{} ",
        state.file_name, play_symbol, mode_str, position_str
    );

    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);
    let content_area = view.content_area();

    // Render video frame based on render mode
    if let Some(ref video_frame) = state.inline_state.current_video_frame {
        match state.inline_state.render_mode {
            RenderMode::Image => {
                // Try to use image protocol (Kitty/Sixel/iTerm2)
                if let Some(mut protocol) = frame_to_image_protocol(video_frame) {
                    let image_widget = StatefulImage::new(None);
                    frame.render_stateful_widget(image_widget, content_area, &mut protocol);
                } else {
                    // Image protocol not available, show message
                    let text = vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "Image protocol not supported by terminal",
                            Style::default().fg(colors.yellow()),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            "Press M to switch to ASCII mode",
                            Style::default().fg(colors.grey()),
                        )),
                    ];
                    let para = Paragraph::new(text);
                    frame.render_widget(para, content_area);
                }
            }
            RenderMode::Ascii => {
                render_ascii_frame(frame, content_area, video_frame, colors);
            }
        }
    } else {
        // No frame data yet
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Loading video...",
                Style::default().fg(colors.yellow()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("File: {}", state.file_name),
                Style::default().fg(colors.cyan()),
            )),
        ];
        let para = Paragraph::new(text);
        frame.render_widget(para, content_area);
    }

    // Help bar
    let mode_hint = match state.inline_state.render_mode {
        RenderMode::Image => "ascii",
        RenderMode::Ascii => "image",
    };
    let mut help = vec![("Space", "play/pause")];
    if state.has_prev() || state.has_next() {
        help.push(("[/]", "prev/next"));
    }
    help.push(("M", mode_hint));
    help.push(("Esc", "close"));
    view.render_help(frame, help);
}

/// Render video frame as colored ASCII art
fn render_ascii_frame(
    frame: &mut Frame,
    content_area: Rect,
    video_frame: &VideoFrame,
    _colors: &ThemeColors,
) {
    let ascii_lines = super::ascii::frame_to_colored_ascii(
        &video_frame.data,
        video_frame.width,
        video_frame.height,
        content_area.width.saturating_sub(2),
        content_area.height.saturating_sub(2),
    );

    let text: Vec<Line> = ascii_lines
        .iter()
        .map(|line| {
            let spans: Vec<Span> = line
                .iter()
                .map(|(ch, (r, g, b))| {
                    Span::styled(
                        ch.to_string(),
                        Style::default().fg(ratatui::style::Color::Rgb(*r, *g, *b)),
                    )
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    let para = Paragraph::new(text);
    frame.render_widget(para, content_area);
}

/// Draw FFmpeg missing dialog
fn draw_ffmpeg_missing(frame: &mut Frame, area: Rect, _state: &VideoState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " FFmpeg Required ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    let install_hint = super::ffmpeg::get_install_hint();

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "FFmpeg is required for video playback.",
            Style::default().fg(colors.fg()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "To install:",
            Style::default().fg(colors.blue()),
        )),
        Line::from(Span::styled(
            format!("  {}", install_hint),
            Style::default().fg(colors.green()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Use Open (O key) to play with system apps.",
            Style::default().fg(colors.grey()),
        )),
    ];

    let para = Paragraph::new(text);
    frame.render_widget(para, content_area);

    modal.render_help(frame, vec![("Enter/Esc", "close")]);
}
