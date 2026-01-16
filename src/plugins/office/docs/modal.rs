//! Q-DOCS Modal Rendering
//!
//! Renders the word processor interface with markdown highlighting.

use super::state::{DocsMode, DocsState, InputMode, MenuCategory};
use crate::app::ThemeColors;
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

pub fn draw_docs_modal(frame: &mut Frame, area: Rect, state: &DocsState, colors: &ThemeColors) {
    match state.mode {
        DocsMode::Edit | DocsMode::Preview => draw_editor(frame, area, state, colors),
        DocsMode::Menu => draw_editor(frame, area, state, colors),
        DocsMode::Find => draw_find_dialog(frame, area, state, colors),
        DocsMode::Replace => draw_replace_dialog(frame, area, state, colors),
        DocsMode::SaveAs => draw_save_as(frame, area, state, colors),
        DocsMode::Help => draw_help(frame, area, state, colors),
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

    // Separator after menu
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
            ("Ctrl+S", "save"),
            ("Esc", "close"),
        ]
    };
    view.render_help(frame, help_items);
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

    for (i, line_idx) in (state.scroll_offset..state.scroll_offset + visible_lines).enumerate() {
        if line_idx >= state.lines.len() {
            break;
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

        // Apply markdown syntax highlighting
        let styled_spans = highlight_markdown_line(line, colors, is_cursor_line);
        spans.extend(styled_spans);

        // Show cursor position
        if is_cursor_line && state.input_mode != InputMode::Normal {
            // Cursor is shown by terminal, but we could add a visual indicator
        }

        view.render_row(frame, start_row + i as u16, spans);
    }

    // Show cursor indicator
    if state.input_mode != InputMode::Normal {
        let cursor_row = start_row + (state.cursor_line - state.scroll_offset) as u16;
        let cursor_col = line_num_width + state.cursor_col;
        // The actual cursor is handled by the terminal, but we track position for rendering
        let _ = (cursor_row, cursor_col);
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

    let stats = format!(
        "Words: {} | Pages: {}",
        state.word_count(),
        state.page_count()
    );

    // Build status line
    let status = if let Some((msg, _)) = &state.status_message {
        format!("{} | {} | {}", mode_indicator, msg, position)
    } else {
        format!(
            "{} | {} | {} | {}",
            mode_indicator,
            state.display_name(),
            stats,
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
        ("Home/End", "Start/end of line", false),
        ("PgUp/PgDn", "Page up/down", false),
        ("Ctrl+Home", "Start of document", false),
        ("Ctrl+End", "End of document", false),
        ("", "", false),
        ("", "Editing", true),
        ("i", "Enter insert mode", false),
        ("Ins", "Toggle insert/overwrite", false),
        ("Backspace", "Delete before cursor", false),
        ("Delete", "Delete at cursor", false),
        ("Ctrl+Z", "Undo", false),
        ("Ctrl+Y", "Redo", false),
        ("", "", false),
        ("", "Formatting", true),
        ("Ctrl+B", "Bold", false),
        ("Ctrl+I", "Italic", false),
        ("", "", false),
        ("", "File", true),
        ("Ctrl+S", "Save", false),
        ("F10", "Menu", false),
        ("F9", "Preview mode", false),
        ("Esc", "Close", false),
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
