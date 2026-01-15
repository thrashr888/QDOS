//! MICROPOLIS modal rendering
//!
//! Renders the city builder game with horizontal scrolling viewport.

use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::{
    prelude::*,
    style::{Color, Style},
    text::Span,
    Frame,
};

use super::super::micropolis::{MicropolisState, MicropolisView, Owner, PropertyType};
use super::super::platform::engine::GameEngine;

/// Main draw function for MICROPOLIS
pub fn draw(frame: &mut Frame, area: Rect, state: &MicropolisState, colors: &ThemeColors) {
    match state.view {
        MicropolisView::Menu => draw_menu(frame, area, colors),
        MicropolisView::City => draw_city(frame, area, state, colors),
        MicropolisView::Buy => draw_buy_dialog(frame, area, state, colors),
        MicropolisView::Status => draw_status(frame, area, state, colors),
        MicropolisView::Disaster => draw_disaster(frame, area, state, colors),
        MicropolisView::GameOver => draw_game_over(frame, area, state, colors),
    }
}

fn draw_menu(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " MICROPOLIS ", colors);
    view.render_frame(frame);

    let title_style = Style::default().fg(colors.yellow()).bold();
    let text_style = Style::default().fg(colors.fg());
    let highlight = Style::default().fg(colors.cyan());

    // ASCII art title
    let title_lines = [
        "███╗   ███╗██╗ ██████╗██████╗  ██████╗ ██████╗  ██████╗ ██╗     ██╗███████╗",
        "████╗ ████║██║██╔════╝██╔══██╗██╔═══██╗██╔══██╗██╔═══██╗██║     ██║██╔════╝",
        "██╔████╔██║██║██║     ██████╔╝██║   ██║██████╔╝██║   ██║██║     ██║███████╗",
        "██║╚██╔╝██║██║██║     ██╔══██╗██║   ██║██╔═══╝ ██║   ██║██║     ██║╚════██║",
        "██║ ╚═╝ ██║██║╚██████╗██║  ██║╚██████╔╝██║     ╚██████╔╝███████╗██║███████║",
        "╚═╝     ╚═╝╚═╝ ╚═════╝╚═╝  ╚═╝ ╚═════╝ ╚═╝      ╚═════╝ ╚══════╝╚═╝╚══════╝",
    ];

    for (i, line) in title_lines.iter().enumerate() {
        view.render_row(frame, i as u16 + 1, vec![Span::styled(*line, title_style)]);
    }

    view.render_row(
        frame,
        8,
        vec![Span::styled("ASCII City Builder", highlight)],
    );
    view.render_row(
        frame,
        10,
        vec![Span::styled("Build your real estate empire!", text_style)],
    );

    // Instructions
    view.render_row(frame, 12, vec![Span::styled("HOW TO PLAY:", title_style)]);
    view.render_row(
        frame,
        13,
        vec![Span::styled(
            "  Buy properties and collect rent each day",
            text_style,
        )],
    );
    view.render_row(
        frame,
        14,
        vec![Span::styled(
            "  Watch out for fires - parks block them!",
            text_style,
        )],
    );
    view.render_row(
        frame,
        15,
        vec![Span::styled("  Go bankrupt and it's game over", text_style)],
    );

    view.render_row(
        frame,
        17,
        vec![Span::styled("Press ENTER or SPACE to start", highlight)],
    );

    view.render_help(frame, vec![("Enter", "start"), ("Esc", "quit")]);
}

