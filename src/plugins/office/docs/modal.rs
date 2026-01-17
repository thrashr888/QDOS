//! Q-DOCS Modal Rendering
//!
//! Renders the word processor interface with markdown highlighting.

use super::ops;
use super::state::{DocsMode, DocsState, ExportFormat, InputMode, MenuCategory};
use crate::app::ThemeColors;
use crate::ui::components::{FullScreenView, ModalFrame};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::Paragraph,
    Frame,
};

// =============================================================================
// MAIN DRAW FUNCTION
// =============================================================================

pub fn draw_docs_modal(frame: &mut Frame, area: Rect, state: &DocsState, colors: &ThemeColors) {
    match state.mode {
        DocsMode::Edit | DocsMode::Preview => draw_editor(frame, area, state, colors),
        DocsMode::Menu => draw_editor(frame, area, state, colors),
        DocsMode::Find => draw_find_dialog(frame, area, state, colors),
        DocsMode::Replace => draw_replace_dialog(frame, area, state, colors),
        DocsMode::SaveAs => draw_save_as(frame, area, state, colors),
        DocsMode::Help => draw_help(frame, area, state, colors),
        DocsMode::Export => draw_export_dialog(frame, area, state, colors),
    }
}

// =============================================================================
// EDITOR VIEW
// =============================================================================

fn draw_editor(frame: &mut Frame, area: Rect, state: &DocsState, colors: &ThemeColors) {
    let title = format!(
        " Q-DOCS: {}{}",
        state.display_name(),
        if state.modified { " [Modified]" } else { "" }
    );

    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    let content_area = view.content_area();
    let mut row = 0;

    // Menu bar (row 0)
    if state.mode == DocsMode::Menu {
        draw_menu_bar(frame, &view, state, colors);

        // Submenu items (row 1)
        draw_submenu(frame, &view, state, colors);
        row = 2;
    }

    // Ruler (Phase 2)
    if state.show_ruler && state.mode != DocsMode::Menu {
        draw_ruler(frame, &view, state, colors, row);
        row += 1;
    }

    // Separator after menu/ruler
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "-".repeat(content_area.width as usize),
            Style::default().fg(colors.grey()),
        )],
    );
    row += 1;

    // Calculate visible lines
    let visible_lines = (content_area.height as usize).saturating_sub(row as usize + 3);

    // Content area
    if state.mode == DocsMode::Preview {
        draw_preview_content(frame, &view, state, colors, row, visible_lines);
    } else {
        draw_edit_content(frame, &view, state, colors, row, visible_lines);
    }

    // Status bar
    let status_row = content_area.height.saturating_sub(2);
    draw_status_bar(frame, &view, state, colors, status_row);

    // Help footer
    let help_items = if state.mode == DocsMode::Menu {
        vec![
            ("Left/Right", "menu"),
            ("Enter", "select"),
            ("Esc", "close"),
        ]
    } else if state.mode == DocsMode::Preview {
        vec![("F9", "edit"), ("jk", "scroll"), ("Esc", "close")]
    } else {
        vec![
            ("F10", "menu"),
            ("F9", "preview"),
            ("F8", "pages"),
            ("Ctrl+S", "save"),
        ]
    };
    view.render_help(frame, help_items);
}

/// Draw the ruler bar (Phase 2)
fn draw_ruler(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DocsState,
    colors: &ThemeColors,
    row: u16,
) {
    let content_area = view.content_area();
    let line_num_width = if state.show_line_numbers { 5 } else { 0 };
    let ruler_width = (content_area.width as usize).saturating_sub(line_num_width);

    let mut ruler = String::new();

    // Add spacing for line numbers
    ruler.push_str(&" ".repeat(line_num_width));

    // Build ruler string
    for col in 0..ruler_width {
        let actual_col = col + state.h_scroll_offset;

        if actual_col == state.left_margin && state.left_margin > 0 {
            ruler.push('['); // Left margin marker
        } else if actual_col == state.right_margin {
            ruler.push(']'); // Right margin marker
        } else if state.tab_stops.contains(&actual_col) {
            ruler.push('|'); // Tab stop marker (CP437 compatible)
        } else if actual_col.is_multiple_of(10) {
            // Column number tens digit
            ruler.push(char::from_digit((actual_col / 10 % 10) as u32, 10).unwrap_or(' '));
        } else if actual_col % 10 == 5 {
            ruler.push('+'); // Mid-point marker
        } else {
            ruler.push('-');
        }
    }

    view.render_row(
        frame,
        row,
        vec![Span::styled(ruler, Style::default().fg(colors.grey()))],
    );
}

