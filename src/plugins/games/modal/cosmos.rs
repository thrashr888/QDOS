//! COSMOS modal rendering
//!
//! Renders the space exploration game with starfields, planet views,
//! and alien first contact sequences.

use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::{
    prelude::*,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

use super::super::cosmos::{CosmosState, CosmosView, DiplomaticStatus, PlanetType};

/// Main draw function for COSMOS
pub fn draw(frame: &mut Frame, area: Rect, state: &CosmosState, colors: &ThemeColors) {
    match state.view {
        CosmosView::Menu => draw_menu(frame, area, state, colors),
        CosmosView::GalaxyMap => draw_galaxy_map(frame, area, state, colors),
        CosmosView::StarSystem => draw_star_system(frame, area, state, colors),
        CosmosView::PlanetSurface => draw_planet_surface(frame, area, state, colors),
        CosmosView::FirstContact => draw_first_contact(frame, area, state, colors),
        CosmosView::Ship => draw_ship(frame, area, state, colors),
        CosmosView::Knowledge => draw_knowledge(frame, area, state, colors),
    }
}

fn draw_menu(frame: &mut Frame, area: Rect, state: &CosmosState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " COSMOS ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let cyan = Style::default().fg(colors.cyan());
    let grey = Style::default().fg(colors.grey());

    // Animated starfield background elements
    let tick = state.tick_count;
    let stars = ["·", "✦", "✧", "*", "·", "✧", "·", "✦"];
    let star_offset = (tick / 3) as usize;

    // Title with stars
    view.render_row(
        frame,
        0,
        vec![Span::styled(
            format!(
                "        {}  ✦  {}    *    {}  ✧  {}",
                stars[(star_offset) % 8],
                stars[(star_offset + 1) % 8],
                stars[(star_offset + 2) % 8],
                stars[(star_offset + 3) % 8]
            ),
            grey,
        )],
    );

    view.render_row(
        frame,
        1,
        vec![Span::styled("   *    ·       ✦        ·      *", grey)],
    );

    // COSMOS title
    view.render_row(
        frame,
        3,
        vec![Span::styled("   ╔═══════════════════════════════╗", cyan)],
    );
    view.render_row(
        frame,
        4,
        vec![Span::styled("   ║   ██████╗ ██████╗ ███████╗    ║", cyan)],
    );
    view.render_row(
        frame,
        5,
        vec![Span::styled("   ║  ██╔════╝██╔═══██╗██╔════╝    ║", cyan)],
    );
    view.render_row(
        frame,
        6,
        vec![Span::styled("   ║  ██║     ██║   ██║███████╗    ║", yellow)],
    );
    view.render_row(
        frame,
        7,
        vec![Span::styled("   ║  ██║     ██║   ██║╚════██║    ║", yellow)],
    );
    view.render_row(
        frame,
        8,
        vec![Span::styled("   ║  ╚██████╗╚██████╔╝███████║    ║", cyan)],
    );
    view.render_row(
        frame,
        9,
        vec![Span::styled("   ║   ╚═════╝ ╚═════╝ ╚══════╝    ║", cyan)],
    );
    view.render_row(
        frame,
        10,
        vec![Span::styled("   ║                               ║", cyan)],
    );
    view.render_row(
        frame,
        11,
        vec![Span::styled("   ║     🚀 A Space Odyssey 🚀     ║", yellow)],
    );
    view.render_row(
        frame,
        12,
        vec![Span::styled("   ╚═══════════════════════════════╝", cyan)],
    );

    // Tagline
    view.render_row(
        frame,
        14,
        vec![Span::styled(
            "        \"Seek. Learn. Wonder.\"",
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::ITALIC),
        )],
    );

    // Instructions
    view.render_row(
        frame,
        16,
        vec![Span::styled("   Explore the galaxy, discover", white)],
    );
    view.render_row(
        frame,
        17,
        vec![Span::styled("   new worlds, and make contact", white)],
    );
    view.render_row(
        frame,
        18,
        vec![Span::styled("   with alien civilizations.", white)],
    );

    view.render_help(frame, vec![("Enter", "begin"), ("Esc", "quit")]);
}

