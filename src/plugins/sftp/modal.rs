//! SFTP Plugin Modal Rendering

use super::ops::format_bytes;
use super::state::{AuthMethod, ConnectField, SftpState, SftpView};
use crate::app::ThemeColors;
use crate::ui::components::ModalFrame;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Draw the SFTP modal
pub fn draw_sftp_modal(frame: &mut Frame, area: Rect, state: &SftpState, colors: &ThemeColors) {
    let modal = ModalFrame::themed(area, " SFTP ", colors);
    modal.render_frame(frame);

    let content_area = modal.content_area();

    match state.view {
        SftpView::Connections => draw_connections_view(frame, content_area, state, colors),
        SftpView::Connect => draw_connect_view(frame, content_area, state, colors),
        SftpView::Browser => draw_browser_view(frame, content_area, state, colors),
        SftpView::Transfer => draw_transfer_view(frame, content_area, state, colors),
        SftpView::SaveProfile => draw_save_profile_view(frame, content_area, state, colors),
        SftpView::Error => draw_error_view(frame, content_area, state, colors),
    }

    let help = match state.view {
        SftpView::Connections => vec![
            ("Enter", "connect"),
            ("N", "new"),
            ("D", "delete"),
            ("Esc", "close"),
        ],
        SftpView::Connect => vec![
            ("Tab", "next field"),
            ("Enter", "connect"),
            ("S", "save"),
            ("Esc", "back"),
        ],
        SftpView::Browser => vec![
            ("Enter", "open"),
            ("G", "download"),
            ("U", "upload"),
            ("Esc", "disconnect"),
        ],
        SftpView::Transfer => vec![("Esc", "cancel")],
        SftpView::SaveProfile => vec![("Enter", "save"), ("Esc", "cancel")],
        SftpView::Error => vec![("Enter", "ok"), ("Esc", "close")],
    };
    modal.render_help(frame, help);
}

fn draw_connections_view(frame: &mut Frame, area: Rect, state: &SftpState, colors: &ThemeColors) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![Span::styled(
        "Saved Connections",
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    if state.profiles.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "No saved connections",
            Style::default().fg(colors.grey()),
        )]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Press N to create a new connection",
            Style::default().fg(colors.green()),
        )]));
    } else {
        for (i, profile) in state.profiles.iter().enumerate() {
            let is_selected = i == state.selected_profile;
            let style = if is_selected {
                Style::default()
                    .fg(colors.yellow())
                    .bg(colors.red())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg())
            };

            let auth_str = match &profile.connection.auth_method {
                AuthMethod::DefaultKey => "[key]",
                AuthMethod::KeyFile(_) => "[key]",
                AuthMethod::Password => "[pass]",
                AuthMethod::Agent => "[agent]",
            };

            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", if is_selected { ">" } else { " " }), style),
                Span::styled(format!("{:<20}", profile.name), style),
                Span::styled(
                    format!(
                        " {}@{}:{}",
                        profile.connection.username,
                        profile.connection.host,
                        profile.connection.port
                    ),
                    if is_selected {
                        style
                    } else {
                        Style::default().fg(colors.grey())
                    },
                ),
                Span::styled(
                    format!(" {}", auth_str),
                    if is_selected {
                        style
                    } else {
                        Style::default().fg(colors.cyan())
                    },
                ),
            ]));
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn draw_connect_view(frame: &mut Frame, area: Rect, state: &SftpState, colors: &ThemeColors) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![Span::styled(
        "New Connection",
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let fields = [
        (ConnectField::Host, "Host:", &state.connection.host),
        (
            ConnectField::Port,
            "Port:",
            &state.connection.port.to_string(),
        ),
        (
            ConnectField::Username,
            "Username:",
            &state.connection.username,
        ),
    ];

    for (field, label, value) in &fields {
        let is_selected = state.connect_field == *field;
        let label_style = Style::default().fg(colors.grey());
        let value_style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg())
        };

        let cursor = if is_selected { "_" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(format!("{:<12}", label), label_style),
            Span::styled(format!("{}{}", value, cursor), value_style),
        ]));
    }

    // Auth method
    let is_auth_selected = state.connect_field == ConnectField::AuthMethod;
    let auth_str = match &state.connection.auth_method {
        AuthMethod::DefaultKey => "SSH Key (default)",
        AuthMethod::Agent => "SSH Agent",
        AuthMethod::Password => "Password",
        AuthMethod::KeyFile(_) => "SSH Key (custom)",
    };
    lines.push(Line::from(vec![
        Span::styled("Auth:       ", Style::default().fg(colors.grey())),
        Span::styled(
            format!(
                "{} {}",
                auth_str,
                if is_auth_selected {
                    "[Tab to change]"
                } else {
                    ""
                }
            ),
            if is_auth_selected {
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg())
            },
        ),
    ]));

    // Password field (only if password auth)
    if matches!(state.connection.auth_method, AuthMethod::Password) {
        let is_pass_selected = state.connect_field == ConnectField::Password;
        let pass_display = "*".repeat(state.connection.password.len());
        let cursor = if is_pass_selected { "_" } else { "" };
        lines.push(Line::from(vec![
            Span::styled("Password:   ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}{}", pass_display, cursor),
                if is_pass_selected {
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors.fg())
                },
            ),
        ]));
    }

    // Key file field (only if key file auth)
    if let AuthMethod::KeyFile(ref path) = state.connection.auth_method {
        let is_key_selected = state.connect_field == ConnectField::KeyFile;
        let cursor = if is_key_selected { "_" } else { "" };
        lines.push(Line::from(vec![
            Span::styled("Key File:   ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}{}", path, cursor),
                if is_key_selected {
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors.fg())
                },
            ),
        ]));
    }

    // Default path
    let is_path_selected = state.connect_field == ConnectField::DefaultPath;
    let cursor = if is_path_selected { "_" } else { "" };
    lines.push(Line::from(vec![
        Span::styled("Start Dir:  ", Style::default().fg(colors.grey())),
        Span::styled(
            format!(
                "{}{}",
                if state.connection.default_path.is_empty() {
                    "/"
                } else {
                    &state.connection.default_path
                },
                cursor
            ),
            if is_path_selected {
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg())
            },
        ),
    ]));

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn draw_browser_view(frame: &mut Frame, area: Rect, state: &SftpState, colors: &ThemeColors) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // Status bar
        Constraint::Min(1),    // File list
    ])
    .split(area);

    // Status bar
    let status_indicator = Span::styled("● ", Style::default().fg(colors.green()));
    let path_str = &state.current_dir;
    let truncated_path = if path_str.len() > chunks[0].width as usize - 15 {
        format!(
            "...{}",
            &path_str[path_str.len() - (chunks[0].width as usize - 18)..]
        )
    } else {
        path_str.to_string()
    };

    let line1 = Line::from(vec![
        status_indicator,
        Span::styled(
            state.connection.display(),
            Style::default().fg(colors.cyan()),
        ),
    ]);
    let line2 = Line::from(vec![Span::styled(
        truncated_path,
        Style::default().fg(colors.fg()),
    )]);

    let status_para = Paragraph::new(vec![line1, line2]);
    frame.render_widget(status_para, chunks[0]);

    // File list
    let visible_height = chunks[1].height as usize;
    let scroll_offset = if state.selected_file >= visible_height {
        state.selected_file - visible_height + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();

    if state.files.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "Empty directory",
            Style::default().fg(colors.grey()),
        )]));
    } else {
        for (i, file) in state
            .files
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_height)
        {
            let is_selected = i == state.selected_file;
            let name_style = if is_selected {
                Style::default()
                    .fg(colors.yellow())
                    .bg(colors.red())
                    .add_modifier(Modifier::BOLD)
            } else if file.is_dir {
                Style::default().fg(colors.blue())
            } else {
                Style::default().fg(colors.fg())
            };

            let type_indicator = if file.is_dir {
                Span::styled("<DIR>", Style::default().fg(colors.blue()))
            } else {
                Span::styled("     ", Style::default())
            };

            let name = if file.name.len() > 35 {
                format!("{}...", &file.name[..32])
            } else {
                file.name.clone()
            };

            let size_str = if file.is_dir {
                "        ".to_string()
            } else {
                format!("{:>8}", format_bytes(file.size))
            };

            let perms = file.permissions_string();

            lines.push(Line::from(vec![
                type_indicator,
                Span::raw(" "),
                Span::styled(format!("{:<35}", name), name_style),
                Span::styled(size_str, Style::default().fg(colors.grey())),
                Span::raw(" "),
                Span::styled(perms, Style::default().fg(colors.grey())),
            ]));
        }
    }

    let file_para = Paragraph::new(lines);
    frame.render_widget(file_para, chunks[1]);
}

