//! STORYWEAVER - AI Choose Your Own Adventure
//!
//! Interactive fiction powered by Claude API with branching paths.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crate::plugins::qmind::api::{chat::create_chat_provider, AIApiConfig};
use crossterm::event::{KeyCode, KeyEvent};
use serde::{Deserialize, Serialize};

/// Pre-made story templates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoryTemplate {
    DragonsKeep,
    StarshipHorizon,
    BlackwoodManor,
    SamuraisHonor,
    SurvivalDayZero,
    Custom,
}

impl StoryTemplate {
    pub fn all() -> &'static [StoryTemplate] {
        &[
            StoryTemplate::DragonsKeep,
            StoryTemplate::StarshipHorizon,
            StoryTemplate::BlackwoodManor,
            StoryTemplate::SamuraisHonor,
            StoryTemplate::SurvivalDayZero,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            StoryTemplate::DragonsKeep => "THE DRAGON'S KEEP",
            StoryTemplate::StarshipHorizon => "STARSHIP HORIZON",
            StoryTemplate::BlackwoodManor => "MYSTERY OF BLACKWOOD MANOR",
            StoryTemplate::SamuraisHonor => "THE SAMURAI'S HONOR",
            StoryTemplate::SurvivalDayZero => "SURVIVAL: DAY ZERO",
            StoryTemplate::Custom => "CUSTOM STORY",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            StoryTemplate::DragonsKeep => "#",
            StoryTemplate::StarshipHorizon => "*",
            StoryTemplate::BlackwoodManor => "?",
            StoryTemplate::SamuraisHonor => "+",
            StoryTemplate::SurvivalDayZero => "!",
            StoryTemplate::Custom => ">",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            StoryTemplate::DragonsKeep => "A classic fantasy quest to rescue the kingdom",
            StoryTemplate::StarshipHorizon => "Space exploration and first contact mysteries",
            StoryTemplate::BlackwoodManor => "Gothic detective story with supernatural twists",
            StoryTemplate::SamuraisHonor => "Historical adventure in feudal Japan",
            StoryTemplate::SurvivalDayZero => "Post-apocalyptic survival horror",
            StoryTemplate::Custom => "Create your own adventure!",
        }
    }

    pub fn age_rating(&self) -> &'static str {
        match self {
            StoryTemplate::DragonsKeep => "Ages 8+",
            StoryTemplate::StarshipHorizon => "Ages 10+",
            StoryTemplate::BlackwoodManor => "Ages 12+",
            StoryTemplate::SamuraisHonor => "Ages 13+",
            StoryTemplate::SurvivalDayZero => "Ages 15+",
            StoryTemplate::Custom => "Varies",
        }
    }

    pub fn genre(&self) -> &'static str {
        match self {
            StoryTemplate::DragonsKeep => "Fantasy Adventure",
            StoryTemplate::StarshipHorizon => "Science Fiction",
            StoryTemplate::BlackwoodManor => "Gothic Mystery",
            StoryTemplate::SamuraisHonor => "Historical Drama",
            StoryTemplate::SurvivalDayZero => "Survival Horror",
            StoryTemplate::Custom => "Custom",
        }
    }

    /// Get the opening scenario for this story
    pub fn opening_prompt(&self) -> &'static str {
        match self {
            StoryTemplate::DragonsKeep => {
                r#"You are a young adventurer in a medieval fantasy kingdom. The dragon Scorax has captured Princess Elena and holds her in the highest tower of the Dark Castle. The king has promised half his kingdom to whoever rescues her. Armed with your grandfather's sword and a map from a mysterious old woman, you set forth on your quest. You stand at a crossroads - the Dark Forest lies ahead, and smoke rises from the castle in the distance."#
            }

            StoryTemplate::StarshipHorizon => {
                r#"You are the newly promoted First Officer aboard the generation ship Horizon, humanity's hope for a new home among the stars. After 200 years of travel, the ship's AI has detected an anomaly - a planet that shouldn't exist according to the original charts. Strange signals emanate from its surface. As the captain lies in critical condition after a mysterious accident, the crew looks to you for leadership."#
            }

            StoryTemplate::BlackwoodManor => {
                r#"You are a detective called to Blackwood Manor on a stormy night. Lord Blackwood has been found dead in his locked study - but there's no sign of how the killer escaped. The family has gathered in the drawing room: Lady Blackwood, the cold widow; Edmund, the gambling son; Margaret, the estranged daughter just returned from abroad; and the loyal butler, Graves. The local constable whispers that the manor is cursed."#
            }

            StoryTemplate::SamuraisHonor => {
                r#"You are Takeshi, a ronin in feudal Japan, 1603. Your master was betrayed and killed by the Shogun's forces. Now you wander, seeking purpose. A village elder approaches you - his village is being terrorized by bandits who were once samurai like yourself. He cannot pay, but he offers you shelter and the chance to regain your honor. In the distance, you see smoke rising from the village."#
            }

            StoryTemplate::SurvivalDayZero => {
                r#"Day Zero. The emergency broadcast cut out three hours ago. From your apartment window, you watched the city descend into chaos. Now the streets are quiet - too quiet. Your supplies: a flashlight, a kitchen knife, and three days of canned food. Your phone shows one unread message from your sister across town: "Coming to find you. Stay alive." You hear footsteps in the hallway outside your door."#
            }

            StoryTemplate::Custom => "",
        }
    }
}

