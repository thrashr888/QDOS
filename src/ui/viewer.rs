//! File viewer UI components
//!
//! This module contains all file viewer drawing functions including:
//! - Normal/ASCII view with syntax highlighting
//! - Hex view
//! - Image view (with Kitty/Sixel/iTerm2 protocol detection)
//! - Markdown view

use crate::app::{FileViewerState, ViewFilter, ViewMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;
use std::sync::{Mutex, OnceLock};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

use super::{COLOR_BLUE, COLOR_FG, COLOR_GREEN, COLOR_RED};

// Lazy-loaded syntax highlighting resources
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

// Lazy-loaded image picker (detects Kitty/Sixel/iTerm2 protocols)
static IMAGE_PICKER: OnceLock<Mutex<Picker>> = OnceLock::new();

/// Get or initialize the image picker with terminal protocol detection
fn get_image_picker() -> &'static Mutex<Picker> {
    IMAGE_PICKER.get_or_init(|| {
        // Try to detect terminal capabilities, fall back to halfblocks
        let picker = Picker::from_query_stdio()
            .unwrap_or_else(|_| Picker::from_fontsize((8, 16))); // Default font size fallback
        Mutex::new(picker)
    })
}

/// Get or create image protocol for the given file
fn get_or_create_image_protocol(_file_path: &str, content: &[u8]) -> Option<StatefulProtocol> {
    // Create protocol for the image (recreated each render for simplicity)
    // Future optimization: cache protocols by file path
    if let Ok(dyn_img) = image::load_from_memory(content) {
        if let Ok(mut picker) = get_image_picker().lock() {
            return Some(picker.new_resize_protocol(dyn_img));
        }
    }
    None
}

fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn get_theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

