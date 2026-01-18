//! QDCONFIG modal rendering
//!
//! Rendering functions for the configuration modal.

use super::state::{QdconfigField, QdconfigState};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Display item for rendering
struct DisplayItem {
    name: String,
    value: String,
    is_header: bool,
    is_info: bool,
    /// Selectable index (None for non-selectable items like headers/spacers)
    selectable_idx: Option<usize>,
}

/// Draw the configuration modal
pub fn draw_config_modal(
    frame: &mut Frame,
    area: Rect,
    state: &QdconfigState,
    colors: &ThemeColors,
) {
    // Create full-screen view
    let view = FullScreenView::new(area, " R-DOS STARTUP CONFIGURATION ", colors);
    view.render_frame(frame);

    // Content area
    let content_area = view.content_area();
    let visible_height = content_area.height.saturating_sub(3) as usize;

    // Build all items (config fields + plugins)
    let mut items: Vec<DisplayItem> = vec![];

    // Add config fields
    for (i, field) in QdconfigField::ALL.iter().enumerate() {
        let is_selected = i == state.selected;
        let is_editing = is_selected && state.editing;

        let name = field.name().to_string();
        let value = match field {
            QdconfigField::SearchSpec => {
                if is_editing {
                    format!("{}█", state.input_buffer)
                } else {
                    state.search_spec.clone()
                }
            }
            QdconfigField::SortMethod => state.sort_method_name().to_string(),
            QdconfigField::SortDirection => {
                if state.sort_asc {
                    "Ascending".to_string()
                } else {
                    "Descending".to_string()
                }
            }
            QdconfigField::ShowHidden => if state.show_hidden { "Yes" } else { "No" }.to_string(),
            QdconfigField::ConfirmDelete => {
                if state.confirm_delete { "Yes" } else { "No" }.to_string()
            }
            QdconfigField::Editor => {
                if is_editing {
                    format!("{}█", state.input_buffer)
                } else {
                    state
                        .editor
                        .clone()
                        .unwrap_or_else(|| "$EDITOR".to_string())
                }
            }
            QdconfigField::ColorTheme => state.theme().name().to_string(),
            QdconfigField::MouseSupport => {
                if state.mouse_support { "Yes" } else { "No" }.to_string()
            }
            QdconfigField::UppercaseNames => {
                if state.uppercase_names { "Yes" } else { "No" }.to_string()
            }
            QdconfigField::AutoRefresh => {
                if state.auto_refresh_interval == 0 {
                    "Off".to_string()
                } else {
                    format!("{} sec", state.auto_refresh_interval)
                }
            }
        };

        items.push(DisplayItem {
            name,
            value,
            is_header: false,
            is_info: false,
            selectable_idx: Some(i),
        });
    }

    // Add plugins header and items (with blank line before)
    if !state.plugins.is_empty() {
        items.push(DisplayItem {
            name: String::new(),
            value: String::new(),
            is_header: false,
            is_info: true,
            selectable_idx: None, // Spacer - not selectable
        });
        items.push(DisplayItem {
            name: "Registered Plugins:".to_string(),
            value: String::new(),
            is_header: true,
            is_info: false,
            selectable_idx: None, // Header - not selectable
        });

        for (i, (id, name, description)) in state.plugins.iter().enumerate() {
            // Plugin selectable index = config fields count + plugin index
            let selectable_idx = QdconfigField::ALL.len() + i;
            let is_selected = selectable_idx == state.selected;
            items.push(DisplayItem {
                name: format!(
                    "{} {} ({}) - {}",
                    if is_selected { "▶" } else { " " },
                    id,
                    name,
                    description
                ),
                value: String::new(),
                is_header: false,
                is_info: true,
                selectable_idx: Some(selectable_idx),
            });
        }

        // Blank line after plugins
        items.push(DisplayItem {
            name: String::new(),
            value: String::new(),
            is_header: false,
            is_info: true,
            selectable_idx: None, // Spacer - not selectable
        });
    }

    // Add info line
    items.push(DisplayItem {
        name: "Settings saved to: ~/Library/Application Support/rdos/".to_string(),
        value: String::new(),
        is_header: false,
        is_info: true,
        selectable_idx: None, // Info - not selectable
    });

    // Find the display index of the currently selected item
    let selected_display_idx = items
        .iter()
        .position(|item| item.selectable_idx == Some(state.selected))
        .unwrap_or(0);

    // Calculate scroll to keep selection visible (center selection in view when possible)
    let scroll = if selected_display_idx >= visible_height {
        selected_display_idx.saturating_sub(visible_height / 2)
    } else {
        0
    }
    .min(items.len().saturating_sub(visible_height));

    // Build visible lines
    let mut lines: Vec<Line> = vec![Line::from("")];

    for item in items.iter().skip(scroll).take(visible_height) {
        // Check if this item is selected using its selectable index
        let is_selected = item.selectable_idx == Some(state.selected);

        if item.is_header {
            lines.push(Line::from(Span::styled(
                &item.name,
                Style::default()
                    .fg(colors.blue())
                    .add_modifier(Modifier::BOLD),
            )));
        } else if item.is_info {
            // Plugin item or info/spacer line
            let style = if is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(colors.grey())
            };
            lines.push(Line::from(Span::styled(&item.name, style)));
        } else {
            // Config field
            let is_editing = is_selected && state.editing;

            let line_style = if is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(colors.fg()).bg(colors.bg())
            };

            let name_style = if is_selected {
                Style::default()
                    .fg(colors.yellow())
                    .bg(colors.red())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.blue())
            };

            let value_style = if is_editing || is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(colors.green())
            };

            let padded_name = format!("  {:<22}", format!("{}:", item.name));
            let padded_value = format!("{:<20}", item.value);

            lines.push(Line::from(vec![
                Span::styled(padded_name, name_style),
                Span::styled(padded_value, value_style),
                Span::styled(
                    " ".repeat(area.width.saturating_sub(44) as usize),
                    line_style,
                ),
            ]));
        }
    }

    // Show scroll indicator if content extends beyond view
    let total_items = items.len();
    if total_items > visible_height {
        let indicator = format!(
            " [{}/{}] ",
            scroll + 1,
            total_items.saturating_sub(visible_height) + 1
        );
        let indicator_len = indicator.len() as u16;
        let indicator_x = content_area.x + content_area.width.saturating_sub(indicator_len + 1);
        frame.render_widget(
            Paragraph::new(Span::styled(indicator, Style::default().fg(colors.grey()))),
            Rect::new(indicator_x, content_area.y, indicator_len + 1, 1),
        );
    }

    frame.render_widget(Paragraph::new(lines), content_area);

    // Help line
    if state.editing {
        view.render_footer(
            frame,
            vec![Span::styled(
                " Type value, Enter to confirm, ESC to cancel",
                Style::default().fg(colors.green()),
            )],
        );
    } else {
        view.render_help(
            frame,
            vec![
                ("↑↓", "select"),
                ("Enter/Space", "toggle"),
                ("S", "save"),
                ("R", "reload"),
                ("ESC", "close"),
            ],
        );
    }
}
