//! GUMSHOE UI Modal - Detective game rendering
//!
//! Renders all views for the GUMSHOE geography detective game.

use super::super::gumshoe::{GumshoeState, GumshoeView};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;

/// Main draw function dispatcher
pub fn draw_gumshoe(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GumshoeState,
    colors: &ThemeColors,
) {
    match state.view {
        GumshoeView::CaseIntro => draw_case_intro(frame, view, state, colors),
        GumshoeView::Map => draw_map(frame, view, state, colors),
        GumshoeView::Investigate => draw_investigate(frame, view, state, colors),
        GumshoeView::Witness => draw_witness(frame, view, state, colors),
        GumshoeView::Travel => draw_travel(frame, view, state, colors),
        GumshoeView::Dossier => draw_dossier(frame, view, state, colors),
        GumshoeView::Arrest => draw_arrest(frame, view, state, colors),
        GumshoeView::CaseWon => draw_case_won(frame, view, state, colors),
        GumshoeView::CaseLost => draw_case_lost(frame, view, state, colors),
        GumshoeView::GameOver => draw_game_over(frame, view, state, colors),
    }
}

fn draw_header(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GumshoeState,
    colors: &ThemeColors,
) -> u16 {
    let mut row = 0u16;

    // Agency header
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                "INTERPOL DETECTIVE AGENCY",
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "            Agent: {}  Rank: {}",
                    state.get_rank(),
                    state.get_rank_stars()
                ),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 1;

    // Separator
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "\u{2500}".repeat(76),
            Style::default().fg(colors.grey()),
        )],
    );
    row += 1;

    row
}

fn draw_case_intro(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GumshoeState,
    colors: &ThemeColors,
) {
    let mut row: u16 = draw_header(frame, view, state, colors);
    row += 1;

    // Case file
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!("CASE FILE #{}", state.case_number),
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 2;

    // Stolen item
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("STOLEN: ", Style::default().fg(colors.fg())),
            Span::styled(&state.stolen_item, Style::default().fg(colors.red())),
        ],
    );
    row += 2;

    // Starting location
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("LAST SEEN: ", Style::default().fg(colors.fg())),
            Span::styled(
                format!(
                    "{}, {}",
                    state.current_city.name(),
                    state.current_city.country()
                ),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 2;

    // Time limit
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("TIME LIMIT: ", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{} hours", state.time_remaining),
                Style::default().fg(colors.green()),
            ),
        ],
    );
    row += 2;

    // Budget
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("BUDGET: ", Style::default().fg(colors.fg())),
            Span::styled(
                format!("${}", state.money),
                Style::default().fg(colors.green()),
            ),
        ],
    );
    row += 3;

    // Instructions
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Your mission: Track down the thief and recover the stolen item!",
            Style::default().fg(colors.fg()),
        )],
    );
    row += 1;
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Gather clues, identify the suspect, and make the arrest.",
            Style::default().fg(colors.grey()),
        )],
    );

    view.render_help(frame, vec![("Enter", "begin case"), ("Esc", "quit")]);
}

fn draw_map(frame: &mut Frame, view: &FullScreenView, state: &GumshoeState, colors: &ThemeColors) {
    let mut row: u16 = draw_header(frame, view, state, colors);

    // Current location and status
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                format!(
                    "Location: {}, {}",
                    state.current_city.name(),
                    state.current_city.country()
                ),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(
                format!("       Time: {}h", state.time_remaining),
                Style::default().fg(if state.time_remaining <= 12 {
                    colors.red()
                } else {
                    colors.green()
                }),
            ),
            Span::styled(
                format!("   ${}", state.money),
                Style::default().fg(colors.green()),
            ),
        ],
    );
    row += 1;

    // Separator
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "\u{2500}".repeat(76),
            Style::default().fg(colors.grey()),
        )],
    );
    row += 1;

    // Simple ASCII world map representation
    let map_art = [
        "                                    WORLD MAP                                  ",
        "     .---.                                                                     ",
        "    /     \\     EUROPE        ASIA                                            ",
        "   |  NA   |   .---.       .-------.                                          ",
        "    \\     /   / EUR \\     /  ASIA   \\                                        ",
        "     '---'    '-----'    '-----------'                                        ",
        "                  \\           |                                               ",
        "        .---.     AFRICA      |      .---.                                    ",
        "       / SA  \\   .----.      /      / OCE \\                                  ",
        "      '------'  | AFR  |    /      '------'                                   ",
        "                '------'   /                                                  ",
    ];

    for line in &map_art {
        view.render_row(
            frame,
            row,
            vec![Span::styled(*line, Style::default().fg(colors.grey()))],
        );
        row += 1;
    }

    // Current position marker
    row += 1;
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("You are at: ", Style::default().fg(colors.fg())),
            Span::styled(
                format!(
                    "{} ({})",
                    state.current_city.landmark(),
                    state.current_city.name()
                ),
                Style::default().fg(colors.yellow()),
            ),
        ],
    );
    row += 2;

    // Case info
    if let Some(_criminal) = state.get_criminal() {
        view.render_row(
            frame,
            row,
            vec![
                Span::styled("Case #", Style::default().fg(colors.fg())),
                Span::styled(
                    format!("{}", state.case_number),
                    Style::default().fg(colors.cyan()),
                ),
                Span::styled(
                    format!(
                        "   Suspect: {}",
                        if state.suspect_clues.is_empty() {
                            "Unknown"
                        } else {
                            "Partial ID"
                        }
                    ),
                    Style::default().fg(colors.grey()),
                ),
                Span::styled(
                    format!("   Clues: {}", state.suspect_clues.len()),
                    Style::default().fg(colors.green()),
                ),
            ],
        );
    }

    // Warrant status
    row += 1;
    if state.warrant_ready {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "WARRANT READY - Press A to make arrest!",
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            )],
        );
    } else {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!(
                    "Need {} more clues for warrant",
                    3_usize.saturating_sub(state.suspect_clues.len())
                ),
                Style::default().fg(colors.grey()),
            )],
        );
    }

    let mut help = vec![("I", "investigate"), ("T", "travel"), ("D", "dossier")];
    if state.warrant_ready {
        help.push(("A", "arrest"));
    }
    help.push(("Esc", "quit"));
    view.render_help(frame, help);
}

