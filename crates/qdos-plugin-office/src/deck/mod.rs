//! Q-DECK - Presentation Editor Plugin
//!
//! ANSI/ASCII slideshow editor inspired by demo scene aesthetics.
//! Features slide templates, presentation mode, and sixel image support.

pub mod image;
mod modal;
pub mod state;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qdos_plugin_api::ThemeColors;
use qdos_plugin_api::{KeyHandleResult, Plugin, PluginCapabilities};
use ratatui::{layout::Rect, Frame};
use state::{DeckMode, DeckState, SlideTemplate};
use std::any::Any;
use std::path::PathBuf;

// =============================================================================
// DECK PLUGIN
// =============================================================================

pub struct DeckPlugin {
    pub state: Option<DeckState>,
}

impl Default for DeckPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DeckPlugin {
    pub fn new() -> Self {
        Self { state: None }
    }

    pub fn launch(&mut self) {
        self.state = Some(DeckState::new());
    }

    pub fn load_file(&mut self, path: &PathBuf) -> Result<(), String> {
        let state = load_qdeck(path)?;
        self.state = Some(state);
        Ok(())
    }

    // =========================================================================
    // KEY HANDLING
    // =========================================================================

    pub fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let Some(state) = self.state.as_mut() else {
            return KeyHandleResult::NotHandled;
        };

        match state.mode {
            DeckMode::Edit => self.handle_edit_key(key, cwd),
            DeckMode::Present => self.handle_present_key(key),
            DeckMode::SlideList => self.handle_slide_list_key(key),
            DeckMode::Menu => self.handle_menu_key(key, cwd),
            DeckMode::SaveAs => self.handle_save_as_key(key, cwd),
            DeckMode::Help => {
                // Any key exits help
                if let Some(state) = self.state.as_mut() {
                    state.mode = DeckMode::Edit;
                }
                KeyHandleResult::Handled
            }
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            // Navigation between slides
            KeyCode::Left | KeyCode::PageUp => {
                state.prev_slide();
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::PageDown => {
                state.next_slide();
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                state.first_slide();
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                state.last_slide();
                KeyHandleResult::Handled
            }

            // Presentation mode
            KeyCode::F(5) => {
                state.enter_present_mode();
                KeyHandleResult::Handled
            }

            // Slide list/sorter
            KeyCode::F(6) => {
                state.enter_slide_list_mode();
                KeyHandleResult::Handled
            }

            // New slide
            KeyCode::Insert => {
                state.add_slide(SlideTemplate::Bullets);
                KeyHandleResult::Handled
            }

            // Delete slide
            KeyCode::Delete => {
                state.delete_slide();
                KeyHandleResult::Handled
            }

            // Duplicate slide
            KeyCode::Char('d') if ctrl => {
                state.duplicate_slide();
                KeyHandleResult::Handled
            }

            // Cycle template
            KeyCode::Tab => {
                let current_idx = SlideTemplate::all()
                    .iter()
                    .position(|t| *t == state.current().template)
                    .unwrap_or(0);
                let next_idx = (current_idx + 1) % SlideTemplate::all().len();
                state.current_mut().template = SlideTemplate::all()[next_idx];
                state.modified = true;
                KeyHandleResult::Handled
            }

            // Save
            KeyCode::Char('s') if ctrl => {
                if let Some(path) = state.file_path.clone() {
                    match save_qdeck(state, &path) {
                        Ok(()) => {
                            state.modified = false;
                            state.status_message = Some(("Saved".to_string(), 30));
                        }
                        Err(e) => {
                            state.status_message = Some((format!("Error: {}", e), 60));
                        }
                    }
                } else {
                    state.enter_save_as_mode();
                }
                KeyHandleResult::Handled
            }

            // Timer toggle
            KeyCode::Char('t') if ctrl => {
                state.toggle_timer();
                KeyHandleResult::Handled
            }

            // Close
            KeyCode::Esc => KeyHandleResult::CloseModal,

            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_present_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            // Next slide
            KeyCode::Right
            | KeyCode::Down
            | KeyCode::PageDown
            | KeyCode::Char(' ')
            | KeyCode::Enter => {
                state.next_slide();
                KeyHandleResult::Handled
            }

            // Previous slide
            KeyCode::Left | KeyCode::Up | KeyCode::PageUp | KeyCode::Backspace => {
                state.prev_slide();
                KeyHandleResult::Handled
            }

            // First/last slide
            KeyCode::Home => {
                state.first_slide();
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                state.last_slide();
                KeyHandleResult::Handled
            }

            // Go to specific slide (1-9)
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let num = c.to_digit(10).unwrap_or(0) as usize;
                if num > 0 && num <= state.slides.len() {
                    state.goto_slide(num - 1);
                }
                KeyHandleResult::Handled
            }

            // Exit presentation
            KeyCode::Esc | KeyCode::Char('q') => {
                state.exit_present_mode();
                KeyHandleResult::Handled
            }

            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_slide_list_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Up => {
                state.prev_slide();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                state.next_slide();
                KeyHandleResult::Handled
            }

            // Move slide in order
            KeyCode::Char('k') => {
                state.move_slide_up();
                KeyHandleResult::Handled
            }
            KeyCode::Char('j') => {
                state.move_slide_down();
                KeyHandleResult::Handled
            }

            // New slide
            KeyCode::Insert => {
                state.add_slide(SlideTemplate::Bullets);
                KeyHandleResult::Handled
            }

            // Delete slide
            KeyCode::Delete => {
                state.delete_slide();
                KeyHandleResult::Handled
            }

            // Edit selected slide
            KeyCode::Enter => {
                state.exit_slide_list_mode();
                KeyHandleResult::Handled
            }

            // Back to edit
            KeyCode::Esc => {
                state.exit_slide_list_mode();
                KeyHandleResult::Handled
            }

            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Esc => {
                state.exit_menu_mode();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_save_as_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Char(c) if !c.is_control() => {
                if !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                    state.save_as_insert(c);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                state.save_as_backspace();
                KeyHandleResult::Handled
            }

            KeyCode::Enter => {
                if !state.save_as_input.is_empty() {
                    let mut path = state.save_as_full_path(cwd);

                    // Add .qdeck extension if missing
                    if path.extension().is_none() {
                        path.set_extension("qdeck");
                    }

                    state.exit_save_as_mode();

                    match save_qdeck(state, &path) {
                        Ok(()) => {
                            state.file_path = Some(path);
                            state.modified = false;
                            state.status_message = Some(("Saved".to_string(), 30));
                        }
                        Err(e) => {
                            state.status_message = Some((format!("Error: {}", e), 60));
                        }
                    }
                }
                KeyHandleResult::Handled
            }

            KeyCode::Esc => {
                state.exit_save_as_mode();
                KeyHandleResult::Handled
            }

            _ => KeyHandleResult::Handled,
        }
    }

