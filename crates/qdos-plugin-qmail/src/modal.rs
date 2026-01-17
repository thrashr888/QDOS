//! Q-MAIL UI rendering

use crate::state::{AccountSetupField, ComposeField, QMailState, QMailView};
use qdos_plugin_api::prelude::{FullScreenView, ThemeColors};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

// =============================================================================
// ACCOUNT SETUP VIEW
// =============================================================================

pub fn draw_account_setup(state: &QMailState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-MAIL: Account Setup ", colors);
    view.render_frame(frame);

    let normal = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(colors.cyan());
    let edit_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::UNDERLINED);

    view.render_row(
        frame,
        0,
        vec![Span::styled(
            "Configure your email account (Gmail defaults provided)",
            Style::default().fg(colors.grey()),
        )],
    );
    view.render_row(frame, 1, vec![Span::raw("")]);

    // Account fields
    let fields = [
        ("Name:", &state.setup_account.name, AccountSetupField::Name),
        (
            "Email:",
            &state.setup_account.email,
            AccountSetupField::Email,
        ),
        (
            "IMAP Server:",
            &state.setup_account.imap_server,
            AccountSetupField::ImapServer,
        ),
        (
            "IMAP Port:",
            &state.setup_account.imap_port.to_string(),
            AccountSetupField::ImapPort,
        ),
        (
            "SMTP Server:",
            &state.setup_account.smtp_server,
            AccountSetupField::SmtpServer,
        ),
        (
            "SMTP Port:",
            &state.setup_account.smtp_port.to_string(),
            AccountSetupField::SmtpPort,
        ),
        (
            "Username:",
            &state.setup_account.username,
            AccountSetupField::Username,
        ),
    ];

    let mut row = 2u16;
    for (label, value, field) in fields {
        let is_current = state.setup_field == field;
        let field_style = if is_current { highlight } else { normal };

        view.render_row(
            frame,
            row,
            vec![Span::styled(format!("   {:14}", label), label_style)],
        );
        row += 1;

        let display_value = if is_current {
            format!("[{}|]", value)
        } else if value.is_empty() {
            "[________________]".to_string()
        } else {
            format!("[{}]", value)
        };

        let value_style = if is_current { edit_style } else { field_style };
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("   {:40}", display_value),
                value_style,
            )],
        );
        row += 1;
    }

    // Password field (masked)
    let is_password = state.setup_field == AccountSetupField::Password;
    view.render_row(
        frame,
        row,
        vec![Span::styled("   Password:     ", label_style)],
    );
    row += 1;
    let password_display = if is_password {
        format!("[{}|]", "*".repeat(state.password_buffer.len()))
    } else if state.password_buffer.is_empty() {
        "[________________]".to_string()
    } else {
        format!("[{}]", "*".repeat(state.password_buffer.len()))
    };
    let password_style = if is_password { edit_style } else { normal };
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!("   {:40}", password_display),
            password_style,
        )],
    );
    row += 1;

    // TLS toggle
    let is_tls = state.setup_field == AccountSetupField::UseTls;
    let tls_style = if is_tls { highlight } else { normal };
    let tls_marker = if state.setup_account.use_tls {
        "[x]"
    } else {
        "[ ]"
    };
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!("   {} Use TLS", tls_marker),
            tls_style,
        )],
    );

    // Status message
    if let Some(msg) = &state.status_message {
        let status_y = view.content_height().saturating_sub(2);
        view.render_row(
            frame,
            status_y,
            vec![Span::styled(msg, Style::default().fg(colors.green()))],
        );
    }

    view.render_help(
        frame,
        vec![
            ("Tab", "next"),
            ("S-Tab", "prev"),
            ("Space", "toggle TLS"),
            ("Enter", "save"),
            ("Esc", "close"),
        ],
    );
}

// =============================================================================
// FOLDER LIST VIEW
// =============================================================================

