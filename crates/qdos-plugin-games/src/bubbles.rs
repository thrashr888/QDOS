//! Bubbles game implementation
//!
//! Classic bubble shooter - aim and shoot colored bubbles to match 3+ and pop them.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;

pub const BOARD_WIDTH: usize = 15;
pub const BOARD_HEIGHT: usize = 18;
pub const COLORS: usize = 5;

/// Bubble color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BubbleColor {
    Red,
    Blue,
    Green,
    Yellow,
    Purple,
    Empty,
}

impl BubbleColor {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..COLORS) {
            0 => BubbleColor::Red,
            1 => BubbleColor::Blue,
            2 => BubbleColor::Green,
            3 => BubbleColor::Yellow,
            _ => BubbleColor::Purple,
        }
    }

    pub fn to_char(self) -> char {
        match self {
            BubbleColor::Red => 'R',
            BubbleColor::Blue => 'B',
            BubbleColor::Green => 'G',
            BubbleColor::Yellow => 'Y',
            BubbleColor::Purple => 'P',
            BubbleColor::Empty => ' ',
        }
    }
}

/// Flying bubble state
#[derive(Debug, Clone)]
pub struct FlyingBubble {
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
    pub color: BubbleColor,
}

/// Bubbles game state
pub struct BubblesState {
    pub board: [[BubbleColor; BOARD_WIDTH]; BOARD_HEIGHT],
    pub shooter_angle: f32, // -60 to 60 degrees from vertical
    pub current_bubble: BubbleColor,
    pub next_bubble: BubbleColor,
    pub flying_bubble: Option<FlyingBubble>,
    pub score: u32,
    pub game_over: bool,
    pub game_won: bool,
    pending_events: Vec<GameEvent>,
}

impl Default for BubblesState {
    fn default() -> Self {
        Self::new()
    }
}

impl BubblesState {
    pub fn new() -> Self {
        let mut state = Self {
            board: [[BubbleColor::Empty; BOARD_WIDTH]; BOARD_HEIGHT],
            shooter_angle: 0.0,
            current_bubble: BubbleColor::random(),
            next_bubble: BubbleColor::random(),
            flying_bubble: None,
            score: 0,
            game_over: false,
            game_won: false,
            pending_events: Vec::new(),
        };
        state.reset();
        state
    }

    pub fn reset(&mut self) {
        // Fill top rows with random bubbles
        for y in 0..6 {
            for x in 0..BOARD_WIDTH {
                // Offset odd rows
                let offset = if y % 2 == 1 { 1 } else { 0 };
                if x + offset < BOARD_WIDTH {
                    self.board[y][x] = BubbleColor::random();
                }
            }
        }
        for y in 6..BOARD_HEIGHT {
            for x in 0..BOARD_WIDTH {
                self.board[y][x] = BubbleColor::Empty;
            }
        }

        self.shooter_angle = 0.0;
        self.current_bubble = BubbleColor::random();
        self.next_bubble = BubbleColor::random();
        self.flying_bubble = None;
        self.score = 0;
        self.game_over = false;
        self.game_won = false;
        self.pending_events.clear();
        self.pending_events.push(GameEvent::GameStarted);
    }

    pub fn aim_left(&mut self) {
        self.shooter_angle = (self.shooter_angle - 5.0).max(-75.0);
    }

    pub fn aim_right(&mut self) {
        self.shooter_angle = (self.shooter_angle + 5.0).min(75.0);
    }

    pub fn shoot(&mut self) {
        if self.flying_bubble.is_some() || self.game_over {
            return;
        }

        let angle_rad = self.shooter_angle.to_radians();
        let speed = 0.8;

        self.flying_bubble = Some(FlyingBubble {
            x: (BOARD_WIDTH / 2) as f32,
            y: (BOARD_HEIGHT - 1) as f32,
            dx: angle_rad.sin() * speed,
            dy: -angle_rad.cos() * speed,
            color: self.current_bubble,
        });

        self.current_bubble = self.next_bubble;
        self.next_bubble = BubbleColor::random();
    }

    fn find_matches(&self, start_x: usize, start_y: usize) -> Vec<(usize, usize)> {
        let color = self.board[start_y][start_x];
        if color == BubbleColor::Empty {
            return vec![];
        }

        let mut matches = vec![];
        let mut visited = [[false; BOARD_WIDTH]; BOARD_HEIGHT];
        let mut stack = vec![(start_x, start_y)];

        while let Some((x, y)) = stack.pop() {
            if visited[y][x] || self.board[y][x] != color {
                continue;
            }
            visited[y][x] = true;
            matches.push((x, y));

            // Check neighbors (hex grid pattern)
            let neighbors = self.get_neighbors(x, y);
            for (nx, ny) in neighbors {
                if !visited[ny][nx] && self.board[ny][nx] == color {
                    stack.push((nx, ny));
                }
            }
        }

        matches
    }