/// Story tone for custom stories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoryTone {
    #[default]
    Adventure,
    Dark,
    Comedic,
    Romantic,
    Mystery,
    Horror,
}

impl StoryTone {
    pub fn all() -> &'static [StoryTone] {
        &[
            StoryTone::Adventure,
            StoryTone::Dark,
            StoryTone::Comedic,
            StoryTone::Romantic,
            StoryTone::Mystery,
            StoryTone::Horror,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            StoryTone::Adventure => "Adventure",
            StoryTone::Dark => "Dark Fantasy",
            StoryTone::Comedic => "Comedic",
            StoryTone::Romantic => "Romantic",
            StoryTone::Mystery => "Mystery",
            StoryTone::Horror => "Horror",
        }
    }
}

/// A single chapter/scene in the story
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryChapter {
    pub narrative: String,
    pub choices: Vec<StoryChoice>,
    pub scene_art: Option<String>,
}

/// A choice the player can make
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryChoice {
    pub label: char, // A, B, C, D
    pub text: String,
}

/// Character state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CharacterState {
    pub name: String,
    pub items: Vec<String>,
    pub allies: Vec<String>,
    pub reputation: String,
    pub notes: Vec<String>, // Story-specific flags
}

impl CharacterState {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            items: Vec::new(),
            allies: Vec::new(),
            reputation: "Unknown".to_string(),
            notes: Vec::new(),
        }
    }
}

/// Current view in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoryweaverView {
    #[default]
    StorySelect, // Choose a pre-made story
    CustomCreate, // Create custom story
    Loading,      // Generating chapter
    Playing,      // Reading/choosing
    GameOver,     // Story ended
    Error,        // API error
}

/// Main game state
#[derive(Debug)]
pub struct StoryweaverState {
    // View
    pub view: StoryweaverView,
    pub selected_story: usize,
    pub custom_cursor: usize, // 0=premise, 1=tone, 2=generate

    // Story setup
    pub active_template: Option<StoryTemplate>,
    pub custom_premise: String,
    pub custom_tone: StoryTone,

    // Gameplay
    pub chapters: Vec<StoryChapter>,
    pub current_chapter: usize,
    pub selected_choice: usize,
    pub choice_history: Vec<(usize, char)>, // (chapter, choice made)
    pub character: CharacterState,

    // Stats
    pub total_choices: u32,
    pub chapters_read: u32,

    // Error
    pub error_message: Option<String>,

    // Animation
    pub loading_frame: u8,
    pub text_reveal: usize, // For typewriter effect

    // API
    api_available: bool,

    // Deferred generation (so Loading UI can render first)
    pub pending_generation: bool,
    pub pending_choice: Option<(char, String)>, // (choice_label, choice_text) for continue

    // GameEngine events
    pending_events: Vec<GameEvent>,
}

impl Default for StoryweaverState {
    fn default() -> Self {
        Self::new()
    }
}