fn draw_galaxy_map(frame: &mut Frame, area: Rect, state: &CosmosState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Galaxy Map ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let cyan = Style::default().fg(colors.cyan());
    let green = Style::default().fg(colors.green());
    let grey = Style::default().fg(colors.grey());
    let red = Style::default().fg(colors.red());

    // Status bar
    let fuel_bar = "█".repeat((state.fuel / 10) as usize);
    let fuel_empty = "░".repeat((10 - state.fuel / 10) as usize);
    view.render_row(
        frame,
        0,
        vec![
            Span::styled("Fuel: ", white),
            Span::styled(fuel_bar, green),
            Span::styled(fuel_empty, grey),
            Span::styled(format!(" {}%", state.fuel), white),
            Span::styled("  Data: ", white),
            Span::styled(format!("{}", state.data_collected), cyan),
            Span::styled(
                format!("  Explored: {}%", state.exploration_percentage()),
                grey,
            ),
        ],
    );

    // Draw star systems on a grid
    for (i, system) in state.systems.iter().enumerate() {
        // Map system position to screen
        let screen_x = (system.x + 5) as u16;
        let screen_y = (system.y + 2) as u16;

        if screen_y > 18 {
            continue;
        }

        let is_current = i == state.current_system;
        let is_selected = i == state.selected_system;

        let style = if is_current && is_selected {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else if is_current {
            yellow
        } else if is_selected {
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD)
        } else if system.visited {
            green
        } else {
            grey
        };

        // Create a line with the star positioned correctly
        let prefix_spaces = " ".repeat(screen_x as usize);
        let star_symbol = if system.fully_explored {
            format!("[{}]", system.star_type.symbol())
        } else if system.visited {
            format!("({})", system.star_type.symbol())
        } else {
            system.star_type.symbol().to_string()
        };

        view.render_row(
            frame,
            screen_y,
            vec![Span::styled(
                format!("{}{}", prefix_spaces, star_symbol),
                style,
            )],
        );
    }

    // Selected system info
    if state.selected_system < state.systems.len() {
        let sys = &state.systems[state.selected_system];
        let status = if state.selected_system == state.current_system {
            " (YOU ARE HERE)"
        } else if sys.visited {
            " (visited)"
        } else {
            ""
        };

        view.render_row(
            frame,
            16,
            vec![
                Span::styled(
                    format!("Selected: {} {}", sys.name, sys.star_type.name()),
                    yellow,
                ),
                Span::styled(status.to_string(), green),
            ],
        );
        view.render_row(
            frame,
            17,
            vec![Span::styled(
                format!(
                    "Planets: {}  Warp cost: {} fuel",
                    sys.planets.len(),
                    if state.selected_system == state.current_system {
                        0
                    } else {
                        15
                    }
                ),
                white,
            )],
        );
    }

    // Message
    if let Some(msg) = &state.message {
        view.render_row(frame, 18, vec![Span::styled(msg.clone(), red)]);
    }

    view.render_help(
        frame,
        vec![
            ("Arrows", "select"),
            ("W/Enter", "warp"),
            ("S", "ship"),
            ("Esc", "quit"),
        ],
    );
}

