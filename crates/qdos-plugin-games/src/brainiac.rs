//! BRAINIAC - AI-Powered Trivia Game
//!
//! Dynamic trivia powered by Claude API with age-adaptive difficulty.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use qdos_plugin_qmind::api::{chat::create_chat_provider, AIApiConfig};
use serde::{Deserialize, Serialize};

/// Trivia categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriviaCategory {
    Science,
    History,
    Movies,
    Music,
    Sports,
    Food,
    Animals,
    Space,
    Literature,
    Technology,
    Custom,
}

impl TriviaCategory {
    pub fn all() -> &'static [TriviaCategory] {
        &[
            TriviaCategory::Science,
            TriviaCategory::History,
            TriviaCategory::Movies,
            TriviaCategory::Music,
            TriviaCategory::Sports,
            TriviaCategory::Food,
            TriviaCategory::Animals,
            TriviaCategory::Space,
            TriviaCategory::Literature,
            TriviaCategory::Technology,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            TriviaCategory::Science => "Science & Nature",
            TriviaCategory::History => "History & Geography",
            TriviaCategory::Movies => "Movies & TV",
            TriviaCategory::Music => "Music & Arts",
            TriviaCategory::Sports => "Sports & Games",
            TriviaCategory::Food => "Food & Cooking",
            TriviaCategory::Animals => "Animals & Wildlife",
            TriviaCategory::Space => "Space & Astronomy",
            TriviaCategory::Literature => "Literature & Books",
            TriviaCategory::Technology => "Technology",
            TriviaCategory::Custom => "Custom Topic",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TriviaCategory::Science => "~",
            TriviaCategory::History => "#",
            TriviaCategory::Movies => "*",
            TriviaCategory::Music => "d",
            TriviaCategory::Sports => "o",
            TriviaCategory::Food => "+",
            TriviaCategory::Animals => "@",
            TriviaCategory::Space => ".",
            TriviaCategory::Literature => "=",
            TriviaCategory::Technology => ">",
            TriviaCategory::Custom => "?",
        }
    }
}

/// Age bracket for difficulty adjustment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgeBracket {
    Kids,    // 5-8
    Tweens,  // 9-12
    Teens,   // 13-17
    Adults,  // 18-59
    Seniors, // 60+
}

impl AgeBracket {
    pub fn from_age(age: u8) -> Self {
        match age {
            0..=8 => AgeBracket::Kids,
            9..=12 => AgeBracket::Tweens,
            13..=17 => AgeBracket::Teens,
            18..=59 => AgeBracket::Adults,
            _ => AgeBracket::Seniors,
        }
    }

    pub fn difficulty_label(&self) -> &'static str {
        match self {
            AgeBracket::Kids => "Easy",
            AgeBracket::Tweens => "Medium",
            AgeBracket::Teens => "Medium-Hard",
            AgeBracket::Adults => "Hard",
            AgeBracket::Seniors => "Classic",
        }
    }
}

/// A single trivia question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriviaQuestion {
    pub question: String,
    pub options: [String; 4],
    pub correct_index: usize,
    pub fun_fact: String,
    pub category: TriviaCategory,
}

/// Game mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameMode {
    #[default]
    QuickPlay, // 10 questions
    Marathon,  // 50 questions, 3 lives
    Challenge, // 5 seconds per question
}

impl GameMode {
    pub fn name(&self) -> &'static str {
        match self {
            GameMode::QuickPlay => "Quick Play",
            GameMode::Marathon => "Marathon",
            GameMode::Challenge => "Challenge",
        }
    }

    pub fn question_count(&self) -> usize {
        match self {
            GameMode::QuickPlay => 10,
            GameMode::Marathon => 50,
            GameMode::Challenge => 10,
        }
    }

    pub fn time_limit(&self) -> u32 {
        match self {
            GameMode::QuickPlay => 15,
            GameMode::Marathon => 15,
            GameMode::Challenge => 5,
        }
    }
}

/// Current view in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrainiacView {
    #[default]
    Setup, // Age and category selection
    Loading,  // Generating questions
    Playing,  // Active gameplay
    Feedback, // Showing answer feedback
    GameOver, // Final results
    Error,    // API error
}

