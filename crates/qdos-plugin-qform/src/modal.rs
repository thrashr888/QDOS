//! Q-FORM UI rendering

use crate::state::{DesignerMode, ExportFormat, FieldType, QFormState, QFormView};
use qdos_plugin_api::prelude::{FullScreenView, ThemeColors};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

// =============================================================================
// FORM LIST VIEW
// =============================================================================

pub fn draw_form_list(state: &QFormState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-FORM ", colors);
    view.render_frame(frame);

    let normal = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(colors.grey());

    view.render_row(
        frame,
        0,
        vec![Span::styled(
            "YOUR FORMS",
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );

    view.render_row(
        frame,
        1,
        vec![Span::styled(
            "----------",
            Style::default().fg(colors.grey()),
        )],
    );

    if state.forms.is_empty() {
        view.render_row(
            frame,
            3,
            vec![Span::styled(
                "No forms yet. Press N to create one or T for templates.",
                desc_style,
            )],
        );
    } else {
        for (i, form) in state.forms.iter().enumerate() {
            let is_selected = i == state.form_cursor;
            let style = if is_selected { highlight } else { normal };
            let marker = if is_selected { ">" } else { " " };

            let record_count = state
                .records
                .iter()
                .filter(|r| r.form_id == form.id)
                .count();
            let field_count = form.fields.len();

            let line = format!(
                " {} [{}] {:30} {:2} fields  {:4} records",
                marker,
                if is_selected { "x" } else { " " },
                form.title,
                field_count,
                record_count,
            );

            view.render_row(frame, 3 + i as u16, vec![Span::styled(line, style)]);
        }
    }

    // Status bar
    let status_y = view.content_height().saturating_sub(2);
    let status = format!("{} forms", state.forms.len());
    view.render_row(frame, status_y, vec![Span::styled(status, desc_style)]);

    view.render_help(
        frame,
        vec![
            ("N", "new"),
            ("T", "template"),
            ("Enter", "open"),
            ("D", "design"),
            ("E", "entry"),
            ("R", "records"),
            ("Del", "delete"),
            ("Esc", "exit"),
        ],
    );
}

// =============================================================================
// DESIGNER VIEW
// =============================================================================

pub fn draw_designer(state: &QFormState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let title = if let Some(form) = state.current_form() {
        format!(" Q-FORM: {} (Design) ", form.title)
    } else {
        " Q-FORM: Designer ".to_string()
    };
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    let normal = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let type_style = Style::default().fg(colors.cyan());
    let validation_style = Style::default().fg(colors.green());
    let edit_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::UNDERLINED);

    if let Some(form) = state.current_form() {
        if form.fields.is_empty() {
            view.render_row(
                frame,
                1,
                vec![Span::styled(
                    "No fields yet. Press A to add one.",
                    Style::default().fg(colors.grey()),
                )],
            );
        } else {
            for (i, field) in form.fields.iter().enumerate() {
                let is_selected = i == state.designer_field;
                let label_style = if is_selected { highlight } else { normal };

                // Check if we're editing this field's label
                let label_text = if is_selected && state.designer_mode == DesignerMode::EditLabel {
                    format!("[{}|]", state.field_edit_buffer)
                } else {
                    format!("[{}]", field.label)
                };

                let label_display_style =
                    if is_selected && state.designer_mode == DesignerMode::EditLabel {
                        edit_style
                    } else {
                        label_style
                    };

                let validation = if field.validation.required {
                    "Required"
                } else {
                    "Optional"
                };

                let marker = if is_selected { ">" } else { " " };
                let row = 1 + i as u16 * 2;

                // Main field line
                view.render_row(
                    frame,
                    row,
                    vec![
                        Span::styled(format!("{} Field {}: ", marker, i + 1), normal),
                        Span::styled(format!("{:20}", label_text), label_display_style),
                        Span::styled(format!("  {:12}", field.field_type.name()), type_style),
                        Span::styled(format!("  {}", validation), validation_style),
                    ],
                );

                // Options line for choice fields
                if let FieldType::Choice { options, multi } = &field.field_type {
                    let opts_str = options.join(", ");
                    let multi_str = if *multi { "(multi)" } else { "" };
                    view.render_row(
                        frame,
                        row + 1,
                        vec![Span::styled(
                            format!("           Options: {} {}", opts_str, multi_str),
                            Style::default().fg(colors.grey()),
                        )],
                    );
                }

                // Rows for textarea
                if let FieldType::TextArea { rows } = &field.field_type {
                    view.render_row(
                        frame,
                        row + 1,
                        vec![Span::styled(
                            format!("           ({} rows)", rows),
                            Style::default().fg(colors.grey()),
                        )],
                    );
                }
            }
        }

        // Add field prompt
        let add_row = 1 + form.fields.len() as u16 * 2 + 1;
        if state.designer_mode == DesignerMode::AddField {
            view.render_row(
                frame,
                add_row,
                vec![
                    Span::styled("   [+ Add Field: ", normal),
                    Span::styled(format!("{}|", state.field_edit_buffer), edit_style),
                    Span::styled("]", normal),
                ],
            );
        } else {
            view.render_row(
                frame,
                add_row,
                vec![Span::styled(
                    "   [+ Add Field (A)]",
                    Style::default().fg(colors.grey()),
                )],
            );
        }

        // Type selection popup
        if state.designer_mode == DesignerMode::EditType {
            draw_type_selector(state, frame, area, colors);
        }
    }

    // Status bar
    let status_y = view.content_height().saturating_sub(2);
    let status = match state.designer_mode {
        DesignerMode::Navigate => "Navigate: Use arrows to select fields",
        DesignerMode::EditLabel => "Edit: Type label, Enter to confirm, Esc to cancel",
        DesignerMode::EditType => "Type: Use arrows to select, Enter to confirm",
        DesignerMode::EditOptions => "Options: Edit choice options",
        DesignerMode::AddField => "Add: Type field label, Enter to add",
    };
    view.render_row(
        frame,
        status_y,
        vec![Span::styled(status, Style::default().fg(colors.grey()))],
    );

    let help = match state.designer_mode {
        DesignerMode::Navigate => vec![
            ("^v", "up/down"),
            ("A", "add"),
            ("E", "edit"),
            ("T", "type"),
            ("Del", "remove"),
            ("Esc", "back"),
        ],
        DesignerMode::EditLabel | DesignerMode::AddField => {
            vec![("Enter", "confirm"), ("Esc", "cancel")]
        }
        DesignerMode::EditType => vec![("^v", "select"), ("Enter", "confirm"), ("Esc", "cancel")],
        DesignerMode::EditOptions => vec![("Enter", "confirm"), ("Esc", "cancel")],
    };

    view.render_help(frame, help);
}

fn draw_type_selector(state: &QFormState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    // Draw a popup for type selection
    let types = FieldType::all_types();
    let popup_width = 30u16;
    let popup_height = (types.len() + 2) as u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Draw border
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Select Type ")
        .border_style(Style::default().fg(colors.cyan()))
        .title_style(Style::default().fg(colors.yellow()));
    frame.render_widget(block, popup_area);

    // Draw options
    for (i, field_type) in types.iter().enumerate() {
        let is_selected = i == state.type_cursor;
        let style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg())
        };
        let marker = if is_selected { ">" } else { " " };
        let text = format!(" {} {}", marker, field_type.name());
        let para = Paragraph::new(text).style(style);
        frame.render_widget(
            para,
            Rect::new(popup_x + 1, popup_y + 1 + i as u16, popup_width - 2, 1),
        );
    }
}