fn draw_star_system(frame: &mut Frame, area: Rect, state: &CosmosState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Star System ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let cyan = Style::default().fg(colors.cyan());
    let green = Style::default().fg(colors.green());
    let grey = Style::default().fg(colors.grey());
    let red = Style::default().fg(colors.red());

    let system = state.current_system();

    // System header
    view.render_row(
        frame,
        0,
        vec![
            Span::styled(format!("{} SYSTEM", system.name.to_uppercase()), yellow),
            Span::styled(format!("  {}", system.star_type.symbol()), cyan),
            Span::styled(format!(" {}", system.star_type.name()), white),
        ],
    );

    // Fuel bar
    let fuel_bar = "█".repeat((state.fuel / 10) as usize);
    let fuel_empty = "░".repeat((10 - state.fuel / 10) as usize);
    view.render_row(
        frame,
        1,
        vec![
            Span::styled("Fuel: ", white),
            Span::styled(fuel_bar, green),
            Span::styled(fuel_empty, grey),
            Span::styled(format!(" {}%", state.fuel), white),
        ],
    );

    view.render_row(frame, 2, vec![Span::styled("─".repeat(50), grey)]);

    // Planet list
    for (i, planet) in system.planets.iter().enumerate() {
        let is_selected = i == state.selected_planet;
        let row = 3 + i as u16;

        let prefix = if is_selected { "▸ " } else { "  " };

        let mut spans = vec![];

        // Selection indicator and planet symbol
        let planet_style = if is_selected { yellow } else { white };
        spans.push(Span::styled(prefix.to_string(), planet_style));
        spans.push(Span::styled(
            format!("{} ", planet.planet_type.symbol()),
            planet_style,
        ));
        spans.push(Span::styled(format!("{:<12}", planet.name), planet_style));

        // Planet type
        spans.push(Span::styled(
            format!("{:<12}", planet.planet_type.name()),
            if is_selected { cyan } else { grey },
        ));

        // Scan status
        if planet.scanned {
            if planet.has_life {
                spans.push(Span::styled(" LIFE", green));
            }
            if planet.has_ruins {
                spans.push(Span::styled(" RUINS", yellow));
            }
            if planet.alien_species.is_some() {
                spans.push(Span::styled(" SIGNALS", cyan));
            }
            if planet.landed {
                spans.push(Span::styled(" ✓", green));
            }
        } else {
            spans.push(Span::styled(" [unscanned]", grey));
        }

        view.render_row(frame, row, spans);
    }

    // Selected planet details
    let detail_row = 3 + system.planets.len() as u16 + 1;
    if state.selected_planet < system.planets.len() {
        let planet = &system.planets[state.selected_planet];
        view.render_row(frame, detail_row, vec![Span::styled("─".repeat(50), grey)]);

        let mut details = format!("{}: {}", planet.name, planet.planet_type.name());
        if planet.planet_type == PlanetType::GasGiant {
            details.push_str(" - Can refuel here");
        } else if !planet.scanned {
            details.push_str(" - Scan to learn more");
        }
        view.render_row(frame, detail_row + 1, vec![Span::styled(details, white)]);
    }

    // Message
    if let Some(msg) = &state.message {
        view.render_row(frame, 17, vec![Span::styled(msg.clone(), red)]);
    }

    view.render_help(
        frame,
        vec![
            ("↑/↓", "select"),
            ("S", "scan"),
            ("L", "land"),
            ("R", "refuel"),
            ("M", "map"),
        ],
    );
}

fn draw_planet_surface(frame: &mut Frame, area: Rect, state: &CosmosState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Planet Surface ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let cyan = Style::default().fg(colors.cyan());
    let green = Style::default().fg(colors.green());
    let grey = Style::default().fg(colors.grey());
    let red = Style::default().fg(colors.red());

    if let Some(planet_idx) = state.current_planet {
        let planet = &state.systems[state.current_system].planets[planet_idx];

        view.render_row(
            frame,
            0,
            vec![
                Span::styled(format!("{} ", planet.planet_type.symbol()), yellow),
                Span::styled(planet.name.clone(), yellow),
                Span::styled(format!(" - {}", planet.planet_type.name()), white),
            ],
        );

        view.render_row(frame, 1, vec![Span::styled("─".repeat(50), grey)]);

        // Surface visualization based on planet type
        let (terrain1, terrain2, terrain3) = match planet.planet_type {
            PlanetType::Terrestrial => ("🌲🌲🌲 🏔️ 🏔️ 🌲🌲", "🌲🌲 🏔️  🏔️ 🌲🌲", "🌲🌲🌲🌲 🌲🌲🌲"),
            PlanetType::Desert => ("🏜️ 🏜️ 🏜️ 🌵 🏜️", "🏜️ 🌵 🏜️ 🏜️ 🏜️", "🏜️ 🏜️ 🏜️ 🏜️ 🌵"),
            PlanetType::Ice => ("❄️ ❄️ ❄️ 🏔️ ❄️", "❄️ 🏔️ ❄️ ❄️ ❄️", "❄️ ❄️ ❄️ ❄️ 🏔️"),
            PlanetType::Ocean => ("🌊 🌊 🌊 🏝️ 🌊", "🌊 🏝️ 🌊 🌊 🌊", "🌊 🌊 🌊 🌊 🌊"),
            PlanetType::Volcanic => ("🌋 🌋 🔥 🔥 🌋", "🔥 🌋 🌋 🔥 🌋", "🌋 🔥 🌋 🌋 🔥"),
            PlanetType::Barren => ("🌑 🌑 🌑 🪨 🌑", "🌑 🪨 🌑 🌑 🌑", "🌑 🌑 🌑 🌑 🪨"),
            PlanetType::GasGiant => ("☁️ ☁️ ☁️ ☁️ ☁️", "☁️ ☁️ ☁️ ☁️ ☁️", "☁️ ☁️ ☁️ ☁️ ☁️"),
        };

        view.render_row(
            frame,
            3,
            vec![Span::styled(format!("  {}", terrain1), white)],
        );
        view.render_row(
            frame,
            4,
            vec![Span::styled(format!("  {}", terrain2), white)],
        );
        view.render_row(
            frame,
            5,
            vec![Span::styled(format!("  {}", terrain3), white)],
        );

        view.render_row(frame, 7, vec![Span::styled("─".repeat(50), grey)]);

        // Planet information
        let mut info_row = 8;
        if planet.has_life {
            view.render_row(
                frame,
                info_row,
                vec![Span::styled("  ✓ Complex life forms detected", green)],
            );
            info_row += 1;
        }
        if planet.has_ruins {
            view.render_row(
                frame,
                info_row,
                vec![Span::styled("  ★ Ancient ruins discovered!", yellow)],
            );
            info_row += 1;
        }
        if let Some(species) = planet.alien_species {
            view.render_row(
                frame,
                info_row,
                vec![
                    Span::styled("  ◈ Intelligent signals: ", cyan),
                    Span::styled(species.name(), yellow),
                ],
            );

            let status = state.get_diplomatic_status(species);
            view.render_row(
                frame,
                info_row + 1,
                vec![
                    Span::styled("    Diplomatic status: ", white),
                    Span::styled(
                        status.name(),
                        match status {
                            DiplomaticStatus::Allied => green,
                            DiplomaticStatus::Friendly => cyan,
                            DiplomaticStatus::Cautious => yellow,
                            _ => grey,
                        },
                    ),
                ],
            );
        }

        // Message
        if let Some(msg) = &state.message {
            view.render_row(frame, 17, vec![Span::styled(msg.clone(), red)]);
        }

        // Help based on what's available
        if planet.alien_species.is_some() {
            view.render_help(frame, vec![("C", "contact"), ("O/Esc", "orbit")]);
        } else {
            view.render_help(frame, vec![("O/Esc", "return to orbit")]);
        }
    }
}