    // =========================================================================
    // RENDERING
    // =========================================================================

    pub fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        if let Some(state) = &self.state {
            modal::draw_deck_modal(frame, area, state, colors);
        }
    }

    pub fn tick(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.tick_count = state.tick_count.wrapping_add(1);

            // Timer tick (assuming ~10 ticks per second)
            if state.tick_count % 10 == 0 {
                state.timer_tick();
            }

            // Update transition
            if state.transition_progress < 1.0 {
                state.transition_progress = (state.transition_progress + 0.1).min(1.0);
            }

            // Decrement status message
            if let Some((_, ticks)) = &mut state.status_message {
                if *ticks > 0 {
                    *ticks -= 1;
                } else {
                    state.status_message = None;
                }
            }
        }
    }
}

// =============================================================================
// FILE FORMAT
// =============================================================================

/// Load a .qdeck presentation file (Markdown-based format)
pub fn load_qdeck(path: &PathBuf) -> Result<DeckState, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let mut state = DeckState::new();
    state.file_path = Some(path.clone());
    state.slides.clear();

    // Simple markdown-based parser
    // Slides are separated by ---
    // # Title for slide titles
    // * bullets
    // ```code blocks```

    let mut current_slide = state::Slide::default();
    let mut in_code_block = false;
    let mut code_content = String::new();
    let mut code_lang = String::new();
    let mut in_frontmatter = false;
    let mut frontmatter_done = false;

    for line in content.lines() {
        // Handle frontmatter
        if line == "---" && !frontmatter_done {
            if !in_frontmatter {
                in_frontmatter = true;
                continue;
            } else {
                in_frontmatter = false;
                frontmatter_done = true;
                continue;
            }
        }

        if in_frontmatter {
            if let Some(rest) = line.strip_prefix("title:") {
                state.title = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("author:") {
                state.author = rest.trim().to_string();
            }
            continue;
        }

        // Slide separator (non-frontmatter ---)
        if line == "---" && frontmatter_done {
            if !current_slide.title.is_empty() || !current_slide.content.is_empty() {
                state.slides.push(current_slide);
            }
            current_slide = state::Slide::default();
            continue;
        }

        // Code block
        if line.starts_with("```") {
            if in_code_block {
                // End code block
                current_slide.content.push(state::ContentBlock::Code {
                    language: code_lang.clone(),
                    content: code_content.trim_end().to_string(),
                });
                code_content.clear();
                code_lang.clear();
                in_code_block = false;
            } else {
                // Start code block
                in_code_block = true;
                code_lang = line.strip_prefix("```").unwrap_or("").to_string();
            }
            continue;
        }

        if in_code_block {
            code_content.push_str(line);
            code_content.push('\n');
            continue;
        }

        // Title (# Heading)
        if let Some(rest) = line.strip_prefix("# ") {
            current_slide.title = rest.to_string();
            continue;
        }

        // Subtitle (## Subheading on title slide)
        if let Some(rest) = line.strip_prefix("## ") {
            if current_slide.title.is_empty() {
                current_slide.title = rest.to_string();
            } else {
                current_slide.subtitle = Some(rest.to_string());
            }
            continue;
        }

        // Template comment
        if line.starts_with("<!-- template:") {
            let template_str = line
                .strip_prefix("<!-- template:")
                .unwrap_or("")
                .strip_suffix("-->")
                .unwrap_or("")
                .trim()
                .to_lowercase();

            current_slide.template = match template_str.as_str() {
                "title" => SlideTemplate::Title,
                "bullets" => SlideTemplate::Bullets,
                "twocol" | "two-col" => SlideTemplate::TwoCol,
                "image" => SlideTemplate::Image,
                "code" => SlideTemplate::Code,
                "quote" => SlideTemplate::Quote,
                _ => SlideTemplate::Bullets,
            };
            continue;
        }

        // Bullet
        if let Some(rest) = line.strip_prefix("* ").or_else(|| line.strip_prefix("- ")) {
            // Add to existing bullets or create new
            if let Some(state::ContentBlock::Bullets(ref mut items)) =
                current_slide.content.last_mut()
            {
                items.push(rest.to_string());
            } else {
                current_slide
                    .content
                    .push(state::ContentBlock::Bullets(vec![rest.to_string()]));
            }
            continue;
        }

        // Numbered item
        if line
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            if let Some(rest) = line.split(". ").nth(1) {
                if let Some(state::ContentBlock::Numbered(ref mut items)) =
                    current_slide.content.last_mut()
                {
                    items.push(rest.to_string());
                } else {
                    current_slide
                        .content
                        .push(state::ContentBlock::Numbered(vec![rest.to_string()]));
                }
                continue;
            }
        }

        // Quote (> text)
        if let Some(rest) = line.strip_prefix("> ") {
            current_slide.content.push(state::ContentBlock::Quote {
                text: rest.to_string(),
                author: String::new(),
            });
            continue;
        }

        // Plain text (non-empty lines)
        if !line.trim().is_empty() {
            current_slide.content.push(state::ContentBlock::Text {
                content: line.to_string(),
                bold: false,
                italic: false,
                color: None,
            });
        }
    }

    // Add last slide
    if !current_slide.title.is_empty() || !current_slide.content.is_empty() {
        state.slides.push(current_slide);
    }

    // Ensure at least one slide
    if state.slides.is_empty() {
        state
            .slides
            .push(state::Slide::title_slide("New Presentation", None));
    }

    state.modified = false;
    Ok(state)
}

