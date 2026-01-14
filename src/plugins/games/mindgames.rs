use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// Constants
const TOTAL_QUESTIONS: usize = 10;
const BASE_SCORE: u32 = 100;
const FEEDBACK_DURATION: u32 = 20; // 2 seconds at 10 ticks/sec

// View States
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MindgamesView {
    #[default]
    ModeSelect,
    Playing,
    Feedback,
    GameOver,
}

// Game Modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MindgamesMode {
    PatternMaster,
    MemoryMatrix,
    NumberNinja,
    DailyChallenge,
}

// Pattern Master Phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternPhase {
    ShowPattern,
    AnswerPrompt,
}

// Memory Matrix Phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPhase {
    Memorize,
    Recall,
}

// Pattern Types for Pattern Master
#[derive(Debug, Clone, Copy)]
enum PatternType {
    Arithmetic,     // +n pattern
    Geometric,      // ×n pattern
    Fibonacci,      // Fibonacci sequence
    Doubling,       // Double previous
    Squares,        // n²
    Primes,         // Prime numbers
    AlternatingOps, // Alternating operations
}

// Operation Types for Number Ninja
#[derive(Debug, Clone, Copy)]
enum Operation {
    Add,
    Sub,
    Mul,
}

// Main Game State
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindgamesState {
    // View control
    pub view: MindgamesView,
    pub mode: Option<MindgamesMode>,
    pub selected_mode: usize,

    // Game progress
    pub question_index: usize,
    pub total_questions: usize,
    pub correct_count: u32,
    pub score: u32,
    pub streak: u32,
    pub best_streak: u32,

    // Timing
    pub time_remaining: u32,
    pub tick_counter: u32,
    pub question_start_time: u32,

    // Feedback
    pub last_correct: bool,
    pub feedback_timer: u32,

    // Pattern Master state
    pub pattern_phase: PatternPhase,
    pub pattern_sequence: Vec<String>,
    pub pattern_choices: [String; 4],
    pub pattern_correct: usize,
    pub pattern_selected: usize,
    pub pattern_display_timer: u32,

    // Memory Matrix state
    pub memory_phase: MemoryPhase,
    pub memory_grid_size: (usize, usize),
    pub memory_filled_cells: Vec<(usize, usize)>,
    pub memory_player_cells: Vec<(usize, usize)>,
    pub memory_cursor: (usize, usize),
    pub memory_display_timer: u32,

    // Number Ninja state
    pub number_equation: String,
    pub number_choices: [i32; 4],
    pub number_correct: usize,
    pub number_selected: usize,

    // Daily Challenge
    pub daily_seed: u64,
    pub daily_date: String,
    pub daily_question_types: Vec<DailyQuestionType>,

    // RNG state
    #[serde(skip)]
    rng: Option<StdRng>,

    // Events
    #[serde(skip)]
    pending_events: Vec<GameEvent>,
}

// Daily Challenge Question Types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DailyQuestionType {
    Pattern,
    Memory,
    Number,
}

impl Default for MindgamesState {
    fn default() -> Self {
        Self::new()
    }
}

