//! Q-PAINT UI Rendering
//!
//! Terminal UI for the pixel art editor using ratatui with sixel graphics support.

use crate::state::{QPaintState, QPaintView, Tool, DOS_PALETTE};
use image::{ImageBuffer, Rgb, RgbImage};
use qdos_plugin_api::ThemeColors;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    Frame,
};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;
use std::sync::{Mutex, OnceLock};

// Lazy-loaded image picker (detects Kitty/Sixel/iTerm2 protocols)
static IMAGE_PICKER: OnceLock<Mutex<Picker>> = OnceLock::new();

/// Check if sixel graphics are supported
#[allow(dead_code)]
fn is_sixel_supported() -> bool {
    get_image_picker().lock().is_ok()
}

/// Get or initialize the image picker with terminal protocol detection
fn get_image_picker() -> &'static Mutex<Picker> {
    IMAGE_PICKER.get_or_init(|| {
        let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::from_fontsize((8, 16)));
        Mutex::new(picker)
    })
}

/// Convert canvas pixels to an image and create a rendering protocol
fn canvas_to_protocol(state: &QPaintState) -> Option<StatefulProtocol> {
    let canvas = &state.canvas;
    let zoom = state.zoom as u32;

    // Create image from canvas
    let img: RgbImage = ImageBuffer::from_fn(canvas.width, canvas.height, |x, y| {
        let (r, g, b) = canvas.get_pixel(x, y);
        Rgb([r, g, b])
    });

    // Apply zoom by scaling with nearest neighbor (pixelated look)
    let scaled = if zoom > 1 {
        image::imageops::resize(
            &img,
            canvas.width * zoom,
            canvas.height * zoom,
            image::imageops::FilterType::Nearest,
        )
    } else {
        img
    };

    // Convert to DynamicImage for ratatui-image
    let dyn_img = image::DynamicImage::ImageRgb8(scaled);

    // Get the picker and create protocol
    if let Ok(mut picker) = get_image_picker().lock() {
        Some(picker.new_resize_protocol(dyn_img))
    } else {
        None
    }
}

/// Render Q-PAINT modal
pub fn render(frame: &mut Frame, area: Rect, state: &QPaintState, colors: &ThemeColors) {
    match state.view {
        QPaintView::Editor => render_editor(frame, area, state, colors),
        QPaintView::Palette => render_palette(frame, area, state, colors),
        QPaintView::FileMenu => render_file_menu(frame, area, state, colors),
        QPaintView::Help => render_help(frame, area, state, colors),
    }
}

/// Render the main editor view
fn render_editor(frame: &mut Frame, area: Rect, state: &QPaintState, colors: &ThemeColors) {
    use ui_components::FullScreenView;

    let title = if let Some(path) = &state.file_path {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();
        format!(
            " Q-PAINT: {}{} ",
            name,
            if state.modified { "*" } else { "" }
        )
    } else {
        format!(
            " Q-PAINT: untitled{} ",
            if state.modified { "*" } else { "" }
        )
    };

    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    let content_area = view.content_area();

    // Toolbar row
    render_toolbar(
        frame,
        content_area.x,
        content_area.y,
        content_area.width,
        state,
        colors,
    );

    // Canvas area
    let canvas_y = content_area.y + 2;
    let canvas_height = content_area.height.saturating_sub(4);
    render_canvas(
        frame,
        content_area.x,
        canvas_y,
        content_area.width,
        canvas_height,
        state,
        colors,
    );

    // Status bar
    let status_y = content_area.y + content_area.height - 1;
    render_status_bar(
        frame,
        content_area.x,
        status_y,
        content_area.width,
        state,
        colors,
    );

    // Help footer
    view.render_help(
        frame,
        vec![
            ("Arrow", "move"),
            ("Space", "draw"),
            ("Tab", "palette"),
            ("Z", "zoom"),
            ("^Z", "undo"),
            ("^S", "save"),
            ("?", "help"),
            ("Esc", "exit"),
        ],
    );
}

