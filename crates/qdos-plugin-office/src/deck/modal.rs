//! Q-DECK Modal Rendering
//!
//! Renders the presentation editor and presentation mode.

use super::image;
use super::state::{ContentBlock, DeckMode, DeckState, SlideTemplate};
use qdos_plugin_api::ui::{FullScreenView, ModalFrame};
use qdos_plugin_api::ThemeColors;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};
use std::path::Path;

// =============================================================================
// MAIN DRAW FUNCTION
// =============================================================================

pub fn draw_deck_modal(frame: &mut Frame, area: Rect, state: &DeckState, colors: &ThemeColors) {
    match state.mode {
        DeckMode::Present => draw_present_mode(frame, area, state, colors),
        DeckMode::SlideList => draw_slide_list(frame, area, state, colors),
        DeckMode::SaveAs => draw_save_as_dialog(frame, area, state, colors),
        _ => draw_edit_mode(frame, area, state, colors),
    }
}

// =============================================================================
// EDIT MODE
// =============================================================================

fn draw_edit_mode(frame: &mut Frame, area: Rect, state: &DeckState, colors: &ThemeColors) {
    let modified_marker = if state.modified { " [*]" } else { "" };
    let title = format!(
        " Q-DECK: {}{}  {} ",
        state.display_name(),
        modified_marker,
        state.slide_indicator()
    );

    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    let content = view.content_area();
    let slide = state.current();

    // Calculate slide preview area (centered box)
    let preview_width = content.width.saturating_sub(4).min(76);
    let preview_height = content.height.saturating_sub(6).min(20);
    let preview_x = (content.width.saturating_sub(preview_width)) / 2;

    // Draw slide border
    let border_style = Style::default().fg(colors.grey());
    let title_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());

    // Top border of slide preview
    let top_border = format!(
        "{}{}{}",
        " ".repeat(preview_x as usize),
        "",
        "".repeat(preview_width as usize - 2),
    );
    view.render_row(frame, 0, vec![Span::styled(top_border, border_style)]);

    // Draw slide content
    let mut row = 1;

    // Slide title
    let centered_title = center_text(&slide.title, preview_width as usize - 4);
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(" ".repeat(preview_x as usize + 1), border_style),
            Span::styled(centered_title, title_style),
        ],
    );
    row += 2;

    // Subtitle if present
    if let Some(ref subtitle) = slide.subtitle {
        let centered_sub = center_text(subtitle, preview_width as usize - 4);
        view.render_row(
            frame,
            row,
            vec![
                Span::styled(" ".repeat(preview_x as usize + 1), border_style),
                Span::styled(centered_sub, Style::default().fg(colors.cyan())),
            ],
        );
        row += 2;
    }

    // Content blocks
    for block in &slide.content {
        match block {
            ContentBlock::Bullets(items) => {
                for item in items {
                    if row >= preview_height {
                        break;
                    }
                    let bullet_text = format!("  * {}", item);
                    view.render_row(
                        frame,
                        row,
                        vec![
                            Span::styled(" ".repeat(preview_x as usize + 1), border_style),
                            Span::styled(bullet_text, text_style),
                        ],
                    );
                    row += 1;
                }
            }
            ContentBlock::Text { content, bold, .. } => {
                let style = if *bold {
                    text_style.add_modifier(Modifier::BOLD)
                } else {
                    text_style
                };
                view.render_row(
                    frame,
                    row,
                    vec![
                        Span::styled(" ".repeat(preview_x as usize + 1), border_style),
                        Span::styled(format!("  {}", content), style),
                    ],
                );
                row += 1;
            }
            ContentBlock::Code { content, .. } => {
                let code_style = Style::default().fg(colors.green());
                for line in content.lines().take(5) {
                    if row >= preview_height {
                        break;
                    }
                    view.render_row(
                        frame,
                        row,
                        vec![
                            Span::styled(" ".repeat(preview_x as usize + 1), border_style),
                            Span::styled(format!("  {}", line), code_style),
                        ],
                    );
                    row += 1;
                }
            }
            ContentBlock::AnsiArt(art) => {
                for line in art.lines().take(8) {
                    if row >= preview_height {
                        break;
                    }
                    view.render_row(
                        frame,
                        row,
                        vec![
                            Span::styled(" ".repeat(preview_x as usize + 1), border_style),
                            Span::styled(format!("  {}", line), text_style),
                        ],
                    );
                    row += 1;
                }
            }
            ContentBlock::Quote { text, author } => {
                let quote_style = Style::default()
                    .fg(colors.cyan())
                    .add_modifier(Modifier::ITALIC);
                view.render_row(
                    frame,
                    row,
                    vec![
                        Span::styled(" ".repeat(preview_x as usize + 1), border_style),
                        Span::styled(format!("  \"{}\"", text), quote_style),
                    ],
                );
                row += 1;
                view.render_row(
                    frame,
                    row,
                    vec![
                        Span::styled(" ".repeat(preview_x as usize + 1), border_style),
                        Span::styled(format!("    - {}", author), text_style),
                    ],
                );
                row += 1;
            }
            ContentBlock::Image { alt, .. } => {
                // Show placeholder for image in edit mode
                let img_style = Style::default().fg(colors.grey());
                view.render_row(
                    frame,
                    row,
                    vec![
                        Span::styled(" ".repeat(preview_x as usize + 1), border_style),
                        Span::styled(format!("  [Image: {}]", alt), img_style),
                    ],
                );
                row += 1;
            }
            ContentBlock::Separator => {
                let sep = "".repeat((preview_width as usize).saturating_sub(8));
                view.render_row(
                    frame,
                    row,
                    vec![
                        Span::styled(" ".repeat(preview_x as usize + 1), border_style),
                        Span::styled(format!("  {}", sep), border_style),
                    ],
                );
                row += 1;
            }
            ContentBlock::Numbered(items) => {
                for (i, item) in items.iter().enumerate() {
                    if row >= preview_height {
                        break;
                    }
                    let num_text = format!("  {}. {}", i + 1, item);
                    view.render_row(
                        frame,
                        row,
                        vec![
                            Span::styled(" ".repeat(preview_x as usize + 1), border_style),
                            Span::styled(num_text, text_style),
                        ],
                    );
                    row += 1;
                }
            }
        }
    }

    // Template bar
    let template_row = preview_height + 1;
    let template_style = Style::default().fg(colors.grey());
    let selected_template = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    let mut template_spans = vec![Span::styled(" ", template_style)];
    for template in SlideTemplate::all() {
        let is_selected = *template == slide.template;
        let style = if is_selected {
            selected_template
        } else {
            template_style
        };
        template_spans.push(Span::styled(format!("[{}] ", template.name()), style));
    }
    template_spans.push(Span::styled(
        format!("    Theme: {}", state.theme.name),
        Style::default().fg(colors.cyan()),
    ));

    view.render_row(frame, template_row, template_spans);

    // Status bar
    let status_row = template_row + 1;
    let status = if let Some((msg, _)) = &state.status_message {
        msg.clone()
    } else {
        format!(
            "Template: {}  Timer: {}",
            slide.template.name(),
            state.format_timer()
        )
    };
    view.render_row(
        frame,
        status_row,
        vec![Span::styled(
            format!(" {} ", status),
            Style::default().fg(colors.green()),
        )],
    );

    // Help row
    let help = vec![
        ("F5", "present"),
        ("</>", "slides"),
        ("Ins", "new"),
        ("Del", "delete"),
        ("Tab", "template"),
        ("Esc", "close"),
    ];
    view.render_help(frame, help);
}

