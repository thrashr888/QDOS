//! Q-PAINT State Types
//!
//! Canvas, tools, and application state for the sixel-based pixel art editor.

use std::path::PathBuf;

/// DOS 16-color palette (CGA/EGA standard colors)
pub const DOS_PALETTE: [(u8, u8, u8); 16] = [
    (0, 0, 0),       // 0: Black
    (0, 0, 170),     // 1: Blue
    (0, 170, 0),     // 2: Green
    (0, 170, 170),   // 3: Cyan
    (170, 0, 0),     // 4: Red
    (170, 0, 170),   // 5: Magenta
    (170, 85, 0),    // 6: Brown
    (170, 170, 170), // 7: Light Gray
    (85, 85, 85),    // 8: Dark Gray
    (85, 85, 255),   // 9: Light Blue
    (85, 255, 85),   // A: Light Green
    (85, 255, 255),  // B: Light Cyan
    (255, 85, 85),   // C: Light Red
    (255, 85, 255),  // D: Light Magenta
    (255, 255, 85),  // E: Yellow
    (255, 255, 255), // F: White
];

/// Current view/mode in Q-PAINT
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QPaintView {
    #[default]
    Editor,
    Palette,
    FileMenu,
    Help,
}

/// Drawing tools (in-scope: Pencil, Brush, Line, Eraser, ColorPicker, Select, Text)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Pencil,
    Brush,
    Line,
    Eraser,
    ColorPicker,
    Select,
    Text,
}

impl Tool {
    /// Get display name for tool
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Pencil => "Pencil",
            Tool::Brush => "Brush",
            Tool::Line => "Line",
            Tool::Eraser => "Eraser",
            Tool::ColorPicker => "Picker",
            Tool::Select => "Select",
            Tool::Text => "Text",
        }
    }

    /// Get keyboard shortcut
    pub fn key(&self) -> char {
        match self {
            Tool::Pencil => 'P',
            Tool::Brush => 'B',
            Tool::Line => 'L',
            Tool::Eraser => 'E',
            Tool::ColorPicker => 'I',
            Tool::Select => 'S',
            Tool::Text => 'T',
        }
    }

    /// Get all available tools
    pub fn all() -> &'static [Tool] {
        &[
            Tool::Pencil,
            Tool::Brush,
            Tool::Line,
            Tool::Eraser,
            Tool::ColorPicker,
            Tool::Select,
            Tool::Text,
        ]
    }
}

/// Pixel canvas
#[derive(Debug, Clone)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    /// RGB pixel data (width * height * 3 bytes)
    pub pixels: Vec<u8>,
}

impl Canvas {
    /// Create a new canvas with the given dimensions
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height * 3) as usize;
        Self {
            width,
            height,
            pixels: vec![0; size], // Black background
        }
    }

    /// Get pixel color at (x, y)
    pub fn get_pixel(&self, x: u32, y: u32) -> (u8, u8, u8) {
        if x >= self.width || y >= self.height {
            return (0, 0, 0);
        }
        let idx = ((y * self.width + x) * 3) as usize;
        (self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2])
    }

    /// Set pixel color at (x, y)
    pub fn set_pixel(&mut self, x: u32, y: u32, color: (u8, u8, u8)) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 3) as usize;
        self.pixels[idx] = color.0;
        self.pixels[idx + 1] = color.1;
        self.pixels[idx + 2] = color.2;
    }

    /// Fill entire canvas with a color
    #[allow(dead_code)]
    pub fn fill(&mut self, color: (u8, u8, u8)) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.set_pixel(x, y, color);
            }
        }
    }

    /// Get a copy of pixel data (for undo)
    pub fn snapshot(&self) -> Vec<u8> {
        self.pixels.clone()
    }

    /// Restore from a snapshot
    pub fn restore(&mut self, snapshot: &[u8]) {
        if snapshot.len() == self.pixels.len() {
            self.pixels.copy_from_slice(snapshot);
        }
    }
}

/// File menu mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileMode {
    #[default]
    Open,
    Save,
    New,
}

/// Selection state
#[derive(Debug, Clone, Default)]
pub struct Selection {
    pub start_x: u32,
    pub start_y: u32,
    pub end_x: u32,
    pub end_y: u32,
    pub active: bool,
}

impl Selection {
    /// Get normalized bounds (min_x, min_y, max_x, max_y)
    pub fn bounds(&self) -> (u32, u32, u32, u32) {
        let min_x = self.start_x.min(self.end_x);
        let max_x = self.start_x.max(self.end_x);
        let min_y = self.start_y.min(self.end_y);
        let max_y = self.start_y.max(self.end_y);
        (min_x, min_y, max_x, max_y)
    }

    /// Get width and height
    pub fn size(&self) -> (u32, u32) {
        let (min_x, min_y, max_x, max_y) = self.bounds();
        (max_x - min_x + 1, max_y - min_y + 1)
    }
}

/// Main Q-PAINT application state
#[derive(Debug, Clone)]
pub struct QPaintState {
    /// Current view
    pub view: QPaintView,
    /// The canvas
    pub canvas: Canvas,

    // Tool state
    pub tool: Tool,
    pub brush_size: u8,
    pub fg_color: (u8, u8, u8),
    pub bg_color: (u8, u8, u8),

    // Cursor/viewport
    pub cursor_x: u32,
    pub cursor_y: u32,
    pub zoom: u8,
    pub scroll_x: u32,
    pub scroll_y: u32,

