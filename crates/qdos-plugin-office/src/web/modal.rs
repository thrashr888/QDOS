//! Q-WEB Modal Rendering
//!
//! Renders the text browser interface.

use super::state::{WebMode, WebState};
use qdos_plugin_api::ui::{FullScreenView, ModalFrame};
use qdos_plugin_api::ThemeColors;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

// =============================================================================
// MAIN DRAW FUNCTION
// =============================================================================

pub fn draw_web_modal(frame: &mut Frame, area: Rect, state: &WebState, colors: &ThemeColors) {
    match state.mode {
        WebMode::Bookmarks => draw_bookmarks(frame, area, state, colors),
        WebMode::History => draw_history(frame, area, state, colors),
        WebMode::SaveAs => draw_save_as(frame, area, state, colors),
        _ => draw_browse_mode(frame, area, state, colors),
    }
}

// =============================================================================
// BROWSE MODE
// =============================================================================

fn draw_browse_mode(frame: &mut Frame, area: Rect, state: &WebState, colors: &ThemeColors) {
    let page = state.current_page();
    let title = format!(
        " Q-WEB: {} ",
        if page.title.len() > 50 {
            format!("{}...", &page.title[..47])
        } else {
            page.title.clone()
        }
    );

    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    let content_area = view.content_area();

    // Status bar (first row)
    let status_style = Style::default().fg(colors.green());
    let url_style = Style::default().fg(colors.cyan());

    // URL bar with mode indicator
    let mode_indicator = match state.mode {
        WebMode::UrlInput => " [GOTO URL] ",
        WebMode::Search => " [SEARCH] ",
        WebMode::Loading => " [LOADING] ",
        _ => "",
    };

    let mut status_spans = vec![];

    if !mode_indicator.is_empty() {
        status_spans.push(Span::styled(
            mode_indicator,
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        ));
    }

    match state.mode {
        WebMode::UrlInput => {
            status_spans.push(Span::styled(&state.url_input, url_style));
            status_spans.push(Span::styled("_", Style::default().fg(colors.yellow())));
        }
        WebMode::Search => {
            status_spans.push(Span::styled("/", status_style));
            status_spans.push(Span::styled(&state.search_query, url_style));
            status_spans.push(Span::styled("_", Style::default().fg(colors.yellow())));
        }
        _ => {
            status_spans.push(Span::styled(&page.url, url_style));
        }
    }

    view.render_row(frame, 0, status_spans);

    // Separator
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            "─".repeat(content_area.width as usize),
            Style::default().fg(colors.grey()),
        )],
    );

    // Page content
    let content_height = (content_area.height as usize).saturating_sub(4);
    let text_style = Style::default().fg(colors.fg());
    let link_style = Style::default()
        .fg(colors.cyan())
        .add_modifier(Modifier::UNDERLINED);
    let selected_link_style = Style::default()
        .fg(colors.yellow())
        .bg(colors.red())
        .add_modifier(Modifier::BOLD);

    for (i, line_idx) in (page.scroll..page.scroll + content_height).enumerate() {
        if line_idx >= page.content.len() {
            break;
        }

        let line = &page.content[line_idx];

        // Check if this line contains a selected link (check both links and nav_links)
        let has_selected_link = if let Some(selected_link) = state.selected_link_info() {
            selected_link.line == line_idx
        } else {
            false
        };

        // Check if this line contains any link (for styling)
        let has_any_link = page.links.iter().any(|l| l.line == line_idx)
            || page.nav_links.iter().any(|l| l.line == line_idx);

        let style = if has_selected_link {
            selected_link_style
        } else if has_any_link || (line.trim_start().starts_with('[') && line.contains(']')) {
            link_style
        } else {
            text_style
        };

        view.render_row(
            frame,
            (i + 2) as u16,
            vec![Span::styled(line.clone(), style)],
        );
    }

    // Status bar: show selected link URL or general info
    let info_row = content_area.height.saturating_sub(2);
    if let Some((msg, _)) = &state.status_message {
        // Temporary status message (e.g., "12 links")
        view.render_row(
            frame,
            info_row,
            vec![Span::styled(
                format!(" {} ", msg),
                Style::default().fg(colors.yellow()),
            )],
        );
    } else if let Some(link) = state.selected_link_info() {
        // Show selected link URL prominently
        let max_url_width = content_area.width.saturating_sub(4) as usize;
        let display_url = if link.url.len() > max_url_width {
            format!("{}...", &link.url[..max_url_width.saturating_sub(3)])
        } else {
            link.url.clone()
        };
        view.render_row(
            frame,
            info_row,
            vec![
                Span::styled(" -> ", Style::default().fg(colors.green())),
                Span::styled(display_url, Style::default().fg(colors.cyan())),
            ],
        );
    } else {
        // Default info bar
        let info = format!(
            " Links: {}  Mode: {}  Tabs: {}/{}  [{}%] ",
            state.link_count(),
            state.render_mode.name(),
            state.active_tab + 1,
            state.tabs.len(),
            if page.content.is_empty() {
                0
            } else {
                (page.scroll * 100) / page.content.len().max(1)
            }
        );
        view.render_row(
            frame,
            info_row,
            vec![Span::styled(info, Style::default().fg(colors.grey()))],
        );
    }

    // Help footer
    view.render_help(
        frame,
        vec![
            ("G", "goto"),
            ("Tab", "link"),
            ("[/]", "back/fwd"),
            ("B", "bookmarks"),
            ("/", "search"),
            ("Esc", "close"),
        ],
    );
}