/// Render the toolbar
fn render_toolbar(
    frame: &mut Frame,
    x: u16,
    y: u16,
    width: u16,
    state: &QPaintState,
    colors: &ThemeColors,
) {
    // In-scope tools per requirements
    let tools = Tool::all();

    let mut spans = Vec::new();

    for tool in tools {
        let is_selected = *tool == state.tool;
        let key = tool.key();
        let name = tool.name();

        if is_selected {
            spans.push(Span::styled(
                format!("[{}]{}", key, name),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!("[{}]", key),
                Style::default().fg(colors.green()),
            ));
            spans.push(Span::styled(name, Style::default().fg(colors.fg())));
        }
        spans.push(Span::raw(" "));
    }

    let line = Line::from(spans);
    let toolbar_area = Rect::new(x, y, width, 1);
    frame.render_widget(ratatui::widgets::Paragraph::new(line), toolbar_area);

    // Separator line
    let sep_area = Rect::new(x, y + 1, width, 1);
    let sep = "─".repeat(width as usize);
    frame.render_widget(
        ratatui::widgets::Paragraph::new(sep).style(Style::default().fg(colors.fg())),
        sep_area,
    );
}

/// Render the canvas using sixel graphics
fn render_canvas(
    frame: &mut Frame,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    state: &QPaintState,
    colors: &ThemeColors,
) {
    // Try sixel rendering first
    if let Some(mut protocol) = canvas_to_protocol(state) {
        let canvas_area = Rect::new(x, y, width, height.saturating_sub(1));
        let image_widget = StatefulImage::new(None);
        frame.render_stateful_widget(image_widget, canvas_area, &mut protocol);

        // Render cursor position indicator below the canvas
        let cursor_y = y + height.saturating_sub(1);
        let cursor_info = format!(
            "Cursor: ({},{}) - Use arrows to move, Space to draw",
            state.cursor_x, state.cursor_y
        );
        let cursor_line = Line::from(vec![Span::styled(
            cursor_info,
            Style::default().fg(colors.green()),
        )]);
        let cursor_area = Rect::new(x, cursor_y, width, 1);
        frame.render_widget(ratatui::widgets::Paragraph::new(cursor_line), cursor_area);

        return;
    }

    // Fallback to ASCII rendering if sixel not supported
    render_canvas_ascii(frame, x, y, width, height, state, colors);
}

/// Fallback ASCII canvas rendering (for terminals without sixel support)
fn render_canvas_ascii(
    frame: &mut Frame,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    state: &QPaintState,
    colors: &ThemeColors,
) {
    let zoom = state.zoom as u16;
    let canvas = &state.canvas;

    // Calculate visible area
    let chars_per_pixel = zoom.max(1);
    let visible_cols = width / chars_per_pixel;
    let visible_rows = height;

    for row in 0..visible_rows {
        let canvas_y = state.scroll_y + row as u32;
        if canvas_y >= canvas.height {
            continue;
        }

        let mut spans = Vec::new();

        for col in 0..visible_cols {
            let canvas_x = state.scroll_x + col as u32;
            if canvas_x >= canvas.width {
                break;
            }

            let (r, g, b) = canvas.get_pixel(canvas_x, canvas_y);
            let is_cursor = canvas_x == state.cursor_x && canvas_y == state.cursor_y;

            // Selection highlight
            let is_selected = state.selection.active && {
                let (min_x, min_y, max_x, max_y) = state.selection.bounds();
                canvas_x >= min_x && canvas_x <= max_x && canvas_y >= min_y && canvas_y <= max_y
            };

            // Convert RGB to ratatui Color
            let pixel_color = Color::Rgb(r, g, b);

            // Generate pixel character based on zoom
            let ch = if is_cursor {
                // Cursor shown with inverse colors
                let char_str = "X".repeat(chars_per_pixel as usize);
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(colors.yellow())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Black)
                        .bg(pixel_color)
                        .add_modifier(Modifier::BOLD)
                };
                Span::styled(char_str, style)
            } else {
                // Use block character with pixel color as background
                let char_str = if r == 0 && g == 0 && b == 0 {
                    ".".repeat(chars_per_pixel as usize)
                } else {
                    "#".repeat(chars_per_pixel as usize)
                };

                let style = if is_selected {
                    Style::default().fg(pixel_color).bg(colors.blue())
                } else {
                    Style::default().fg(pixel_color).bg(Color::Reset)
                };

                Span::styled(char_str, style)
            };

            spans.push(ch);
        }

        let line = Line::from(spans);
        let row_area = Rect::new(x, y + row, width, 1);
        frame.render_widget(ratatui::widgets::Paragraph::new(line), row_area);
    }
}

