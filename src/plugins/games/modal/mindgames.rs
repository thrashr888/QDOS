use crate::app::ThemeColors;
use crate::plugins::games::mindgames::{
    DailyQuestionType, MemoryPhase, MindgamesMode, MindgamesState, MindgamesView, PatternPhase,
};
use crate::ui::components::FullScreenView;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
    Frame,
};

pub fn draw_mindgames(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &MindgamesState,
    colors: &ThemeColors,
) {
    match state.view {
        MindgamesView::ModeSelect => draw_mode_select(frame, view, state, colors),
        MindgamesView::Playing => match state.mode {
            Some(MindgamesMode::PatternMaster) => draw_pattern(frame, view, state, colors),
            Some(MindgamesMode::MemoryMatrix) => draw_memory(frame, view, state, colors),
            Some(MindgamesMode::NumberNinja) => draw_number(frame, view, state, colors),
            Some(MindgamesMode::DailyChallenge) => draw_daily(frame, view, state, colors),
            None => {}
        },
        MindgamesView::Feedback => draw_feedback(frame, view, state, colors),
        MindgamesView::GameOver => draw_game_over(frame, view, state, colors),
    }
}

fn draw_mode_select(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &MindgamesState,
    colors: &ThemeColors,
) {
    let mut row = 1;

    // Title
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "═══ MINDGAMES - Brain Training Challenge ═══",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 2;

    // Mode options
    let modes = [
        ("PATTERN MASTER", "Complete the sequence - find patterns"),
        ("MEMORY MATRIX", "Memorize and recall grid patterns"),
        ("NUMBER NINJA", "Lightning-fast mental math"),
        ("DAILY CHALLENGE", "Mixed challenge with today's seed"),
    ];

    for (i, (name, desc)) in modes.iter().enumerate() {
        let selected = i == state.selected_mode;

        if selected {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("► {}", name),
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
                    format!("  {}", name),
                    Style::default().fg(colors.blue()),
                )],
            );
        }
        row += 1;

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("    {}", desc),
                Style::default().fg(colors.grey()),
            )],
        );
        row += 2;
    }

    // Help
    view.render_help(
        frame,
        vec![("↑↓", "select"), ("Enter", "start"), ("Esc", "back")],
    );
}

fn draw_pattern(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &MindgamesState,
    colors: &ThemeColors,
) {
    let mut row = 0;

    // Header
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                format!(
                    "Question {}/{}",
                    state.question_index + 1,
                    state.total_questions
                ),
                Style::default().fg(colors.grey()),
            ),
            Span::styled("  Score: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.score),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Streak: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}^", state.streak),
                Style::default()
                    .fg(colors.red())
                    .add_modifier(Modifier::BOLD),
            ),
        ],
    );
    row += 2;

    match state.pattern_phase {
        PatternPhase::ShowPattern => {
            // Show sequence
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    "Study this pattern:",
                    Style::default()
                        .fg(colors.blue())
                        .add_modifier(Modifier::BOLD),
                )],
            );
            row += 2;

            // Display sequence centered
            let sequence_str = state.pattern_sequence.join("   ");
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    sequence_str,
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD),
                )],
            );
            row += 2;

            // Timer
            let timer_bars = (state.pattern_display_timer / 3).min(10);
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("Time: ", Style::default().fg(colors.grey())),
                    Span::styled(
                        "█".repeat(timer_bars as usize),
                        Style::default().fg(colors.cyan()),
                    ),
                    Span::styled(
                        "░".repeat((10 - timer_bars) as usize),
                        Style::default().fg(colors.grey()),
                    ),
                ],
            );
        }
        PatternPhase::AnswerPrompt => {
            // Sequence with question mark
            let mut display_seq = state.pattern_sequence.clone();
            display_seq.push("?".to_string());
            let sequence_str = display_seq.join("   ");

            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    sequence_str,
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD),
                )],
            );
            row += 3;

            // Choices
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    "What comes next?",
                    Style::default().fg(colors.blue()),
                )],
            );
            row += 2;

            for (i, choice) in state.pattern_choices.iter().enumerate() {
                let selected = i == state.pattern_selected;

                if selected {
                    view.render_row(
                        frame,
                        row,
                        vec![Span::styled(
                            format!("  {} ► {}", i + 1, choice),
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
                            format!("  {}   {}", i + 1, choice),
                            Style::default().fg(colors.blue()),
                        )],
                    );
                }
                row += 1;
            }

            row += 1;

            // Timer
            let timer_bars = (state.time_remaining / 2).min(10);
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("Time: ", Style::default().fg(colors.grey())),
                    Span::styled(
                        "█".repeat(timer_bars as usize),
                        Style::default().fg(if timer_bars > 5 {
                            colors.green()
                        } else {
                            colors.red()
                        }),
                    ),
                    Span::styled(
                        "░".repeat((10 - timer_bars) as usize),
                        Style::default().fg(colors.grey()),
                    ),
                ],
            );
        }
    }

    // Help
    if state.pattern_phase == PatternPhase::AnswerPrompt {
        view.render_help(
            frame,
            vec![
                ("1-4", "quick answer"),
                ("↑↓", "select"),
                ("Enter", "submit"),
            ],
        );
    }
}

