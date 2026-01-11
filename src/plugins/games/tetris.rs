//! Tetris game implementation

use rand::Rng;

pub const BOARD_WIDTH: usize = 10;
pub const BOARD_HEIGHT: usize = 20;

/// Tetromino piece types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceType {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl PieceType {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..7) {
            0 => PieceType::I,
            1 => PieceType::O,
            2 => PieceType::T,
            3 => PieceType::S,
            4 => PieceType::Z,
            5 => PieceType::J,
            _ => PieceType::L,
        }
    }

    /// Get the shape as offsets from center for each rotation state
    pub fn shape(&self, rotation: usize) -> &'static [(i32, i32)] {
        match self {
            PieceType::I => match rotation % 4 {
                0 => &[(0, -1), (0, 0), (0, 1), (0, 2)],
                1 => &[(-1, 0), (0, 0), (1, 0), (2, 0)],
                2 => &[(0, -1), (0, 0), (0, 1), (0, 2)],
                _ => &[(-1, 0), (0, 0), (1, 0), (2, 0)],
            },
            PieceType::O => &[(0, 0), (1, 0), (0, 1), (1, 1)],
            PieceType::T => match rotation % 4 {
                0 => &[(0, 0), (-1, 0), (1, 0), (0, -1)],
                1 => &[(0, 0), (0, -1), (0, 1), (1, 0)],
                2 => &[(0, 0), (-1, 0), (1, 0), (0, 1)],
                _ => &[(0, 0), (0, -1), (0, 1), (-1, 0)],
            },
            PieceType::S => match rotation % 2 {
                0 => &[(0, 0), (1, 0), (0, 1), (-1, 1)],
                _ => &[(0, 0), (0, -1), (1, 0), (1, 1)],
            },
            PieceType::Z => match rotation % 2 {
                0 => &[(0, 0), (-1, 0), (0, 1), (1, 1)],
                _ => &[(0, 0), (0, 1), (1, 0), (1, -1)],
            },
            PieceType::J => match rotation % 4 {
                0 => &[(0, 0), (-1, 0), (1, 0), (-1, -1)],
                1 => &[(0, 0), (0, -1), (0, 1), (1, -1)],
                2 => &[(0, 0), (-1, 0), (1, 0), (1, 1)],
                _ => &[(0, 0), (0, -1), (0, 1), (-1, 1)],
            },
            PieceType::L => match rotation % 4 {
                0 => &[(0, 0), (-1, 0), (1, 0), (1, -1)],
                1 => &[(0, 0), (0, -1), (0, 1), (1, 1)],
                2 => &[(0, 0), (-1, 0), (1, 0), (-1, 1)],
                _ => &[(0, 0), (0, -1), (0, 1), (-1, -1)],
            },
        }
    }

    pub fn char(&self) -> char {
        match self {
            PieceType::I => '█',
            PieceType::O => '█',
            PieceType::T => '█',
            PieceType::S => '█',
            PieceType::Z => '█',
            PieceType::J => '█',
            PieceType::L => '█',
        }
    }
}

/// Current falling piece
#[derive(Debug, Clone)]
pub struct Piece {
    pub piece_type: PieceType,
    pub x: i32,
    pub y: i32,
    pub rotation: usize,
}

impl Piece {
    pub fn new(piece_type: PieceType) -> Self {
        Self {
            piece_type,
            x: (BOARD_WIDTH / 2) as i32,
            y: 0,
            rotation: 0,
        }
    }

    pub fn blocks(&self) -> Vec<(i32, i32)> {
        self.piece_type
            .shape(self.rotation)
            .iter()
            .map(|(dx, dy)| (self.x + dx, self.y + dy))
            .collect()
    }
}

/// Tetris game state
pub struct TetrisState {
    pub board: [[Option<PieceType>; BOARD_WIDTH]; BOARD_HEIGHT],
    pub current_piece: Option<Piece>,
    pub next_piece: PieceType,
    pub score: u32,
    pub level: u32,
    pub lines_cleared: u32,
    pub game_over: bool,
    pub tick_count: u32,
}

impl Default for TetrisState {
    fn default() -> Self {
        Self::new()
    }
}

impl TetrisState {
    pub fn new() -> Self {
        Self {
            board: [[None; BOARD_WIDTH]; BOARD_HEIGHT],
            current_piece: None,
            next_piece: PieceType::random(),
            score: 0,
            level: 1,
            lines_cleared: 0,
            game_over: false,
            tick_count: 0,
        }
    }