fn draw_menu_bar(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DocsState,
    colors: &ThemeColors,
) {
    let mut spans = vec![];
    let normal_style = Style::default().fg(colors.fg());
    let highlight_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    for (i, cat) in MenuCategory::all().iter().enumerate() {
        let style = if i == state.menu_category {
            highlight_style
        } else {
            normal_style
        };
        spans.push(Span::styled(format!(" {} ", cat.name()), style));
    }

    view.render_row(frame, 0, spans);
}

fn draw_submenu(frame: &mut Frame, view: &FullScreenView, state: &DocsState, colors: &ThemeColors) {
    let category = MenuCategory::all()[state.menu_category];
    let items = category.items();

    let mut spans = vec![];
    let normal_style = Style::default().fg(colors.fg());
    let highlight_style = Style::default()
        .fg(colors.yellow())
        .bg(colors.red())
        .add_modifier(Modifier::BOLD);

    for (i, item) in items.iter().enumerate() {
        let style = if i == state.menu_item {
            highlight_style
        } else {
            normal_style
        };

        let shortcut = item
            .shortcut()
            .map(|s| format!(" ({})", s))
            .unwrap_or_default();
        spans.push(Span::styled(
            format!(" {}{} ", item.name(), shortcut),
            style,
        ));
    }

    view.render_row(frame, 1, spans);
}

