//! Games plugin
//!
//! Built-in retro games including Tetris, Snake, Breakout, Rogue, Star Trek, Clicker, Brainiac, Storyweaver, Dope Wars, Minesweeper, Artillery, Mindgames, and Gumshoe.

pub mod adventure;
pub mod artillery;
pub mod biolab;
pub mod blackjack;
pub mod brainiac;
pub mod breakout;
pub mod caverns;
pub mod clicker;
pub mod dopewars;
pub mod dungeon;
pub mod gumshoe;
pub mod junglerun;
pub mod micropolis;
pub mod mindgames;
pub mod minesweeper;
mod modal;
pub mod neondrive;
pub mod platform;
pub mod rogue;
pub mod roulette;
pub mod snake;
pub mod state;
pub mod storyweaver;
pub mod tetris;
pub mod trek;

use crate::app::ThemeColors;
use crate::config::Config;
use crate::plugins::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo, SoundEvent,
};
use crate::sound::{ChiptuneMelody, ChiptuneMusic};
use crossterm::event::{KeyCode, KeyEvent};
use platform::{AchievementManager, GameEngine, StatsTracker};
use ratatui::{layout::Rect, Frame};
use state::{GameType, GamesState, GamesView};
use std::any::Any;
use std::path::PathBuf;

/// Games plugin - built-in retro games
pub struct GamesPlugin {
    pub state: GamesState,
    pub stats: StatsTracker,
    pub achievements: AchievementManager,
    pending_sounds: Vec<SoundEvent>,
    music: ChiptuneMusic,
    sounds_enabled: bool,
}

impl Default for GamesPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl GamesPlugin {
    pub fn new() -> Self {
        // Check if sounds are enabled from config
        let sounds_enabled = Config::load()
            .map(|c| c.general.play_sounds)
            .unwrap_or(true);

        let mut plugin = Self {
            state: GamesState::new(),
            stats: StatsTracker::empty(),
            achievements: AchievementManager::new(),
            pending_sounds: Vec::new(),
            music: ChiptuneMusic::new(),
            sounds_enabled,
        };
        plugin.load_from_config();
        plugin
    }

    /// Load persisted data from config file
    pub fn load_from_config(&mut self) {
        if let Ok(config) = Config::load() {
            // Decode leaderboards from base64 JSON
            self.state
                .load_leaderboards(config.games.get_leaderboards());

            // Decode clicker state from base64 JSON
            if let Some(clicker_state) = config.games.get_clicker_state() {
                self.state.clicker = clicker_state;
            }

            // Decode player stats from base64 JSON
            self.stats = StatsTracker::new(config.games.get_player_stats());

            // Decode achievements from base64 JSON
            self.achievements = AchievementManager::from_state(config.games.get_achievements());
        }
    }

    /// Save persisted data to config file
    pub fn save_to_config(&self) {
        if let Ok(mut config) = Config::load() {
            // Encode leaderboards to base64 JSON
            config.games.set_leaderboards(&self.state.leaderboards);

            // Encode clicker state to base64 JSON
            config.games.set_clicker_state(&self.state.clicker);

            // Encode player stats to base64 JSON
            config.games.set_player_stats(&self.stats.stats);

            // Encode achievements to base64 JSON
            config.games.set_achievements(&self.achievements.state);

            let _ = config.save();
        }
    }

    pub fn open_modal(&mut self) {
        // Reload config when opening modal (in case it changed externally)
        self.load_from_config();
        // Refresh sound setting
        self.sounds_enabled = Config::load()
            .map(|c| c.general.play_sounds)
            .unwrap_or(true);
        self.state.view = GamesView::Menu;
        // Start background music if sounds are enabled
        if self.sounds_enabled {
            self.music.play(ChiptuneMelody::GameMenu);
        }
    }

    /// Start a game with stats tracking
    fn start_game_with_stats(&mut self) {
        // Stop menu music when starting a game
        self.music.stop();
        let game_type = self.state.selected_game_type();
        self.stats.on_game_start(game_type);
        self.state.start_game();
    }