// =============================================================================
// BOOKMARKS VIEW
// =============================================================================

fn draw_bookmarks(frame: &mut Frame, area: Rect, state: &WebState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-WEB: Bookmarks ", colors);
    view.render_frame(frame);

    let selected_style = Style::default()
        .fg(colors.yellow())
        .bg(colors.red())
        .add_modifier(Modifier::BOLD);
    let normal_style = Style::default().fg(colors.fg());
    let url_style = Style::default().fg(colors.cyan());

    if state.bookmarks.is_empty() {
        view.render_row(
            frame,
            0,
            vec![Span::styled(
                "  No bookmarks yet. Press Ctrl+A on a page to add one.",
                Style::default().fg(colors.grey()),
            )],
        );
    } else {
        for (i, bookmark) in state.bookmarks.iter().enumerate() {
            let is_selected = i == state.bookmarks_selected;
            let style = if is_selected {
                selected_style
            } else {
                normal_style
            };

            let prefix = if is_selected { ">" } else { " " };
            let line = format!("{} {}. {}", prefix, i + 1, bookmark.title);

            view.render_row(frame, (i * 2) as u16, vec![Span::styled(line, style)]);
            view.render_row(
                frame,
                (i * 2 + 1) as u16,
                vec![Span::styled(format!("     {}", bookmark.url), url_style)],
            );
        }
    }

    view.render_help(
        frame,
        vec![("Enter", "open"), ("Del", "remove"), ("Esc", "back")],
    );
}

// =============================================================================
// HISTORY VIEW
// =============================================================================

fn draw_history(frame: &mut Frame, area: Rect, state: &WebState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-WEB: History ", colors);
    view.render_frame(frame);

    let selected_style = Style::default()
        .fg(colors.yellow())
        .bg(colors.red())
        .add_modifier(Modifier::BOLD);
    let normal_style = Style::default().fg(colors.fg());
    let url_style = Style::default().fg(colors.cyan());

    if state.global_history.is_empty() {
        view.render_row(
            frame,
            0,
            vec![Span::styled(
                "  No history yet.",
                Style::default().fg(colors.grey()),
            )],
        );
    } else {
        for (i, entry) in state.global_history.iter().enumerate() {
            if i >= 20 {
                break;
            }

            let is_selected = i == state.history_selected;
            let style = if is_selected {
                selected_style
            } else {
                normal_style
            };

            let prefix = if is_selected { ">" } else { " " };
            let title = if entry.title.is_empty() {
                "(Untitled)"
            } else {
                &entry.title
            };
            let line = format!("{} {}", prefix, title);

            view.render_row(frame, (i * 2) as u16, vec![Span::styled(line, style)]);
            view.render_row(
                frame,
                (i * 2 + 1) as u16,
                vec![Span::styled(format!("    {}", entry.url), url_style)],
            );
        }
    }

    view.render_help(frame, vec![("Enter", "open"), ("Esc", "back")]);
}

// =============================================================================
// SAVE AS DIALOG
// =============================================================================

fn draw_save_as(frame: &mut Frame, area: Rect, state: &WebState, colors: &ThemeColors) {
    let width = area.width.min(60);
    let height = 10;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    let modal = ModalFrame::themed(modal_area, " SAVE PAGE ", colors);
    modal.render_frame(frame);

    let grey = Style::default().fg(colors.grey());
    let label = Style::default().fg(colors.green());
    let input_style = Style::default().fg(colors.yellow()).bg(colors.red());

    modal.render_row(
        frame,
        0,
        vec![
            Span::styled("Page: ", grey),
            Span::styled(
                if state.display_title().len() > 40 {
                    format!("{}...", &state.display_title()[..37])
                } else {
                    state.display_title().to_string()
                },
                Style::default().fg(colors.fg()),
            ),
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
        vec![Span::styled("(Use .txt for text, .html for source)", grey)],
    );

    modal.render_help(frame, vec![("Enter", "save"), ("Esc", "cancel")]);
}
