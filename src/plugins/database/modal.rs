//! Database plugin modal rendering

use super::state::{ConnectField, DatabaseState, DatabaseType, DatabaseView};
use crate::app::ThemeColors;
use crate::ui::components::{FullScreenView, ModalFrame};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Frame,
};

fn centered_modal_area(area: Rect) -> Rect {
    let width = area.width.min(60);
    let height = area.height.min(16);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

pub fn draw_database_modal(
    frame: &mut Frame,
    area: Rect,
    state: &DatabaseState,
    colors: &ThemeColors,
) {
    match state.view {
        DatabaseView::TypeSelect => draw_type_select(frame, area, state, colors),
        DatabaseView::Profiles => draw_profiles_view(frame, area, state, colors),
        DatabaseView::Tables => draw_tables_view(frame, area, state, colors),
        DatabaseView::TableDetail => draw_table_detail(frame, area, state, colors),
        DatabaseView::Query => draw_query_view(frame, area, state, colors),
        DatabaseView::Results => draw_results_view(frame, area, state, colors),
        DatabaseView::Connect => draw_connect_view(frame, area, state, colors),
        DatabaseView::SaveProfile => draw_save_profile(frame, area, state, colors),
        DatabaseView::Error => draw_error(frame, area, state, colors),
    }
}

fn draw_type_select(frame: &mut Frame, area: Rect, state: &DatabaseState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " Database - Select Type ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    let types = DatabaseState::available_types();

    let rows: Vec<Row> = types
        .iter()
        .enumerate()
        .map(|(i, db_type)| {
            let description = match db_type {
                DatabaseType::SQLite => "Local file database",
                DatabaseType::PostgreSQL => "PostgreSQL server",
                DatabaseType::MySQL => "MySQL/MariaDB server",
            };

            let style = if i == state.selected_type_idx {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(colors.fg())
            };

            Row::new(vec![db_type.name().to_string(), description.to_string()]).style(style)
        })
        .collect();

    let widths = [Constraint::Percentage(40), Constraint::Percentage(60)];

    let table = Table::new(rows, widths)
        .header(Row::new(vec!["Database", "Description"]).style(Style::default().fg(colors.blue())))
        .block(Block::default());

    let mut table_state = TableState::default();
    table_state.select(Some(state.selected_type_idx));
    frame.render_stateful_widget(table, content_area, &mut table_state);

    // Show profiles option if profiles exist
    let mut help = vec![("↑/↓", "select"), ("Enter", "open")];
    if !state.profiles.is_empty() {
        help.push(("P", "profiles"));
    }
    help.push(("Esc", "close"));
    modal.render_help(frame, help);
}

fn draw_profiles_view(frame: &mut Frame, area: Rect, state: &DatabaseState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " Database - Saved Profiles ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    if state.profiles.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No saved profiles",
                Style::default().fg(colors.yellow()),
            )),
        ];
        let para = Paragraph::new(text);
        frame.render_widget(para, content_area);
    } else {
        let rows: Vec<Row> = state
            .profiles
            .iter()
            .enumerate()
            .map(|(i, profile)| {
                let db_type = match profile.db_type.as_str() {
                    "postgresql" => "PostgreSQL",
                    "mysql" => "MySQL",
                    _ => &profile.db_type,
                };
                let host_info = format!("{}:{}", profile.config.host, profile.config.port);

                let style = if i == state.selected_profile {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };

                Row::new(vec![profile.name.clone(), db_type.to_string(), host_info]).style(style)
            })
            .collect();

        let widths = [
            Constraint::Percentage(40),
            Constraint::Percentage(25),
            Constraint::Percentage(35),
        ];

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["Name", "Type", "Host"]).style(Style::default().fg(colors.blue())),
            )
            .block(Block::default());

        let mut table_state = TableState::default();
        table_state.select(Some(state.selected_profile));
        frame.render_stateful_widget(table, content_area, &mut table_state);
    }

    modal.render_help(
        frame,
        vec![
            ("↑/↓", "select"),
            ("Enter", "connect"),
            ("D", "delete"),
            ("Esc", "back"),
        ],
    );
}

fn draw_save_profile(frame: &mut Frame, area: Rect, state: &DatabaseState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " Save Connection Profile ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Min(1),
    ])
    .split(content_area);

    // Instructions
    let instruction = Line::from(Span::styled(
        "Enter a name for this connection profile:",
        Style::default().fg(colors.grey()),
    ));
    frame.render_widget(Paragraph::new(instruction), chunks[0]);

    // Name input
    let display_value = if state.profile_name.is_empty() {
        "".to_string()
    } else {
        state.profile_name.clone()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors.yellow()))
        .title(" Name ");

    let display_with_cursor = format!("{}|", display_value);
    let para = Paragraph::new(Line::from(Span::styled(
        display_with_cursor,
        Style::default().fg(colors.yellow()),
    )))
    .block(block);

    frame.render_widget(para, chunks[1]);

    modal.render_help(frame, vec![("Enter", "save"), ("Esc", "cancel")]);
}

