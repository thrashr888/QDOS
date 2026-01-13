//! Breakout game implementation

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};

pub const BOARD_WIDTH: usize = 40;
pub const BOARD_HEIGHT: usize = 20;
pub const PADDLE_WIDTH: usize = 6;
pub const BRICK_ROWS: usize = 4;
pub const BRICKS_PER_ROW: usize = 10;

/// Ball state
#[derive(Debug, Clone, Copy)]
pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
}

impl Ball {
    pub fn new() -> Self {
        Self {
            x: (BOARD_WIDTH / 2) as f32,
            y: (BOARD_HEIGHT - 4) as f32,
            dx: 0.5,
            dy: -0.5,
        }
    }

    pub fn reset(&mut self, paddle_x: usize) {
        self.x = paddle_x as f32 + (PADDLE_WIDTH / 2) as f32;
        self.y = (BOARD_HEIGHT - 4) as f32;
        self.dx = if self.dx > 0.0 { 0.5 } else { -0.5 };
        self.dy = -0.5;
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x.round() as i32, self.y.round() as i32)
    }
}

impl Default for Ball {
    fn default() -> Self {
        Self::new()
    }
}

/// Breakout game state
pub struct BreakoutState {
    pub paddle_x: usize,
    pub ball: Ball,
    pub bricks: [[bool; BRICKS_PER_ROW]; BRICK_ROWS],
    pub score: u32,
    pub lives: u32,
    pub game_over: bool,
    pub game_won: bool,
    pub ball_launched: bool,
    pending_events: Vec<GameEvent>,
}

impl Default for BreakoutState {
    fn default() -> Self {
        Self::new()
    }
}

impl BreakoutState {
    pub fn new() -> Self {
        let mut state = Self {
            paddle_x: (BOARD_WIDTH - PADDLE_WIDTH) / 2,
            ball: Ball::new(),
            bricks: [[true; BRICKS_PER_ROW]; BRICK_ROWS],
            score: 0,
            lives: 3,
            game_over: false,
            game_won: false,
            ball_launched: false,
            pending_events: Vec::new(),
        };
        state.reset();
        state
    }

    pub fn reset(&mut self) {
        self.paddle_x = (BOARD_WIDTH - PADDLE_WIDTH) / 2;
        self.ball = Ball::new();
        self.bricks = [[true; BRICKS_PER_ROW]; BRICK_ROWS];
        self.score = 0;
        self.lives = 3;
        self.game_over = false;
        self.game_won = false;
        self.ball_launched = false;
        self.pending_events.clear();
        self.pending_events.push(GameEvent::GameStarted);
    }

    pub fn move_paddle_left(&mut self) {
        if self.paddle_x > 0 {
            self.paddle_x -= 1;
            if !self.ball_launched {
                self.ball.x = self.paddle_x as f32 + (PADDLE_WIDTH / 2) as f32;
            }
        }
    }

    pub fn move_paddle_right(&mut self) {
        if self.paddle_x + PADDLE_WIDTH < BOARD_WIDTH {
            self.paddle_x += 1;
            if !self.ball_launched {
                self.ball.x = self.paddle_x as f32 + (PADDLE_WIDTH / 2) as f32;
            }
        }
    }

    pub fn launch_ball(&mut self) {
        if !self.ball_launched && !self.game_over {
            self.ball_launched = true;
        }
    }

    fn count_bricks(&self) -> usize {
        self.bricks
            .iter()
            .flat_map(|row| row.iter())
            .filter(|&&b| b)
            .count()
    }

    /// Get paddle positions for rendering
    pub fn paddle_positions(&self) -> impl Iterator<Item = usize> {
        self.paddle_x..(self.paddle_x + PADDLE_WIDTH)
    }