/// Render the status bar
fn render_status_bar(
    frame: &mut Frame,
    x: u16,
    y: u16,
    width: u16,
    state: &QPaintState,
    colors: &ThemeColors,
) {
    // Color swatches
    let (fr, fg, fb) = state.fg_color;
    let (br, bg_c, bb) = state.bg_color;

    let mut spans = vec![
        Span::styled("FG:", Style::default().fg(colors.fg())),
        Span::styled("##", Style::default().fg(Color::Rgb(fr, fg, fb))),
        Span::raw(" "),
        Span::styled("BG:", Style::default().fg(colors.fg())),
        Span::styled("..", Style::default().fg(Color::Rgb(br, bg_c, bb))),
        Span::raw("  "),
        Span::styled(
            format!("Tool:{}", state.tool.name()),
            Style::default().fg(colors.fg()),
        ),
        Span::raw("  "),
        Span::styled(
            format!("Size:{}", state.brush_size),
            Style::default().fg(colors.fg()),
        ),
        Span::raw("  "),
        Span::styled(
            format!("Zoom:{}x", state.zoom),
            Style::default().fg(colors.fg()),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{}x{}", state.canvas.width, state.canvas.height),
            Style::default().fg(colors.fg()),
        ),
        Span::raw("  "),
        Span::styled(
            format!("({},{})", state.cursor_x, state.cursor_y),
            Style::default().fg(colors.green()),
        ),
    ];

    if !state.status.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            &state.status,
            Style::default().fg(colors.yellow()),
        ));
    }

    let line = Line::from(spans);
    let status_area = Rect::new(x, y, width, 1);
    frame.render_widget(ratatui::widgets::Paragraph::new(line), status_area);
}

/// Render the palette view
fn render_palette(frame: &mut Frame, area: Rect, state: &QPaintState, colors: &ThemeColors) {
    use ui_components::FullScreenView;

    let view = FullScreenView::new(area, " Q-PAINT: Palette ", colors);
    view.render_frame(frame);

    let _content = view.content_area();

    // Title
    view.render_row(
        frame,
        0,
        vec![Span::styled(
            "Standard DOS Palette:",
            Style::default().fg(colors.fg()),
        )],
    );

    // First row of colors (0-7)
    let mut row1_spans = vec![Span::raw(" ")];
    for (i, &(r, g, b)) in DOS_PALETTE.iter().enumerate().take(8) {
        let is_selected = i == state.palette_index;
        let prefix = if is_selected { "[" } else { " " };
        let suffix = if is_selected { "]" } else { " " };

        row1_spans.push(Span::styled(
            format!("{}{:X}", prefix, i),
            Style::default().fg(colors.fg()),
        ));
        row1_spans.push(Span::styled("##", Style::default().fg(Color::Rgb(r, g, b))));
        row1_spans.push(Span::styled(suffix, Style::default().fg(colors.fg())));
        row1_spans.push(Span::raw(" "));
    }
    view.render_row(frame, 2, row1_spans);

    // Second row of colors (8-F)
    let mut row2_spans = vec![Span::raw(" ")];
    for (i, &(r, g, b)) in DOS_PALETTE.iter().enumerate().skip(8) {
        let is_selected = i == state.palette_index;
        let prefix = if is_selected { "[" } else { " " };
        let suffix = if is_selected { "]" } else { " " };

        row2_spans.push(Span::styled(
            format!("{}{:X}", prefix, i),
            Style::default().fg(colors.fg()),
        ));
        row2_spans.push(Span::styled("##", Style::default().fg(Color::Rgb(r, g, b))));
        row2_spans.push(Span::styled(suffix, Style::default().fg(colors.fg())));
        row2_spans.push(Span::raw(" "));
    }
    view.render_row(frame, 3, row2_spans);

    // Current colors
    let (fr, fg_c, fb) = state.fg_color;
    let (br, bg_c, bb) = state.bg_color;

    view.render_row(frame, 5, vec![Span::raw("")]);
    view.render_row(
        frame,
        6,
        vec![
            Span::styled("Current:  FG: ", Style::default().fg(colors.fg())),
            Span::styled("##", Style::default().fg(Color::Rgb(fr, fg_c, fb))),
            Span::styled(
                format!(" R:{:3} G:{:3} B:{:3}", fr, fg_c, fb),
                Style::default().fg(colors.fg()),
            ),
        ],
    );
    view.render_row(
        frame,
        7,
        vec![
            Span::styled("          BG: ", Style::default().fg(colors.fg())),
            Span::styled("..", Style::default().fg(Color::Rgb(br, bg_c, bb))),
            Span::styled(
                format!(" R:{:3} G:{:3} B:{:3}", br, bg_c, bb),
                Style::default().fg(colors.fg()),
            ),
        ],
    );

    // Selected color name
    let color_name = match state.palette_index {
        0 => "Black",
        1 => "Blue",
        2 => "Green",
        3 => "Cyan",
        4 => "Red",
        5 => "Magenta",
        6 => "Brown",
        7 => "Light Gray",
        8 => "Dark Gray",
        9 => "Light Blue",
        10 => "Light Green",
        11 => "Light Cyan",
        12 => "Light Red",
        13 => "Light Magenta",
        14 => "Yellow",
        15 => "White",
        _ => "Custom",
    };

    view.render_row(
        frame,
        9,
        vec![Span::styled(
            format!("Selected: {:X} - {}", state.palette_index, color_name),
            Style::default().fg(colors.yellow()),
        )],
    );

    view.render_help(
        frame,
        vec![
            ("0-F", "select"),
            ("Enter", "set FG"),
            ("Shift+Enter", "set BG"),
            ("Esc", "back"),
        ],
    );
}