fn draw_memory(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &MindgamesState,
    colors: &ThemeColors,
) {
    let mut row = 0;

    // Header
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                format!(
                    "Question {}/{}",
                    state.question_index + 1,
                    state.total_questions
                ),
                Style::default().fg(colors.grey()),
            ),
            Span::styled("  Score: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.score),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
        ],
    );
    row += 2;

    match state.memory_phase {
        MemoryPhase::Memorize => {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    "Memorize this pattern:",
                    Style::default()
                        .fg(colors.blue())
                        .add_modifier(Modifier::BOLD),
                )],
            );
            row += 2;

            // Draw grid
            let (rows, cols) = state.memory_grid_size;
            for r in 0..rows {
                let mut line = vec![Span::raw("    ")];
                for c in 0..cols {
                    let filled = state.memory_filled_cells.contains(&(r, c));
                    if filled {
                        line.push(Span::styled(
                            "█ ",
                            Style::default()
                                .fg(colors.cyan())
                                .add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        line.push(Span::styled("░ ", Style::default().fg(colors.grey())));
                    }
                }
                view.render_row(frame, row, line);
                row += 1;
            }

            row += 1;

            // Timer
            let timer_bars = (state.memory_display_timer / 5).min(10);
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("Time: ", Style::default().fg(colors.grey())),
                    Span::styled(
                        "█".repeat(timer_bars as usize),
                        Style::default().fg(colors.cyan()),
                    ),
                    Span::styled(
                        "░".repeat((10 - timer_bars) as usize),
                        Style::default().fg(colors.grey()),
                    ),
                ],
            );
        }
        MemoryPhase::Recall => {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    "Recreate the pattern:",
                    Style::default()
                        .fg(colors.blue())
                        .add_modifier(Modifier::BOLD),
                )],
            );
            row += 2;

            // Draw grid with cursor
            let (rows, cols) = state.memory_grid_size;
            for r in 0..rows {
                let mut line = vec![Span::raw("    ")];
                for c in 0..cols {
                    let is_cursor = (r, c) == state.memory_cursor;
                    let is_selected = state.memory_player_cells.contains(&(r, c));

                    if is_cursor && is_selected {
                        line.push(Span::styled(
                            "█ ",
                            Style::default()
                                .fg(colors.yellow())
                                .add_modifier(Modifier::BOLD),
                        ));
                    } else if is_cursor {
                        line.push(Span::styled(
                            "□ ",
                            Style::default()
                                .fg(colors.yellow())
                                .add_modifier(Modifier::BOLD),
                        ));
                    } else if is_selected {
                        line.push(Span::styled("█ ", Style::default().fg(colors.cyan())));
                    } else {
                        line.push(Span::styled("░ ", Style::default().fg(colors.grey())));
                    }
                }
                view.render_row(frame, row, line);
                row += 1;
            }
        }
    }

    // Help
    if state.memory_phase == MemoryPhase::Recall {
        view.render_help(
            frame,
            vec![
                ("Arrows/HJKL", "move"),
                ("Space", "toggle"),
                ("Enter", "submit"),
            ],
        );
    }
}