// =============================================================================
// ENTRY VIEW
// =============================================================================

pub fn draw_entry(state: &QFormState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let title = if let Some(form) = state.current_form() {
        format!(" Q-FORM: {} (Entry) ", form.title)
    } else {
        " Q-FORM: Entry ".to_string()
    };
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    let normal = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(colors.cyan());
    let error_style = Style::default().fg(colors.red());
    let edit_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::UNDERLINED);

    if let Some(form) = state.current_form() {
        let mut row = 1u16;

        for (i, field) in form.fields.iter().enumerate() {
            let is_current = i == state.entry_field;
            let value = state
                .entry_values
                .get(&field.id)
                .cloned()
                .unwrap_or_default();
            let error = state.entry_errors.get(&field.id);

            // Label
            let label_text = if field.validation.required {
                format!("{}*:", field.label)
            } else {
                format!("{}:", field.label)
            };
            view.render_row(
                frame,
                row,
                vec![Span::styled(format!("   {:20}", label_text), label_style)],
            );
            row += 1;

            // Value based on field type
            match &field.field_type {
                FieldType::Text { .. } | FieldType::Number { .. } | FieldType::Date { .. } => {
                    let display_value = if is_current {
                        format!("[{}|]", value)
                    } else if value.is_empty() {
                        "[________________]".to_string()
                    } else {
                        format!("[{}]", value)
                    };
                    let style = if is_current { edit_style } else { normal };
                    view.render_row(
                        frame,
                        row,
                        vec![Span::styled(format!("   {:40}", display_value), style)],
                    );
                    row += 1;
                }
                FieldType::TextArea {
                    rows: textarea_rows,
                } => {
                    let lines: Vec<&str> = value.lines().collect();
                    let style = if is_current { edit_style } else { normal };
                    for r in 0..*textarea_rows {
                        let line_text = lines.get(r as usize).unwrap_or(&"");
                        let display = if is_current && r == 0 {
                            format!("[{}|", line_text)
                        } else if r == 0 {
                            format!("[{}", line_text)
                        } else if r == textarea_rows - 1 {
                            format!(" {}]", line_text)
                        } else {
                            format!(" {}", line_text)
                        };
                        view.render_row(
                            frame,
                            row,
                            vec![Span::styled(format!("   {:40}", display), style)],
                        );
                        row += 1;
                    }
                }
                FieldType::Choice { options, multi } => {
                    let selected: Vec<&str> = if *multi {
                        value.split(',').map(|s| s.trim()).collect()
                    } else {
                        vec![value.as_str()]
                    };

                    for (opt_i, opt) in options.iter().enumerate() {
                        let is_option_selected = selected.contains(&opt.as_str());
                        let is_cursor_here = is_current && opt_i == state.choice_cursor;
                        let marker = if is_option_selected { "[x]" } else { "[ ]" };
                        let style = if is_cursor_here { highlight } else { normal };
                        view.render_row(
                            frame,
                            row,
                            vec![Span::styled(format!("      {} {}", marker, opt), style)],
                        );
                        row += 1;
                    }
                }
                FieldType::Checkbox => {
                    let is_checked = value == "true";
                    let marker = if is_checked { "[x]" } else { "[ ]" };
                    let style = if is_current { highlight } else { normal };
                    view.render_row(
                        frame,
                        row,
                        vec![Span::styled(format!("   {} Yes", marker), style)],
                    );
                    row += 1;
                }
                FieldType::FileRef => {
                    let display = if value.is_empty() {
                        "[Browse...]".to_string()
                    } else {
                        format!("[{}]", value)
                    };
                    let style = if is_current { edit_style } else { normal };
                    view.render_row(
                        frame,
                        row,
                        vec![Span::styled(format!("   {:40}", display), style)],
                    );
                    row += 1;
                }
            }

            // Error message
            if let Some(err) = error {
                view.render_row(
                    frame,
                    row,
                    vec![Span::styled(format!("   ! {}", err), error_style)],
                );
                row += 1;
            }

            row += 1; // Spacing
        }
    }

    // Status bar
    let status_y = view.content_height().saturating_sub(2);
    if let Some(form) = state.current_form() {
        let status = format!("Field {}/{}", state.entry_field + 1, form.fields.len());
        view.render_row(
            frame,
            status_y,
            vec![Span::styled(status, Style::default().fg(colors.grey()))],
        );
    }

    view.render_help(
        frame,
        vec![
            ("Tab", "next"),
            ("S-Tab", "prev"),
            ("Space", "toggle"),
            ("C-Enter", "submit"),
            ("Esc", "cancel"),
        ],
    );
}

