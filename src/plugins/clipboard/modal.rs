//! Clipboard modal drawing function

use crate::app::{App, ClipboardState};
use crate::ui::components::ModalFrame;
use ratatui::{layout::Rect, style::Style, text::Span, Frame};

/// Draw clipboard selection modal
pub fn draw_clipboard_modal(frame: &mut Frame, area: Rect, state: &ClipboardState, app: &App) {
    let colors = app.colors();
    let label_style = Style::default().fg(colors.green()).bg(colors.bg());
    let value_style = Style::default().fg(colors.fg()).bg(colors.bg());
    let selected_style = Style::default()
        .fg(colors.yellow())
        .bg(colors.bg())
        .add_modifier(ratatui::style::Modifier::BOLD);

    // Use ModalFrame for consistent double-line border styling
    let modal = ModalFrame::themed(area, " Copy to Clipboard (Y) ", &colors)
        .no_title_separator()
        .no_footer_separator();
    modal.render_frame(frame);

    // Empty row at top
    modal.render_row(frame, 0, vec![]);

    // Render clipboard items
    for (i, item) in state.items.iter().enumerate() {
        let num = format!("[{}] ", i + 1);
        let is_selected = i == state.selected;

        let row_spans = if is_selected {
            vec![
                Span::styled(num, selected_style),
                Span::styled(&item.label, selected_style),
                Span::styled(": ", selected_style),
                Span::styled(&item.value, selected_style),
            ]
        } else {
            vec![
                Span::styled(num, label_style),
                Span::styled(&item.label, label_style),
                Span::styled(": ", value_style),
                Span::styled(&item.value, value_style),
            ]
        };

        modal.render_row(frame, (i + 1) as u16, row_spans);
    }

    // Help line
    modal.render_help(frame, vec![("1-9/Enter", "copy"), ("Esc", "cancel")]);
}
