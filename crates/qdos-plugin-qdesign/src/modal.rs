//! Q-DESIGN UI rendering

use crate::state::{QDesignState, QDesignView, TextAlignment, Tool};
use qdos_plugin_api::prelude::{FullScreenView, ThemeColors};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

// =============================================================================
// TEMPLATE SELECTOR
// =============================================================================

pub fn draw_template_select(
    state: &QDesignState,
    frame: &mut Frame,
    area: Rect,
    colors: &ThemeColors,
) {
    let view = FullScreenView::new(area, " Q-DESIGN: Templates ", colors);
    view.render_frame(frame);

    let normal = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(colors.grey());

    view.render_row(
        frame,
        0,
        vec![Span::styled(
            "Choose a template to get started:",
            Style::default().fg(colors.cyan()),
        )],
    );

    view.render_row(frame, 1, vec![Span::raw("")]);

    for (i, template) in state.templates.iter().enumerate() {
        let is_selected = i == state.template_cursor;
        let style = if is_selected { highlight } else { normal };
        let marker = if is_selected { ">" } else { " " };

        let line = format!(
            " {} [{}] {:20} {}",
            marker,
            if is_selected { "x" } else { " " },
            template.name,
            template.description
        );

        view.render_row(frame, 3 + i as u16 * 2, vec![Span::styled(line, style)]);

        // Show dimensions
        let dims = format!("        Size: {}x{} chars", template.width, template.height);
        view.render_row(
            frame,
            4 + i as u16 * 2,
            vec![Span::styled(dims, desc_style)],
        );
    }

    view.render_help(
        frame,
        vec![
            ("^v", "select"),
            ("Enter", "choose"),
            ("?", "help"),
            ("Esc", "exit"),
        ],
    );
}

// =============================================================================
// CANVAS VIEW
// =============================================================================

pub fn draw_canvas(state: &QDesignState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let title = format!(
        " Q-DESIGN: {} {}",
        state.title,
        if state.modified { "*" } else { "" }
    );
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    // Toolbar
    let toolbar_style = Style::default().fg(colors.cyan());
    let tool_active = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    let tools_display: Vec<Span> = Tool::all()
        .iter()
        .map(|t| {
            let style = if *t == state.tool {
                tool_active
            } else {
                toolbar_style
            };
            Span::styled(format!(" [{}] ", t.name()), style)
        })
        .collect();
    view.render_row(frame, 0, tools_display);

    // Canvas area - calculate based on view content area
    let content = view.content_area();
    if let Some(page) = state.current_page() {
        let canvas_x = content.x;
        let canvas_y = content.y + 1; // After toolbar row
        let canvas_width = content.width.min(page.width + 2);
        let canvas_height = content.height.saturating_sub(3).min(page.height + 2);
        let canvas_area = Rect::new(canvas_x, canvas_y, canvas_width, canvas_height);

        // Draw page background
        let page_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors.grey()));
        frame.render_widget(page_block, canvas_area);

        // Draw frames
        for (i, f) in page.frames.iter().enumerate() {
            let is_selected = state.selected_frame == Some(i);
            draw_frame_on_canvas(
                frame,
                canvas_area,
                f,
                is_selected,
                colors,
                page.width,
                page.height,
            );
        }

        // Draw cursor in text frame mode
        if state.tool == Tool::TextFrame && !state.creating_frame {
            let cursor_x = canvas_x + 1 + state.cursor_x.min(canvas_width.saturating_sub(3));
            let cursor_y = canvas_y + 1 + state.cursor_y.min(canvas_height.saturating_sub(3));
            if cursor_x < canvas_x + canvas_width && cursor_y < canvas_y + canvas_height {
                let cursor = Paragraph::new("+").style(
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD),
                );
                frame.render_widget(cursor, Rect::new(cursor_x, cursor_y, 1, 1));
            }
        }
    }

    // Status bar
    let status_y = view.content_height().saturating_sub(2);
    let status = if let Some(f) = state.selected_frame_ref() {
        format!(
            "Frame: Text {}x{} at ({},{})  Align:{}  Border:{}",
            f.width,
            f.height,
            f.x,
            f.y,
            f.alignment.name(),
            if f.border { "Yes" } else { "No" }
        )
    } else if state.tool == Tool::TextFrame {
        format!(
            "Cursor: ({}, {})  Tool: {}",
            state.cursor_x,
            state.cursor_y,
            state.tool.name()
        )
    } else {
        format!("Tool: {}  Tab to select frames", state.tool.name())
    };
    view.render_row(
        frame,
        status_y,
        vec![Span::styled(status, Style::default().fg(colors.grey()))],
    );

    let help = if state.selected_frame.is_some() {
        vec![
            ("Arrows", "move"),
            ("Enter", "edit"),
            ("A", "align"),
            ("B", "border"),
            ("Del", "delete"),
            ("Esc", "deselect"),
        ]
    } else if state.tool == Tool::TextFrame {
        vec![
            ("Arrows", "cursor"),
            ("Enter", "create"),
            ("Tab", "tool"),
            ("Esc", "back"),
        ]
    } else {
        vec![
            ("Tab", "select/tool"),
            ("T", "text tool"),
            ("?", "help"),
            ("Esc", "back"),
        ]
    };
    view.render_help(frame, help);
}

