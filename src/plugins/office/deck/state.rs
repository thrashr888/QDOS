//! Q-DECK State Types
//!
//! Data structures for the presentation editor.

use std::path::PathBuf;

// =============================================================================
// CONSTANTS
// =============================================================================

pub const MAX_SLIDES: usize = 100;

// =============================================================================
// DECK MODE
// =============================================================================

/// Operating mode for Q-DECK
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeckMode {
    #[default]
    Edit, // Editing slide content
    Present,   // Full-screen presentation
    SlideList, // Slide overview/sorter
    Menu,      // Lotus-style menu
    SaveAs,    // Save As dialog
    Help,      // Help overlay
}

// =============================================================================
// SLIDE TEMPLATE
// =============================================================================

/// Slide layout templates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlideTemplate {
    #[default]
    Title, // Centered title with optional subtitle
    Bullets, // Title with bullet list
    TwoCol,  // Two-column layout
    Image,   // Large image/ASCII art area
    Code,    // Code block with syntax highlighting
    Quote,   // Large quote with attribution
    Blank,   // Empty slide for custom content
}

impl SlideTemplate {
    pub const fn all() -> &'static [SlideTemplate] {
        &[
            SlideTemplate::Title,
            SlideTemplate::Bullets,
            SlideTemplate::TwoCol,
            SlideTemplate::Image,
            SlideTemplate::Code,
            SlideTemplate::Quote,
            SlideTemplate::Blank,
        ]
    }

    pub const fn name(&self) -> &'static str {
        match self {
            SlideTemplate::Title => "Title",
            SlideTemplate::Bullets => "Bullets",
            SlideTemplate::TwoCol => "Two-Col",
            SlideTemplate::Image => "Image",
            SlideTemplate::Code => "Code",
            SlideTemplate::Quote => "Quote",
            SlideTemplate::Blank => "Blank",
        }
    }

    pub const fn key(&self) -> char {
        match self {
            SlideTemplate::Title => 'T',
            SlideTemplate::Bullets => 'B',
            SlideTemplate::TwoCol => '2',
            SlideTemplate::Image => 'I',
            SlideTemplate::Code => 'C',
            SlideTemplate::Quote => 'Q',
            SlideTemplate::Blank => 'K',
        }
    }
}

// =============================================================================
// TRANSITION
// =============================================================================

/// Slide transition effects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Transition {
    #[default]
    None, // Instant switch
    Fade,       // Fade through blank
    SlideLeft,  // Slide from right
    SlideRight, // Slide from left
    SlideUp,    // Slide from bottom
    SlideDown,  // Slide from top
    Reveal,     // Line by line reveal
}

impl Transition {
    pub const fn name(&self) -> &'static str {
        match self {
            Transition::None => "None",
            Transition::Fade => "Fade",
            Transition::SlideLeft => "Slide Left",
            Transition::SlideRight => "Slide Right",
            Transition::SlideUp => "Slide Up",
            Transition::SlideDown => "Slide Down",
            Transition::Reveal => "Reveal",
        }
    }
}

// =============================================================================
// CONTENT BLOCK
// =============================================================================

/// Types of content within a slide
#[derive(Debug, Clone, PartialEq)]
pub enum ContentBlock {
    /// Plain text with optional styling
    Text {
        content: String,
        bold: bool,
        italic: bool,
        color: Option<String>, // ANSI color name or hex
    },
    /// Bullet list
    Bullets(Vec<String>),
    /// Numbered list
    Numbered(Vec<String>),
    /// Code block with language hint
    Code { language: String, content: String },
    /// ANSI art block (preserves exact formatting)
    AnsiArt(String),
    /// Image reference (path or URL, rendered via sixel)
    Image { path: String, alt: String },
    /// Quote with attribution
    Quote { text: String, author: String },
    /// Horizontal separator
    Separator,
}

// =============================================================================
// SLIDE
// =============================================================================

/// A single slide in the presentation
#[derive(Debug, Clone, Default)]
pub struct Slide {
    /// Slide title
    pub title: String,
    /// Slide subtitle (optional)
    pub subtitle: Option<String>,
    /// Layout template
    pub template: SlideTemplate,
    /// Content blocks
    pub content: Vec<ContentBlock>,
    /// Speaker notes (not shown in presentation)
    pub notes: String,
    /// Transition to this slide
    pub transition: Transition,
    /// Background color (ANSI color name)
    pub background: Option<String>,
}

impl Slide {
    pub fn new(title: &str, template: SlideTemplate) -> Self {
        Self {
            title: title.to_string(),
            template,
            ..Default::default()
        }
    }