fn draw_edit_content(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DocsState,
    colors: &ThemeColors,
    start_row: u16,
    visible_lines: usize,
) {
    let line_num_width = if state.show_line_numbers { 5 } else { 0 };
    let selection = state.selection_bounds();
    let mut rendered_row = 0;

    for line_idx in state.scroll_offset..state.scroll_offset + visible_lines {
        if line_idx >= state.lines.len() {
            break;
        }

        // Check for page boundary (Phase 3)
        if state.page_view_enabled && line_idx > 0 && line_idx % state.lines_per_page == 0 {
            let page_num = line_idx / state.lines_per_page;
            let content_width = view.content_area().width as usize;
            let page_text = format!(" Page {} ", page_num);
            let pad_len = (content_width.saturating_sub(page_text.len())) / 2;
            let page_break = format!(
                "{}{}{}",
                "-".repeat(pad_len),
                page_text,
                "-".repeat(pad_len)
            );

            view.render_row(
                frame,
                start_row + rendered_row as u16,
                vec![Span::styled(page_break, Style::default().fg(colors.grey()))],
            );
            rendered_row += 1;

            if rendered_row >= visible_lines {
                break;
            }
        }

        let line = &state.lines[line_idx];
        let is_cursor_line = line_idx == state.cursor_line;

        let mut spans = vec![];

        // Line numbers
        if state.show_line_numbers {
            let num_style = if is_cursor_line {
                Style::default().fg(colors.yellow())
            } else {
                Style::default().fg(colors.grey())
            };
            spans.push(Span::styled(format!("{:4} ", line_idx + 1), num_style));
        }

        // Check if this line has selection (Phase 1)
        if let Some(((sel_start_line, sel_start_col), (sel_end_line, sel_end_col))) = selection {
            if line_idx >= sel_start_line && line_idx <= sel_end_line {
                // This line contains selection
                let (line_sel_start, line_sel_end) =
                    if line_idx == sel_start_line && line_idx == sel_end_line {
                        (sel_start_col, sel_end_col)
                    } else if line_idx == sel_start_line {
                        (sel_start_col, line.len())
                    } else if line_idx == sel_end_line {
                        (0, sel_end_col)
                    } else {
                        (0, line.len())
                    };

                // Clamp to line bounds
                let line_sel_start = line_sel_start.min(line.len());
                let line_sel_end = line_sel_end.min(line.len());

                let before = &line[..line_sel_start];
                let selected = &line[line_sel_start..line_sel_end];
                let after = &line[line_sel_end..];

                let selection_style = Style::default().fg(colors.yellow()).bg(colors.blue());
                let normal_style = Style::default().fg(colors.fg());

                if !before.is_empty() {
                    spans.push(Span::styled(before.to_string(), normal_style));
                }
                if !selected.is_empty() {
                    spans.push(Span::styled(selected.to_string(), selection_style));
                }
                if !after.is_empty() {
                    spans.push(Span::styled(after.to_string(), normal_style));
                }
                if line.is_empty() {
                    // Empty line but in selection range - show highlight
                    if line_idx > sel_start_line && line_idx < sel_end_line {
                        spans.push(Span::styled(" ", selection_style));
                    }
                }
            } else {
                // No selection on this line - apply markdown syntax highlighting
                let styled_spans = highlight_markdown_line(line, colors, is_cursor_line);
                spans.extend(styled_spans);
            }
        } else {
            // No selection - apply markdown syntax highlighting
            let styled_spans = highlight_markdown_line(line, colors, is_cursor_line);
            spans.extend(styled_spans);
        }

        view.render_row(frame, start_row + rendered_row as u16, spans);
        rendered_row += 1;

        if rendered_row >= visible_lines {
            break;
        }
    }

    // Show cursor indicator
    if state.mode == DocsMode::Edit || state.mode == DocsMode::Preview {
        let cursor_y = (state.cursor_line - state.scroll_offset) as u16;
        let cursor_x = (line_num_width as usize + state.cursor_col) as u16;
        let content = view.content_area();

        if cursor_y < visible_lines as u16 && cursor_x < content.width {
            let cursor_area =
                Rect::new(content.x + cursor_x, content.y + start_row + cursor_y, 1, 1);

            // Get character at cursor
            let cursor_char = state
                .lines
                .get(state.cursor_line)
                .and_then(|l| l.chars().nth(state.cursor_col))
                .unwrap_or(' ');

            // Style based on input mode
            let cursor_style = if state.input_mode == InputMode::Insert {
                Style::default().fg(colors.bg()).bg(colors.yellow())
            } else if state.input_mode == InputMode::Overwrite {
                Style::default().fg(colors.bg()).bg(colors.red())
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

fn draw_preview_content(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DocsState,
    colors: &ThemeColors,
    start_row: u16,
    visible_lines: usize,
) {
    // Render markdown as formatted text
    let rendered = render_markdown(&state.lines);

    for (i, line_idx) in (state.preview_scroll..state.preview_scroll + visible_lines).enumerate() {
        if line_idx >= rendered.len() {
            break;
        }

        let (line, style_type) = &rendered[line_idx];
        let style = match style_type {
            MarkdownStyle::Heading1 => Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
            MarkdownStyle::Heading2 => Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
            MarkdownStyle::Heading3 => Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD),
            MarkdownStyle::Code => Style::default().fg(colors.grey()),
            MarkdownStyle::Quote => Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::ITALIC),
            MarkdownStyle::ListItem => Style::default().fg(colors.fg()),
            MarkdownStyle::HorizontalRule => Style::default().fg(colors.grey()),
            MarkdownStyle::Normal => Style::default().fg(colors.fg()),
        };

        view.render_row(
            frame,
            start_row + i as u16,
            vec![Span::styled(line.clone(), style)],
        );
    }
}

fn draw_status_bar(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DocsState,
    colors: &ThemeColors,
    row: u16,
) {
    let mode_indicator = match state.mode {
        DocsMode::Preview => "[PREVIEW]",
        _ => match state.input_mode {
            InputMode::Normal => "",
            InputMode::Insert => "[INSERT]",
            InputMode::Overwrite => "[OVERWRITE]",
        },
    };

    let position = format!("Ln {}, Col {}", state.cursor_line + 1, state.cursor_col + 1);

    // Page info (Phase 3)
    let page_info = if state.page_view_enabled {
        format!(
            " | Page {}/{}",
            state.line_to_page(state.cursor_line) + 1,
            state.total_pages()
        )
    } else {
        String::new()
    };

    let stats = format!(
        "Words: {} | Pages: {}{}",
        state.word_count(),
        state.page_count(),
        page_info
    );

    // Selection info
    let selection_info = if state.has_selection() {
        if let Some(text) = state.selected_text() {
            format!(" | {} chars", text.len())
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Build status line
    let status = if let Some((msg, _)) = &state.status_message {
        format!("{} | {} | {}", mode_indicator, msg, position)
    } else {
        format!(
            "{} | {} | {}{} | {}",
            mode_indicator,
            state.display_name(),
            stats,
            selection_info,
            position
        )
    };

    view.render_row(
        frame,
        row,
        vec![Span::styled(status, Style::default().fg(colors.green()))],
    );
}

// =============================================================================
// MARKDOWN HIGHLIGHTING
// =============================================================================

#[derive(Debug, Clone, Copy)]
enum MarkdownStyle {
    Normal,
    Heading1,
    Heading2,
    Heading3,
    Code,
    Quote,
    ListItem,
    HorizontalRule,
}

fn highlight_markdown_line(
    line: &str,
    colors: &ThemeColors,
    _is_cursor: bool,
) -> Vec<Span<'static>> {
    let trimmed = line.trim_start();

    // Headings
    if trimmed.starts_with("### ") {
        return vec![Span::styled(
            line.to_string(),
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD),
        )];
    }
    if trimmed.starts_with("## ") {
        return vec![Span::styled(
            line.to_string(),
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )];
    }
    if trimmed.starts_with("# ") {
        return vec![Span::styled(
            line.to_string(),
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )];
    }

    // Code blocks
    if trimmed.starts_with("```") {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(colors.grey()),
        )];
    }

    // Blockquotes
    if trimmed.starts_with("> ") {
        return vec![Span::styled(
            line.to_string(),
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::ITALIC),
        )];
    }

    // Lists
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(colors.fg()),
        )];
    }

    // Numbered lists
    if trimmed
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
        && trimmed.contains(". ")
    {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(colors.fg()),
        )];
    }

    // Horizontal rules
    if trimmed == "---" || trimmed == "***" || trimmed == "___" {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(colors.grey()),
        )];
    }

    // Inline formatting (simplified - full parsing would be more complex)
    let mut spans = vec![];
    let mut current = String::new();
    let mut in_bold = false;
    let mut in_italic = false;
    let mut in_code = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Check for backtick (inline code)
        if c == '`' {
            if !current.is_empty() {
                let style = get_inline_style(in_bold, in_italic, in_code, colors);
                spans.push(Span::styled(current.clone(), style));
                current.clear();
            }
            in_code = !in_code;
            i += 1;
            continue;
        }

        // Check for ** (bold)
        if c == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            if !current.is_empty() {
                let style = get_inline_style(in_bold, in_italic, in_code, colors);
                spans.push(Span::styled(current.clone(), style));
                current.clear();
            }
            in_bold = !in_bold;
            i += 2;
            continue;
        }

        // Check for * (italic) - but not ** which we handled above
        if c == '*' && (i + 1 >= chars.len() || chars[i + 1] != '*') {
            if !current.is_empty() {
                let style = get_inline_style(in_bold, in_italic, in_code, colors);
                spans.push(Span::styled(current.clone(), style));
                current.clear();
            }
            in_italic = !in_italic;
            i += 1;
            continue;
        }

        current.push(c);
        i += 1;
    }

    if !current.is_empty() {
        let style = get_inline_style(in_bold, in_italic, in_code, colors);
        spans.push(Span::styled(current, style));
    }

    if spans.is_empty() {
        spans.push(Span::styled(
            line.to_string(),
            Style::default().fg(colors.fg()),
        ));
    }

    spans
}

