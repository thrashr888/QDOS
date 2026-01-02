//! File viewer UI components
//!
//! This module contains all file viewer drawing functions including:
//! - Normal/ASCII view with syntax highlighting
//! - Hex view
//! - Image view
//! - Markdown view
//! - Shell command view

use crate::app::{App, FileViewerState, ShellCommandState, ViewFilter, ViewMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

use super::{COLOR_BLUE, COLOR_FG, COLOR_GREEN, COLOR_RED};

// Lazy-loaded syntax highlighting resources
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

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
    };
    let filter_str = match state.filter {
        ViewFilter::Off => "",
        ViewFilter::Ascii => " [Filter: ASCII]",
        ViewFilter::WordStar => " [Filter: W/S]",
    };
    let title = format!(
        " VIEW: {}  Mode: {}{}",
        state.file_name.to_uppercase(),
        mode_str,
        filter_str
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
    }

    // Separator
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(COLOR_FG))),
        chunks[3],
    );

    // Help line
    let help_spans = vec![
        Span::styled(" H", Style::default().fg(COLOR_BLUE)),
        Span::raw("ex "),
        Span::styled("N", Style::default().fg(COLOR_BLUE)),
        Span::raw("ormal "),
        Span::styled("I", Style::default().fg(COLOR_BLUE)),
        Span::raw("mage "),
        Span::styled("M", Style::default().fg(COLOR_BLUE)),
        Span::raw("arkdown "),
        Span::styled("F", Style::default().fg(COLOR_BLUE)),
        Span::raw("ilter "),
        Span::styled("↑↓", Style::default().fg(COLOR_BLUE)),
        Span::raw(" scroll "),
        Span::styled("Esc", Style::default().fg(COLOR_BLUE)),
        Span::raw(" exit"),
    ];
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

/// Draw shell command screen (full screen)
pub(super) fn draw_shell_command(
    frame: &mut Frame,
    area: Rect,
    state: &ShellCommandState,
    app: &App,
) {
    // Clear the entire screen
    frame.render_widget(Clear, area);

    // Layout: title, separator, working dir, input, separator, output, separator, help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(1), // Separator
            Constraint::Length(1), // Working directory
            Constraint::Length(1), // Empty
            Constraint::Length(1), // Input prompt
            Constraint::Length(1), // Separator
            Constraint::Min(5),    // Output area
            Constraint::Length(1), // Separator
            Constraint::Length(1), // Help line
        ])
        .split(area);

    // Title (centered)
    let title = "R-DOS Shell Command";
    let padding = (area.width as usize).saturating_sub(title.len()) / 2;
    let title_line = format!("{:>width$}{}", "", title, width = padding);
    frame.render_widget(
        Paragraph::new(Span::styled(
            title_line,
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

    // Working directory
    let wd_line = format!(" Working Directory: {}", app.current_path.display());
    frame.render_widget(
        Paragraph::new(Span::styled(wd_line, Style::default().fg(COLOR_GREEN))),
        chunks[2],
    );

    // Input prompt with cursor
    let input_line = format!(" $ {}_", state.input);
    frame.render_widget(
        Paragraph::new(Span::styled(input_line, Style::default().fg(COLOR_FG))),
        chunks[4],
    );

    // Separator
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(COLOR_FG))),
        chunks[5],
    );

    // Output area
    let output_height = chunks[6].height as usize;
    let visible_lines: Vec<Line> = state
        .output
        .iter()
        .skip(state.scroll_offset)
        .take(output_height)
        .map(|line| {
            let style = if line.starts_with("stderr:") {
                Style::default().fg(COLOR_RED)
            } else {
                Style::default().fg(COLOR_FG)
            };
            Line::from(Span::styled(format!(" {}", line), style))
        })
        .collect();

    // Show exit code at bottom if command completed
    let mut output_lines = visible_lines;
    if let Some(code) = state.exit_code {
        if output_lines.len() < output_height {
            output_lines.push(Line::from(""));
            let exit_style = if code == 0 {
                Style::default().fg(COLOR_GREEN)
            } else {
                Style::default().fg(COLOR_RED)
            };
            output_lines.push(Line::from(Span::styled(
                format!(" [Exit code: {}]", code),
                exit_style,
            )));
        }
    }

    frame.render_widget(Paragraph::new(output_lines), chunks[6]);

    // Separator
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(COLOR_FG))),
        chunks[7],
    );

    // Help line
    let help_spans = vec![
        Span::styled(" Enter", Style::default().fg(COLOR_BLUE)),
        Span::raw(" run, "),
        Span::styled("↑/↓", Style::default().fg(COLOR_BLUE)),
        Span::raw(" history, "),
        Span::styled("PgUp/PgDn", Style::default().fg(COLOR_BLUE)),
        Span::raw(" scroll, "),
        Span::styled("Tab", Style::default().fg(COLOR_BLUE)),
        Span::raw(" complete, "),
        Span::styled("Esc", Style::default().fg(COLOR_BLUE)),
        Span::raw(" exit"),
    ];
    frame.render_widget(Paragraph::new(Line::from(help_spans)), chunks[8]);
}