/// Render the file menu
fn render_file_menu(frame: &mut Frame, area: Rect, state: &QPaintState, colors: &ThemeColors) {
    use ui_components::FullScreenView;

    let title = match state.file_mode {
        crate::state::FileMode::Open => " Q-PAINT: Open File ",
        crate::state::FileMode::Save => " Q-PAINT: Save File ",
        crate::state::FileMode::New => " Q-PAINT: New Canvas ",
    };

    let view = FullScreenView::new(area, title, colors);
    view.render_frame(frame);

    if matches!(state.file_mode, crate::state::FileMode::New) {
        // New canvas dialog
        view.render_row(
            frame,
            1,
            vec![Span::styled(
                "Enter canvas dimensions:",
                Style::default().fg(colors.fg()),
            )],
        );
        view.render_row(
            frame,
            3,
            vec![
                Span::styled("Size: ", Style::default().fg(colors.fg())),
                Span::styled(
                    &state.file_input,
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "_",
                    Style::default()
                        .fg(colors.fg())
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ],
        );
        view.render_row(
            frame,
            5,
            vec![Span::styled(
                "Format: WIDTHxHEIGHT (e.g., 64x64)",
                Style::default().fg(colors.grey()),
            )],
        );

        view.render_help(frame, vec![("Enter", "create"), ("Esc", "cancel")]);
    } else {
        // File list
        view.render_row(
            frame,
            0,
            vec![Span::styled(
                "Image files:",
                Style::default().fg(colors.fg()),
            )],
        );

        let content = view.content_area();
        let list_height = content.height.saturating_sub(6) as usize;

        for (i, file) in state.file_list.iter().enumerate().take(list_height) {
            let is_selected = i == state.file_selected;
            let style = if is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(colors.fg())
            };

            view.render_row(frame, (i + 2) as u16, vec![Span::styled(file, style)]);
        }

        // Input field for save
        if matches!(state.file_mode, crate::state::FileMode::Save) {
            let input_y = content.height.saturating_sub(3);
            view.render_row(
                frame,
                input_y,
                vec![
                    Span::styled("Filename: ", Style::default().fg(colors.fg())),
                    Span::styled(
                        &state.file_input,
                        Style::default()
                            .fg(colors.yellow())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "_",
                        Style::default()
                            .fg(colors.fg())
                            .add_modifier(Modifier::SLOW_BLINK),
                    ),
                ],
            );
        }

        view.render_help(
            frame,
            vec![
                ("Up/Down", "select"),
                ("Enter", "confirm"),
                ("Esc", "cancel"),
            ],
        );
    }
}

