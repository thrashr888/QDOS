//! Q-SHEET Modal Rendering
//!
//! Renders the spreadsheet grid using FullScreenView.

use super::state::{
    FileMenuItem, MenuCategory, SheetMode, SheetState, DEFAULT_COL_WIDTH, MAX_COLS, ROW_NUM_WIDTH,
};
use crate::app::ThemeColors;
use crate::plugins::office::shared::OfficeDocument;
use crate::ui::components::{FullScreenView, ModalFrame};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

// =============================================================================
// MAIN DRAW FUNCTION
// =============================================================================

pub fn draw_sheet_modal(frame: &mut Frame, area: Rect, state: &SheetState, colors: &ThemeColors) {
    // Handle Save As dialog separately
    if state.mode == SheetMode::SaveAs {
        draw_save_as_dialog(frame, area, state, colors);
        return;
    }

    // Calculate title with modified indicator
    let modified_marker = if state.modified { " [*]" } else { "" };
    let title = format!(" Q-SHEET: {}{} ", state.display_name(), modified_marker);

    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    let content = view.content_area();

    // Reserve rows for menu bar if active
    let menu_rows = if state.mode == SheetMode::Menu { 2 } else { 0 };

    // Calculate visible rows and columns
    let visible_rows = content.height.saturating_sub(3 + menu_rows) as usize; // Header + status + help + menu
    let available_width = content.width as usize;

    // Calculate visible columns based on available width
    let visible_cols = calculate_visible_cols(state, available_width);

    // Draw menu bar if active (rows 0-1)
    if state.mode == SheetMode::Menu {
        draw_menu_bar(frame, &view, state, colors);
    }

    // Row 0 (or 2 if menu): Column headers
    draw_column_headers(frame, &view, state, visible_cols, colors, menu_rows);

    // Rows 1-N: Grid content
    draw_grid_content(
        frame,
        &view,
        state,
        visible_rows,
        visible_cols,
        colors,
        menu_rows,
    );

    // Status bar
    draw_status_bar(frame, &view, state, visible_rows, colors, menu_rows);

    // Help row
    let help = match state.mode {
        SheetMode::Navigate => vec![
            ("/", "menu"),
            ("Arrows", "move"),
            ("F2", "edit"),
            ("Ctrl+S", "save"),
            ("Esc", "close"),
        ],
        SheetMode::Edit => vec![("Enter", "confirm"), ("Tab", "next"), ("Esc", "cancel")],
        SheetMode::Menu => vec![
            ("←→", "category"),
            ("↑↓", "item"),
            ("Letter", "select"),
            ("Enter", "execute"),
            ("Esc", "cancel"),
        ],
        SheetMode::SaveAs => vec![("Enter", "save"), ("Esc", "cancel")],
    };
    view.render_help(frame, help);
}

// =============================================================================
// COLUMN HEADERS
// =============================================================================

fn draw_column_headers(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &SheetState,
    visible_cols: usize,
    colors: &ThemeColors,
    row_offset: u16,
) {
    let header_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    let mut spans = Vec::new();

    // Row number column header (empty)
    spans.push(Span::styled(
        format!("{:>width$}|", "", width = ROW_NUM_WIDTH - 1),
        header_style,
    ));

    // Column headers (A, B, C, ...)
    for i in 0..visible_cols {
        let col = state.scroll_col + i;
        if col >= MAX_COLS {
            break;
        }

        let col_letter = SheetState::col_to_letter(col);
        let width = state.col_widths[col];
        spans.push(Span::styled(
            format!("{:^width$}|", col_letter, width = width),
            header_style,
        ));
    }

    view.render_row(frame, row_offset, spans);

    // Separator line
    let sep_style = Style::default().fg(colors.grey());
    let mut sep = String::new();
    sep.push_str(&format!("{:->width$}+", "", width = ROW_NUM_WIDTH - 1));
    for i in 0..visible_cols {
        let col = state.scroll_col + i;
        if col >= MAX_COLS {
            break;
        }
        let width = state.col_widths[col];
        sep.push_str(&format!("{:->width$}+", "", width = width));
    }
    view.render_row(frame, row_offset + 1, vec![Span::styled(sep, sep_style)]);
}

// =============================================================================
// GRID CONTENT
// =============================================================================