/// Convert syntect color to ratatui color
fn syntect_to_ratatui_color(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

/// Check if a file extension supports syntax highlighting
#[allow(dead_code)] // May be used for UI indicator
fn supports_syntax_highlighting(file_name: &str) -> bool {
    let ss = get_syntax_set();
    ss.find_syntax_for_file(file_name)
        .ok()
        .flatten()
        .map(|s| s.name != "Plain Text")
        .unwrap_or(false)
}

/// Draw file viewer screen (full screen)
pub(super) fn draw_file_viewer(frame: &mut Frame, area: Rect, state: &FileViewerState) {
    // Clear the entire screen
    frame.render_widget(Clear, area);

    // Layout: title bar, separator, content, separator, status/help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title bar with file name and mode
            Constraint::Length(1), // Separator
            Constraint::Min(5),    // Content area
            Constraint::Length(1), // Separator
            Constraint::Length(1), // Status/help line
        ])
        .split(area);

    // Title bar: file name and mode/filter info
    let mode_str = match state.mode {
        ViewMode::Normal => "NORMAL",
        ViewMode::Hex => "HEX",
        ViewMode::Image => "IMAGE",
        ViewMode::Markdown => "MARKDOWN",
        ViewMode::Blame => "BLAME",
        ViewMode::Diff => "DIFF",
    };
    let filter_str = match state.filter {
        ViewFilter::Off => "",
        ViewFilter::Ascii => " [Filter: ASCII]",
        ViewFilter::WordStar => " [Filter: W/S]",
    };

    // Show commit info if viewing a historical version
    let version_str = if let Some(entry) = state.current_commit() {
        let short_hash = &entry.hash[..7.min(entry.hash.len())];
        let short_msg = if entry.message.len() > 25 {
            format!("{}...", &entry.message[..22])
        } else {
            entry.message.clone()
        };
        format!("  [{}] {} - {}", short_hash, entry.date, short_msg)
    } else if state.is_git_repo && !state.git_history.is_empty() {
        "  [working copy]".to_string()
    } else {
        String::new()
    };

    let title = format!(
        " VIEW: {}  Mode: {}{}{}",
        state.file_name.to_uppercase(),
        mode_str,
        filter_str,
        version_str
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            title,
            Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
        )),
        chunks[0],
    );

    // Separator
    let sep = "═".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(COLOR_FG))),
        chunks[1],
    );

    // Content area
    let content_height = chunks[2].height as usize;
    match state.mode {
        ViewMode::Normal => draw_normal_view(frame, chunks[2], state, content_height),
        ViewMode::Hex => draw_hex_view(frame, chunks[2], state, content_height),
        ViewMode::Image => draw_image_view(frame, chunks[2], state),
        ViewMode::Markdown => draw_markdown_view(frame, chunks[2], state, content_height),
        ViewMode::Blame => draw_blame_view(frame, chunks[2], state, content_height),
        ViewMode::Diff => draw_diff_view(frame, chunks[2], state, content_height),
    }

    // Separator
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(COLOR_FG))),
        chunks[3],
    );

    // Help line - conditionally show history navigation
    let mut help_spans = vec![
        Span::styled(" H", Style::default().fg(COLOR_BLUE)),
        Span::raw("ex "),
        Span::styled("N", Style::default().fg(COLOR_BLUE)),
        Span::raw("ormal "),
        Span::styled("I", Style::default().fg(COLOR_BLUE)),
        Span::raw("mage "),
        Span::styled("M", Style::default().fg(COLOR_BLUE)),
        Span::raw("arkdown "),
    ];

    // Add git-specific options if in git repo
    if state.is_git_repo {
        help_spans.push(Span::styled("B", Style::default().fg(COLOR_BLUE)));
        help_spans.push(Span::raw("lame "));
        help_spans.push(Span::styled("D", Style::default().fg(COLOR_BLUE)));
        help_spans.push(Span::raw("iff "));
    }

    help_spans.push(Span::styled("F", Style::default().fg(COLOR_BLUE)));
    help_spans.push(Span::raw("ilter "));
    help_spans.push(Span::styled("↑↓", Style::default().fg(COLOR_BLUE)));
    help_spans.push(Span::raw(" scroll "));

    // Add history navigation if available
    if state.has_older_version() {
        help_spans.push(Span::styled("←", Style::default().fg(COLOR_BLUE)));
        help_spans.push(Span::raw(" older "));
    }
    if state.has_newer_version() {
        help_spans.push(Span::styled("→", Style::default().fg(COLOR_BLUE)));
        help_spans.push(Span::raw(" newer "));
    }

    help_spans.push(Span::styled("Esc", Style::default().fg(COLOR_BLUE)));
    help_spans.push(Span::raw(" exit"));

    frame.render_widget(Paragraph::new(Line::from(help_spans)), chunks[4]);
}

