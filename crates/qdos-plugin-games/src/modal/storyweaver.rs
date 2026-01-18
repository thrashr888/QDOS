//! STORYWEAVER game modal rendering
//!
//! This module handles the rendering of the STORYWEAVER choose-your-own-adventure game.

use super::super::storyweaver::{StoryTemplate, StoryweaverState, StoryweaverView};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
    Frame,
};

/// Main draw function for STORYWEAVER
pub fn draw_storyweaver(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &StoryweaverState,
    colors: &ThemeColors,
) {
    match state.view {
        StoryweaverView::StorySelect => draw_story_select(frame, view, state, colors),
        StoryweaverView::CustomCreate => draw_custom_create(frame, view, state, colors),
        StoryweaverView::Loading => draw_loading(frame, view, state, colors),
        StoryweaverView::Playing => draw_playing(frame, view, state, colors),
        StoryweaverView::GameOver => draw_game_over(frame, view, state, colors),
        StoryweaverView::Error => draw_error(frame, view, state, colors),
    }
}

fn draw_story_select(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &StoryweaverState,
    colors: &ThemeColors,
) {
    // Title
    view.render_row(
        frame,
        0,
        vec![Span::styled(
            "       SELECT YOUR ADVENTURE",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );

    view.render_row(
        frame,
        1,
        vec![Span::styled(
            "  ════════════════════════════════════════",
            Style::default().fg(colors.blue()),
        )],
    );

    // Story list
    let stories = StoryTemplate::all();
    for (i, template) in stories.iter().enumerate() {
        let is_selected = i == state.selected_story;
        let row = 3 + (i as u16 * 2);

        let arrow = if is_selected { ">" } else { " " };
        let icon = template.icon();

        let name_style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg())
        };

        let desc_style = if is_selected {
            Style::default().fg(colors.grey()).bg(colors.red())
        } else {
            Style::default().fg(colors.grey())
        };

        let age_style = if is_selected {
            Style::default().fg(colors.cyan()).bg(colors.red())
        } else {
            Style::default().fg(colors.cyan())
        };

        view.render_row(
            frame,
            row,
            vec![
                Span::styled(format!("  {} ", arrow), name_style),
                Span::styled(format!("{} ", icon), Style::default().fg(colors.cyan())),
                Span::styled(format!("{:<30}", template.name()), name_style),
                Span::styled(format!("[{}]", template.age_rating()), age_style),
            ],
        );

        view.render_row(
            frame,
            row + 1,
            vec![
                Span::styled("      ", Style::default()),
                Span::styled(template.description(), desc_style),
            ],
        );
    }

    // Custom story option
    let custom_row = 3 + (stories.len() as u16 * 2);
    let is_custom_selected = state.selected_story == stories.len();

    let custom_style = if is_custom_selected {
        Style::default()
            .fg(colors.yellow())
            .bg(colors.red())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.green())
    };

    view.render_row(
        frame,
        custom_row + 1,
        vec![Span::styled(
            "  ────────────────────────────────────────",
            Style::default().fg(colors.grey()),
        )],
    );

    let arrow = if is_custom_selected { ">" } else { " " };
    view.render_row(
        frame,
        custom_row + 2,
        vec![
            Span::styled(format!("  {} ", arrow), custom_style),
            Span::styled("> ", Style::default().fg(colors.green())),
            Span::styled("CREATE YOUR OWN STORY...", custom_style),
        ],
    );

    view.render_row(
        frame,
        custom_row + 3,
        vec![
            Span::styled("      ", Style::default()),
            Span::styled(
                "Enter any premise and let AI generate your adventure!",
                Style::default().fg(colors.grey()),
            ),
        ],
    );

    // API status
    if !state.is_api_available() {
        view.render_row(
            frame,
            custom_row + 5,
            vec![Span::styled(
                "  ! AI not configured - Set ANTHROPIC_API_KEY",
                Style::default().fg(colors.red()),
            )],
        );
    }

    let help = vec![("^v", "select"), ("Enter", "start"), ("Esc", "back")];
    view.render_help(frame, help);
}