fn draw_first_contact(frame: &mut Frame, area: Rect, state: &CosmosState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " First Contact ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let cyan = Style::default().fg(colors.cyan());
    let green = Style::default().fg(colors.green());
    let grey = Style::default().fg(colors.grey());

    if let Some(species) = state.active_contact {
        view.render_row(
            frame,
            0,
            vec![Span::styled(
                format!("  Contacting: {}", species.name()),
                yellow,
            )],
        );

        view.render_row(frame, 2, vec![Span::styled("─".repeat(50), grey)]);

        // Animated transmission
        let phase = state.contact_phase as usize;
        let dots = ".".repeat((phase % 4) + 1);

        view.render_row(
            frame,
            4,
            vec![Span::styled(
                format!("  Incoming transmission{}", dots),
                cyan,
            )],
        );

        view.render_row(
            frame,
            6,
            vec![Span::styled("  ┌─────────────────────────────────┐", white)],
        );
        view.render_row(
            frame,
            7,
            vec![Span::styled(
                format!("  │  {}  │", species.greeting()),
                cyan,
            )],
        );
        view.render_row(
            frame,
            8,
            vec![Span::styled("  └─────────────────────────────────┘", white)],
        );

        view.render_row(frame, 10, vec![Span::styled("  Translation:", grey)]);
        view.render_row(
            frame,
            11,
            vec![Span::styled(
                format!("  \"{}\"", species.translated_greeting()),
                green,
            )],
        );

        let status = state.get_diplomatic_status(species);
        view.render_row(
            frame,
            14,
            vec![
                Span::styled("  Status: ", white),
                Span::styled(status.name(), yellow),
            ],
        );

        if status == DiplomaticStatus::FirstContact {
            view.render_row(
                frame,
                16,
                vec![Span::styled(
                    format!("  Knowledge shared: +{} data", species.knowledge_gift()),
                    green,
                )],
            );
        }
    }

    view.render_help(frame, vec![("Enter", "acknowledge"), ("Esc", "leave")]);
}