// =============================================================================
// PRESENTATION MODE
// =============================================================================

fn draw_present_mode(frame: &mut Frame, area: Rect, state: &DeckState, colors: &ThemeColors) {
    // Full black background
    let bg_style = Style::default().bg(colors.bg());

    // Clear the entire area
    for y in area.y..area.y + area.height {
        frame.render_widget(
            ratatui::widgets::Paragraph::new(" ".repeat(area.width as usize)).style(bg_style),
            Rect::new(area.x, y, area.width, 1),
        );
    }

    let slide = state.current();
    let title_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());
    let bullet_style = Style::default().fg(colors.cyan());

    // Center content vertically
    let content_height = calculate_slide_height(slide);
    let start_y = area.y + (area.height.saturating_sub(content_height as u16)) / 2;

    let mut y = start_y;

    // Title (always centered)
    let title_text = center_text(&slide.title, area.width as usize - 4);
    frame.render_widget(
        ratatui::widgets::Paragraph::new(title_text).style(title_style),
        Rect::new(area.x + 2, y, area.width - 4, 1),
    );
    y += 2;

    // Subtitle
    if let Some(ref subtitle) = slide.subtitle {
        let sub_text = center_text(subtitle, area.width as usize - 4);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(sub_text).style(Style::default().fg(colors.cyan())),
            Rect::new(area.x + 2, y, area.width - 4, 1),
        );
        y += 2;
    }

    // Content
    for block in &slide.content {
        match block {
            ContentBlock::Bullets(items) => {
                for item in items {
                    if y >= area.y + area.height - 2 {
                        break;
                    }
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(format!("    * {}", item))
                            .style(bullet_style),
                        Rect::new(area.x + 4, y, area.width - 8, 1),
                    );
                    y += 1;
                }
                y += 1;
            }
            ContentBlock::Text { content, bold, .. } => {
                let style = if *bold {
                    text_style.add_modifier(Modifier::BOLD)
                } else {
                    text_style
                };
                frame.render_widget(
                    ratatui::widgets::Paragraph::new(format!("    {}", content)).style(style),
                    Rect::new(area.x + 4, y, area.width - 8, 1),
                );
                y += 1;
            }
            ContentBlock::Code { content, .. } => {
                let code_style = Style::default().fg(colors.green());
                y += 1;
                for line in content.lines() {
                    if y >= area.y + area.height - 2 {
                        break;
                    }
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(format!("    {}", line)).style(code_style),
                        Rect::new(area.x + 4, y, area.width - 8, 1),
                    );
                    y += 1;
                }
                y += 1;
            }
            ContentBlock::AnsiArt(art) => {
                for line in art.lines() {
                    if y >= area.y + area.height - 2 {
                        break;
                    }
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(line.to_string()).style(text_style),
                        Rect::new(area.x + 2, y, area.width - 4, 1),
                    );
                    y += 1;
                }
            }
            ContentBlock::Quote { text, author } => {
                let quote_style = Style::default()
                    .fg(colors.cyan())
                    .add_modifier(Modifier::ITALIC);
                y += 1;
                let centered_quote = center_text(&format!("\"{}\"", text), area.width as usize - 8);
                frame.render_widget(
                    ratatui::widgets::Paragraph::new(centered_quote).style(quote_style),
                    Rect::new(area.x + 4, y, area.width - 8, 1),
                );
                y += 2;
                let centered_author =
                    center_text(&format!("- {}", author), area.width as usize - 8);
                frame.render_widget(
                    ratatui::widgets::Paragraph::new(centered_author).style(text_style),
                    Rect::new(area.x + 4, y, area.width - 8, 1),
                );
                y += 1;
            }
            ContentBlock::Image { path, alt } => {
                // Calculate image area (centered, leaving space for title and indicator)
                let img_height = (area.height as usize).saturating_sub(y as usize + 3);
                let img_width = area.width.saturating_sub(8);
                let img_x = area.x + 4;
                let img_area = Rect::new(img_x, y, img_width, img_height as u16);

                // Get base directory from state's file_path or use current dir
                let base_dir = state
                    .file_path
                    .as_ref()
                    .and_then(|p| p.parent())
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

                // Try to render the image via sixel/kitty
                if !image::render_image(frame, img_area, path, &base_dir) {
                    // Fallback: show placeholder if image can't be rendered
                    let placeholder = format!("[Image: {}]", alt);
                    let centered = center_text(&placeholder, area.width as usize - 8);
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(centered)
                            .style(Style::default().fg(colors.grey())),
                        Rect::new(area.x + 4, y + img_height as u16 / 2, area.width - 8, 1),
                    );
                }
                y += img_height as u16;
            }
            ContentBlock::Numbered(_) | ContentBlock::Separator => {
                y += 1;
            }
        }
    }

    // Slide indicator at bottom
    let indicator = format!(
        " {} / {}  {}  Press Esc to exit ",
        state.current_slide + 1,
        state.slides.len(),
        state.format_timer()
    );
    let indicator_len = indicator.len() as u16;
    let indicator_x = area.x + (area.width.saturating_sub(indicator_len)) / 2;
    frame.render_widget(
        ratatui::widgets::Paragraph::new(indicator).style(Style::default().fg(colors.grey())),
        Rect::new(indicator_x, area.y + area.height - 1, indicator_len, 1),
    );
}

