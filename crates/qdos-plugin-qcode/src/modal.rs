//! Q-CODE UI Rendering
//!
//! Modal rendering for the code editor.

use crate::state::{EditorBuffer, QCodeState, QCodeView};
use qdos_plugin_api::prelude::{FullScreenView, ThemeColors};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

// =============================================================================
// SYNTAX HIGHLIGHTING
// =============================================================================

/// Get syntax highlighting color for a token based on file extension
fn get_token_style(token: &str, ext: Option<&str>, colors: &ThemeColors) -> Style {
    let ext = ext.unwrap_or("");

    // Keywords for various languages
    let keywords: &[&str] = match ext {
        "rs" => &[
            "fn", "let", "mut", "const", "pub", "mod", "use", "struct", "enum", "impl", "trait",
            "for", "while", "loop", "if", "else", "match", "return", "break", "continue", "self",
            "Self", "true", "false", "Some", "None", "Ok", "Err", "async", "await", "where",
            "type", "dyn", "static", "unsafe", "extern", "crate", "super", "move", "ref", "in",
        ],
        "py" => &[
            "def", "class", "import", "from", "if", "elif", "else", "for", "while", "try",
            "except", "finally", "with", "as", "return", "yield", "break", "continue", "pass",
            "raise", "True", "False", "None", "and", "or", "not", "in", "is", "lambda", "global",
            "nonlocal", "async", "await",
        ],
        "js" | "ts" | "jsx" | "tsx" => &[
            "function",
            "const",
            "let",
            "var",
            "if",
            "else",
            "for",
            "while",
            "do",
            "switch",
            "case",
            "default",
            "break",
            "continue",
            "return",
            "throw",
            "try",
            "catch",
            "finally",
            "class",
            "extends",
            "new",
            "this",
            "super",
            "import",
            "export",
            "from",
            "async",
            "await",
            "true",
            "false",
            "null",
            "undefined",
            "typeof",
            "instanceof",
            "in",
            "of",
        ],
        "go" => &[
            "func",
            "package",
            "import",
            "var",
            "const",
            "type",
            "struct",
            "interface",
            "map",
            "chan",
            "if",
            "else",
            "for",
            "range",
            "switch",
            "case",
            "default",
            "break",
            "continue",
            "return",
            "go",
            "defer",
            "select",
            "true",
            "false",
            "nil",
        ],
        "c" | "h" | "cpp" | "hpp" | "cc" => &[
            "int",
            "char",
            "float",
            "double",
            "void",
            "long",
            "short",
            "unsigned",
            "signed",
            "struct",
            "union",
            "enum",
            "typedef",
            "if",
            "else",
            "for",
            "while",
            "do",
            "switch",
            "case",
            "default",
            "break",
            "continue",
            "return",
            "goto",
            "sizeof",
            "static",
            "extern",
            "const",
            "volatile",
            "register",
            "auto",
            "class",
            "public",
            "private",
            "protected",
            "virtual",
            "override",
            "template",
            "typename",
            "namespace",
            "using",
            "true",
            "false",
            "nullptr",
            "new",
            "delete",
            "this",
            "throw",
            "try",
            "catch",
        ],
        "java" => &[
            "public",
            "private",
            "protected",
            "class",
            "interface",
            "extends",
            "implements",
            "static",
            "final",
            "abstract",
            "void",
            "int",
            "long",
            "double",
            "float",
            "boolean",
            "char",
            "byte",
            "short",
            "if",
            "else",
            "for",
            "while",
            "do",
            "switch",
            "case",
            "default",
            "break",
            "continue",
            "return",
            "throw",
            "try",
            "catch",
            "finally",
            "new",
            "this",
            "super",
            "true",
            "false",
            "null",
            "import",
            "package",
            "instanceof",
        ],
        "sh" | "bash" | "zsh" => &[
            "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac",
            "function", "return", "exit", "break", "continue", "local", "export", "source", "in",
            "true", "false",
        ],
        "toml" => &["true", "false"],
        "json" => &["true", "false", "null"],
        "yaml" | "yml" => &["true", "false", "null", "yes", "no"],
        "md" | "markdown" => &[],
        _ => &[],
    };

    // Check if token is a keyword
    if keywords.contains(&token) {
        return Style::default()
            .fg(colors.magenta())
            .add_modifier(Modifier::BOLD);
    }

    // Check for strings (starts with " or ')
    if token.starts_with('"') || token.starts_with('\'') {
        return Style::default().fg(colors.green());
    }

    // Check for numbers
    if token
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        return Style::default().fg(colors.cyan());
    }

    // Check for comments
    if token.starts_with("//") || token.starts_with('#') || token.starts_with("--") {
        return Style::default().fg(colors.grey());
    }

    // Default style
    Style::default().fg(colors.fg())
}