fn get_inline_style(bold: bool, italic: bool, code: bool, colors: &ThemeColors) -> Style {
    if code {
        return Style::default().fg(colors.grey());
    }

    let mut style = Style::default().fg(colors.fg());
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    style
}

fn render_markdown(lines: &[String]) -> Vec<(String, MarkdownStyle)> {
    let mut rendered = Vec::new();
    let mut in_code_block = false;

    for line in lines {
        let trimmed = line.trim_start();

        // Toggle code block state
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            rendered.push((line.clone(), MarkdownStyle::Code));
            continue;
        }

        if in_code_block {
            rendered.push((line.clone(), MarkdownStyle::Code));
            continue;
        }

        // Headings
        if trimmed.starts_with("### ") {
            rendered.push((trimmed[4..].to_string(), MarkdownStyle::Heading3));
        } else if trimmed.starts_with("## ") {
            rendered.push((trimmed[3..].to_string(), MarkdownStyle::Heading2));
        } else if trimmed.starts_with("# ") {
            rendered.push((trimmed[2..].to_string(), MarkdownStyle::Heading1));
        }
        // Horizontal rules
        else if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            rendered.push(("-".repeat(40), MarkdownStyle::HorizontalRule));
        }
        // Blockquotes
        else if trimmed.starts_with("> ") {
            rendered.push((format!("  {}", &trimmed[2..]), MarkdownStyle::Quote));
        }
        // Lists
        else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            rendered.push((format!("  * {}", &trimmed[2..]), MarkdownStyle::ListItem));
        }
        // Normal text
        else {
            // Strip inline markdown for preview
            let clean = strip_inline_markdown(trimmed);
            rendered.push((clean, MarkdownStyle::Normal));
        }
    }

    rendered
}

