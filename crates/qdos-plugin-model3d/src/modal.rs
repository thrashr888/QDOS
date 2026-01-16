//! 3D Model Viewer modal rendering

use super::render;
use super::state::{DrawStyle, ModelState, ModelView, RenderMode};
use image::{ImageBuffer, Rgb};
use qdos_plugin_api::prelude::ThemeColors;
use qdos_plugin_api::prelude::{FullScreenView, ModalFrame};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use ratatui_image::picker::Picker;
use ratatui_image::StatefulImage;
use std::sync::{Mutex, OnceLock};

// Lazy-loaded image picker
static IMAGE_PICKER: OnceLock<Mutex<Option<Picker>>> = OnceLock::new();

fn get_image_picker() -> &'static Mutex<Option<Picker>> {
    IMAGE_PICKER.get_or_init(|| {
        let picker = Picker::from_termios().ok();
        Mutex::new(picker)
    })
}

fn centered_modal_area(area: Rect) -> Rect {
    let width = area.width.min(55);
    let height = area.height.min(14);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

pub fn draw_model_modal(frame: &mut Frame, area: Rect, state: &ModelState, colors: &ThemeColors) {
    match state.view {
        ModelView::Viewer => draw_viewer(frame, area, state, colors),
        ModelView::Error => draw_error(frame, area, state, colors),
    }
}

fn draw_viewer(frame: &mut Frame, area: Rect, state: &ModelState, colors: &ThemeColors) {
    let mode_str = match state.render_mode {
        RenderMode::Ascii => "ASCII",
        RenderMode::Image => "IMG",
    };

    let style_str = match state.draw_style {
        DrawStyle::Wireframe => "WIRE",
        DrawStyle::Filled => "FILL",
    };

    let rotate_str = if state.auto_rotate { "AUTO" } else { "MANUAL" };

    let position_str = state.file_position();
    let position_display = if position_str.is_empty() {
        String::new()
    } else {
        format!(" [{}]", position_str)
    };

    let title = format!(
        " {} [{}] [{}] [{}]{} ",
        state.file_name, mode_str, style_str, rotate_str, position_display
    );

    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);
    let content_area = view.content_area();

    if let Some(ref model) = state.model {
        match state.render_mode {
            RenderMode::Ascii => {
                let lines = render::render_ascii_with_style(
                    model,
                    &state.camera,
                    content_area.width,
                    content_area.height,
                    state.draw_style,
                );

                let text: Vec<Line> = lines
                    .iter()
                    .map(|line| {
                        Line::from(Span::styled(
                            line.clone(),
                            Style::default().fg(colors.green()),
                        ))
                    })
                    .collect();

                let para = Paragraph::new(text);
                frame.render_widget(para, content_area);
            }
            RenderMode::Image => {
                // Render to image buffer
                let img_width = (content_area.width as u32 * 8).min(640);
                let img_height = (content_area.height as u32 * 16).min(480);

                let rgb_data = render::render_image_with_style(
                    model,
                    &state.camera,
                    img_width,
                    img_height,
                    state.draw_style,
                );

                // Convert to image and display
                if let Some(img) =
                    ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(img_width, img_height, rgb_data)
                {
                    let dyn_img = image::DynamicImage::ImageRgb8(img);

                    if let Ok(mut guard) = get_image_picker().lock() {
                        if let Some(ref mut picker) = *guard {
                            let mut protocol = picker.new_resize_protocol(dyn_img);
                            let image_widget = StatefulImage::new(None);
                            frame.render_stateful_widget(image_widget, content_area, &mut protocol);
                        } else {
                            draw_no_image_support(frame, content_area, colors);
                        }
                    }
                }
            }
        }
    } else {
        // No model loaded
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No model loaded",
                Style::default().fg(colors.yellow()),
            )),
        ];
        let para = Paragraph::new(text);
        frame.render_widget(para, content_area);
    }

    // Help bar
    let mode_hint = match state.render_mode {
        RenderMode::Ascii => "image",
        RenderMode::Image => "ascii",
    };

    let style_hint = match state.draw_style {
        DrawStyle::Wireframe => "filled",
        DrawStyle::Filled => "wire",
    };

    let mut help = vec![("Arrows", "rotate"), ("R", "auto-rotate"), ("+/-", "zoom")];
    if state.has_prev() || state.has_next() {
        help.push(("[/]", "prev/next"));
    }
    help.push(("F", style_hint));
    help.push(("M", mode_hint));
    help.push(("Esc", "close"));
    view.render_help(frame, help);
}

fn draw_no_image_support(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Image protocol not supported",
            Style::default().fg(colors.yellow()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press M for ASCII mode",
            Style::default().fg(colors.grey()),
        )),
    ];
    let para = Paragraph::new(text);
    frame.render_widget(para, area);
}

fn draw_error(frame: &mut Frame, area: Rect, state: &ModelState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " Model - Error ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    let error_msg = state.error.as_deref().unwrap_or("Unknown error");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Error loading model:",
            Style::default().fg(colors.red()),
        )),
        Line::from(""),
        Line::from(Span::styled(error_msg, Style::default().fg(colors.fg()))),
    ];

    let para = Paragraph::new(text);
    frame.render_widget(para, content_area);

    modal.render_help(frame, vec![("Enter/Esc", "close")]);
}