// =============================================================================
// SLIDE LIST MODE
// =============================================================================

fn draw_slide_list(frame: &mut Frame, area: Rect, state: &DeckState, colors: &ThemeColors) {
    let title = format!(" Q-DECK: {} - Slide Sorter ", state.display_name());
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    let selected_style = Style::default()
        .fg(colors.yellow())
        .bg(colors.red())
        .add_modifier(Modifier::BOLD);
    let normal_style = Style::default().fg(colors.fg());

    for (i, slide) in state.slides.iter().enumerate() {
        if i >= (area.height - 4) as usize {
            break;
        }

        let is_selected = i == state.current_slide;
        let style = if is_selected {
            selected_style
        } else {
            normal_style
        };

        let prefix = if is_selected { ">" } else { " " };
        let template_name = slide.template.name();
        let line = format!(
            "{} {:2}. [{}] {}",
            prefix,
            i + 1,
            template_name,
            truncate(&slide.title, 50)
        );

        view.render_row(frame, i as u16, vec![Span::styled(line, style)]);
    }

    view.render_help(
        frame,
        vec![
            ("Up/Down", "select"),
            ("Enter", "edit"),
            ("Ins", "new"),
            ("Del", "delete"),
            ("Esc", "back"),
        ],
    );
}

// =============================================================================
// SAVE AS DIALOG
// =============================================================================