fn strip_inline_markdown(text: &str) -> String {
    text.replace("**", "")
        .replace("__", "")
        .replace("*", "")
        .replace("_", "")
        .replace("`", "")
}

// =============================================================================
// DIALOGS
// =============================================================================

fn draw_find_dialog(frame: &mut Frame, area: Rect, state: &DocsState, colors: &ThemeColors) {
    // First draw the editor behind
    draw_editor(frame, area, state, colors);

    // Then overlay the find dialog
    let width = area.width.min(50);
    let height = 6;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + 3;
    let modal_area = Rect::new(x, y, width, height);

    let modal = ModalFrame::themed(modal_area, " FIND ", colors);
    modal.render_frame(frame);

    let label_style = Style::default().fg(colors.green());
    let input_style = Style::default().fg(colors.yellow()).bg(colors.red());

    modal.render_row(
        frame,
        0,
        vec![
            Span::styled("Find: ", label_style),
            Span::styled(format!("{}|", state.find_query), input_style),
        ],
    );

    if !state.find_results.is_empty() {
        modal.render_row(
            frame,
            2,
            vec![Span::styled(
                format!(
                    "Match {} of {}",
                    state.find_index + 1,
                    state.find_results.len()
                ),
                Style::default().fg(colors.grey()),
            )],
        );
    }

    modal.render_help(frame, vec![("Enter", "next"), ("Esc", "close")]);
}

fn draw_replace_dialog(frame: &mut Frame, area: Rect, state: &DocsState, colors: &ThemeColors) {
    // First draw the editor behind
    draw_editor(frame, area, state, colors);

    let width = area.width.min(50);
    let height = 8;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + 3;
    let modal_area = Rect::new(x, y, width, height);

    let modal = ModalFrame::themed(modal_area, " FIND & REPLACE ", colors);
    modal.render_frame(frame);

    let label_style = Style::default().fg(colors.green());
    let input_style = Style::default().fg(colors.yellow()).bg(colors.red());

    modal.render_row(
        frame,
        0,
        vec![
            Span::styled("Find:    ", label_style),
            Span::styled(state.find_query.clone(), input_style),
        ],
    );

    modal.render_row(
        frame,
        2,
        vec![
            Span::styled("Replace: ", label_style),
            Span::styled(format!("{}|", state.replace_text), input_style),
        ],
    );

    modal.render_help(
        frame,
        vec![("Enter", "replace"), ("A", "all"), ("Esc", "close")],
    );
}

fn draw_save_as(frame: &mut Frame, area: Rect, state: &DocsState, colors: &ThemeColors) {
    let width = area.width.min(60);
    let height = 10;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    let modal = ModalFrame::themed(modal_area, " SAVE AS ", colors);
    modal.render_frame(frame);

    let grey = Style::default().fg(colors.grey());
    let label = Style::default().fg(colors.green());
    let input_style = Style::default().fg(colors.yellow()).bg(colors.red());

    modal.render_row(
        frame,
        0,
        vec![
            Span::styled("Document: ", grey),
            Span::styled(state.display_name(), Style::default().fg(colors.fg())),
        ],
    );

    modal.render_row(
        frame,
        2,
        vec![
            Span::styled("Filename: ", label),
            Span::styled(state.save_as_input.clone(), input_style),
        ],
    );

    modal.render_row(
        frame,
        4,
        vec![Span::styled(
            "(Use .md for Markdown, .txt for plain text)",
            grey,
        )],
    );

    modal.render_help(frame, vec![("Enter", "save"), ("Esc", "cancel")]);
}