/// Draw image view mode
fn draw_image_view(frame: &mut Frame, area: Rect, state: &FileViewerState) {
    // Try to load and display the image
    match image::load_from_memory(&state.content) {
        Ok(img) => {
            // Get the image dimensions
            let img_width = img.width();
            let img_height = img.height();

            // Calculate the aspect ratio and fit to area
            // Terminal characters are typically 2:1 height:width ratio
            let term_aspect = (area.width as f64) / (area.height as f64 * 2.0);
            let img_aspect = img_width as f64 / img_height as f64;

            let (display_width, display_height) = if img_aspect > term_aspect {
                // Image is wider than area
                let w = area.width as u32;
                let h = ((w as f64 / img_aspect) / 2.0).max(1.0) as u32;
                (w, h.min(area.height as u32))
            } else {
                // Image is taller than area
                let h = area.height as u32;
                let w = (h as f64 * img_aspect * 2.0).max(1.0) as u32;
                (w.min(area.width as u32), h)
            };

            // Center the image in the area
            let x_offset = (area.width.saturating_sub(display_width as u16)) / 2;
            let y_offset = (area.height.saturating_sub(display_height as u16)) / 2;

            // Convert image to RGBA and resize
            let rgba_img = img.to_rgba8();
            let resized = image::imageops::resize(
                &rgba_img,
                display_width,
                display_height,
                image::imageops::FilterType::Triangle,
            );

            // Render as half-block characters (upper half, lower half)
            // Each character cell represents 2 vertical pixels
            let mut lines: Vec<Line> = Vec::new();

            for y in (0..display_height).step_by(2) {
                let mut spans: Vec<Span> = Vec::new();

                // Add left padding
                if x_offset > 0 {
                    spans.push(Span::raw(" ".repeat(x_offset as usize)));
                }

                for x in 0..display_width {
                    let top_pixel = resized.get_pixel(x, y);
                    let bottom_pixel = if y + 1 < display_height {
                        resized.get_pixel(x, y + 1)
                    } else {
                        top_pixel
                    };

                    // Use half-block character with top color as foreground, bottom as background
                    let fg = Color::Rgb(top_pixel[0], top_pixel[1], top_pixel[2]);
                    let bg = Color::Rgb(bottom_pixel[0], bottom_pixel[1], bottom_pixel[2]);

                    spans.push(Span::styled("\u{2580}", Style::default().fg(fg).bg(bg)));
                }

                lines.push(Line::from(spans));
            }

            // Add top padding
            let mut padded_lines: Vec<Line> = Vec::new();
            for _ in 0..y_offset {
                padded_lines.push(Line::from(""));
            }
            padded_lines.extend(lines);

            frame.render_widget(Paragraph::new(padded_lines), area);
        }
        Err(e) => {
            // Show error if image can't be loaded
            let error_msg = vec![
                Line::from(""),
                Line::from(Span::styled(
                    " Cannot display image",
                    Style::default().fg(COLOR_RED).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!(" Error: {}", e),
                    Style::default().fg(COLOR_FG),
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
