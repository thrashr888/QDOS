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

    let filtered = state.filtered_apps();
    let content_height = view.content_height() as usize;

    // Row 0: Search filter
    let filter_text = if state.filter.is_empty() {
        "Type to filter...".to_string()
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

    // Row 2: Column headers
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            "   Key  Name                Description",
            Style::default().fg(grey).bg(bg),
        )],
    );

    // Rows 3+: App list
    let max_visible = content_height.saturating_sub(4); // header rows + count row
    for (i, app) in filtered.iter().take(max_visible).enumerate() {
        let is_selected = i == state.selected_index;
        let prefix = if is_selected { ">" } else { " " };

        let style = if is_selected {
            Style::default().fg(yellow).bg(red)
        } else if !app.available {
            Style::default().fg(grey).bg(bg)
        } else {
            Style::default().fg(fg).bg(bg)
        };

        let key_style = if is_selected {
            Style::default().fg(yellow).bg(red)
        } else {
            Style::default().fg(green).bg(bg)
        };

        // Format: > K  Name                Description
        let name_padded = format!("{:<18}", app.name);
        let desc_truncated = if app.description.len() > 35 {
            format!("{}...", &app.description[..32])
        } else {
            app.description.clone()
        };

        let row = 3 + i as u16;
        view.render_row(
            frame,
            row,
            vec![
                Span::styled(format!(" {} ", prefix), style),
                Span::styled(format!("{}  ", app.key), key_style),
                Span::styled(name_padded, style),
                Span::styled(desc_truncated, style),
            ],
        );
    }

    // Show count near bottom
    let count_text = if state.filter.is_empty() {
        format!(" {} apps", filtered.len())
    } else {
        format!(" {} of {} apps", filtered.len(), state.apps.len())
    };
    let count_row = content_height.saturating_sub(1) as u16;
    view.render_row(
        frame,
        count_row,
        vec![Span::styled(count_text, Style::default().fg(grey).bg(bg))],
    );

    // Help footer
    view.render_help(
        frame,
        vec![
            ("A-Z", "launch"),
            ("Enter", "open"),
            ("Type", "filter"),
            ("Esc", "close"),
        ],
    );
}
