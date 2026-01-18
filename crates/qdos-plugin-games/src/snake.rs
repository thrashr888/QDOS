//! Snake game implementation

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;

pub const BOARD_WIDTH: usize = 40;
pub const BOARD_HEIGHT: usize = 18;

/// Direction of movement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

/// A position on the board
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn moved(&self, direction: Direction) -> Self {
        match direction {
            Direction::Up => Position::new(self.x, self.y - 1),
            Direction::Down => Position::new(self.x, self.y + 1),
            Direction::Left => Position::new(self.x - 1, self.y),
            Direction::Right => Position::new(self.x + 1, self.y),
        }
    }
}

/// Snake game state
pub struct SnakeState {
    pub body: Vec<Position>,
    pub direction: Direction,
    pub next_direction: Direction,
    pub food: Position,
    pub score: u32,
    pub game_over: bool,
    pub tick_count: u32,
    pending_events: Vec<GameEvent>,
}

impl Default for SnakeState {
    fn default() -> Self {
        Self::new()
    }
}

impl SnakeState {
    pub fn new() -> Self {
        let mut state = Self {
            body: Vec::new(),
            direction: Direction::Right,
            next_direction: Direction::Right,
            food: Position::new(0, 0),
            score: 0,
            game_over: false,
            tick_count: 0,
            pending_events: Vec::new(),
        };
        state.reset();
        state
    }

    pub fn reset(&mut self) {
        // Start in the middle
        let start_x = (BOARD_WIDTH / 2) as i32;
        let start_y = (BOARD_HEIGHT / 2) as i32;

        self.body = vec![
            Position::new(start_x, start_y),
            Position::new(start_x - 1, start_y),
            Position::new(start_x - 2, start_y),
        ];

        self.direction = Direction::Right;
        self.next_direction = Direction::Right;
        self.score = 0;
        self.game_over = false;
        self.tick_count = 0;
        self.pending_events.clear();
        self.spawn_food();
        self.pending_events.push(GameEvent::GameStarted);
    }

    fn spawn_food(&mut self) {
        let mut rng = rand::thread_rng();
        loop {
            let x = rng.gen_range(0..BOARD_WIDTH as i32);
            let y = rng.gen_range(0..BOARD_HEIGHT as i32);
            let pos = Position::new(x, y);

            // Make sure food doesn't spawn on snake
            if !self.body.contains(&pos) {
                self.food = pos;
                break;
            }
        }
    }

    pub fn set_direction(&mut self, direction: Direction) {
        // Can't reverse direction
        if direction != self.direction.opposite() {
            self.next_direction = direction;
        }
    }

    fn move_snake(&mut self) {
        self.direction = self.next_direction;

        let head = self.body[0];
        let new_head = head.moved(self.direction);

        // Check wall collision
        if new_head.x < 0
            || new_head.x >= BOARD_WIDTH as i32
            || new_head.y < 0
            || new_head.y >= BOARD_HEIGHT as i32
        {
            self.game_over = true;
            return;
        }

        // Check self collision
        if self.body.contains(&new_head) {
            self.game_over = true;
            return;
        }

        // Move head
        self.body.insert(0, new_head);

        // Check food
        if new_head == self.food {
            let old_score = self.score;
            self.score += 10;
            self.spawn_food();
            // Don't remove tail - snake grows
            self.pending_events.push(GameEvent::FoodEaten);
            self.pending_events.push(GameEvent::ScoreChanged {
                old: old_score,
                new: self.score,
            });
        } else {
            // Remove tail
            self.body.pop();
        }
    }

    /// Check if a position is part of the snake body
    pub fn is_snake(&self, pos: Position) -> bool {
        self.body.contains(&pos)
    }

    /// Check if a position is the head
    pub fn is_head(&self, pos: Position) -> bool {
        self.body.first() == Some(&pos)
    }
}

// === GameEngine Implementation ===

impl GameEngine for SnakeState {
    fn tick(&mut self) {
        if self.game_over {
            return;
        }

        self.tick_count += 1;

        // Speed based on score
        let move_interval = 3_u32.saturating_sub(self.score / 50).max(1);

        if self.tick_count >= move_interval {
            self.tick_count = 0;
            self.move_snake();
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.set_direction(Direction::Up);
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.set_direction(Direction::Down);
                KeyHandleResult::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.set_direction(Direction::Left);
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.set_direction(Direction::Right);
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

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn get_stat(&self, key: &str) -> Option<u64> {
        match key {
            "food_eaten" => Some((self.score / 10) as u64),
            "length" => Some(self.body.len() as u64),
            _ => None,
        }
    }
}