    pub fn title_slide(title: &str, subtitle: Option<&str>) -> Self {
        Self {
            title: title.to_string(),
            subtitle: subtitle.map(|s| s.to_string()),
            template: SlideTemplate::Title,
            ..Default::default()
        }
    }

    pub fn bullets_slide(title: &str, bullets: Vec<String>) -> Self {
        Self {
            title: title.to_string(),
            template: SlideTemplate::Bullets,
            content: vec![ContentBlock::Bullets(bullets)],
            ..Default::default()
        }
    }
}

// =============================================================================
// DECK THEME
// =============================================================================

/// Presentation theme
#[derive(Debug, Clone, Default)]
pub struct DeckTheme {
    pub name: String,
    pub title_color: String,
    pub text_color: String,
    pub accent_color: String,
    pub background: String,
    pub border_style: BorderStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderStyle {
    #[default]
    Double, // ═══
    Single, // ───
    Heavy,  // ━━━
    Ascii,  // ===
    None,
}

impl DeckTheme {
    pub fn corporate() -> Self {
        Self {
            name: "Corporate".to_string(),
            title_color: "yellow".to_string(),
            text_color: "white".to_string(),
            accent_color: "cyan".to_string(),
            background: "blue".to_string(),
            border_style: BorderStyle::Double,
        }
    }

    pub fn hacker() -> Self {
        Self {
            name: "Hacker".to_string(),
            title_color: "green".to_string(),
            text_color: "green".to_string(),
            accent_color: "cyan".to_string(),
            background: "black".to_string(),
            border_style: BorderStyle::Single,
        }
    }

    pub fn retro() -> Self {
        Self {
            name: "Retro".to_string(),
            title_color: "magenta".to_string(),
            text_color: "cyan".to_string(),
            accent_color: "yellow".to_string(),
            background: "black".to_string(),
            border_style: BorderStyle::Ascii,
        }
    }
}

// =============================================================================
// DECK STATE
// =============================================================================

/// Main presentation state
pub struct DeckState {
    // Document info
    pub file_path: Option<PathBuf>,
    pub modified: bool,

    // Presentation metadata
    pub title: String,
    pub author: String,
    pub theme: DeckTheme,

    // Slides
    pub slides: Vec<Slide>,
    pub current_slide: usize,

    // Mode
    pub mode: DeckMode,

    // Edit state
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub edit_buffer: String,

    // Presentation state
    pub presenting: bool,
    pub transition_progress: f32, // 0.0 to 1.0
    pub timer_seconds: u32,
    pub timer_running: bool,

    // Menu state
    pub menu_category: usize,
    pub menu_item: usize,

    // Save As state
    pub save_as_input: String,
    pub save_as_cursor: usize,

    // Animation
    pub tick_count: u32,

    // Status message
    pub status_message: Option<(String, u32)>,
}

impl Default for DeckState {
    fn default() -> Self {
        Self::new()
    }
}

impl DeckState {
    pub fn new() -> Self {
        // Start with a default title slide
        let title_slide = Slide::title_slide("New Presentation", Some("Created with Q-DECK"));

        Self {
            file_path: None,
            modified: false,
            title: "New Presentation".to_string(),
            author: String::new(),
            theme: DeckTheme::corporate(),
            slides: vec![title_slide],
            current_slide: 0,
            mode: DeckMode::Edit,
            cursor_line: 0,
            cursor_col: 0,
            edit_buffer: String::new(),
            presenting: false,
            transition_progress: 1.0,
            timer_seconds: 0,
            timer_running: false,
            menu_category: 0,
            menu_item: 0,
            save_as_input: String::new(),
            save_as_cursor: 0,
            tick_count: 0,
            status_message: None,
        }
    }

    // =========================================================================
    // SLIDE NAVIGATION
    // =========================================================================

    pub fn slide_count(&self) -> usize {
        self.slides.len()
    }

    pub fn current(&self) -> &Slide {
        &self.slides[self.current_slide]
    }

    pub fn current_mut(&mut self) -> &mut Slide {
        &mut self.slides[self.current_slide]
    }

    pub fn next_slide(&mut self) {
        if self.current_slide < self.slides.len() - 1 {
            self.current_slide += 1;
            self.transition_progress = 0.0;
        }
    }

    pub fn prev_slide(&mut self) {
        if self.current_slide > 0 {
            self.current_slide -= 1;
            self.transition_progress = 0.0;
        }
    }

    pub fn goto_slide(&mut self, index: usize) {
        if index < self.slides.len() {
            self.current_slide = index;
            self.transition_progress = 0.0;
        }
    }

    pub fn first_slide(&mut self) {
        self.current_slide = 0;
        self.transition_progress = 0.0;
    }

