//! Games plugin modal rendering
//!
//! This module handles all modal rendering for the games plugin, including
//! the game menu, individual game views, and shared screens like game over
//! and leaderboards.

// Per-game rendering modules
pub mod achievements;
mod adventure;
mod artillery;
mod asteroid;
mod baccarat;
mod biolab;
mod blackjack;
mod blockworld;
mod brainiac;
mod breakout;
mod bubbles;
mod caverns;
mod clicker;
mod cosmos;
mod craps;
mod dinojump;
mod dopewars;
mod dungeon;
mod gumshoe;
mod junglerun;
mod micropolis;
mod mindgames;
mod minesweeper;
mod neondrive;
mod poker;
mod rogue;
mod roulette;
mod slots;
mod snake;
pub mod splash;
mod stats;
mod storyweaver;
mod tetris;
mod trek;
mod westworld;

// Re-export game draw functions
pub use adventure::draw as draw_adventure;
pub use artillery::draw as draw_artillery;
pub use asteroid::draw_asteroid;
pub use baccarat::draw as draw_baccarat;
pub use biolab::draw_biolab;
pub use blackjack::draw as draw_blackjack;
pub use blockworld::draw_blockworld;
pub use brainiac::draw_brainiac;
pub use breakout::draw_breakout;
pub use bubbles::draw_bubbles;
pub use caverns::draw_caverns;
pub use clicker::draw_clicker;
pub use cosmos::draw as draw_cosmos;
pub use craps::draw as draw_craps;
pub use dinojump::draw_dinojump;
pub use dopewars::draw_dopewars;
pub use dungeon::draw_dungeon;
pub use gumshoe::draw_gumshoe;
pub use junglerun::draw as draw_junglerun;
pub use micropolis::draw as draw_micropolis;
pub use mindgames::draw_mindgames;
pub use minesweeper::draw as draw_minesweeper;
pub use neondrive::draw as draw_neondrive;
pub use poker::draw as draw_poker;
pub use rogue::draw_rogue;
pub use roulette::draw as draw_roulette;
pub use slots::draw as draw_slots;
pub use snake::draw_snake;
pub use storyweaver::draw_storyweaver;
pub use tetris::draw_tetris;
pub use trek::draw_trek;
pub use westworld::draw_westworld;

use super::state::{GameType, GamesState, GamesView};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

// =============================================================================
// GAMES MENU ASCII ART
// =============================================================================

/// Large GAMES title - line 1
const GAMES_TITLE_1: &str = " ▄▄▄▄▄  ▄▄▄▄▄  ▄   ▄  ▄▄▄▄▄  ▄▄▄▄▄ ";
/// Large GAMES title - line 2
const GAMES_TITLE_2: &str = "█       █   █  ██ ██  █      █     ";
/// Large GAMES title - line 3
const GAMES_TITLE_3: &str = "█  ▀▀▀  █████  █ █ █  █████  ▀▀▀▀█ ";
/// Large GAMES title - line 4
const GAMES_TITLE_4: &str = "█    █  █   █  █   █  █          █ ";
/// Large GAMES title - line 5
const GAMES_TITLE_5: &str = " ▀▀▀▀   ▀   ▀  ▀   ▀  ▀▀▀▀▀  ▀▀▀▀  ";

/// Decorative separator
const GAMES_SEPARATOR: &str = "─═══════════════════════════════════─";

// =============================================================================
// PUBLIC API
// =============================================================================

