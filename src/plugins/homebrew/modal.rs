//! Homebrew modal rendering
//!
//! UI for the Homebrew modal using ModalFrame.

use super::state::{HomebrewState, HomebrewView, PackageStatus};
use crate::app::ThemeColors;
use crate::ui::components::ModalFrame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::Frame;

/// Draw the Homebrew modal
pub fn draw_homebrew_modal(
    frame: &mut Frame,
    area: Rect,
    state: &HomebrewState,
    colors: &ThemeColors,
) {
    // Calculate centered modal area
    let popup_width = 75u16.min(area.width.saturating_sub(4));
    let popup_height = 22u16.min(area.height.saturating_sub(4));
    let popup_x = (area.width - popup_width) / 2;
    let popup_y = (area.height - popup_height) / 2;
    let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let title = if state.search_query.is_empty() {
        " Homebrew Packages ".to_string()
    } else {
        format!(" Homebrew: {} ", state.search_query)
    };

    let modal = ModalFrame::themed(modal_area, &title, colors);
    modal.render_frame(frame);

    let bg = colors.bg();
    let fg = colors.fg();
    let grey = colors.grey();
    let green = colors.green();
    let yellow = colors.yellow();
    let red = colors.red();
    let cyan = colors.cyan();

    let visible_height = modal.content_height() as usize;

    // Check if Homebrew is available
    if !state.homebrew_available {
        modal.render_row(
            frame,
            1,
            vec![Span::styled(
                " Homebrew not found. Install from https://brew.sh",
                Style::default().fg(red).bg(bg),
            )],
        );
        modal.render_help(frame, vec![("Esc", "close")]);
        return;
    }

    // Loading state
    if state.loading {
        modal.render_row(
            frame,
            1,
            vec![Span::styled(
                " Loading packages...",
                Style::default().fg(grey).bg(bg),
            )],
        );
        return;
    }

    // Error state
    if let Some(ref error) = state.error {
        modal.render_row(
            frame,
            1,
            vec![Span::styled(
                format!(" Error: {}", error),
                Style::default().fg(red).bg(bg),
            )],
        );
        modal.render_help(frame, vec![("R", "retry"), ("Esc", "close")]);
        return;
    }

    // Header row
    modal.render_row(
        frame,
        0,
        vec![Span::styled(
            " S Name                         Version     Description",
            Style::default().fg(grey).bg(bg),
        )],
    );

    // Package list
    let filtered = state.filtered_packages();
    if filtered.is_empty() {
        modal.render_row(
            frame,
            2,
            vec![Span::styled(
                if state.search_query.is_empty() {
                    " No packages loaded. Press R to refresh."
                } else {
                    " No matching packages found"
                },
                Style::default().fg(grey).bg(bg),
            )],
        );
    } else {
        let max_visible = visible_height.saturating_sub(2);
        let scroll_offset = if state.selected_index >= max_visible {
            state.selected_index - max_visible + 1
        } else {
            0
        };

        for (i, pkg) in filtered
            .iter()
            .skip(scroll_offset)
            .take(max_visible)
            .enumerate()
        {
            let actual_index = i + scroll_offset;
            let is_selected = actual_index == state.selected_index;
            let prefix = if is_selected { ">" } else { " " };

            let style = if is_selected {
                Style::default().fg(yellow).bg(red)
            } else {
                Style::default().fg(fg).bg(bg)
            };

            let status_style = if is_selected {
                Style::default().fg(yellow).bg(red)
            } else {
                match pkg.status {
                    PackageStatus::Installed => Style::default().fg(green).bg(bg),
                    PackageStatus::Outdated => Style::default().fg(cyan).bg(bg),
                    PackageStatus::Installing => Style::default().fg(yellow).bg(bg),
                    PackageStatus::Available => Style::default().fg(grey).bg(bg),
                }
            };

            // Format: > S Name                         Version     Description
            let name_truncated = if pkg.name.len() > 26 {
                format!("{}...", &pkg.name[..23])
            } else {
                format!("{:<26}", pkg.name)
            };

            let version = pkg
                .installed_version
                .as_ref()
                .or(pkg.version.as_ref())
                .cloned()
                .unwrap_or_default();
            let version_truncated = if version.len() > 10 {
                format!("{}...", &version[..7])
            } else {
                format!("{:<10}", version)
            };

            let desc_width = popup_width.saturating_sub(46) as usize;
            let desc_truncated = if pkg.description.len() > desc_width {
                format!("{}...", &pkg.description[..desc_width.saturating_sub(3)])
            } else {
                pkg.description.clone()
            };

            let row = 1 + i as u16;
            modal.render_row(
                frame,
                row,
                vec![
                    Span::styled(format!("{} ", prefix), style),
                    Span::styled(format!("{} ", pkg.status.icon()), status_style),
                    Span::styled(format!("{} ", name_truncated), style),
                    Span::styled(format!("{} ", version_truncated), style),
                    Span::styled(desc_truncated, style),
                ],
            );
        }
    }

    // Help footer based on view
    match state.view {
        HomebrewView::List => {
            modal.render_help(
                frame,
                vec![
                    ("Enter", "install"),
                    ("/", "search"),
                    ("R", "refresh"),
                    ("Esc", "close"),
                ],
            );
        }
        HomebrewView::Search => {
            modal.render_help(frame, vec![("Enter", "search"), ("Esc", "cancel")]);
        }
        HomebrewView::Details => {
            modal.render_help(
                frame,
                vec![("i", "install"), ("u", "uninstall"), ("Esc", "back")],
            );
        }
    }
}
