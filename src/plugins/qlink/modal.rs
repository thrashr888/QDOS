//! Q-LINK plugin modal rendering
//!
//! UI components for the MCP client plugin.

use super::state::{ConnectionStatus, QLinkState, QLinkView};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{layout::Rect, style::Style, text::Span, Frame};

/// Draw the Q-LINK modal
pub fn draw_qlink_modal(frame: &mut Frame, area: Rect, state: &QLinkState, colors: &ThemeColors) {
    match state.view {
        QLinkView::ServerList => draw_server_list(frame, area, state, colors),
        QLinkView::Mounting => draw_mounting(frame, area, state, colors),
        QLinkView::Details => draw_details(frame, area, state, colors),
    }
}

/// Draw the server list view
fn draw_server_list(frame: &mut Frame, area: Rect, state: &QLinkState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-LINK Network ", colors);
    view.render_frame(frame);

    let mut row = 0;

    // Header
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "MCP Server Connections",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );
    row += 2;

    if state.servers.is_empty() {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "No MCP servers configured.",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        row += 2;

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "Add servers to your config file:",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        row += 1;

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "  ~/Library/Application Support/rdos/config.toml",
                Style::default().fg(colors.cyan()).bg(colors.bg()),
            )],
        );
        row += 2;

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "[qlink.servers.example]",
                Style::default().fg(colors.yellow()).bg(colors.bg()),
            )],
        );
        row += 1;

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                r#"name = "My Server""#,
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                r#"command = "npx""#,
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                r#"args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]"#,
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
    } else {
        // Server list
        for (i, server) in state.servers.iter().enumerate() {
            let is_selected = i == state.selected_index;
            let prefix = if is_selected { ">" } else { " " };

            // Status indicator
            let (status_char, status_color) = match server.status {
                ConnectionStatus::Disconnected => ("*", colors.grey()),
                ConnectionStatus::Connecting => ("~", colors.yellow()),
                ConnectionStatus::Connected => ("+", colors.green()),
                ConnectionStatus::Error => ("!", colors.red()),
            };

            let style = if is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(colors.fg()).bg(colors.bg())
            };

            let status_style = if is_selected {
                Style::default().fg(status_color).bg(colors.red())
            } else {
                Style::default().fg(status_color).bg(colors.bg())
            };

            view.render_row(
                frame,
                row,
                vec![
                    Span::styled(format!("{} ", prefix), style),
                    Span::styled(format!("[{}] ", status_char), status_style),
                    Span::styled(format!("{:<20}", server.config.name), style),
                    Span::styled(
                        format!(" {}", server.config.mount_path.display()),
                        if is_selected {
                            Style::default().fg(colors.cyan()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.cyan()).bg(colors.bg())
                        },
                    ),
                ],
            );
            row += 1;
        }
    }

    // Status/error message
    row += 1;
    if let Some(ref msg) = state.status_message {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                msg.as_str(),
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
    } else if let Some(ref err) = state.error {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Error: {}", err),
                Style::default().fg(colors.red()).bg(colors.bg()),
            )],
        );
    }

    // Help
    if state.servers.is_empty() {
        view.render_help(frame, vec![("Esc", "close")]);
    } else {
        let selected = state.selected_server();
        let is_connected = selected.map(|s| s.is_connected()).unwrap_or(false);

        if is_connected {
            view.render_help(
                frame,
                vec![
                    ("Enter", "navigate"),
                    ("D", "disconnect"),
                    ("I", "info"),
                    ("Esc", "close"),
                ],
            );
        } else {
            view.render_help(
                frame,
                vec![("Enter", "connect"), ("I", "info"), ("Esc", "close")],
            );
        }
    }
}

/// Draw the mounting/connecting view
fn draw_mounting(frame: &mut Frame, area: Rect, state: &QLinkState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-LINK Network ", colors);
    view.render_frame(frame);

    let server_name = state
        .selected_server()
        .map(|s| s.config.name.as_str())
        .unwrap_or("Unknown");

    let mut row = 5;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!("Connecting to {}...", server_name),
            Style::default().fg(colors.yellow()).bg(colors.bg()),
        )],
    );
    row += 2;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Starting MCP server process",
            Style::default().fg(colors.grey()).bg(colors.bg()),
        )],
    );

    view.render_help(frame, vec![("Esc", "cancel")]);
}

/// Draw server details view
fn draw_details(frame: &mut Frame, area: Rect, state: &QLinkState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-LINK Server Details ", colors);
    view.render_frame(frame);

    let mut row = 0;

    if let Some(server) = state.selected_server() {
        // Server name
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                &server.config.name,
                Style::default().fg(colors.yellow()).bg(colors.bg()),
            )],
        );
        row += 2;

        // Status
        let (status_text, status_color) = match server.status {
            ConnectionStatus::Disconnected => ("Disconnected", colors.grey()),
            ConnectionStatus::Connecting => ("Connecting...", colors.yellow()),
            ConnectionStatus::Connected => ("Connected", colors.green()),
            ConnectionStatus::Error => ("Error", colors.red()),
        };

        view.render_row(
            frame,
            row,
            vec![
                Span::styled(
                    "Status: ",
                    Style::default().fg(colors.grey()).bg(colors.bg()),
                ),
                Span::styled(
                    status_text,
                    Style::default().fg(status_color).bg(colors.bg()),
                ),
            ],
        );
        row += 1;

        // Mount path
        view.render_row(
            frame,
            row,
            vec![
                Span::styled(
                    "Mount:  ",
                    Style::default().fg(colors.grey()).bg(colors.bg()),
                ),
                Span::styled(
                    server.config.mount_path.display().to_string(),
                    Style::default().fg(colors.cyan()).bg(colors.bg()),
                ),
            ],
        );
        row += 1;

        // Command
        view.render_row(
            frame,
            row,
            vec![
                Span::styled(
                    "Command: ",
                    Style::default().fg(colors.grey()).bg(colors.bg()),
                ),
                Span::styled(
                    &server.config.command,
                    Style::default().fg(colors.fg()).bg(colors.bg()),
                ),
            ],
        );
        row += 1;

        // Args
        if !server.config.args.is_empty() {
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled(
                        "Args:    ",
                        Style::default().fg(colors.grey()).bg(colors.bg()),
                    ),
                    Span::styled(
                        server.config.args.join(" "),
                        Style::default().fg(colors.fg()).bg(colors.bg()),
                    ),
                ],
            );
            row += 1;
        }

        // Connection time
        if let Some(connected_at) = server.connected_at {
            let duration = connected_at.elapsed();
            let mins = duration.as_secs() / 60;
            let secs = duration.as_secs() % 60;
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled(
                        "Uptime:  ",
                        Style::default().fg(colors.grey()).bg(colors.bg()),
                    ),
                    Span::styled(
                        format!("{}m {}s", mins, secs),
                        Style::default().fg(colors.green()).bg(colors.bg()),
                    ),
                ],
            );
            row += 1;
        }

        // Error message
        if let Some(ref error) = server.error {
            row += 1;
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("Error: {}", error),
                    Style::default().fg(colors.red()).bg(colors.bg()),
                )],
            );
        }
    } else {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "No server selected",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
    }

    view.render_help(frame, vec![("Esc", "back")]);
}

#[cfg(test)]
mod tests {
    use super::super::state::ServerConfig;
    use super::*;

    #[test]
    fn test_state_with_servers() {
        let mut state = QLinkState::new();
        state.add_server(ServerConfig::new("test", "Test Server", "echo"));
        assert!(!state.servers.is_empty());
        assert!(state.selected_server().is_some());
    }
}