fn draw_custom_create(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &StoryweaverState,
    colors: &ThemeColors,
) {
    // Title
    view.render_row(
        frame,
        0,
        vec![Span::styled(
            "       CREATE YOUR OWN ADVENTURE",
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD),
        )],
    );

    view.render_row(
        frame,
        1,
        vec![Span::styled(
            "  ════════════════════════════════════════",
            Style::default().fg(colors.blue()),
        )],
    );

    // Premise input
    let premise_style = if state.custom_cursor == 0 {
        Style::default().fg(colors.yellow()).bg(colors.blue())
    } else {
        Style::default().fg(colors.fg())
    };

    view.render_row(
        frame,
        3,
        vec![Span::styled("  Enter your story premise:", premise_style)],
    );

    // Text input box
    let box_style = if state.custom_cursor == 0 {
        Style::default().fg(colors.cyan())
    } else {
        Style::default().fg(colors.grey())
    };

    view.render_row(
        frame,
        4,
        vec![Span::styled(
            "  +────────────────────────────────────────────────────────+",
            box_style,
        )],
    );

    // Show premise text (wrap to 56 chars)
    let premise = &state.custom_premise;
    let display_premise = if premise.len() > 56 {
        format!("{}...", &premise[premise.len() - 53..])
    } else {
        format!("{}_", premise)
    };

    view.render_row(
        frame,
        5,
        vec![
            Span::styled("  | ", box_style),
            Span::styled(
                format!("{:<56}", display_premise),
                Style::default().fg(colors.fg()),
            ),
            Span::styled(" |", box_style),
        ],
    );

    view.render_row(
        frame,
        6,
        vec![Span::styled(
            "  +────────────────────────────────────────────────────────+",
            box_style,
        )],
    );

    // Tone selector
    let tone_style = if state.custom_cursor == 1 {
        Style::default().fg(colors.yellow()).bg(colors.blue())
    } else {
        Style::default().fg(colors.fg())
    };

    view.render_row(
        frame,
        8,
        vec![
            Span::styled("  Tone: ", tone_style),
            Span::styled("<", Style::default().fg(colors.cyan())),
            Span::styled(
                format!(" {} ", state.custom_tone.name()),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(">", Style::default().fg(colors.cyan())),
        ],
    );

    // Generate button
    let generate_style = if state.custom_cursor == 2 {
        Style::default()
            .fg(colors.yellow())
            .bg(colors.green())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.green())
    };

    view.render_row(
        frame,
        10,
        vec![
            Span::styled("           ", Style::default()),
            Span::styled("[ Generate Story ]", generate_style),
        ],
    );

    // Error message
    if let Some(ref err) = state.error_message {
        view.render_row(
            frame,
            12,
            vec![Span::styled(
                format!("  Error: {}", err),
                Style::default().fg(colors.red()),
            )],
        );
    }

    let help = vec![
        ("^v", "navigate"),
        ("<>", "tone"),
        ("Enter", "generate"),
        ("Esc", "back"),
    ];
    view.render_help(frame, help);
}

