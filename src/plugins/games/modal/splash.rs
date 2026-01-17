//! Game splash screen rendering
//!
//! Displays sixel splash screen images for games with music.

use crate::app::ThemeColors;
use crate::plugins::games::state::GameType;
use crate::ui::components::FullScreenView;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    Frame,
};
use ratatui_image::picker::Picker;
use ratatui_image::StatefulImage;
use std::sync::{Mutex, OnceLock};

// Lazy-loaded image picker (detects Kitty/Sixel/iTerm2 protocols)
static IMAGE_PICKER: OnceLock<Mutex<Picker>> = OnceLock::new();

/// Get or initialize the image picker with terminal protocol detection
fn get_image_picker() -> &'static Mutex<Picker> {
    IMAGE_PICKER.get_or_init(|| {
        let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::from_fontsize((8, 16)));
        Mutex::new(picker)
    })
}

// Embed splash images at compile time
const SPLASH_ADVENTURE: &[u8] = include_bytes!("../../../../assets/splash/adventure.png");
const SPLASH_BIOLAB: &[u8] = include_bytes!("../../../../assets/splash/biolab.png");
const SPLASH_BLACKJACK: &[u8] = include_bytes!("../../../../assets/splash/blackjack.png");
const SPLASH_BLOCKWORLD: &[u8] = include_bytes!("../../../../assets/splash/blockworld.png");
const SPLASH_BREAKOUT: &[u8] = include_bytes!("../../../../assets/splash/breakout.png");
const SPLASH_CAVERNS: &[u8] = include_bytes!("../../../../assets/splash/caverns.png");
const SPLASH_COSMOS: &[u8] = include_bytes!("../../../../assets/splash/cosmos.png");
const SPLASH_DOPEWARS: &[u8] = include_bytes!("../../../../assets/splash/dopewars.png");
const SPLASH_DUNGEON: &[u8] = include_bytes!("../../../../assets/splash/dungeon.png");
const SPLASH_GUMSHOE: &[u8] = include_bytes!("../../../../assets/splash/gumshoe.png");
const SPLASH_MICROPOLIS: &[u8] = include_bytes!("../../../../assets/splash/micropolis.png");
const SPLASH_MINESWEEPER: &[u8] = include_bytes!("../../../../assets/splash/minesweeper.png");
const SPLASH_POKER: &[u8] = include_bytes!("../../../../assets/splash/poker.png");
const SPLASH_ROGUE: &[u8] = include_bytes!("../../../../assets/splash/rogue.png");
const SPLASH_SLOTS: &[u8] = include_bytes!("../../../../assets/splash/slots.png");
const SPLASH_SNAKE: &[u8] = include_bytes!("../../../../assets/splash/snake.png");
const SPLASH_TETRIS: &[u8] = include_bytes!("../../../../assets/splash/tetris.png");
const SPLASH_TREK: &[u8] = include_bytes!("../../../../assets/splash/trek.png");
const SPLASH_WESTWORLD: &[u8] = include_bytes!("../../../../assets/splash/westworld.png");

/// Get splash image data for a game type
pub fn get_splash_image(game_type: GameType) -> Option<&'static [u8]> {
    match game_type {
        GameType::Adventure => Some(SPLASH_ADVENTURE),
        GameType::Biolab => Some(SPLASH_BIOLAB),
        GameType::Blackjack => Some(SPLASH_BLACKJACK),
        GameType::Blockworld => Some(SPLASH_BLOCKWORLD),
        GameType::Breakout => Some(SPLASH_BREAKOUT),
        GameType::Caverns => Some(SPLASH_CAVERNS),
        GameType::Cosmos => Some(SPLASH_COSMOS),
        GameType::DopeWars => Some(SPLASH_DOPEWARS),
        GameType::Dungeon => Some(SPLASH_DUNGEON),
        GameType::Gumshoe => Some(SPLASH_GUMSHOE),
        GameType::Micropolis => Some(SPLASH_MICROPOLIS),
        GameType::Minesweeper => Some(SPLASH_MINESWEEPER),
        GameType::Poker => Some(SPLASH_POKER),
        GameType::Rogue => Some(SPLASH_ROGUE),
        GameType::Slots => Some(SPLASH_SLOTS),
        GameType::Snake => Some(SPLASH_SNAKE),
        GameType::Tetris => Some(SPLASH_TETRIS),
        GameType::Trek => Some(SPLASH_TREK),
        GameType::Westworld => Some(SPLASH_WESTWORLD),
        // Games without splash screens yet - return None for ASCII fallback
        _ => None,
    }
}

/// Render splash screen for a game
pub fn draw_splash(frame: &mut Frame, area: Rect, game_type: GameType, colors: &ThemeColors) {
    // Try to get and display splash image
    if let Some(image_data) = get_splash_image(game_type) {
        if let Ok(img) = image::load_from_memory(image_data) {
            if let Ok(mut picker) = get_image_picker().lock() {
                // Reserve space for prompt at bottom
                let image_area =
                    Rect::new(area.x, area.y, area.width, area.height.saturating_sub(2));

                let mut protocol = picker.new_resize_protocol(img);
                let widget = StatefulImage::new(None);
                frame.render_stateful_widget(widget, image_area, &mut protocol);

                // Draw prompt at bottom
                draw_splash_prompt(frame, area, colors);
                return;
            }
        }
    }

    // Fallback to ASCII splash
    draw_ascii_splash(frame, area, game_type, colors);
}

/// Draw the "Press any key" prompt
fn draw_splash_prompt(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let prompt_y = area.y + area.height.saturating_sub(1);
    let prompt = Line::from(vec![
        Span::styled(
            "Press any key to start",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK),
        ),
        Span::raw("  "),
        Span::styled("Q/Esc", Style::default().fg(colors.green())),
        Span::styled(" to exit", Style::default().fg(colors.fg())),
    ]);

    let prompt_area = Rect::new(area.x, prompt_y, area.width, 1);
    frame.render_widget(
        ratatui::widgets::Paragraph::new(prompt).alignment(ratatui::layout::Alignment::Center),
        prompt_area,
    );
}

/// Fallback ASCII splash screen
fn draw_ascii_splash(frame: &mut Frame, area: Rect, game_type: GameType, colors: &ThemeColors) {
    let view = FullScreenView::new(area, &format!(" {} ", game_type.name()), colors);
    view.render_frame(frame);

    let content = view.content_area();
    let center_y = content.height / 2;

    // Game title
    view.render_row(
        frame,
        center_y.saturating_sub(2),
        vec![Span::styled(
            game_type.name(),
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );

    // Description
    view.render_row(
        frame,
        center_y,
        vec![Span::styled(
            game_type.description(),
            Style::default().fg(colors.fg()),
        )],
    );

    // Prompt
    view.render_row(
        frame,
        center_y + 3,
        vec![Span::styled(
            "Press any key to start",
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::SLOW_BLINK),
        )],
    );

    view.render_help(frame, vec![("Any key", "start"), ("Q/Esc", "exit")]);
}