impl MindgamesState {
    pub fn new() -> Self {
        Self {
            view: MindgamesView::ModeSelect,
            mode: None,
            selected_mode: 0,
            question_index: 0,
            total_questions: TOTAL_QUESTIONS,
            correct_count: 0,
            score: 0,
            streak: 0,
            best_streak: 0,
            time_remaining: 15,
            tick_counter: 0,
            question_start_time: 0,
            last_correct: false,
            feedback_timer: 0,
            pattern_phase: PatternPhase::ShowPattern,
            pattern_sequence: Vec::new(),
            pattern_choices: [String::new(), String::new(), String::new(), String::new()],
            pattern_correct: 0,
            pattern_selected: 0,
            pattern_display_timer: 0,
            memory_phase: MemoryPhase::Memorize,
            memory_grid_size: (3, 3),
            memory_filled_cells: Vec::new(),
            memory_player_cells: Vec::new(),
            memory_cursor: (0, 0),
            memory_display_timer: 0,
            number_equation: String::new(),
            number_choices: [0, 0, 0, 0],
            number_correct: 0,
            number_selected: 0,
            daily_seed: 0,
            daily_date: String::new(),
            daily_question_types: Vec::new(),
            rng: None,
            pending_events: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    // Mode Selection
    pub fn mode_select_prev(&mut self) {
        if self.selected_mode > 0 {
            self.selected_mode -= 1;
        } else {
            self.selected_mode = 3; // Wrap to Daily Challenge
        }
    }

    pub fn mode_select_next(&mut self) {
        if self.selected_mode < 3 {
            self.selected_mode += 1;
        } else {
            self.selected_mode = 0; // Wrap to Pattern Master
        }
    }

    // Start Game
    pub fn start_game(&mut self) {
        self.mode = Some(match self.selected_mode {
            0 => MindgamesMode::PatternMaster,
            1 => MindgamesMode::MemoryMatrix,
            2 => MindgamesMode::NumberNinja,
            3 => MindgamesMode::DailyChallenge,
            _ => MindgamesMode::PatternMaster,
        });

        self.view = MindgamesView::Playing;
        self.question_index = 0;
        self.correct_count = 0;
        self.score = 0;
        self.streak = 0;
        self.best_streak = 0;

        // Initialize Daily Challenge if needed
        if matches!(self.mode, Some(MindgamesMode::DailyChallenge)) {
            self.init_daily_challenge();
        }

        self.generate_question();
    }

    // Daily Challenge Initialization
    fn init_daily_challenge(&mut self) {
        let (seed, date) = Self::get_daily_seed();
        self.daily_seed = seed;
        self.daily_date = date;
        self.rng = Some(StdRng::seed_from_u64(seed));

        // Generate question type sequence: 3 Pattern + 3 Memory + 4 Number
        let mut types = Vec::new();
        for _ in 0..3 {
            types.push(DailyQuestionType::Pattern);
        }
        for _ in 0..3 {
            types.push(DailyQuestionType::Memory);
        }
        for _ in 0..4 {
            types.push(DailyQuestionType::Number);
        }

        // Shuffle with seeded RNG
        if let Some(rng) = &mut self.rng {
            types.shuffle(rng);
        }

        self.daily_question_types = types;
    }

    fn get_daily_seed() -> (u64, String) {
        let now = Local::now();
        let date_str = now.format("%Y-%m-%d").to_string();

        // Simple hash: sum of byte values as seed
        let seed = date_str
            .bytes()
            .map(|b| b as u64)
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b));

        (seed, date_str)
    }

    // Question Generation
    fn generate_question(&mut self) {
        match self.mode {
            Some(MindgamesMode::PatternMaster) => self.generate_pattern_question(),
            Some(MindgamesMode::MemoryMatrix) => self.generate_memory_question(),
            Some(MindgamesMode::NumberNinja) => self.generate_number_question(),
            Some(MindgamesMode::DailyChallenge) => self.generate_daily_question(),
            None => {}
        }

        self.time_remaining = 15;
        self.question_start_time = 15;
    }

    fn generate_pattern_question(&mut self) {
        let difficulty = ((self.question_index / 2) + 1).min(5) as u32;
        let (sequence, _answer, choices, correct_idx) = if let Some(ref mut rng) = self.rng {
            Self::generate_pattern(difficulty, rng)
        } else {
            Self::generate_pattern(difficulty, &mut rand::thread_rng())
        };

        self.pattern_sequence = sequence;
        self.pattern_choices = choices;
        self.pattern_correct = correct_idx;
        self.pattern_selected = 0;
        self.pattern_phase = PatternPhase::ShowPattern;
        self.pattern_display_timer = 30; // 3 seconds
    }

    fn generate_memory_question(&mut self) {
        let difficulty = ((self.question_index / 2) + 1).min(5) as u32;
        let (rows, cols, cells) = if let Some(ref mut rng) = self.rng {
            Self::generate_memory_grid(difficulty, rng)
        } else {
            Self::generate_memory_grid(difficulty, &mut rand::thread_rng())
        };

        self.memory_grid_size = (rows, cols);
        self.memory_filled_cells = cells;
        self.memory_player_cells = Vec::new();
        self.memory_cursor = (rows / 2, cols / 2);
        self.memory_phase = MemoryPhase::Memorize;

        // Display time based on difficulty
        self.memory_display_timer = match difficulty {
            1 => 30, // 3 seconds
            2 => 35,
            3 => 40,
            4 => 45,
            _ => 50, // 5 seconds
        };
    }

    fn generate_number_question(&mut self) {
        let difficulty = ((self.question_index / 2) + 1).min(5) as u32;
        let (equation, _answer, choices, correct_idx) = if let Some(ref mut rng) = self.rng {
            Self::generate_equation(difficulty, rng)
        } else {
            Self::generate_equation(difficulty, &mut rand::thread_rng())
        };

        self.number_equation = equation;
        self.number_choices = choices;
        self.number_correct = correct_idx;
        self.number_selected = 0;
    }

