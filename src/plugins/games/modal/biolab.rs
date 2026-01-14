//! BIOLAB modal rendering
//!
//! Renders all views for the BIOLAB educational biology game with colorful diagrams.

use super::super::biolab::{
    BiolabState, BiolabView, BodyPart, DnaLabMode, LabType, Organelle, BODY_SYSTEMS, SLIDES,
    SPECIMENS,
};
use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
    Frame,
};

// =============================================================================
// MAIN DISPATCHER
// =============================================================================

pub fn draw_biolab(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BiolabState,
    colors: &ThemeColors,
) {
    match state.view {
        BiolabView::LabMenu => draw_menu(frame, view, state, colors),
        BiolabView::Microscope => draw_microscope(frame, view, state, colors),
        BiolabView::DnaLab => draw_dna_lab(frame, view, state, colors),
        BiolabView::Dissection => draw_dissection(frame, view, state, colors),
        BiolabView::Anatomy => draw_anatomy(frame, view, state, colors),
        BiolabView::Quiz => draw_quiz(frame, view, state, colors),
        BiolabView::QuizFeedback => draw_feedback(frame, view, state, colors),
        BiolabView::QuizResults => draw_results(frame, view, state, colors),
        BiolabView::Progress => draw_progress(frame, view, state, colors),
        BiolabView::Loading => draw_loading(frame, view, state, colors),
        BiolabView::Error => draw_error(frame, view, state, colors),
    }
}

// =============================================================================
// LAB MENU
// =============================================================================

fn draw_menu(frame: &mut Frame, view: &FullScreenView, state: &BiolabState, colors: &ThemeColors) {
    let mut row = 0u16;

    // Title
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╔══════════════════════════════════════════════════════════════════════════╗",
            Style::default().fg(colors.cyan()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "║                      🔬  B I O L A B  🔬                                 ║",
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "║                   Interactive Biology Learning                           ║",
            Style::default().fg(colors.cyan()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╠══════════════════════════════════════════════════════════════════════════╣",
            Style::default().fg(colors.cyan()),
        )],
    );
    row += 2;

    // Lab selection
    for (i, lab) in LabType::all().iter().enumerate() {
        let selected = i == state.selected_lab;
        let prefix = if selected { "►" } else { " " };

        let name_style = if selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.blue())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg())
        };

        let icon_color = match lab {
            LabType::Microscope => colors.cyan(),
            LabType::DnaLab => colors.green(),
            LabType::Dissection => colors.red(),
            LabType::Anatomy => colors.yellow(),
        };

        view.render_row(
            frame,
            row,
            vec![
                Span::styled(format!("   {} ", prefix), name_style),
                Span::styled(format!("{}) ", i + 1), Style::default().fg(colors.grey())),
                Span::styled(format!("{} ", lab.icon()), Style::default().fg(icon_color)),
                Span::styled(format!("{:<12}", lab.name()), name_style),
                Span::styled(
                    format!(" - {}", lab.description()),
                    Style::default().fg(colors.grey()),
                ),
            ],
        );
        row += 2;
    }

    // Stats summary
    row += 1;
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╠══════════════════════════════════════════════════════════════════════════╣",
            Style::default().fg(colors.cyan()),
        )],
    );
    row += 1;

    let slides_done = state.slides_viewed.iter().filter(|&&v| v).count();
    let specimens_done = state.specimens_viewed.iter().filter(|&&v| v).count();
    let systems_done = state.systems_viewed.iter().filter(|&&v| v).count();

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("   Score: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.total_score),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  Slides: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}/{}", slides_done, SLIDES.len()),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled("  │  Specimens: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}/{}", specimens_done, SPECIMENS.len()),
                Style::default().fg(colors.red()),
            ),
            Span::styled("  │  Systems: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}/{}", systems_done, BODY_SYSTEMS.len()),
                Style::default().fg(colors.green()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("   Topics Mastered: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.topics_mastered.len()),
                Style::default()
                    .fg(colors.green())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  Quizzes: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.quizzes_taken),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled("  │  Perfect: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.perfect_quizzes),
                Style::default().fg(colors.yellow()),
            ),
        ],
    );

    view.render_help(
        frame,
        vec![
            ("↑↓/1-4", "select"),
            ("Enter", "open"),
            ("P", "progress"),
            ("Esc", "exit"),
        ],
    );
}

// =============================================================================
// MICROSCOPE LAB
// =============================================================================

fn draw_microscope(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BiolabState,
    colors: &ThemeColors,
) {
    let slide = state.current_slide();
    let mut row = 0u16;

    // Header with slide navigation
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(" 🔬 MICROSCOPE  ", Style::default().fg(colors.cyan())),
            Span::styled(
                format!("Slide {}/{}: ", state.selected_slide + 1, SLIDES.len()),
                Style::default().fg(colors.grey()),
            ),
            Span::styled(
                slide.name,
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "────────────────────────────────────────────────────────────────────────────",
            Style::default().fg(colors.blue()),
        )],
    );
    row += 1;

    // Cell diagram based on slide type
    draw_cell_diagram(
        frame,
        view,
        row,
        slide.name,
        state.selected_organelle,
        colors,
    );
    row += 13;

    // Description
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            slide.description,
            Style::default().fg(colors.grey()),
        )],
    );
    row += 2;

    // Organelle list
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Structures:",
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    for (i, org) in slide.organelles.iter().enumerate() {
        let selected = i == state.selected_organelle;
        draw_organelle_item(frame, view, row, org, selected, colors);
        row += 1;
    }

    view.render_help(
        frame,
        vec![
            ("←→", "slides"),
            ("↑↓", "select"),
            ("Q", "quiz"),
            ("Esc", "back"),
        ],
    );
}

