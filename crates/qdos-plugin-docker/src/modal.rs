//! Docker plugin modal rendering

use super::state::{folder_name, BuildStatus, DockerState, DockerTab, DockerView};
use qdos_plugin_api::prelude::ThemeColors;
use qdos_plugin_api::prelude::{FullScreenView, TabBar};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    Frame,
};

/// Draw the Docker modal
pub fn draw_docker_modal(frame: &mut Frame, area: Rect, state: &DockerState, colors: &ThemeColors) {
    let folder = state
        .cwd
        .as_ref()
        .map(folder_name)
        .unwrap_or_else(|| ".".to_string());
    let title = format!(" Docker: {} ", folder);
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    // Tab bar
    draw_tab_bar(frame, &view, state, colors);

    // Check Docker availability
    if !state.docker_available {
        view.render_row(
            frame,
            2,
            vec![Span::styled(
                "Docker is not available. Is Docker running?",
                Style::default().fg(colors.red()),
            )],
        );
        let help = vec![("r", "retry"), ("Esc", "close")];
        view.render_help(frame, help);
        return;
    }

    match state.view {
        DockerView::Containers => draw_containers(frame, &view, state, colors),
        DockerView::Images => draw_images(frame, &view, state, colors),
        DockerView::Volumes => draw_volumes(frame, &view, state, colors),
        DockerView::Networks => draw_networks(frame, &view, state, colors),
        DockerView::Logs => draw_logs(frame, &view, state, colors),
        DockerView::Inspect => draw_inspect(frame, &view, state, colors),
        DockerView::Pull => draw_pull_input(frame, &view, state, colors),
        DockerView::Exec => draw_exec_input(frame, &view, state, colors),
        DockerView::Confirm => draw_confirm(frame, &view, state, colors),
        DockerView::Build => draw_build_input(frame, &view, state, colors),
        DockerView::BuildOutput => draw_build_output(frame, &view, state, colors),
        DockerView::Compose => draw_compose(frame, &view, state, colors),
        DockerView::ComposeLogs => draw_compose_logs(frame, &view, state, colors),
    }
}

fn draw_tab_bar(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DockerState,
    colors: &ThemeColors,
) {
    let tabs = ["Containers", "Images", "Volumes", "Networks"];
    let selected = match state.tab {
        DockerTab::Containers => 0,
        DockerTab::Images => 1,
        DockerTab::Volumes => 2,
        DockerTab::Networks => 3,
    };

    let tab_bar = TabBar::new(&tabs, selected);
    view.render_row(frame, 0, tab_bar.render(colors));
}

fn draw_containers(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DockerState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Loading state
    if state.loading {
        let msg = state.loading_message.as_deref().unwrap_or("Loading...");
        view.render_row(
            frame,
            2,
            vec![Span::styled(msg, Style::default().fg(colors.yellow()))],
        );
        return;
    }

    // Error state
    if let Some(err) = &state.error {
        view.render_row(
            frame,
            2,
            vec![Span::styled(
                format!("Error: {}", err),
                Style::default().fg(colors.red()),
            )],
        );
        let help = vec![("Esc", "close"), ("r", "retry")];
        view.render_help(frame, help);
        return;
    }

    // Header row
    let all_indicator = if state.show_all_containers {
        " [all]"
    } else {
        ""
    };
    let header = Line::from(vec![
        Span::styled(
            format!("{:<12}", "ID"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<20}", "Name"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<20}", "Image"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<15}", "Status"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("Ports{}", all_indicator),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    view.render_row(frame, 2, header.spans);

    // Empty state
    if state.containers.is_empty() {
        view.render_row(
            frame,
            4,
            vec![Span::styled(
                "No containers found",
                Style::default().fg(colors.grey()),
            )],
        );
        let help = vec![
            ("a", "toggle all"),
            ("p", "pull image"),
            ("r", "refresh"),
            ("Esc", "close"),
        ];
        view.render_help(frame, help);
        return;
    }

    // Container rows
    let start = state.container_scroll;
    let end = (start + content_height - 4).min(state.containers.len());

    for (i, container) in state.containers[start..end].iter().enumerate() {
        let is_selected = start + i == state.selected_container;

        let status_color = match container.status {
            super::state::ContainerStatus::Running => colors.green(),
            super::state::ContainerStatus::Paused => colors.yellow(),
            super::state::ContainerStatus::Stopped => colors.grey(),
            super::state::ContainerStatus::Restarting => colors.cyan(),
            super::state::ContainerStatus::Dead => colors.red(),
            super::state::ContainerStatus::Created => colors.blue(),
        };

        let base_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg())
        };

        let status_style = if is_selected {
            Style::default().fg(status_color).bg(colors.red())
        } else {
            Style::default().fg(status_color)
        };

        let id_display = if container.id.len() > 12 {
            format!("{:.12}", container.id)
        } else {
            format!("{:<12}", container.id)
        };

        let name_display = if container.name.len() > 18 {
            format!("{:.18}..", container.name)
        } else {
            format!("{:<20}", container.name)
        };

        let image_display = if container.image.len() > 18 {
            format!("{:.18}..", container.image)
        } else {
            format!("{:<20}", container.image)
        };

        let status_display = format!(
            "{} {:<13}",
            container.status.symbol(),
            if container.status_text.len() > 12 {
                format!("{:.12}", container.status_text)
            } else {
                container.status_text.clone()
            }
        );

        let row = Line::from(vec![
            Span::styled(id_display, base_style),
            Span::styled(name_display, base_style),
            Span::styled(image_display, base_style),
            Span::styled(status_display, status_style),
            Span::styled(&container.ports, base_style),
        ]);

        view.render_row(frame, (i + 3) as u16, row.spans);
    }

    // Message
    if let Some(msg) = &state.message {
        view.render_row(
            frame,
            (content_height - 1) as u16,
            vec![Span::styled(msg, Style::default().fg(colors.green()))],
        );
    }

    // Help footer
    let help = vec![
        ("s", "start"),
        ("t", "stop"),
        ("r", "restart"),
        ("l", "logs"),
        ("x", "exec"),
        ("d", "remove"),
        ("a", "all"),
        ("Esc", "close"),
    ];
    view.render_help(frame, help);
}

