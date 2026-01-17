//! Q-DESIGN state and data structures

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// =============================================================================
// VIEWS
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QDesignView {
    #[default]
    TemplateSelect,
    Canvas,
    TextEdit,
    Export,
    Help,
}

// =============================================================================
// TOOLS
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Select,
    TextFrame,
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::TextFrame => "Text",
        }
    }

    pub fn all() -> Vec<Tool> {
        vec![Tool::Select, Tool::TextFrame]
    }
}

// =============================================================================
// TEXT ALIGNMENT
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextAlignment {
    #[default]
    Left,
    Center,
    Right,
}

impl TextAlignment {
    pub fn name(&self) -> &'static str {
        match self {
            TextAlignment::Left => "Left",
            TextAlignment::Center => "Center",
            TextAlignment::Right => "Right",
        }
    }
}

// =============================================================================
// FRAME
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub text: String,
    pub alignment: TextAlignment,
    pub border: bool,
}

impl Frame {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
            text: String::new(),
            alignment: TextAlignment::Center,
            border: true,
        }
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.text = text.to_string();
        self
    }

    pub fn with_alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn with_border(mut self, border: bool) -> Self {
        self.border = border;
        self
    }
}

// =============================================================================
// PAGE
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub width: u16,
    pub height: u16,
    pub frames: Vec<Frame>,
}

impl Page {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            frames: Vec::new(),
        }
    }

    pub fn add_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
}

// =============================================================================
// TEMPLATES
// =============================================================================

#[derive(Debug, Clone)]
pub struct Template {
    pub name: &'static str,
    pub description: &'static str,
    pub width: u16,
    pub height: u16,
    pub frames: Vec<Frame>,
}

impl Template {
    pub fn all() -> Vec<Template> {
        vec![
            Template {
                name: "Birthday Card",
                description: "4x6 greeting card",
                width: 60,
                height: 20,
                frames: vec![
                    Frame::new(5, 3, 50, 3)
                        .with_text("HAPPY BIRTHDAY!")
                        .with_alignment(TextAlignment::Center),
                    Frame::new(5, 10, 50, 5)
                        .with_text("Wishing you a wonderful day!")
                        .with_alignment(TextAlignment::Center),
                ],
            },
            Template {
                name: "Business Card",
                description: "3.5x2 standard card",
                width: 50,
                height: 12,
                frames: vec![
                    Frame::new(2, 2, 46, 2)
                        .with_text("Your Name Here")
                        .with_alignment(TextAlignment::Center),
                    Frame::new(2, 5, 46, 2)
                        .with_text("Title / Position")
                        .with_alignment(TextAlignment::Center),
                    Frame::new(2, 8, 46, 2)
                        .with_text("email@example.com")
                        .with_alignment(TextAlignment::Center),
                ],
            },
            Template {
                name: "Flyer",
                description: "8.5x11 full page",
                width: 70,
                height: 22,
                frames: vec![
                    Frame::new(5, 2, 60, 3)
                        .with_text("EVENT TITLE")
                        .with_alignment(TextAlignment::Center),
                    Frame::new(5, 7, 60, 8)
                        .with_text("Event details go here...")
                        .with_alignment(TextAlignment::Left),
                    Frame::new(5, 17, 60, 3)
                        .with_text("Date: TBD  Location: TBD")
                        .with_alignment(TextAlignment::Center),
                ],
            },
            Template {
                name: "Banner",
                description: "Large horizontal banner",
                width: 76,
                height: 10,
                frames: vec![Frame::new(3, 2, 70, 6)
                    .with_text("YOUR MESSAGE HERE")
                    .with_alignment(TextAlignment::Center)],
            },
            Template {
                name: "Blank",
                description: "Start from scratch",
                width: 70,
                height: 20,
                frames: Vec::new(),
            },
        ]
    }
}

// =============================================================================
// STATE
// =============================================================================

#[derive(Debug)]
pub struct QDesignState {
    pub view: QDesignView,
    pub tool: Tool,