    pub fn last_slide(&mut self) {
        self.current_slide = self.slides.len().saturating_sub(1);
        self.transition_progress = 0.0;
    }

    // =========================================================================
    // SLIDE MANAGEMENT
    // =========================================================================

    pub fn add_slide(&mut self, template: SlideTemplate) {
        let slide = Slide::new("New Slide", template);
        let insert_pos = self.current_slide + 1;
        if insert_pos >= self.slides.len() {
            self.slides.push(slide);
        } else {
            self.slides.insert(insert_pos, slide);
        }
        self.current_slide = insert_pos.min(self.slides.len() - 1);
        self.modified = true;
    }

    pub fn delete_slide(&mut self) {
        if self.slides.len() > 1 {
            self.slides.remove(self.current_slide);
            if self.current_slide >= self.slides.len() {
                self.current_slide = self.slides.len() - 1;
            }
            self.modified = true;
        }
    }

    pub fn duplicate_slide(&mut self) {
        let slide = self.slides[self.current_slide].clone();
        let insert_pos = self.current_slide + 1;
        if insert_pos >= self.slides.len() {
            self.slides.push(slide);
        } else {
            self.slides.insert(insert_pos, slide);
        }
        self.current_slide = insert_pos;
        self.modified = true;
    }

    pub fn move_slide_up(&mut self) {
        if self.current_slide > 0 {
            self.slides.swap(self.current_slide, self.current_slide - 1);
            self.current_slide -= 1;
            self.modified = true;
        }
    }

    pub fn move_slide_down(&mut self) {
        if self.current_slide < self.slides.len() - 1 {
            self.slides.swap(self.current_slide, self.current_slide + 1);
            self.current_slide += 1;
            self.modified = true;
        }
    }

    // =========================================================================
    // MODE MANAGEMENT
    // =========================================================================

    pub fn enter_present_mode(&mut self) {
        self.mode = DeckMode::Present;
        self.presenting = true;
        self.first_slide();
    }

    pub fn exit_present_mode(&mut self) {
        self.mode = DeckMode::Edit;
        self.presenting = false;
    }

    pub fn enter_menu_mode(&mut self) {
        self.mode = DeckMode::Menu;
        self.menu_category = 0;
        self.menu_item = 0;
    }

    pub fn exit_menu_mode(&mut self) {
        self.mode = DeckMode::Edit;
    }

    pub fn enter_save_as_mode(&mut self) {
        self.mode = DeckMode::SaveAs;
        self.save_as_input.clear();
        self.save_as_cursor = 0;
    }

    pub fn exit_save_as_mode(&mut self) {
        self.mode = DeckMode::Edit;
    }

    pub fn enter_slide_list_mode(&mut self) {
        self.mode = DeckMode::SlideList;
    }

    pub fn exit_slide_list_mode(&mut self) {
        self.mode = DeckMode::Edit;
    }

    // =========================================================================
    // TIMER
    // =========================================================================

    pub fn toggle_timer(&mut self) {
        self.timer_running = !self.timer_running;
    }

    pub fn reset_timer(&mut self) {
        self.timer_seconds = 0;
        self.timer_running = false;
    }

    pub fn timer_tick(&mut self) {
        if self.timer_running {
            self.timer_seconds += 1;
        }
    }

    pub fn format_timer(&self) -> String {
        let mins = self.timer_seconds / 60;
        let secs = self.timer_seconds % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    // =========================================================================
    // DISPLAY HELPERS
    // =========================================================================

    pub fn display_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled.qdeck")
            .to_string()
    }

    pub fn slide_indicator(&self) -> String {
        format!("Slide {} of {}", self.current_slide + 1, self.slides.len())
    }

    // =========================================================================
    // SAVE AS HELPERS
    // =========================================================================

    pub fn save_as_insert(&mut self, c: char) {
        self.save_as_input.insert(self.save_as_cursor, c);
        self.save_as_cursor += 1;
    }

    pub fn save_as_backspace(&mut self) {
        if self.save_as_cursor > 0 {
            self.save_as_cursor -= 1;
            self.save_as_input.remove(self.save_as_cursor);
        }
    }

    pub fn save_as_full_path(&self, cwd: &std::path::Path) -> PathBuf {
        let input = &self.save_as_input;
        if input.starts_with('/') {
            PathBuf::from(input)
        } else if let Some(rest) = input.strip_prefix("~/") {
            // Simple tilde expansion
            if let Some(home) = dirs::home_dir() {
                home.join(rest)
            } else {
                cwd.join(input)
            }
        } else {
            cwd.join(input)
        }
    }
}
