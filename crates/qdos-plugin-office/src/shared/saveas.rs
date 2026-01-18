//! Save As Dialog
//!
//! Reusable Save As dialog for office applications.

use crossterm::event::{KeyCode, KeyEvent};
use qdos_plugin_api::ui::ModalFrame;
use qdos_plugin_api::ThemeColors;
use ratatui::{layout::Rect, style::Style, text::Span, Frame};
use std::path::{Path, PathBuf};

// =============================================================================
// SAVE AS STATE
// =============================================================================

/// State for the Save As dialog
#[derive(Debug, Clone, Default)]
pub struct SaveAsState {
    /// User input for filename
    pub input: String,
    /// Cursor position in input
    pub cursor: usize,
    /// Base directory for saving
    pub base_dir: PathBuf,
    /// Suggested extension (e.g., "csv", "xlsx")
    pub default_ext: String,
}

impl SaveAsState {
    /// Create a new Save As state
    pub fn new(base_dir: PathBuf, default_ext: &str) -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            base_dir,
            default_ext: default_ext.to_string(),
        }
    }

    /// Get the full path for the current input
    pub fn full_path(&self) -> PathBuf {
        let mut path = self.base_dir.clone();
        let filename = if self.input.is_empty() {
            format!("untitled.{}", self.default_ext)
        } else if Path::new(&self.input).extension().is_none() {
            format!("{}.{}", self.input, self.default_ext)
        } else {
            self.input.clone()
        };
        path.push(filename);
        path
    }

    /// Insert character at cursor
    pub fn insert(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += 1;
    }

    /// Delete character before cursor
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.input.remove(self.cursor);
        }
    }

    /// Delete character at cursor
    pub fn delete(&mut self) {
        if self.cursor < self.input.len() {
            self.input.remove(self.cursor);
        }
    }

    /// Move cursor left
    pub fn cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor right
    pub fn cursor_right(&mut self) {
        if self.cursor < self.input.len() {
            self.cursor += 1;
        }
    }

    /// Move cursor to start
    pub fn cursor_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end
    pub fn cursor_end(&mut self) {
        self.cursor = self.input.len();
    }
}

// =============================================================================
// SAVE AS RESULT
// =============================================================================

/// Result of handling a Save As key
#[derive(Debug, Clone)]
pub enum SaveAsResult {
    /// Continue editing
    Continue,
    /// User confirmed save with path
    Save(PathBuf),
    /// User cancelled
    Cancel,
}

// =============================================================================
// KEY HANDLING
// =============================================================================

/// Handle a key event in the Save As dialog
pub fn handle_save_as_key(state: &mut SaveAsState, key: KeyEvent) -> SaveAsResult {
    match key.code {
        KeyCode::Enter => {
            if !state.input.is_empty() {
                SaveAsResult::Save(state.full_path())
            } else {
                SaveAsResult::Continue
            }
        }
        KeyCode::Esc => SaveAsResult::Cancel,
        KeyCode::Backspace => {
            state.backspace();
            SaveAsResult::Continue
        }
        KeyCode::Delete => {
            state.delete();
            SaveAsResult::Continue
        }
        KeyCode::Left => {
            state.cursor_left();
            SaveAsResult::Continue
        }
        KeyCode::Right => {
            state.cursor_right();
            SaveAsResult::Continue
        }
        KeyCode::Home => {
            state.cursor_home();
            SaveAsResult::Continue
        }
        KeyCode::End => {
            state.cursor_end();
            SaveAsResult::Continue
        }
        KeyCode::Tab => {
            // Tab completion
            if let Some(completed) = tab_complete(&state.input, &state.base_dir) {
                state.input = completed;
                state.cursor = state.input.len();
            }
            SaveAsResult::Continue
        }
        KeyCode::Char(c) => {
            // Filter out invalid filename characters
            if is_valid_filename_char(c) {
                state.insert(c);
            }
            SaveAsResult::Continue
        }
        _ => SaveAsResult::Continue,
    }
}

/// Check if character is valid for filenames
fn is_valid_filename_char(c: char) -> bool {
    !matches!(
        c,
        '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'
    )
}

/// Tab completion for filenames
pub fn tab_complete(input: &str, base_dir: &Path) -> Option<String> {
    if input.is_empty() {
        return None;
    }

    // List files in base directory
    let entries = std::fs::read_dir(base_dir).ok()?;
    let input_lower = input.to_lowercase();

    // Find first matching entry
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.to_lowercase().starts_with(&input_lower) {
            return Some(name);
        }
    }

    None
}

// =============================================================================
// MODAL RENDERING
// =============================================================================

/// Draw the Save As modal
pub fn draw_save_as_modal(
    frame: &mut Frame,
    area: Rect,
    state: &SaveAsState,
    colors: &ThemeColors,
) {
    // Calculate centered modal area
    let width = area.width.min(60);
    let height = 12;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    let modal = ModalFrame::themed(modal_area, " SAVE AS ", colors);
    modal.render_frame(frame);

    let grey = Style::default().fg(colors.grey());
    let normal = Style::default().fg(colors.fg());
    let label = Style::default().fg(colors.green());
    let input_style = Style::default().fg(colors.yellow()).bg(colors.red());

    // Directory row
    let dir_display = state.base_dir.to_string_lossy();
    let dir_truncated = if dir_display.len() > 40 {
        format!("...{}", &dir_display[dir_display.len() - 37..])
    } else {
        dir_display.to_string()
    };
    modal.render_row(
        frame,
        0,
        vec![
            Span::styled("Directory: ", grey),
            Span::styled(dir_truncated, normal),
        ],
    );

    // Filename input row
    let input_display = format!("{}█", state.input);
    modal.render_row(
        frame,
        2,
        vec![
            Span::styled("Filename:  ", label),
            Span::styled(input_display, input_style),
        ],
    );

    // Extension hint
    let ext_hint = format!("(Default extension: .{})", state.default_ext);
    modal.render_row(frame, 4, vec![Span::styled(ext_hint, grey)]);

    // Full path preview
    let preview = state.full_path();
    let preview_str = preview.to_string_lossy();
    let preview_truncated = if preview_str.len() > 50 {
        format!("...{}", &preview_str[preview_str.len() - 47..])
    } else {
        preview_str.to_string()
    };
    modal.render_row(
        frame,
        5,
        vec![
            Span::styled("Will save: ", grey),
            Span::styled(preview_truncated, normal),
        ],
    );

    // Help footer
    modal.render_help(
        frame,
        vec![("Tab", "complete"), ("Enter", "save"), ("Esc", "cancel")],
    );
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_as_state() {
        let mut state = SaveAsState::new(PathBuf::from("/tmp"), "csv");

        state.insert('t');
        state.insert('e');
        state.insert('s');
        state.insert('t');

        assert_eq!(state.input, "test");
        assert_eq!(state.cursor, 4);

        let path = state.full_path();
        assert_eq!(path, PathBuf::from("/tmp/test.csv"));
    }

    #[test]
    fn test_save_as_with_extension() {
        let mut state = SaveAsState::new(PathBuf::from("/tmp"), "csv");
        state.input = "data.xlsx".to_string();

        let path = state.full_path();
        assert_eq!(path, PathBuf::from("/tmp/data.xlsx"));
    }

    #[test]
    fn test_valid_filename_chars() {
        assert!(is_valid_filename_char('a'));
        assert!(is_valid_filename_char('1'));
        assert!(is_valid_filename_char('.'));
        assert!(is_valid_filename_char('-'));
        assert!(!is_valid_filename_char('/'));
        assert!(!is_valid_filename_char('*'));
    }
}