/// Simple tokenizer for syntax highlighting
fn tokenize_line(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut string_char = '"';
    let in_comment = false;

    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Check for comment start
        if !in_string && !in_comment {
            if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                // Rest of line is comment
                tokens.push(line[i..].to_string());
                break;
            }
            if c == '#' && (current.is_empty() || current.chars().all(|c| c.is_whitespace())) {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                // Rest of line is comment
                tokens.push(line[i..].to_string());
                break;
            }
        }

        // Handle strings
        if !in_comment && (c == '"' || c == '\'') {
            if in_string && c == string_char {
                current.push(c);
                tokens.push(current.clone());
                current.clear();
                in_string = false;
            } else if !in_string {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                in_string = true;
                string_char = c;
                current.push(c);
            } else {
                current.push(c);
            }
            i += 1;
            continue;
        }

        if in_string || in_comment {
            current.push(c);
            i += 1;
            continue;
        }

        // Word boundaries
        if c.is_whitespace()
            || c == '('
            || c == ')'
            || c == '{'
            || c == '}'
            || c == '['
            || c == ']'
            || c == ','
            || c == ';'
            || c == ':'
            || c == '.'
            || c == '+'
            || c == '-'
            || c == '*'
            || c == '/'
            || c == '='
            || c == '<'
            || c == '>'
            || c == '!'
            || c == '&'
            || c == '|'
        {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            tokens.push(c.to_string());
        } else {
            current.push(c);
        }

        i += 1;
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

// =============================================================================
// FILE TREE VIEW
// =============================================================================