/// Render help screen
fn render_help(frame: &mut Frame, area: Rect, _state: &QPaintState, colors: &ThemeColors) {
    use ui_components::FullScreenView;

    let view = FullScreenView::new(area, " Q-PAINT: Help ", colors);
    view.render_frame(frame);

    let help_text = [
        ("Drawing Tools:", ""),
        ("  P", "Pencil - single pixel drawing"),
        ("  B", "Brush - variable size brush"),
        ("  L", "Line - draw straight lines (bonus)"),
        ("  E", "Eraser - erase with BG color"),
        ("  I", "Color picker - pick color from canvas"),
        ("  S", "Select - rectangle selection"),
        ("  T", "Text - add text to canvas"),
        ("", ""),
        ("Navigation:", ""),
        ("  Arrows", "Move cursor"),
        ("  Space", "Draw/Apply tool"),
        ("  Shift+Arrow", "Draw while moving"),
        ("  Z/Shift+Z", "Zoom in/out (up to 16x)"),
        ("  +/-", "Brush size"),
        ("", ""),
        ("Colors:", ""),
        ("  1-9, 0", "Quick palette colors"),
        ("  Tab", "Open color palette"),
        ("", ""),
        ("File:", ""),
        ("  Ctrl+N", "New canvas"),
        ("  Ctrl+O", "Open file (PNG/GIF/BMP)"),
        ("  Ctrl+S", "Save file"),
        ("  Ctrl+Shift+S", "Save as..."),
        ("", ""),
        ("Edit:", ""),
        ("  Ctrl+Z", "Undo"),
        ("  Ctrl+Y", "Redo"),
        ("  Ctrl+C", "Copy selection"),
        ("  Ctrl+X", "Cut selection"),
        ("  Ctrl+V", "Paste"),
        ("  Delete", "Clear selection"),
        ("", ""),
        ("Requires:", "Sixel-capable terminal"),
        ("", "(Kitty, WezTerm, iTerm2, foot)"),
    ];

    for (i, (key, desc)) in help_text.iter().enumerate() {
        if i >= view.content_area().height as usize - 1 {
            break;
        }

        if key.is_empty() {
            view.render_row(frame, i as u16, vec![Span::raw("")]);
        } else if desc.is_empty() {
            view.render_row(
                frame,
                i as u16,
                vec![Span::styled(
                    *key,
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD),
                )],
            );
        } else {
            view.render_row(
                frame,
                i as u16,
                vec![
                    Span::styled(format!("{:12}", key), Style::default().fg(colors.green())),
                    Span::styled(*desc, Style::default().fg(colors.fg())),
                ],
            );
        }
    }

    view.render_help(frame, vec![("Esc", "back")]);
}

/// UI component helpers (minimal FullScreenView implementation)
mod ui_components {
    use qdos_plugin_api::ThemeColors;
    use ratatui::{
        layout::Rect,
        style::Style,
        text::{Line, Span},
        widgets::{Block, Borders, Paragraph},
        Frame,
    };

    pub struct FullScreenView<'a> {
        area: Rect,
        title: &'a str,
        colors: &'a ThemeColors,
    }

    impl<'a> FullScreenView<'a> {
        pub fn new(area: Rect, title: &'a str, colors: &'a ThemeColors) -> Self {
            Self {
                area,
                title,
                colors,
            }
        }

        pub fn render_frame(&self, frame: &mut Frame) {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(self.title)
                .border_style(Style::default().fg(self.colors.fg()))
                .title_style(Style::default().fg(self.colors.yellow()));

            frame.render_widget(block, self.area);
        }

        pub fn content_area(&self) -> Rect {
            Rect::new(
                self.area.x + 1,
                self.area.y + 1,
                self.area.width.saturating_sub(2),
                self.area.height.saturating_sub(3),
            )
        }

        pub fn render_row(&self, frame: &mut Frame, row: u16, spans: Vec<Span>) {
            let content = self.content_area();
            if row >= content.height {
                return;
            }

            let row_area = Rect::new(content.x, content.y + row, content.width, 1);
            let line = Line::from(spans);
            frame.render_widget(Paragraph::new(line), row_area);
        }

        pub fn render_help(&self, frame: &mut Frame, items: Vec<(&str, &str)>) {
            let mut spans = Vec::new();

            for (i, (key, action)) in items.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::raw("  "));
                }
                spans.push(Span::styled(*key, Style::default().fg(self.colors.green())));
                spans.push(Span::raw(":"));
                spans.push(Span::styled(*action, Style::default().fg(self.colors.fg())));
            }

            let help_area = Rect::new(
                self.area.x,
                self.area.y + self.area.height - 1,
                self.area.width,
                1,
            );
            frame.render_widget(Paragraph::new(Line::from(spans)), help_area);
        }
    }
}