fn draw_cell_diagram(
    frame: &mut Frame,
    view: &FullScreenView,
    start_row: u16,
    slide_name: &str,
    selected: usize,
    colors: &ThemeColors,
) {
    let diagram = match slide_name {
        "Animal Cell" => get_animal_cell_art(selected, colors),
        "Plant Cell" => get_plant_cell_art(selected, colors),
        "Bacteria" => get_bacteria_art(selected, colors),
        "Blood Cells" => get_blood_cell_art(selected, colors),
        "Neuron" => get_neuron_art(selected, colors),
        "Muscle Fiber" => get_muscle_art(selected, colors),
        _ => get_animal_cell_art(selected, colors),
    };

    for (i, line) in diagram.iter().enumerate() {
        view.render_row(frame, start_row + i as u16, line.clone());
    }
}

fn get_animal_cell_art(selected: usize, colors: &ThemeColors) -> Vec<Vec<Span<'static>>> {
    let nucleus_color = if selected == 0 {
        colors.yellow()
    } else {
        colors.cyan()
    };
    let mito_color = if selected == 1 {
        colors.yellow()
    } else {
        colors.green()
    };
    let er_color = if selected == 2 {
        colors.yellow()
    } else {
        colors.blue()
    };
    let ribo_color = if selected == 3 {
        colors.yellow()
    } else {
        colors.red()
    };
    let golgi_color = if selected == 4 {
        colors.yellow()
    } else {
        colors.cyan()
    };
    let membrane_color = if selected == 5 {
        colors.yellow()
    } else {
        colors.fg()
    };

    vec![
        vec![Span::styled(
            "              ╭────────────────────────────────────╮              ",
            Style::default().fg(membrane_color),
        )],
        vec![Span::styled(
            "             ╱                                      ╲             ",
            Style::default().fg(membrane_color),
        )],
        vec![
            Span::styled("            │    ", Style::default().fg(membrane_color)),
            Span::styled("╭──────╮", Style::default().fg(nucleus_color)),
            Span::styled(
                "                              │",
                Style::default().fg(membrane_color),
            ),
        ],
        vec![
            Span::styled("            │   ", Style::default().fg(membrane_color)),
            Span::styled("╱ ●●●●● ╲", Style::default().fg(nucleus_color)),
            Span::styled(
                "   ◄── NUCLEUS               │",
                Style::default().fg(colors.grey()),
            ),
        ],
        vec![
            Span::styled("            │  ", Style::default().fg(membrane_color)),
            Span::styled("│ ●●●●●●● │", Style::default().fg(nucleus_color)),
            Span::styled(
                "                             │",
                Style::default().fg(membrane_color),
            ),
        ],
        vec![
            Span::styled("            │   ", Style::default().fg(membrane_color)),
            Span::styled("╲ ●●●●● ╱", Style::default().fg(nucleus_color)),
            Span::styled("   ", Style::default()),
            Span::styled("∿∿∿", Style::default().fg(er_color)),
            Span::styled(
                " ◄── ER                  │",
                Style::default().fg(colors.grey()),
            ),
        ],
        vec![
            Span::styled("            │    ", Style::default().fg(membrane_color)),
            Span::styled("╰──────╯", Style::default().fg(nucleus_color)),
            Span::styled("    ", Style::default()),
            Span::styled("∿∿", Style::default().fg(er_color)),
            Span::styled(
                "                       │",
                Style::default().fg(membrane_color),
            ),
        ],
        vec![Span::styled(
            "            │                                        │            ",
            Style::default().fg(membrane_color),
        )],
        vec![
            Span::styled("            │  ", Style::default().fg(membrane_color)),
            Span::styled("⬡ ⬡ ⬡", Style::default().fg(mito_color)),
            Span::styled(
                " ◄── MITOCHONDRIA           ",
                Style::default().fg(colors.grey()),
            ),
            Span::styled("│", Style::default().fg(membrane_color)),
        ],
        vec![
            Span::styled(
                "            │                ",
                Style::default().fg(membrane_color),
            ),
            Span::styled("○○○", Style::default().fg(ribo_color)),
            Span::styled(" ◄── RIBOSOMES      ", Style::default().fg(colors.grey())),
            Span::styled("│", Style::default().fg(membrane_color)),
        ],
        vec![
            Span::styled("            │  ", Style::default().fg(membrane_color)),
            Span::styled("═══", Style::default().fg(golgi_color)),
            Span::styled(
                " ◄── GOLGI                         │",
                Style::default().fg(colors.grey()),
            ),
        ],
        vec![Span::styled(
            "             ╲                                      ╱             ",
            Style::default().fg(membrane_color),
        )],
        vec![Span::styled(
            "              ╰────────────────────────────────────╯              ",
            Style::default().fg(membrane_color),
        )],
    ]
}