fn draw_grid_content(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &SheetState,
    visible_rows: usize,
    visible_cols: usize,
    colors: &ThemeColors,
    row_offset: u16,
) {
    let normal_style = Style::default().fg(colors.fg());
    let row_num_style = Style::default().fg(colors.grey());
    let cursor_style = Style::default()
        .fg(colors.yellow())
        .bg(colors.red())
        .add_modifier(Modifier::BOLD);
    let edit_style = Style::default()
        .fg(colors.fg())
        .bg(colors.blue())
        .add_modifier(Modifier::BOLD);

    for i in 0..visible_rows {
        let row = state.scroll_row + i;
        if row >= state.row_count {
            break;
        }

        let mut spans = Vec::new();

        // Row number
        spans.push(Span::styled(
            format!("{:>width$}|", row + 1, width = ROW_NUM_WIDTH - 1),
            row_num_style,
        ));

        // Cell values
        for j in 0..visible_cols {
            let col = state.scroll_col + j;
            if col >= MAX_COLS {
                break;
            }

            let is_cursor = col == state.cursor_col && row == state.cursor_row;
            let width = state.col_widths[col];

            let (content, style) = if is_cursor {
                match state.mode {
                    SheetMode::Edit => {
                        // Show edit buffer with cursor
                        let mut display = state.edit_buffer.clone();
                        if display.len() > width - 1 {
                            display = display[..width - 1].to_string();
                        }
                        (format!("{:<width$}", display, width = width), edit_style)
                    }
                    SheetMode::Navigate | SheetMode::Menu => {
                        let display = state.get_cell_display(col, row);
                        let truncated = truncate_cell(&display, width);
                        (
                            format!("{:<width$}", truncated, width = width),
                            cursor_style,
                        )
                    }
                    SheetMode::SaveAs => {
                        // Shouldn't reach here, but handle it
                        let display = state.get_cell_display(col, row);
                        let truncated = truncate_cell(&display, width);
                        (
                            format!("{:<width$}", truncated, width = width),
                            normal_style,
                        )
                    }
                }
            } else {
                let display = state.get_cell_display(col, row);
                let truncated = truncate_cell(&display, width);
                (
                    format!("{:<width$}", truncated, width = width),
                    normal_style,
                )
            };

            spans.push(Span::styled(content, style));
            spans.push(Span::styled("|", row_num_style));
        }

        view.render_row(frame, row_offset + 2 + i as u16, spans);
    }
}

// =============================================================================
// STATUS BAR
// =============================================================================

fn draw_status_bar(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &SheetState,
    visible_rows: usize,
    colors: &ThemeColors,
    row_offset: u16,
) {
    let status_style = Style::default().fg(colors.cyan());
    let formula_style = Style::default()
        .fg(colors.green())
        .add_modifier(Modifier::BOLD);
    let indicator_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    // Lotus 1-2-3 style status indicator
    let mode_indicator = match state.mode {
        SheetMode::Navigate => "READY",
        SheetMode::Edit => "EDIT",
        SheetMode::Menu => "MENU",
        SheetMode::SaveAs => "INPUT",
    };

    // Show status message if present, otherwise show cell info
    let info_content = if let Some((msg, _)) = &state.status_message {
        Span::styled(format!(" {}", msg), formula_style)
    } else {
        let cell_addr = state.cell_address();
        let cell_value = state.get_cell(state.cursor_col, state.cursor_row);

        let formula_str = cell_value
            .formula_text()
            .map(|f| format!("  Formula: {}", f))
            .unwrap_or_default();

        let value_str = format!("  Value: {}", cell_value.display());

        Span::styled(
            format!(" Cell: {}{}{}", cell_addr, formula_str, value_str),
            status_style,
        )
    };

    view.render_row(
        frame,
        row_offset + 2 + visible_rows as u16,
        vec![
            Span::styled(format!(" {:6} ", mode_indicator), indicator_style),
            info_content,
            Span::styled(
                format!(
                    "{}Rows: {}  Cols: {}",
                    " ".repeat(10),
                    state.row_count,
                    MAX_COLS
                ),
                formula_style,
            ),
        ],
    );
}

// =============================================================================
// HELPERS
// =============================================================================

/// Calculate how many columns fit in the available width
fn calculate_visible_cols(state: &SheetState, available_width: usize) -> usize {
    let mut width_used = ROW_NUM_WIDTH;
    let mut count = 0;

    for i in 0..MAX_COLS {
        let col = state.scroll_col + i;
        if col >= MAX_COLS {
            break;
        }

        let col_width = state.col_widths[col] + 1; // +1 for separator
        if width_used + col_width > available_width {
            break;
        }

        width_used += col_width;
        count += 1;
    }

    count.max(1) // At least one column
}