fn draw_tables_view(frame: &mut Frame, area: Rect, state: &DatabaseState, colors: &ThemeColors) {
    let title = format!(" {} - Tables ", state.db_name);
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);
    let content_area = view.content_area();

    if state.tables.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No tables found",
                Style::default().fg(colors.yellow()),
            )),
        ];
        let para = Paragraph::new(text);
        frame.render_widget(para, content_area);
    } else {
        // Build table rows
        let rows: Vec<Row> = state
            .tables
            .iter()
            .enumerate()
            .map(|(i, table)| {
                let row_count = table
                    .row_count
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "?".to_string());
                let col_count = table.columns.len().to_string();

                let style = if i == state.selected_table {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };

                Row::new(vec![table.name.clone(), col_count, row_count]).style(style)
            })
            .collect();

        let widths = [
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ];

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["Table", "Columns", "Rows"])
                    .style(Style::default().fg(colors.blue())),
            )
            .block(Block::default());

        let mut table_state = TableState::default();
        table_state.select(Some(state.selected_table));
        frame.render_stateful_widget(table, content_area, &mut table_state);
    }

    view.render_help(
        frame,
        vec![
            ("↑/↓", "select"),
            ("Enter", "browse"),
            ("Q", "query"),
            ("Esc", "close"),
        ],
    );
}

fn draw_table_detail(frame: &mut Frame, area: Rect, state: &DatabaseState, colors: &ThemeColors) {
    let table_name = state
        .current_table
        .as_ref()
        .map(|t| t.name.as_str())
        .unwrap_or("?");
    let title = format!(" {} ", table_name);
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);
    let content_area = view.content_area();

    if let Some(ref table) = state.current_table {
        // Show column info
        let rows: Vec<Row> = table
            .columns
            .iter()
            .map(|col| {
                let pk = if col.primary_key { "PK" } else { "" };
                let null = if col.nullable { "NULL" } else { "NOT NULL" };
                let default = col
                    .default_value
                    .as_ref()
                    .map(|d| format!("= {}", d))
                    .unwrap_or_default();

                Row::new(vec![
                    col.name.clone(),
                    col.data_type.clone(),
                    pk.to_string(),
                    null.to_string(),
                    default,
                ])
                .style(Style::default().fg(colors.fg()))
            })
            .collect();

        let widths = [
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(10),
            Constraint::Percentage(15),
            Constraint::Percentage(25),
        ];

        let column_table = Table::new(rows, widths)
            .header(
                Row::new(vec!["Column", "Type", "Key", "Null", "Default"])
                    .style(Style::default().fg(colors.blue())),
            )
            .block(Block::default());

        // Split area for columns and row count
        let chunks =
            Layout::vertical([Constraint::Min(5), Constraint::Length(2)]).split(content_area);

        frame.render_widget(column_table, chunks[0]);

        // Row count info
        let row_info = if let Some(count) = table.row_count {
            format!("{} rows", count)
        } else {
            "? rows".to_string()
        };
        let info_line = Line::from(vec![
            Span::styled("Rows: ", Style::default().fg(colors.grey())),
            Span::styled(row_info, Style::default().fg(colors.green())),
        ]);
        frame.render_widget(Paragraph::new(info_line), chunks[1]);
    }

    view.render_help(
        frame,
        vec![("S", "SELECT *"), ("Q", "query"), ("Esc", "back")],
    );
}

fn draw_query_view(frame: &mut Frame, area: Rect, state: &DatabaseState, colors: &ThemeColors) {
    let title = format!(" {} - Query ", state.db_name);
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);
    let content_area = view.content_area();

    // Split into query input and hint areas
    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).split(content_area);

    // Query input with cursor
    let query_display = if state.query.is_empty() {
        Span::styled("Enter SQL query...", Style::default().fg(colors.grey()))
    } else {
        // Show query with cursor
        let before = &state.query[..state.query_cursor];
        let after = &state.query[state.query_cursor..];
        Span::styled(
            format!("{}|{}", before, after),
            Style::default().fg(colors.fg()),
        )
    };

    let query_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors.blue()))
        .title(" SQL ");

    let query_para = Paragraph::new(Line::from(query_display)).block(query_block);
    frame.render_widget(query_para, chunks[0]);

    // Hint
    let hint = Line::from(vec![
        Span::styled("Tip: ", Style::default().fg(colors.grey())),
        Span::styled(
            "Press Enter to execute, Esc to cancel",
            Style::default().fg(colors.green()),
        ),
    ]);
    frame.render_widget(Paragraph::new(hint), chunks[1]);

    view.render_help(frame, vec![("Enter", "execute"), ("Esc", "cancel")]);
}