fn get_plant_cell_art(selected: usize, colors: &ThemeColors) -> Vec<Vec<Span<'static>>> {
    let wall_color = if selected == 0 {
        colors.yellow()
    } else {
        colors.green()
    };
    let chloro_color = if selected == 1 {
        colors.yellow()
    } else {
        colors.green()
    };
    let vacuole_color = if selected == 2 {
        colors.yellow()
    } else {
        colors.cyan()
    };
    let nucleus_color = if selected == 3 {
        colors.yellow()
    } else {
        colors.red()
    };
    let mito_color = if selected == 4 {
        colors.yellow()
    } else {
        colors.blue()
    };

    vec![
        vec![Span::styled(
            "            ╔══════════════════════════════════════════╗          ",
            Style::default().fg(wall_color),
        )],
        vec![Span::styled(
            "            ║  CELL WALL                               ║          ",
            Style::default().fg(wall_color),
        )],
        vec![
            Span::styled("            ║  ", Style::default().fg(wall_color)),
            Span::styled(
                "┌────────────────────────────────────┐",
                Style::default().fg(colors.fg()),
            ),
            Span::styled("  ║", Style::default().fg(wall_color)),
        ],
        vec![
            Span::styled("            ║  │ ", Style::default().fg(wall_color)),
            Span::styled("╭────╮", Style::default().fg(nucleus_color)),
            Span::styled("   ", Style::default()),
            Span::styled("****", Style::default().fg(chloro_color)),
            Span::styled(
                " CHLOROPLASTS         │  ║",
                Style::default().fg(colors.grey()),
            ),
        ],
        vec![
            Span::styled("            ║  │ ", Style::default().fg(wall_color)),
            Span::styled("│ @@ │", Style::default().fg(nucleus_color)),
            Span::styled("   ", Style::default()),
            Span::styled("****", Style::default().fg(chloro_color)),
            Span::styled(
                "                      │  ║",
                Style::default().fg(wall_color),
            ),
        ],
        vec![
            Span::styled("            ║  │ ", Style::default().fg(wall_color)),
            Span::styled("╰────╯", Style::default().fg(nucleus_color)),
            Span::styled(
                " NUCLEUS                      │  ║",
                Style::default().fg(colors.grey()),
            ),
        ],
        vec![
            Span::styled("            ║  │ ", Style::default().fg(wall_color)),
            Span::styled(
                "╔════════════════════════════╗",
                Style::default().fg(vacuole_color),
            ),
            Span::styled("    │  ║", Style::default().fg(wall_color)),
        ],
        vec![
            Span::styled("            ║  │ ", Style::default().fg(wall_color)),
            Span::styled(
                "║  CENTRAL VACUOLE           ║",
                Style::default().fg(vacuole_color),
            ),
            Span::styled("    │  ║", Style::default().fg(wall_color)),
        ],
        vec![
            Span::styled("            ║  │ ", Style::default().fg(wall_color)),
            Span::styled(
                "║  (water storage)           ║",
                Style::default().fg(vacuole_color),
            ),
            Span::styled("    │  ║", Style::default().fg(wall_color)),
        ],
        vec![
            Span::styled("            ║  │ ", Style::default().fg(wall_color)),
            Span::styled(
                "╚════════════════════════════╝",
                Style::default().fg(vacuole_color),
            ),
            Span::styled("    │  ║", Style::default().fg(wall_color)),
        ],
        vec![
            Span::styled("            ║  │   ", Style::default().fg(wall_color)),
            Span::styled("⬡⬡⬡", Style::default().fg(mito_color)),
            Span::styled(
                " MITOCHONDRIA                   │  ║",
                Style::default().fg(colors.grey()),
            ),
        ],
        vec![
            Span::styled("            ║  ", Style::default().fg(wall_color)),
            Span::styled(
                "└────────────────────────────────────┘",
                Style::default().fg(colors.fg()),
            ),
            Span::styled("  ║", Style::default().fg(wall_color)),
        ],
        vec![Span::styled(
            "            ╚══════════════════════════════════════════╝          ",
            Style::default().fg(wall_color),
        )],
    ]
}

fn get_bacteria_art(selected: usize, colors: &ThemeColors) -> Vec<Vec<Span<'static>>> {
    let wall_color = if selected == 0 {
        colors.yellow()
    } else {
        colors.green()
    };
    let flag_color = if selected == 1 {
        colors.yellow()
    } else {
        colors.cyan()
    };
    let plasmid_color = if selected == 2 {
        colors.yellow()
    } else {
        colors.blue()
    };
    let ribo_color = if selected == 3 {
        colors.yellow()
    } else {
        colors.red()
    };
    let nucleoid_color = if selected == 4 {
        colors.yellow()
    } else {
        colors.fg()
    };

    vec![
        vec![Span::styled(
            "                                                                  ",
            Style::default(),
        )],
        vec![
            Span::styled(
                "                    ╭─────────────────────╮",
                Style::default().fg(wall_color),
            ),
            Span::styled("∿∿∿", Style::default().fg(flag_color)),
        ],
        vec![Span::styled(
            "                   ╱                       ╲                      ",
            Style::default().fg(wall_color),
        )],
        vec![
            Span::styled("                  │  ", Style::default().fg(wall_color)),
            Span::styled("●●●●●●", Style::default().fg(nucleoid_color)),
            Span::styled(
                "  NUCLEOID          │  FLAGELLUM",
                Style::default().fg(colors.grey()),
            ),
        ],
        vec![
            Span::styled("                  │  ", Style::default().fg(wall_color)),
            Span::styled("●●●●●●", Style::default().fg(nucleoid_color)),
            Span::styled("  (DNA region)      │", Style::default().fg(colors.grey())),
        ],
        vec![Span::styled(
            "                  │                         │                    ",
            Style::default().fg(wall_color),
        )],
        vec![
            Span::styled("                  │    ", Style::default().fg(wall_color)),
            Span::styled("○", Style::default().fg(plasmid_color)),
            Span::styled(" PLASMID            │", Style::default().fg(colors.grey())),
        ],
        vec![Span::styled(
            "                  │                         │                    ",
            Style::default().fg(wall_color),
        )],
        vec![
            Span::styled("                  │  ", Style::default().fg(wall_color)),
            Span::styled("···", Style::default().fg(ribo_color)),
            Span::styled(" RIBOSOMES          │", Style::default().fg(colors.grey())),
        ],
        vec![Span::styled(
            "                   ╲                       ╱                      ",
            Style::default().fg(wall_color),
        )],
        vec![Span::styled(
            "                    ╰─────────────────────╯                       ",
            Style::default().fg(wall_color),
        )],
        vec![Span::styled(
            "                       CELL WALL (peptidoglycan)                  ",
            Style::default().fg(colors.grey()),
        )],
        vec![Span::styled(
            "                                                                  ",
            Style::default(),
        )],
    ]
}

