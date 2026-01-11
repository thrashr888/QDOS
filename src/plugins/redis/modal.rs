//! Redis plugin modal rendering

use super::state::{ConnectField, RedisState, RedisValue, RedisView};
use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    Frame,
};

/// Draw the Redis modal
pub fn draw_redis_modal(frame: &mut Frame, area: Rect, state: &RedisState, colors: &ThemeColors) {
    let title = if state.connected {
        format!(" Redis - {} ", state.connection.display_name())
    } else {
        " Redis ".to_string()
    };

    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    match state.view {
        RedisView::Connect => draw_connect(frame, &view, state, colors),
        RedisView::Profiles => draw_profiles(frame, &view, state, colors),
        RedisView::SaveProfile => draw_save_profile(frame, &view, state, colors),
        RedisView::KeyBrowser => draw_key_browser(frame, &view, state, colors),
        RedisView::KeyDetail => draw_key_detail(frame, &view, state, colors),
        RedisView::ServerInfo => draw_server_info(frame, &view, state, colors),
        RedisView::Confirm => draw_confirm(frame, &view, state, colors),
        RedisView::Error => draw_error(frame, &view, state, colors),
    }
}

fn draw_connect(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &RedisState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Loading state
    if state.loading {
        let msg = state.loading_message.as_deref().unwrap_or("Connecting...");
        view.render_row(
            frame,
            (content_height / 2) as u16,
            vec![Span::styled(msg, Style::default().fg(colors.yellow()))],
        );
        return;
    }

    // Error message
    if let Some(err) = &state.error {
        view.render_row(
            frame,
            0,
            vec![Span::styled(
                format!("Error: {}", err),
                Style::default().fg(colors.red()),
            )],
        );
    }

    // Title
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            "Connect to Redis",
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );

    // Form fields
    let fields = [
        (ConnectField::Host, "Host:", &state.connection.host),
        (
            ConnectField::Port,
            "Port:",
            &state.connection.port.to_string(),
        ),
    ];

    let mut row = 4u16;
    for (field, label, value) in &fields {
        let is_selected = *field == state.connect_field;
        let label_style = Style::default().fg(colors.grey());
        let value_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg())
        };

        view.render_row(
            frame,
            row,
            vec![
                Span::styled(format!("{:<12}", label), label_style),
                Span::styled(value.to_string(), value_style),
            ],
        );
        row += 1;
    }

    // Password field (masked)
    let is_selected = state.connect_field == ConnectField::Password;
    let pass_display = state
        .connection
        .password
        .as_ref()
        .map(|p| "*".repeat(p.len()))
        .unwrap_or_default();
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Password:   ", Style::default().fg(colors.grey())),
            Span::styled(
                if pass_display.is_empty() {
                    "(none)".to_string()
                } else {
                    pass_display
                },
                if is_selected {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                },
            ),
        ],
    );
    row += 1;

    // Database field
    let is_selected = state.connect_field == ConnectField::Database;
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Database:   ", Style::default().fg(colors.grey())),
            Span::styled(
                state.connection.database.to_string(),
                if is_selected {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                },
            ),
        ],
    );
    row += 1;

    // TLS field
    let is_selected = state.connect_field == ConnectField::Tls;
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("TLS:        ", Style::default().fg(colors.grey())),
            Span::styled(
                if state.connection.tls { "Yes" } else { "No" },
                if is_selected {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                },
            ),
        ],
    );
    row += 1;

    // Name field
    let is_selected = state.connect_field == ConnectField::Name;
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Name:       ", Style::default().fg(colors.grey())),
            Span::styled(
                if state.connection.name.is_empty() {
                    "(optional)".to_string()
                } else {
                    state.connection.name.clone()
                },
                if is_selected {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.grey())
                },
            ),
        ],
    );

    // Help footer
    let help = vec![
        ("Enter", "connect"),
        ("Tab", "next field"),
        ("p", "profiles"),
        ("Esc", "close"),
    ];
    view.render_help(frame, help);
}