pub fn draw_folder_list(state: &QMailState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let title = if let Some(account) = state.current_account() {
        format!(" Q-MAIL: {} ", account.email)
    } else {
        " Q-MAIL: Folders ".to_string()
    };
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    let normal = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    view.render_row(
        frame,
        0,
        vec![Span::styled(
            "FOLDERS",
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        1,
        vec![Span::styled("-------", Style::default().fg(colors.grey()))],
    );

    for (i, folder) in state.folders.iter().enumerate() {
        let is_selected = i == state.folder_cursor;
        let style = if is_selected { highlight } else { normal };
        let marker = if is_selected { ">" } else { " " };

        let unread_str = if folder.unread > 0 {
            format!(" ({})", folder.unread)
        } else {
            String::new()
        };

        let line = format!(" {} {:12}{:6}", marker, folder.name, unread_str);
        view.render_row(frame, 3 + i as u16, vec![Span::styled(line, style)]);
    }

    // Status
    if state.connected {
        let status_y = view.content_height().saturating_sub(2);
        let status = if let Some(msg) = &state.status_message {
            msg.clone()
        } else {
            "Connected (mock mode)".to_string()
        };
        view.render_row(
            frame,
            status_y,
            vec![Span::styled(status, Style::default().fg(colors.green()))],
        );
    }

    view.render_help(
        frame,
        vec![
            ("^v", "select"),
            ("Enter", "open"),
            ("C", "compose"),
            ("?", "help"),
            ("Esc", "close"),
        ],
    );
}

// =============================================================================
// MESSAGE LIST VIEW
// =============================================================================

pub fn draw_message_list(state: &QMailState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let folder_name = state
        .folders
        .get(state.folder_cursor)
        .map(|f| f.name.as_str())
        .unwrap_or("INBOX");
    let title = format!(" Q-MAIL: {} ", folder_name);
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    let normal = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let unread_style = Style::default()
        .fg(colors.fg())
        .add_modifier(Modifier::BOLD);
    let header_style = Style::default()
        .fg(colors.cyan())
        .add_modifier(Modifier::BOLD);

    // Header
    view.render_row(
        frame,
        0,
        vec![Span::styled(
            format!("{:3} {:20} {:40} {:10}", "", "From", "Subject", "Date"),
            header_style,
        )],
    );
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            "-".repeat(area.width.saturating_sub(4) as usize),
            Style::default().fg(colors.grey()),
        )],
    );

    if state.messages.is_empty() {
        view.render_row(
            frame,
            3,
            vec![Span::styled(
                "No messages in this folder.",
                Style::default().fg(colors.grey()),
            )],
        );
    } else {
        let max_rows = view.content_height().saturating_sub(4) as usize;
        let start = state.message_scroll;
        let visible: Vec<_> = state.messages.iter().skip(start).take(max_rows).collect();

        for (i, msg) in visible.iter().enumerate() {
            let actual_idx = start + i;
            let is_selected = actual_idx == state.message_cursor;

            let base_style = if is_selected {
                highlight
            } else if !msg.is_read {
                unread_style
            } else {
                normal
            };

            let marker = if is_selected { ">" } else { " " };
            let unread_marker = if msg.is_read { " " } else { "*" };

            // Truncate from and subject
            let from: String = msg.from.chars().take(18).collect();
            let subject: String = msg.subject.chars().take(38).collect();

            // Format date
            let now = chrono::Utc::now();
            let date_str = if msg.date.date_naive() == now.date_naive() {
                msg.date.format("%H:%M").to_string()
            } else {
                msg.date.format("%b %d").to_string()
            };

            let line = format!(
                "{}{} {:18} {:38} {:>8}",
                marker, unread_marker, from, subject, date_str
            );
            view.render_row(frame, 2 + i as u16, vec![Span::styled(line, base_style)]);
        }
    }

    // Status bar
    let status_y = view.content_height().saturating_sub(2);
    let status = format!(
        "{} messages | {} unread",
        state.messages.len(),
        state.messages.iter().filter(|m| !m.is_read).count()
    );
    view.render_row(
        frame,
        status_y,
        vec![Span::styled(status, Style::default().fg(colors.grey()))],
    );

    view.render_help(
        frame,
        vec![
            ("^v", "navigate"),
            ("Enter", "read"),
            ("C", "compose"),
            ("D", "delete"),
            ("A", "archive"),
            ("U", "toggle read"),
            ("Esc", "back"),
        ],
    );
}

