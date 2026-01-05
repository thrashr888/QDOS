//! Apps launcher modal rendering
//!
//! UI for the F12 Apps launcher using FullScreenView.

use super::state::AppsState;
use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::Frame;

/// Draw the Apps launcher modal
pub fn draw_apps_modal(frame: &mut Frame, area: Rect, state: &AppsState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " F12 - RDOS Apps ", colors);
    view.render_frame(frame);

    let bg = colors.bg();
    let fg = colors.fg();
    let grey = colors.grey();
    let green = colors.green();
    let yellow = colors.yellow();
    let red = colors.red();
    let cyan = colors.cyan();

    let content_height = view.content_height() as usize;
    if content_height < 4 {
        return; // Not enough space to render
    }

    // Row 0: Search filter
    let filter_text = if state.filter.is_empty() {
        "Type to filter... (Shift+Key to quick launch)".to_string()
    } else {
        format!("Filter: {}_", state.filter)
    };
    view.render_row(
        frame,
        0,
        vec![Span::styled(
            format!(" {}", filter_text),
            Style::default()
                .fg(if state.filter.is_empty() { grey } else { fg })
                .bg(bg),
        )],
    );

    // Row 1: Empty separator
    view.render_row(frame, 1, vec![]);

    // Get filtered apps as flat list for scrolling
    let filtered = state.filtered_apps();
    let total_items = filtered.len();

    // Calculate how much space we have for apps (minus header rows and footer)
    let max_visible = content_height.saturating_sub(4);
    if max_visible == 0 {
        return;
    }

    // Calculate scroll offset to keep selection visible
    let scroll_offset = if state.selected_index >= max_visible {
        state.selected_index - max_visible + 1
    } else {
        0
    };

    let mut current_row: u16 = 2;

    // Render apps with scrolling (flat list, no category headers for simplicity with scroll)
    for (i, app) in filtered
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(max_visible)
    {
        if current_row as usize >= content_height.saturating_sub(2) {
            break;
        }

        let is_selected = i == state.selected_index;
        let prefix = if is_selected { ">" } else { " " };

        // Determine style based on enabled/available status
        let (style, key_style, status_indicator) = if is_selected {
            (
                Style::default().fg(yellow).bg(red),
                Style::default().fg(yellow).bg(red),
                if !app.enabled {
                    " [OFF]"
                } else if !app.available {
                    " [N/A]"
                } else {
                    ""
                },
            )
        } else if !app.enabled {
            (
                Style::default().fg(grey).bg(bg),
                Style::default().fg(grey).bg(bg),
                " [OFF]",
            )
        } else if !app.available {
            (
                Style::default().fg(grey).bg(bg),
                Style::default().fg(grey).bg(bg),
                " [N/A]",
            )
        } else {
            (
                Style::default().fg(fg).bg(bg),
                Style::default().fg(green).bg(bg),
                "",
            )
        };

        // Format: > K  Name                Description [status]
        let name_padded = format!("{:<16}", app.name);
        let max_desc_len = 30usize.saturating_sub(status_indicator.len());
        let desc_truncated = if app.description.len() > max_desc_len {
            format!("{}...", &app.description[..max_desc_len.saturating_sub(3)])
        } else {
            app.description.clone()
        };

        let status_style = if is_selected {
            Style::default().fg(yellow).bg(red)
        } else {
            Style::default().fg(cyan).bg(bg)
        };

        view.render_row(
            frame,
            current_row,
            vec![
                Span::styled(format!(" {} ", prefix), style),
                Span::styled(format!("Shift+{}  ", app.key), key_style),
                Span::styled(name_padded, style),
                Span::styled(desc_truncated, style),
                Span::styled(status_indicator.to_string(), status_style),
            ],
        );
        current_row += 1;
    }

    // Show count and scroll indicator near bottom
    let total_apps = state.apps.len();
    let filtered_apps = filtered.len();
    let enabled_count = state.apps.iter().filter(|a| a.enabled).count();

    let scroll_info = if total_items > max_visible {
        format!(" [{}/{}]", state.selected_index + 1, total_items)
    } else {
        String::new()
    };

    let count_text = if state.filter.is_empty() {
        format!(
            " {} apps ({} enabled){}",
            total_apps, enabled_count, scroll_info
        )
    } else {
        format!(" {} of {} apps{}", filtered_apps, total_apps, scroll_info)
    };
    let count_row = (content_height.saturating_sub(1) as u16).min(area.height.saturating_sub(3));
    view.render_row(
        frame,
        count_row,
        vec![Span::styled(count_text, Style::default().fg(grey).bg(bg))],
    );

    // Help footer
    view.render_help(
        frame,
        vec![
            ("Shift+Key", "launch"),
            ("Enter", "open"),
            ("Space", "toggle"),
            ("Type", "filter"),
            ("Esc", "close"),
        ],
    );
}