fn get_blood_cell_art(selected: usize, colors: &ThemeColors) -> Vec<Vec<Span<'static>>> {
    let rbc_color = if selected == 0 {
        colors.yellow()
    } else {
        colors.red()
    };
    let wbc_color = if selected == 1 {
        colors.yellow()
    } else {
        colors.fg()
    };
    let platelet_color = if selected == 2 {
        colors.yellow()
    } else {
        colors.cyan()
    };
    let plasma_color = if selected == 3 {
        colors.yellow()
    } else {
        colors.blue()
    };

    vec![
        vec![Span::styled(
            "    ──────────────────────────────────────────────────────────    ",
            Style::default().fg(plasma_color),
        )],
        vec![
            Span::styled("    ~~~ ", Style::default().fg(plasma_color)),
            Span::styled("PLASMA", Style::default().fg(colors.grey())),
            Span::styled(
                " ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ ~~~    ",
                Style::default().fg(plasma_color),
            ),
        ],
        vec![Span::styled(
            "    ───────────────────────────────────────────────────────────   ",
            Style::default().fg(plasma_color),
        )],
        vec![
            Span::styled("           ", Style::default()),
            Span::styled("◯", Style::default().fg(rbc_color)),
            Span::styled("      ", Style::default()),
            Span::styled("◯", Style::default().fg(rbc_color)),
            Span::styled("        ", Style::default()),
            Span::styled("◯", Style::default().fg(rbc_color)),
            Span::styled("        ", Style::default()),
            Span::styled("◯", Style::default().fg(rbc_color)),
        ],
        vec![
            Span::styled("        ", Style::default()),
            Span::styled("◯", Style::default().fg(rbc_color)),
            Span::styled("   ", Style::default()),
            Span::styled("RED BLOOD CELLS", Style::default().fg(colors.grey())),
            Span::styled("         ", Style::default()),
            Span::styled("◯", Style::default().fg(rbc_color)),
        ],
        vec![
            Span::styled("           ", Style::default()),
            Span::styled("◯", Style::default().fg(rbc_color)),
            Span::styled("        ", Style::default()),
            Span::styled("◯", Style::default().fg(rbc_color)),
            Span::styled("      ", Style::default()),
            Span::styled("◯", Style::default().fg(rbc_color)),
            Span::styled("       ", Style::default()),
            Span::styled("◯", Style::default().fg(rbc_color)),
        ],
        vec![Span::styled(
            "                                                                  ",
            Style::default(),
        )],
        vec![
            Span::styled("                  ", Style::default()),
            Span::styled("(@)", Style::default().fg(wbc_color)),
            Span::styled(" WHITE BLOOD CELL", Style::default().fg(colors.grey())),
        ],
        vec![Span::styled(
            "                                                                  ",
            Style::default(),
        )],
        vec![
            Span::styled("               ", Style::default()),
            Span::styled("• •", Style::default().fg(platelet_color)),
            Span::styled("   ", Style::default()),
            Span::styled("•", Style::default().fg(platelet_color)),
            Span::styled("  ", Style::default()),
            Span::styled("PLATELETS", Style::default().fg(colors.grey())),
            Span::styled("   ", Style::default()),
            Span::styled("• •", Style::default().fg(platelet_color)),
        ],
        vec![Span::styled(
            "    ───────────────────────────────────────────────────────────   ",
            Style::default().fg(plasma_color),
        )],
        vec![Span::styled(
            "    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~   ",
            Style::default().fg(plasma_color),
        )],
        vec![Span::styled(
            "    ───────────────────────────────────────────────────────────   ",
            Style::default().fg(plasma_color),
        )],
    ]
}

fn get_neuron_art(selected: usize, colors: &ThemeColors) -> Vec<Vec<Span<'static>>> {
    let soma_color = if selected == 0 {
        colors.yellow()
    } else {
        colors.cyan()
    };
    let dendrite_color = if selected == 1 {
        colors.yellow()
    } else {
        colors.green()
    };
    let axon_color = if selected == 2 {
        colors.yellow()
    } else {
        colors.fg()
    };
    let myelin_color = if selected == 3 {
        colors.yellow()
    } else {
        colors.blue()
    };
    let synapse_color = if selected == 4 {
        colors.yellow()
    } else {
        colors.red()
    };

    vec![
        vec![
            Span::styled("      ", Style::default()),
            Span::styled("╱╲", Style::default().fg(dendrite_color)),
            Span::styled("  DENDRITES", Style::default().fg(colors.grey())),
        ],
        vec![
            Span::styled("     ", Style::default()),
            Span::styled("╱  ╲", Style::default().fg(dendrite_color)),
        ],
        vec![
            Span::styled("    ", Style::default()),
            Span::styled("╱    ╲", Style::default().fg(dendrite_color)),
        ],
        vec![
            Span::styled("   ", Style::default()),
            Span::styled("╱  ╭──╮  ╲", Style::default().fg(dendrite_color)),
        ],
        vec![
            Span::styled("  ", Style::default()),
            Span::styled("╱", Style::default().fg(dendrite_color)),
            Span::styled("  ╱", Style::default().fg(soma_color)),
            Span::styled(" @@ ", Style::default().fg(soma_color)),
            Span::styled("╲", Style::default().fg(soma_color)),
            Span::styled("  ╲", Style::default().fg(dendrite_color)),
            Span::styled("    CELL BODY (SOMA)", Style::default().fg(colors.grey())),
        ],
        vec![
            Span::styled("     ", Style::default()),
            Span::styled("╲  ╰──╯  ╱", Style::default().fg(soma_color)),
        ],
        vec![
            Span::styled("        ", Style::default()),
            Span::styled("│", Style::default().fg(axon_color)),
        ],
        vec![
            Span::styled("      ", Style::default()),
            Span::styled("══╪══", Style::default().fg(myelin_color)),
            Span::styled("     MYELIN SHEATH", Style::default().fg(colors.grey())),
        ],
        vec![
            Span::styled("        ", Style::default()),
            Span::styled("│", Style::default().fg(axon_color)),
            Span::styled("       AXON", Style::default().fg(colors.grey())),
        ],
        vec![
            Span::styled("      ", Style::default()),
            Span::styled("══╪══", Style::default().fg(myelin_color)),
        ],
        vec![
            Span::styled("        ", Style::default()),
            Span::styled("│", Style::default().fg(axon_color)),
        ],
        vec![
            Span::styled("       ", Style::default()),
            Span::styled("╱│╲", Style::default().fg(synapse_color)),
            Span::styled("    SYNAPTIC TERMINALS", Style::default().fg(colors.grey())),
        ],
        vec![
            Span::styled("      ", Style::default()),
            Span::styled("● ● ●", Style::default().fg(synapse_color)),
        ],
    ]
}