// =============================================================================
// RECORDS VIEW
// =============================================================================

pub fn draw_records(state: &QFormState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let title = if let Some(form) = state.current_form() {
        format!(" Q-FORM: {} (Records) ", form.title)
    } else {
        " Q-FORM: Records ".to_string()
    };
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    let normal = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let header_style = Style::default()
        .fg(colors.cyan())
        .add_modifier(Modifier::BOLD);

    let records = state.records_for_current_form();

    if records.is_empty() {
        view.render_row(
            frame,
            1,
            vec![Span::styled(
                "No records yet. Use Entry mode to add data.",
                Style::default().fg(colors.grey()),
            )],
        );
    } else if let Some(form) = state.current_form() {
        // Header row
        let mut header_spans = vec![Span::styled(format!("{:4}", "#"), header_style)];
        for field in form.fields.iter().take(4) {
            header_spans.push(Span::styled(format!(" {:15}", field.label), header_style));
        }
        header_spans.push(Span::styled("  Submitted", header_style));
        view.render_row(frame, 1, header_spans);

        // Separator
        view.render_row(
            frame,
            2,
            vec![Span::styled(
                "-".repeat(area.width.saturating_sub(4) as usize),
                Style::default().fg(colors.grey()),
            )],
        );

        // Records
        let max_rows = view.content_height().saturating_sub(5) as usize;
        let start = state.record_scroll;
        let visible_records: Vec<_> = records.iter().skip(start).take(max_rows).collect();

        for (i, record) in visible_records.iter().enumerate() {
            let actual_index = start + i;
            let is_selected = actual_index == state.record_cursor;
            let style = if is_selected { highlight } else { normal };

            let mut spans = vec![Span::styled(format!("{:4}", actual_index + 1), style)];

            for field in form.fields.iter().take(4) {
                let value = record.data.get(&field.id).cloned().unwrap_or_default();
                let truncated: String = value.chars().take(15).collect();
                spans.push(Span::styled(format!(" {:15}", truncated), style));
            }

            let date_str = record.submitted.format("%Y-%m-%d %H:%M").to_string();
            spans.push(Span::styled(format!("  {}", date_str), style));

            view.render_row(frame, 3 + i as u16, spans);
        }
    }

    // Status bar
    let status_y = view.content_height().saturating_sub(2);
    let status = format!("{} records", records.len());
    view.render_row(
        frame,
        status_y,
        vec![Span::styled(status, Style::default().fg(colors.grey()))],
    );

    view.render_help(
        frame,
        vec![
            ("^v", "navigate"),
            ("X", "export"),
            ("Del", "delete"),
            ("Esc", "back"),
        ],
    );
}