fn draw_transfer_view(frame: &mut Frame, area: Rect, state: &SftpState, colors: &ThemeColors) {
    let mut lines: Vec<Line> = Vec::new();

    if let Some(ref transfer) = state.transfer {
        let direction = match transfer.direction {
            super::state::TransferDirection::Download => "Downloading",
            super::state::TransferDirection::Upload => "Uploading",
        };

        lines.push(Line::from(vec![Span::styled(
            direction,
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("File: ", Style::default().fg(colors.grey())),
            Span::styled(&transfer.filename, Style::default().fg(colors.fg())),
        ]));

        lines.push(Line::from(""));

        // Progress bar
        let percent = transfer.progress_percent();
        let bar_width = 40;
        let filled = (bar_width * percent as usize) / 100;
        let empty = bar_width - filled;

        let progress_bar = format!("[{}{}] {}%", "=".repeat(filled), " ".repeat(empty), percent);

        lines.push(Line::from(vec![Span::styled(
            progress_bar,
            Style::default().fg(colors.cyan()),
        )]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![Span::styled(
            format!(
                "{} / {}",
                format_bytes(transfer.transferred_bytes),
                format_bytes(transfer.total_bytes)
            ),
            Style::default().fg(colors.grey()),
        )]));

        if let Some(ref error) = transfer.error {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                format!("Error: {}", error),
                Style::default().fg(colors.red()),
            )]));
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn draw_save_profile_view(frame: &mut Frame, area: Rect, state: &SftpState, colors: &ThemeColors) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![Span::styled(
        "Save Connection",
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("Profile Name: ", Style::default().fg(colors.grey())),
        Span::styled(
            format!("{}_", &state.profile_name),
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        format!("Connection: {}", state.connection.display()),
        Style::default().fg(colors.grey()),
    )]));

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn draw_error_view(frame: &mut Frame, area: Rect, state: &SftpState, colors: &ThemeColors) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![Span::styled(
        "Error",
        Style::default()
            .fg(colors.red())
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    if let Some(ref error) = state.error {
        // Word wrap error message
        for line in error.chars().collect::<Vec<_>>().chunks(60) {
            lines.push(Line::from(vec![Span::styled(
                line.iter().collect::<String>(),
                Style::default().fg(colors.fg()),
            )]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Press Enter or Esc to continue",
        Style::default().fg(colors.grey()),
    )]));

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}