fn draw_city(frame: &mut Frame, area: Rect, state: &MicropolisState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " MICROPOLIS ", colors);
    view.render_frame(frame);

    let content_width: usize = 78;

    // Header with stats
    let header = format!(
        "{}: {}          Day {}    Cash: ${}    Net: ${}",
        "MICROPOLIS",
        state.city_name,
        state.day,
        format_money(state.cash),
        format_money(state.net_worth())
    );
    view.render_row(
        frame,
        0,
        vec![Span::styled(
            truncate_or_pad(&header, content_width),
            Style::default().fg(colors.yellow()),
        )],
    );

    // Sky with clouds (row 2-3)
    let sky = draw_sky(state, content_width);
    view.render_row(frame, 2, sky);

    // Buildings (rows 4-8)
    let (buildings_top, buildings_mid, buildings_bot) =
        draw_buildings(state, content_width, colors);
    view.render_row(frame, 4, buildings_top);
    view.render_row(frame, 5, buildings_mid);
    view.render_row(frame, 6, buildings_bot);

    // Ground/road (row 7)
    let ground: String = "─".repeat(content_width);
    view.render_row(
        frame,
        7,
        vec![Span::styled(ground, Style::default().fg(colors.grey()))],
    );

    // Street names (row 8-9)
    let streets = draw_streets(state, content_width, colors);
    view.render_row(frame, 8, streets);

    // Cursor indicator (row 10)
    let cursor = draw_cursor(state, content_width, colors);
    view.render_row(frame, 10, cursor);

    // Property info (rows 12-14)
    if let Some(prop) = state.selected_property() {
        let owner_str = match prop.owner {
            Owner::None => "Available",
            Owner::Player => "YOURS",
            Owner::Npc => "For Sale",
        };
        let fire_str = if prop.on_fire { " [ON FIRE!]" } else { "" };

        view.render_row(
            frame,
            12,
            vec![Span::styled(
                format!(
                    "Selected: {} ({}) - Condition: {}%{}",
                    prop.property_type.name(),
                    owner_str,
                    prop.condition,
                    fire_str
                ),
                Style::default().fg(if prop.on_fire {
                    colors.red()
                } else {
                    colors.fg()
                }),
            )],
        );

        if prop.owner == Owner::Player {
            view.render_row(
                frame,
                13,
                vec![Span::styled(
                    format!(
                        "  Rent: ${}/day   Maintenance: ${}/day   Value: ${}",
                        prop.rent(),
                        prop.property_type.daily_maintenance(),
                        prop.value()
                    ),
                    Style::default().fg(colors.green()),
                )],
            );
        } else if prop.owner != Owner::Player {
            let cost = prop.buy_cost();
            view.render_row(
                frame,
                13,
                vec![Span::styled(
                    format!("  Cost to buy: ${}", cost),
                    Style::default().fg(colors.cyan()),
                )],
            );
        }
    }

    // Message (row 15)
    if let Some(msg) = &state.message {
        view.render_row(
            frame,
            15,
            vec![Span::styled(
                msg.clone(),
                Style::default().fg(colors.yellow()),
            )],
        );
    }

    // Status bar (row 17)
    let status = format!(
        "Properties: {}    Income: ${}/day    Expenses: ${}/day    Fire Risk: {}",
        state.player_property_count(),
        state.daily_income(),
        state.daily_expenses(),
        state.fire_risk()
    );
    view.render_row(
        frame,
        17,
        vec![Span::styled(status, Style::default().fg(colors.fg()))],
    );

    view.render_help(
        frame,
        vec![
            ("</>", "move"),
            ("B", "buy"),
            ("N", "next day"),
            ("R", "repair"),
            ("S", "status"),
            ("Esc", "quit"),
        ],
    );
}

fn draw_sky(state: &MicropolisState, width: usize) -> Vec<Span<'static>> {
    let mut sky = String::new();
    let tick = state.tick_count as usize;

    for i in 0..width {
        let char = match (i + tick / 10) % 20 {
            0 | 1 => '=',
            10 => '*',
            _ => ' ',
        };
        sky.push(char);
    }

    vec![Span::styled(sky, Style::default().fg(Color::White))]
}

fn draw_buildings(
    state: &MicropolisState,
    width: usize,
    colors: &ThemeColors,
) -> (Vec<Span<'static>>, Vec<Span<'static>>, Vec<Span<'static>>) {
    let mut top = String::new();
    let mut mid = String::new();
    let mut bot = String::new();

    let visible_start = state.camera_x;
    let visible_end = (state.camera_x + width / 2).min(state.properties.len());

    // Pad to center in viewport
    let padding = (width - (visible_end - visible_start) * 2) / 2;
    for _ in 0..padding {
        top.push(' ');
        mid.push(' ');
        bot.push(' ');
    }

    for i in visible_start..visible_end {
        let prop = &state.properties[i];
        let is_selected = i == state.cursor_x;

        let (t, m, b) = building_sprite(prop, is_selected, state.tick_count, colors);
        top.push_str(&t);
        mid.push_str(&m);
        bot.push_str(&b);
    }

    // Collect into spans with default style
    (
        vec![Span::styled(top, Style::default().fg(colors.fg()))],
        vec![Span::styled(mid, Style::default().fg(colors.fg()))],
        vec![Span::styled(bot, Style::default().fg(colors.fg()))],
    )
}

