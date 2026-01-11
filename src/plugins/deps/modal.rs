//! Dependency Manager modal rendering

use super::state::{DepsState, DepsView};
use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    Frame,
};

/// Draw the dependencies modal
pub fn draw_deps_modal(frame: &mut Frame, area: Rect, state: &DepsState, colors: &ThemeColors) {
    let pm_name = state
        .package_manager
        .map(|pm| pm.name())
        .unwrap_or("Unknown");
    let project_name = state.project_name.as_deref().unwrap_or("");

    let title = if project_name.is_empty() {
        format!(" Dependencies ({}) ", pm_name)
    } else {
        format!(" {} - {} ", project_name, pm_name)
    };

    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    match state.view {
        DepsView::List | DepsView::Outdated => draw_package_list(frame, &view, state, colors),
        DepsView::SearchInput => draw_search_input(frame, &view, state, colors),
        DepsView::Search => draw_search_results(frame, &view, state, colors),
        DepsView::Install => draw_install_input(frame, &view, state, colors),
        DepsView::Output => draw_output(frame, &view, state, colors),
        DepsView::Confirm => draw_confirm(frame, &view, state, colors),
    }
}

fn draw_package_list(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DepsState,
    colors: &ThemeColors,
) {
    let visible = state.visible_packages();
    let content_height = view.content_height() as usize;

    // Loading state
    if state.loading {
        let msg = state.loading_message.as_deref().unwrap_or("Loading...");
        view.render_row(
            frame,
            0,
            vec![Span::styled(msg, Style::default().fg(colors.yellow()))],
        );
        return;
    }

    // Error state
    if let Some(err) = &state.error {
        view.render_row(
            frame,
            0,
            vec![Span::styled(
                format!("Error: {}", err),
                Style::default().fg(colors.red()),
            )],
        );
        let help = vec![("Esc", "close"), ("r", "retry")];
        view.render_help(frame, help);
        return;
    }

    // Empty state
    if visible.is_empty() {
        if state.show_outdated_only {
            view.render_row(
                frame,
                0,
                vec![Span::styled(
                    "No outdated packages",
                    Style::default().fg(colors.green()),
                )],
            );
        } else {
            view.render_row(
                frame,
                0,
                vec![Span::styled(
                    "No packages found",
                    Style::default().fg(colors.grey()),
                )],
            );
        }
        let help = vec![("i", "install"), ("/", "search"), ("Esc", "close")];
        view.render_help(frame, help);
        return;
    }

    // Header row
    let header = Line::from(vec![
        Span::styled(
            format!("{:<30}", "Package"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<12}", "Current"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<12}", "Latest"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Type",
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    view.render_row(frame, 0, header.spans);

    // Package rows
    let start = state.scroll_offset;
    let end = (start + content_height - 1).min(visible.len());

    for (i, pkg) in visible[start..end].iter().enumerate() {
        let is_selected = start + i == state.selected_index;

        let name_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else if pkg.is_outdated {
            Style::default().fg(colors.yellow())
        } else {
            Style::default().fg(colors.fg())
        };

        let version_style = if is_selected {
            Style::default().fg(colors.fg()).bg(colors.red())
        } else {
            Style::default().fg(colors.grey())
        };

        let latest_style = if is_selected {
            Style::default().fg(colors.green()).bg(colors.red())
        } else if pkg.is_outdated {
            Style::default().fg(colors.green())
        } else {
            Style::default().fg(colors.grey())
        };

        let type_str = if pkg.is_dev { "dev" } else { "" };
        let type_style = if is_selected {
            Style::default().fg(colors.cyan()).bg(colors.red())
        } else {
            Style::default().fg(colors.cyan())
        };

        let name_display = if pkg.name.len() > 28 {
            format!("{:.28}..", pkg.name)
        } else {
            format!("{:<30}", pkg.name)
        };

        let row = Line::from(vec![
            Span::styled(name_display, name_style),
            Span::styled(
                format!("{:<12}", pkg.current_version.as_deref().unwrap_or("-")),
                version_style,
            ),
            Span::styled(
                format!("{:<12}", pkg.latest_version.as_deref().unwrap_or("-")),
                latest_style,
            ),
            Span::styled(type_str, type_style),
        ]);

        view.render_row(frame, (i + 1) as u16, row.spans);
    }

    // Status line
    let _status = format!(
        "{}/{} packages{}{}",
        state.selected_index + 1,
        visible.len(),
        if state.outdated_count > 0 {
            format!(" ({} outdated)", state.outdated_count)
        } else {
            String::new()
        },
        if state.show_outdated_only {
            " [outdated only]"
        } else {
            ""
        }
    );

    if let Some(msg) = &state.message {
        view.render_row(
            frame,
            content_height as u16,
            vec![Span::styled(msg, Style::default().fg(colors.green()))],
        );
    }

    // Help footer
    let help = if state.view == DepsView::Outdated {
        vec![
            ("u", "update"),
            ("U", "update all"),
            ("Tab", "all"),
            ("/", "search"),
            ("Esc", "close"),
        ]
    } else {
        vec![
            ("i", "install"),
            ("d", "uninstall"),
            ("u", "update"),
            ("o", "outdated"),
            ("/", "search"),
            ("Esc", "close"),
        ]
    };
    view.render_help(frame, help);
}

fn draw_search_input(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DepsState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Prompt
    let mut spans = vec![Span::styled("Search: ", Style::default().fg(colors.cyan()))];

    // Input with cursor
    let before = &state.search_query[..state.search_cursor];
    let cursor_char = state
        .search_query
        .chars()
        .nth(state.search_cursor)
        .map(|c| c.to_string())
        .unwrap_or_else(|| " ".to_string());
    let after = if state.search_cursor < state.search_query.len() {
        &state.search_query[state.search_cursor + 1..]
    } else {
        ""
    };

    spans.push(Span::raw(before.to_string()));
    spans.push(Span::styled(
        cursor_char,
        Style::default().fg(colors.bg()).bg(colors.fg()),
    ));
    spans.push(Span::raw(after.to_string()));

    view.render_row(frame, (content_height / 2) as u16, spans);

    let help = vec![("Enter", "search"), ("Esc", "cancel")];
    view.render_help(frame, help);
}

fn draw_search_results(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DepsState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Loading state
    if state.loading {
        view.render_row(
            frame,
            0,
            vec![Span::styled(
                "Searching...",
                Style::default().fg(colors.yellow()),
            )],
        );
        return;
    }

    // Query display
    view.render_row(
        frame,
        0,
        vec![
            Span::styled("Search: ", Style::default().fg(colors.cyan())),
            Span::raw(&state.search_query),
        ],
    );

    // Empty state
    if state.search_results.is_empty() {
        view.render_row(
            frame,
            2,
            vec![Span::styled(
                "No results found",
                Style::default().fg(colors.grey()),
            )],
        );
        let help = vec![("/", "new search"), ("Esc", "back")];
        view.render_help(frame, help);
        return;
    }

    // Results
    for (i, result) in state
        .search_results
        .iter()
        .enumerate()
        .take(content_height - 2)
    {
        let is_selected = i == state.selected_result;

        let name_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg())
        };

        let desc = if result.description.len() > 50 {
            format!("{:.50}...", result.description)
        } else {
            result.description.clone()
        };

        let row = vec![
            Span::styled(format!("{:<20}", result.name), name_style),
            Span::styled(
                format!(" {} ", result.version),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(desc, Style::default().fg(colors.grey())),
        ];

        view.render_row(frame, (i + 2) as u16, row);
    }

    let help = vec![("Enter", "install"), ("/", "new search"), ("Esc", "back")];
    view.render_help(frame, help);
}

fn draw_install_input(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DepsState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Prompt
    let dev_indicator = if state.install_as_dev { " (dev)" } else { "" };
    let mut spans = vec![Span::styled(
        format!("Install package{}: ", dev_indicator),
        Style::default().fg(colors.cyan()),
    )];

    // Input with cursor
    let before = &state.install_input[..state.install_cursor];
    let cursor_char = state
        .install_input
        .chars()
        .nth(state.install_cursor)
        .map(|c| c.to_string())
        .unwrap_or_else(|| " ".to_string());
    let after = if state.install_cursor < state.install_input.len() {
        &state.install_input[state.install_cursor + 1..]
    } else {
        ""
    };

    spans.push(Span::raw(before.to_string()));
    spans.push(Span::styled(
        cursor_char,
        Style::default().fg(colors.bg()).bg(colors.fg()),
    ));
    spans.push(Span::raw(after.to_string()));

    view.render_row(frame, (content_height / 2) as u16, spans);

    let help = vec![
        ("Enter", "install"),
        ("Tab", "toggle dev"),
        ("Esc", "cancel"),
    ];
    view.render_help(frame, help);
}

fn draw_output(frame: &mut Frame, view: &FullScreenView, state: &DepsState, colors: &ThemeColors) {
    let content_height = view.content_height() as usize;

    // Loading state
    if state.loading {
        let msg = state.loading_message.as_deref().unwrap_or("Running...");
        view.render_row(
            frame,
            0,
            vec![Span::styled(msg, Style::default().fg(colors.yellow()))],
        );
        return;
    }

    // Output lines
    let start = state.output_scroll;
    let end = (start + content_height).min(state.command_output.len());

    for (i, line) in state.command_output[start..end].iter().enumerate() {
        let style = if line.contains("error") || line.contains("Error") {
            Style::default().fg(colors.red())
        } else if line.contains("warning") || line.contains("Warning") {
            Style::default().fg(colors.yellow())
        } else {
            Style::default().fg(colors.fg())
        };

        let display = if line.len() > 78 {
            format!("{:.78}", line)
        } else {
            line.clone()
        };

        view.render_row(frame, i as u16, vec![Span::styled(display, style)]);
    }

    let help = vec![("Enter/Esc", "close")];
    view.render_help(frame, help);
}

fn draw_confirm(frame: &mut Frame, view: &FullScreenView, state: &DepsState, colors: &ThemeColors) {
    let content_height = view.content_height() as usize;

    if let Some(action) = &state.confirm_action {
        let message = action.to_string();
        view.render_row(
            frame,
            (content_height / 2) as u16,
            vec![Span::styled(message, Style::default().fg(colors.yellow()))],
        );
    }

    let help = vec![("Y", "confirm"), ("N", "cancel")];
    view.render_help(frame, help);
}