/// Draw normal/ASCII view mode with optional syntax highlighting
fn draw_normal_view(frame: &mut Frame, area: Rect, state: &FileViewerState, height: usize) {
    // Try to get content as string for syntax highlighting
    let content_str = String::from_utf8_lossy(&state.content);

    // Check if we should use syntax highlighting
    let ss = get_syntax_set();
    let ts = get_theme_set();

    let syntax = ss
        .find_syntax_for_file(&state.file_name)
        .ok()
        .flatten()
        .filter(|s| s.name != "Plain Text");

    // Calculate scroll
    let all_lines: Vec<&str> = content_str.lines().collect();
    let max_scroll = all_lines.len().saturating_sub(height);
    let scroll = state.scroll_offset.min(max_scroll);

    let visible_lines: Vec<Line> = if let Some(syntax) = syntax {
        // Use syntax highlighting
        let theme = &ts.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);

        all_lines
            .iter()
            .enumerate()
            .skip(scroll)
            .take(height)
            .map(|(_, line)| {
                let highlighted = highlighter.highlight_line(line, ss).unwrap_or_default();

                let mut spans: Vec<Span> = vec![Span::raw(" ")]; // Left padding

                for (style, text) in highlighted {
                    let fg = syntect_to_ratatui_color(style.foreground);
                    let mut ratatui_style = Style::default().fg(fg);

                    // Apply font style modifiers
                    if style
                        .font_style
                        .contains(syntect::highlighting::FontStyle::BOLD)
                    {
                        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                    }
                    if style
                        .font_style
                        .contains(syntect::highlighting::FontStyle::ITALIC)
                    {
                        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                    }
                    if style
                        .font_style
                        .contains(syntect::highlighting::FontStyle::UNDERLINE)
                    {
                        ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
                    }

                    spans.push(Span::styled(text.to_string(), ratatui_style));
                }

                Line::from(spans)
            })
            .collect()
    } else {
        // Fall back to plain text with filter applied
        let lines: Vec<String> = state
            .content
            .split(|&b| b == b'\n')
            .map(|line| {
                line.iter()
                    .map(|&b| match state.filter {
                        ViewFilter::Off => {
                            if (32..127).contains(&b) {
                                b as char
                            } else if b == b'\t' {
                                ' '
                            } else if b == b'\r' {
                                ' '
                            } else {
                                '.'
                            }
                        }
                        ViewFilter::Ascii => {
                            if (32..127).contains(&b) {
                                b as char
                            } else {
                                ' '
                            }
                        }
                        ViewFilter::WordStar => {
                            let b = b & 0x7F; // Strip high bit
                            if (32..127).contains(&b) {
                                b as char
                            } else {
                                ' '
                            }
                        }
                    })
                    .collect::<String>()
            })
            .collect();

        lines
            .iter()
            .skip(scroll)
            .take(height)
            .map(|line| {
                Line::from(Span::styled(
                    format!(" {}", line),
                    Style::default().fg(COLOR_FG),
                ))
            })
            .collect()
    };

    frame.render_widget(Paragraph::new(visible_lines), area);
}

/// Draw hex view mode
fn draw_hex_view(frame: &mut Frame, area: Rect, state: &FileViewerState, height: usize) {
    let bytes_per_line: usize = 16;
    let total_lines = state.content.len().div_ceil(bytes_per_line);

    // Calculate max scroll
    let max_scroll = total_lines.saturating_sub(height);
    let scroll = state.scroll_offset.min(max_scroll);

    let mut lines: Vec<Line> = Vec::new();

    for line_idx in scroll..(scroll + height).min(total_lines) {
        let offset = line_idx * bytes_per_line;
        let end = (offset + bytes_per_line).min(state.content.len());
        let chunk = &state.content[offset..end];

        // Build the hex line
        let mut spans = Vec::new();

        // Offset (8 hex digits)
        spans.push(Span::styled(
            format!(" {:08X}  ", offset),
            Style::default().fg(COLOR_BLUE),
        ));

        // Hex bytes (two groups of 8)
        for (i, &byte) in chunk.iter().enumerate() {
            if i == 8 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                format!("{:02X} ", byte),
                Style::default().fg(COLOR_FG),
            ));
        }

        // Pad if less than 16 bytes
        for i in chunk.len()..bytes_per_line {
            if i == 8 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::raw("   "));
        }

        // ASCII representation
        spans.push(Span::raw("  "));
        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if (32..127).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        spans.push(Span::styled(ascii, Style::default().fg(COLOR_GREEN)));

        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

/// Draw image view mode using ratatui-image with protocol detection
/// Supports Kitty graphics, Sixel, iTerm2, and falls back to Unicode halfblocks
fn draw_image_view(frame: &mut Frame, area: Rect, state: &FileViewerState) {
    // Try to get or create the image protocol
    if let Some(mut protocol) = get_or_create_image_protocol(
        &state.file_path.to_string_lossy(),
        &state.content,
    ) {
        // Use StatefulImage widget for rendering
        let image_widget = StatefulImage::new(None);
        frame.render_stateful_widget(image_widget, area, &mut protocol);
    } else {
        // Show error if image can't be loaded
        let error_msg = vec![
            Line::from(""),
            Line::from(Span::styled(
                " Cannot display image",
                Style::default().fg(COLOR_RED).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!(" File: {}", state.file_path.display()),
                Style::default().fg(COLOR_GREEN),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " Press N for normal view or H for hex view",
                Style::default().fg(COLOR_BLUE),
            )),
        ];
        frame.render_widget(Paragraph::new(error_msg), area);
    }
}