    // Document
    pub pages: Vec<Page>,
    pub current_page: usize,
    pub title: String,
    pub file_path: Option<PathBuf>,
    pub modified: bool,

    // Selection
    pub selected_frame: Option<usize>,
    pub cursor_x: u16,
    pub cursor_y: u16,

    // Frame creation
    pub creating_frame: bool,
    pub create_start_x: u16,
    pub create_start_y: u16,

    // Templates
    pub templates: Vec<Template>,
    pub template_cursor: usize,

    // Text editing
    pub text_edit_buffer: String,
    pub text_cursor: usize,

    // Export
    pub export_path: String,

    // Config
    pub designs_path: PathBuf,

    // Status
    pub status_message: Option<String>,
}

impl Default for QDesignState {
    fn default() -> Self {
        Self::new()
    }
}

impl QDesignState {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rdos")
            .join("qdesign");

        Self {
            view: QDesignView::TemplateSelect,
            tool: Tool::Select,
            pages: Vec::new(),
            current_page: 0,
            title: "Untitled".to_string(),
            file_path: None,
            modified: false,
            selected_frame: None,
            cursor_x: 0,
            cursor_y: 0,
            creating_frame: false,
            create_start_x: 0,
            create_start_y: 0,
            templates: Template::all(),
            template_cursor: 0,
            text_edit_buffer: String::new(),
            text_cursor: 0,
            export_path: "design.txt".to_string(),
            designs_path: config_dir.join("designs"),
            status_message: None,
        }
    }

    // =========================================================================
    // PAGE OPERATIONS
    // =========================================================================

    pub fn current_page(&self) -> Option<&Page> {
        self.pages.get(self.current_page)
    }

    pub fn current_page_mut(&mut self) -> Option<&mut Page> {
        self.pages.get_mut(self.current_page)
    }

    pub fn create_from_template(&mut self, template_idx: usize) {
        if let Some(template) = self.templates.get(template_idx) {
            let mut page = Page::new(template.width, template.height);
            for frame in &template.frames {
                page.add_frame(frame.clone());
            }
            self.pages = vec![page];
            self.current_page = 0;
            self.title = template.name.to_string();
            self.modified = false;
            self.selected_frame = None;
            self.cursor_x = 0;
            self.cursor_y = 0;
        }
    }

    // =========================================================================
    // FRAME OPERATIONS
    // =========================================================================

    pub fn selected_frame_ref(&self) -> Option<&Frame> {
        let frame_idx = self.selected_frame?;
        let page = self.current_page()?;
        page.frames.get(frame_idx)
    }

    pub fn add_text_frame(&mut self, x: u16, y: u16, width: u16, height: u16) {
        let current_page = self.current_page;
        if let Some(page) = self.pages.get_mut(current_page) {
            let frame = Frame::new(x, y, width, height);
            page.frames.push(frame);
            self.selected_frame = Some(page.frames.len() - 1);
            self.modified = true;
        }
    }

    pub fn delete_selected_frame(&mut self) {
        let selected = match self.selected_frame {
            Some(idx) => idx,
            None => return,
        };
        let current_page = self.current_page;
        if let Some(page) = self.pages.get_mut(current_page) {
            if selected < page.frames.len() {
                page.frames.remove(selected);
                self.modified = true;
                self.selected_frame = None;
            }
        }
    }

    pub fn move_selected_frame(&mut self, dx: i16, dy: i16) {
        let selected = match self.selected_frame {
            Some(idx) => idx,
            None => return,
        };
        let current_page = self.current_page;
        if let Some(page) = self.pages.get_mut(current_page) {
            if let Some(frame) = page.frames.get_mut(selected) {
                let new_x = (frame.x as i16 + dx).max(0) as u16;
                let new_y = (frame.y as i16 + dy).max(0) as u16;
                // Clamp to page bounds
                frame.x = new_x.min(page.width.saturating_sub(frame.width));
                frame.y = new_y.min(page.height.saturating_sub(frame.height));
                self.modified = true;
            }
        }
    }

    pub fn start_text_edit(&mut self) {
        if let Some(frame) = self.selected_frame_ref() {
            self.text_edit_buffer = frame.text.clone();
            self.text_cursor = self.text_edit_buffer.len();
            self.view = QDesignView::TextEdit;
        }
    }

    pub fn apply_text_edit(&mut self) {
        let selected = match self.selected_frame {
            Some(idx) => idx,
            None => return,
        };
        let new_text = self.text_edit_buffer.clone();
        let current_page = self.current_page;
        if let Some(page) = self.pages.get_mut(current_page) {
            if let Some(frame) = page.frames.get_mut(selected) {
                frame.text = new_text;
                self.modified = true;
            }
        }
        self.view = QDesignView::Canvas;
    }

    pub fn cancel_text_edit(&mut self) {
        self.text_edit_buffer.clear();
        self.text_cursor = 0;
        self.view = QDesignView::Canvas;
    }

    pub fn cycle_alignment(&mut self) {
        let selected = match self.selected_frame {
            Some(idx) => idx,
            None => return,
        };
        let current_page = self.current_page;
        if let Some(page) = self.pages.get_mut(current_page) {
            if let Some(frame) = page.frames.get_mut(selected) {
                frame.alignment = match frame.alignment {
                    TextAlignment::Left => TextAlignment::Center,
                    TextAlignment::Center => TextAlignment::Right,
                    TextAlignment::Right => TextAlignment::Left,
                };
                self.modified = true;
            }
        }
    }

    pub fn toggle_border(&mut self) {
        let selected = match self.selected_frame {
            Some(idx) => idx,
            None => return,
        };
        let current_page = self.current_page;
        if let Some(page) = self.pages.get_mut(current_page) {
            if let Some(frame) = page.frames.get_mut(selected) {
                frame.border = !frame.border;
                self.modified = true;
            }
        }
    }

    // =========================================================================
    // SELECTION
    // =========================================================================

    pub fn select_frame_at(&mut self, x: u16, y: u16) -> bool {
        if let Some(page) = self.current_page() {
            // Iterate in reverse to select topmost frame
            for (i, frame) in page.frames.iter().enumerate().rev() {
                if x >= frame.x
                    && x < frame.x + frame.width
                    && y >= frame.y
                    && y < frame.y + frame.height
                {
                    self.selected_frame = Some(i);
                    return true;
                }
            }
        }
        self.selected_frame = None;
        false
    }

    pub fn select_next_frame(&mut self) {
        if let Some(page) = self.current_page() {
            if page.frames.is_empty() {
                return;
            }
            let current = self.selected_frame.unwrap_or(0);
            self.selected_frame = Some((current + 1) % page.frames.len());
        }
    }

    pub fn select_prev_frame(&mut self) {
        if let Some(page) = self.current_page() {
            if page.frames.is_empty() {
                return;
            }
            let len = page.frames.len();
            let current = self.selected_frame.unwrap_or(0);
            self.selected_frame = Some((current + len - 1) % len);
        }
    }

    // =========================================================================
    // NAVIGATION
    // =========================================================================

    pub fn template_cursor_up(&mut self) {
        if self.template_cursor > 0 {
            self.template_cursor -= 1;
        }
    }

    pub fn template_cursor_down(&mut self) {
        if self.template_cursor + 1 < self.templates.len() {
            self.template_cursor += 1;
        }
    }

    pub fn cycle_tool(&mut self) {
        let tools = Tool::all();
        let current = tools.iter().position(|t| *t == self.tool).unwrap_or(0);
        self.tool = tools[(current + 1) % tools.len()];
    }

    pub fn move_cursor(&mut self, dx: i16, dy: i16) {
        // Copy page dimensions to avoid borrow conflict
        let (page_width, page_height) = match self.current_page() {
            Some(page) => (page.width, page.height),
            None => return,
        };
        let new_x = (self.cursor_x as i16 + dx).max(0) as u16;
        let new_y = (self.cursor_y as i16 + dy).max(0) as u16;
        self.cursor_x = new_x.min(page_width.saturating_sub(1));
        self.cursor_y = new_y.min(page_height.saturating_sub(1));
    }

    // =========================================================================
    // EXPORT
    // =========================================================================

    pub fn export_ascii(&self, path: &str) -> Result<String, String> {
        let page = self.current_page().ok_or("No page to export")?;

        // Get dimensions
        let width = page.width as usize;
        let height = page.height as usize;

        // Create a character buffer
        let mut buffer: Vec<Vec<char>> = vec![vec![' '; width]; height];

        // Draw page border (top and bottom)
        for (i, cell) in buffer[0].iter_mut().enumerate() {
            if i < width {
                *cell = '-';
            }
        }
        for (i, cell) in buffer[height - 1].iter_mut().enumerate() {
            if i < width {
                *cell = '-';
            }
        }
        for row in buffer.iter_mut() {
            row[0] = '|';
            row[width - 1] = '|';
        }
        buffer[0][0] = '+';
        buffer[0][width - 1] = '+';
        buffer[height - 1][0] = '+';
        buffer[height - 1][width - 1] = '+';

        // Draw frames
        for frame in &page.frames {
            // Draw border if enabled
            if frame.border {
                for x in frame.x..frame.x + frame.width {
                    if (frame.y as usize) < buffer.len() && (x as usize) < width {
                        buffer[frame.y as usize][x as usize] = '-';
                    }
                    let bottom_y = frame.y + frame.height - 1;
                    if (bottom_y as usize) < buffer.len() && (x as usize) < width {
                        buffer[bottom_y as usize][x as usize] = '-';
                    }
                }
                for y in frame.y..frame.y + frame.height {
                    if (y as usize) < buffer.len() && (frame.x as usize) < width {
                        buffer[y as usize][frame.x as usize] = '|';
                    }
                    let right_x = frame.x + frame.width - 1;
                    if (y as usize) < buffer.len() && (right_x as usize) < width {
                        buffer[y as usize][right_x as usize] = '|';
                    }
                }
                // Corners
                if (frame.y as usize) < buffer.len() && (frame.x as usize) < width {
                    buffer[frame.y as usize][frame.x as usize] = '+';
                }
                let right_x = frame.x + frame.width - 1;
                if (frame.y as usize) < buffer.len() && (right_x as usize) < width {
                    buffer[frame.y as usize][right_x as usize] = '+';
                }
                let bottom_y = frame.y + frame.height - 1;
                if (bottom_y as usize) < buffer.len() && (frame.x as usize) < width {
                    buffer[bottom_y as usize][frame.x as usize] = '+';
                }
                if (bottom_y as usize) < buffer.len() && (right_x as usize) < width {
                    buffer[bottom_y as usize][right_x as usize] = '+';
                }
            }

            // Draw text
            if !frame.text.is_empty() {
                let inner_width = if frame.border {
                    frame.width.saturating_sub(2)
                } else {
                    frame.width
                } as usize;
                let inner_x = if frame.border { frame.x + 1 } else { frame.x } as usize;
                let inner_y = if frame.border { frame.y + 1 } else { frame.y } as usize;

                let text_chars: Vec<char> = frame.text.chars().collect();
                let text_len = text_chars.len().min(inner_width);

                let start_x = match frame.alignment {
                    TextAlignment::Left => inner_x,
                    TextAlignment::Center => inner_x + (inner_width.saturating_sub(text_len)) / 2,
                    TextAlignment::Right => inner_x + inner_width.saturating_sub(text_len),
                };

                if inner_y < buffer.len() {
                    for (i, c) in text_chars.iter().take(inner_width).enumerate() {
                        let x = start_x + i;
                        if x < width {
                            buffer[inner_y][x] = *c;
                        }
                    }
                }
            }
        }

        // Convert buffer to string
        let output: String = buffer
            .iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        std::fs::write(path, &output).map_err(|e| e.to_string())?;
        Ok(format!("Exported to {}", path))
    }
}