/// Truncate cell content to fit width
fn truncate_cell(content: &str, width: usize) -> String {
    if content.len() <= width {
        content.to_string()
    } else if width > 2 {
        format!("{}...", &content[..width - 3])
    } else {
        content[..width].to_string()
    }
}

// Silence unused warning for DEFAULT_COL_WIDTH
const _: usize = DEFAULT_COL_WIDTH;

// =============================================================================
// LOTUS 1-2-3 STYLE MENU BAR
// =============================================================================

fn draw_menu_bar(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &SheetState,
    colors: &ThemeColors,
) {
    let normal = Style::default().fg(colors.fg());
    let selected = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let hint = Style::default().fg(colors.grey());

    // Row 0: Category bar (Worksheet, Range, Copy, Move, File, etc.)
    let mut spans = Vec::new();
    spans.push(Span::styled(" ", normal));

    for (i, cat) in MenuCategory::all().iter().enumerate() {
        let is_selected = i == state.menu_category;
        let style = if is_selected { selected } else { normal };

        // Highlight first letter
        let name = cat.name();
        let first = &name[0..1];
        let rest = &name[1..];

        spans.push(Span::styled(first, Style::default().fg(colors.cyan())));
        spans.push(Span::styled(rest, style));
        spans.push(Span::styled("  ", normal));
    }

    view.render_row(frame, 0, spans);

    // Row 1: Submenu items or description for selected category
    let current_cat = MenuCategory::from_index(state.menu_category);
    match current_cat {
        MenuCategory::File => {
            let mut submenu_spans = Vec::new();
            submenu_spans.push(Span::styled(" ", normal));

            for (i, item) in FileMenuItem::all().iter().enumerate() {
                let is_selected = i == state.menu_item;
                let style = if is_selected { selected } else { normal };

                let name = item.name();
                let first = &name[0..1];
                let rest = &name[1..];

                submenu_spans.push(Span::styled(first, Style::default().fg(colors.cyan())));
                submenu_spans.push(Span::styled(rest, style));
                submenu_spans.push(Span::styled("  ", normal));
            }

            view.render_row(frame, 1, submenu_spans);
        }
        MenuCategory::Quit => {
            view.render_row(
                frame,
                1,
                vec![Span::styled(" Press Enter to close spreadsheet", hint)],
            );
        }
        _ => {
            // Show description for each unimplemented menu
            let desc = match current_cat {
                MenuCategory::Worksheet => "Global settings, Insert/Delete rows/columns",
                MenuCategory::Range => "Format cells, Named ranges, Protection",
                MenuCategory::Copy => "Copy cells to another location",
                MenuCategory::Move => "Move cells to another location",
                MenuCategory::Print => "Print worksheet to printer or file",
                MenuCategory::Graph => "Create charts and graphs",
                MenuCategory::Data => "Sort, Fill, Query, What-if tables",
                MenuCategory::Tools => "Macros, Spell check, Analysis tools",
                _ => "",
            };
            view.render_row(
                frame,
                1,
                vec![
                    Span::styled(format!(" {} ", desc), hint),
                    Span::styled("(Coming soon)", Style::default().fg(colors.grey())),
                ],
            );
        }
    }
}

// =============================================================================
// SAVE AS DIALOG
// =============================================================================

fn draw_save_as_dialog(frame: &mut Frame, area: Rect, state: &SheetState, colors: &ThemeColors) {
    // Calculate centered modal area
    let width = area.width.min(60);
    let height = 12;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    let modal = ModalFrame::themed(modal_area, " SAVE AS ", colors);
    modal.render_frame(frame);

    let grey = Style::default().fg(colors.grey());
    let normal = Style::default().fg(colors.fg());
    let label = Style::default().fg(colors.green());
    let input_style = Style::default().fg(colors.yellow()).bg(colors.red());

    // Current file info
    let current_name = state.display_name();
    modal.render_row(
        frame,
        0,
        vec![
            Span::styled("Current: ", grey),
            Span::styled(current_name, normal),
        ],
    );

    // Filename input row
    let input_display = format!("{}█", state.save_as_input);
    modal.render_row(
        frame,
        2,
        vec![
            Span::styled("Filename: ", label),
            Span::styled(input_display, input_style),
        ],
    );

    // Extension hint
    modal.render_row(
        frame,
        4,
        vec![Span::styled(
            "(Extension: .csv for CSV, .xlsx for Excel)",
            grey,
        )],
    );

    // Help footer
    modal.render_help(frame, vec![("Enter", "save"), ("Esc", "cancel")]);
}
