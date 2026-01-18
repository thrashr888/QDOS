//! Attribute modal drawing function

use crate::app::App;
use crate::app::{AttrValue, AttributeState};
use crate::ui::components::ModalFrame;
use ratatui::{layout::Rect, style::Style, text::Span, Frame};

/// Draw the Attribute modal
pub fn draw_attribute_modal(frame: &mut Frame, area: Rect, state: &AttributeState, app: &App) {
    let colors = app.colors();
    let title = if state.display_only {
        " Display File Attributes "
    } else if state.for_tagged {
        " Change Tagged Files Attributes "
    } else {
        " Change File Attributes "
    };

    let modal = ModalFrame::themed(area, title, &colors).no_footer_separator();
    modal.render_frame(frame);

    let label_style = Style::default().fg(colors.green()).bg(colors.bg());
    let value_style = Style::default().fg(colors.fg()).bg(colors.bg());
    let grey_style = Style::default().fg(colors.grey()).bg(colors.bg());

    // File name
    modal.render_row(
        frame,
        0,
        vec![
            Span::styled("File: ", label_style),
            Span::styled(&state.name, value_style),
        ],
    );
    modal.render_row(frame, 1, vec![]);
    modal.render_row(
        frame,
        2,
        vec![Span::styled("Current attributes:", label_style)],
    );
    modal.render_row(frame, 3, vec![]);

    // Show original values
    let orig_text = format!(
        "  Original: {} {} {} {}",
        if state.original[0] { "HID" } else { "   " },
        if state.original[1] { "SYS" } else { "   " },
        if state.original[2] { "R/O" } else { "   " },
        if state.original[3] { "ARC" } else { "   " },
    );
    modal.render_row(frame, 4, vec![Span::styled(orig_text, grey_style)]);
    modal.render_row(frame, 5, vec![]);

    // Build attribute bars
    let mut attr_spans: Vec<Span> = Vec::new();
    for i in 0..4 {
        let name = AttributeState::attr_name(i);
        let value = state.attrs[i];

        // Determine if this attribute is modifiable
        // Only R/O (index 2) is modifiable on Unix
        let is_modifiable = i == 2 && !state.display_only;
        let is_selected = i == state.selected && !state.display_only;

        let style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else if is_modifiable {
            Style::default().fg(colors.fg()).bg(colors.bg())
        } else {
            Style::default().fg(colors.grey()).bg(colors.bg())
        };

        let value_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            match value {
                AttrValue::On => Style::default().fg(colors.green()).bg(colors.bg()),
                AttrValue::Off => Style::default().fg(colors.grey()).bg(colors.bg()),
                AttrValue::NoChange => Style::default().fg(colors.blue()).bg(colors.bg()),
            }
        };

        attr_spans.push(Span::styled(format!("[ {} ", name), style));
        attr_spans.push(Span::styled(value.as_str(), value_style));
        attr_spans.push(Span::styled(" ]  ", style));
    }
    modal.render_row(frame, 6, attr_spans);

    // Help text
    if state.display_only {
        modal.render_help(frame, vec![("Any key", "close")]);
    } else {
        modal.render_row(
            frame,
            8,
            vec![Span::styled(
                "Note: Only R/O (Read-Only) can be changed on Unix",
                grey_style,
            )],
        );
        modal.render_help(
            frame,
            vec![
                ("←→", "select"),
                ("SPACE", "toggle"),
                ("Enter", "apply"),
                ("ESC", "cancel"),
            ],
        );
    }
}
