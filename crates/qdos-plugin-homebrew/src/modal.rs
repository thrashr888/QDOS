//! Homebrew modal rendering
//!
//! UI for the Homebrew modal with tabs, info view, and confirm dialogs.

use super::state::{HomebrewState, HomebrewTab, HomebrewView, PackageStatus};
use qdos_plugin_api::prelude::ModalFrame;
use qdos_plugin_api::prelude::ThemeColors;
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
    match state.view {
        HomebrewView::List | HomebrewView::SearchInput => {
            draw_list_view(frame, area, state, colors)
        }
        HomebrewView::Info => draw_info_view(frame, area, state, colors),
        HomebrewView::Confirm => draw_confirm_view(frame, area, state, colors),
        HomebrewView::Output => draw_output_view(frame, area, state, colors),
    }
}

/// Draw the main list view with tabs
fn draw_list_view(frame: &mut Frame, area: Rect, state: &HomebrewState, colors: &ThemeColors) {
    // Calculate centered modal area
    let popup_width = 78u16.min(area.width.saturating_sub(4));
    let popup_height = 22u16.min(area.height.saturating_sub(4));
    let popup_x = (area.width - popup_width) / 2;
    let popup_y = (area.height - popup_height) / 2;
    let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let title = if state.view == HomebrewView::SearchInput {
        format!(" Search: {} ", state.search_query)
    } else {
        " Homebrew Packages ".to_string()
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
    let blue = colors.blue();

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
        let msg = state.loading_message.as_deref().unwrap_or("Loading...");
        modal.render_row(
            frame,
            1,
            vec![Span::styled(
                format!(" {} ", msg),
                Style::default().fg(yellow).bg(bg),
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
        modal.render_help(frame, vec![("r", "retry"), ("Esc", "close")]);
        return;
    }

    // Tab bar (row 0)
    let mut tab_spans = vec![Span::styled(" ", Style::default().fg(fg).bg(bg))];
    for tab in HomebrewTab::all() {
        let is_active = *tab == state.tab;
        let style = if is_active {
            Style::default().fg(yellow).bg(blue)
        } else {
            Style::default().fg(grey).bg(bg)
        };

        let label = match tab {
            HomebrewTab::Recommended => "Recommended",
            HomebrewTab::Installed => {
                // Show count
                &format!("Installed ({})", state.count_installed())
            }
            HomebrewTab::Search => "Search",
        };

        // For Installed tab, we need to handle differently
        if matches!(tab, HomebrewTab::Installed) {
            tab_spans.push(Span::styled(
                format!(" Installed ({}) ", state.count_installed()),
                style,
            ));
        } else {
            tab_spans.push(Span::styled(format!(" {} ", label), style));
        }
        tab_spans.push(Span::styled(" ", Style::default().fg(fg).bg(bg)));
    }

    // Add outdated count if any
    if state.outdated_count > 0 {
        let outdated_text = if state.show_outdated_only {
            format!("  [{}] outdated (o=all)", state.outdated_count)
        } else {
            format!("  {} outdated (o=filter)", state.outdated_count)
        };
        tab_spans.push(Span::styled(
            outdated_text,
            Style::default().fg(cyan).bg(bg),
        ));
    }

    modal.render_row(frame, 0, tab_spans);

    // Header row
    modal.render_row(
        frame,
        1,
        vec![Span::styled(
            " S Name                         Version     Description",
            Style::default().fg(grey).bg(bg),
        )],
    );

    // Package list
    let filtered = state.filtered_packages();
    let visible_height = modal.content_height() as usize;

    if filtered.is_empty() {
        let msg = match state.tab {
            HomebrewTab::Recommended => " No recommended packages loaded. Press r to refresh.",
            HomebrewTab::Installed => " No packages installed.",
            HomebrewTab::Search => " No search results. Press / to search.",
        };
        modal.render_row(
            frame,
            3,
            vec![Span::styled(msg, Style::default().fg(grey).bg(bg))],
        );
    } else {
        let max_visible = visible_height.saturating_sub(3);
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
            // For outdated packages, show a shorter name to make room for "(outdated)"
            let name_display = if pkg.status == PackageStatus::Outdated {
                if pkg.name.len() > 16 {
                    format!("{}... ^", &pkg.name[..13])
                } else {
                    format!("{} ^", pkg.name)
                }
            } else {
                pkg.name.clone()
            };

            let name_truncated = if name_display.len() > 26 {
                format!("{}...", &name_display[..23])
            } else {
                format!("{:<26}", name_display)
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

            let row = 2 + i as u16;
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
        HomebrewView::SearchInput => {
            modal.render_help(frame, vec![("Enter", "search"), ("Esc", "cancel")]);
        }
        _ => {
            modal.render_help(
                frame,
                vec![
                    ("Tab", "switch"),
                    ("i", "info"),
                    ("/", "search"),
                    ("u", "update"),
                    ("g", "upgrade"),
                    ("x", "uninstall"),
                ],
            );
        }
    }
}

/// Draw the package info view
fn draw_info_view(frame: &mut Frame, area: Rect, state: &HomebrewState, colors: &ThemeColors) {
    let popup_width = 70u16.min(area.width.saturating_sub(4));
    let popup_height = 18u16.min(area.height.saturating_sub(4));
    let popup_x = (area.width - popup_width) / 2;
    let popup_y = (area.height - popup_height) / 2;
    let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let bg = colors.bg();
    let fg = colors.fg();
    let grey = colors.grey();
    let green = colors.green();
    let cyan = colors.cyan();

    let info = match &state.package_info {
        Some(info) => info,
        None => {
            let modal = ModalFrame::themed(modal_area, " Package Info ", colors);
            modal.render_frame(frame);
            modal.render_row(
                frame,
                1,
                vec![Span::styled(
                    " No package info available",
                    Style::default().fg(grey).bg(bg),
                )],
            );
            modal.render_help(frame, vec![("Esc", "back")]);
            return;
        }
    };

    let title = format!(" {} ", info.name);
    let modal = ModalFrame::themed(modal_area, &title, colors);
    modal.render_frame(frame);

    let mut row = 0u16;

    // Version
    let version_text = if info.installed {
        if let Some(ref inst_ver) = info.installed_version {
            if inst_ver != &info.version && !info.version.is_empty() {
                format!("{} (installed: {})", info.version, inst_ver)
            } else {
                inst_ver.clone()
            }
        } else {
            info.version.clone()
        }
    } else {
        info.version.clone()
    };

    modal.render_row(
        frame,
        row,
        vec![
            Span::styled(" Version: ", Style::default().fg(grey).bg(bg)),
            Span::styled(version_text, Style::default().fg(fg).bg(bg)),
        ],
    );
    row += 1;

    // Status
    let (status_text, status_color) = if info.installed {
        ("Installed", green)
    } else {
        ("Not installed", grey)
    };
    modal.render_row(
        frame,
        row,
        vec![
            Span::styled(" Status:  ", Style::default().fg(grey).bg(bg)),
            Span::styled(status_text, Style::default().fg(status_color).bg(bg)),
        ],
    );
    row += 2;

    // Description
    if !info.description.is_empty() {
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                " Description:",
                Style::default().fg(grey).bg(bg),
            )],
        );
        row += 1;

        // Word wrap description
        let max_width = (popup_width - 4) as usize;
        let words: Vec<&str> = info.description.split_whitespace().collect();
        let mut line = String::from(" ");
        for word in words {
            if line.len() + word.len() + 1 > max_width {
                modal.render_row(
                    frame,
                    row,
                    vec![Span::styled(&line, Style::default().fg(fg).bg(bg))],
                );
                row += 1;
                line = format!(" {}", word);
            } else {
                if line.len() > 1 {
                    line.push(' ');
                }
                line.push_str(word);
            }
        }
        if line.len() > 1 {
            modal.render_row(
                frame,
                row,
                vec![Span::styled(line, Style::default().fg(fg).bg(bg))],
            );
            row += 1;
        }
        row += 1;
    }

    // Homepage
    if !info.homepage.is_empty() {
        modal.render_row(
            frame,
            row,
            vec![
                Span::styled(" Homepage: ", Style::default().fg(grey).bg(bg)),
                Span::styled(&info.homepage, Style::default().fg(cyan).bg(bg)),
            ],
        );
    }

    // Help based on install status
    if info.installed {
        modal.render_help(
            frame,
            vec![
                ("h", "homepage"),
                ("g", "upgrade"),
                ("x", "uninstall"),
                ("Esc", "back"),
            ],
        );
    } else {
        modal.render_help(
            frame,
            vec![("Enter", "install"), ("h", "homepage"), ("Esc", "back")],
        );
    }
}

/// Draw the confirm dialog
fn draw_confirm_view(frame: &mut Frame, area: Rect, state: &HomebrewState, colors: &ThemeColors) {
    let popup_width = 50u16.min(area.width.saturating_sub(4));
    let popup_height = 8u16.min(area.height.saturating_sub(4));
    let popup_x = (area.width - popup_width) / 2;
    let popup_y = (area.height - popup_height) / 2;
    let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let modal = ModalFrame::themed(modal_area, " Confirm ", colors);
    modal.render_frame(frame);

    let bg = colors.bg();
    let fg = colors.fg();
    let yellow = colors.yellow();

    if let Some(ref action) = state.confirm_action {
        // Message
        modal.render_row(
            frame,
            1,
            vec![Span::styled(
                format!(" {}", action.message()),
                Style::default().fg(fg).bg(bg),
            )],
        );

        // Command preview
        modal.render_row(
            frame,
            3,
            vec![Span::styled(
                format!(" $ {}", action.command()),
                Style::default().fg(yellow).bg(bg),
            )],
        );
    }

    modal.render_help(frame, vec![("y", "yes"), ("n", "no")]);
}

/// Draw the command output view
fn draw_output_view(frame: &mut Frame, area: Rect, state: &HomebrewState, colors: &ThemeColors) {
    let popup_width = 76u16.min(area.width.saturating_sub(4));
    let popup_height = 20u16.min(area.height.saturating_sub(4));
    let popup_x = (area.width - popup_width) / 2;
    let popup_y = (area.height - popup_height) / 2;
    let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let title = state
        .last_command
        .as_ref()
        .map(|c| format!(" $ {} ", c))
        .unwrap_or_else(|| " Output ".to_string());

    let modal = ModalFrame::themed(modal_area, &title, colors);
    modal.render_frame(frame);

    let bg = colors.bg();
    let fg = colors.fg();
    let grey = colors.grey();

    let visible_height = modal.content_height() as usize;

    // Get output lines
    let output = state.command_output.as_deref().unwrap_or("No output");
    let lines: Vec<&str> = output.lines().collect();
    let total_lines = lines.len();

    // Clamp scroll position
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = state.output_scroll.min(max_scroll);

    // Render visible lines
    for (i, line) in lines.iter().skip(scroll).take(visible_height).enumerate() {
        let display_line = if line.len() > (popup_width - 4) as usize {
            format!("{}...", &line[..(popup_width - 7) as usize])
        } else {
            line.to_string()
        };

        modal.render_row(
            frame,
            i as u16,
            vec![Span::styled(
                format!(" {}", display_line),
                Style::default().fg(fg).bg(bg),
            )],
        );
    }

    // Show scroll indicator if needed
    if total_lines > visible_height {
        let indicator = format!(
            " [{}/{}] ",
            scroll + 1,
            total_lines.saturating_sub(visible_height - 1).max(1)
        );
        modal.render_row(
            frame,
            visible_height as u16,
            vec![Span::styled(indicator, Style::default().fg(grey).bg(bg))],
        );
    }

    modal.render_help(frame, vec![("↑↓", "scroll"), ("any key", "continue")]);
}