fn draw_loading(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &StoryweaverState,
    colors: &ThemeColors,
) {
    let center = view.content_height() / 2;
    let frame_idx = state.loading_frame as usize;

    // Animated quill writing - 4 frame animation
    let quill_frames = [
        [
            "        .--.",
            "       /    \\",
            "      |  ()  |  *",
            "       \\    /  /",
            "    ____`--'__/",
            "   |          |",
            "   |  ~~~~~~  |",
            "   |__________|",
        ],
        [
            "        .--.",
            "       /    \\",
            "      |  ()  | .",
            "       \\    / ' ",
            "    ____`--'___",
            "   |          |",
            "   |  ~~~~~~  |",
            "   |__________|",
        ],
        [
            "        .--.",
            "       /    \\",
            "      |  ()  |",
            "       \\    / ,",
            "    ____`--'_/_",
            "   |          |",
            "   |  ~~~~~~~ |",
            "   |__________|",
        ],
        [
            "        .--.",
            "       /    \\",
            "      |  ()  |",
            "       \\    /  \\",
            "    ____`--'___*",
            "   |          |",
            "   |  ~~~~~~~~|",
            "   |__________|",
        ],
    ];

    let quill = &quill_frames[frame_idx % quill_frames.len()];

    // Center the quill
    for (i, line) in quill.iter().enumerate() {
        view.render_row(
            frame,
            center - 5 + i as u16,
            vec![Span::styled(
                format!("                    {}", line),
                Style::default().fg(colors.yellow()),
            )],
        );
    }

    // Progress dots animation
    let dots = ["   ", ".  ", ".. ", "...", " ..", "  .", "   ", "   "];
    let dot_anim = dots[frame_idx % dots.len()];

    view.render_row(
        frame,
        center + 4,
        vec![Span::styled(
            format!("              Weaving your story{}", dot_anim),
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );

    // Spinning book pages indicator
    let spinner = ["|", "/", "-", "\\"];
    let spin = spinner[frame_idx % spinner.len()];

    view.render_row(
        frame,
        center + 6,
        vec![Span::styled(
            format!("                   [{}]", spin),
            Style::default().fg(colors.grey()),
        )],
    );
}

fn draw_playing(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &StoryweaverState,
    colors: &ThemeColors,
) {
    let chapter = match state.current_chapter_data() {
        Some(c) => c,
        None => return,
    };

    let template = state.active_template.unwrap_or(StoryTemplate::Custom);

    // Header
    view.render_row(
        frame,
        0,
        vec![
            Span::styled(
                format!("  {} ", template.icon()),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(
                template.name().to_string(),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "          Chapter {}    Choices: {}",
                    state.current_chapter + 1,
                    state.total_choices
                ),
                Style::default().fg(colors.grey()),
            ),
        ],
    );

    view.render_row(
        frame,
        1,
        vec![Span::styled(
            "  ════════════════════════════════════════════════════════════",
            Style::default().fg(colors.blue()),
        )],
    );

    // Narrative with typewriter effect
    let narrative = &chapter.narrative;
    let visible_text = if state.text_reveal >= narrative.len() {
        narrative.as_str()
    } else {
        // Find nearest char boundary to avoid panicking on multi-byte chars
        let mut end = state.text_reveal;
        while end > 0 && !narrative.is_char_boundary(end) {
            end -= 1;
        }
        &narrative[..end]
    };

    // Word wrap narrative to ~60 chars
    // Calculate available lines: content_height - header(3) - choices(7) - footer
    let max_narrative_lines = (view.content_height() as usize).saturating_sub(10).max(8);
    let lines = wrap_text(visible_text, 60);
    for (i, line) in lines.iter().enumerate().take(max_narrative_lines) {
        view.render_row(
            frame,
            3 + i as u16,
            vec![
                Span::styled("  ", Style::default()),
                Span::styled(line.clone(), Style::default().fg(colors.fg())),
            ],
        );
    }

    // Choices - position below narrative
    if state.text_reveal >= narrative.len() && !chapter.choices.is_empty() {
        // Position choices after narrative, but ensure minimum space at top
        let narrative_display_lines = lines.len().min(max_narrative_lines);
        let choice_start = (3 + narrative_display_lines as u16 + 1).max(10);

        view.render_row(
            frame,
            choice_start - 1,
            vec![Span::styled(
                "  ──────────────────────────────────────────────────────────",
                Style::default().fg(colors.grey()),
            )],
        );

        view.render_row(
            frame,
            choice_start,
            vec![Span::styled(
                "  What do you do?",
                Style::default()
                    .fg(colors.cyan())
                    .add_modifier(Modifier::BOLD),
            )],
        );

        for (i, choice) in chapter.choices.iter().enumerate() {
            let is_selected = i == state.selected_choice;
            let row = choice_start + 2 + i as u16;

            let style = if is_selected {
                Style::default()
                    .fg(colors.yellow())
                    .bg(colors.red())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg())
            };

            let arrow = if is_selected { ">" } else { " " };

            view.render_row(
                frame,
                row,
                vec![
                    Span::styled(format!("  {} ", arrow), style),
                    Span::styled(
                        format!("[{}] ", choice.label),
                        Style::default().fg(colors.cyan()),
                    ),
                    Span::styled(choice.text.clone(), style),
                ],
            );
        }
    }

    let help = if state.text_reveal < narrative.len() {
        vec![("Space", "skip"), ("Esc", "quit")]
    } else {
        vec![("^v", "select"), ("Enter", "choose"), ("Esc", "quit")]
    };
    view.render_help(frame, help);
}

fn draw_game_over(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &StoryweaverState,
    colors: &ThemeColors,
) {
    let center = view.content_height() / 2;

    // Show final narrative
    if let Some(chapter) = state.current_chapter_data() {
        let lines = wrap_text(&chapter.narrative, 60);
        for (i, line) in lines.iter().enumerate().take(8) {
            view.render_row(
                frame,
                2 + i as u16,
                vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(line.clone(), Style::default().fg(colors.fg())),
                ],
            );
        }
    }

    // Stats box
    view.render_row(
        frame,
        center + 2,
        vec![Span::styled(
            "  ╔═══════════════════════════════════════════╗",
            Style::default().fg(colors.yellow()),
        )],
    );

    view.render_row(
        frame,
        center + 3,
        vec![Span::styled(
            format!("  ║{:^43}║", "YOUR JOURNEY ENDS"),
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );

    view.render_row(
        frame,
        center + 4,
        vec![Span::styled(
            format!(
                "  ║{:^43}║",
                format!(
                    "Chapters: {}    Choices: {}",
                    state.chapters_read, state.total_choices
                )
            ),
            Style::default().fg(colors.cyan()),
        )],
    );

    view.render_row(
        frame,
        center + 5,
        vec![Span::styled(
            format!("  ║{:^43}║", format!("Score: {}", state.final_score())),
            Style::default().fg(colors.green()),
        )],
    );

    view.render_row(
        frame,
        center + 6,
        vec![Span::styled(
            "  ╚═══════════════════════════════════════════╝",
            Style::default().fg(colors.yellow()),
        )],
    );

    let help = vec![("Enter", "new story"), ("Esc", "menu")];
    view.render_help(frame, help);
}

fn draw_error(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &StoryweaverState,
    colors: &ThemeColors,
) {
    let center = view.content_height() / 2;

    view.render_row(
        frame,
        center - 1,
        vec![Span::styled(
            "  ╔═══════════════════════════════════════════╗",
            Style::default().fg(colors.red()),
        )],
    );

    view.render_row(
        frame,
        center,
        vec![Span::styled(
            format!("  ║{:^43}║", "STORY ERROR"),
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::BOLD),
        )],
    );

    let error = state.error_message.as_deref().unwrap_or("Unknown error");
    let truncated = if error.len() > 40 {
        format!("{}...", &error[..37])
    } else {
        error.to_string()
    };

    view.render_row(
        frame,
        center + 1,
        vec![Span::styled(
            format!("  ║ {:<41} ║", truncated),
            Style::default().fg(colors.yellow()),
        )],
    );

    view.render_row(
        frame,
        center + 2,
        vec![Span::styled(
            "  ╚═══════════════════════════════════════════╝",
            Style::default().fg(colors.red()),
        )],
    );

    let help = vec![("Enter", "retry"), ("Esc", "back")];
    view.render_help(frame, help);
}

/// Simple word wrap helper
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= width {
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