    fn generate_daily_question(&mut self) {
        if self.question_index >= self.daily_question_types.len() {
            return;
        }

        match self.daily_question_types[self.question_index] {
            DailyQuestionType::Pattern => self.generate_pattern_question(),
            DailyQuestionType::Memory => self.generate_memory_question(),
            DailyQuestionType::Number => self.generate_number_question(),
        }
    }

    // Pattern Generation
    fn generate_pattern(
        difficulty: u32,
        rng: &mut dyn rand::RngCore,
    ) -> (Vec<String>, String, [String; 4], usize) {
        let pattern_type = Self::select_pattern_type(rng);
        let sequence = Self::create_sequence(pattern_type, difficulty);
        let answer = sequence.last().unwrap().clone();
        let visible_sequence = sequence[..sequence.len() - 1].to_vec();

        let distractors = Self::generate_pattern_distractors(&answer, rng);
        let (choices, correct_idx) = Self::shuffle_choices(answer.clone(), distractors, rng);

        (visible_sequence, answer, choices, correct_idx)
    }

    fn select_pattern_type(rng: &mut dyn rand::RngCore) -> PatternType {
        let types = [
            PatternType::Arithmetic,
            PatternType::Geometric,
            PatternType::Fibonacci,
            PatternType::Doubling,
            PatternType::Squares,
            PatternType::Primes,
            PatternType::AlternatingOps,
        ];
        types[rng.gen_range(0..types.len())]
    }

