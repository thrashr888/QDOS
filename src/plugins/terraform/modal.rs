//! Terraform plugin modal rendering

use super::state::{
    folder_name, CommandStatus, MenuItem, TerraformState, TerraformTab, TerraformView,
};
use crate::app::ThemeColors;
use crate::ui::components::{FullScreenView, TabBar};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    Frame,
};

/// Draw the Terraform modal
pub fn draw_terraform_modal(
    frame: &mut Frame,
    area: Rect,
    state: &TerraformState,
    colors: &ThemeColors,
) {
    let workspace = state.current_workspace().unwrap_or("default");
    let folder = state
        .cwd
        .as_ref()
        .map(folder_name)
        .unwrap_or_else(|| ".".to_string());
    let title = format!(" Terraform: {} ({}) ", folder, workspace);

    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    // Check Terraform availability
    if !state.terraform_available {
        view.render_row(
            frame,
            2,
            vec![Span::styled(
                "Terraform is not installed or not in PATH",
                Style::default().fg(colors.red()),
            )],
        );
        let help = vec![("Esc", "close")];
        view.render_help(frame, help);
        return;
    }

    // Tab bar
    draw_tab_bar(frame, &view, state, colors);

    match state.view {
        TerraformView::Menu => draw_menu(frame, &view, state, colors),
        TerraformView::Init => draw_output(frame, &view, state, colors, "Initializing..."),
        TerraformView::Plan => draw_output(frame, &view, state, colors, "Planning..."),
        TerraformView::Apply => draw_output(frame, &view, state, colors, "Applying..."),
        TerraformView::Workspaces => draw_workspaces(frame, &view, state, colors),
        TerraformView::State => draw_state(frame, &view, state, colors),
        TerraformView::StateDetail => draw_state_detail(frame, &view, state, colors),
        TerraformView::Output => draw_output(frame, &view, state, colors, "Running..."),
        TerraformView::Confirm => draw_confirm(frame, &view, state, colors),
    }
}

fn draw_tab_bar(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &TerraformState,
    colors: &ThemeColors,
) {
    let tabs = ["Operations", "Workspaces", "State"];
    let selected = match state.tab {
        TerraformTab::Operations => 0,
        TerraformTab::Workspaces => 1,
        TerraformTab::State => 2,
    };

    let tab_bar = TabBar::new(&tabs, selected);
    view.render_row(frame, 0, tab_bar.render(colors));
}

fn draw_menu(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &TerraformState,
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
    }

    // Initialization warning
    if !state.initialized {
        view.render_row(
            frame,
            2,
            vec![Span::styled(
                "Not initialized - run Init first",
                Style::default().fg(colors.yellow()),
            )],
        );
    }

    // Menu items
    let start_row = 4u16;
    for (i, item) in MenuItem::all().iter().enumerate() {
        let is_selected = i == state.selected_menu;

        let key_style = if is_selected {
            Style::default().fg(colors.cyan()).bg(colors.red())
        } else {
            Style::default().fg(colors.cyan())
        };

        let name_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg())
        };

        let desc_style = if is_selected {
            Style::default().fg(colors.grey()).bg(colors.red())
        } else {
            Style::default().fg(colors.grey())
        };

        let row = Line::from(vec![
            Span::styled(format!("[{}] ", item.key()), key_style),
            Span::styled(format!("{:<10}", item.name()), name_style),
            Span::styled(format!(" - {}", item.description()), desc_style),
        ]);

        view.render_row(frame, start_row + i as u16, row.spans);
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
        ("i", "init"),
        ("p", "plan"),
        ("a", "apply"),
        ("Tab", "switch"),
        ("Esc", "close"),
    ];
    view.render_help(frame, help);
}

fn draw_workspaces(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &TerraformState,
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

    // Header
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            "Workspaces",
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        )],
    );

    // Empty state
    if state.workspaces.is_empty() {
        view.render_row(
            frame,
            4,
            vec![Span::styled(
                "No workspaces found",
                Style::default().fg(colors.grey()),
            )],
        );
        let help = vec![("n", "new"), ("r", "refresh"), ("Esc", "close")];
        view.render_help(frame, help);
        return;
    }

    // Workspace list
    let start = state.workspace_scroll;
    let end = (start + content_height - 5).min(state.workspaces.len());

    for (i, workspace) in state.workspaces[start..end].iter().enumerate() {
        let is_selected = start + i == state.selected_workspace;

        let marker = if workspace.is_current { "* " } else { "  " };

        let base_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else if workspace.is_current {
            Style::default().fg(colors.green())
        } else {
            Style::default().fg(colors.fg())
        };

        view.render_row(
            frame,
            (i + 4) as u16,
            vec![Span::styled(
                format!("{}{}", marker, workspace.name),
                base_style,
            )],
        );
    }

    // Help footer
    let help = vec![
        ("Enter", "select"),
        ("n", "new"),
        ("d", "delete"),
        ("r", "refresh"),
        ("Esc", "back"),
    ];
    view.render_help(frame, help);
}