/// Save a presentation to .qdeck format
pub fn save_qdeck(state: &DeckState, path: &PathBuf) -> Result<(), String> {
    let mut output = String::new();

    // Frontmatter
    output.push_str("---\n");
    output.push_str(&format!("title: {}\n", state.title));
    if !state.author.is_empty() {
        output.push_str(&format!("author: {}\n", state.author));
    }
    output.push_str(&format!("theme: {}\n", state.theme.name.to_lowercase()));
    output.push_str("---\n\n");

    // Slides
    for (i, slide) in state.slides.iter().enumerate() {
        if i > 0 {
            output.push_str("\n---\n\n");
        }

        // Title
        output.push_str(&format!("# {}\n", slide.title));

        // Template comment
        output.push_str(&format!(
            "<!-- template: {} -->\n",
            slide.template.name().to_lowercase()
        ));

        // Subtitle
        if let Some(ref subtitle) = slide.subtitle {
            output.push_str(&format!("## {}\n", subtitle));
        }

        output.push('\n');

        // Content
        for block in &slide.content {
            match block {
                state::ContentBlock::Bullets(items) => {
                    for item in items {
                        output.push_str(&format!("* {}\n", item));
                    }
                    output.push('\n');
                }
                state::ContentBlock::Numbered(items) => {
                    for (i, item) in items.iter().enumerate() {
                        output.push_str(&format!("{}. {}\n", i + 1, item));
                    }
                    output.push('\n');
                }
                state::ContentBlock::Text { content, .. } => {
                    output.push_str(content);
                    output.push_str("\n\n");
                }
                state::ContentBlock::Code { language, content } => {
                    output.push_str(&format!("```{}\n", language));
                    output.push_str(content);
                    output.push_str("\n```\n\n");
                }
                state::ContentBlock::AnsiArt(art) => {
                    output.push_str("```ansi\n");
                    output.push_str(art);
                    output.push_str("\n```\n\n");
                }
                state::ContentBlock::Quote { text, author } => {
                    output.push_str(&format!("> {}\n", text));
                    if !author.is_empty() {
                        output.push_str(&format!("> - {}\n", author));
                    }
                    output.push('\n');
                }
                state::ContentBlock::Image { path, alt } => {
                    output.push_str(&format!("![{}]({})\n\n", alt, path));
                }
                state::ContentBlock::Separator => {
                    output.push_str("---\n\n");
                }
            }
        }

        // Notes
        if !slide.notes.is_empty() {
            output.push_str(&format!("<!-- notes: {} -->\n", slide.notes));
        }
    }

    std::fs::write(path, output).map_err(|e| format!("Failed to write file: {}", e))
}

