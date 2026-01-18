//! Trek game modal rendering
//!
//! This module contains the rendering logic for the Star Trek-style space
//! exploration game. It renders the sector grid, long range scan, status bar,
//! and command interface using the FullScreenView component.

use super::super::trek::{self, TrekState};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
    Frame,
};

/// Renders the Trek game modal with sector map, long range scan, and status.
///
/// The display consists of:
/// - Status bar (top): Energy, Shields, Torpedoes, Stardate, Klingons remaining, Docked status
/// - Sector grid (left): 8x8 grid showing Enterprise, Klingons, Starbases, Stars
/// - Long range scan (right): 8x8 galaxy overview with sensor codes
/// - Position info: Current quadrant and sector coordinates
/// - Message/prompt area: Command input and mode indicator
/// - Help footer: Available commands
pub fn draw_trek(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &TrekState,
    colors: &ThemeColors,
) {
    // Status bar
    view.render_row(
        frame,
        0,
        vec![
            Span::styled(
                format!("E:{}", state.energy),
                Style::default().fg(if state.energy < 500 {
                    colors.red()
                } else {
                    colors.green()
                }),
            ),
            Span::styled(
                format!(" S:{}", state.shields),
                Style::default().fg(if state.shields > 0 {
                    colors.cyan()
                } else {
                    colors.grey()
                }),
            ),
            Span::styled(
                format!(" T:{}", state.torpedoes),
                Style::default().fg(if state.torpedoes > 0 {
                    colors.yellow()
                } else {
                    colors.grey()
                }),
            ),
            Span::styled(
                format!(" SD:{:.1}", state.stardate),
                Style::default().fg(colors.fg()),
            ),
            Span::styled(
                format!(" K:{}", state.klingons_remaining),
                Style::default().fg(colors.red()),
            ),
            Span::styled(
                if state.docked { " [DOCKED]" } else { "" }.to_string(),
                Style::default().fg(colors.green()),
            ),
        ],
    );

    // Draw sector grid on the left
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            format!("╔{}╗  Long Range Scan", "═══".repeat(trek::SECTOR_SIZE)),
            Style::default().fg(colors.cyan()),
        )],
    );

    for y in 0..trek::SECTOR_SIZE {
        let mut row_spans: Vec<Span> = vec![Span::styled("║", Style::default().fg(colors.cyan()))];

        for x in 0..trek::SECTOR_SIZE {
            let entity = state.sector[y][x];
            let entity_str = entity.char();
            let entity_color = match entity {
                trek::SectorEntity::Enterprise => colors.green(),
                trek::SectorEntity::Klingon => colors.red(),
                trek::SectorEntity::Starbase => colors.cyan(),
                trek::SectorEntity::Star => colors.yellow(),
                trek::SectorEntity::Empty => colors.grey(),
            };
            row_spans.push(Span::styled(entity_str, Style::default().fg(entity_color)));
        }

        row_spans.push(Span::styled("║", Style::default().fg(colors.cyan())));

        // Long range scan on the right side
        row_spans.push(Span::raw("  "));
        if y < trek::GALAXY_SIZE {
            for x in 0..trek::GALAXY_SIZE {
                let q = &state.galaxy[y][x];
                let is_current = x == state.quadrant_x && y == state.quadrant_y;
                let code = if q.scanned {
                    q.sensor_code()
                } else {
                    "***".to_string()
                };
                let style = if is_current {
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD)
                } else if q.klingons > 0 {
                    Style::default().fg(colors.red())
                } else if q.starbases > 0 {
                    Style::default().fg(colors.cyan())
                } else {
                    Style::default().fg(colors.grey())
                };
                row_spans.push(Span::styled(format!("{} ", code), style));
            }
        }

        view.render_row(frame, 2 + y as u16, row_spans);
    }

    // Bottom border of sector
    view.render_row(
        frame,
        2 + trek::SECTOR_SIZE as u16,
        vec![Span::styled(
            format!("╚{}╝", "═══".repeat(trek::SECTOR_SIZE)),
            Style::default().fg(colors.cyan()),
        )],
    );

    // Position info
    view.render_row(
        frame,
        3 + trek::SECTOR_SIZE as u16,
        vec![Span::styled(
            format!(
                "Quadrant {}-{}, Sector {}-{}",
                state.quadrant_x + 1,
                state.quadrant_y + 1,
                state.sector_x + 1,
                state.sector_y + 1
            ),
            Style::default().fg(colors.grey()),
        )],
    );

    // Message/prompt area
    let prompt_row = 4 + trek::SECTOR_SIZE as u16;
    let prompt_style = Style::default().fg(colors.yellow());
    let input_display = format!("{}{}", state.message, state.input_buffer);
    view.render_row(
        frame,
        prompt_row,
        vec![Span::styled(input_display, prompt_style)],
    );

    // Command mode indicator
    let mode_str = match state.mode {
        trek::CommandMode::Main => "",
        trek::CommandMode::Navigation => "[NAV]",
        trek::CommandMode::Phasers => "[PHASER]",
        trek::CommandMode::Torpedoes => "[TORPEDO]",
        trek::CommandMode::Shields => "[SHIELDS]",
        trek::CommandMode::Computer => "[COMPUTER]",
    };
    if !mode_str.is_empty() {
        view.render_row(
            frame,
            prompt_row + 1,
            vec![Span::styled(
                mode_str,
                Style::default()
                    .fg(colors.green())
                    .add_modifier(Modifier::BOLD),
            )],
        );
    }

    let help = vec![
        ("N", "nav"),
        ("S", "scan"),
        ("L", "LRS"),
        ("P", "phasers"),
        ("T", "torpedoes"),
        ("H", "shields"),
        ("C", "computer"),
        ("D", "damage"),
        ("Esc", "quit"),
    ];
    view.render_help(frame, help);
}
