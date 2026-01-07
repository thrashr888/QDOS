//! Network Drives modal rendering
//!
//! UI for the Drives modal using ModalFrame.

use super::state::{DrivesSection, DrivesState};
use crate::app::ThemeColors;
use crate::ui::components::ModalFrame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;

/// Draw the Drives modal
pub fn draw_drives_modal(frame: &mut Frame, area: Rect, state: &DrivesState, colors: &ThemeColors) {
    // Calculate centered modal area
    let popup_width = 70u16.min(area.width.saturating_sub(4));
    let popup_height = 22u16.min(area.height.saturating_sub(4));
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
    let has_network_shares = !state.network_shares.is_empty();

    // Calculate space for each section
    let volumes_height = if has_network_shares {
        (visible_height / 2).max(4)
    } else {
        visible_height.saturating_sub(1)
    };

    // Section tabs
    let vol_tab_style = if state.section == DrivesSection::Volumes {
        Style::default()
            .fg(yellow)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(grey)
    };
    let net_tab_style = if state.section == DrivesSection::NetworkShares {
        Style::default()
            .fg(yellow)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(grey)
    };

    let mut tabs = vec![Span::styled(" [Volumes]", vol_tab_style)];
    if has_network_shares {
        tabs.push(Span::styled("  ", Style::default().fg(grey)));
        tabs.push(Span::styled(
            format!("[Network ({})]", state.network_shares.len()),
            net_tab_style,
        ));
    }
    modal.render_row(frame, 0, tabs);

    // Volumes section header
    modal.render_row(
        frame,
        1,
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
        let max_visible = volumes_height.saturating_sub(2);
        for (i, vol) in state.volumes.iter().take(max_visible).enumerate() {
            let is_selected =
                state.section == DrivesSection::Volumes && i == state.selected_index;
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

            let row = 2 + i as u16;
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

    // Network shares section (if any)
    if has_network_shares {
        let net_start_row = (volumes_height + 1) as u16;

        modal.render_row(
            frame,
            net_start_row,
            vec![Span::styled(
                " Proto  Available Network Shares",
                Style::default().fg(grey).bg(bg),
            )],
        );

        let net_max_visible = visible_height.saturating_sub(volumes_height + 2);
        for (i, share) in state.network_shares.iter().take(net_max_visible).enumerate() {
            let is_selected =
                state.section == DrivesSection::NetworkShares && i == state.selected_index;
            let prefix = if is_selected { ">" } else { " " };

            let style = if is_selected {
                Style::default().fg(yellow).bg(red)
            } else {
                Style::default().fg(fg).bg(bg)
            };

            let proto_style = if is_selected {
                Style::default().fg(yellow).bg(red)
            } else {
                Style::default().fg(cyan).bg(bg)
            };

            let row = net_start_row + 1 + i as u16;
            modal.render_row(
                frame,
                row,
                vec![
                    Span::styled(format!("{} ", prefix), style),
                    Span::styled(format!("{:<5} ", share.protocol.as_str()), proto_style),
                    Span::styled(&share.name, style),
                ],
            );
        }
    }

    // Help footer
    let mut help = vec![
        ("Enter", if state.section == DrivesSection::Volumes { "go to" } else { "mount" }),
        ("↑↓", "select"),
    ];
    if has_network_shares {
        help.push(("Tab", "switch"));
    }
    help.extend([("R", "refresh"), ("Esc", "close")]);

    modal.render_help(frame, help);
}