    fn get_neighbors(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let mut neighbors = vec![];
        let offsets: &[(i32, i32)] = if y.is_multiple_of(2) {
            &[(-1, 0), (1, 0), (-1, -1), (0, -1), (-1, 1), (0, 1)]
        } else {
            &[(-1, 0), (1, 0), (0, -1), (1, -1), (0, 1), (1, 1)]
        };

        for (dx, dy) in offsets {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx >= 0 && nx < BOARD_WIDTH as i32 && ny >= 0 && ny < BOARD_HEIGHT as i32 {
                neighbors.push((nx as usize, ny as usize));
            }
        }
        neighbors
    }

    fn remove_floating(&mut self) -> u32 {
        // Find all bubbles connected to top row
        let mut connected = [[false; BOARD_WIDTH]; BOARD_HEIGHT];
        let mut stack: Vec<(usize, usize)> = vec![];

        // Start from top row
        for x in 0..BOARD_WIDTH {
            if self.board[0][x] != BubbleColor::Empty {
                stack.push((x, 0));
            }
        }

        while let Some((x, y)) = stack.pop() {
            if connected[y][x] || self.board[y][x] == BubbleColor::Empty {
                continue;
            }
            connected[y][x] = true;

            for (nx, ny) in self.get_neighbors(x, y) {
                if !connected[ny][nx] && self.board[ny][nx] != BubbleColor::Empty {
                    stack.push((nx, ny));
                }
            }
        }

        // Remove unconnected bubbles
        let mut removed = 0;
        for (y, row) in connected.iter().enumerate() {
            for (x, &is_connected) in row.iter().enumerate() {
                if self.board[y][x] != BubbleColor::Empty && !is_connected {
                    self.board[y][x] = BubbleColor::Empty;
                    removed += 1;
                }
            }
        }
        removed
    }

    fn check_game_over(&mut self) {
        // Game over if bubbles reach bottom rows
        for x in 0..BOARD_WIDTH {
            if self.board[BOARD_HEIGHT - 3][x] != BubbleColor::Empty {
                self.game_over = true;
                self.pending_events
                    .push(GameEvent::GameEnded { won: false });
                return;
            }
        }

        // Check win - all bubbles cleared
        let mut has_bubbles = false;
        for y in 0..BOARD_HEIGHT {
            for x in 0..BOARD_WIDTH {
                if self.board[y][x] != BubbleColor::Empty {
                    has_bubbles = true;
                    break;
                }
            }
        }

        if !has_bubbles {
            self.game_won = true;
            self.game_over = true;
            self.pending_events.push(GameEvent::GameEnded { won: true });
        }
    }
}

impl GameEngine for BubblesState {
    fn tick(&mut self) {
        if self.game_over {
            return;
        }

        // Extract bubble info to avoid borrow issues
        let bubble_info = self
            .flying_bubble
            .as_ref()
            .map(|b| (b.x, b.y, b.dx, b.dy, b.color));

        if let Some((mut x, mut y, mut dx, dy, color)) = bubble_info {
            // Move bubble
            x += dx;
            y += dy;

            // Bounce off walls
            if x <= 0.0 || x >= (BOARD_WIDTH - 1) as f32 {
                dx = -dx;
                x = x.clamp(0.0, (BOARD_WIDTH - 1) as f32);
            }

            // Update flying bubble position
            if let Some(ref mut bubble) = self.flying_bubble {
                bubble.x = x;
                bubble.y = y;
                bubble.dx = dx;
            }

            // Check collision with board
            let bx = x.round() as usize;
            let by = y.round() as usize;

            let should_land = by == 0
                || (by < BOARD_HEIGHT && bx < BOARD_WIDTH && {
                    // Check if adjacent to existing bubble
                    self.get_neighbors(bx, by)
                        .iter()
                        .any(|&(nx, ny)| self.board[ny][nx] != BubbleColor::Empty)
                });

            if should_land && by < BOARD_HEIGHT && bx < BOARD_WIDTH {
                // Land the bubble
                self.board[by][bx] = color;
                self.flying_bubble = None;

                // Check for matches
                let matches = self.find_matches(bx, by);
                if matches.len() >= 3 {
                    for (mx, my) in &matches {
                        self.board[*my][*mx] = BubbleColor::Empty;
                    }
                    let old_score = self.score;
                    self.score += (matches.len() as u32) * 10;
                    self.pending_events.push(GameEvent::ScoreChanged {
                        old: old_score,
                        new: self.score,
                    });

                    // Remove floating bubbles
                    let floating = self.remove_floating();
                    if floating > 0 {
                        let old_score = self.score;
                        self.score += floating * 20;
                        self.pending_events.push(GameEvent::ScoreChanged {
                            old: old_score,
                            new: self.score,
                        });
                    }
                }

                self.check_game_over();
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        if self.game_over {
            if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                self.reset();
                return KeyHandleResult::Handled;
            }
            return KeyHandleResult::NotHandled;
        }

        match key.code {
            KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('h') => {
                self.aim_left();
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('l') => {
                self.aim_right();
                KeyHandleResult::Handled
            }
            KeyCode::Char(' ') | KeyCode::Up | KeyCode::Enter => {
                self.shoot();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn get_score(&self) -> u32 {
        self.score
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }
}