fn draw_images(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DockerState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Loading state
    if state.loading {
        let msg = state.loading_message.as_deref().unwrap_or("Loading...");
        view.render_row(
            frame,
            2,
            vec![Span::styled(msg, Style::default().fg(colors.yellow()))],
        );
        return;
    }

    // Header row
    let header = Line::from(vec![
        Span::styled(
            format!("{:<12}", "ID"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<30}", "Repository"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<15}", "Tag"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Size",
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    view.render_row(frame, 2, header.spans);

    // Empty state
    if state.images.is_empty() {
        view.render_row(
            frame,
            4,
            vec![Span::styled(
                "No images found",
                Style::default().fg(colors.grey()),
            )],
        );
        let help = vec![("p", "pull"), ("r", "refresh"), ("Esc", "close")];
        view.render_help(frame, help);
        return;
    }

    // Image rows
    let start = state.image_scroll;
    let end = (start + content_height - 4).min(state.images.len());

    for (i, image) in state.images[start..end].iter().enumerate() {
        let is_selected = start + i == state.selected_image;

        let base_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg())
        };

        let id_display = if image.id.len() > 12 {
            format!("{:.12}", image.id)
        } else {
            format!("{:<12}", image.id)
        };

        let repo_display = if image.repository.len() > 28 {
            format!("{:.28}..", image.repository)
        } else {
            format!("{:<30}", image.repository)
        };

        let tag_display = if image.tag.len() > 13 {
            format!("{:.13}..", image.tag)
        } else {
            format!("{:<15}", image.tag)
        };

        let row = Line::from(vec![
            Span::styled(id_display, base_style),
            Span::styled(repo_display, base_style),
            Span::styled(tag_display, base_style),
            Span::styled(&image.size, base_style),
        ]);

        view.render_row(frame, (i + 3) as u16, row.spans);
    }

    // Help footer
    let help = vec![
        ("p", "pull"),
        ("d", "remove"),
        ("i", "inspect"),
        ("P", "prune"),
        ("Esc", "close"),
    ];
    view.render_help(frame, help);
}

fn draw_volumes(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DockerState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Loading state
    if state.loading {
        let msg = state.loading_message.as_deref().unwrap_or("Loading...");
        view.render_row(
            frame,
            2,
            vec![Span::styled(msg, Style::default().fg(colors.yellow()))],
        );
        return;
    }

    // Header row
    let header = Line::from(vec![
        Span::styled(
            format!("{:<40}", "Name"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<10}", "Driver"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Mountpoint",
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    view.render_row(frame, 2, header.spans);

    // Empty state
    if state.volumes.is_empty() {
        view.render_row(
            frame,
            4,
            vec![Span::styled(
                "No volumes found",
                Style::default().fg(colors.grey()),
            )],
        );
        let help = vec![("r", "refresh"), ("Esc", "close")];
        view.render_help(frame, help);
        return;
    }

    // Volume rows
    let start = state.volume_scroll;
    let end = (start + content_height - 4).min(state.volumes.len());

    for (i, volume) in state.volumes[start..end].iter().enumerate() {
        let is_selected = start + i == state.selected_volume;

        let base_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg())
        };

        let name_display = if volume.name.len() > 38 {
            format!("{:.38}..", volume.name)
        } else {
            format!("{:<40}", volume.name)
        };

        let row = Line::from(vec![
            Span::styled(name_display, base_style),
            Span::styled(format!("{:<10}", volume.driver), base_style),
            Span::styled(&volume.mountpoint, base_style),
        ]);

        view.render_row(frame, (i + 3) as u16, row.spans);
    }

    // Help footer
    let help = vec![("d", "remove"), ("r", "refresh"), ("Esc", "close")];
    view.render_help(frame, help);
}

fn draw_networks(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DockerState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Loading state
    if state.loading {
        let msg = state.loading_message.as_deref().unwrap_or("Loading...");
        view.render_row(
            frame,
            2,
            vec![Span::styled(msg, Style::default().fg(colors.yellow()))],
        );
        return;
    }

    // Header row
    let header = Line::from(vec![
        Span::styled(
            format!("{:<12}", "ID"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<25}", "Name"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<15}", "Driver"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Scope",
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    view.render_row(frame, 2, header.spans);

    // Empty state
    if state.networks.is_empty() {
        view.render_row(
            frame,
            4,
            vec![Span::styled(
                "No networks found",
                Style::default().fg(colors.grey()),
            )],
        );
        let help = vec![("r", "refresh"), ("Esc", "close")];
        view.render_help(frame, help);
        return;
    }

    // Network rows
    let start = state.network_scroll;
    let end = (start + content_height - 4).min(state.networks.len());

    for (i, network) in state.networks[start..end].iter().enumerate() {
        let is_selected = start + i == state.selected_network;

        let base_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg())
        };

        let id_display = if network.id.len() > 12 {
            format!("{:.12}", network.id)
        } else {
            format!("{:<12}", network.id)
        };

        let name_display = if network.name.len() > 23 {
            format!("{:.23}..", network.name)
        } else {
            format!("{:<25}", network.name)
        };

        let row = Line::from(vec![
            Span::styled(id_display, base_style),
            Span::styled(name_display, base_style),
            Span::styled(format!("{:<15}", network.driver), base_style),
            Span::styled(&network.scope, base_style),
        ]);

        view.render_row(frame, (i + 3) as u16, row.spans);
    }

    // Help footer
    let help = vec![("d", "remove"), ("r", "refresh"), ("Esc", "close")];
    view.render_help(frame, help);
}

fn draw_logs(frame: &mut Frame, view: &FullScreenView, state: &DockerState, colors: &ThemeColors) {
    let content_height = view.content_height() as usize;

    // Title with container info
    if let Some(container) = state.selected_container() {
        view.render_row(
            frame,
            2,
            vec![
                Span::styled("Logs: ", Style::default().fg(colors.cyan())),
                Span::raw(&container.name),
                if state.following_logs {
                    Span::styled(" [following]", Style::default().fg(colors.green()))
                } else {
                    Span::raw("")
                },
            ],
        );
    }

    // Log lines
    let start = state.output_scroll;
    let end = (start + content_height - 4).min(state.output_lines.len());

    for (i, line) in state.output_lines[start..end].iter().enumerate() {
        let style = if line.contains("error") || line.contains("Error") || line.contains("ERROR") {
            Style::default().fg(colors.red())
        } else if line.contains("warn") || line.contains("Warn") || line.contains("WARN") {
            Style::default().fg(colors.yellow())
        } else {
            Style::default().fg(colors.fg())
        };

        let display = if line.len() > 78 {
            format!("{:.78}", line)
        } else {
            line.clone()
        };

        view.render_row(frame, (i + 3) as u16, vec![Span::styled(display, style)]);
    }

    // Help footer
    let help = vec![("f", "follow"), ("Esc", "back")];
    view.render_help(frame, help);
}

fn draw_inspect(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DockerState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    view.render_row(
        frame,
        2,
        vec![Span::styled(
            "Inspect Output",
            Style::default().fg(colors.cyan()),
        )],
    );

    // Output lines
    let start = state.output_scroll;
    let end = (start + content_height - 4).min(state.output_lines.len());

    for (i, line) in state.output_lines[start..end].iter().enumerate() {
        let display = if line.len() > 78 {
            format!("{:.78}", line)
        } else {
            line.clone()
        };

        view.render_row(
            frame,
            (i + 3) as u16,
            vec![Span::styled(display, Style::default().fg(colors.fg()))],
        );
    }

    // Help footer
    let help = vec![("Esc", "back")];
    view.render_help(frame, help);
}

fn draw_pull_input(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DockerState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Prompt
    let mut spans = vec![Span::styled(
        "Pull image: ",
        Style::default().fg(colors.cyan()),
    )];

    // Input with cursor
    let before = &state.pull_image_name[..state.pull_cursor];
    let cursor_char = state
        .pull_image_name
        .chars()
        .nth(state.pull_cursor)
        .map(|c| c.to_string())
        .unwrap_or_else(|| " ".to_string());
    let after = if state.pull_cursor < state.pull_image_name.len() {
        &state.pull_image_name[state.pull_cursor + 1..]
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

    let help = vec![("Enter", "pull"), ("Esc", "cancel")];
    view.render_help(frame, help);
}

fn draw_exec_input(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DockerState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Container name
    if let Some(container) = state.selected_container() {
        view.render_row(
            frame,
            (content_height / 2 - 1) as u16,
            vec![
                Span::styled("Container: ", Style::default().fg(colors.grey())),
                Span::raw(&container.name),
            ],
        );
    }

    // Prompt
    let mut spans = vec![Span::styled(
        "Command: ",
        Style::default().fg(colors.cyan()),
    )];

    // Input with cursor
    let before = &state.exec_command[..state.exec_cursor];
    let cursor_char = state
        .exec_command
        .chars()
        .nth(state.exec_cursor)
        .map(|c| c.to_string())
        .unwrap_or_else(|| " ".to_string());
    let after = if state.exec_cursor < state.exec_command.len() {
        &state.exec_command[state.exec_cursor + 1..]
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

    let help = vec![("Enter", "exec"), ("Esc", "cancel")];
    view.render_help(frame, help);
}

fn draw_confirm(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DockerState,
    colors: &ThemeColors,
) {
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

fn draw_build_input(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DockerState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Show Dockerfile context
    view.render_row(
        frame,
        2,
        vec![
            Span::styled("Dockerfile: ", Style::default().fg(colors.grey())),
            Span::styled(
                state
                    .context
                    .path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| ".".to_string()),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );

    // Prompt for tag
    let mut spans = vec![Span::styled(
        "Image Tag: ",
        Style::default().fg(colors.cyan()),
    )];

    // Input with cursor
    let before = &state.build_tag[..state.build_cursor];
    let cursor_char = state
        .build_tag
        .chars()
        .nth(state.build_cursor)
        .map(|c| c.to_string())
        .unwrap_or_else(|| " ".to_string());
    let after = if state.build_cursor < state.build_tag.len() {
        &state.build_tag[state.build_cursor + 1..]
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

    let help = vec![("Enter", "build"), ("Esc", "cancel")];
    view.render_help(frame, help);
}

fn draw_build_output(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DockerState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Status header
    let (status_text, status_color) = match state.build_status {
        BuildStatus::NotStarted => ("Starting...", colors.grey()),
        BuildStatus::Running => ("Building...", colors.yellow()),
        BuildStatus::Success => ("Build Complete", colors.green()),
        BuildStatus::Failed => ("Build Failed", colors.red()),
    };

    view.render_row(
        frame,
        1,
        vec![
            Span::styled("Status: ", Style::default().fg(colors.grey())),
            Span::styled(status_text, Style::default().fg(status_color)),
            Span::styled(
                format!("  Tag: {}", state.build_tag),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );

    // Scroll indicator
    let total_lines = state.output_lines.len();
    let scroll_info = if total_lines > 0 {
        format!(
            " [{}/{}]",
            (state.output_scroll + 1).min(total_lines),
            total_lines
        )
    } else {
        String::new()
    };
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            scroll_info,
            Style::default().fg(colors.grey()),
        )],
    );

    // Show build output
    let start = state.output_scroll;
    let visible_lines = content_height.saturating_sub(5);
    let end = (start + visible_lines).min(state.output_lines.len());

    for (i, line) in state.output_lines[start..end].iter().enumerate() {
        let style = if line.contains("error") || line.contains("Error") || line.contains("ERROR") {
            Style::default().fg(colors.red())
        } else if line.contains("Step")
            || line.starts_with("Successfully")
            || line.contains("completed successfully")
        {
            Style::default().fg(colors.green())
        } else if line.starts_with(" --->") || line.starts_with("Removing") {
            Style::default().fg(colors.cyan())
        } else {
            Style::default().fg(colors.fg())
        };

        let display = if line.len() > 78 {
            format!("{:.78}", line)
        } else {
            line.clone()
        };

        view.render_row(frame, (i + 3) as u16, vec![Span::styled(display, style)]);
    }

    // Help footer based on status
    let help = if state.build_status.is_done() {
        vec![
            ("Tab", "images"),
            ("Enter", "done"),
            ("Esc", "close"),
            ("↑↓", "scroll"),
        ]
    } else {
        vec![("↑↓", "scroll"), ("PgUp/Dn", "page")]
    };
    view.render_help(frame, help);
}

fn draw_compose(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DockerState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Loading state
    if state.loading {
        let msg = state.loading_message.as_deref().unwrap_or("Loading...");
        view.render_row(
            frame,
            2,
            vec![Span::styled(msg, Style::default().fg(colors.yellow()))],
        );
        return;
    }

    // Show compose file
    view.render_row(
        frame,
        1,
        vec![
            Span::styled("Compose: ", Style::default().fg(colors.grey())),
            Span::styled(
                state
                    .context
                    .path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| ".".to_string()),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );

    // Header
    let header = Line::from(vec![
        Span::styled(
            format!("{:<30}", "Service"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<15}", "Status"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Ports",
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    view.render_row(frame, 3, header.spans);

    // Empty state
    if state.compose_services.is_empty() {
        view.render_row(
            frame,
            5,
            vec![Span::styled(
                "No services running (press 'u' to start)",
                Style::default().fg(colors.grey()),
            )],
        );
        let help = vec![
            ("u", "up"),
            ("d", "down"),
            ("r", "refresh"),
            ("Esc", "close"),
        ];
        view.render_help(frame, help);
        return;
    }

    // Service rows
    let start = state.service_scroll;
    let end = (start + content_height - 5).min(state.compose_services.len());

    for (i, service) in state.compose_services[start..end].iter().enumerate() {
        let is_selected = start + i == state.selected_service;

        let base_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg())
        };

        let status_style = if is_selected {
            Style::default().fg(colors.green()).bg(colors.red())
        } else if service.status.to_lowercase().contains("running")
            || service.status.to_lowercase().contains("up")
        {
            Style::default().fg(colors.green())
        } else {
            Style::default().fg(colors.grey())
        };

        let name_display = if service.name.len() > 28 {
            format!("{:.28}..", service.name)
        } else {
            format!("{:<30}", service.name)
        };

        let status_display = format!("{:<15}", &service.status);

        let row = Line::from(vec![
            Span::styled(name_display, base_style),
            Span::styled(status_display, status_style),
            Span::styled(&service.ports, base_style),
        ]);

        view.render_row(frame, (i + 4) as u16, row.spans);
    }

    let help = vec![
        ("u", "up"),
        ("d", "down"),
        ("l", "logs"),
        ("R", "restart"),
        ("Esc", "close"),
    ];
    view.render_help(frame, help);
}

fn draw_compose_logs(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DockerState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Loading state
    if state.loading {
        let msg = state
            .loading_message
            .as_deref()
            .unwrap_or("Loading logs...");
        view.render_row(
            frame,
            2,
            vec![Span::styled(msg, Style::default().fg(colors.yellow()))],
        );
        return;
    }

    // Service name header
    if let Some(service) = state.compose_services.get(state.selected_service) {
        view.render_row(
            frame,
            1,
            vec![
                Span::styled("Service: ", Style::default().fg(colors.grey())),
                Span::styled(&service.name, Style::default().fg(colors.cyan())),
            ],
        );
    }

    // Log lines
    let start = state.output_scroll;
    let end = (start + content_height - 3).min(state.output_lines.len());

    for (i, line) in state.output_lines[start..end].iter().enumerate() {
        let style = if line.contains("error") || line.contains("Error") || line.contains("ERROR") {
            Style::default().fg(colors.red())
        } else if line.contains("warn") || line.contains("Warn") || line.contains("WARN") {
            Style::default().fg(colors.yellow())
        } else {
            Style::default().fg(colors.fg())
        };

        let display = if line.len() > 78 {
            format!("{:.78}", line)
        } else {
            line.clone()
        };

        view.render_row(frame, (i + 2) as u16, vec![Span::styled(display, style)]);
    }

    let help = vec![("Esc", "back")];
    view.render_help(frame, help);
}