/// Draw markdown view mode
fn draw_markdown_view(frame: &mut Frame, area: Rect, state: &FileViewerState, height: usize) {
    // Convert content to string
    let content_str = String::from_utf8_lossy(&state.content);

    // Parse and render markdown manually (simplified version)
    // termimad is primarily for printing directly; we'll do a simplified render for ratatui
    let mut lines: Vec<Line> = Vec::new();

    for raw_line in content_str.lines() {
        let line = raw_line;

        // Headers
        if line.starts_with("# ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", &line[2..]),
                Style::default().fg(COLOR_BLUE).add_modifier(Modifier::BOLD),
            )));
        } else if line.starts_with("## ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", &line[3..]),
                Style::default().fg(COLOR_BLUE).add_modifier(Modifier::BOLD),
            )));
        } else if line.starts_with("### ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", &line[4..]),
                Style::default().fg(COLOR_BLUE),
            )));
        } else if line.starts_with("#### ")
            || line.starts_with("##### ")
            || line.starts_with("###### ")
        {
            let header_content = line.trim_start_matches('#').trim_start();
            lines.push(Line::from(Span::styled(
                format!(" {}", header_content),
                Style::default().fg(COLOR_BLUE),
            )));
        }
        // Code blocks
        else if line.starts_with("```") {
            lines.push(Line::from(Span::styled(
                " \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                Style::default().fg(COLOR_GREEN),
            )));
        }
        // Bullet points
        else if line.starts_with("- ") || line.starts_with("* ") {
            lines.push(Line::from(Span::styled(
                format!("  \u{2022} {}", &line[2..]),
                Style::default().fg(COLOR_FG),
            )));
        }
        // Numbered lists
        else if line
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            && line.contains(". ")
        {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(COLOR_FG),
            )));
        }
        // Blockquotes
        else if line.starts_with("> ") {
            lines.push(Line::from(Span::styled(
                format!(" \u{2502} {}", &line[2..]),
                Style::default().fg(COLOR_GREEN),
            )));
        }
        // Horizontal rules
        else if line == "---" || line == "***" || line == "___" {
            lines.push(Line::from(Span::styled(
                " \u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}",
                Style::default().fg(COLOR_FG),
            )));
        }
        // Links and emphasis (simplified - just show as-is with color hints)
        else if line.contains("**") || line.contains("__") {
            // Bold text - simple approach, just highlight the whole line
            let clean_line = line.replace("**", "").replace("__", "");
            lines.push(Line::from(Span::styled(
                format!(" {}", clean_line),
                Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
            )));
        } else if line.contains('*') || line.contains('_') {
            // Italic text - show with different color
            let clean_line = line
                .chars()
                .filter(|&c| c != '*' && c != '_')
                .collect::<String>();
            lines.push(Line::from(Span::styled(
                format!(" {}", clean_line),
                Style::default().fg(COLOR_FG).add_modifier(Modifier::ITALIC),
            )));
        }
        // Code inline
        else if line.contains('`') {
            lines.push(Line::from(Span::styled(
                format!(" {}", line),
                Style::default().fg(COLOR_GREEN),
            )));
        }
        // Regular text
        else if line.trim().is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(Span::styled(
                format!(" {}", line),
                Style::default().fg(COLOR_FG),
            )));
        }
    }

    // Calculate max scroll
    let max_scroll = lines.len().saturating_sub(height);
    let scroll = state.scroll_offset.min(max_scroll);

    // Render visible lines
    let visible_lines: Vec<Line> = lines.into_iter().skip(scroll).take(height).collect();

    frame.render_widget(Paragraph::new(visible_lines), area);
}