// =============================================================================
// MESSAGE READ VIEW
// =============================================================================

pub fn draw_message_read(state: &QMailState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-MAIL: Read Message ", colors);
    view.render_frame(frame);

    if let Some(msg) = &state.current_message {
        let header_style = Style::default().fg(colors.cyan());
        let normal = Style::default().fg(colors.fg());

        // Headers
        view.render_row(
            frame,
            0,
            vec![
                Span::styled("From: ", header_style),
                Span::styled(&msg.header.from, normal),
            ],
        );
        view.render_row(
            frame,
            1,
            vec![
                Span::styled("Date: ", header_style),
                Span::styled(msg.header.date.format("%Y-%m-%d %H:%M").to_string(), normal),
            ],
        );
        view.render_row(
            frame,
            2,
            vec![
                Span::styled("Subject: ", header_style),
                Span::styled(&msg.header.subject, normal),
            ],
        );
        view.render_row(
            frame,
            3,
            vec![Span::styled(
                "-".repeat(area.width.saturating_sub(4) as usize),
                Style::default().fg(colors.grey()),
            )],
        );

        // Body
        let body_lines: Vec<&str> = msg.body.lines().collect();
        let max_rows = view.content_height().saturating_sub(6) as usize;
        let visible: Vec<_> = body_lines
            .iter()
            .skip(state.message_scroll_offset)
            .take(max_rows)
            .collect();

        for (i, line) in visible.iter().enumerate() {
            view.render_row(frame, 5 + i as u16, vec![Span::styled(**line, normal)]);
        }

        // Status bar
        let status_y = view.content_height().saturating_sub(2);
        let status = format!(
            "Message {} | Line {}/{}",
            state.message_cursor + 1,
            state.message_scroll_offset + 1,
            body_lines.len()
        );
        view.render_row(
            frame,
            status_y,
            vec![Span::styled(status, Style::default().fg(colors.grey()))],
        );
    }

    view.render_help(
        frame,
        vec![
            ("^v", "scroll"),
            ("R", "reply"),
            ("D", "delete"),
            ("A", "archive"),
            ("Esc", "back"),
        ],
    );
}

// =============================================================================
// COMPOSE VIEW
// =============================================================================

pub fn draw_compose(state: &QMailState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-MAIL: Compose ", colors);
    view.render_frame(frame);

    let normal = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(colors.cyan());
    let edit_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::UNDERLINED);

    // To field
    let is_to = state.compose_field == ComposeField::To;
    view.render_row(frame, 0, vec![Span::styled("To:      ", label_style)]);
    let to_display = if is_to {
        format!("[{}|]", state.draft.to)
    } else if state.draft.to.is_empty() {
        "[________________________________________________]".to_string()
    } else {
        format!("[{}]", state.draft.to)
    };
    let to_style = if is_to { edit_style } else { normal };
    view.render_row(
        frame,
        1,
        vec![Span::styled(format!("         {}", to_display), to_style)],
    );

    // Subject field
    let is_subject = state.compose_field == ComposeField::Subject;
    view.render_row(frame, 3, vec![Span::styled("Subject: ", label_style)]);
    let subject_display = if is_subject {
        format!("[{}|]", state.draft.subject)
    } else if state.draft.subject.is_empty() {
        "[________________________________________________]".to_string()
    } else {
        format!("[{}]", state.draft.subject)
    };
    let subject_style = if is_subject { edit_style } else { normal };
    view.render_row(
        frame,
        4,
        vec![Span::styled(
            format!("         {}", subject_display),
            subject_style,
        )],
    );

    // Separator
    view.render_row(
        frame,
        6,
        vec![Span::styled(
            "-".repeat(area.width.saturating_sub(4) as usize),
            Style::default().fg(colors.grey()),
        )],
    );

    // Body field
    let is_body = state.compose_field == ComposeField::Body;
    let body_style = if is_body { highlight } else { normal };

    let body_lines: Vec<&str> = state.draft.body.lines().collect();
    let max_body_rows = view.content_height().saturating_sub(10) as usize;

    for (i, line) in body_lines.iter().take(max_body_rows).enumerate() {
        view.render_row(frame, 7 + i as u16, vec![Span::styled(*line, body_style)]);
    }

    // Show cursor in body if editing
    if is_body && body_lines.is_empty() {
        view.render_row(frame, 7, vec![Span::styled("|", edit_style)]);
    }

    // Status bar
    let status_y = view.content_height().saturating_sub(2);
    let field_name = match state.compose_field {
        ComposeField::To => "To",
        ComposeField::Subject => "Subject",
        ComposeField::Body => "Body",
    };
    let status = format!("Editing: {} | Ln 1, Col 1", field_name);
    view.render_row(
        frame,
        status_y,
        vec![Span::styled(status, Style::default().fg(colors.grey()))],
    );

    view.render_help(
        frame,
        vec![
            ("Tab", "next field"),
            ("Ctrl+Enter", "send"),
            ("Esc", "discard"),
        ],
    );
}