fn draw_ship(frame: &mut Frame, area: Rect, state: &CosmosState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Ship: CURIOSITY ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let cyan = Style::default().fg(colors.cyan());
    let green = Style::default().fg(colors.green());
    let grey = Style::default().fg(colors.grey());

    // Ship ASCII art
    view.render_row(
        frame,
        1,
        vec![Span::styled("         ╱═══════════════╲", cyan)],
    );
    view.render_row(
        frame,
        2,
        vec![Span::styled("        ╱   ╭─────────╮   ╲", cyan)],
    );
    view.render_row(
        frame,
        3,
        vec![Span::styled("       ║    │ BRIDGE  │    ║", cyan)],
    );
    view.render_row(
        frame,
        4,
        vec![Span::styled("       ║    ╰────╥────╯    ║", cyan)],
    );
    view.render_row(
        frame,
        5,
        vec![Span::styled("  ┌────╨─────────╨─────────╨────┐", cyan)],
    );
    view.render_row(
        frame,
        6,
        vec![Span::styled("  │ ▓▓▓▓▓  ║CARGO║  ▓▓▓▓▓ │", cyan)],
    );
    view.render_row(
        frame,
        7,
        vec![Span::styled("  │ ▓ENG▓  ╠═════╣  ▓LAB▓ │", cyan)],
    );
    view.render_row(
        frame,
        8,
        vec![Span::styled("  └───┬────╨─────╨────┬───┘", cyan)],
    );
    view.render_row(frame, 9, vec![Span::styled("      └═══╧═════╧═══╝", cyan)]);

    view.render_row(frame, 11, vec![Span::styled("─".repeat(50), grey)]);

    // Ship stats
    let fuel_bar = "█".repeat((state.fuel / 10) as usize);
    let fuel_empty = "░".repeat((10 - state.fuel / 10) as usize);
    view.render_row(
        frame,
        12,
        vec![
            Span::styled("  Fuel:   ", white),
            Span::styled(fuel_bar, green),
            Span::styled(fuel_empty, grey),
            Span::styled(format!(" {}%", state.fuel), white),
        ],
    );

    let hull_bar = "█".repeat((state.hull / 10) as usize);
    let hull_empty = "░".repeat((10 - state.hull / 10) as usize);
    view.render_row(
        frame,
        13,
        vec![
            Span::styled("  Hull:   ", white),
            Span::styled(hull_bar, cyan),
            Span::styled(hull_empty, grey),
            Span::styled(format!(" {}%", state.hull), white),
        ],
    );

    view.render_row(
        frame,
        14,
        vec![
            Span::styled("  Data:   ", white),
            Span::styled(format!("{} units collected", state.data_collected), yellow),
        ],
    );

    view.render_help(frame, vec![("Enter/Esc", "back to map")]);
}

fn draw_knowledge(frame: &mut Frame, area: Rect, state: &CosmosState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Knowledge Database ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let cyan = Style::default().fg(colors.cyan());
    let green = Style::default().fg(colors.green());
    let grey = Style::default().fg(colors.grey());

    view.render_row(
        frame,
        0,
        vec![
            Span::styled("  Data Collected: ", white),
            Span::styled(format!("{}", state.data_collected), yellow),
        ],
    );

    view.render_row(frame, 2, vec![Span::styled("  EXPLORATION", cyan)]);
    view.render_row(frame, 3, vec![Span::styled("─".repeat(40), grey)]);

    view.render_row(
        frame,
        4,
        vec![
            Span::styled("  Stars explored:   ", white),
            Span::styled(
                format!("{}/{}", state.stars_explored, state.systems.len()),
                green,
            ),
        ],
    );
    view.render_row(
        frame,
        5,
        vec![
            Span::styled("  Planets scanned:  ", white),
            Span::styled(format!("{}", state.planets_scanned), green),
        ],
    );
    view.render_row(
        frame,
        6,
        vec![
            Span::styled("  Planets landed:   ", white),
            Span::styled(format!("{}", state.planets_landed), green),
        ],
    );
    view.render_row(
        frame,
        7,
        vec![
            Span::styled("  Ruins discovered: ", white),
            Span::styled(format!("{}", state.ruins_discovered), yellow),
        ],
    );

    view.render_row(frame, 9, vec![Span::styled("  DIPLOMACY", cyan)]);
    view.render_row(frame, 10, vec![Span::styled("─".repeat(40), grey)]);

    view.render_row(
        frame,
        11,
        vec![
            Span::styled("  Species contacted: ", white),
            Span::styled(format!("{}", state.species_contacted), green),
        ],
    );

    // List contacted species
    for (i, contact) in state.contacts.iter().enumerate() {
        view.render_row(
            frame,
            12 + i as u16,
            vec![
                Span::styled(format!("    {} ", contact.species.symbol()), cyan),
                Span::styled(contact.species.name(), white),
                Span::styled(format!(" - {}", contact.status.name()), yellow),
            ],
        );
    }

    view.render_help(frame, vec![("Enter/Esc", "back to map")]);
}
