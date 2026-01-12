//! Command Palette modal rendering

use super::state::{PaletteCategory, PaletteState};
use crate::app::ThemeColors;
use crate::ui::components::ModalFrame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;

/// Modal dimensions
const MODAL_WIDTH: u16 = 60;
const MODAL_HEIGHT: u16 = 17;

/// Draw the command palette modal
pub fn draw_palette_modal(
    frame: &mut Frame,
    area: Rect,
    state: &PaletteState,
    colors: &ThemeColors,
) {
    // Calculate centered modal area
    let width = area.width.min(MODAL_WIDTH);
    let height = area.height.min(MODAL_HEIGHT);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    let modal = ModalFrame::themed(modal_area, " Command Palette ", colors);
    modal.render_frame(frame);

    // Input row (row 0)
    let input_spans = render_input(state, colors, width as usize - 6);
    modal.render_row(frame, 0, input_spans);

    // Results list (rows 1-8)
    let visible_count = state.max_visible.min(height.saturating_sub(6) as usize);
    for (row_idx, (idx, result)) in state.visible_results().enumerate().take(visible_count) {
        let is_selected = idx == state.selected;
        let row = (row_idx + 1) as u16;

        // Selection indicator
        let indicator = if is_selected { "> " } else { "  " };

        // Category color
        let cat_color = match result.category {
            PaletteCategory::Calculator => colors.cyan(),
            PaletteCategory::Commands => colors.yellow(),
            PaletteCategory::Apps => colors.green(),
            PaletteCategory::Files => colors.grey(),
        };

        // Build row spans
        let label_style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg())
        };

        let desc_style = Style::default().fg(colors.grey());
        let cat_style = Style::default().fg(cat_color);

        // Calculate available widths
        let avail_width = (width as usize).saturating_sub(10); // borders + padding + indicator
        let cat_label = result.category.label();
        let cat_width = cat_label.len() + 2; // space before cat
        let label_max = avail_width.saturating_sub(cat_width);

        // Truncate label if needed
        let label = if result.label.len() > label_max {
            format!("{}...", &result.label[..label_max.saturating_sub(3)])
        } else {
            result.label.clone()
        };

        // Calculate padding
        let pad = avail_width.saturating_sub(label.len() + cat_width);

        let spans = vec![
            Span::styled(indicator, label_style),
            Span::styled(label, label_style),
            Span::styled(" ".repeat(pad), desc_style),
            Span::styled(format!(" {}", cat_label), cat_style),
        ];

        modal.render_row(frame, row, spans);
    }

    // Fill remaining rows if needed
    for row_idx in state.results.len().min(visible_count)..visible_count {
        let row = (row_idx + 1) as u16;
        modal.render_row(frame, row, vec![]);
    }

    // Help row
    modal.render_help(
        frame,
        vec![
            ("Enter", "select"),
            ("Esc", "close"),
            ("\u{2191}\u{2193}", "navigate"),
        ],
    );
}

/// Render the input field with cursor
fn render_input(
    state: &PaletteState,
    colors: &ThemeColors,
    max_width: usize,
) -> Vec<Span<'static>> {
    let prompt_style = Style::default().fg(colors.green());
    let input_style = Style::default().fg(colors.fg());
    let cursor_style = Style::default()
        .fg(colors.bg())
        .bg(colors.fg())
        .add_modifier(Modifier::BOLD);

    let mut spans = vec![Span::styled("> ", prompt_style)];

    if state.input.is_empty() {
        // Show placeholder
        let placeholder = "Type to search...";
        spans.push(Span::styled(
            placeholder,
            Style::default().fg(colors.grey()),
        ));
        // Cursor at start
        spans.insert(1, Span::styled(" ", cursor_style));
    } else {
        // Show input with cursor
        let input = &state.input;
        let cursor = state.cursor;

        // Text before cursor
        if cursor > 0 {
            let before: String = input.chars().take(cursor).collect();
            spans.push(Span::styled(before, input_style));
        }

        // Cursor character (or space if at end)
        let cursor_char: String = input
            .chars()
            .nth(cursor)
            .map(|c| c.to_string())
            .unwrap_or(" ".to_string());
        spans.push(Span::styled(cursor_char, cursor_style));

        // Text after cursor
        if cursor < input.len() {
            let after: String = input.chars().skip(cursor + 1).collect();
            if !after.is_empty() {
                spans.push(Span::styled(after, input_style));
            }
        }
    }

    // Truncate if too long
    let total_width: usize = spans.iter().map(|s| s.width()).sum();
    if total_width > max_width {
        // Just show "..." - this is a simplification
        vec![
            Span::styled("> ", prompt_style),
            Span::styled("...", input_style),
        ]
    } else {
        spans
    }
}