fn draw_investigate(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GumshoeState,
    colors: &ThemeColors,
) {
    let mut row: u16 = draw_header(frame, view, state, colors);

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "INVESTIGATING: {}, {}",
                state.current_city.name(),
                state.current_city.country()
            ),
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 2;

    // Investigation options
    let options = [
        (
            "Airport",
            "Talk to ticket agents",
            state.investigated_airport,
        ),
        ("Hotel", "Interview the concierge", state.investigated_hotel),
        (
            "Landmark",
            "Search for witnesses",
            state.investigated_landmark,
        ),
    ];

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "WHERE TO INVESTIGATE? (4 hours each)",
            Style::default().fg(colors.fg()),
        )],
    );
    row += 2;

    for (i, (name, desc, done)) in options.iter().enumerate() {
        let prefix = if i == state.selected_investigation {
            "> "
        } else {
            "  "
        };
        let status = if *done { " [DONE]" } else { "" };

        let style = if *done {
            Style::default().fg(colors.grey())
        } else if i == state.selected_investigation {
            Style::default().fg(colors.yellow())
        } else {
            Style::default().fg(colors.fg())
        };

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("{}[{}] {} - {}{}", prefix, i + 1, name, desc, status),
                style,
            )],
        );
        row += 1;
    }

    view.render_help(
        frame,
        vec![("↑↓", "select"), ("Enter", "investigate"), ("Esc", "back")],
    );
}

fn draw_witness(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GumshoeState,
    colors: &ThemeColors,
) {
    let mut row: u16 = draw_header(frame, view, state, colors);

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "WITNESS INTERVIEW",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 2;

    // Witness ASCII art
    let witness = ["     .---.", "    ( o o )", "     \\ - /", "      '-'"];
    for line in &witness {
        view.render_row(
            frame,
            row,
            vec![Span::styled(*line, Style::default().fg(colors.cyan()))],
        );
        row += 1;
    }
    row += 1;

    // Clue
    if let Some(clue) = &state.last_clue {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("\"{}\"", clue),
                Style::default().fg(colors.fg()),
            )],
        );
    }

    view.render_help(frame, vec![("Enter", "continue")]);
}

fn draw_travel(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GumshoeState,
    colors: &ThemeColors,
) {
    let mut row: u16 = draw_header(frame, view, state, colors);

    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                "TRAVEL DESK",
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "                                     Budget: ${}",
                    state.money
                ),
                Style::default().fg(colors.green()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "Current: {}, {}",
                state.current_city.name(),
                state.current_city.country()
            ),
            Style::default().fg(colors.cyan()),
        )],
    );
    row += 2;

    // Table header
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "{:<20} {:<15} {:>8} {:>8}",
                "CITY", "COUNTRY", "TIME", "COST"
            ),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "\u{2500}".repeat(55),
            Style::default().fg(colors.grey()),
        )],
    );
    row += 1;

    // Destinations
    let destinations = state.get_destinations();
    let visible_count = 10.min(destinations.len());

    for (i, (city, time, cost)) in destinations.iter().take(visible_count).enumerate() {
        let prefix = if i == state.selected_destination {
            "> "
        } else {
            "  "
        };
        let can_afford = *cost <= state.money && *time <= state.time_remaining;

        let style = if !can_afford {
            Style::default().fg(colors.grey())
        } else if i == state.selected_destination {
            Style::default().fg(colors.yellow())
        } else {
            Style::default().fg(colors.fg())
        };

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!(
                    "{}{:<18} {:<15} {:>5}h ${:>6}",
                    prefix,
                    city.name(),
                    city.country(),
                    time,
                    cost
                ),
                style,
            )],
        );
        row += 1;
    }

    view.render_help(
        frame,
        vec![("↑↓", "select"), ("Enter", "book flight"), ("Esc", "back")],
    );
}