    // Selection
    pub selection: Selection,
    pub clipboard: Option<(u32, u32, Vec<u8>)>, // (width, height, pixels)

    // Drawing state
    #[allow(dead_code)]
    pub drawing: bool,
    pub shape_start: Option<(u32, u32)>,

    // History
    pub undo_stack: Vec<Vec<u8>>,
    pub redo_stack: Vec<Vec<u8>>,
    pub max_undo: usize,

    // Palette
    pub palette: Vec<(u8, u8, u8)>,
    pub palette_index: usize,

    // File
    pub file_path: Option<PathBuf>,
    pub modified: bool,
    pub file_mode: FileMode,
    pub file_list: Vec<String>,
    pub file_selected: usize,
    pub file_input: String,

    // Status message
    pub status: String,
}

impl Default for QPaintState {
    fn default() -> Self {
        Self::new(32, 32)
    }
}

impl QPaintState {
    /// Create a new Q-PAINT state with default canvas size
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            view: QPaintView::Editor,
            canvas: Canvas::new(width, height),

            tool: Tool::Pencil,
            brush_size: 1,
            fg_color: DOS_PALETTE[15], // White
            bg_color: DOS_PALETTE[0],  // Black

            cursor_x: width / 2,
            cursor_y: height / 2,
            zoom: 4,
            scroll_x: 0,
            scroll_y: 0,

            selection: Selection::default(),
            clipboard: None,

            drawing: false,
            shape_start: None,

            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo: 50,

            palette: DOS_PALETTE.to_vec(),
            palette_index: 15,

            file_path: None,
            modified: false,
            file_mode: FileMode::Open,
            file_list: Vec::new(),
            file_selected: 0,
            file_input: String::new(),

            status: String::new(),
        }
    }

    /// Save current state to undo stack
    pub fn save_undo(&mut self) {
        let snapshot = self.canvas.snapshot();
        self.undo_stack.push(snapshot);
        if self.undo_stack.len() > self.max_undo {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.modified = true;
    }

    /// Undo last action
    pub fn undo(&mut self) -> bool {
        if let Some(snapshot) = self.undo_stack.pop() {
            let current = self.canvas.snapshot();
            self.redo_stack.push(current);
            self.canvas.restore(&snapshot);
            true
        } else {
            false
        }
    }

    /// Redo last undone action
    pub fn redo(&mut self) -> bool {
        if let Some(snapshot) = self.redo_stack.pop() {
            let current = self.canvas.snapshot();
            self.undo_stack.push(current);
            self.canvas.restore(&snapshot);
            true
        } else {
            false
        }
    }

    /// Move cursor with bounds checking
    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let new_x = (self.cursor_x as i32 + dx).clamp(0, self.canvas.width as i32 - 1) as u32;
        let new_y = (self.cursor_y as i32 + dy).clamp(0, self.canvas.height as i32 - 1) as u32;
        self.cursor_x = new_x;
        self.cursor_y = new_y;
    }

    /// Set foreground color from palette index
    pub fn set_fg_from_palette(&mut self, index: usize) {
        if index < self.palette.len() {
            self.fg_color = self.palette[index];
            self.palette_index = index;
        }
    }

    /// Set background color from palette index
    pub fn set_bg_from_palette(&mut self, index: usize) {
        if index < self.palette.len() {
            self.bg_color = self.palette[index];
        }
    }

    /// Zoom in (max 16x for sixel detail work)
    pub fn zoom_in(&mut self) {
        if self.zoom < 16 {
            self.zoom *= 2;
        }
    }

    /// Zoom out (min 1x)
    pub fn zoom_out(&mut self) {
        if self.zoom > 1 {
            self.zoom /= 2;
        }
    }

    /// Copy selection to clipboard
    pub fn copy_selection(&mut self) {
        if !self.selection.active {
            return;
        }

        let (min_x, min_y, max_x, max_y) = self.selection.bounds();
        let width = max_x - min_x + 1;
        let height = max_y - min_y + 1;
        let mut pixels = Vec::with_capacity((width * height * 3) as usize);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let (r, g, b) = self.canvas.get_pixel(x, y);
                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
            }
        }

        self.clipboard = Some((width, height, pixels));
    }

    /// Paste clipboard at cursor
    pub fn paste(&mut self) {
        if let Some((width, height, ref pixels)) = self.clipboard.clone() {
            self.save_undo();
            let start_x = self.cursor_x;
            let start_y = self.cursor_y;

            for dy in 0..height {
                for dx in 0..width {
                    let x = start_x + dx;
                    let y = start_y + dy;
                    if x < self.canvas.width && y < self.canvas.height {
                        let idx = ((dy * width + dx) * 3) as usize;
                        let color = (pixels[idx], pixels[idx + 1], pixels[idx + 2]);
                        self.canvas.set_pixel(x, y, color);
                    }
                }
            }
        }
    }

    /// Clear selection area with background color
    pub fn clear_selection(&mut self) {
        if !self.selection.active {
            return;
        }

        self.save_undo();
        let (min_x, min_y, max_x, max_y) = self.selection.bounds();

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                self.canvas.set_pixel(x, y, self.bg_color);
            }
        }

        self.selection.active = false;
    }

    /// Start a new canvas
    pub fn new_canvas(&mut self, width: u32, height: u32) {
        self.canvas = Canvas::new(width, height);
        self.cursor_x = width / 2;
        self.cursor_y = height / 2;
        self.scroll_x = 0;
        self.scroll_y = 0;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.file_path = None;
        self.modified = false;
        self.selection = Selection::default();
    }
}