    pub fn reset(&mut self) {
        self.board = [[None; BOARD_WIDTH]; BOARD_HEIGHT];
        self.current_piece = None;
        self.next_piece = PieceType::random();
        self.score = 0;
        self.level = 1;
        self.lines_cleared = 0;
        self.game_over = false;
        self.tick_count = 0;
        self.spawn_piece();
    }

    pub fn spawn_piece(&mut self) {
        let piece = Piece::new(self.next_piece);
        self.next_piece = PieceType::random();

        // Check if spawn position is valid
        if self.is_valid_position(&piece) {
            self.current_piece = Some(piece);
        } else {
            self.game_over = true;
        }
    }

    fn is_valid_position(&self, piece: &Piece) -> bool {
        for (x, y) in piece.blocks() {
            if x < 0 || x >= BOARD_WIDTH as i32 || y >= BOARD_HEIGHT as i32 {
                return false;
            }
            if y >= 0 && self.board[y as usize][x as usize].is_some() {
                return false;
            }
        }
        true
    }

    pub fn move_left(&mut self) {
        if let Some(piece) = &self.current_piece {
            let mut test_piece = piece.clone();
            test_piece.x -= 1;
            if self.is_valid_position(&test_piece) {
                self.current_piece.as_mut().unwrap().x -= 1;
            }
        }
    }

    pub fn move_right(&mut self) {
        if let Some(piece) = &self.current_piece {
            let mut test_piece = piece.clone();
            test_piece.x += 1;
            if self.is_valid_position(&test_piece) {
                self.current_piece.as_mut().unwrap().x += 1;
            }
        }
    }

    pub fn rotate(&mut self) {
        if let Some(piece) = &self.current_piece {
            let mut test_piece = piece.clone();
            test_piece.rotation = (test_piece.rotation + 1) % 4;
            if self.is_valid_position(&test_piece) {
                self.current_piece.as_mut().unwrap().rotation = test_piece.rotation;
            }
        }
    }

    pub fn soft_drop(&mut self) -> bool {
        if let Some(piece) = &self.current_piece {
            let mut test_piece = piece.clone();
            test_piece.y += 1;
            if self.is_valid_position(&test_piece) {
                self.current_piece.as_mut().unwrap().y += 1;
                return true;
            }
            return false;
        }
        false
    }

    pub fn hard_drop(&mut self) {
        while self.soft_drop() {}
        self.lock_piece();
    }

    pub fn lock_piece(&mut self) {
        if let Some(piece) = self.current_piece.take() {
            for (x, y) in piece.blocks() {
                if y >= 0 && y < BOARD_HEIGHT as i32 && x >= 0 && x < BOARD_WIDTH as i32 {
                    self.board[y as usize][x as usize] = Some(piece.piece_type);
                }
            }
            self.clear_lines();
            self.spawn_piece();
        }
    }

    fn clear_lines(&mut self) {
        let mut lines_to_clear = Vec::new();

        for y in 0..BOARD_HEIGHT {
            if self.board[y].iter().all(|cell| cell.is_some()) {
                lines_to_clear.push(y);
            }
        }

        let cleared = lines_to_clear.len() as u32;
        if cleared > 0 {
            // Remove cleared lines and shift down
            for &y in lines_to_clear.iter().rev() {
                for row in (1..=y).rev() {
                    self.board[row] = self.board[row - 1];
                }
                self.board[0] = [None; BOARD_WIDTH];
            }

            // Update score (classic scoring)
            self.lines_cleared += cleared;
            self.score += match cleared {
                1 => 100 * self.level,
                2 => 300 * self.level,
                3 => 500 * self.level,
                4 => 800 * self.level, // Tetris!
                _ => 0,
            };

            // Level up every 10 lines
            self.level = 1 + self.lines_cleared / 10;
        }
    }

    /// Called each game tick
    pub fn tick(&mut self) {
        if self.game_over {
            return;
        }

        self.tick_count += 1;

        // Drop speed based on level (ticks between drops)
        let drop_interval = (20_u32).saturating_sub(self.level * 2).max(1);

        if self.tick_count >= drop_interval {
            self.tick_count = 0;
            if !self.soft_drop() {
                self.lock_piece();
            }
        }
    }
}