fn draw_frame_on_canvas(
    frame: &mut Frame,
    canvas_area: Rect,
    f: &crate::state::Frame,
    is_selected: bool,
    colors: &ThemeColors,
    page_width: u16,
    page_height: u16,
) {
    // Calculate visible portion of frame
    let visible_width = canvas_area.width.saturating_sub(2).min(page_width);
    let visible_height = canvas_area.height.saturating_sub(2).min(page_height);

    if f.x >= visible_width || f.y >= visible_height {
        return; // Frame is outside visible area
    }

    let frame_x = canvas_area.x + 1 + f.x;
    let frame_y = canvas_area.y + 1 + f.y;
    let frame_width = f.width.min(visible_width.saturating_sub(f.x));
    let frame_height = f.height.min(visible_height.saturating_sub(f.y));

    if frame_width == 0 || frame_height == 0 {
        return;
    }

    let frame_area = Rect::new(frame_x, frame_y, frame_width, frame_height);

    // Draw border
    let border_style = if is_selected {
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD)
    } else if f.border {
        Style::default().fg(colors.cyan())
    } else {
        Style::default().fg(colors.grey())
    };

    let border = if f.border || is_selected {
        Borders::ALL
    } else {
        Borders::NONE
    };

    let block = Block::default().borders(border).border_style(border_style);
    frame.render_widget(block, frame_area);

    // Draw text inside frame
    if !f.text.is_empty() && frame_height > 2 && frame_width > 2 {
        let inner_width = frame_width.saturating_sub(2) as usize;
        let text_display: String = f.text.chars().take(inner_width).collect();

        let text_x = match f.alignment {
            TextAlignment::Left => frame_x + 1,
            TextAlignment::Center => {
                frame_x + 1 + (inner_width.saturating_sub(text_display.len()) / 2) as u16
            }
            TextAlignment::Right => {
                frame_x + 1 + (inner_width.saturating_sub(text_display.len())) as u16
            }
        };

        let text_style = if is_selected {
            Style::default().fg(colors.yellow())
        } else {
            Style::default().fg(colors.fg())
        };

        let para = Paragraph::new(text_display).style(text_style);
        let text_area = Rect::new(text_x, frame_y + 1, inner_width as u16, 1);
        frame.render_widget(para, text_area);
    }
}

// =============================================================================
// TEXT EDIT VIEW
// =============================================================================

pub fn draw_text_edit(state: &QDesignState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    // Draw canvas in background
    draw_canvas(state, frame, area, colors);

    // Draw text edit popup
    let popup_width = 50u16.min(area.width.saturating_sub(4));
    let popup_height = 8u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Edit Text ")
        .border_style(Style::default().fg(colors.cyan()))
        .title_style(
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(block, popup_area);

    // Text input area
    let input_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::UNDERLINED);
    let text_display = format!("{}|", state.text_edit_buffer);
    let para = Paragraph::new(text_display).style(input_style);
    frame.render_widget(
        para,
        Rect::new(popup_x + 2, popup_y + 2, popup_width.saturating_sub(4), 1),
    );

    // Help text
    let help_text = "Enter: confirm  Esc: cancel";
    let help_para = Paragraph::new(help_text).style(Style::default().fg(colors.green()));
    frame.render_widget(
        help_para,
        Rect::new(
            popup_x + 2,
            popup_y + popup_height - 2,
            popup_width.saturating_sub(4),
            1,
        ),
    );
}

