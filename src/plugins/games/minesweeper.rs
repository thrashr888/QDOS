use super::platform::{GameEngine, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;
use serde::{Deserialize, Serialize};

const GRID_WIDTH: usize = 16;
const GRID_HEIGHT: usize = 16;
const MINE_COUNT: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellState {
    Hidden,
    Revealed,
    Flagged,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cell {
    pub is_mine: bool,
    pub adjacent_mines: u8,
    pub state: CellState,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            is_mine: false,
            adjacent_mines: 0,
            state: CellState::Hidden,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinesweeperState {
    pub grid: Vec<Vec<Cell>>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub game_over: bool,
    pub game_won: bool,
    pub first_click: bool,
    pub flags_placed: usize,
    pub cells_revealed: usize,
    pub time_elapsed: u32, // seconds
    tick_count: u32,
}

impl Default for MinesweeperState {
    fn default() -> Self {
        Self::new()
    }
}

impl MinesweeperState {
    pub fn new() -> Self {
        Self {
            grid: vec![vec![Cell::default(); GRID_WIDTH]; GRID_HEIGHT],
            cursor_x: GRID_WIDTH / 2,
            cursor_y: GRID_HEIGHT / 2,
            game_over: false,
            game_won: false,
            first_click: true,
            flags_placed: 0,
            cells_revealed: 0,
            time_elapsed: 0,
            tick_count: 0,
        }
    }

    fn place_mines(&mut self, safe_x: usize, safe_y: usize) {
        let mut rng = rand::thread_rng();
        let mut mines_placed = 0;

        while mines_placed < MINE_COUNT {
            let x = rng.gen_range(0..GRID_WIDTH);
            let y = rng.gen_range(0..GRID_HEIGHT);

            // Don't place mine on first click or adjacent cells
            if (x == safe_x && y == safe_y) || (x.abs_diff(safe_x) <= 1 && y.abs_diff(safe_y) <= 1)
            {
                continue;
            }

            if !self.grid[y][x].is_mine {
                self.grid[y][x].is_mine = true;
                mines_placed += 1;
            }
        }

        // Calculate adjacent mine counts
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                if !self.grid[y][x].is_mine {
                    self.grid[y][x].adjacent_mines = self.count_adjacent_mines(x, y);
                }
            }
        }
    }

    fn count_adjacent_mines(&self, x: usize, y: usize) -> u8 {
        let mut count = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && nx < GRID_WIDTH as i32 && ny >= 0 && ny < GRID_HEIGHT as i32
                    && self.grid[ny as usize][nx as usize].is_mine {
                        count += 1;
                    }
            }
        }
        count
    }

    pub fn reveal_cell(&mut self, x: usize, y: usize) {
        if self.game_over || self.game_won {
            return;
        }

        let cell = &self.grid[y][x];
        if cell.state != CellState::Hidden {
            return;
        }

        // Place mines on first click
        if self.first_click {
            self.place_mines(x, y);
            self.first_click = false;
        }

        // Reveal the cell
        self.grid[y][x].state = CellState::Revealed;
        self.cells_revealed += 1;

        // Check if hit mine
        if self.grid[y][x].is_mine {
            self.game_over = true;
            self.reveal_all_mines();
            return;
        }

        // If no adjacent mines, flood fill reveal
        if self.grid[y][x].adjacent_mines == 0 {
            self.flood_fill(x, y);
        }

        // Check win condition
        let total_cells = GRID_WIDTH * GRID_HEIGHT;
        if self.cells_revealed == total_cells - MINE_COUNT {
            self.game_won = true;
            self.game_over = true;
        }
    }

    fn flood_fill(&mut self, x: usize, y: usize) {
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && nx < GRID_WIDTH as i32 && ny >= 0 && ny < GRID_HEIGHT as i32 {
                    let nx = nx as usize;
                    let ny = ny as usize;
                    if self.grid[ny][nx].state == CellState::Hidden && !self.grid[ny][nx].is_mine {
                        self.grid[ny][nx].state = CellState::Revealed;
                        self.cells_revealed += 1;
                        if self.grid[ny][nx].adjacent_mines == 0 {
                            self.flood_fill(nx, ny);
                        }
                    }
                }
            }
        }
    }

    pub fn toggle_flag(&mut self) {
        if self.game_over || self.game_won {
            return;
        }

        let cell = &mut self.grid[self.cursor_y][self.cursor_x];
        match cell.state {
            CellState::Hidden => {
                if self.flags_placed < MINE_COUNT {
                    cell.state = CellState::Flagged;
                    self.flags_placed += 1;
                }
            }
            CellState::Flagged => {
                cell.state = CellState::Hidden;
                self.flags_placed -= 1;
            }
            CellState::Revealed => {}
        }
    }

    fn reveal_all_mines(&mut self) {
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                if self.grid[y][x].is_mine {
                    self.grid[y][x].state = CellState::Revealed;
                }
            }
        }
    }

    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let new_x = (self.cursor_x as i32 + dx)
            .max(0)
            .min((GRID_WIDTH - 1) as i32) as usize;
        let new_y = (self.cursor_y as i32 + dy)
            .max(0)
            .min((GRID_HEIGHT - 1) as i32) as usize;
        self.cursor_x = new_x;
        self.cursor_y = new_y;
    }
}

impl GameEngine for MinesweeperState {
    fn tick(&mut self) {
        if !self.game_over && !self.game_won && !self.first_click {
            self.tick_count += 1;
            if self.tick_count >= 10 {
                // ~10Hz tick rate = 1 second
                self.time_elapsed += 1;
                self.tick_count = 0;
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        if self.game_over || self.game_won {
            match key.code {
                KeyCode::Esc => return KeyHandleResult::RequestQuit,
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    *self = Self::new();
                    return KeyHandleResult::Handled;
                }
                _ => return KeyHandleResult::NotHandled,
            }
        }

        match key.code {
            KeyCode::Esc => KeyHandleResult::RequestQuit,
            KeyCode::Char('p') | KeyCode::Char('P') => KeyHandleResult::RequestPause,
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_cursor(0, -1);
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_cursor(0, 1);
                KeyHandleResult::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.move_cursor(-1, 0);
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.move_cursor(1, 0);
                KeyHandleResult::Handled
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.reveal_cell(self.cursor_x, self.cursor_y);
                if self.game_over {
                    return KeyHandleResult::GameOver;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.toggle_flag();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn get_score(&self) -> u32 {
        if self.game_won {
            // Score based on time: faster is better
            // Max score 10000, decreases with time
            10000u32.saturating_sub(self.time_elapsed * 10)
        } else {
            0
        }
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn is_game_won(&self) -> bool {
        self.game_won
    }
}