impl StoryweaverState {
    pub fn new() -> Self {
        let api_config = AIApiConfig::from_env();
        Self {
            view: StoryweaverView::StorySelect,
            selected_story: 0,
            custom_cursor: 0,
            active_template: None,
            custom_premise: String::new(),
            custom_tone: StoryTone::Adventure,
            chapters: Vec::new(),
            current_chapter: 0,
            selected_choice: 0,
            choice_history: Vec::new(),
            character: CharacterState::default(),
            total_choices: 0,
            chapters_read: 0,
            error_message: None,
            loading_frame: 0,
            text_reveal: 0,
            api_available: api_config.is_configured(),
            pending_generation: false,
            pending_choice: None,
            pending_events: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.view = StoryweaverView::StorySelect;
        self.selected_story = 0;
        self.custom_cursor = 0;
        self.active_template = None;
        self.custom_premise.clear();
        self.custom_tone = StoryTone::Adventure;
        self.chapters.clear();
        self.current_chapter = 0;
        self.selected_choice = 0;
        self.choice_history.clear();
        self.character = CharacterState::default();
        self.total_choices = 0;
        self.chapters_read = 0;
        self.error_message = None;
        self.loading_frame = 0;
        self.text_reveal = 0;
        self.pending_generation = false;
        self.pending_choice = None;
        self.pending_events.clear();
    }

    pub fn is_api_available(&self) -> bool {
        self.api_available
    }

    /// Start a pre-made story
    pub fn start_story(&mut self, template: StoryTemplate) {
        self.active_template = Some(template);
        self.view = StoryweaverView::Loading;
        self.chapters.clear();
        self.current_chapter = 0;
        self.choice_history.clear();
        self.character = CharacterState::new("Adventurer");
        self.total_choices = 0;
        self.chapters_read = 0;
        self.error_message = None;
        self.pending_generation = true; // Will be processed in tick()
        self.pending_choice = None;
        self.pending_events.push(GameEvent::GameStarted);
    }

    /// Start a custom story
    pub fn start_custom_story(&mut self) {
        if self.custom_premise.trim().is_empty() {
            self.error_message = Some("Please enter a story premise".to_string());
            return;
        }
        self.active_template = Some(StoryTemplate::Custom);
        self.view = StoryweaverView::Loading;
        self.chapters.clear();
        self.current_chapter = 0;
        self.choice_history.clear();
        self.character = CharacterState::new("Protagonist");
        self.total_choices = 0;
        self.chapters_read = 0;
        self.error_message = None;
        self.pending_generation = true; // Will be processed in tick()
        self.pending_choice = None;
        self.pending_events.push(GameEvent::GameStarted);
    }

    /// Generate the opening chapter
    pub fn generate_opening(&mut self) {
        let config = AIApiConfig::from_env();
        let provider = match create_chat_provider(config) {
            Ok(p) => p,
            Err(e) => {
                self.error_message = Some(format!("API Error: {}", e));
                self.view = StoryweaverView::Error;
                return;
            }
        };

        let template = self.active_template.unwrap_or(StoryTemplate::Custom);
        let opening = if template == StoryTemplate::Custom {
            self.custom_premise.clone()
        } else {
            template.opening_prompt().to_string()
        };

        let tone = if template == StoryTemplate::Custom {
            self.custom_tone.name()
        } else {
            template.genre()
        };

        let system_prompt = format!(
            r#"You are a choose-your-own-adventure story narrator. Write immersive, engaging prose.

Story setup: {}
Tone: {}

Generate the opening scene with EXACTLY 4 choices for the reader.

Return ONLY JSON in this exact format:
{{
  "narrative": "2-3 paragraphs of atmospheric, engaging prose describing the scene",
  "choices": [
    {{"label": "A", "text": "First choice option"}},
    {{"label": "B", "text": "Second choice option"}},
    {{"label": "C", "text": "Third choice option"}},
    {{"label": "D", "text": "Fourth choice option"}}
  ],
  "scene_art": "Brief description for ASCII art (castle, spaceship, etc)"
}}

Guidelines:
- Write in second person ("You see...", "You feel...")
- Each choice should lead to meaningfully different outcomes
- Maintain the established tone throughout
- Keep narrative under 500 words"#,
            opening, tone
        );

        let user_prompt = "Generate the opening scene with 4 meaningful choices.";

        match provider.prompt(Some(&system_prompt), user_prompt) {
            Ok(response) => {
                if let Ok(chapter) = parse_chapter(&response.content) {
                    self.chapters.push(chapter);
                    self.view = StoryweaverView::Playing;
                    self.text_reveal = 0;
                    self.chapters_read = 1;
                    self.pending_events.push(GameEvent::ChapterCompleted);
                } else {
                    self.error_message = Some("Failed to parse story chapter".to_string());
                    self.view = StoryweaverView::Error;
                }
            }
            Err(e) => {
                self.error_message = Some(format!("API Error: {}", e));
                self.view = StoryweaverView::Error;
            }
        }
    }

    /// Record choice and trigger deferred generation
    pub fn make_choice(&mut self) {
        if self.view != StoryweaverView::Playing {
            return;
        }

        let current = &self.chapters[self.current_chapter];
        if self.selected_choice >= current.choices.len() {
            return;
        }

        let choice = &current.choices[self.selected_choice];
        let choice_label = choice.label;
        let choice_text = choice.text.clone();

        // Record choice
        self.choice_history
            .push((self.current_chapter, choice_label));
        self.total_choices += 1;

        // Set up for deferred generation (so Loading UI renders first)
        self.view = StoryweaverView::Loading;
        self.pending_choice = Some((choice_label, choice_text));
    }

    /// Generate next chapter based on stored choice (called from tick)
    fn generate_continuation(&mut self, choice_label: char, choice_text: String) {
        let config = AIApiConfig::from_env();
        let provider = match create_chat_provider(config) {
            Ok(p) => p,
            Err(e) => {
                self.error_message = Some(format!("API Error: {}", e));
                self.view = StoryweaverView::Error;
                return;
            }
        };

        // Build story context from previous chapters
        let story_context = self.build_story_context();
        let template = self.active_template.unwrap_or(StoryTemplate::Custom);
        let tone = if template == StoryTemplate::Custom {
            self.custom_tone.name()
        } else {
            template.genre()
        };

        let system_prompt = format!(
            r#"You are continuing a choose-your-own-adventure story.

Story so far:
{}

The reader chose: {} - "{}"

Tone: {}

Continue the story based on this choice. Generate the next scene with EXACTLY 4 new choices.
If this feels like a natural ending point (after at least 5 chapters), you may end with a "THE END" type narrative and no choices.

Return ONLY JSON in this exact format:
{{
  "narrative": "2-3 paragraphs continuing from the choice",
  "choices": [
    {{"label": "A", "text": "First choice"}},
    {{"label": "B", "text": "Second choice"}},
    {{"label": "C", "text": "Third choice"}},
    {{"label": "D", "text": "Fourth choice"}}
  ],
  "scene_art": "Brief art description"
}}

For an ending, use empty choices array:
{{
  "narrative": "Final narrative with satisfying conclusion...\n\nTHE END",
  "choices": [],
  "scene_art": "Final scene description"
}}"#,
            story_context, choice_label, choice_text, tone
        );

        let user_prompt = format!(
            "Continue the story after the reader chose: {} - {}",
            choice_label, choice_text
        );

        match provider.prompt(Some(&system_prompt), &user_prompt) {
            Ok(response) => {
                if let Ok(chapter) = parse_chapter(&response.content) {
                    let is_ending = chapter.choices.is_empty();
                    self.chapters.push(chapter);
                    self.current_chapter = self.chapters.len() - 1;
                    self.selected_choice = 0;
                    self.text_reveal = 0;
                    self.chapters_read += 1;

                    self.pending_events.push(GameEvent::ChapterCompleted);

                    if is_ending {
                        self.view = StoryweaverView::GameOver;
                        self.pending_events.push(GameEvent::StoryCompleted);
                        self.pending_events.push(GameEvent::GameEnded { won: true });
                    } else {
                        self.view = StoryweaverView::Playing;
                    }
                } else {
                    self.error_message = Some("Failed to parse story chapter".to_string());
                    self.view = StoryweaverView::Error;
                }
            }
            Err(e) => {
                self.error_message = Some(format!("API Error: {}", e));
                self.view = StoryweaverView::Error;
            }
        }
    }

    /// Build context from previous chapters for continuity
    fn build_story_context(&self) -> String {
        let mut context = String::new();

        for (i, chapter) in self.chapters.iter().enumerate() {
            // Truncate long narratives
            let narrative = if chapter.narrative.len() > 300 {
                format!("{}...", &chapter.narrative[..300])
            } else {
                chapter.narrative.clone()
            };

            context.push_str(&format!("Chapter {}: {}\n", i + 1, narrative));

            if let Some((_, choice)) = self.choice_history.iter().find(|(ch, _)| *ch == i) {
                if let Some(c) = chapter.choices.iter().find(|c| c.label == *choice) {
                    context.push_str(&format!("Choice: {} - {}\n\n", choice, c.text));
                }
            }
        }

        context
    }

    /// Get current chapter if available
    pub fn current_chapter_data(&self) -> Option<&StoryChapter> {
        self.chapters.get(self.current_chapter)
    }

    /// Calculate final score
    pub fn final_score(&self) -> u32 {
        // Score based on chapters read and choices made
        (self.chapters_read * 100) + (self.total_choices * 50)
    }

    /// Game tick for animations and deferred generation
    pub fn tick(&mut self) {
        self.loading_frame = (self.loading_frame + 1) % 8;

        // Process deferred generation (so Loading UI renders first)
        if self.view == StoryweaverView::Loading {
            if self.pending_generation {
                self.pending_generation = false;
                self.generate_opening();
            } else if let Some((choice_label, choice_text)) = self.pending_choice.take() {
                self.generate_continuation(choice_label, choice_text);
            }
        }

        // Typewriter effect for text reveal
        if self.view == StoryweaverView::Playing {
            if let Some(chapter) = self.chapters.get(self.current_chapter) {
                if self.text_reveal < chapter.narrative.len() {
                    self.text_reveal += 3; // Reveal 3 chars per tick
                }
            }
        }
    }

    // Navigation helpers
    pub fn story_up(&mut self) {
        if self.selected_story > 0 {
            self.selected_story -= 1;
        }
    }

    pub fn story_down(&mut self) {
        let max = StoryTemplate::all().len(); // +1 for custom
        if self.selected_story < max {
            self.selected_story += 1;
        }
    }

    pub fn choice_up(&mut self) {
        if self.selected_choice > 0 {
            self.selected_choice -= 1;
        }
    }

    pub fn choice_down(&mut self) {
        if let Some(chapter) = self.chapters.get(self.current_chapter) {
            if self.selected_choice < chapter.choices.len().saturating_sub(1) {
                self.selected_choice += 1;
            }
        }
    }

    pub fn custom_up(&mut self) {
        if self.custom_cursor > 0 {
            self.custom_cursor -= 1;
        }
    }

    pub fn custom_down(&mut self) {
        if self.custom_cursor < 2 {
            self.custom_cursor += 1;
        }
    }

    pub fn tone_left(&mut self) {
        let tones = StoryTone::all();
        let idx = tones
            .iter()
            .position(|t| *t == self.custom_tone)
            .unwrap_or(0);
        if idx > 0 {
            self.custom_tone = tones[idx - 1];
        }
    }

    pub fn tone_right(&mut self) {
        let tones = StoryTone::all();
        let idx = tones
            .iter()
            .position(|t| *t == self.custom_tone)
            .unwrap_or(0);
        if idx < tones.len() - 1 {
            self.custom_tone = tones[idx + 1];
        }
    }

    pub fn add_premise_char(&mut self, c: char) {
        if self.custom_premise.len() < 200 {
            self.custom_premise.push(c);
        }
    }

    pub fn backspace_premise(&mut self) {
        self.custom_premise.pop();
    }
}

/// Parse chapter from JSON response
fn parse_chapter(response: &str) -> Result<StoryChapter, ()> {
    let json_str = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    #[derive(Deserialize)]
    struct ParsedChapter {
        narrative: String,
        choices: Vec<ParsedChoice>,
        scene_art: Option<String>,
    }

    #[derive(Deserialize)]
    struct ParsedChoice {
        label: String,
        text: String,
    }

    let parsed: ParsedChapter = serde_json::from_str(json_str).map_err(|_| ())?;

    Ok(StoryChapter {
        narrative: parsed.narrative,
        choices: parsed
            .choices
            .into_iter()
            .map(|c| StoryChoice {
                label: c.label.chars().next().unwrap_or('A'),
                text: c.text,
            })
            .collect(),
        scene_art: parsed.scene_art,
    })
}

// === GameEngine Implementation ===

impl GameEngine for StoryweaverState {
    fn tick(&mut self) {
        self.loading_frame = (self.loading_frame + 1) % 8;

        // Process deferred generation
        if self.view == StoryweaverView::Loading {
            if self.pending_generation {
                self.pending_generation = false;
                self.generate_opening();
            } else if let Some((choice_label, choice_text)) = self.pending_choice.take() {
                self.generate_continuation(choice_label, choice_text);
            }
        }

        // Typewriter effect for text reveal
        if self.view == StoryweaverView::Playing {
            if let Some(chapter) = self.chapters.get(self.current_chapter) {
                if self.text_reveal < chapter.narrative.len() {
                    self.text_reveal += 3;
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            StoryweaverView::StorySelect => match key.code {
                KeyCode::Up => {
                    self.story_up();
                    KeyHandleResult::Handled
                }
                KeyCode::Down => {
                    self.story_down();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    let templates = StoryTemplate::all();
                    if self.selected_story < templates.len() {
                        self.start_story(templates[self.selected_story]);
                    } else {
                        self.view = StoryweaverView::CustomCreate;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::NotHandled,
            },
            StoryweaverView::CustomCreate => match key.code {
                KeyCode::Up => {
                    self.custom_up();
                    KeyHandleResult::Handled
                }
                KeyCode::Down => {
                    self.custom_down();
                    KeyHandleResult::Handled
                }
                KeyCode::Left if self.custom_cursor == 1 => {
                    self.tone_left();
                    KeyHandleResult::Handled
                }
                KeyCode::Right if self.custom_cursor == 1 => {
                    self.tone_right();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter if self.custom_cursor == 2 => {
                    self.start_custom_story();
                    KeyHandleResult::Handled
                }
                KeyCode::Char(c) if self.custom_cursor == 0 => {
                    self.add_premise_char(c);
                    KeyHandleResult::Handled
                }
                KeyCode::Backspace if self.custom_cursor == 0 => {
                    self.backspace_premise();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    self.view = StoryweaverView::StorySelect;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::NotHandled,
            },
            StoryweaverView::Playing => match key.code {
                KeyCode::Up => {
                    self.choice_up();
                    KeyHandleResult::Handled
                }
                KeyCode::Down => {
                    self.choice_down();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.make_choice();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::NotHandled,
            },
            StoryweaverView::GameOver | StoryweaverView::Error => match key.code {
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
        self.view == StoryweaverView::GameOver
    }

    fn is_game_won(&self) -> bool {
        // Story games are always "won" when completed
        self.view == StoryweaverView::GameOver
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn get_stat(&self, key: &str) -> Option<u64> {
        match key {
            "chapters" => Some(self.chapters_read as u64),
            "choices" => Some(self.total_choices as u64),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_story_templates() {
        for template in StoryTemplate::all() {
            assert!(!template.name().is_empty());
            assert!(!template.description().is_empty());
            assert!(!template.opening_prompt().is_empty());
        }
    }

    #[test]
    fn test_parse_chapter() {
        let json = r#"{
            "narrative": "You stand at the crossroads...",
            "choices": [
                {"label": "A", "text": "Go left"},
                {"label": "B", "text": "Go right"}
            ],
            "scene_art": "crossroads"
        }"#;

        let chapter = parse_chapter(json).unwrap();
        assert_eq!(chapter.choices.len(), 2);
        assert_eq!(chapter.choices[0].label, 'A');
    }

    #[test]
    fn test_character_state() {
        let mut char = CharacterState::new("Hero");
        assert_eq!(char.name, "Hero");
        char.items.push("Sword".to_string());
        assert_eq!(char.items.len(), 1);
    }
}