fn get_muscle_art(selected: usize, colors: &ThemeColors) -> Vec<Vec<Span<'static>>> {
    let fiber_color = if selected == 0 {
        colors.yellow()
    } else {
        colors.red()
    };
    let sarco_color = if selected == 1 {
        colors.yellow()
    } else {
        colors.cyan()
    };
    let striation_color = if selected == 2 {
        colors.yellow()
    } else {
        colors.fg()
    };
    let nuclei_color = if selected == 3 {
        colors.yellow()
    } else {
        colors.green()
    };

    vec![
        vec![Span::styled(
            "    MUSCLE FIBERS (Skeletal Muscle)                               ",
            Style::default().fg(colors.grey()),
        )],
        vec![Span::styled(
            "                                                                  ",
            Style::default(),
        )],
        vec![
            Span::styled("    ", Style::default()),
            Span::styled("║", Style::default().fg(fiber_color)),
            Span::styled("│", Style::default().fg(striation_color)),
            Span::styled("║", Style::default().fg(fiber_color)),
            Span::styled("│", Style::default().fg(striation_color)),
            Span::styled("║", Style::default().fg(fiber_color)),
            Span::styled("│", Style::default().fg(striation_color)),
            Span::styled("║", Style::default().fg(fiber_color)),
            Span::styled("│", Style::default().fg(striation_color)),
            Span::styled("║", Style::default().fg(fiber_color)),
            Span::styled("│", Style::default().fg(striation_color)),
            Span::styled("║", Style::default().fg(fiber_color)),
            Span::styled("│", Style::default().fg(striation_color)),
            Span::styled("║", Style::default().fg(fiber_color)),
            Span::styled(
                "   STRIATIONS (light/dark bands)",
                Style::default().fg(colors.grey()),
            ),
        ],
        vec![
            Span::styled("    ", Style::default()),
            Span::styled("║", Style::default().fg(fiber_color)),
            Span::styled("│", Style::default().fg(striation_color)),
            Span::styled("║", Style::default().fg(fiber_color)),
            Span::styled("│", Style::default().fg(striation_color)),
            Span::styled("║", Style::default().fg(fiber_color)),
            Span::styled("│", Style::default().fg(striation_color)),
            Span::styled("║", Style::default().fg(fiber_color)),
            Span::styled("│", Style::default().fg(striation_color)),
            Span::styled("║", Style::default().fg(fiber_color)),
            Span::styled("│", Style::default().fg(striation_color)),
            Span::styled("║", Style::default().fg(fiber_color)),
            Span::styled("│", Style::default().fg(striation_color)),
            Span::styled("║", Style::default().fg(fiber_color)),
        ],
        vec![
            Span::styled("    ", Style::default()),
            Span::styled("@", Style::default().fg(nuclei_color)),
            Span::styled("│║│║│║│║│║│║│", Style::default().fg(fiber_color)),
            Span::styled(
                "   @ = NUCLEI (at edge)",
                Style::default().fg(colors.grey()),
            ),
        ],
        vec![
            Span::styled("    ", Style::default()),
            Span::styled("║│║│║│║│║│║│║│║", Style::default().fg(fiber_color)),
        ],
        vec![Span::styled(
            "    ──────────────────────────────────────────────────────────    ",
            Style::default().fg(colors.grey()),
        )],
        vec![Span::styled(
            "                                                                  ",
            Style::default(),
        )],
        vec![
            Span::styled("    ", Style::default()),
            Span::styled("├──┤├──┤├──┤├──┤", Style::default().fg(sarco_color)),
            Span::styled(
                "    SARCOMERES (contractile units)",
                Style::default().fg(colors.grey()),
            ),
        ],
        vec![Span::styled(
            "                                                                  ",
            Style::default(),
        )],
        vec![Span::styled(
            "    When muscle contracts, sarcomeres shorten                     ",
            Style::default().fg(colors.grey()),
        )],
        vec![Span::styled(
            "                                                                  ",
            Style::default(),
        )],
        vec![Span::styled(
            "                                                                  ",
            Style::default(),
        )],
    ]
}

fn draw_organelle_item(
    frame: &mut Frame,
    view: &FullScreenView,
    row: u16,
    org: &Organelle,
    selected: bool,
    colors: &ThemeColors,
) {
    let style = if selected {
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.fg())
    };

    let prefix = if selected { "►" } else { " " };

    view.render_row(
        frame,
        row,
        vec![
            Span::styled(format!(" {} ", prefix), style),
            Span::styled(
                format!("[{}] ", org.symbol),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(format!("{}: ", org.name), style),
            Span::styled(org.description, Style::default().fg(colors.grey())),
        ],
    );
}

// =============================================================================
// DNA LAB
// =============================================================================

fn draw_dna_lab(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BiolabState,
    colors: &ThemeColors,
) {
    let mut row = 0u16;

    // Header with mode tabs
    let helix_style = if state.dna_mode == DnaLabMode::Helix {
        Style::default()
            .fg(colors.yellow())
            .bg(colors.blue())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.grey())
    };
    let build_style = if state.dna_mode == DnaLabMode::Build {
        Style::default()
            .fg(colors.yellow())
            .bg(colors.blue())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.grey())
    };
    let trans_style = if state.dna_mode == DnaLabMode::Transcription {
        Style::default()
            .fg(colors.yellow())
            .bg(colors.blue())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.grey())
    };

    view.render_row(
        frame,
        row,
        vec![
            Span::styled(" 🧬 DNA LAB  ", Style::default().fg(colors.green())),
            Span::styled(" [Helix] ", helix_style),
            Span::styled(" [Build] ", build_style),
            Span::styled(" [Transcribe] ", trans_style),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "────────────────────────────────────────────────────────────────────────────",
            Style::default().fg(colors.blue()),
        )],
    );
    row += 2;

    match state.dna_mode {
        DnaLabMode::Helix => draw_dna_helix(frame, view, row, colors),
        DnaLabMode::Build => draw_dna_build(frame, view, row, state, colors),
        DnaLabMode::Transcription => draw_dna_transcription(frame, view, row, state, colors),
    }

    view.render_help(
        frame,
        vec![
            ("Tab", "mode"),
            ("A/T/G/C", "edit"),
            ("Q", "quiz"),
            ("Esc", "back"),
        ],
    );
}