// =============================================================================
// PLUGIN TRAIT
// =============================================================================

impl Plugin for DeckPlugin {
    fn id(&self) -> &str {
        "deck"
    }

    fn name(&self) -> &str {
        "Q-DECK"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: false,
            has_keys: true,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.state = Some(DeckState::new());
        Ok(())
    }

    fn handle_global_key(
        &mut self,
        _key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        DeckPlugin::handle_modal_key(self, key, cwd)
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        DeckPlugin::draw_modal(self, frame, area, colors);
    }

    fn tick(&mut self) {
        DeckPlugin::tick(self);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-DECK - Presentation Editor".to_string(),
            "".to_string(),
            "ANSI/ASCII slideshow editor inspired by".to_string(),
            "demo scene aesthetics with sixel image support.".to_string(),
            "".to_string(),
            "EDIT MODE:".to_string(),
            "  Left/Right    Navigate slides".to_string(),
            "  F5            Start presentation".to_string(),
            "  F6            Slide sorter view".to_string(),
            "  Insert        Add new slide".to_string(),
            "  Delete        Remove slide".to_string(),
            "  Tab           Cycle template".to_string(),
            "  Ctrl+D        Duplicate slide".to_string(),
            "  Ctrl+S        Save presentation".to_string(),
            "  Ctrl+T        Toggle timer".to_string(),
            "  Esc           Close Q-DECK".to_string(),
            "".to_string(),
            "PRESENTATION MODE:".to_string(),
            "  Space/Enter   Next slide".to_string(),
            "  Backspace     Previous slide".to_string(),
            "  1-9           Jump to slide".to_string(),
            "  Home/End      First/last slide".to_string(),
            "  Esc           Exit presentation".to_string(),
            "".to_string(),
            "SLIDE TEMPLATES:".to_string(),
            "  Title         Centered title/subtitle".to_string(),
            "  Bullets       Title with bullet list".to_string(),
            "  Two-Col       Two column layout".to_string(),
            "  Image         Large image/ASCII art".to_string(),
            "  Code          Code block display".to_string(),
            "  Quote         Quote with attribution".to_string(),
            "  Blank         Empty custom slide".to_string(),
            "".to_string(),
            "FILE FORMAT:".to_string(),
            "  .qdeck files are Markdown-based with".to_string(),
            "  frontmatter for metadata and --- for".to_string(),
            "  slide separators.".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