fn draw_profiles(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &RedisState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    view.render_row(
        frame,
        0,
        vec![Span::styled(
            "Saved Profiles",
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );

    if state.profiles.is_empty() {
        view.render_row(
            frame,
            2,
            vec![Span::styled(
                "No saved profiles",
                Style::default().fg(colors.grey()),
            )],
        );
    } else {
        for (i, profile) in state.profiles.iter().enumerate().take(content_height - 3) {
            let is_selected = i == state.selected_profile;
            let style = if is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(colors.fg())
            };

            view.render_row(
                frame,
                (i + 2) as u16,
                vec![Span::styled(profile.display_name(), style)],
            );
        }
    }

    let help = vec![("Enter", "connect"), ("d", "delete"), ("Esc", "back")];
    view.render_help(frame, help);
}

fn draw_save_profile(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &RedisState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    view.render_row(
        frame,
        (content_height / 2) as u16,
        vec![
            Span::styled("Save as: ", Style::default().fg(colors.cyan())),
            Span::styled(&state.connection.name, Style::default().fg(colors.fg())),
        ],
    );

    let help = vec![("Enter", "save"), ("Esc", "cancel")];
    view.render_help(frame, help);
}

fn draw_key_browser(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &RedisState,
    colors: &ThemeColors,
) {
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

    // Filter bar
    let filter_spans = vec![
        Span::styled("Filter: ", Style::default().fg(colors.grey())),
        Span::styled(&state.key_filter, Style::default().fg(colors.fg())),
        Span::styled(" | Type: ", Style::default().fg(colors.grey())),
        Span::styled(state.type_filter.name(), Style::default().fg(colors.cyan())),
    ];
    view.render_row(frame, 0, filter_spans);

    // Header row
    let header = Line::from(vec![
        Span::styled(
            format!("{:<3}", "T"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<50}", "Key"),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "TTL",
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    view.render_row(frame, 2, header.spans);

    // Keys
    let visible = state.visible_keys();
    if visible.is_empty() {
        view.render_row(
            frame,
            4,
            vec![Span::styled(
                if state.key_filter.is_empty() {
                    "No keys found"
                } else {
                    "No matching keys"
                },
                Style::default().fg(colors.grey()),
            )],
        );
    } else {
        let start = state.key_scroll;
        let end = (start + content_height - 4).min(visible.len());

        for (i, key) in visible[start..end].iter().enumerate() {
            let is_selected = start + i == state.selected_key;

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

            let name_display = if key.name.len() > 48 {
                format!("{:.48}..", key.name)
            } else {
                format!("{:<50}", key.name)
            };

            let ttl_display = match key.ttl {
                Some(-1) => "-".to_string(),
                Some(-2) => "expired".to_string(),
                Some(t) => format!("{}s", t),
                None => "?".to_string(),
            };

            let row = Line::from(vec![
                Span::styled(format!("{:<3}", key.key_type.symbol()), type_style),
                Span::styled(name_display, base_style),
                Span::styled(ttl_display, base_style),
            ]);

            view.render_row(frame, (i + 3) as u16, row.spans);
        }
    }

    // Status line
    let status = format!(
        "{}/{} keys{}",
        state.selected_key + 1,
        visible.len(),
        if state.scan_complete {
            ""
        } else {
            " (loading...)"
        }
    );
    if let Some(msg) = &state.message {
        view.render_row(
            frame,
            (content_height - 1) as u16,
            vec![Span::styled(msg, Style::default().fg(colors.green()))],
        );
    } else {
        view.render_row(
            frame,
            (content_height - 1) as u16,
            vec![Span::styled(status, Style::default().fg(colors.grey()))],
        );
    }

    // Help footer
    let help = vec![
        ("/", "filter"),
        ("T", "type"),
        ("Enter", "view"),
        ("d", "delete"),
        ("I", "info"),
        ("Esc", "close"),
    ];
    view.render_help(frame, help);
}

fn draw_key_detail(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &RedisState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    // Key info header
    if let Some(key) = &state.current_key {
        view.render_row(
            frame,
            0,
            vec![
                Span::styled("Key: ", Style::default().fg(colors.grey())),
                Span::styled(&key.name, Style::default().fg(colors.cyan())),
                Span::styled(
                    format!(" ({})", key.key_type.name()),
                    Style::default().fg(colors.grey()),
                ),
            ],
        );

        if let Some(ttl) = key.ttl {
            let ttl_text = if ttl == -1 {
                "no expiry".to_string()
            } else if ttl == -2 {
                "expired".to_string()
            } else {
                format!("{}s", ttl)
            };
            view.render_row(
                frame,
                1,
                vec![
                    Span::styled("TTL: ", Style::default().fg(colors.grey())),
                    Span::styled(ttl_text, Style::default().fg(colors.fg())),
                ],
            );
        }
    }

    // Value display
    let start_row = 3u16;
    match &state.current_value {
        RedisValue::String(s) => {
            // Multi-line string display
            for (i, line) in s.lines().enumerate().take(content_height - 5) {
                let row = (start_row + i as u16).min(content_height as u16 - 2);
                view.render_row(
                    frame,
                    row,
                    vec![Span::styled(line, Style::default().fg(colors.fg()))],
                );
            }
        }
        RedisValue::List(items) | RedisValue::Set(items) | RedisValue::Stream(items) => {
            for (i, item) in items
                .iter()
                .enumerate()
                .skip(state.detail_scroll)
                .take(content_height - 5)
            {
                let row =
                    (start_row + (i - state.detail_scroll) as u16).min(content_height as u16 - 2);
                view.render_row(
                    frame,
                    row,
                    vec![
                        Span::styled(format!("{}: ", i), Style::default().fg(colors.grey())),
                        Span::styled(item, Style::default().fg(colors.fg())),
                    ],
                );
            }
        }
        RedisValue::ZSet(items) => {
            for (i, (member, score)) in items
                .iter()
                .enumerate()
                .skip(state.detail_scroll)
                .take(content_height - 5)
            {
                let row =
                    (start_row + (i - state.detail_scroll) as u16).min(content_height as u16 - 2);
                view.render_row(
                    frame,
                    row,
                    vec![
                        Span::styled(
                            format!("{:.2}: ", score),
                            Style::default().fg(colors.cyan()),
                        ),
                        Span::styled(member, Style::default().fg(colors.fg())),
                    ],
                );
            }
        }
        RedisValue::Hash(items) => {
            for (i, (field, value)) in items
                .iter()
                .enumerate()
                .skip(state.detail_scroll)
                .take(content_height - 5)
            {
                let row =
                    (start_row + (i - state.detail_scroll) as u16).min(content_height as u16 - 2);
                view.render_row(
                    frame,
                    row,
                    vec![
                        Span::styled(format!("{}: ", field), Style::default().fg(colors.cyan())),
                        Span::styled(value, Style::default().fg(colors.fg())),
                    ],
                );
            }
        }
        RedisValue::None => {
            view.render_row(
                frame,
                start_row,
                vec![Span::styled(
                    "(no value)",
                    Style::default().fg(colors.grey()),
                )],
            );
        }
    }

    let help = vec![("Esc", "back")];
    view.render_help(frame, help);
}

fn draw_server_info(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &RedisState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height() as usize;

    view.render_row(
        frame,
        0,
        vec![Span::styled(
            "Server Information",
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );

    let start = state.info_scroll;
    let end = (start + content_height - 3).min(state.server_info.len());

    for (i, line) in state.server_info[start..end].iter().enumerate() {
        let style = if line.starts_with('#') {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else if line.contains(':') {
            Style::default().fg(colors.fg())
        } else {
            Style::default().fg(colors.grey())
        };

        view.render_row(frame, (i + 2) as u16, vec![Span::styled(line, style)]);
    }

    let help = vec![("Esc", "back")];
    view.render_help(frame, help);
}

fn draw_confirm(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &RedisState,
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

fn draw_error(frame: &mut Frame, view: &FullScreenView, state: &RedisState, colors: &ThemeColors) {
    let content_height = view.content_height() as usize;

    if let Some(err) = &state.error {
        view.render_row(
            frame,
            (content_height / 2) as u16,
            vec![Span::styled(
                format!("Error: {}", err),
                Style::default().fg(colors.red()),
            )],
        );
    }

    let help = vec![("Enter/Esc", "back")];
    view.render_help(frame, help);
}