    fn create_sequence(pattern_type: PatternType, difficulty: u32) -> Vec<String> {
        match pattern_type {
            PatternType::Arithmetic => {
                let step = match difficulty {
                    1 => 2,
                    2 => 3,
                    3 => 5,
                    4 => 7,
                    _ => 11,
                };
                let start = 1;
                (0..5).map(|i| (start + i * step).to_string()).collect()
            }
            PatternType::Geometric => {
                let mult = match difficulty {
                    1 | 2 => 2,
                    3 | 4 => 3,
                    _ => 4,
                };
                let mut val = 2;
                (0..5)
                    .map(|_| {
                        let s = val.to_string();
                        val *= mult;
                        s
                    })
                    .collect()
            }
            PatternType::Fibonacci => {
                let (mut a, mut b) = (1, 1);
                let mut result = vec![a.to_string()];
                for _ in 0..4 {
                    result.push(b.to_string());
                    let next = a + b;
                    a = b;
                    b = next;
                }
                result
            }
            PatternType::Doubling => {
                let start = match difficulty {
                    1 => 3,
                    2 => 5,
                    _ => 7,
                };
                let mut val = start;
                (0..5)
                    .map(|_| {
                        let s = val.to_string();
                        val *= 2;
                        s
                    })
                    .collect()
            }
            PatternType::Squares => (1..=5).map(|i| (i * i).to_string()).collect(),
            PatternType::Primes => {
                let primes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29];
                let start = match difficulty {
                    1 => 0,
                    2 => 1,
                    3 => 2,
                    _ => 3,
                };
                primes[start..start + 5]
                    .iter()
                    .map(|p| p.to_string())
                    .collect()
            }
            PatternType::AlternatingOps => {
                let mut val = 1;
                let mut result = vec![val.to_string()];
                for i in 0..4 {
                    if i % 2 == 0 {
                        val += 2;
                    } else {
                        val -= 1;
                    }
                    result.push(val.to_string());
                }
                result
            }
        }
    }

    fn generate_pattern_distractors(answer: &str, _rng: &mut dyn rand::RngCore) -> [String; 3] {
        let answer_val: i32 = answer.parse().unwrap_or(0);
        [
            (answer_val + 1).to_string(),
            (answer_val - 1).to_string(),
            (answer_val * 2).to_string(),
        ]
    }

    fn shuffle_choices(
        answer: String,
        distractors: [String; 3],
        _rng: &mut dyn rand::RngCore,
    ) -> ([String; 4], usize) {
        let answer_clone = answer.clone();
        let mut choices = [
            answer,
            distractors[0].clone(),
            distractors[1].clone(),
            distractors[2].clone(),
        ];

        // Shuffle
        for i in (1..choices.len()).rev() {
            let j = _rng.gen_range(0..=i);
            choices.swap(i, j);
        }

        // Find correct index
        let correct_idx = choices.iter().position(|c| c == &answer_clone).unwrap();

        (
            [
                choices[0].clone(),
                choices[1].clone(),
                choices[2].clone(),
                choices[3].clone(),
            ],
            correct_idx,
        )
    }

    // Memory Grid Generation
    fn generate_memory_grid(
        difficulty: u32,
        rng: &mut dyn rand::RngCore,
    ) -> (usize, usize, Vec<(usize, usize)>) {
        let (rows, cols, cell_count) = match difficulty {
            1 => (3, 3, 4),
            2 => (3, 3, 5),
            3 => (4, 4, 6),
            4 => (4, 4, 8),
            _ => (5, 5, 10),
        };

        let mut cells = HashSet::new();
        while cells.len() < cell_count {
            let row = rng.gen_range(0..rows);
            let col = rng.gen_range(0..cols);
            cells.insert((row, col));
        }

        (rows, cols, cells.into_iter().collect())
    }

    // Number Ninja Generation
    fn generate_equation(
        difficulty: u32,
        rng: &mut dyn rand::RngCore,
    ) -> (String, i32, [i32; 4], usize) {
        let (op, max_num) = match difficulty {
            1 => (Operation::Add, 20),
            2 => (Operation::Sub, 50),
            3 => (Operation::Mul, 12),
            4 => (Operation::Add, 100),
            _ => {
                let ops = [Operation::Add, Operation::Sub, Operation::Mul];
                (ops[rng.gen_range(0..ops.len())], 20)
            }
        };

        let (a, b) = (rng.gen_range(1..=max_num), rng.gen_range(1..=max_num));
        let (equation, answer) = match op {
            Operation::Add => (format!("{} + {}", a, b), a + b),
            Operation::Sub => {
                let (big, small) = (a.max(b), a.min(b));
                (format!("{} - {}", big, small), big - small)
            }
            Operation::Mul => (format!("{} × {}", a, b), a * b),
        };

        let distractors = Self::generate_number_distractors(answer, rng);
        let (choices, correct_idx) = Self::shuffle_number_choices(answer, distractors, rng);

        (equation, answer, choices, correct_idx)
    }

    fn generate_number_distractors(answer: i32, _rng: &mut dyn rand::RngCore) -> [i32; 3] {
        [answer + 1, answer - 1, answer + 5]
    }

    fn shuffle_number_choices(
        answer: i32,
        distractors: [i32; 3],
        _rng: &mut dyn rand::RngCore,
    ) -> ([i32; 4], usize) {
        let mut choices = [answer, distractors[0], distractors[1], distractors[2]];

        for i in (1..choices.len()).rev() {
            let j = _rng.gen_range(0..=i);
            choices.swap(i, j);
        }

        let correct_idx = choices.iter().position(|c| *c == answer).unwrap();

        (
            [choices[0], choices[1], choices[2], choices[3]],
            correct_idx,
        )
    }

    // Check Answer
    pub fn check_answer(&mut self) {
        let correct = match self.mode {
            Some(MindgamesMode::PatternMaster) => self.pattern_selected == self.pattern_correct,
            Some(MindgamesMode::MemoryMatrix) => {
                let player_set: HashSet<_> = self.memory_player_cells.iter().cloned().collect();
                let filled_set: HashSet<_> = self.memory_filled_cells.iter().cloned().collect();
                player_set == filled_set
            }
            Some(MindgamesMode::NumberNinja) => self.number_selected == self.number_correct,
            Some(MindgamesMode::DailyChallenge) => {
                if self.question_index >= self.daily_question_types.len() {
                    return;
                }
                match self.daily_question_types[self.question_index] {
                    DailyQuestionType::Pattern => self.pattern_selected == self.pattern_correct,
                    DailyQuestionType::Memory => {
                        let player_set: HashSet<_> =
                            self.memory_player_cells.iter().cloned().collect();
                        let filled_set: HashSet<_> =
                            self.memory_filled_cells.iter().cloned().collect();
                        player_set == filled_set
                    }
                    DailyQuestionType::Number => self.number_selected == self.number_correct,
                }
            }
            None => false,
        };

        self.last_correct = correct;

        if correct {
            self.correct_count += 1;
            self.streak += 1;
            self.best_streak = self.best_streak.max(self.streak);

            let points = self.calculate_points();
            self.score += points;

            self.pending_events
                .push(GameEvent::QuestionAnswered { correct: true });
        } else {
            self.streak = 0;
            self.pending_events
                .push(GameEvent::QuestionAnswered { correct: false });
        }

        self.view = MindgamesView::Feedback;
        self.feedback_timer = FEEDBACK_DURATION;
    }

    fn calculate_points(&self) -> u32 {
        let mut points = BASE_SCORE;

        // Time bonus (mode-specific)
        points += match self.mode {
            Some(MindgamesMode::PatternMaster) => {
                if self.time_remaining > 10 {
                    50
                } else if self.time_remaining > 5 {
                    25
                } else {
                    0
                }
            }
            Some(MindgamesMode::MemoryMatrix) => 0, // No time bonus
            Some(MindgamesMode::NumberNinja) => {
                if self.time_remaining > 5 {
                    25
                } else {
                    0
                }
            }
            Some(MindgamesMode::DailyChallenge) => {
                if self.question_index < self.daily_question_types.len() {
                    match self.daily_question_types[self.question_index] {
                        DailyQuestionType::Pattern => {
                            if self.time_remaining > 10 {
                                50
                            } else if self.time_remaining > 5 {
                                25
                            } else {
                                0
                            }
                        }
                        DailyQuestionType::Memory => 0,
                        DailyQuestionType::Number => {
                            if self.time_remaining > 5 {
                                25
                            } else {
                                0
                            }
                        }
                    }
                } else {
                    0
                }
            }
            None => 0,
        };

        // Streak multiplier
        let multiplier = if self.streak >= 5 {
            2.0
        } else if self.streak >= 3 {
            1.5
        } else {
            1.0
        };

        (points as f64 * multiplier) as u32
    }

    // Next Question
    pub fn next_question(&mut self) {
        self.question_index += 1;

        if self.question_index >= self.total_questions {
            self.game_over();
        } else {
            self.view = MindgamesView::Playing;
            self.generate_question();
        }
    }

    // Game Over
    fn game_over(&mut self) {
        // Perfect game bonus
        if self.correct_count == self.total_questions as u32 {
            self.score *= 2;
        }

        self.view = MindgamesView::GameOver;
        let won = self.correct_count == self.total_questions as u32;
        self.pending_events.push(GameEvent::GameEnded { won });
    }

    pub fn final_score(&self) -> u32 {
        self.score
    }

    pub fn accuracy(&self) -> f32 {
        if self.total_questions == 0 {
            0.0
        } else {
            (self.correct_count as f32 / self.total_questions as f32) * 100.0
        }
    }
}