fn draw_results_view(frame: &mut Frame, area: Rect, state: &DatabaseState, colors: &ThemeColors) {
    let title = format!(" {} - Results ", state.db_name);
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);
    let content_area = view.content_area();

    if let Some(ref results) = state.results {
        // Split into results table and status bar
        let chunks =
            Layout::vertical([Constraint::Min(5), Constraint::Length(1)]).split(content_area);

        if results.rows.is_empty() {
            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No results",
                    Style::default().fg(colors.yellow()),
                )),
            ];
            let para = Paragraph::new(text);
            frame.render_widget(para, chunks[0]);
        } else {
            // Calculate column widths based on content
            let col_count = results.columns.len();
            let col_width = 100 / col_count.max(1);
            let widths: Vec<Constraint> = (0..col_count)
                .map(|_| Constraint::Percentage(col_width as u16))
                .collect();

            // Build rows
            let rows: Vec<Row> = results
                .rows
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    let style = if i == state.selected_row {
                        Style::default().fg(colors.yellow()).bg(colors.red())
                    } else {
                        Style::default().fg(colors.fg())
                    };

                    // Truncate long values
                    let cells: Vec<String> = row
                        .values
                        .iter()
                        .map(|v| {
                            if v.len() > 30 {
                                format!("{}...", &v[..27])
                            } else {
                                v.clone()
                            }
                        })
                        .collect();

                    Row::new(cells).style(style)
                })
                .collect();

            let results_table = Table::new(rows, widths)
                .header(Row::new(results.columns.clone()).style(Style::default().fg(colors.blue())))
                .block(Block::default());

            let mut table_state = TableState::default();
            table_state.select(Some(state.selected_row));
            frame.render_stateful_widget(results_table, chunks[0], &mut table_state);
        }

        // Status bar
        let status = format!(
            "{} rows in {}ms",
            results.row_count, results.execution_time_ms
        );
        let status_line = Line::from(Span::styled(status, Style::default().fg(colors.green())));
        frame.render_widget(Paragraph::new(status_line), chunks[1]);
    }

    view.render_help(
        frame,
        vec![("↑/↓", "scroll"), ("Q", "new query"), ("Esc", "tables")],
    );
}

fn draw_connect_view(frame: &mut Frame, area: Rect, state: &DatabaseState, colors: &ThemeColors) {
    let title = format!(" {} - Connect ", state.db_name);
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);
    let content_area = view.content_area();

    // Split into form fields
    let chunks = Layout::vertical([
        Constraint::Length(3), // Host
        Constraint::Length(3), // Port
        Constraint::Length(3), // User
        Constraint::Length(3), // Password
        Constraint::Length(3), // Database
        Constraint::Min(1),    // Spacer
    ])
    .split(content_area);

    // Helper to draw a form field
    let draw_field = |frame: &mut Frame,
                      area: Rect,
                      label: &str,
                      value: &str,
                      selected: bool,
                      is_password: bool| {
        let display_value = if is_password && !value.is_empty() {
            "*".repeat(value.len())
        } else if value.is_empty() {
            "".to_string()
        } else {
            value.to_string()
        };

        let border_color = if selected {
            colors.yellow()
        } else {
            colors.grey()
        };

        let text_color = if selected {
            colors.yellow()
        } else {
            colors.fg()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(format!(" {} ", label));

        let display_with_cursor = if selected {
            format!("{}|", display_value)
        } else {
            display_value
        };

        let para = Paragraph::new(Line::from(Span::styled(
            display_with_cursor,
            Style::default().fg(text_color),
        )))
        .block(block);

        frame.render_widget(para, area);
    };

    // Draw each field
    draw_field(
        frame,
        chunks[0],
        "Host",
        &state.connection.host,
        state.connect_field == ConnectField::Host,
        false,
    );
    draw_field(
        frame,
        chunks[1],
        "Port",
        &state.connection.port.to_string(),
        state.connect_field == ConnectField::Port,
        false,
    );
    draw_field(
        frame,
        chunks[2],
        "User",
        &state.connection.user,
        state.connect_field == ConnectField::User,
        false,
    );
    draw_field(
        frame,
        chunks[3],
        "Password",
        &state.connection.password,
        state.connect_field == ConnectField::Password,
        true,
    );
    draw_field(
        frame,
        chunks[4],
        "Database",
        &state.connection.database,
        state.connect_field == ConnectField::Database,
        false,
    );

    view.render_help(
        frame,
        vec![
            ("Tab/↓", "next"),
            ("Enter", "connect"),
            ("F2", "save"),
            ("Esc", "cancel"),
        ],
    );
}

fn draw_error(frame: &mut Frame, area: Rect, state: &DatabaseState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " Database - Error ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    let error_msg = state.error.as_deref().unwrap_or("Unknown error");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled("Error:", Style::default().fg(colors.red()))),
        Line::from(""),
        Line::from(Span::styled(error_msg, Style::default().fg(colors.fg()))),
    ];

    let para = Paragraph::new(text);
    frame.render_widget(para, content_area);

    modal.render_help(frame, vec![("Enter/Esc", "close")]);
}