    /// Check if there's a brick at the given position
    pub fn brick_at(&self, x: usize, y: usize) -> Option<usize> {
        let brick_start_y = 2;
        let brick_width = BOARD_WIDTH / BRICKS_PER_ROW;

        if y >= brick_start_y && y < brick_start_y + BRICK_ROWS {
            let row = y - brick_start_y;
            let col = x / brick_width;

            if col < BRICKS_PER_ROW && self.bricks[row][col] {
                return Some(row);
            }
        }
        None
    }
}

// === GameEngine Implementation ===

impl GameEngine for BreakoutState {
    fn tick(&mut self) {
        if self.game_over || self.game_won || !self.ball_launched {
            return;
        }

        // Move ball
        self.ball.x += self.ball.dx;
        self.ball.y += self.ball.dy;

        let (bx, by) = self.ball.pos();

        // Wall collisions
        if bx <= 0 || bx >= (BOARD_WIDTH - 1) as i32 {
            self.ball.dx = -self.ball.dx;
            self.ball.x = self.ball.x.clamp(0.0, (BOARD_WIDTH - 1) as f32);
        }
        if by <= 0 {
            self.ball.dy = -self.ball.dy;
            self.ball.y = 0.0;
        }

        // Bottom - lose life
        if by >= (BOARD_HEIGHT - 1) as i32 {
            self.lives -= 1;
            if self.lives == 0 {
                self.game_over = true;
                self.pending_events
                    .push(GameEvent::GameEnded { won: false });
            } else {
                self.ball_launched = false;
                self.ball.reset(self.paddle_x);
            }
            return;
        }

        // Paddle collision
        let paddle_y = (BOARD_HEIGHT - 2) as i32;
        if by == paddle_y
            && bx >= self.paddle_x as i32
            && bx < (self.paddle_x + PADDLE_WIDTH) as i32
        {
            self.ball.dy = -self.ball.dy.abs();

            // Angle based on where ball hits paddle
            let hit_pos = (bx - self.paddle_x as i32) as f32 / PADDLE_WIDTH as f32;
            self.ball.dx = (hit_pos - 0.5) * 1.0;
            if self.ball.dx.abs() < 0.2 {
                self.ball.dx = if self.ball.dx >= 0.0 { 0.2 } else { -0.2 };
            }
        }

        // Brick collision
        let brick_start_y = 2;
        let brick_width = BOARD_WIDTH / BRICKS_PER_ROW;

        if by >= brick_start_y as i32 && by < (brick_start_y + BRICK_ROWS) as i32 {
            let row = (by - brick_start_y as i32) as usize;
            let col = (bx as usize) / brick_width;

            if col < BRICKS_PER_ROW && self.bricks[row][col] {
                self.bricks[row][col] = false;
                self.ball.dy = -self.ball.dy;
                let old_score = self.score;
                self.score += 10 * (BRICK_ROWS - row) as u32;

                // Emit events
                self.pending_events.push(GameEvent::BrickDestroyed);
                self.pending_events.push(GameEvent::ScoreChanged {
                    old: old_score,
                    new: self.score,
                });

                // Check win
                if self.count_bricks() == 0 {
                    self.game_won = true;
                    self.pending_events.push(GameEvent::GameEnded { won: true });
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.move_paddle_left();
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.move_paddle_right();
                KeyHandleResult::Handled
            }
            KeyCode::Char(' ') => {
                self.launch_ball();
                KeyHandleResult::Handled
            }
            KeyCode::Char('p') | KeyCode::Char('P') => KeyHandleResult::RequestPause,
            KeyCode::Esc => KeyHandleResult::RequestQuit,
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn get_score(&self) -> u32 {
        self.score
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn is_game_won(&self) -> bool {
        self.game_won
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn get_stat(&self, key: &str) -> Option<u64> {
        match key {
            "bricks_destroyed" => {
                let total = BRICK_ROWS * BRICKS_PER_ROW;
                Some((total - self.count_bricks()) as u64)
            }
            "lives" => Some(self.lives as u64),
            _ => None,
        }
    }
}