fn draw_dna_helix(frame: &mut Frame, view: &FullScreenView, start: u16, colors: &ThemeColors) {
    let helix_art = [
        ("        A", "════", "T"),
        ("       ╱ ", "    ", " ╲"),
        ("      T", "════════", "A"),
        ("       ╲ ", "    ", " ╱"),
        ("        G", "══════", "C"),
        ("       ╱ ", "    ", " ╲"),
        ("      C", "════════", "G"),
        ("       ╲ ", "    ", " ╱"),
        ("        A", "══════", "T"),
        ("       ╱ ", "    ", " ╲"),
        ("      T", "════════", "A"),
    ];

    let colors_list = [colors.red(), colors.cyan(), colors.green(), colors.yellow()];

    for (i, (left, middle, right)) in helix_art.iter().enumerate() {
        let color = colors_list[i % 4];
        view.render_row(
            frame,
            start + i as u16,
            vec![
                Span::styled(*left, Style::default().fg(color)),
                Span::styled(*middle, Style::default().fg(colors.grey())),
                Span::styled(*right, Style::default().fg(color)),
            ],
        );
    }

    view.render_row(
        frame,
        start + 13,
        vec![Span::styled(
            "  THE DOUBLE HELIX - A pairs with T, G pairs with C",
            Style::default().fg(colors.grey()),
        )],
    );
}

fn draw_dna_build(
    frame: &mut Frame,
    view: &FullScreenView,
    start: u16,
    state: &BiolabState,
    colors: &ThemeColors,
) {
    view.render_row(
        frame,
        start,
        vec![Span::styled(
            "Build your DNA sequence! Use A, T, G, C keys:",
            Style::default().fg(colors.cyan()),
        )],
    );

    let mut row = start + 2;

    // Show sequence with cursor
    let mut spans = vec![Span::styled("  DNA: ", Style::default().fg(colors.grey()))];

    for (i, base) in state.dna_sequence.iter().enumerate() {
        let is_cursor = i == state.dna_cursor;
        let base_color = match base {
            super::super::biolab::DnaBase::Adenine => colors.red(),
            super::super::biolab::DnaBase::Thymine => colors.cyan(),
            super::super::biolab::DnaBase::Guanine => colors.green(),
            super::super::biolab::DnaBase::Cytosine => colors.yellow(),
        };

        let style = if is_cursor {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.blue())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(base_color)
        };

        spans.push(Span::styled(format!(" {} ", base.char()), style));
    }
    view.render_row(frame, row, spans);
    row += 2;

    // Show complementary strand
    let mut comp_spans = vec![Span::styled(" Comp: ", Style::default().fg(colors.grey()))];
    for base in &state.dna_sequence {
        let comp = base.complement();
        let comp_color = match comp {
            super::super::biolab::DnaBase::Adenine => colors.red(),
            super::super::biolab::DnaBase::Thymine => colors.cyan(),
            super::super::biolab::DnaBase::Guanine => colors.green(),
            super::super::biolab::DnaBase::Cytosine => colors.yellow(),
        };
        comp_spans.push(Span::styled(
            format!(" {} ", comp.char()),
            Style::default().fg(comp_color),
        ));
    }
    view.render_row(frame, row, comp_spans);
    row += 3;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Base Pairing Rules: A-T (2 bonds), G-C (3 bonds)",
            Style::default().fg(colors.grey()),
        )],
    );
}

fn draw_dna_transcription(
    frame: &mut Frame,
    view: &FullScreenView,
    start: u16,
    state: &BiolabState,
    colors: &ThemeColors,
) {
    view.render_row(
        frame,
        start,
        vec![Span::styled(
            "TRANSCRIPTION: DNA → mRNA",
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD),
        )],
    );

    let mut row = start + 2;

    // DNA strand
    let mut dna_spans = vec![Span::styled("  DNA:  ", Style::default().fg(colors.grey()))];
    for base in &state.dna_sequence {
        dna_spans.push(Span::styled(
            format!(" {} ", base.char()),
            Style::default().fg(colors.cyan()),
        ));
    }
    view.render_row(frame, row, dna_spans);
    row += 1;

    // Arrows
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "         ↓   ↓   ↓   ↓   ↓   ↓   ↓",
            Style::default().fg(colors.grey()),
        )],
    );
    row += 1;

    // mRNA strand
    let mut mrna_spans = vec![Span::styled("  mRNA: ", Style::default().fg(colors.grey()))];
    for base in &state.dna_sequence {
        mrna_spans.push(Span::styled(
            format!(" {} ", base.rna_complement()),
            Style::default().fg(colors.red()),
        ));
    }
    view.render_row(frame, row, mrna_spans);
    row += 3;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Note: In RNA, Uracil (U) replaces Thymine (T)",
            Style::default().fg(colors.grey()),
        )],
    );
}

// =============================================================================
// DISSECTION LAB
// =============================================================================