    /// End a game with stats tracking
    fn end_game_with_stats(&mut self) {
        if let Some(game_type) = self.state.current_game {
            let score = self.state.score;
            let won = self.is_game_won(game_type);
            let level = self.get_game_level(game_type);
            self.stats.on_game_end(game_type, score, won, level);

            // Emit game over sound (won = success, lost = game over)
            if won {
                self.pending_sounds.push(SoundEvent::Success);
            } else {
                self.pending_sounds.push(SoundEvent::GameOver);
            }

            // Check achievements with final score
            self.achievements
                .check_all(&self.stats.stats, Some((game_type, score)));
        }
        self.state.game_over();
        self.save_to_config();
    }

    /// Return to menu and restart background music
    fn back_to_menu(&mut self) {
        self.state.return_to_menu();
        if self.sounds_enabled {
            self.music.play(ChiptuneMelody::GameMenu);
        }
    }

    /// Check if the current game was won
    fn is_game_won(&self, game: GameType) -> bool {
        match game {
            GameType::Tetris => false, // Endless
            GameType::Snake => false,  // Endless
            GameType::Breakout => self.state.breakout.is_game_won(),
            GameType::Rogue => self.state.rogue.is_game_won(),
            GameType::Trek => self.state.trek.is_game_won(),
            GameType::Clicker => false, // Endless
            GameType::Brainiac => self.state.brainiac.is_game_won(),
            GameType::Storyweaver => self.state.storyweaver.is_game_won(),
            GameType::DopeWars => self.state.dopewars.is_game_won(),
            GameType::Minesweeper => self.state.minesweeper.is_game_won(),
            GameType::Artillery => self.state.artillery.is_game_won(),
            GameType::Mindgames => self.state.mindgames.is_game_won(),
            GameType::Gumshoe => self.state.gumshoe.is_game_won(),
            GameType::Dungeon => self.state.dungeon.is_game_won(),
            GameType::Caverns => self.state.caverns.is_game_won(),
            GameType::Biolab => self.state.biolab.is_game_won(),
            GameType::Neondrive => self.state.neondrive.is_game_won(),
            GameType::Micropolis => self.state.micropolis.is_game_won(),
            GameType::JungleRun => self.state.junglerun.game_won,
            GameType::Adventure => self.state.adventure.game_won,
            GameType::Blackjack => false, // Casino - no win condition
            GameType::Roulette => false,  // Casino - no win condition
        }
    }

    /// Get the current level/floor for applicable games
    fn get_game_level(&self, game: GameType) -> Option<u32> {
        match game {
            GameType::Tetris => Some(self.state.tetris.get_level().unwrap_or(1)),
            GameType::Rogue => Some(self.state.rogue.get_level().unwrap_or(1)),
            GameType::Clicker => self.state.clicker.get_level(),
            GameType::Storyweaver => self.state.storyweaver.get_level(),
            _ => None,
        }
    }
}

impl Plugin for GamesPlugin {
    fn id(&self) -> &str {
        "games"
    }