// =============================================================================
// EXPORT VIEW
// =============================================================================

pub fn draw_export(state: &QFormState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-FORM: Export ", colors);
    view.render_frame(frame);

    let normal = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(colors.cyan());

    view.render_row(frame, 1, vec![Span::styled("Export Records", label_style)]);

    view.render_row(frame, 3, vec![Span::styled("Format:", label_style)]);

    let csv_style = if state.export_format == ExportFormat::Csv {
        highlight
    } else {
        normal
    };
    let json_style = if state.export_format == ExportFormat::Json {
        highlight
    } else {
        normal
    };

    view.render_row(
        frame,
        4,
        vec![
            Span::styled(
                if state.export_format == ExportFormat::Csv {
                    "  (o) CSV"
                } else {
                    "  ( ) CSV"
                },
                csv_style,
            ),
            Span::styled("    ", normal),
            Span::styled(
                if state.export_format == ExportFormat::Json {
                    "(o) JSON"
                } else {
                    "( ) JSON"
                },
                json_style,
            ),
        ],
    );

    view.render_row(frame, 6, vec![Span::styled("Output:", label_style)]);
    view.render_row(
        frame,
        7,
        vec![Span::styled(
            format!("  [{}|]", state.export_path),
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::UNDERLINED),
        )],
    );

    let records = state.records_for_current_form();
    view.render_row(
        frame,
        9,
        vec![Span::styled(
            format!("Records to export: {}", records.len()),
            Style::default().fg(colors.grey()),
        )],
    );

    view.render_help(
        frame,
        vec![("Tab", "format"), ("Enter", "export"), ("Esc", "cancel")],
    );
}

// =============================================================================
// HELP VIEW
// =============================================================================

pub fn draw_help(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-FORM: Help ", colors);
    view.render_frame(frame);

    let header_style = Style::default()
        .fg(colors.cyan())
        .add_modifier(Modifier::BOLD);
    let normal = Style::default().fg(colors.fg());
    let key_style = Style::default().fg(colors.yellow());

    let help_text = [
        ("Q-FORM - Form Builder & Data Entry", header_style),
        ("", normal),
        ("Form List:", header_style),
        ("  N        Create new form", normal),
        ("  T        Create from template", normal),
        ("  Enter    Open selected form", normal),
        ("  D        Open in designer", normal),
        ("  E        Open in entry mode", normal),
        ("  R        View records", normal),
        ("  Del      Delete form", normal),
        ("", normal),
        ("Designer:", header_style),
        ("  A        Add field", normal),
        ("  E        Edit field label", normal),
        ("  T        Change field type", normal),
        ("  Del      Remove field", normal),
        ("  Ctrl+Up  Move field up", normal),
        ("  Ctrl+Dn  Move field down", normal),
        ("  Ctrl+S   Save form", normal),
        ("", normal),
        ("Entry:", header_style),
        ("  Tab      Next field", normal),
        ("  S-Tab    Previous field", normal),
        ("  Space    Toggle checkbox/choice", normal),
        ("  Ctrl+Ent Submit record", normal),
    ];

    for (i, (text, style)) in help_text.iter().enumerate() {
        if i as u16 + 1 >= view.content_height() {
            break;
        }
        // Key binding line
        if text.contains("  ") && !text.is_empty() {
            let parts: Vec<&str> = text.splitn(2, "  ").collect();
            if parts.len() == 2 {
                view.render_row(
                    frame,
                    i as u16,
                    vec![
                        Span::styled(format!("  {:8}", parts[0].trim()), key_style),
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

pub fn draw_qform(state: &QFormState, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    match state.view {
        QFormView::FormList => draw_form_list(state, frame, area, colors),
        QFormView::Designer => draw_designer(state, frame, area, colors),
        QFormView::Entry => draw_entry(state, frame, area, colors),
        QFormView::Records => draw_records(state, frame, area, colors),
        QFormView::Export => draw_export(state, frame, area, colors),
        QFormView::Help => draw_help(frame, area, colors),
    }
}
