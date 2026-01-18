//! Brainiac game modal rendering
//!
//! This module handles the visual rendering of the Brainiac AI-powered trivia game
//! within the games plugin modal. It displays setup screens, loading animations,
//! questions with answers, feedback, and game over statistics.

use super::super::brainiac::{BrainiacState, BrainiacView};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
    Frame,
};

/// BRAINIAC title art - ANSI block style
const BRAINIAC_TITLE: &[&str] = &[
    "██████╗ ██████╗  █████╗ ██╗███╗   ██╗██╗ █████╗  ██████╗",
    "██╔══██╗██╔══██╗██╔══██╗██║████╗  ██║██║██╔══██╗██╔════╝",
    "██████╔╝██████╔╝███████║██║██╔██╗ ██║██║███████║██║     ",
    "██╔══██╗██╔══██╗██╔══██║██║██║╚██╗██║██║██╔══██║██║     ",
    "██████╔╝██║  ██║██║  ██║██║██║ ╚████║██║██║  ██║╚██████╗",
    "╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝╚═╝╚═╝  ╚═╝ ╚═════╝",
];

/// Brain animation frames
const BRAIN_FRAMES: &[&str] = &["  ,---.  ", " / o o \\ ", " \\  ~  / ", "  '---'  "];

/// Renders the Brainiac game state to the terminal.
///
/// Dispatches to the appropriate drawing function based on the current view state.
pub fn draw_brainiac(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BrainiacState,
    colors: &ThemeColors,
) {
    match state.view {
        BrainiacView::Setup => draw_brainiac_setup(frame, view, state, colors),
        BrainiacView::Loading => draw_brainiac_loading(frame, view, state, colors),
        BrainiacView::Playing => draw_brainiac_playing(frame, view, state, colors),
        BrainiacView::Feedback => draw_brainiac_feedback(frame, view, state, colors),
        BrainiacView::GameOver => draw_brainiac_gameover(frame, view, state, colors),
        BrainiacView::Error => draw_brainiac_error(frame, view, state, colors),
    }
}

/// Renders the Brainiac setup screen.
///
/// Shows title, API status, and options for player age, category, and game mode.
pub fn draw_brainiac_setup(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BrainiacState,
    colors: &ThemeColors,
) {
    let mut row = 0;

    // Title with brain gradient: pink/red at top (cortex), cyan at bottom (tech)
    for (i, line) in BRAINIAC_TITLE.iter().enumerate() {
        let row_color = match i {
            0 | 1 => colors.red(),    // Brain tissue pink/red
            2 | 3 => colors.yellow(), // Neural activity
            _ => colors.cyan(),       // Tech/digital
        };
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("{:^78}", line),
                Style::default().fg(row_color).add_modifier(Modifier::BOLD),
            )],
        );
        row += 1;
    }
    row += 1;

    // Subtitle
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!("{:^78}", "AI-Powered Trivia Game"),
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 2;

    // API status
    let api_status = if state.is_api_available() {
        ("AI Ready", colors.green())
    } else {
        ("No API Key - Using Fallback Questions", colors.red())
    };
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!("{:^78}", api_status.0),
            Style::default().fg(api_status.1),
        )],
    );
    row += 2;

    // Age selection
    let age_style = if state.setup_cursor == 0 {
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.fg())
    };
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                format!("{:>30}", "Player Age: "),
                Style::default().fg(colors.fg()),
            ),
            Span::styled("< ", age_style),
            Span::styled(format!("{:3}", state.player_age), age_style),
            Span::styled(" >", age_style),
        ],
    );
    row += 1;

    // Category selection
    let cat_style = if state.setup_cursor == 1 {
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.fg())
    };
    let cat_name = state
        .selected_category
        .map(|c| c.name())
        .unwrap_or("All Topics (Mixed)");
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                format!("{:>30}", "Topic: "),
                Style::default().fg(colors.fg()),
            ),
            Span::styled("< ", cat_style),
            Span::styled(format!("{:20}", cat_name), cat_style),
            Span::styled(" >", cat_style),
        ],
    );
    row += 1;

    // Mode selection
    let mode_style = if state.setup_cursor == 2 {
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.fg())
    };
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                format!("{:>30}", "Mode: "),
                Style::default().fg(colors.fg()),
            ),
            Span::styled("< ", mode_style),
            Span::styled(format!("{:20}", state.game_mode.name()), mode_style),
            Span::styled(" >", mode_style),
        ],
    );
    row += 2;

    // Start button
    let start_style = if state.setup_cursor == 3 {
        Style::default()
            .fg(colors.yellow())
            .bg(colors.blue())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.green())
    };
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!("{:^78}", "[ START GAME ]"),
            start_style,
        )],
    );

    let help = vec![
        ("\u{2191}\u{2193}", "select"),
        ("\u{2190}\u{2192}", "change"),
        ("Enter", "start"),
        ("Esc", "back"),
    ];
    view.render_help(frame, help);
}

