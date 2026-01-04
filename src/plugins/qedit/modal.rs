//! Q-EDIT modal rendering

use super::state::{DisplayMode, EditorMode, QEditMenuItem, QEditState};
use crate::app::ThemeColors;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

/// Draw the Q-EDIT modal
pub fn draw_qedit_modal(frame: &mut Frame, area: Rect, state: &QEditState, colors: &ThemeColors) {
    // Clear the area
    frame.render_widget(Clear, area);

    // Layout: menu bar, separator, content, separator, status
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Menu bar
            Constraint::Length(1), // Separator
            Constraint::Min(5),    // Editor content
            Constraint::Length(1), // Separator
            Constraint::Length(1), // Status bar
        ])
        .split(area);

    // Draw menu bar
    draw_menu_bar(frame, chunks[0], state, colors);

    // Draw separator
    let sep = "═".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(colors.fg()))),
        chunks[1],
    );

    // Draw editor content
    draw_editor_content(frame, chunks[2], state, colors);

    // Draw bottom separator
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(colors.fg()))),
        chunks[3],
    );

    // Draw status bar
    draw_status_bar(frame, chunks[4], state, colors);
}

/// Draw the menu bar
fn draw_menu_bar(frame: &mut Frame, area: Rect, state: &QEditState, colors: &ThemeColors) {
    let menu_items = QEditMenuItem::all();
    let mut spans = Vec::new();

    for (i, item) in menu_items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default().fg(colors.fg())));
        }

        let style = if state.mode == EditorMode::Command && i == state.menu_index {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.blue())
        };

        spans.push(Span::styled(item.name(), style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Draw the editor content area
fn draw_editor_content(frame: &mut Frame, area: Rect, state: &QEditState, colors: &ThemeColors) {
    let visible_lines = area.height as usize;
    let visible_cols = area.width as usize;

    match state.display_mode {
        DisplayMode::Ascii => {
            draw_ascii_content(frame, area, state, colors, visible_lines, visible_cols)
        }
        DisplayMode::Hex => draw_hex_content(frame, area, state, colors, visible_lines),
    }
}

/// Draw ASCII mode content
fn draw_ascii_content(
    frame: &mut Frame,
    area: Rect,
    state: &QEditState,
    colors: &ThemeColors,
    visible_lines: usize,
    visible_cols: usize,
) {
    let mut lines = Vec::new();

    for i in 0..visible_lines {
        let line_idx = state.scroll_offset + i;
        if line_idx >= state.lines.len() {
            // Empty line
            lines.push(Line::from(Span::styled(
                "~",
                Style::default().fg(colors.grey()),
            )));
        } else {
            let line_text = &state.lines[line_idx];
            let display_text: String = line_text
                .chars()
                .skip(state.h_scroll_offset)
                .take(visible_cols)
                .collect();

            // Highlight current line
            let style = if line_idx == state.cursor_line {
                Style::default().fg(colors.fg())
            } else {
                Style::default().fg(colors.fg())
            };

            lines.push(Line::from(Span::styled(display_text, style)));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);

    // Draw cursor (if in editing mode)
    if state.mode != EditorMode::Command {
        let cursor_y = state.cursor_line.saturating_sub(state.scroll_offset) as u16;
        let cursor_x = state.cursor_col.saturating_sub(state.h_scroll_offset) as u16;

        if cursor_y < area.height && cursor_x < area.width {
            let cursor_area = Rect::new(area.x + cursor_x, area.y + cursor_y, 1, 1);

            // Get character at cursor
            let cursor_char = state
                .lines
                .get(state.cursor_line)
                .and_then(|l| l.chars().nth(state.cursor_col))
                .unwrap_or(' ');

            let cursor_style = if state.mode == EditorMode::Insert {
                Style::default().fg(colors.bg()).bg(colors.yellow())
            } else {
                Style::default()
                    .fg(colors.bg())
                    .bg(colors.fg())
                    .add_modifier(Modifier::UNDERLINED)
            };

            frame.render_widget(
                Paragraph::new(Span::styled(cursor_char.to_string(), cursor_style)),
                cursor_area,
            );
        }
    }
}

/// Draw hex mode content
fn draw_hex_content(
    frame: &mut Frame,
    area: Rect,
    state: &QEditState,
    colors: &ThemeColors,
    visible_lines: usize,
) {
    // Convert text to bytes for hex display
    let text: String = state.lines.join("\n");
    let bytes = text.as_bytes();

    let mut lines = Vec::new();

    // Header
    lines.push(Line::from(Span::styled(
        " Offset    00 01 02 03 04 05 06 07  08 09 0A 0B 0C 0D 0E 0F   ASCII",
        Style::default().fg(colors.blue()),
    )));

    // Hex content
    let bytes_per_line = 16;
    let start_offset = state.scroll_offset * bytes_per_line;

    for i in 0..(visible_lines.saturating_sub(1)) {
        let offset = start_offset + (i * bytes_per_line);
        if offset >= bytes.len() {
            break;
        }

        let mut hex_part = format!(" {:08X}  ", offset);
        let mut ascii_part = String::new();

        for j in 0..bytes_per_line {
            let byte_offset = offset + j;
            if byte_offset < bytes.len() {
                let byte = bytes[byte_offset];
                hex_part.push_str(&format!("{:02X} ", byte));
                // ASCII representation
                if (0x20..0x7F).contains(&byte) {
                    ascii_part.push(byte as char);
                } else {
                    ascii_part.push('.');
                }
            } else {
                hex_part.push_str("   ");
                ascii_part.push(' ');
            }

            // Add space in middle
            if j == 7 {
                hex_part.push(' ');
            }
        }

        let line_text = format!("{}  {}", hex_part, ascii_part);
        lines.push(Line::from(Span::styled(
            line_text,
            Style::default().fg(colors.fg()),
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

/// Draw the status bar
fn draw_status_bar(frame: &mut Frame, area: Rect, state: &QEditState, colors: &ThemeColors) {
    let file_name = state.display_name();
    let modified = if state.modified { " [Modified]" } else { "" };
    let mode = state.mode.name();
    let display = state.display_mode.name();
    let indent = if state.auto_indent { "Indent" } else { "" };
    let line = state.cursor_line + 1;
    let col = state.cursor_col + 1;
    let bytes = state.byte_count();

    let status = format!(
        " {}{}    {}  {}  {}  Line: {}  Col: {}  ({} bytes)",
        file_name, modified, mode, display, indent, line, col, bytes
    );

    frame.render_widget(
        Paragraph::new(Span::styled(status, Style::default().fg(colors.green()))),
        area,
    );
}
