//! Network Drives modal rendering
//!
//! UI for the Drives modal using ModalFrame.

use super::state::DrivesState;
use crate::app::ThemeColors;
use crate::ui::components::ModalFrame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::Frame;

/// Draw the Drives modal
pub fn draw_drives_modal(frame: &mut Frame, area: Rect, state: &DrivesState, colors: &ThemeColors) {
    // Calculate centered modal area
    let popup_width = 70u16.min(area.width.saturating_sub(4));
    let popup_height = 20u16.min(area.height.saturating_sub(4));
    let popup_x = (area.width - popup_width) / 2;
    let popup_y = (area.height - popup_height) / 2;
    let modal_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let modal = ModalFrame::themed(modal_area, " Drives & Volumes ", colors);
    modal.render_frame(frame);

    let bg = colors.bg();
    let fg = colors.fg();
    let grey = colors.grey();
    let green = colors.green();
    let yellow = colors.yellow();
    let red = colors.red();
    let cyan = colors.cyan();
    let blue = colors.blue();

    let visible_height = modal.content_height() as usize;

    // Header row
    modal.render_row(
        frame,
        0,
        vec![Span::styled(
            " Type  Name                   Size      Free   Mount Point",
            Style::default().fg(grey).bg(bg),
        )],
    );

    // Volume list
    if state.volumes.is_empty() {
        modal.render_row(
            frame,
            2,
            vec![Span::styled(
                " No volumes found",
                Style::default().fg(grey).bg(bg),
            )],
        );
    } else {
        let max_visible = visible_height.saturating_sub(2);
        for (i, vol) in state.volumes.iter().take(max_visible).enumerate() {
            let is_selected = i == state.selected_index;
            let prefix = if is_selected { ">" } else { " " };

            let style = if is_selected {
                Style::default().fg(yellow).bg(red)
            } else {
                Style::default().fg(fg).bg(bg)
            };

            let type_style = if is_selected {
                Style::default().fg(yellow).bg(red)
            } else {
                match vol.volume_type {
                    super::state::VolumeType::Network => Style::default().fg(cyan).bg(bg),
                    super::state::VolumeType::Local => Style::default().fg(green).bg(bg),
                    super::state::VolumeType::DiskImage => Style::default().fg(blue).bg(bg),
                    _ => Style::default().fg(grey).bg(bg),
                }
            };

            // Format: > TYPE  Name                   Size      Free   Mount
            let name_truncated = if vol.name.len() > 20 {
                format!("{}...", &vol.name[..17])
            } else {
                format!("{:<20}", vol.name)
            };

            let mount_truncated = if vol.mount_point.len() > 15 {
                format!("...{}", &vol.mount_point[vol.mount_point.len() - 12..])
            } else {
                vol.mount_point.clone()
            };

            let row = 1 + i as u16;
            modal.render_row(
                frame,
                row,
                vec![
                    Span::styled(format!("{} ", prefix), style),
                    Span::styled(format!("{:<4} ", vol.volume_type.icon()), type_style),
                    Span::styled(format!("{} ", name_truncated), style),
                    Span::styled(format!("{:>8} ", vol.formatted_size()), style),
                    Span::styled(format!("{:>8} ", vol.formatted_free()), style),
                    Span::styled(mount_truncated, style),
                ],
            );
        }
    }

    // Help footer
    modal.render_help(
        frame,
        vec![
            ("Enter", "go to"),
            ("↑↓", "select"),
            ("R", "refresh"),
            ("Esc", "close"),
        ],
    );
}