/// Renders the Brainiac loading screen.
///
/// Shows a brain animation and loading dots while questions are being generated.
pub fn draw_brainiac_loading(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BrainiacState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height();
    let center = content_height / 2;

    // Brain animation
    for (i, line) in BRAIN_FRAMES.iter().enumerate() {
        view.render_row(
            frame,
            center - 2 + i as u16,
            vec![Span::styled(
                format!("{:^78}", line),
                Style::default().fg(colors.cyan()),
            )],
        );
    }

    // Loading text with animation
    let dots = ".".repeat((state.brain_frame as usize % 4) + 1);
    view.render_row(
        frame,
        center + 3,
        vec![Span::styled(
            format!("{:^78}", format!("Generating questions{}", dots)),
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );

    let help = vec![("", "Please wait...")];
    view.render_help(frame, help);
}

/// Renders the Brainiac playing screen.
///
/// Shows question number, score, streak, timer, category, question text, and answer options.
pub fn draw_brainiac_playing(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BrainiacState,
    colors: &ThemeColors,
) {
    let mut row = 0;

    // Header: Question X of Y | Score | Streak
    let question_num = state.current_question + 1;
    let total = state.questions.len();
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                format!("  Question {} of {}  ", question_num, total),
                Style::default().fg(colors.fg()),
            ),
            Span::styled("  |  ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("Score: {}  ", state.score),
                Style::default().fg(colors.green()),
            ),
            Span::styled("  |  ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("Streak: {}", state.streak),
                Style::default().fg(if state.streak >= 3 {
                    colors.yellow()
                } else {
                    colors.fg()
                }),
            ),
        ],
    );
    row += 2;

    // Timer bar
    let time_pct =
        (state.time_remaining as f64 / state.game_mode.time_limit() as f64 * 40.0) as usize;
    let timer_filled = "#".repeat(time_pct);
    let timer_empty = "-".repeat(40 - time_pct);
    let timer_color = if state.time_remaining <= 3 {
        colors.red()
    } else if state.time_remaining <= 7 {
        colors.yellow()
    } else {
        colors.green()
    };
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("  Time: [", Style::default().fg(colors.fg())),
            Span::styled(&timer_filled, Style::default().fg(timer_color)),
            Span::styled(&timer_empty, Style::default().fg(colors.grey())),
            Span::styled(
                format!("] {}s", state.time_remaining),
                Style::default().fg(colors.fg()),
            ),
        ],
    );
    row += 2;

    // Question
    if let Some(q) = state.current_question_data() {
        // Category badge
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  {} {}", q.category.icon(), q.category.name()),
                Style::default().fg(colors.cyan()),
            )],
        );
        row += 2;

        // Question text (wrapped)
        let question_lines = wrap_text(&q.question, 70);
        for line in question_lines {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("  {}", line),
                    Style::default()
                        .fg(colors.fg())
                        .add_modifier(Modifier::BOLD),
                )],
            );
            row += 1;
        }
        row += 1;

        // Options
        for (i, option) in q.options.iter().enumerate() {
            let is_selected = i == state.selected_answer;
            let style = if is_selected {
                Style::default()
                    .fg(colors.yellow())
                    .bg(colors.blue())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg())
            };
            let prefix = if is_selected { ">" } else { " " };
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("  {} {}) {}", prefix, (b'A' + i as u8) as char, option),
                    style,
                )],
            );
            row += 1;
        }
    }

    let help = vec![
        ("\u{2191}\u{2193}", "select"),
        ("Enter", "answer"),
        ("1-4", "quick answer"),
        ("Esc", "quit"),
    ];
    view.render_help(frame, help);
}