fn draw_file_tree(state: &QCodeState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Files ")
        .border_style(Style::default().fg(colors.blue()))
        .title_style(
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_height = inner.height as usize;
    let start = state.file_tree_scroll;
    let end = (start + visible_height).min(state.file_tree.len());

    for (i, entry) in state
        .file_tree
        .iter()
        .skip(start)
        .take(end - start)
        .enumerate()
    {
        let is_selected = start + i == state.file_tree_cursor;
        let style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else if entry.is_dir {
            Style::default().fg(colors.cyan())
        } else {
            Style::default().fg(colors.fg())
        };

        let marker = if is_selected { ">" } else { " " };
        let icon = if entry.is_dir { "[D]" } else { "   " };
        let name = &entry.name;
        let text = format!("{} {} {}", marker, icon, name);

        let para = Paragraph::new(text).style(style);
        frame.render_widget(para, Rect::new(inner.x, inner.y + i as u16, inner.width, 1));
    }
}

// =============================================================================
// EDITOR VIEW
// =============================================================================

fn draw_editor(
    buffer: &EditorBuffer,
    frame: &mut Frame,
    area: Rect,
    colors: &ThemeColors,
    active: bool,
) {
    let border_color = if active { colors.cyan() } else { colors.blue() };
    let title = format!(
        " {} {} ",
        buffer.display_name(),
        if buffer.modified { "*" } else { "" }
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(border_color))
        .title_style(
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Calculate line number width
    let total_lines = buffer.lines.len();
    let line_num_width = format!("{}", total_lines).len().max(3) as u16 + 1;

    let visible_height = inner.height as usize;
    let start = buffer.scroll_offset;
    let end = (start + visible_height).min(buffer.lines.len());

    let ext = buffer.extension();
    let ext_ref = ext.as_deref();

    for (i, line) in buffer
        .lines
        .iter()
        .skip(start)
        .take(end - start)
        .enumerate()
    {
        let line_num = start + i + 1;
        let is_cursor_line = start + i == buffer.cursor_row;

        // Draw line number
        let line_num_style = if is_cursor_line {
            Style::default().fg(colors.yellow())
        } else {
            Style::default().fg(colors.grey())
        };
        let line_num_text = format!("{:>width$} ", line_num, width = line_num_width as usize - 1);
        let line_num_para = Paragraph::new(line_num_text).style(line_num_style);
        frame.render_widget(
            line_num_para,
            Rect::new(inner.x, inner.y + i as u16, line_num_width, 1),
        );

        // Draw line content with syntax highlighting
        let content_x = inner.x + line_num_width;
        let content_width = inner.width.saturating_sub(line_num_width);

        // Simple syntax highlighting
        let tokens = tokenize_line(line);
        let mut x_offset = 0u16;

        for token in tokens {
            let style = get_token_style(&token, ext_ref, colors);
            let token_len = token.len() as u16;

            if x_offset + token_len <= content_width {
                let para = Paragraph::new(token.clone()).style(style);
                frame.render_widget(
                    para,
                    Rect::new(content_x + x_offset, inner.y + i as u16, token_len, 1),
                );
            }
            x_offset += token_len;
        }

        // Draw cursor if on this line and editor is active
        if active && is_cursor_line {
            let cursor_x =
                content_x + (buffer.cursor_col as u16).min(content_width.saturating_sub(1));
            let cursor_y = inner.y + i as u16;
            if cursor_x < inner.x + inner.width {
                // Get character at cursor position or space
                let cursor_char = line.chars().nth(buffer.cursor_col).unwrap_or(' ');
                let cursor_text = cursor_char.to_string();
                let cursor_style = Style::default()
                    .fg(colors.bg())
                    .bg(colors.fg())
                    .add_modifier(Modifier::BOLD);
                let cursor_para = Paragraph::new(cursor_text).style(cursor_style);
                frame.render_widget(cursor_para, Rect::new(cursor_x, cursor_y, 1, 1));
            }
        }
    }
}

// =============================================================================
// HELP VIEW
// =============================================================================

fn draw_help(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-CODE Help ", colors);
    view.render_frame(frame);

    let header_style = Style::default()
        .fg(colors.cyan())
        .add_modifier(Modifier::BOLD);
    let normal = Style::default().fg(colors.fg());
    let key_style = Style::default().fg(colors.yellow());

    let help_text = [
        ("Q-CODE - Simple Code Editor", header_style),
        ("", normal),
        ("File Tree:", header_style),
        ("  Up/Down      Navigate files", normal),
        ("  Enter        Open file/directory", normal),
        ("  Backspace    Go up directory", normal),
        ("  Tab          Switch to editor", normal),
        ("", normal),
        ("Editor:", header_style),
        ("  Arrow keys   Move cursor", normal),
        ("  Home/End     Line start/end", normal),
        ("  PgUp/PgDn    Scroll", normal),
        ("  Ctrl+S       Save file", normal),
        ("  Tab          Switch to file tree", normal),
        ("  Enter        New line", normal),
        ("  Backspace    Delete character", normal),
        ("", normal),
        ("General:", header_style),
        ("  F1           Show this help", normal),
        ("  Esc          Exit Q-CODE", normal),
    ];

    for (i, (text, style)) in help_text.iter().enumerate() {
        if i as u16 + 1 >= view.content_height() {
            break;
        }
        // Key binding line
        if text.contains("  ") && !text.is_empty() {
            let parts: Vec<&str> = text.splitn(2, "  ").collect();
            if parts.len() == 2 {
                view.render_row(
                    frame,
                    i as u16,
                    vec![
                        Span::styled(format!("  {:12}", parts[0].trim()), key_style),
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

pub fn draw_qcode(state: &QCodeState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    // Clear the area with background
    let clear = ratatui::widgets::Clear;
    frame.render_widget(clear, area);

    match state.view {
        QCodeView::Help => {
            draw_help(frame, area, colors);
        }
        _ => {
            // Use FullScreenView as outer container for IDE layout
            let title = if let Some(buffer) = state.current_buffer() {
                format!(
                    " Q-CODE: {} {}",
                    buffer.display_name(),
                    if buffer.modified { "*" } else { "" }
                )
            } else {
                " Q-CODE ".to_string()
            };

            let view = FullScreenView::new(area, &title, colors);
            view.render_frame(frame);

            let content = view.content_area();

            // Split into file tree and editor panes
            let tree_width = state.tree_width.min(content.width / 3);
            let editor_width = content.width.saturating_sub(tree_width);

            let tree_area = Rect::new(content.x, content.y, tree_width, content.height);
            let editor_area = Rect::new(
                content.x + tree_width,
                content.y,
                editor_width,
                content.height,
            );

            // Draw file tree
            draw_file_tree(state, frame, tree_area, colors);

            // Draw editor
            if let Some(buffer) = state.current_buffer() {
                let active = state.view == QCodeView::Editor;
                draw_editor(buffer, frame, editor_area, colors, active);
            } else {
                // Empty editor placeholder
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(" Editor ")
                    .border_style(Style::default().fg(colors.blue()));
                frame.render_widget(block, editor_area);
            }

            // Draw status in help area
            let help = match state.view {
                QCodeView::FileTree => vec![
                    ("^v", "navigate"),
                    ("Enter", "open"),
                    ("Tab", "editor"),
                    ("F1", "help"),
                    ("Esc", "exit"),
                ],
                QCodeView::Editor => vec![
                    ("Arrows", "move"),
                    ("C-S", "save"),
                    ("Tab", "files"),
                    ("F1", "help"),
                    ("Esc", "exit"),
                ],
                QCodeView::Terminal => vec![("Tab", "switch"), ("Esc", "exit")],
                QCodeView::Help => vec![("Esc", "back")],
            };

            view.render_help(frame, help);
        }
    }
}