// =============================================================================
// HELP VIEW
// =============================================================================

pub fn draw_help(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-MAIL: Help ", colors);
    view.render_frame(frame);

    let header_style = Style::default()
        .fg(colors.cyan())
        .add_modifier(Modifier::BOLD);
    let normal = Style::default().fg(colors.fg());
    let key_style = Style::default().fg(colors.yellow());

    let help_text = [
        ("Q-MAIL - Email Client", header_style),
        ("", normal),
        ("A terminal email client with IMAP/SMTP support.", normal),
        ("Currently running in mock mode for demonstration.", normal),
        ("", normal),
        ("Folder List:", header_style),
        ("  Up/Down    Navigate folders", normal),
        ("  Enter      Open folder", normal),
        ("  C          Compose new message", normal),
        ("  ?          Show this help", normal),
        ("", normal),
        ("Message List:", header_style),
        ("  Up/Down    Navigate messages", normal),
        ("  Enter      Read message", normal),
        ("  C          Compose new message", normal),
        ("  D          Delete message", normal),
        ("  A          Archive message", normal),
        ("  U          Toggle read/unread", normal),
        ("", normal),
        ("Message Reader:", header_style),
        ("  Up/Down    Scroll message", normal),
        ("  R          Reply to message", normal),
        ("  D          Delete message", normal),
        ("  A          Archive message", normal),
        ("", normal),
        ("Compose:", header_style),
        ("  Tab        Next field", normal),
        ("  Ctrl+Enter Send message", normal),
        ("  Esc        Discard draft", normal),
    ];

    for (i, (text, style)) in help_text.iter().enumerate() {
        if i as u16 + 1 >= view.content_height() {
            break;
        }
        // Key binding line
        if text.starts_with("  ") {
            let parts: Vec<&str> = text.splitn(2, "  ").collect();
            if parts.len() == 2 {
                view.render_row(
                    frame,
                    i as u16,
                    vec![
                        Span::styled(format!("  {:10}", parts[0].trim()), key_style),
                        Span::styled(parts[1], *style),
                    ],
                );
                continue;
            }
        }
        view.render_row(
            frame,
            i as u16,
            vec![Span::styled(format!("  {}", text), *style)],
        );
    }

    view.render_help(frame, vec![("Esc", "back")]);
}

// =============================================================================
// MAIN DRAW FUNCTION
// =============================================================================

pub fn draw_qmail(state: &QMailState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    match state.view {
        QMailView::AccountSetup => draw_account_setup(state, frame, area, colors),
        QMailView::FolderList => draw_folder_list(state, frame, area, colors),
        QMailView::MessageList => draw_message_list(state, frame, area, colors),
        QMailView::MessageRead => draw_message_read(state, frame, area, colors),
        QMailView::Compose => draw_compose(state, frame, area, colors),
        QMailView::Help => draw_help(frame, area, colors),
    }
}