fn draw_dissection(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BiolabState,
    colors: &ThemeColors,
) {
    let specimen = state.current_specimen();
    let mut row = 0u16;

    // Header
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(" 🔬 DISSECTION  ", Style::default().fg(colors.red())),
            Span::styled(
                format!(
                    "Specimen {}/{}: ",
                    state.selected_specimen + 1,
                    SPECIMENS.len()
                ),
                Style::default().fg(colors.grey()),
            ),
            Span::styled(
                specimen.name,
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "────────────────────────────────────────────────────────────────────────────",
            Style::default().fg(colors.red()),
        )],
    );
    row += 1;

    // Description
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            specimen.description,
            Style::default().fg(colors.grey()),
        )],
    );
    row += 2;

    // Parts list
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Anatomy:",
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    for (i, part) in specimen.parts.iter().enumerate() {
        let selected = i == state.selected_part;
        draw_body_part_item(frame, view, row, part, selected, colors);
        row += 2;
    }

    view.render_help(
        frame,
        vec![
            ("←→", "specimen"),
            ("↑↓", "select"),
            ("Q", "quiz"),
            ("Esc", "back"),
        ],
    );
}

fn draw_body_part_item(
    frame: &mut Frame,
    view: &FullScreenView,
    row: u16,
    part: &BodyPart,
    selected: bool,
    colors: &ThemeColors,
) {
    let style = if selected {
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.fg())
    };

    let prefix = if selected { "►" } else { " " };

    view.render_row(
        frame,
        row,
        vec![
            Span::styled(format!(" {} ", prefix), style),
            Span::styled(format!("{}: ", part.name), style),
            Span::styled(part.description, Style::default().fg(colors.grey())),
        ],
    );

    if selected {
        view.render_row(
            frame,
            row + 1,
            vec![
                Span::styled("      Function: ", Style::default().fg(colors.cyan())),
                Span::styled(part.function, Style::default().fg(colors.green())),
            ],
        );
    }
}

// =============================================================================
// ANATOMY LAB
// =============================================================================

fn draw_anatomy(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BiolabState,
    colors: &ThemeColors,
) {
    let system = state.current_system();
    let mut row = 0u16;

    // Header
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(" 🧠 ANATOMY  ", Style::default().fg(colors.yellow())),
            Span::styled(
                format!(
                    "System {}/{}: ",
                    state.selected_system + 1,
                    BODY_SYSTEMS.len()
                ),
                Style::default().fg(colors.grey()),
            ),
            Span::styled(
                system.name,
                Style::default()
                    .fg(colors.cyan())
                    .add_modifier(Modifier::BOLD),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "────────────────────────────────────────────────────────────────────────────",
            Style::default().fg(colors.yellow()),
        )],
    );
    row += 1;

    // Description
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            system.description,
            Style::default().fg(colors.grey()),
        )],
    );
    row += 2;

    // Parts
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Components:",
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    for (i, part) in system.parts.iter().enumerate() {
        let selected = i == state.selected_part;
        let style = if selected {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg())
        };
        let prefix = if selected { "►" } else { " " };

        view.render_row(
            frame,
            row,
            vec![
                Span::styled(format!(" {} ", prefix), style),
                Span::styled(format!("{}: ", part.name), style),
                Span::styled(part.description, Style::default().fg(colors.grey())),
            ],
        );
        row += 1;
    }

    view.render_help(
        frame,
        vec![
            ("←→", "system"),
            ("↑↓", "select"),
            ("Q", "quiz"),
            ("Esc", "back"),
        ],
    );
}

// =============================================================================
// QUIZ VIEWS
// =============================================================================

fn draw_quiz(frame: &mut Frame, view: &FullScreenView, state: &BiolabState, colors: &ThemeColors) {
    let mut row = 0u16;

    // Header
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(" 📝 QUIZ: ", Style::default().fg(colors.cyan())),
            Span::styled(
                state.quiz_topic.name(),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "   Question {}/{}",
                    state.current_question + 1,
                    state.quiz_questions.len()
                ),
                Style::default().fg(colors.grey()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "────────────────────────────────────────────────────────────────────────────",
            Style::default().fg(colors.cyan()),
        )],
    );
    row += 2;

    if let Some(q) = state.quiz_questions.get(state.current_question) {
        // Question
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                &q.question,
                Style::default()
                    .fg(colors.fg())
                    .add_modifier(Modifier::BOLD),
            )],
        );
        row += 3;

        // Options
        for (i, option) in q.options.iter().enumerate() {
            let selected = i == state.selected_answer;
            let letter = (b'A' + i as u8) as char;

            let style = if selected {
                Style::default()
                    .fg(colors.yellow())
                    .bg(colors.blue())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg())
            };

            let prefix = if selected { "►" } else { " " };

            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("  {} {}) {}", prefix, letter, option),
                    style,
                )],
            );
            row += 2;
        }
    }

    // Score
    row += 1;
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("  Score: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}/{}", state.quiz_correct, state.current_question),
                Style::default().fg(colors.green()),
            ),
        ],
    );

    view.render_help(
        frame,
        vec![("↑↓/A-D", "select"), ("Enter", "answer"), ("Esc", "quit")],
    );
}

fn draw_feedback(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BiolabState,
    colors: &ThemeColors,
) {
    let mut row = 0u16;

    if let Some(q) = state.quiz_questions.get(state.current_question) {
        let correct = state.selected_answer == q.correct_index;

        // Result
        if correct {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    "   ✓ CORRECT!",
                    Style::default()
                        .fg(colors.green())
                        .add_modifier(Modifier::BOLD),
                )],
            );
        } else {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    "   ✗ INCORRECT",
                    Style::default()
                        .fg(colors.red())
                        .add_modifier(Modifier::BOLD),
                )],
            );
        }
        row += 2;

        // Question
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                &q.question,
                Style::default().fg(colors.grey()),
            )],
        );
        row += 2;

        // Correct answer
        let correct_letter = (b'A' + q.correct_index as u8) as char;
        view.render_row(
            frame,
            row,
            vec![
                Span::styled("   Correct Answer: ", Style::default().fg(colors.cyan())),
                Span::styled(
                    format!("{}) {}", correct_letter, q.options[q.correct_index]),
                    Style::default()
                        .fg(colors.green())
                        .add_modifier(Modifier::BOLD),
                ),
            ],
        );
        row += 2;

        // Explanation (wrapped to fit terminal width)
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "   Explanation:",
                Style::default().fg(colors.yellow()),
            )],
        );
        row += 1;

        // Wrap explanation text to ~70 chars (terminal width minus padding)
        let explanation_lines = wrap_text(&q.explanation, 70);
        for line in explanation_lines {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("   {}", line),
                    Style::default().fg(colors.fg()),
                )],
            );
            row += 1;
        }
    }

    view.render_help(frame, vec![("Enter/Space", "continue"), ("Esc", "quit")]);
}