fn building_sprite(
    prop: &super::super::micropolis::Property,
    selected: bool,
    tick: u32,
    _colors: &ThemeColors,
) -> (String, String, String) {
    let fire_frame = tick % 4 < 2;

    if prop.on_fire {
        let fire = if fire_frame { "*" } else { "^" };
        return (
            format!("{}{}", fire, fire),
            format!("{}{}", prop.property_type.symbol(), fire),
            "──".to_string(),
        );
    }

    let sym = prop.property_type.symbol();
    let highlight = if selected { '>' } else { ' ' };

    match prop.property_type {
        PropertyType::Empty => (
            format!("{} ", highlight),
            format!("{}.", highlight),
            "──".to_string(),
        ),
        PropertyType::House => (
            format!("{}/\\", highlight),
            format!("{}[]", highlight),
            "──".to_string(),
        ),
        PropertyType::Shop => (
            format!("{}┌┐", highlight),
            format!("{}$$", highlight),
            "──".to_string(),
        ),
        PropertyType::Factory => (
            format!("{}##", highlight),
            format!("{}{}{}", highlight, sym, sym),
            "──".to_string(),
        ),
        PropertyType::Park => (
            format!("{}##", highlight),
            format!("{}##", highlight),
            "──".to_string(),
        ),
    }
}

fn draw_streets(state: &MicropolisState, width: usize, colors: &ThemeColors) -> Vec<Span<'static>> {
    let mut streets = String::new();

    let visible_start = state.camera_x;
    let visible_end = (state.camera_x + width / 2).min(state.properties.len());

    let padding = (width - (visible_end - visible_start) * 2) / 2;
    for _ in 0..padding {
        streets.push(' ');
    }

    for i in visible_start..visible_end {
        if i % 5 == 0 {
            let name = state.street_name(i);
            let short_name: String = name.chars().take(2).collect();
            streets.push_str(&short_name);
        } else {
            streets.push_str("  ");
        }
    }

    vec![Span::styled(streets, Style::default().fg(colors.grey()))]
}

fn draw_cursor(state: &MicropolisState, width: usize, colors: &ThemeColors) -> Vec<Span<'static>> {
    let mut cursor_line = String::new();

    let visible_start = state.camera_x;
    let visible_end = (state.camera_x + width / 2).min(state.properties.len());

    let padding = (width - (visible_end - visible_start) * 2) / 2;
    for _ in 0..padding {
        cursor_line.push(' ');
    }

    for i in visible_start..visible_end {
        if i == state.cursor_x {
            cursor_line.push_str("▲▲");
        } else {
            cursor_line.push_str("  ");
        }
    }

    vec![Span::styled(
        cursor_line,
        Style::default().fg(colors.yellow()),
    )]
}