/// Main game state
#[derive(Debug)]
pub struct BrainiacState {
    // Setup
    pub view: BrainiacView,
    pub player_age: u8,
    pub selected_category: Option<TriviaCategory>,
    pub custom_topic: String,
    pub game_mode: GameMode,
    pub setup_cursor: usize, // 0=age, 1=category, 2=mode, 3=start

    // Gameplay
    pub questions: Vec<TriviaQuestion>,
    pub current_question: usize,
    pub selected_answer: usize,
    pub score: u32,
    pub streak: u32,
    pub best_streak: u32,
    pub lives: u32,
    pub time_remaining: u32,
    pub tick_counter: u32,

    // Feedback
    pub last_correct: bool,
    pub feedback_timer: u32,

    // Results
    pub correct_count: u32,
    pub total_time_bonus: u32,

    // Error state
    pub error_message: Option<String>,

    // Animation
    pub brain_frame: u8,
    pub celebration_frame: u8,

    // API
    api_available: bool,

    // Deferred generation (so Loading UI can render first)
    pub pending_generation: bool,

    // GameEngine events
    pending_events: Vec<GameEvent>,
}

impl Default for BrainiacState {
    fn default() -> Self {
        Self::new()
    }
}

impl BrainiacState {
    pub fn new() -> Self {
        let api_config = AIApiConfig::from_env();
        Self {
            view: BrainiacView::Setup,
            player_age: 18,
            selected_category: None,
            custom_topic: String::new(),
            game_mode: GameMode::QuickPlay,
            setup_cursor: 0,
            questions: Vec::new(),
            current_question: 0,
            selected_answer: 0,
            score: 0,
            streak: 0,
            best_streak: 0,
            lives: 3,
            time_remaining: 15,
            tick_counter: 0,
            last_correct: false,
            feedback_timer: 0,
            correct_count: 0,
            total_time_bonus: 0,
            error_message: None,
            brain_frame: 0,
            celebration_frame: 0,
            api_available: api_config.is_configured(),
            pending_generation: false,
            pending_events: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.view = BrainiacView::Setup;
        self.questions.clear();
        self.current_question = 0;
        self.selected_answer = 0;
        self.score = 0;
        self.streak = 0;
        self.best_streak = 0;
        self.lives = 3;
        self.time_remaining = self.game_mode.time_limit();
        self.tick_counter = 0;
        self.last_correct = false;
        self.feedback_timer = 0;
        self.correct_count = 0;
        self.total_time_bonus = 0;
        self.error_message = None;
        self.setup_cursor = 0;
        self.pending_generation = false;
        self.pending_events.clear();
    }

    /// Check if API is available
    pub fn is_api_available(&self) -> bool {
        self.api_available
    }

    /// Start generating questions (sets pending_generation flag for tick to process)
    pub fn start_game(&mut self) {
        self.view = BrainiacView::Loading;
        self.questions.clear();
        self.current_question = 0;
        self.selected_answer = 0;
        self.score = 0;
        self.streak = 0;
        self.best_streak = 0;
        self.lives = if self.game_mode == GameMode::Marathon {
            3
        } else {
            1
        };
        self.time_remaining = self.game_mode.time_limit();
        self.correct_count = 0;
        self.total_time_bonus = 0;
        self.error_message = None;
        self.pending_generation = true; // Will be processed in tick()
        self.pending_events.push(GameEvent::GameStarted);
    }

    /// Generate questions using AI
    pub fn generate_questions(&mut self) {
        let config = AIApiConfig::from_env();
        let provider = match create_chat_provider(config) {
            Ok(p) => p,
            Err(e) => {
                self.error_message = Some(format!("API Error: {}", e));
                self.view = BrainiacView::Error;
                return;
            }
        };

        let bracket = AgeBracket::from_age(self.player_age);
        let count = self.game_mode.question_count(); // Generate all questions at once

        // Category for parsing - use Science as default for mixed topics (it's just metadata)
        let category = self.selected_category.unwrap_or(TriviaCategory::Science);

        // For "All Topics" (None), ask for mixed categories
        let (topic, category_hint) = if let Some(cat) = self.selected_category {
            if cat == TriviaCategory::Custom && !self.custom_topic.is_empty() {
                (self.custom_topic.clone(), "".to_string())
            } else {
                (cat.name().to_string(), "".to_string())
            }
        } else {
            // Mixed categories - explicitly request variety
            (
                "mixed topics (science, history, movies, music, sports, food, animals, space, literature, technology)".to_string(),
                "\n- IMPORTANT: Mix different categories - do NOT make all questions from the same category\n- Each question should ideally be from a different category".to_string(),
            )
        };

        let system_prompt = format!(
            r#"You are a trivia question generator. Generate exactly {} UNIQUE trivia questions about "{}" suitable for a {}-year-old ({} difficulty).

Return ONLY a JSON array with this exact format, no other text:
[
  {{
    "question": "The question text?",
    "options": ["Option A", "Option B", "Option C", "Option D"],
    "correct_index": 0,
    "fun_fact": "An interesting fact about the answer."
  }}
]

Guidelines for {} age group:
- Use appropriate vocabulary and complexity
- Questions should be educational and interesting
- All options should be plausible
- Fun facts should be engaging and memorable
- Each question must be DIFFERENT - no repeating questions{}
- Generate exactly {} questions"#,
            count,
            topic,
            self.player_age,
            bracket.difficulty_label(),
            bracket.difficulty_label(),
            category_hint,
            count
        );

        let user_prompt = format!(
            "Generate {} unique trivia questions about {} for someone who is {} years old. Make sure each question is different and covers different aspects of the topic.",
            count, topic, self.player_age
        );

        match provider.prompt(Some(&system_prompt), &user_prompt) {
            Ok(response) => {
                if let Ok(questions) = parse_questions(&response.content, category) {
                    self.questions = questions;
                    if !self.questions.is_empty() {
                        self.view = BrainiacView::Playing;
                        self.time_remaining = self.game_mode.time_limit();
                    } else {
                        self.error_message = Some("Failed to parse questions".to_string());
                        self.view = BrainiacView::Error;
                    }
                } else {
                    self.error_message = Some("Invalid question format".to_string());
                    self.view = BrainiacView::Error;
                }
            }
            Err(e) => {
                self.error_message = Some(format!("API Error: {}", e));
                self.view = BrainiacView::Error;
            }
        }
    }

    /// Handle answer selection
    pub fn select_answer(&mut self) {
        if self.view != BrainiacView::Playing {
            return;
        }

        let current = &self.questions[self.current_question];
        self.last_correct = self.selected_answer == current.correct_index;

        if self.last_correct {
            // Calculate score
            let base_points = 100;
            let time_bonus = if self.time_remaining > 12 {
                50
            } else if self.time_remaining > 8 {
                25
            } else {
                0
            };
            self.total_time_bonus += time_bonus;

            self.streak += 1;
            if self.streak > self.best_streak {
                self.best_streak = self.streak;
            }

            let streak_multiplier = if self.streak >= 5 {
                2.0
            } else if self.streak >= 3 {
                1.5
            } else {
                1.0
            };

            let old_score = self.score;
            let points = ((base_points + time_bonus) as f64 * streak_multiplier) as u32;
            self.score += points;
            self.correct_count += 1;

            self.pending_events
                .push(GameEvent::QuestionAnswered { correct: true });
            self.pending_events.push(GameEvent::ScoreChanged {
                old: old_score,
                new: self.score,
            });
        } else {
            self.streak = 0;
            if self.game_mode == GameMode::Marathon {
                self.lives = self.lives.saturating_sub(1);
            }
            self.pending_events
                .push(GameEvent::QuestionAnswered { correct: false });
        }

        self.view = BrainiacView::Feedback;
        self.feedback_timer = 50; // ~5 seconds at 10 ticks/sec
    }

    /// Move to next question or end game
    pub fn next_question(&mut self) {
        self.current_question += 1;

        // Check end conditions
        if self.current_question >= self.questions.len()
            || (self.game_mode == GameMode::Marathon && self.lives == 0)
        {
            self.view = BrainiacView::GameOver;
            // Winning is completing all questions with lives remaining
            let won = self.current_question >= self.questions.len() && self.lives > 0;
            self.pending_events.push(GameEvent::GameEnded { won });
            return;
        }

        // Need more questions?
        if self.current_question >= self.questions.len() - 1
            && self.current_question < self.game_mode.question_count() - 1
        {
            // Generate more questions
            self.generate_more_questions();
        }

        self.view = BrainiacView::Playing;
        self.selected_answer = 0;
        self.time_remaining = self.game_mode.time_limit();
    }

    /// Generate additional questions
    fn generate_more_questions(&mut self) {
        // For simplicity, just call generate_questions again
        // In a real implementation, this would be async/non-blocking
        let old_questions = self.questions.clone();
        self.generate_questions();
        if self.view == BrainiacView::Playing {
            // Prepend old questions back
            let mut new_questions = old_questions;
            new_questions.append(&mut self.questions);
            self.questions = new_questions;
        }
    }

    /// Game tick (called ~10x per second)
    pub fn tick(&mut self) {
        self.brain_frame = (self.brain_frame + 1) % 4;

        match self.view {
            BrainiacView::Playing => {
                self.tick_counter += 1;
                if self.tick_counter >= 10 {
                    // 1 second
                    self.tick_counter = 0;
                    if self.time_remaining > 0 {
                        self.time_remaining -= 1;
                    } else {
                        // Time's up - treat as wrong answer
                        self.last_correct = false;
                        self.streak = 0;
                        if self.game_mode == GameMode::Marathon {
                            self.lives = self.lives.saturating_sub(1);
                        }
                        self.view = BrainiacView::Feedback;
                        self.feedback_timer = 50; // ~5 seconds at 10 ticks/sec
                    }
                }
            }
            BrainiacView::Feedback => {
                if self.feedback_timer > 0 {
                    self.feedback_timer -= 1;
                } else {
                    self.next_question();
                }
                self.celebration_frame = (self.celebration_frame + 1) % 8;
            }
            BrainiacView::Loading => {
                // Process deferred generation (so loading UI renders first)
                if self.pending_generation {
                    self.pending_generation = false;
                    self.generate_questions();
                }
            }
            _ => {}
        }
    }

    // Navigation helpers
    pub fn answer_up(&mut self) {
        if self.selected_answer > 0 {
            self.selected_answer -= 1;
        } else {
            self.selected_answer = 3;
        }
    }

    pub fn answer_down(&mut self) {
        self.selected_answer = (self.selected_answer + 1) % 4;
    }

    pub fn setup_up(&mut self) {
        if self.setup_cursor > 0 {
            self.setup_cursor -= 1;
        }
    }

    pub fn setup_down(&mut self) {
        if self.setup_cursor < 3 {
            self.setup_cursor += 1;
        }
    }

    pub fn setup_left(&mut self) {
        match self.setup_cursor {
            0 => {
                // Age
                if self.player_age > 5 {
                    self.player_age -= 1;
                }
            }
            1 => {
                // Category
                let categories = TriviaCategory::all();
                if let Some(cat) = self.selected_category {
                    let idx = categories.iter().position(|c| *c == cat).unwrap_or(0);
                    if idx > 0 {
                        self.selected_category = Some(categories[idx - 1]);
                    } else {
                        self.selected_category = None; // Mixed
                    }
                }
            }
            2 => {
                // Mode
                self.game_mode = match self.game_mode {
                    GameMode::QuickPlay => GameMode::Challenge,
                    GameMode::Marathon => GameMode::QuickPlay,
                    GameMode::Challenge => GameMode::Marathon,
                };
            }
            _ => {}
        }
    }

    pub fn setup_right(&mut self) {
        match self.setup_cursor {
            0 => {
                // Age
                if self.player_age < 99 {
                    self.player_age += 1;
                }
            }
            1 => {
                // Category
                let categories = TriviaCategory::all();
                if let Some(cat) = self.selected_category {
                    let idx = categories.iter().position(|c| *c == cat).unwrap_or(0);
                    if idx < categories.len() - 1 {
                        self.selected_category = Some(categories[idx + 1]);
                    }
                } else {
                    self.selected_category = Some(categories[0]);
                }
            }
            2 => {
                // Mode
                self.game_mode = match self.game_mode {
                    GameMode::QuickPlay => GameMode::Marathon,
                    GameMode::Marathon => GameMode::Challenge,
                    GameMode::Challenge => GameMode::QuickPlay,
                };
            }
            _ => {}
        }
    }

    /// Get current question if available
    pub fn current_question_data(&self) -> Option<&TriviaQuestion> {
        self.questions.get(self.current_question)
    }

    /// Calculate final score with bonuses
    pub fn final_score(&self) -> u32 {
        let mut final_score = self.score;

        // Perfect game bonus
        if self.correct_count == self.game_mode.question_count() as u32 {
            final_score *= 2;
        }

        final_score
    }

    /// Get accuracy percentage
    pub fn accuracy(&self) -> u32 {
        if self.current_question == 0 {
            0
        } else {
            (self.correct_count * 100) / self.current_question.max(1) as u32
        }
    }
}

/// Parse questions from JSON response
fn parse_questions(response: &str, category: TriviaCategory) -> Result<Vec<TriviaQuestion>, ()> {
    // Try to extract JSON array from response
    let json_str = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    #[derive(Deserialize)]
    struct ParsedQuestion {
        question: String,
        options: [String; 4],
        correct_index: usize,
        fun_fact: String,
    }

    let parsed: Vec<ParsedQuestion> = serde_json::from_str(json_str).map_err(|_| ())?;

    Ok(parsed
        .into_iter()
        .map(|p| TriviaQuestion {
            question: p.question,
            options: p.options,
            correct_index: p.correct_index.min(3),
            fun_fact: p.fun_fact,
            category,
        })
        .collect())
}

/// Generate fallback questions when API is unavailable
pub fn fallback_questions() -> Vec<TriviaQuestion> {
    vec![
        TriviaQuestion {
            question: "What planet is known as the Red Planet?".to_string(),
            options: [
                "Venus".to_string(),
                "Mars".to_string(),
                "Jupiter".to_string(),
                "Saturn".to_string(),
            ],
            correct_index: 1,
            fun_fact: "Mars appears red because of iron oxide (rust) on its surface.".to_string(),
            category: TriviaCategory::Space,
        },
        TriviaQuestion {
            question: "What is the largest mammal on Earth?".to_string(),
            options: [
                "African Elephant".to_string(),
                "Blue Whale".to_string(),
                "Giraffe".to_string(),
                "Polar Bear".to_string(),
            ],
            correct_index: 1,
            fun_fact: "Blue whales can grow up to 100 feet long and weigh 200 tons!".to_string(),
            category: TriviaCategory::Animals,
        },
        TriviaQuestion {
            question: "In what year did World War II end?".to_string(),
            options: [
                "1943".to_string(),
                "1944".to_string(),
                "1945".to_string(),
                "1946".to_string(),
            ],
            correct_index: 2,
            fun_fact: "WWII ended on September 2, 1945, when Japan formally surrendered."
                .to_string(),
            category: TriviaCategory::History,
        },
        TriviaQuestion {
            question: "What is the chemical symbol for gold?".to_string(),
            options: [
                "Go".to_string(),
                "Gd".to_string(),
                "Au".to_string(),
                "Ag".to_string(),
            ],
            correct_index: 2,
            fun_fact: "Au comes from 'aurum', the Latin word for gold.".to_string(),
            category: TriviaCategory::Science,
        },
        TriviaQuestion {
            question: "Who wrote 'Romeo and Juliet'?".to_string(),
            options: [
                "Charles Dickens".to_string(),
                "William Shakespeare".to_string(),
                "Jane Austen".to_string(),
                "Mark Twain".to_string(),
            ],
            correct_index: 1,
            fun_fact: "Shakespeare wrote Romeo and Juliet around 1594-1596.".to_string(),
            category: TriviaCategory::Literature,
        },
    ]
}

// === GameEngine Implementation ===

impl GameEngine for BrainiacState {
    fn tick(&mut self) {
        self.brain_frame = (self.brain_frame + 1) % 4;

        match self.view {
            BrainiacView::Playing => {
                self.tick_counter += 1;
                if self.tick_counter >= 10 {
                    self.tick_counter = 0;
                    if self.time_remaining > 0 {
                        self.time_remaining -= 1;
                    } else {
                        // Time's up - treat as wrong answer
                        self.last_correct = false;
                        self.streak = 0;
                        if self.game_mode == GameMode::Marathon {
                            self.lives = self.lives.saturating_sub(1);
                        }
                        self.pending_events
                            .push(GameEvent::QuestionAnswered { correct: false });
                        self.view = BrainiacView::Feedback;
                        self.feedback_timer = 50;
                    }
                }
            }
            BrainiacView::Feedback => {
                if self.feedback_timer > 0 {
                    self.feedback_timer -= 1;
                } else {
                    self.next_question();
                }
                self.celebration_frame = (self.celebration_frame + 1) % 8;
            }
            BrainiacView::Loading => {
                if self.pending_generation {
                    self.pending_generation = false;
                    self.generate_questions();
                }
            }
            _ => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            BrainiacView::Setup => match key.code {
                KeyCode::Up => {
                    self.setup_up();
                    KeyHandleResult::Handled
                }
                KeyCode::Down => {
                    self.setup_down();
                    KeyHandleResult::Handled
                }
                KeyCode::Left => {
                    self.setup_left();
                    KeyHandleResult::Handled
                }
                KeyCode::Right => {
                    self.setup_right();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    if self.setup_cursor == 3 {
                        self.start_game();
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::NotHandled,
            },
            BrainiacView::Playing => match key.code {
                KeyCode::Up => {
                    self.answer_up();
                    KeyHandleResult::Handled
                }
                KeyCode::Down => {
                    self.answer_down();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.select_answer();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::NotHandled,
            },
            BrainiacView::GameOver | BrainiacView::Error => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.reset();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::NotHandled,
            },
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn get_score(&self) -> u32 {
        self.final_score()
    }

    fn is_game_over(&self) -> bool {
        self.view == BrainiacView::GameOver
    }

    fn is_game_won(&self) -> bool {
        self.view == BrainiacView::GameOver
            && self.correct_count >= self.game_mode.question_count() as u32
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn get_stat(&self, key: &str) -> Option<u64> {
        match key {
            "correct" => Some(self.correct_count as u64),
            "streak" => Some(self.best_streak as u64),
            "accuracy" => Some(self.accuracy() as u64),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_age_bracket() {
        assert_eq!(AgeBracket::from_age(5), AgeBracket::Kids);
        assert_eq!(AgeBracket::from_age(10), AgeBracket::Tweens);
        assert_eq!(AgeBracket::from_age(15), AgeBracket::Teens);
        assert_eq!(AgeBracket::from_age(30), AgeBracket::Adults);
        assert_eq!(AgeBracket::from_age(70), AgeBracket::Seniors);
    }

    #[test]
    fn test_game_mode_settings() {
        assert_eq!(GameMode::QuickPlay.question_count(), 10);
        assert_eq!(GameMode::QuickPlay.time_limit(), 15);
        assert_eq!(GameMode::Challenge.time_limit(), 5);
    }

    #[test]
    fn test_fallback_questions() {
        let questions = fallback_questions();
        assert!(!questions.is_empty());
        for q in &questions {
            assert!(q.correct_index < 4);
            assert_eq!(q.options.len(), 4);
        }
    }

    #[test]
    fn test_parse_questions() {
        let json = r#"[
            {
                "question": "Test question?",
                "options": ["A", "B", "C", "D"],
                "correct_index": 2,
                "fun_fact": "Interesting!"
            }
        ]"#;
        let questions = parse_questions(json, TriviaCategory::Science).unwrap();
        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0].correct_index, 2);
    }
}