/// Renders the Brainiac feedback screen.
///
/// Shows whether the answer was correct or wrong, streak bonuses, and fun facts.
pub fn draw_brainiac_feedback(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BrainiacState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height();
    let center = content_height / 2 - 3;

    if state.last_correct {
        // Correct answer feedback
        view.render_row(
            frame,
            center,
            vec![Span::styled(
                format!("{:^78}", "*** CORRECT! ***"),
                Style::default()
                    .fg(colors.green())
                    .add_modifier(Modifier::BOLD),
            )],
        );

        if state.streak >= 3 {
            view.render_row(
                frame,
                center + 1,
                vec![Span::styled(
                    format!(
                        "{:^78}",
                        format!(
                            "STREAK: {} (x{:.1} bonus!)",
                            state.streak,
                            if state.streak >= 5 { 2.0 } else { 1.5 }
                        )
                    ),
                    Style::default().fg(colors.yellow()),
                )],
            );
        }
    } else {
        // Wrong answer feedback
        view.render_row(
            frame,
            center,
            vec![Span::styled(
                format!("{:^78}", "Not quite..."),
                Style::default().fg(colors.red()),
            )],
        );

        if let Some(q) = state.current_question_data() {
            let correct_letter = (b'A' + q.correct_index as u8) as char;
            view.render_row(
                frame,
                center + 2,
                vec![Span::styled(
                    format!(
                        "{:^78}",
                        format!(
                            "The correct answer was: {}) {}",
                            correct_letter, q.options[q.correct_index]
                        )
                    ),
                    Style::default().fg(colors.yellow()),
                )],
            );
        }
    }

    // Fun fact
    if let Some(q) = state.current_question_data() {
        let fun_fact_lines = wrap_text(&q.fun_fact, 60);
        for (i, line) in fun_fact_lines.iter().enumerate() {
            view.render_row(
                frame,
                center + 4 + i as u16,
                vec![Span::styled(
                    format!("{:^78}", line),
                    Style::default().fg(colors.cyan()),
                )],
            );
        }
    }

    let help = vec![("", "Next question...")];
    view.render_help(frame, help);
}

/// Renders the Brainiac game over screen.
///
/// Shows final score, accuracy, best streak, and a rating based on performance.
pub fn draw_brainiac_gameover(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BrainiacState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height();
    let center = content_height / 2 - 5;

    // Title
    view.render_row(
        frame,
        center,
        vec![Span::styled(
            format!("{:^78}", "=== GAME COMPLETE ==="),
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );

    // Final score
    view.render_row(
        frame,
        center + 2,
        vec![Span::styled(
            format!("{:^78}", format!("Final Score: {}", state.final_score())),
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD),
        )],
    );

    // Stats
    view.render_row(
        frame,
        center + 4,
        vec![Span::styled(
            format!(
                "{:^78}",
                format!(
                    "Correct: {}/{}  |  Accuracy: {}%  |  Best Streak: {}",
                    state.correct_count,
                    state.current_question,
                    state.accuracy(),
                    state.best_streak
                )
            ),
            Style::default().fg(colors.fg()),
        )],
    );

    // Rating
    let rating = if state.accuracy() >= 90 {
        "GENIUS!"
    } else if state.accuracy() >= 70 {
        "Excellent!"
    } else if state.accuracy() >= 50 {
        "Good job!"
    } else {
        "Keep practicing!"
    };
    view.render_row(
        frame,
        center + 6,
        vec![Span::styled(
            format!("{:^78}", rating),
            Style::default().fg(colors.cyan()),
        )],
    );

    let help = vec![
        ("Enter", "leaderboard"),
        ("R", "play again"),
        ("Esc", "menu"),
    ];
    view.render_help(frame, help);
}

/// Renders the Brainiac error screen.
///
/// Shows an error message when something goes wrong (e.g., API failure).
pub fn draw_brainiac_error(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BrainiacState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height();
    let center = content_height / 2 - 2;

    view.render_row(
        frame,
        center,
        vec![Span::styled(
            format!("{:^78}", "ERROR"),
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::BOLD),
        )],
    );

    if let Some(msg) = &state.error_message {
        let lines = wrap_text(msg, 60);
        for (i, line) in lines.iter().enumerate() {
            view.render_row(
                frame,
                center + 2 + i as u16,
                vec![Span::styled(
                    format!("{:^78}", line),
                    Style::default().fg(colors.yellow()),
                )],
            );
        }
    }

    let help = vec![("Enter/Esc", "back to setup")];
    view.render_help(frame, help);
}

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

    lines
}