fn draw_buy_dialog(frame: &mut Frame, area: Rect, state: &MicropolisState, colors: &ThemeColors) {
    // First draw city behind
    draw_city(frame, area, state, colors);

    // Then draw buy dialog overlay
    let view = FullScreenView::new(area, " BUILD ", colors);

    let title_style = Style::default().fg(colors.yellow()).bold();
    let text_style = Style::default().fg(colors.fg());
    let selected_style = Style::default().fg(colors.cyan()).bold();

    view.render_row(
        frame,
        8,
        vec![Span::styled(
            "╔════════════════════════════════╗",
            title_style,
        )],
    );
    view.render_row(
        frame,
        9,
        vec![Span::styled(
            "║       SELECT BUILDING TYPE     ║",
            title_style,
        )],
    );
    view.render_row(
        frame,
        10,
        vec![Span::styled(
            "╠════════════════════════════════╣",
            title_style,
        )],
    );

    let buildings = [
        ("1. House", PropertyType::House),
        ("2. Shop", PropertyType::Shop),
        ("3. Factory", PropertyType::Factory),
        ("4. Park", PropertyType::Park),
    ];

    for (idx, (name, prop_type)) in buildings.iter().enumerate() {
        let style = if idx == state.buy_selection {
            selected_style
        } else {
            text_style
        };
        let marker = if idx == state.buy_selection { ">" } else { " " };
        let line = format!("║ {} {:20} ${:>6} ║", marker, name, prop_type.base_price());
        view.render_row(frame, 11 + idx as u16, vec![Span::styled(line, style)]);
    }

    view.render_row(
        frame,
        15,
        vec![Span::styled(
            "╠════════════════════════════════╣",
            title_style,
        )],
    );
    view.render_row(
        frame,
        16,
        vec![Span::styled(
            format!("║  Your cash: ${:>18} ║", format_money(state.cash)),
            text_style,
        )],
    );
    view.render_row(
        frame,
        17,
        vec![Span::styled(
            "╚════════════════════════════════╝",
            title_style,
        )],
    );

    view.render_help(
        frame,
        vec![("1-4", "select"), ("Enter", "build"), ("Esc", "cancel")],
    );
}

fn draw_status(frame: &mut Frame, area: Rect, state: &MicropolisState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " FINANCIAL STATUS ", colors);
    view.render_frame(frame);

    let title_style = Style::default().fg(colors.yellow()).bold();
    let text_style = Style::default().fg(colors.fg());
    let green = Style::default().fg(colors.green());
    let red = Style::default().fg(colors.red());

    view.render_row(
        frame,
        1,
        vec![Span::styled("FINANCIAL SUMMARY", title_style)],
    );
    view.render_row(
        frame,
        2,
        vec![Span::styled("─────────────────", text_style)],
    );

    view.render_row(
        frame,
        4,
        vec![Span::styled(format!("Day: {}", state.day), text_style)],
    );
    view.render_row(
        frame,
        5,
        vec![Span::styled(
            format!("Cash: ${}", format_money(state.cash)),
            green,
        )],
    );
    view.render_row(
        frame,
        6,
        vec![Span::styled(
            format!("Net Worth: ${}", format_money(state.net_worth())),
            green,
        )],
    );

    view.render_row(frame, 8, vec![Span::styled("DAILY ECONOMICS", title_style)]);
    view.render_row(frame, 9, vec![Span::styled("───────────────", text_style)]);

    view.render_row(
        frame,
        10,
        vec![Span::styled(
            format!("Daily Income: ${}", state.daily_income()),
            green,
        )],
    );
    view.render_row(
        frame,
        11,
        vec![Span::styled(
            format!("Daily Expenses: ${}", state.daily_expenses()),
            red,
        )],
    );

    let net = state.daily_income() - state.daily_expenses();
    let net_style = if net >= 0 { green } else { red };
    view.render_row(
        frame,
        12,
        vec![Span::styled(format!("Net Daily: ${}", net), net_style)],
    );

    view.render_row(frame, 14, vec![Span::styled("PROPERTIES", title_style)]);
    view.render_row(frame, 15, vec![Span::styled("──────────", text_style)]);

    let houses = state
        .properties
        .iter()
        .filter(|p| p.owner == Owner::Player && p.property_type == PropertyType::House)
        .count();
    let shops = state
        .properties
        .iter()
        .filter(|p| p.owner == Owner::Player && p.property_type == PropertyType::Shop)
        .count();
    let factories = state
        .properties
        .iter()
        .filter(|p| p.owner == Owner::Player && p.property_type == PropertyType::Factory)
        .count();
    let parks = state
        .properties
        .iter()
        .filter(|p| p.owner == Owner::Player && p.property_type == PropertyType::Park)
        .count();

    view.render_row(
        frame,
        16,
        vec![Span::styled(
            format!("Houses: {}    Shops: {}", houses, shops),
            text_style,
        )],
    );
    view.render_row(
        frame,
        17,
        vec![Span::styled(
            format!("Factories: {}    Parks: {}", factories, parks),
            text_style,
        )],
    );
    view.render_row(
        frame,
        18,
        vec![Span::styled(
            format!("Fire Risk: {}", state.fire_risk()),
            if state.fire_risk() == "HIGH" {
                red
            } else {
                text_style
            },
        )],
    );

    view.render_help(frame, vec![("Enter/Esc", "close")]);
}