fn draw_number(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &MindgamesState,
    colors: &ThemeColors,
) {
    let mut row = 0;

    // Header
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                format!(
                    "Question {}/{}",
                    state.question_index + 1,
                    state.total_questions
                ),
                Style::default().fg(colors.grey()),
            ),
            Span::styled("  Score: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.score),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Streak: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}^", state.streak),
                Style::default()
                    .fg(colors.red())
                    .add_modifier(Modifier::BOLD),
            ),
        ],
    );
    row += 2;

    // Equation
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            &state.number_equation,
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 3;

    // Choices
    for (i, choice) in state.number_choices.iter().enumerate() {
        let selected = i == state.number_selected;

        if selected {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("  {} ► {}", i + 1, choice),
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
                    format!("  {}   {}", i + 1, choice),
                    Style::default().fg(colors.blue()),
                )],
            );
        }
        row += 1;
    }

    row += 1;

    // Timer
    let timer_bars = (state.time_remaining / 2).min(10);
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Time: ", Style::default().fg(colors.grey())),
            Span::styled(
                "█".repeat(timer_bars as usize),
                Style::default().fg(if timer_bars > 5 {
                    colors.green()
                } else {
                    colors.red()
                }),
            ),
            Span::styled(
                "░".repeat((10 - timer_bars) as usize),
                Style::default().fg(colors.grey()),
            ),
        ],
    );

    // Help
    view.render_help(
        frame,
        vec![
            ("1-4", "quick answer"),
            ("↑↓", "select"),
            ("Enter", "submit"),
        ],
    );
}

fn draw_daily(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &MindgamesState,
    colors: &ThemeColors,
) {
    // Show daily date banner
    let row = 0;
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("DAILY CHALLENGE ", Style::default().fg(colors.yellow())),
            Span::styled(
                &state.daily_date,
                Style::default()
                    .fg(colors.cyan())
                    .add_modifier(Modifier::BOLD),
            ),
        ],
    );

    // Delegate to specific renderer based on question type
    if state.question_index < state.daily_question_types.len() {
        match state.daily_question_types[state.question_index] {
            DailyQuestionType::Pattern => draw_pattern(frame, view, state, colors),
            DailyQuestionType::Memory => draw_memory(frame, view, state, colors),
            DailyQuestionType::Number => draw_number(frame, view, state, colors),
        }
    }
}

fn draw_feedback(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &MindgamesState,
    colors: &ThemeColors,
) {
    let mut row = 3;

    // Correct/Wrong Banner
    if state.last_correct {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "✓ CORRECT!",
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
                "✗ WRONG",
                Style::default()
                    .fg(colors.red())
                    .add_modifier(Modifier::BOLD),
            )],
        );
    }
    row += 3;

    if state.last_correct {
        // Show score gained
        view.render_row(
            frame,
            row,
            vec![
                Span::styled("Score: ", Style::default().fg(colors.grey())),
                Span::styled(
                    format!("+{} points", state.score),
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD),
                ),
            ],
        );
        row += 1;

        // Show streak
        if state.streak > 1 {
            let multiplier_text = if state.streak >= 5 {
                " (2.0× MULTIPLIER!)"
            } else if state.streak >= 3 {
                " (1.5× multiplier)"
            } else {
                ""
            };

            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("Streak: ", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{}^", state.streak),
                        Style::default()
                            .fg(colors.red())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        multiplier_text,
                        Style::default()
                            .fg(colors.yellow())
                            .add_modifier(Modifier::BOLD),
                    ),
                ],
            );
        }
    } else {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "Streak broken!",
                Style::default().fg(colors.grey()),
            )],
        );
    }

    row += 3;

    // Auto-advance countdown
    let countdown = state.feedback_timer / 10;
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!("Next question in {}...", countdown),
            Style::default().fg(colors.grey()),
        )],
    );
}

fn draw_game_over(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &MindgamesState,
    colors: &ThemeColors,
) {
    let mut row = 2;

    // Title
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "═══ GAME COMPLETE ═══",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 3;

    // Final Score
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Final Score: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.final_score()),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
        ],
    );
    row += 2;

    // Accuracy
    let accuracy = state.accuracy();
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Accuracy: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!(
                    "{:.1}% ({}/{})",
                    accuracy, state.correct_count, state.total_questions
                ),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 1;

    // Best Streak
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Best Streak: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}^", state.best_streak),
                Style::default()
                    .fg(colors.red())
                    .add_modifier(Modifier::BOLD),
            ),
        ],
    );
    row += 3;

    // Rating
    let rating = if accuracy >= 100.0 {
        ("GENIUS! PERFECT GAME!", colors.green())
    } else if accuracy >= 80.0 {
        ("Excellent!", colors.green())
    } else if accuracy >= 60.0 {
        ("Good Job!", colors.cyan())
    } else if accuracy >= 40.0 {
        ("Keep Practicing!", colors.yellow())
    } else {
        ("Try Again!", colors.red())
    };

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            rating.0,
            Style::default().fg(rating.1).add_modifier(Modifier::BOLD),
        )],
    );

    // Help
    view.render_help(frame, vec![("Enter", "play again"), ("Esc", "exit")]);
}