fn draw_results(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BiolabState,
    colors: &ThemeColors,
) {
    let mut row = 2u16;

    let total = state.quiz_questions.len() as u32;
    let percent = if total > 0 {
        (state.quiz_correct as f32 / total as f32 * 100.0) as u32
    } else {
        0
    };

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╔══════════════════════════════════════════════════════════════╗",
            Style::default().fg(colors.cyan()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "║                      QUIZ COMPLETE!                          ║",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╠══════════════════════════════════════════════════════════════╣",
            Style::default().fg(colors.cyan()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║              Score: {}/{}  ({}%)                          ║",
                state.quiz_correct, total, percent
            ),
            Style::default().fg(colors.fg()),
        )],
    );
    row += 1;

    let grade = if percent >= 90 {
        ("A", colors.green())
    } else if percent >= 80 {
        ("B", colors.cyan())
    } else if percent >= 70 {
        ("C", colors.yellow())
    } else if percent >= 60 {
        ("D", colors.red())
    } else {
        ("F", colors.red())
    };

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("║              Grade: ", Style::default().fg(colors.fg())),
            Span::styled(
                grade.0,
                Style::default().fg(grade.1).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "                                       ║",
                Style::default().fg(colors.fg()),
            ),
        ],
    );
    row += 1;

    if percent >= 80 {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "║              ★ TOPIC MASTERED! ★                           ║",
                Style::default()
                    .fg(colors.green())
                    .add_modifier(Modifier::BOLD),
            )],
        );
        row += 1;
    }

    if percent == 100 {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "║              ★★ PERFECT SCORE! +50 BONUS ★★               ║",
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            )],
        );
        row += 1;
    }

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╚══════════════════════════════════════════════════════════════╝",
            Style::default().fg(colors.cyan()),
        )],
    );

    view.render_help(
        frame,
        vec![("Enter", "continue"), ("R", "retry"), ("Esc", "menu")],
    );
}

// =============================================================================
// PROGRESS & OTHER VIEWS
// =============================================================================

fn draw_progress(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BiolabState,
    colors: &ThemeColors,
) {
    let mut row = 0u16;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╔══════════════════════════════════════════════════════════════════════════╗",
            Style::default().fg(colors.cyan()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "║                          YOUR PROGRESS                                   ║",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╠══════════════════════════════════════════════════════════════════════════╣",
            Style::default().fg(colors.cyan()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Total Score: {:>6}                                                    ║",
                state.total_score
            ),
            Style::default().fg(colors.green()),
        )],
    );
    row += 1;

    let slides_done = state.slides_viewed.iter().filter(|&&v| v).count();
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Microscope Slides: {}/{}                                                 ║",
                slides_done,
                SLIDES.len()
            ),
            Style::default().fg(colors.cyan()),
        )],
    );
    row += 1;

    let specimens_done = state.specimens_viewed.iter().filter(|&&v| v).count();
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Dissection Specimens: {}/{}                                               ║",
                specimens_done,
                SPECIMENS.len()
            ),
            Style::default().fg(colors.red()),
        )],
    );
    row += 1;

    let systems_done = state.systems_viewed.iter().filter(|&&v| v).count();
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Anatomy Systems: {}/{}                                                    ║",
                systems_done,
                BODY_SYSTEMS.len()
            ),
            Style::default().fg(colors.yellow()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Quizzes Taken: {}  │  Perfect Quizzes: {}                                 ║",
                state.quizzes_taken, state.perfect_quizzes
            ),
            Style::default().fg(colors.fg()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Topics Mastered: {}                                                       ║",
                state.topics_mastered.len()
            ),
            Style::default().fg(colors.green()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╚══════════════════════════════════════════════════════════════════════════╝",
            Style::default().fg(colors.cyan()),
        )],
    );

    view.render_help(frame, vec![("Enter/Esc", "back")]);
}

fn draw_loading(
    frame: &mut Frame,
    view: &FullScreenView,
    _state: &BiolabState,
    colors: &ThemeColors,
) {
    let mut row = 8u16;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "          ╔════════════════════════════════════════════╗",
            Style::default().fg(colors.cyan()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "          ║        Generating quiz questions...        ║",
            Style::default().fg(colors.yellow()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "          ║                                            ║",
            Style::default().fg(colors.cyan()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "          ║              🧬  ●  ●  ●  🔬              ║",
            Style::default().fg(colors.green()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "          ╚════════════════════════════════════════════╝",
            Style::default().fg(colors.cyan()),
        )],
    );
}

fn draw_error(frame: &mut Frame, view: &FullScreenView, state: &BiolabState, colors: &ThemeColors) {
    let mut row = 8u16;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "          ╔════════════════════════════════════════════╗",
            Style::default().fg(colors.red()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "          ║                   ERROR                    ║",
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "          ╠════════════════════════════════════════════╣",
            Style::default().fg(colors.red()),
        )],
    );
    row += 1;

    if let Some(msg) = &state.error_message {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("          ║  {}  ║", msg),
                Style::default().fg(colors.yellow()),
            )],
        );
        row += 1;
    }

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "          ╚════════════════════════════════════════════╝",
            Style::default().fg(colors.red()),
        )],
    );

    view.render_help(frame, vec![("Enter/Esc", "continue")]);
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Helper function to wrap text to a maximum width.
///
/// Splits text on whitespace and wraps lines that exceed the maximum width.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}