/// Draw the games modal
pub fn draw_games_modal(
    frame: &mut Frame,
    area: Rect,
    state: &GamesState,
    stats: &super::platform::PlayerStats,
    achievements: &super::platform::AchievementManager,
    session_secs: u64,
    colors: &ThemeColors,
) {
    let title = match state.view {
        GamesView::Menu => " Games ",
        GamesView::Splash => match state.selected_game_type() {
            GameType::Tetris => " Tetris ",
            GameType::Snake => " Snake ",
            GameType::Breakout => " Breakout ",
            GameType::Rogue => " Rogue ",
            GameType::Trek => " Star Trek ",
            GameType::Clicker => " Clicker ",
            GameType::Brainiac => " Brainiac ",
            GameType::Storyweaver => " Storyweaver ",
            GameType::DopeWars => " Dope Wars ",
            GameType::Minesweeper => " Minesweeper ",
            GameType::Artillery => " Artillery ",
            GameType::Mindgames => " Mindgames ",
            GameType::Gumshoe => " Gumshoe ",
            GameType::Dungeon => " Dungeon ",
            GameType::Caverns => " Caverns ",
            GameType::Biolab => " Biolab ",
            GameType::Neondrive => " Neon Drive ",
            GameType::Micropolis => " Micropolis ",
            GameType::JungleRun => " Jungle Run ",
            GameType::Adventure => " Adventure ",
            GameType::Blackjack => " Blackjack ",
            GameType::Roulette => " Roulette ",
            GameType::Cosmos => " Cosmos ",
            GameType::Blockworld => " Blockworld ",
            GameType::Westworld => " Westworld ",
            GameType::Slots => " Slots ",
            GameType::Poker => " Video Poker ",
            GameType::Baccarat => " Baccarat ",
            GameType::Craps => " Craps ",
            GameType::Bubbles => " Bubbles ",
            GameType::Asteroid => " Asteroids ",
            GameType::DinoJump => " Dino Jump ",
        },
        GamesView::Playing | GamesView::Paused => match state.current_game {
            Some(GameType::Tetris) => " Tetris ",
            Some(GameType::Snake) => " Snake ",
            Some(GameType::Breakout) => " Breakout ",
            Some(GameType::Rogue) => " Rogue ",
            Some(GameType::Trek) => " Star Trek ",
            Some(GameType::Clicker) => " Clicker ",
            Some(GameType::Brainiac) => " Brainiac ",
            Some(GameType::Storyweaver) => " Storyweaver ",
            Some(GameType::DopeWars) => " Dope Wars ",
            Some(GameType::Minesweeper) => " Minesweeper ",
            Some(GameType::Artillery) => " Artillery ",
            Some(GameType::Mindgames) => " Mindgames ",
            Some(GameType::Gumshoe) => " Gumshoe ",
            Some(GameType::Dungeon) => " Dungeon ",
            Some(GameType::Caverns) => " Caverns ",
            Some(GameType::Biolab) => " Biolab ",
            Some(GameType::Neondrive) => " Neon Drive ",
            Some(GameType::Micropolis) => " Micropolis ",
            Some(GameType::JungleRun) => " Jungle Run ",
            Some(GameType::Adventure) => " Adventure ",
            Some(GameType::Blackjack) => " Blackjack ",
            Some(GameType::Roulette) => " Roulette ",
            Some(GameType::Cosmos) => " Cosmos ",
            Some(GameType::Blockworld) => " Blockworld ",
            Some(GameType::Westworld) => " Westworld ",
            Some(GameType::Slots) => " Slots ",
            Some(GameType::Poker) => " Video Poker ",
            Some(GameType::Baccarat) => " Baccarat ",
            Some(GameType::Craps) => " Craps ",
            Some(GameType::Bubbles) => " Bubbles ",
            Some(GameType::Asteroid) => " Asteroids ",
            Some(GameType::DinoJump) => " Dino Jump ",
            None => " Games ",
        },
        GamesView::GameOver => " Game Over ",
        GamesView::EnteringInitials => " High Score! ",
        GamesView::Leaderboard => " Leaderboard ",
        GamesView::Stats => " Statistics ",
        GamesView::Achievements => " Achievements ",
    };

    let view = FullScreenView::new(area, title, colors);
    view.render_frame(frame);

    match state.view {
        GamesView::Menu => draw_menu(frame, &view, state, colors),
        GamesView::Splash => splash::draw_splash(frame, area, state.selected_game_type(), colors),
        GamesView::Playing => draw_game(frame, &view, state, colors),
        GamesView::Paused => draw_paused(frame, &view, state, colors),
        GamesView::GameOver => draw_game_over(frame, &view, state, colors),
        GamesView::EnteringInitials => draw_initials_entry(frame, &view, state, colors),
        GamesView::Leaderboard => draw_leaderboard(frame, &view, state, colors),
        GamesView::Stats => stats::draw_stats(
            frame,
            &view,
            stats,
            state.casino_credits,
            state.stats_scroll_offset,
            session_secs,
            colors,
        ),
        GamesView::Achievements => achievements::draw_achievements(
            frame,
            &view,
            achievements,
            stats,
            state.achievements_scroll_offset,
            colors,
        ),
    }
}