/// Draw git blame view mode
fn draw_blame_view(frame: &mut Frame, area: Rect, state: &FileViewerState, height: usize) {
    if state.blame_lines.is_empty() {
        let error_msg = vec![
            Line::from(""),
            Line::from(Span::styled(
                " No blame data available",
                Style::default().fg(COLOR_RED),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " File may not be tracked by git",
                Style::default().fg(COLOR_FG),
            )),
        ];
        frame.render_widget(Paragraph::new(error_msg), area);
        return;
    }

    // Calculate scroll
    let max_scroll = state.blame_lines.len().saturating_sub(height);
    let scroll = state.scroll_offset.min(max_scroll);

    // Build visible lines
    let visible_lines: Vec<Line> = state
        .blame_lines
        .iter()
        .enumerate()
        .skip(scroll)
        .take(height)
        .map(|(line_num, blame)| {
            // Truncate author to 10 chars
            let author = if blame.author.len() > 10 {
                format!("{}…", &blame.author[..9])
            } else {
                format!("{:10}", blame.author)
            };

            // Format: hash author time_ago | content
            let mut spans = vec![
                Span::styled(
                    format!(" {:>4} ", line_num + 1),
                    Style::default().fg(COLOR_BLUE),
                ),
                Span::styled(
                    format!("{} ", blame.hash),
                    Style::default().fg(Color::Rgb(128, 128, 128)),
                ),
                Span::styled(author, Style::default().fg(COLOR_GREEN)),
                Span::styled(
                    format!(" {:>8} │ ", blame.time_ago),
                    Style::default().fg(Color::Rgb(128, 128, 128)),
                ),
                Span::styled(&blame.line_content, Style::default().fg(COLOR_FG)),
            ];

            // Pad to fill width
            let content_len = 6 + 8 + 10 + 11 + blame.line_content.len();
            if content_len < area.width as usize {
                spans.push(Span::raw(" ".repeat(area.width as usize - content_len)));
            }

            Line::from(spans)
        })
        .collect();

    frame.render_widget(Paragraph::new(visible_lines), area);
}

/// Draw git diff view mode
fn draw_diff_view(frame: &mut Frame, area: Rect, state: &FileViewerState, height: usize) {
    if state.diff_lines.is_empty() {
        let error_msg = vec![
            Line::from(""),
            Line::from(Span::styled(
                " No diff data available",
                Style::default().fg(COLOR_FG),
            )),
        ];
        frame.render_widget(Paragraph::new(error_msg), area);
        return;
    }

    // Calculate scroll
    let max_scroll = state.diff_lines.len().saturating_sub(height);
    let scroll = state.scroll_offset.min(max_scroll);

    // Color for cyan (hunk headers)
    let color_cyan = Color::Rgb(0, 170, 170);

    // Build visible lines with color coding
    let visible_lines: Vec<Line> = state
        .diff_lines
        .iter()
        .skip(scroll)
        .take(height)
        .map(|line| {
            let (style, prefix) = if line.starts_with('+') && !line.starts_with("+++") {
                // Added line - green
                (Style::default().fg(COLOR_GREEN), "+")
            } else if line.starts_with('-') && !line.starts_with("---") {
                // Removed line - red
                (Style::default().fg(COLOR_RED), "-")
            } else if line.starts_with("@@") {
                // Hunk header - cyan
                (Style::default().fg(color_cyan), "@")
            } else if line.starts_with("diff ") || line.starts_with("index ") {
                // File header - blue
                (Style::default().fg(COLOR_BLUE), " ")
            } else if line.starts_with("+++") || line.starts_with("---") {
                // File names - blue
                (Style::default().fg(COLOR_BLUE), " ")
            } else {
                // Context line
                (Style::default().fg(COLOR_FG), " ")
            };

            Line::from(vec![
                Span::styled(format!(" {} ", prefix), style),
                Span::styled(line.as_str(), style),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(visible_lines), area);
}