fn draw_disaster(frame: &mut Frame, area: Rect, state: &MicropolisState, colors: &ThemeColors) {
    // Draw city behind
    draw_city(frame, area, state, colors);

    // Disaster overlay
    let view = FullScreenView::new(area, " DISASTER! ", colors);

    let red = Style::default().fg(colors.red()).bold();
    let yellow = Style::default().fg(colors.yellow());

    view.render_row(
        frame,
        8,
        vec![Span::styled("╔════════════════════════════════╗", red)],
    );
    view.render_row(
        frame,
        9,
        vec![Span::styled("║          ^ FIRE! ^          ║", red)],
    );
    view.render_row(
        frame,
        10,
        vec![Span::styled("╠════════════════════════════════╣", red)],
    );

    if let Some(msg) = &state.message {
        view.render_row(
            frame,
            11,
            vec![Span::styled(format!("║ {:30} ║", msg), yellow)],
        );
    }

    view.render_row(
        frame,
        12,
        vec![Span::styled("║                                ║", red)],
    );
    view.render_row(
        frame,
        13,
        vec![Span::styled("║  Press R on burning buildings  ║", yellow)],
    );
    view.render_row(
        frame,
        14,
        vec![Span::styled("║  to repair and extinguish!     ║", yellow)],
    );
    view.render_row(
        frame,
        15,
        vec![Span::styled("╚════════════════════════════════╝", red)],
    );

    view.render_help(frame, vec![("Enter/Esc", "continue")]);
}

fn draw_game_over(frame: &mut Frame, area: Rect, state: &MicropolisState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " GAME OVER ", colors);
    view.render_frame(frame);

    let red = Style::default().fg(colors.red()).bold();
    let text_style = Style::default().fg(colors.fg());
    let yellow = Style::default().fg(colors.yellow());

    view.render_row(
        frame,
        5,
        vec![Span::styled("╔══════════════════════════════════╗", red)],
    );
    view.render_row(
        frame,
        6,
        vec![Span::styled("║           BANKRUPT!              ║", red)],
    );
    view.render_row(
        frame,
        7,
        vec![Span::styled("╚══════════════════════════════════╝", red)],
    );

    view.render_row(
        frame,
        9,
        vec![Span::styled(
            "Your real estate empire has collapsed!",
            text_style,
        )],
    );

    view.render_row(frame, 11, vec![Span::styled("FINAL STATS", yellow)]);
    view.render_row(frame, 12, vec![Span::styled("───────────", text_style)]);
    view.render_row(
        frame,
        13,
        vec![Span::styled(
            format!("Days Survived: {}", state.day),
            text_style,
        )],
    );
    view.render_row(
        frame,
        14,
        vec![Span::styled(
            format!("Properties Owned: {}", state.player_property_count()),
            text_style,
        )],
    );
    view.render_row(
        frame,
        15,
        vec![Span::styled(
            format!(
                "Peak Net Worth: ${}",
                format_money(state.get_score() as i64)
            ),
            yellow,
        )],
    );

    view.render_row(
        frame,
        17,
        vec![Span::styled("Press ENTER to try again", yellow)],
    );

    view.render_help(frame, vec![("Enter", "restart"), ("Esc", "quit")]);
}

// =============================================================================
// HELPERS
// =============================================================================

fn format_money(amount: i64) -> String {
    if amount >= 1_000_000 {
        format!("{:.1}M", amount as f64 / 1_000_000.0)
    } else if amount >= 1_000 {
        format!("{:.1}K", amount as f64 / 1_000.0)
    } else {
        format!("{}", amount)
    }
}

fn truncate_or_pad(s: &str, width: usize) -> String {
    if s.len() > width {
        s[..width].to_string()
    } else {
        format!("{:width$}", s, width = width)
    }
}