// =============================================================================
// MENU RENDERING
// =============================================================================

fn draw_menu(frame: &mut Frame, view: &FullScreenView, state: &GamesState, colors: &ThemeColors) {
    // === LARGE COLORFUL "GAMES" TITLE WITH ANIMATION ===
    let title_lines = [
        GAMES_TITLE_1,
        GAMES_TITLE_2,
        GAMES_TITLE_3,
        GAMES_TITLE_4,
        GAMES_TITLE_5,
    ];

    // Animation: wave offset changes over time, color cycles
    let tick = state.menu_tick;
    let wave_offset = (tick / 2) % 20; // Wave moves every 2 ticks
    let color_phase = (tick / 4) % 4; // Color cycles every 4 ticks

    // Color cycle: cyan -> blue -> magenta -> red -> back
    let cycle_colors = [colors.cyan(), colors.blue(), colors.red(), colors.yellow()];

    for (row, line) in title_lines.iter().enumerate() {
        let mut spans: Vec<Span> = Vec::new();

        // Wave effect: shift each row left/right based on sine-like pattern
        let row_wave = match (wave_offset as i32 + row as i32) % 4 {
            0 => 0,
            1 => 1,
            2 => 0,
            3 => -1,
            _ => 0,
        };
        let margin = (2 + row_wave).max(0) as usize;
        spans.push(Span::raw(" ".repeat(margin)));

        for (i, ch) in line.chars().enumerate() {
            let color = if ch == ' ' {
                colors.bg()
            } else {
                // Animated color cycling with position-based offset
                let phase = ((i + tick as usize + row * 3) / 4) % 4;
                let adjusted_phase = (phase + color_phase as usize) % 4;
                cycle_colors[adjusted_phase]
            };

            let style = if ch == '█' || ch == '▄' || ch == '▀' {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };

            spans.push(Span::styled(ch.to_string(), style));
        }
        view.render_row(frame, row as u16, spans);
    }

    // === DECORATIVE SEPARATOR ===
    view.render_row(
        frame,
        5,
        vec![
            Span::raw("  "),
            Span::styled(GAMES_SEPARATOR, Style::default().fg(colors.blue())),
        ],
    );

    // === "R-DOS ARCADE" subtitle ===
    view.render_row(
        frame,
        6,
        vec![
            Span::raw("       "),
            Span::styled(
                "▒▓█",
                Style::default()
                    .fg(colors.red())
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(
                " R-DOS ARCADE ",
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "█▓▒",
                Style::default()
                    .fg(colors.red())
                    .add_modifier(Modifier::DIM),
            ),
        ],
    );

    view.render_row(
        frame,
        7,
        vec![
            Span::raw("  "),
            Span::styled(GAMES_SEPARATOR, Style::default().fg(colors.blue())),
        ],
    );

    // === GAME LIST WITH SCROLLING ===
    let start_row = 9;
    const MAX_VISIBLE_GAMES: usize = 6;
    let all_games = GameType::all();
    let scroll_offset = state.menu_scroll_offset;
    let visible_end = (scroll_offset + MAX_VISIBLE_GAMES).min(all_games.len());

    // Render visible games
    for (display_idx, i) in (scroll_offset..visible_end).enumerate() {
        let game = &all_games[i];
        let is_selected = i == state.selected_game;
        let high_score = state.high_scores.get(i).copied().unwrap_or(0);

        // Number prefix with color
        let num_style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.cyan())
        };

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

        let arrow = if is_selected { "►" } else { " " };

        view.render_row(
            frame,
            start_row + (display_idx as u16 * 2),
            vec![
                Span::styled("   ", Style::default()),
                Span::styled(arrow, num_style),
                Span::styled(format!(" {}) ", i + 1), num_style),
                Span::styled(format!("{:<12}", game.name()), name_style),
                Span::styled(format!(" - {}", game.description()), desc_style),
            ],
        );

        if high_score > 0 {
            view.render_row(
                frame,
                start_row + (display_idx as u16 * 2) + 1,
                vec![
                    Span::styled("        ", Style::default()),
                    Span::styled("* ", Style::default().fg(colors.yellow())),
                    Span::styled(
                        format!("High Score: {}", high_score),
                        Style::default().fg(colors.green()),
                    ),
                ],
            );
        }
    }

    // Scroll indicators
    let indicator_row = start_row + (MAX_VISIBLE_GAMES as u16 * 2);
    if scroll_offset > 0 || visible_end < all_games.len() {
        let indicator = if scroll_offset > 0 && visible_end < all_games.len() {
            format!(
                "        ↑ {}/{} ↓",
                state.selected_game + 1,
                all_games.len()
            )
        } else if scroll_offset > 0 {
            format!("        ↑ {}/{}", state.selected_game + 1, all_games.len())
        } else {
            format!("        {}/{} ↓", state.selected_game + 1, all_games.len())
        };
        view.render_row(
            frame,
            indicator_row,
            vec![Span::styled(
                indicator,
                Style::default()
                    .fg(colors.grey())
                    .add_modifier(Modifier::DIM),
            )],
        );
    }

    let help = vec![
        ("↑↓/1-11", "select"),
        ("Enter", "play"),
        ("L", "scores"),
        ("S", "stats"),
        ("A", "achieve"),
        ("Esc", "close"),
    ];
    view.render_help(frame, help);
}