fn draw_dossier(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GumshoeState,
    colors: &ThemeColors,
) {
    let mut row: u16 = draw_header(frame, view, state, colors);

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "SUSPECT DOSSIER",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 2;

    // Clues gathered
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "CONFIRMED DETAILS:",
            Style::default().fg(colors.blue()),
        )],
    );
    row += 1;

    if state.suspect_clues.is_empty() {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "  No clues gathered yet. Investigate to find leads!",
                Style::default().fg(colors.grey()),
            )],
        );
        row += 1;
    } else {
        for clue in &state.suspect_clues {
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled(
                        format!("  {}: ", clue.clue_type),
                        Style::default().fg(colors.fg()),
                    ),
                    Span::styled(&clue.value, Style::default().fg(colors.green())),
                ],
            );
            row += 1;
        }
    }
    row += 1;

    // Warrant status
    let warrant_text = if state.warrant_ready {
        "READY TO ARREST".to_string()
    } else {
        format!(
            "Need {} more clues",
            3_usize.saturating_sub(state.suspect_clues.len())
        )
    };
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!("WARRANT STATUS: {}", warrant_text),
            Style::default().fg(if state.warrant_ready {
                colors.green()
            } else {
                colors.grey()
            }),
        )],
    );

    view.render_help(frame, vec![("Esc", "back")]);
}

fn draw_arrest(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GumshoeState,
    colors: &ThemeColors,
) {
    let mut row: u16 = draw_header(frame, view, state, colors);

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "MAKE AN ARREST",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 2;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Select the suspect that matches your clues:",
            Style::default().fg(colors.fg()),
        )],
    );
    row += 2;

    // Show matching suspects
    let suspects = state.get_matching_suspects();
    let matching: Vec<_> = suspects.iter().filter(|(_, _, m)| *m).collect();

    for (i, (_, criminal, _)) in matching.iter().enumerate() {
        let prefix = if i == state.selected_suspect {
            "> "
        } else {
            "  "
        };
        let style = if i == state.selected_suspect {
            Style::default().fg(colors.yellow())
        } else {
            Style::default().fg(colors.fg())
        };

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!(
                    "{}[{}] {} - {}, {} hair, {}",
                    prefix,
                    i + 1,
                    criminal.name,
                    criminal.gender,
                    criminal.hair.name(),
                    criminal.feature.name()
                ),
                style,
            )],
        );
        row += 1;
    }

    row += 1;
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "WARNING: Wrong arrest costs 12 hours!",
            Style::default().fg(colors.red()),
        )],
    );

    view.render_help(
        frame,
        vec![("↑↓", "select"), ("Enter", "arrest"), ("Esc", "back")],
    );
}

fn draw_case_won(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GumshoeState,
    colors: &ThemeColors,
) {
    let mut row: u16 = 2;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "CASE SOLVED!",
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 2;

    if let Some(criminal) = state.get_criminal() {
        view.render_row(
            frame,
            row,
            vec![
                Span::styled("Arrested: ", Style::default().fg(colors.fg())),
                Span::styled(criminal.name, Style::default().fg(colors.yellow())),
            ],
        );
        row += 1;
    }

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Recovered: ", Style::default().fg(colors.fg())),
            Span::styled(&state.stolen_item, Style::default().fg(colors.cyan())),
        ],
    );
    row += 2;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Time Remaining: ", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{} hours", state.time_remaining),
                Style::default().fg(colors.green()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Score: ", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{}", state.score),
                Style::default().fg(colors.yellow()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Rank: ", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{} {}", state.get_rank(), state.get_rank_stars()),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Cases Solved: ", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{}", state.cases_solved),
                Style::default().fg(colors.green()),
            ),
        ],
    );

    view.render_help(frame, vec![("Enter", "next case"), ("Esc", "quit")]);
}

fn draw_case_lost(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GumshoeState,
    colors: &ThemeColors,
) {
    let mut row: u16 = 2;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "TIME'S UP!",
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 2;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "The suspect escaped!",
            Style::default().fg(colors.fg()),
        )],
    );
    row += 2;

    if let Some(criminal) = state.get_criminal() {
        view.render_row(
            frame,
            row,
            vec![
                Span::styled("It was: ", Style::default().fg(colors.fg())),
                Span::styled(criminal.name, Style::default().fg(colors.yellow())),
            ],
        );
        row += 1;
    }

    row += 1;
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Final Score: ", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{}", state.score),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Cases Solved: ", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{}", state.cases_solved),
                Style::default().fg(colors.green()),
            ),
        ],
    );

    view.render_help(frame, vec![("Enter/Esc", "quit")]);
}

fn draw_game_over(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GumshoeState,
    colors: &ThemeColors,
) {
    draw_case_lost(frame, view, state, colors);
}