// =============================================================================
// EXPORT VIEW
// =============================================================================

pub fn draw_export(state: &QDesignState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-DESIGN: Export ", colors);
    view.render_frame(frame);

    let label_style = Style::default().fg(colors.cyan());
    let edit_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::UNDERLINED);

    view.render_row(
        frame,
        1,
        vec![Span::styled("Export as ASCII Art", label_style)],
    );

    view.render_row(frame, 3, vec![Span::styled("Output file:", label_style)]);
    view.render_row(
        frame,
        4,
        vec![Span::styled(
            format!("  [{}|]", state.export_path),
            edit_style,
        )],
    );

    if let Some(page) = state.current_page() {
        view.render_row(
            frame,
            6,
            vec![Span::styled(
                format!("Page size: {}x{} characters", page.width, page.height),
                Style::default().fg(colors.grey()),
            )],
        );
        view.render_row(
            frame,
            7,
            vec![Span::styled(
                format!("Frames: {}", page.frames.len()),
                Style::default().fg(colors.grey()),
            )],
        );
    }

    if let Some(msg) = &state.status_message {
        let style = if msg.starts_with("Error") {
            Style::default().fg(colors.red())
        } else {
            Style::default().fg(colors.green())
        };
        view.render_row(frame, 9, vec![Span::styled(msg.clone(), style)]);
    }

    view.render_help(frame, vec![("Enter", "export"), ("Esc", "cancel")]);
}

// =============================================================================
// HELP VIEW
// =============================================================================

pub fn draw_help(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-DESIGN: Help ", colors);
    view.render_frame(frame);

    let header_style = Style::default()
        .fg(colors.cyan())
        .add_modifier(Modifier::BOLD);
    let normal = Style::default().fg(colors.fg());
    let key_style = Style::default().fg(colors.yellow());

    let help_text = [
        ("Q-DESIGN - Print Designer", header_style),
        ("", normal),
        ("Template Selection:", header_style),
        ("  Up/Down  Select template", normal),
        ("  Enter    Start with template", normal),
        ("", normal),
        ("Canvas:", header_style),
        ("  Tab      Cycle tool / Select next frame", normal),
        ("  T        Switch to text frame tool", normal),
        ("  S        Switch to select tool", normal),
        ("  Arrows   Move cursor / Move selected frame", normal),
        ("  Enter    Create frame / Edit text", normal),
        ("  Del      Delete selected frame", normal),
        ("  A        Cycle text alignment", normal),
        ("  B        Toggle border", normal),
        ("  Ctrl+E   Export", normal),
        ("  Esc      Deselect / Back", normal),
        ("", normal),
        ("Text Editing:", header_style),
        ("  Type     Enter text", normal),
        ("  Enter    Confirm", normal),
        ("  Esc      Cancel", normal),
    ];

    for (i, (text, style)) in help_text.iter().enumerate() {
        if i as u16 + 1 >= view.content_height() {
            break;
        }
        // Key binding line
        if text.contains("  ") {
            let parts: Vec<&str> = text.splitn(2, "  ").collect();
            if parts.len() == 2 {
                view.render_row(
                    frame,
                    i as u16,
                    vec![
                        Span::styled(format!("  {:10}", parts[0].trim()), key_style),
                        Span::styled(parts[1], *style),
                    ],
                );
                continue;
            }
        }
        view.render_row(
            frame,
            i as u16,
            vec![Span::styled(format!("  {}", text), *style)],
        );
    }

    view.render_help(frame, vec![("Esc", "back")]);
}

// =============================================================================
// MAIN DRAW FUNCTION
// =============================================================================

pub fn draw_qdesign(state: &QDesignState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    match state.view {
        QDesignView::TemplateSelect => draw_template_select(state, frame, area, colors),
        QDesignView::Canvas => draw_canvas(state, frame, area, colors),
        QDesignView::TextEdit => draw_text_edit(state, frame, area, colors),
        QDesignView::Export => draw_export(state, frame, area, colors),
        QDesignView::Help => draw_help(frame, area, colors),
    }
}