// =============================================================================
// GAME DISPATCHER
// =============================================================================

fn draw_game(frame: &mut Frame, view: &FullScreenView, state: &GamesState, colors: &ThemeColors) {
    match state.current_game {
        Some(GameType::Tetris) => draw_tetris(frame, view, &state.tetris, colors),
        Some(GameType::Snake) => draw_snake(frame, view, &state.snake, colors),
        Some(GameType::Breakout) => draw_breakout(frame, view, &state.breakout, colors),
        Some(GameType::Rogue) => draw_rogue(frame, view, &state.rogue, colors),
        Some(GameType::Trek) => draw_trek(frame, view, &state.trek, colors),
        Some(GameType::Clicker) => draw_clicker(frame, view, &state.clicker, colors),
        Some(GameType::Brainiac) => draw_brainiac(frame, view, &state.brainiac, colors),
        Some(GameType::Storyweaver) => draw_storyweaver(frame, view, &state.storyweaver, colors),
        Some(GameType::DopeWars) => draw_dopewars(frame, view, &state.dopewars, colors),
        Some(GameType::Minesweeper) => {
            draw_minesweeper(frame, view.area, &state.minesweeper, colors)
        }
        Some(GameType::Artillery) => draw_artillery(frame, view.area, &state.artillery, colors),
        Some(GameType::Mindgames) => draw_mindgames(frame, view, &state.mindgames, colors),
        Some(GameType::Gumshoe) => draw_gumshoe(frame, view, &state.gumshoe, colors),
        Some(GameType::Dungeon) => draw_dungeon(frame, view, &state.dungeon, colors),
        Some(GameType::Caverns) => draw_caverns(frame, view, &state.caverns, colors),
        Some(GameType::Biolab) => draw_biolab(frame, view, &state.biolab, colors),
        Some(GameType::Neondrive) => draw_neondrive(frame, view.area, &state.neondrive, colors),
        Some(GameType::Micropolis) => draw_micropolis(frame, view.area, &state.micropolis, colors),
        Some(GameType::JungleRun) => draw_junglerun(frame, view.area, &state.junglerun, colors),
        Some(GameType::Adventure) => draw_adventure(frame, view.area, &state.adventure, colors),
        Some(GameType::Blackjack) => draw_blackjack(frame, view.area, &state.blackjack, colors),
        Some(GameType::Roulette) => draw_roulette(frame, view.area, &state.roulette, colors),
        Some(GameType::Cosmos) => draw_cosmos(frame, view.area, &state.cosmos, colors),
        Some(GameType::Blockworld) => draw_blockworld(frame, view.area, &state.blockworld, colors),
        Some(GameType::Westworld) => draw_westworld(frame, view.area, &state.westworld, colors),
        Some(GameType::Slots) => draw_slots(frame, view.area, &state.slots, colors),
        Some(GameType::Poker) => draw_poker(frame, view.area, &state.poker, colors),
        Some(GameType::Baccarat) => draw_baccarat(frame, view.area, &state.baccarat, colors),
        Some(GameType::Craps) => draw_craps(frame, view.area, &state.craps, colors),
        Some(GameType::Bubbles) => draw_bubbles(frame, view, &state.bubbles, colors),
        Some(GameType::Asteroid) => draw_asteroid(frame, view, &state.asteroid, colors),
        Some(GameType::DinoJump) => draw_dinojump(frame, view, &state.dinojump, colors),
        None => {}
    }
}