fn draw_help(frame: &mut Frame, area: Rect, state: &DocsState, colors: &ThemeColors) {
    let _ = state; // Silence unused warning

    let view = FullScreenView::new(area, " Q-DOCS Help ", colors);
    view.render_frame(frame);

    let title_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(colors.cyan());
    let desc_style = Style::default().fg(colors.fg());

    let help_lines = [
        ("", "Q-DOCS Word Processor", true),
        ("", "", false),
        ("", "Navigation", true),
        ("Arrow keys", "Move cursor", false),
        ("Shift+Arrow", "Extend selection", false),
        ("Home/End", "Start/end of line", false),
        ("PgUp/PgDn", "Page up/down", false),
        ("Ctrl+PgUp", "Previous page", false),
        ("Ctrl+Home", "Start of document", false),
        ("Ctrl+End", "End of document", false),
        ("", "", false),
        ("", "Editing", true),
        ("i", "Enter insert mode", false),
        ("Ins", "Toggle insert/overwrite", false),
        ("Ctrl+X", "Cut selection", false),
        ("Ctrl+C", "Copy selection", false),
        ("Ctrl+V", "Paste", false),
        ("Ctrl+A", "Select all", false),
        ("Ctrl+Z", "Undo", false),
        ("Ctrl+Y", "Redo", false),
        ("", "", false),
        ("", "File", true),
        ("Ctrl+S", "Save", false),
        ("F10", "Menu", false),
        ("F9", "Preview mode", false),
        ("F8", "Page view", false),
    ];

    for (i, (key, desc, is_title)) in help_lines.iter().enumerate() {
        if *is_title {
            view.render_row(frame, i as u16, vec![Span::styled(*desc, title_style)]);
        } else if !key.is_empty() {
            view.render_row(
                frame,
                i as u16,
                vec![
                    Span::styled(format!("{:12}", key), key_style),
                    Span::styled(*desc, desc_style),
                ],
            );
        }
    }

    view.render_help(frame, vec![("Esc", "close")]);
}

/// Draw the export format selection dialog (Phase 4)
fn draw_export_dialog(frame: &mut Frame, area: Rect, state: &DocsState, colors: &ThemeColors) {
    // First draw the editor behind
    draw_editor(frame, area, state, colors);

    let width = area.width.min(55);
    let height = 12;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    let modal = ModalFrame::themed(modal_area, " EXPORT DOCUMENT ", colors);
    modal.render_frame(frame);

    let label_style = Style::default().fg(colors.green());
    let selected_style = Style::default()
        .fg(colors.yellow())
        .bg(colors.red())
        .add_modifier(Modifier::BOLD);
    let normal_style = Style::default().fg(colors.fg());

    modal.render_row(
        frame,
        0,
        vec![Span::styled("Select export format:", label_style)],
    );

    // Format options
    let formats = ExportFormat::all();
    for (i, fmt) in formats.iter().enumerate() {
        let style = if *fmt == state.export_format {
            selected_style
        } else {
            normal_style
        };
        modal.render_row(
            frame,
            (i + 2) as u16,
            vec![Span::styled(
                format!("  {} - {}", fmt.name(), fmt.description()),
                style,
            )],
        );
    }

    // Show pandoc status for PDF
    let pandoc_status = if ops::pandoc_available() {
        Span::styled("  pandoc: installed", Style::default().fg(colors.green()))
    } else {
        Span::styled("  pandoc: not found", Style::default().fg(colors.grey()))
    };
    modal.render_row(frame, 7, vec![pandoc_status]);

    modal.render_help(
        frame,
        vec![
            ("Up/Down", "select"),
            ("Enter", "export"),
            ("Esc", "cancel"),
        ],
    );
}