    fn name(&self) -> &str {
        "Games"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true // Always available
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Games".to_string(),
            key: 'G',
            description: "Play retro games".to_string(),
            priority: 80,
        })
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        match key.code {
            KeyCode::Char('g') => {
                self.open_modal();
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            GamesView::Menu => match key.code {
                KeyCode::Esc => {
                    self.music.stop();
                    KeyHandleResult::CloseModal
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.state.select_prev();
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.state.select_next();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    self.start_game_with_stats();
                    KeyHandleResult::Handled
                }
                // Number keys 1-6 to directly select and start games
                KeyCode::Char('1') => {
                    self.state.selected_game = 0;
                    self.start_game_with_stats();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('2') => {
                    self.state.selected_game = 1;
                    self.start_game_with_stats();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('3') => {
                    self.state.selected_game = 2;
                    self.start_game_with_stats();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('4') => {
                    self.state.selected_game = 3;
                    self.start_game_with_stats();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('5') => {
                    self.state.selected_game = 4;
                    self.start_game_with_stats();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('6') => {
                    self.state.selected_game = 5;
                    self.start_game_with_stats();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('7') => {
                    self.state.selected_game = 6;
                    self.start_game_with_stats();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('8') => {
                    self.state.selected_game = 7;
                    self.start_game_with_stats();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('9') => {
                    self.state.selected_game = 8;
                    self.start_game_with_stats();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.state.show_leaderboard();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.state.show_stats();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.state.show_achievements();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            GamesView::Stats => match key.code {
                KeyCode::Esc => {
                    self.state.view = GamesView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            GamesView::Achievements => match key.code {
                KeyCode::Esc => {
                    self.state.view = GamesView::Menu;
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.state.achievements_scroll_offset =
                        self.state.achievements_scroll_offset.saturating_sub(1);
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.state.achievements_scroll_offset += 1;
                    KeyHandleResult::Handled
                }
                KeyCode::PageUp => {
                    self.state.achievements_scroll_offset =
                        self.state.achievements_scroll_offset.saturating_sub(10);
                    KeyHandleResult::Handled
                }
                KeyCode::PageDown => {
                    self.state.achievements_scroll_offset += 10;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            GamesView::Playing => {
                use platform::GameEngine;

                // Dispatch to game's handle_key implementation
                let result = match self.state.current_game {
                    Some(GameType::Tetris) => self.state.tetris.handle_key(key),
                    Some(GameType::Snake) => self.state.snake.handle_key(key),
                    Some(GameType::Breakout) => self.state.breakout.handle_key(key),
                    Some(GameType::Rogue) => self.state.rogue.handle_key(key),
                    Some(GameType::Trek) => self.state.trek.handle_key(key),
                    Some(GameType::Clicker) => self.state.clicker.handle_key(key),
                    Some(GameType::Brainiac) => self.state.brainiac.handle_key(key),
                    Some(GameType::Storyweaver) => self.state.storyweaver.handle_key(key),
                    Some(GameType::DopeWars) => self.state.dopewars.handle_key(key),
                    Some(GameType::Minesweeper) => self.state.minesweeper.handle_key(key),
                    Some(GameType::Artillery) => self.state.artillery.handle_key(key),
                    Some(GameType::Mindgames) => self.state.mindgames.handle_key(key),
                    Some(GameType::Gumshoe) => self.state.gumshoe.handle_key(key),
                    Some(GameType::Dungeon) => self.state.dungeon.handle_key(key),
                    Some(GameType::Caverns) => self.state.caverns.handle_key(key),
                    Some(GameType::Biolab) => self.state.biolab.handle_key(key),
                    Some(GameType::Neondrive) => self.state.neondrive.handle_key(key),
                    Some(GameType::Micropolis) => self.state.micropolis.handle_key(key),
                    Some(GameType::JungleRun) => self.state.junglerun.handle_key(key),
                    Some(GameType::Adventure) => self.state.adventure.handle_key(key),
                    Some(GameType::Blackjack) => self.state.blackjack.handle_key(key),
                    Some(GameType::Roulette) => self.state.roulette.handle_key(key),
                    None => platform::KeyHandleResult::NotHandled,
                };

                // Handle platform-level actions based on game's response
                match result {
                    platform::KeyHandleResult::RequestQuit => {
                        self.back_to_menu();
                        KeyHandleResult::Handled
                    }
                    platform::KeyHandleResult::RequestPause => {
                        self.state.toggle_pause();
                        KeyHandleResult::Handled
                    }
                    platform::KeyHandleResult::GameOver => {
                        self.end_game_with_stats();
                        KeyHandleResult::Handled
                    }
                    platform::KeyHandleResult::Handled | platform::KeyHandleResult::NotHandled => {
                        KeyHandleResult::Handled
                    }
                }
            }
            GamesView::Paused => match key.code {
                KeyCode::Esc => {
                    self.back_to_menu();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    self.state.toggle_pause();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            GamesView::GameOver => match key.code {
                KeyCode::Esc => {
                    self.save_to_config(); // Save leaderboards before leaving
                    self.back_to_menu();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    // Restart the same game
                    self.start_game_with_stats();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.state.show_leaderboard();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            GamesView::EnteringInitials => match key.code {
                KeyCode::Left => {
                    self.state.initials_cursor_left();
                    KeyHandleResult::Handled
                }
                KeyCode::Right => {
                    self.state.initials_cursor_right();
                    KeyHandleResult::Handled
                }
                KeyCode::Up => {
                    self.state.initials_next_char();
                    KeyHandleResult::Handled
                }
                KeyCode::Down => {
                    self.state.initials_prev_char();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    self.state.confirm_initials();
                    self.save_to_config(); // Persist leaderboard
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    self.state.cancel_initials();
                    KeyHandleResult::Handled
                }
                // Allow typing letters directly
                KeyCode::Char(c) if c.is_ascii_alphabetic() => {
                    let c = c.to_ascii_uppercase();
                    let mut chars: Vec<char> = self.state.initials_buffer.chars().collect();
                    if self.state.initials_cursor < 3 && self.state.initials_cursor < chars.len() {
                        chars[self.state.initials_cursor] = c;
                        self.state.initials_buffer = chars.into_iter().collect();
                        self.state.initials_cursor_right();
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            GamesView::Leaderboard => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.state.close_leaderboard();
                    KeyHandleResult::Handled
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.state.prev_leaderboard_game();
                    KeyHandleResult::Handled
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.state.next_leaderboard_game();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn tick(&mut self) {
        // Always tick achievement toasts - emit sound if new toast shown
        if self.achievements.tick() {
            self.pending_sounds.push(SoundEvent::Achievement);
        }

        // Always tick menu animation
        if self.state.view == GamesView::Menu {
            self.state.tick_menu();
            return;
        }

        if self.state.view != GamesView::Playing {
            return;
        }

        // Unified trait-based game tick dispatch
        match self.state.current_game {
            Some(GameType::Tetris) => {
                self.state.tetris.tick();
                self.state.score = self.state.tetris.get_score();
                if self.state.tetris.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Snake) => {
                self.state.snake.tick();
                self.state.score = self.state.snake.get_score();
                if self.state.snake.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Breakout) => {
                self.state.breakout.tick();
                self.state.score = self.state.breakout.get_score();
                if self.state.breakout.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Rogue) => {
                self.state.rogue.tick();
                self.state.score = self.state.rogue.get_score();
                if self.state.rogue.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Trek) => {
                self.state.trek.tick();
                self.state.score = self.state.trek.get_score();
                if self.state.trek.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Clicker) => {
                // Clicker has special death screen and auto-save logic
                let was_alive = !self.state.clicker.is_game_over();
                self.state.clicker.tick();
                self.state.score = self.state.clicker.get_score();

                // Clicker has its own death screen (ClickerView::Dead), so only transition
                // to the generic GameOver screen if NOT in the Dead view
                if self.state.clicker.is_game_over()
                    && self.state.clicker.view != clicker::ClickerView::Dead
                    && self.state.clicker.view != clicker::ClickerView::SoulShop
                {
                    self.state.clicker.show_death_screen();
                }

                // Track stats and auto-save when death happens
                if was_alive && self.state.clicker.is_game_over() {
                    let score = self.state.score;
                    let level = self.state.clicker.get_level();
                    self.stats
                        .on_game_end(GameType::Clicker, score, false, level);
                    self.save_to_config();
                }
            }
            Some(GameType::Brainiac) => {
                self.state.brainiac.tick();
                self.state.score = self.state.brainiac.get_score();
                // Brainiac handles its own game over state
            }
            Some(GameType::Storyweaver) => {
                self.state.storyweaver.tick();
                self.state.score = self.state.storyweaver.get_score();
                // Storyweaver handles its own game over state
            }
            Some(GameType::DopeWars) => {
                self.state.dopewars.tick();
                self.state.score = self.state.dopewars.get_score();
                if self.state.dopewars.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Minesweeper) => {
                self.state.minesweeper.tick();
                self.state.score = self.state.minesweeper.get_score();
                if self.state.minesweeper.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Artillery) => {
                self.state.artillery.tick();
                self.state.score = self.state.artillery.get_score();
                if self.state.artillery.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Mindgames) => {
                self.state.mindgames.tick();
                self.state.score = self.state.mindgames.get_score();
                if self.state.mindgames.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Gumshoe) => {
                self.state.gumshoe.tick();
                self.state.score = self.state.gumshoe.get_score();
                if self.state.gumshoe.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Dungeon) => {
                self.state.dungeon.tick();
                self.state.score = self.state.dungeon.get_score();
                if self.state.dungeon.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Caverns) => {
                self.state.caverns.tick();
                self.state.score = self.state.caverns.get_score();
                if self.state.caverns.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Biolab) => {
                self.state.biolab.tick();
                self.state.score = self.state.biolab.get_score();
                // Biolab handles its own game over state
            }
            Some(GameType::Neondrive) => {
                self.state.neondrive.tick();
                self.state.score = self.state.neondrive.get_score();
                if self.state.neondrive.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Micropolis) => {
                self.state.micropolis.tick();
                self.state.score = self.state.micropolis.get_score();
                if self.state.micropolis.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::JungleRun) => {
                self.state.junglerun.tick();
                self.state.score = self.state.junglerun.get_score();
                if self.state.junglerun.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Adventure) => {
                self.state.adventure.tick();
                self.state.score = self.state.adventure.get_score();
                if self.state.adventure.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Blackjack) => {
                self.state.blackjack.tick();
                self.state.score = self.state.blackjack.get_score();
                if self.state.blackjack.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            Some(GameType::Roulette) => {
                self.state.roulette.tick();
                self.state.score = self.state.roulette.get_score();
                if self.state.roulette.is_game_over() {
                    self.end_game_with_stats();
                }
            }
            None => {}
        }
    }

    fn drain_sound_events(&mut self) -> Vec<SoundEvent> {
        std::mem::take(&mut self.pending_sounds)
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_games_modal(
            frame,
            area,
            &self.state,
            &self.stats.stats,
            &self.achievements,
            self.stats.session_duration_secs(),
            colors,
        );

        // Render achievement toast on top if one is active
        if let Some(toast) = &self.achievements.current_toast {
            modal::achievements::draw_achievement_toast(frame, toast, colors);
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Games Plugin".to_string(),
            "".to_string(),
            "Play classic retro games right in R-DOS!".to_string(),
            "".to_string(),
            "Available Games:".to_string(),
            "  Tetris    - Stack falling blocks to clear lines".to_string(),
            "  Snake     - Eat food and grow without hitting yourself".to_string(),
            "  Breakout  - Bounce the ball to break all the bricks".to_string(),
            "  Rogue     - Explore dungeons and defeat monsters".to_string(),
            "  Star Trek - Command the Enterprise, destroy Klingons".to_string(),
            "  Clicker   - Kill monsters, gain gold, buy upgrades".to_string(),
            "  Brainiac  - AI trivia with age-adaptive questions".to_string(),
            "  Storyweaver - AI choose-your-own-adventure books".to_string(),
            "".to_string(),
            "Common Controls:".to_string(),
            "  P         Pause/Resume".to_string(),
            "  Esc       Quit to menu".to_string(),
            "".to_string(),
            "Tetris: ←→ move, ↑ rotate, ↓ drop, Space hard drop".to_string(),
            "Snake: ←↑↓→ direction".to_string(),
            "Breakout: ←→ paddle, Space launch".to_string(),
            "Rogue: ←↑↓→/hjkl move, yubn diagonal, s search, > stairs".to_string(),
            "Star Trek: NSLPTHCD commands (see in-game help)".to_string(),
            "Clicker: h/Space hit, e eat, b shop, s soul shop, > stairs, w save".to_string(),
            "Brainiac: ↑↓ select, Enter answer, 1-4 quick answer".to_string(),
            "Storyweaver: ↑↓ navigate, Enter choose, A-D quick choice, Space skip".to_string(),
            "".to_string(),
            "Clicker Soul System:".to_string(),
            "  Die to earn Souls based on progress".to_string(),
            "  Press 's' to open Soul Shop for permanent upgrades".to_string(),
            "  Gold scales exponentially by floor (×1.5 per level)".to_string(),
            "  Upgrades persist across runs - true incremental!".to_string(),
            "".to_string(),
            "Clicker Save System:".to_string(),
            "  Press 'w' to manually save your game".to_string(),
            "  Auto-saves on death, prestige, and soul upgrades".to_string(),
            "  Your full game state persists between sessions!".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Games".to_string(),
            description: "Retro games (Tetris, Snake, Breakout, Rogue, Trek, Clicker)".to_string(),
            category: PluginCategory::Games,
            key: 'G',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.open_modal();
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