// =============================================================================
// SHARED SCREENS
// =============================================================================

fn draw_paused(frame: &mut Frame, view: &FullScreenView, state: &GamesState, colors: &ThemeColors) {
    // Draw the game in background
    draw_game(frame, view, state, colors);

    // Overlay pause message
    let content_height = view.content_height();
    let pause_row = content_height / 2;

    view.render_row(
        frame,
        pause_row,
        vec![Span::styled(
            "       ═══════════════════════       ",
            Style::default().fg(colors.yellow()),
        )],
    );
    view.render_row(
        frame,
        pause_row + 1,
        vec![Span::styled(
            "       ║      P A U S E D      ║       ",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        pause_row + 2,
        vec![Span::styled(
            "       ═══════════════════════       ",
            Style::default().fg(colors.yellow()),
        )],
    );

    let help = vec![("P", "resume"), ("Esc", "quit")];
    view.render_help(frame, help);
}

fn draw_game_over(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GamesState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height();
    let center_row = content_height / 2;

    // Check if it's a win (breakout, rogue, or trek)
    let is_win = (matches!(state.current_game, Some(GameType::Breakout))
        && state.breakout.game_won)
        || (matches!(state.current_game, Some(GameType::Rogue)) && state.rogue.game_won)
        || (matches!(state.current_game, Some(GameType::Trek)) && state.trek.game_won)
        || (matches!(state.current_game, Some(GameType::Caverns)) && state.caverns.game_won);

    let title = if is_win { "YOU WIN!" } else { "GAME OVER" };
    let title_color = if is_win { colors.green() } else { colors.red() };

    view.render_row(
        frame,
        center_row - 2,
        vec![Span::styled(
            "╔═══════════════════════════════════════════╗",
            Style::default().fg(title_color),
        )],
    );
    view.render_row(
        frame,
        center_row - 1,
        vec![Span::styled(
            format!("║{:^43}║", title),
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        center_row,
        vec![Span::styled(
            format!("║{:^43}║", format!("Final Score: {}", state.score)),
            Style::default().fg(colors.yellow()),
        )],
    );

    // Check high score
    if let Some(game) = state.current_game {
        let idx = match game {
            GameType::Tetris => 0,
            GameType::Snake => 1,
            GameType::Breakout => 2,
            GameType::Rogue => 3,
            GameType::Trek => 4,
            GameType::Clicker => 5,
            GameType::Brainiac => 6,
            GameType::Storyweaver => 7,
            GameType::DopeWars => 8,    // No legacy high score
            GameType::Minesweeper => 9, // No legacy high score
            GameType::Artillery => 10,  // No legacy high score
            GameType::Mindgames => 11,  // No legacy high score
            GameType::Gumshoe => 12,    // No legacy high score
            GameType::Dungeon => 13,    // No legacy high score
            GameType::Caverns => 14,    // No legacy high score
            GameType::Biolab => 15,     // No legacy high score
            GameType::Neondrive => 16,  // No legacy high score
            GameType::Micropolis => 17, // No legacy high score
            GameType::JungleRun => 18,  // No legacy high score
            GameType::Adventure => 19,  // No legacy high score
            GameType::Blackjack => 20,  // Casino game
            GameType::Roulette => 21,   // Casino game
            GameType::Cosmos => 22,     // Space exploration
            GameType::Blockworld => 23, // Mining adventure
            GameType::Westworld => 24,  // Western action
            GameType::Slots => 25,      // Casino slot machine
            GameType::Poker => 26,      // Casino video poker
            GameType::Baccarat => 27,   // Casino baccarat
            GameType::Craps => 28,      // Casino craps
            GameType::Bubbles => 29,    // Puzzle game
            GameType::Asteroid => 30,   // Arcade shooter
            GameType::DinoJump => 31,   // Endless runner
        };
        if idx < 8 && state.score >= state.high_scores[idx] && state.score > 0 {
            view.render_row(
                frame,
                center_row + 1,
                vec![Span::styled(
                    format!("║{:^43}║", "NEW HIGH SCORE!"),
                    Style::default()
                        .fg(colors.green())
                        .add_modifier(Modifier::BOLD),
                )],
            );
        } else {
            view.render_row(
                frame,
                center_row + 1,
                vec![Span::styled(
                    format!("║{:^43}║", ""),
                    Style::default().fg(title_color),
                )],
            );
        }
    }

    view.render_row(
        frame,
        center_row + 2,
        vec![Span::styled(
            "╚═══════════════════════════════════════════╝",
            Style::default().fg(title_color),
        )],
    );

    let help = vec![
        ("Enter", "play again"),
        ("L", "leaderboard"),
        ("Esc", "menu"),
    ];
    view.render_help(frame, help);
}

/// Draw the initials entry screen for high scores
fn draw_initials_entry(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GamesState,
    colors: &ThemeColors,
) {
    let center_row = 6u16;
    let title_color = colors.yellow();

    // Celebratory header
    view.render_row(
        frame,
        center_row - 2,
        vec![Span::styled(
            "╔═══════════════════════════════════════════╗",
            Style::default().fg(title_color),
        )],
    );
    view.render_row(
        frame,
        center_row - 1,
        vec![Span::styled(
            "║     *  N E W   H I G H   S C O R E  *     ║",
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        center_row,
        vec![Span::styled(
            "╠═══════════════════════════════════════════╣",
            Style::default().fg(title_color),
        )],
    );

    // Score display
    view.render_row(
        frame,
        center_row + 1,
        vec![Span::styled(
            format!("║{:^43}║", format!("Score: {}", state.score)),
            Style::default().fg(colors.cyan()),
        )],
    );

    // Initials entry
    view.render_row(
        frame,
        center_row + 3,
        vec![Span::styled(
            format!("║{:^43}║", "Enter your initials:"),
            Style::default().fg(colors.fg()),
        )],
    );

    // Draw the 3-character entry with cursor
    let chars: Vec<char> = state.initials_buffer.chars().collect();
    let mut initials_spans: Vec<Span> = vec![Span::styled(
        "║                 ",
        Style::default().fg(title_color),
    )];

    for (i, ch) in chars.iter().enumerate() {
        let style = if i == state.initials_cursor {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD)
        };
        initials_spans.push(Span::styled(format!(" {} ", ch), style));
    }

    initials_spans.push(Span::styled(
        "                 ║",
        Style::default().fg(title_color),
    ));

    view.render_row(frame, center_row + 5, initials_spans);

    // Instructions
    view.render_row(
        frame,
        center_row + 7,
        vec![Span::styled(
            format!("║{:^43}║", "←→ move   ↑↓ change letter"),
            Style::default().fg(colors.grey()),
        )],
    );

    view.render_row(
        frame,
        center_row + 8,
        vec![Span::styled(
            "╚═══════════════════════════════════════════╝",
            Style::default().fg(title_color),
        )],
    );

    let help = vec![("←→", "move"), ("↑↓", "change"), ("Enter", "confirm")];
    view.render_help(frame, help);
}

/// Draw the leaderboard view
fn draw_leaderboard(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GamesState,
    colors: &ThemeColors,
) {
    let game = state.leaderboard_game.unwrap_or(state.selected_game_type());
    let leaderboard = state.leaderboards.get(game);
    let title_color = colors.cyan();
    let sidebar_width = 18;

    // Draw vertical game list sidebar
    let games = GameType::all();
    let game_index = games.iter().position(|g| *g == game).unwrap_or(0);

    // Calculate which games to show (scroll if needed)
    let visible_rows = 16; // Rows available for game list
    let scroll_offset = if game_index >= visible_rows {
        game_index - visible_rows + 1
    } else {
        0
    };

    // Sidebar header
    view.render_row(
        frame,
        0,
        vec![Span::styled(
            format!("{:<sidebar_width$}", "  GAMES"),
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );

    // Game list in sidebar
    for (i, g) in games
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_rows)
    {
        let is_selected = *g == game;
        let style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.blue())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg())
        };
        let prefix = if is_selected { "> " } else { "  " };
        let name = format!("{}{}", prefix, g.name());
        let row = (i - scroll_offset + 1) as u16;
        view.render_row(
            frame,
            row,
            vec![Span::styled(format!("{:<sidebar_width$}", name), style)],
        );
    }

    // Scroll indicators
    if scroll_offset > 0 {
        view.render_row(
            frame,
            0,
            vec![
                Span::raw(" ".repeat(sidebar_width - 3)),
                Span::styled("[^]", Style::default().fg(colors.grey())),
            ],
        );
    }
    if scroll_offset + visible_rows < games.len() {
        view.render_row(
            frame,
            visible_rows as u16,
            vec![
                Span::raw(" ".repeat(sidebar_width - 3)),
                Span::styled("[v]", Style::default().fg(colors.grey())),
            ],
        );
    }

    // Draw leaderboard content on the right side
    // We'll use direct frame rendering for the right panel
    let content_x = view.content_area().x + sidebar_width as u16 + 1;
    let content_y = view.content_area().y;
    let content_width = 38;

    // Leaderboard header
    let header = format!("{} Leaderboard", game.name());
    let header_line = Line::from(vec![Span::styled(
        format!("{:^content_width$}", header),
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD),
    )]);
    frame.render_widget(
        Paragraph::new(header_line),
        Rect::new(content_x, content_y, content_width as u16, 1),
    );

    // Separator
    let sep_line = Line::from(vec![Span::styled(
        "-".repeat(content_width),
        Style::default().fg(title_color),
    )]);
    frame.render_widget(
        Paragraph::new(sep_line),
        Rect::new(content_x, content_y + 1, content_width as u16, 1),
    );

    // Column headers
    let col_header = Line::from(vec![Span::styled(
        format!("{:<5} {:<6} {:>12}", "RANK", "NAME", "SCORE"),
        Style::default().fg(colors.cyan()),
    )]);
    frame.render_widget(
        Paragraph::new(col_header),
        Rect::new(content_x, content_y + 2, content_width as u16, 1),
    );

    // Leaderboard entries
    if leaderboard.entries.is_empty() {
        let empty_line = Line::from(vec![Span::styled(
            format!("{:^content_width$}", "No scores yet"),
            Style::default().fg(colors.grey()),
        )]);
        frame.render_widget(
            Paragraph::new(empty_line),
            Rect::new(content_x, content_y + 4, content_width as u16, 1),
        );
    } else {
        for (i, entry) in leaderboard.entries.iter().take(10).enumerate() {
            let rank = format!("{:>3}.", i + 1);
            let name = &entry.initials;
            let score = format!("{:>12}", entry.score);

            let style = if i == 0 {
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD)
            } else if i < 3 {
                Style::default().fg(colors.cyan())
            } else {
                Style::default().fg(colors.fg())
            };

            let entry_line = Line::from(vec![Span::styled(
                format!("{} {:<6} {}", rank, name, score),
                style,
            )]);
            frame.render_widget(
                Paragraph::new(entry_line),
                Rect::new(content_x, content_y + 3 + i as u16, content_width as u16, 1),
            );
        }
    }

    // Special stats for Clicker
    if game == GameType::Clicker {
        let souls = &state.clicker.souls;
        let stats_y = content_y + 14;

        let stats_line = Line::from(vec![Span::styled(
            format!("Total Souls: {}", souls.total_souls),
            Style::default().fg(colors.green()),
        )]);
        frame.render_widget(
            Paragraph::new(stats_line),
            Rect::new(content_x, stats_y, content_width as u16, 1),
        );

        let prestige_line = Line::from(vec![Span::styled(
            format!("Total Runs: {}", souls.total_runs),
            Style::default().fg(colors.cyan()),
        )]);
        frame.render_widget(
            Paragraph::new(prestige_line),
            Rect::new(content_x, stats_y + 1, content_width as u16, 1),
        );
    }

    let help = vec![("Up/Dn", "select game"), ("Esc", "back")];
    view.render_help(frame, help);
}

fn draw_clicker_leaderboard(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GamesState,
    colors: &ThemeColors,
    title_color: ratatui::style::Color,
    leaderboard: &super::state::GameLeaderboard,
) {
    let souls = &state.clicker.souls;

    // Stats header
    view.render_row(
        frame,
        4,
        vec![Span::styled(
            "║                   SOUL STATISTICS                   ║",
            Style::default().fg(colors.cyan()),
        )],
    );
    view.render_row(
        frame,
        5,
        vec![Span::styled(
            "╠═══════════════════════════════════════════════════╣",
            Style::default().fg(title_color),
        )],
    );

    // Stats
    let stats = [
        ("Total Souls", format!("{}", souls.total_souls)),
        ("Total Runs", format!("{}", souls.total_runs)),
        ("Total Deaths", format!("{}", souls.total_deaths)),
        ("Best Floor", format!("{}", souls.best_floor)),
        (
            "Monsters Killed",
            format!("{}", souls.total_monsters_killed),
        ),
        ("Gold Earned", format!("{}", souls.total_gold_earned)),
        ("Zoo Cleared", format!("{}", souls.total_zoo_cleared)),
        ("Arcane Dust", format!("{}", souls.dust)),
        ("Alchemy Level", format!("{}", souls.alchemy_level)),
    ];

    for (i, (label, value)) in stats.iter().enumerate() {
        let row = 6 + i as u16;
        let style = Style::default().fg(if i % 2 == 0 {
            colors.fg()
        } else {
            colors.grey()
        });
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("║  {:<20}  {:>24}  ║", label, value),
                style,
            )],
        );
    }

    // Heirloom info
    if let Some(ref heirloom) = souls.heirloom {
        view.render_row(
            frame,
            15,
            vec![Span::styled(
                format!(
                    "║  Heirloom: {:<36}  ║",
                    format!(
                        "{} (STR+{} CRIT+{}% LS+{}%)",
                        heirloom.name,
                        heirloom.str_bonus,
                        heirloom.crit_bonus,
                        heirloom.life_steal_bonus
                    )
                ),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            )],
        );
    } else {
        view.render_row(
            frame,
            15,
            vec![Span::styled(
                "║  Heirloom: None                                   ║",
                Style::default().fg(colors.grey()),
            )],
        );
    }

    // Show top 3 scores at bottom
    view.render_row(
        frame,
        16,
        vec![Span::styled(
            "╠═══════════════════════════════════════════════════╣",
            Style::default().fg(title_color),
        )],
    );

    let top3: String = leaderboard
        .entries
        .iter()
        .take(3)
        .enumerate()
        .map(|(i, e)| {
            let medal = match i {
                0 => "[1]",
                1 => "[2]",
                _ => "[3]",
            };
            format!("{}{} {}", medal, e.initials, e.score)
        })
        .collect::<Vec<_>>()
        .join("  ");

    view.render_row(
        frame,
        16,
        vec![Span::styled(
            format!("║  Top: {:<42}  ║", top3),
            Style::default().fg(colors.green()),
        )],
    );
}