fn draw_save_as_dialog(frame: &mut Frame, area: Rect, state: &DeckState, colors: &ThemeColors) {
    let width = area.width.min(60);
    let height = 10;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    let modal = ModalFrame::themed(modal_area, " SAVE PRESENTATION ", colors);
    modal.render_frame(frame);

    let grey = Style::default().fg(colors.grey());
    let normal = Style::default().fg(colors.fg());
    let label = Style::default().fg(colors.green());
    let input_style = Style::default().fg(colors.yellow()).bg(colors.red());

    modal.render_row(
        frame,
        0,
        vec![
            Span::styled("Current: ", grey),
            Span::styled(state.display_name(), normal),
        ],
    );

    let input_display = state.save_as_input.clone();
    modal.render_row(
        frame,
        2,
        vec![
            Span::styled("Filename: ", label),
            Span::styled(input_display, input_style),
        ],
    );

    modal.render_row(
        frame,
        4,
        vec![Span::styled("(Extension: .qdeck for presentations)", grey)],
    );

    modal.render_help(frame, vec![("Enter", "save"), ("Esc", "cancel")]);
}

// =============================================================================
// HELPERS
// =============================================================================

fn center_text(text: &str, width: usize) -> String {
    if text.len() >= width {
        text.to_string()
    } else {
        let padding = (width - text.len()) / 2;
        format!("{}{}", " ".repeat(padding), text)
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}

fn calculate_slide_height(slide: &super::state::Slide) -> usize {
    let mut height = 2; // Title
    if slide.subtitle.is_some() {
        height += 2;
    }
    for block in &slide.content {
        height += match block {
            ContentBlock::Bullets(items) => items.len() + 1,
            ContentBlock::Numbered(items) => items.len() + 1,
            ContentBlock::Code { content, .. } => content.lines().count() + 2,
            ContentBlock::AnsiArt(art) => art.lines().count(),
            ContentBlock::Quote { .. } => 4,
            _ => 1,
        };
    }
    height
}