fn draw_state(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &TerraformState,
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

    // Header
    let header = Line::from(vec![
        Span::styled(
            format!("{:<20}", "Type"),
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
            "Address",
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    view.render_row(frame, 2, header.spans);

    // Empty state
    if state.resources.is_empty() {
        view.render_row(
            frame,
            4,
            vec![Span::styled(
                "No resources in state",
                Style::default().fg(colors.grey()),
            )],
        );
        let help = vec![("r", "refresh"), ("Esc", "back")];
        view.render_help(frame, help);
        return;
    }

    // Resource list
    let start = state.resource_scroll;
    let end = (start + content_height - 5).min(state.resources.len());

    for (i, resource) in state.resources[start..end].iter().enumerate() {
        let is_selected = start + i == state.selected_resource;

        let base_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg())
        };

        let type_style = if is_selected {
            Style::default().fg(colors.cyan()).bg(colors.red())
        } else {
            Style::default().fg(colors.cyan())
        };

        let type_display = if resource.resource_type.len() > 18 {
            format!("{:.18}..", resource.resource_type)
        } else {
            format!("{:<20}", resource.resource_type)
        };

        let name_display = if resource.name.len() > 23 {
            format!("{:.23}..", resource.name)
        } else {
            format!("{:<25}", resource.name)
        };

        let row = Line::from(vec![
            Span::styled(type_display, type_style),
            Span::styled(name_display, base_style),
            Span::styled(&resource.address, base_style),
        ]);

        view.render_row(frame, (i + 3) as u16, row.spans);
    }

    // Status line
    let status = format!(
        "{}/{} resources",
        state.selected_resource + 1,
        state.resources.len()
    );
    view.render_row(
        frame,
        (content_height - 1) as u16,
        vec![Span::styled(status, Style::default().fg(colors.grey()))],
    );

    // Help footer
    let help = vec![
        ("Enter", "show"),
        ("d", "remove"),
        ("r", "refresh"),
        ("Esc", "back"),
    ];
    view.render_help(frame, help);
}

fn draw_state_detail(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &TerraformState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Resource address header
    if let Some(resource) = &state.current_resource {
        view.render_row(
            frame,
            2,
            vec![Span::styled(
                &resource.address,
                Style::default().fg(colors.cyan()),
            )],
        );
    }

    // Detail lines
    let start = state.detail_scroll;
    let end = (start + content_height - 5).min(state.resource_detail.len());

    for (i, line) in state.resource_detail[start..end].iter().enumerate() {
        let style = if line.contains('=') {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                // Key = Value formatting
                let row = Line::from(vec![
                    Span::styled(parts[0], Style::default().fg(colors.cyan())),
                    Span::styled("=", Style::default().fg(colors.grey())),
                    Span::styled(parts[1], Style::default().fg(colors.fg())),
                ]);
                view.render_row(frame, (i + 3) as u16, row.spans);
                continue;
            }
            Style::default().fg(colors.fg())
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

    let help = vec![("Esc", "back")];
    view.render_help(frame, help);
}

fn draw_output(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &TerraformState,
    colors: &ThemeColors,
    operation_name: &str,
) {
    let content_height = view.content_height() as usize;

    // Status header
    let (status_text, status_color) = match state.command_status {
        CommandStatus::NotStarted => ("Starting...", colors.grey()),
        CommandStatus::Running => (operation_name, colors.yellow()),
        CommandStatus::Success => ("Complete", colors.green()),
        CommandStatus::Failed => ("Failed", colors.red()),
    };

    view.render_row(
        frame,
        1,
        vec![
            Span::styled("Status: ", Style::default().fg(colors.grey())),
            Span::styled(status_text, Style::default().fg(status_color)),
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

    // Output lines
    let start = state.output_scroll;
    let visible_lines = content_height.saturating_sub(5);
    let end = (start + visible_lines).min(state.output_lines.len());

    for (i, line) in state.output_lines[start..end].iter().enumerate() {
        let style = if line.contains("Error") || line.contains("error") || line.contains("failed") {
            Style::default().fg(colors.red())
        } else if line.contains("Warning") || line.contains("warning") {
            Style::default().fg(colors.yellow())
        } else if line.starts_with('+') || line.contains("created") || line.contains("completed") {
            Style::default().fg(colors.green())
        } else if line.starts_with('-') || line.contains("destroyed") {
            Style::default().fg(colors.red())
        } else if line.starts_with('~') || line.contains("modified") {
            Style::default().fg(colors.yellow())
        } else if line.contains("Initializing") || line.contains("Downloading") {
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
    let help = if state.command_status.is_done() {
        vec![
            ("Tab", "next tab"),
            ("Enter", "menu"),
            ("Esc", "close"),
            ("↑↓", "scroll"),
        ]
    } else {
        vec![("↑↓", "scroll"), ("PgUp/Dn", "page")]
    };
    view.render_help(frame, help);
}

fn draw_confirm(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &TerraformState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    if let Some(action) = &state.confirm_action {
        let message = action.to_string();

        // Warning for destructive actions
        let is_destructive = matches!(
            action,
            super::state::ConfirmAction::Destroy | super::state::ConfirmAction::StateRemove(_)
        );

        if is_destructive {
            view.render_row(
                frame,
                (content_height / 2 - 1) as u16,
                vec![Span::styled(
                    "WARNING: This action cannot be undone!",
                    Style::default().fg(colors.red()),
                )],
            );
        }

        view.render_row(
            frame,
            (content_height / 2) as u16,
            vec![Span::styled(message, Style::default().fg(colors.yellow()))],
        );
    }

    let help = vec![("Y", "confirm"), ("N", "cancel")];
    view.render_help(frame, help);
}