fn draw_standard_leaderboard(
    frame: &mut Frame,
    view: &FullScreenView,
    colors: &ThemeColors,
    title_color: ratatui::style::Color,
    leaderboard: &super::state::GameLeaderboard,
) {
    // Column headers
    view.render_row(
        frame,
        4,
        vec![Span::styled(
            "║  Rank   Initials              Score              ║",
            Style::default().fg(colors.grey()),
        )],
    );
    view.render_row(
        frame,
        5,
        vec![Span::styled(
            "║  ────   ────────              ─────              ║",
            Style::default().fg(colors.grey()),
        )],
    );

    // Entries
    for i in 0..10 {
        let row = 6 + i as u16;
        if let Some(entry) = leaderboard.entries.get(i) {
            let rank_str = format!("{}.", i + 1);
            let medal = match i {
                0 => "[1]",
                1 => "[2]",
                2 => "[3]",
                _ => "   ",
            };
            let style = match i {
                0 => Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
                1 => Style::default().fg(colors.fg()),
                2 => Style::default().fg(colors.cyan()),
                _ => Style::default().fg(colors.grey()),
            };
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!(
                        "║  {:>3} {}  {:<3}              {:>10}              ║",
                        rank_str, medal, entry.initials, entry.score
                    ),
                    style,
                )],
            );
        } else {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!(
                        "║  {:>3}     ---              ---------              ║",
                        format!("{}.", i + 1)
                    ),
                    Style::default()
                        .fg(colors.grey())
                        .add_modifier(Modifier::DIM),
                )],
            );
        }
    }

    view.render_row(
        frame,
        16,
        vec![Span::styled(
            "║                                                   ║",
            Style::default().fg(title_color),
        )],
    );
}