// GameEngine Trait Implementation
impl GameEngine for MindgamesState {
    fn tick(&mut self) {
        self.tick_counter += 1;

        match self.view {
            MindgamesView::Playing => {
                match self.mode {
                    Some(MindgamesMode::PatternMaster) => {
                        if self.pattern_phase == PatternPhase::ShowPattern {
                            if self.pattern_display_timer > 0 {
                                self.pattern_display_timer -= 1;
                            } else {
                                self.pattern_phase = PatternPhase::AnswerPrompt;
                            }
                        }
                    }
                    Some(MindgamesMode::MemoryMatrix) => {
                        if self.memory_phase == MemoryPhase::Memorize {
                            if self.memory_display_timer > 0 {
                                self.memory_display_timer -= 1;
                            } else {
                                self.memory_phase = MemoryPhase::Recall;
                            }
                        }
                    }
                    Some(MindgamesMode::DailyChallenge) => {
                        if self.question_index < self.daily_question_types.len() {
                            match self.daily_question_types[self.question_index] {
                                DailyQuestionType::Pattern => {
                                    if self.pattern_phase == PatternPhase::ShowPattern {
                                        if self.pattern_display_timer > 0 {
                                            self.pattern_display_timer -= 1;
                                        } else {
                                            self.pattern_phase = PatternPhase::AnswerPrompt;
                                        }
                                    }
                                }
                                DailyQuestionType::Memory => {
                                    if self.memory_phase == MemoryPhase::Memorize {
                                        if self.memory_display_timer > 0 {
                                            self.memory_display_timer -= 1;
                                        } else {
                                            self.memory_phase = MemoryPhase::Recall;
                                        }
                                    }
                                }
                                DailyQuestionType::Number => {}
                            }
                        }
                    }
                    _ => {}
                }

                // Timer countdown (10 ticks = 1 second)
                if self.tick_counter.is_multiple_of(10) && self.time_remaining > 0 {
                    self.time_remaining -= 1;
                }
            }
            MindgamesView::Feedback => {
                if self.feedback_timer > 0 {
                    self.feedback_timer -= 1;
                } else {
                    self.next_question();
                }
            }
            _ => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            MindgamesView::ModeSelect => match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.mode_select_prev();
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.mode_select_next();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.start_game();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::NotHandled,
            },
            MindgamesView::Playing => self.handle_playing_key(key),
            MindgamesView::Feedback => match key.code {
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled, // Auto-advances
            },
            MindgamesView::GameOver => match key.code {
                KeyCode::Enter | KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.view = MindgamesView::ModeSelect;
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::NotHandled,
            },
        }
    }

    fn get_score(&self) -> u32 {
        self.final_score()
    }

    fn is_game_over(&self) -> bool {
        self.view == MindgamesView::GameOver
    }

    fn is_game_won(&self) -> bool {
        self.view == MindgamesView::GameOver && self.correct_count == self.total_questions as u32
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

impl MindgamesState {
    fn handle_playing_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.mode {
            Some(MindgamesMode::PatternMaster) => self.handle_pattern_key(key),
            Some(MindgamesMode::MemoryMatrix) => self.handle_memory_key(key),
            Some(MindgamesMode::NumberNinja) => self.handle_number_key(key),
            Some(MindgamesMode::DailyChallenge) => self.handle_daily_key(key),
            None => KeyHandleResult::NotHandled,
        }
    }

    fn handle_pattern_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        if self.pattern_phase == PatternPhase::ShowPattern {
            return match key.code {
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            };
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if self.pattern_selected > 0 {
                    self.pattern_selected -= 1;
                } else {
                    self.pattern_selected = 3;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.pattern_selected = (self.pattern_selected + 1) % 4;
                KeyHandleResult::Handled
            }
            KeyCode::Char('1') => {
                self.pattern_selected = 0;
                self.check_answer();
                KeyHandleResult::Handled
            }
            KeyCode::Char('2') => {
                self.pattern_selected = 1;
                self.check_answer();
                KeyHandleResult::Handled
            }
            KeyCode::Char('3') => {
                self.pattern_selected = 2;
                self.check_answer();
                KeyHandleResult::Handled
            }
            KeyCode::Char('4') => {
                self.pattern_selected = 3;
                self.check_answer();
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.check_answer();
                KeyHandleResult::Handled
            }
            KeyCode::Esc => KeyHandleResult::RequestQuit,
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_memory_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        if self.memory_phase == MemoryPhase::Memorize {
            return match key.code {
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            };
        }

        let (rows, cols) = self.memory_grid_size;
        let (row, col) = self.memory_cursor;

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if row > 0 {
                    self.memory_cursor.0 -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if row < rows - 1 {
                    self.memory_cursor.0 += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                if col > 0 {
                    self.memory_cursor.1 -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                if col < cols - 1 {
                    self.memory_cursor.1 += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(' ') => {
                let pos = self.memory_cursor;
                if let Some(idx) = self.memory_player_cells.iter().position(|p| *p == pos) {
                    self.memory_player_cells.remove(idx);
                } else {
                    self.memory_player_cells.push(pos);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.check_answer();
                KeyHandleResult::Handled
            }
            KeyCode::Esc => KeyHandleResult::RequestQuit,
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_number_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if self.number_selected > 0 {
                    self.number_selected -= 1;
                } else {
                    self.number_selected = 3;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.number_selected = (self.number_selected + 1) % 4;
                KeyHandleResult::Handled
            }
            KeyCode::Char('1') => {
                self.number_selected = 0;
                self.check_answer();
                KeyHandleResult::Handled
            }
            KeyCode::Char('2') => {
                self.number_selected = 1;
                self.check_answer();
                KeyHandleResult::Handled
            }
            KeyCode::Char('3') => {
                self.number_selected = 2;
                self.check_answer();
                KeyHandleResult::Handled
            }
            KeyCode::Char('4') => {
                self.number_selected = 3;
                self.check_answer();
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.check_answer();
                KeyHandleResult::Handled
            }
            KeyCode::Esc => KeyHandleResult::RequestQuit,
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_daily_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        if self.question_index >= self.daily_question_types.len() {
            return KeyHandleResult::NotHandled;
        }

        match self.daily_question_types[self.question_index] {
            DailyQuestionType::Pattern => self.handle_pattern_key(key),
            DailyQuestionType::Memory => self.handle_memory_key(key),
            DailyQuestionType::Number => self.handle_number_key(key),
        }
    }
}
