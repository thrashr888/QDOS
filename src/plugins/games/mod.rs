//! Games plugin
//!
//! Built-in retro games including Tetris, Snake, Breakout, Rogue, and Star Trek.

pub mod breakout;
mod modal;
pub mod rogue;
pub mod snake;
pub mod state;
pub mod tetris;
pub mod trek;

use crate::app::ThemeColors;
use crate::plugins::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{GameType, GamesState, GamesView};
use std::any::Any;
use std::path::PathBuf;

/// Games plugin - built-in retro games
pub struct GamesPlugin {
    pub state: GamesState,
}

impl Default for GamesPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl GamesPlugin {
    pub fn new() -> Self {
        Self {
            state: GamesState::new(),
        }
    }

    pub fn open_modal(&mut self) {
        self.state = GamesState::new();
        self.state.view = GamesView::Menu;
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
                KeyCode::Esc => KeyHandleResult::CloseModal,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.state.select_prev();
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.state.select_next();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    self.state.start_game();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            GamesView::Playing => match self.state.current_game {
                Some(GameType::Tetris) => match key.code {
                    KeyCode::Esc => {
                        self.state.return_to_menu();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        self.state.toggle_pause();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Left => {
                        self.state.tetris.move_left();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Right => {
                        self.state.tetris.move_right();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Up => {
                        self.state.tetris.rotate();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Down => {
                        self.state.tetris.soft_drop();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char(' ') => {
                        self.state.tetris.hard_drop();
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                },
                Some(GameType::Snake) => match key.code {
                    KeyCode::Esc => {
                        self.state.return_to_menu();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        self.state.toggle_pause();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Up => {
                        self.state.snake.set_direction(snake::Direction::Up);
                        KeyHandleResult::Handled
                    }
                    KeyCode::Down => {
                        self.state.snake.set_direction(snake::Direction::Down);
                        KeyHandleResult::Handled
                    }
                    KeyCode::Left => {
                        self.state.snake.set_direction(snake::Direction::Left);
                        KeyHandleResult::Handled
                    }
                    KeyCode::Right => {
                        self.state.snake.set_direction(snake::Direction::Right);
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                },
                Some(GameType::Breakout) => match key.code {
                    KeyCode::Esc => {
                        self.state.return_to_menu();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        self.state.toggle_pause();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Left => {
                        self.state.breakout.move_paddle_left();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Right => {
                        self.state.breakout.move_paddle_right();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char(' ') => {
                        self.state.breakout.launch_ball();
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                },
                Some(GameType::Rogue) => match key.code {
                    KeyCode::Esc => {
                        self.state.return_to_menu();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        self.state.toggle_pause();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.state.rogue.move_player(0, -1);
                        KeyHandleResult::Handled
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.state.rogue.move_player(0, 1);
                        KeyHandleResult::Handled
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.state.rogue.move_player(-1, 0);
                        KeyHandleResult::Handled
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        self.state.rogue.move_player(1, 0);
                        KeyHandleResult::Handled
                    }
                    // Diagonal movement
                    KeyCode::Char('y') => {
                        self.state.rogue.move_player(-1, -1);
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char('u') => {
                        self.state.rogue.move_player(1, -1);
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char('b') => {
                        self.state.rogue.move_player(-1, 1);
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char('n') => {
                        self.state.rogue.move_player(1, 1);
                        KeyHandleResult::Handled
                    }
                    // Search for traps
                    KeyCode::Char('s') => {
                        self.state.rogue.search();
                        KeyHandleResult::Handled
                    }
                    // Descend stairs (if on stairs)
                    KeyCode::Char('>') => {
                        self.state.rogue.move_player(0, 0); // Triggers stair check
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                },
                Some(GameType::Trek) => match key.code {
                    KeyCode::Esc => {
                        self.state.return_to_menu();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char(c) => {
                        self.state.trek.handle_key(c);
                        KeyHandleResult::Handled
                    }
                    KeyCode::Enter => {
                        self.state.trek.handle_key('\n');
                        KeyHandleResult::Handled
                    }
                    KeyCode::Backspace => {
                        self.state.trek.handle_key('\x7f');
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                },
                None => KeyHandleResult::Handled,
            },
            GamesView::Paused => match key.code {
                KeyCode::Esc => {
                    self.state.return_to_menu();
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
                    self.state.return_to_menu();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    // Restart the same game
                    self.state.start_game();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn tick(&mut self) {
        if self.state.view != GamesView::Playing {
            return;
        }

        match self.state.current_game {
            Some(GameType::Tetris) => {
                self.state.tetris.tick();
                self.state.score = self.state.tetris.score;
                if self.state.tetris.game_over {
                    self.state.game_over();
                }
            }
            Some(GameType::Snake) => {
                self.state.snake.tick();
                self.state.score = self.state.snake.score;
                if self.state.snake.game_over {
                    self.state.game_over();
                }
            }
            Some(GameType::Breakout) => {
                self.state.breakout.tick();
                self.state.score = self.state.breakout.score;
                if self.state.breakout.game_over || self.state.breakout.game_won {
                    self.state.game_over();
                }
            }
            Some(GameType::Rogue) => {
                self.state.rogue.tick();
                self.state.score = self.state.rogue.gold;
                if self.state.rogue.game_over || self.state.rogue.game_won {
                    self.state.game_over();
                }
            }
            Some(GameType::Trek) => {
                self.state.trek.tick();
                // Score based on energy remaining and klingons destroyed
                let klingons_destroyed = 10_i32.saturating_sub(self.state.trek.klingons_remaining);
                self.state.score = (klingons_destroyed * 100 + self.state.trek.energy / 10) as u32;
                if self.state.trek.game_over || self.state.trek.game_won {
                    self.state.game_over();
                }
            }
            None => {}
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_games_modal(frame, area, &self.state, colors);
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
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Games".to_string(),
            description: "Retro games (Tetris, Snake, Breakout, Rogue, Trek)".to_string(),
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
