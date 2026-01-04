//! File Viewer Plugin for R-DOS
//!
//! Provides file viewing functionality including:
//! - Normal/ASCII view with syntax highlighting
//! - Hex view
//! - Image view (Kitty/Sixel/iTerm2 protocol detection)
//! - Markdown view
//! - Git blame view
//! - Git diff view
//! - Git history navigation

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem};
use crate::plugins::git::ops::{
    load_file_at_commit, load_file_blame, load_file_diff_against_head, load_file_history,
};
use crate::plugins::git::{BlameLine, FileHistoryEntry};
use crate::ui::{COLOR_BLUE, COLOR_FG, COLOR_GREEN, COLOR_RED};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;
use std::any::Any;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

// Lazy-loaded syntax highlighting resources
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

// Lazy-loaded image picker (detects Kitty/Sixel/iTerm2 protocols)
static IMAGE_PICKER: OnceLock<Mutex<Picker>> = OnceLock::new();

fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn get_theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

/// Get or initialize the image picker with terminal protocol detection
fn get_image_picker() -> &'static Mutex<Picker> {
    IMAGE_PICKER.get_or_init(|| {
        let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::from_fontsize((8, 16)));
        Mutex::new(picker)
    })
}

/// Get or create image protocol for the given content
fn get_or_create_image_protocol(content: &[u8]) -> Option<StatefulProtocol> {
    if let Ok(dyn_img) = image::load_from_memory(content) {
        if let Ok(mut picker) = get_image_picker().lock() {
            return Some(picker.new_resize_protocol(dyn_img));
        }
    }
    None
}

/// Convert syntect color to ratatui color
fn syntect_to_ratatui_color(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

/// File viewer display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Normal,
    Hex,
    Image,
    Markdown,
    Blame,
    Diff,
}

/// File viewer filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewFilter {
    #[default]
    Off,
    Ascii,
    WordStar,
}

impl ViewFilter {
    pub fn next(&self) -> ViewFilter {
        match self {
            ViewFilter::Off => ViewFilter::Ascii,
            ViewFilter::Ascii => ViewFilter::WordStar,
            ViewFilter::WordStar => ViewFilter::Off,
        }
    }
}

/// File viewer state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerState {
    /// File name being viewed
    pub file_name: String,
    /// Full file path
    pub file_path: PathBuf,
    /// File contents as bytes
    pub content: Vec<u8>,
    /// Current display mode
    pub mode: ViewMode,
    /// Current filter mode
    pub filter: ViewFilter,
    /// Current scroll offset
    pub scroll_offset: usize,
    /// Whether cursor is on hex side in hex mode
    pub hex_side: bool,
    /// Git history for this file (oldest to newest)
    pub git_history: Vec<FileHistoryEntry>,
    /// Current position in git history (None = working copy)
    pub history_index: Option<usize>,
    /// Whether we're in a git repo
    pub is_git_repo: bool,
    /// Git blame data for blame view
    pub blame_lines: Vec<BlameLine>,
    /// Git diff lines for diff view
    pub diff_lines: Vec<String>,
}

impl ViewerState {
    pub fn new(file_name: String, file_path: PathBuf, content: Vec<u8>) -> Self {
        let mode = Self::detect_mode(&file_name);
        Self {
            file_name,
            file_path,
            content,
            mode,
            filter: ViewFilter::Off,
            scroll_offset: 0,
            hex_side: true,
            git_history: Vec::new(),
            history_index: None,
            is_git_repo: false,
            blame_lines: Vec::new(),
            diff_lines: Vec::new(),
        }
    }

    /// Check if we can go to an older version
    pub fn has_older_version(&self) -> bool {
        if self.git_history.is_empty() {
            return false;
        }
        match self.history_index {
            None => true,
            Some(idx) => idx > 0,
        }
    }

    /// Check if we can go to a newer version
    pub fn has_newer_version(&self) -> bool {
        if self.git_history.is_empty() {
            return false;
        }
        self.history_index.is_some()
    }

    /// Get current commit info (None if viewing working copy)
    pub fn current_commit(&self) -> Option<&FileHistoryEntry> {
        self.history_index.and_then(|idx| self.git_history.get(idx))
    }

    /// Set git history for this file
    pub fn set_git_history(&mut self, history: Vec<FileHistoryEntry>, is_git_repo: bool) {
        self.git_history = history;
        self.is_git_repo = is_git_repo;
    }

    /// Calculate max scroll offset based on mode and visible height
    pub fn max_scroll(&self, visible_height: usize) -> usize {
        match self.mode {
            ViewMode::Normal | ViewMode::Markdown => {
                let line_count = self.content.split(|&b| b == b'\n').count();
                line_count.saturating_sub(visible_height)
            }
            ViewMode::Hex => {
                let bytes_per_line = 16;
                let total_lines = self.content.len().div_ceil(bytes_per_line);
                total_lines.saturating_sub(visible_height)
            }
            ViewMode::Image => 0,
            ViewMode::Blame => self.blame_lines.len().saturating_sub(visible_height),
            ViewMode::Diff => self.diff_lines.len().saturating_sub(visible_height),
        }
    }

    /// Detect the best view mode based on file extension
    pub fn detect_mode(file_name: &str) -> ViewMode {
        let lower = file_name.to_lowercase();
        if lower.ends_with(".md") || lower.ends_with(".markdown") {
            ViewMode::Markdown
        } else if lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".gif")
            || lower.ends_with(".bmp")
            || lower.ends_with(".webp")
            || lower.ends_with(".ico")
        {
            ViewMode::Image
        } else {
            ViewMode::Normal
        }
    }
}

/// File Viewer plugin
pub struct ViewerPlugin {
    modal_open: bool,
    state: Option<ViewerState>,
    current_cwd: PathBuf,
}

impl ViewerPlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: None,
            current_cwd: PathBuf::new(),
        }
    }

    /// Open the viewer with a file
    pub fn open_file(&mut self, file_path: PathBuf, cwd: &PathBuf) -> Result<(), String> {
        let content =
            std::fs::read(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;

        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let mut state = ViewerState::new(file_name, file_path.clone(), content);

        // Load git history if in a git repo
        let history = load_file_history(&file_path, cwd);
        let is_git_repo = !history.is_empty();
        state.set_git_history(history, is_git_repo);

        self.state = Some(state);
        self.modal_open = true;
        self.current_cwd = cwd.clone();

        Ok(())
    }

    /// Get the current state for clipboard operations
    pub fn get_state(&self) -> Option<&ViewerState> {
        self.state.as_ref()
    }

    /// Check if the viewer is currently open
    pub fn is_open(&self) -> bool {
        self.modal_open
    }
}

impl Default for ViewerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ViewerPlugin {
    fn id(&self) -> &str {
        "viewer"
    }

    fn name(&self) -> &str {
        "File Viewer"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn init(&mut self, cwd: &PathBuf) -> Result<(), String> {
        self.current_cwd = cwd.clone();
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "View".to_string(),
            key: 'V',
            description: "View file contents".to_string(),
            priority: 30,
        })
    }

    fn handle_global_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // F3 opens viewer (but file selection is handled by app)
        if key.code == KeyCode::F(3) {
            // The app handles file selection and calls open_file
            KeyHandleResult::NotHandled
        } else {
            KeyHandleResult::NotHandled
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::CloseModal,
        };

        let max_scroll = state.max_scroll(20);

        match key.code {
            KeyCode::Esc => {
                self.modal_open = false;
                self.state = None;
                return KeyHandleResult::CloseModal;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                state.scroll_offset = state.scroll_offset.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                state.scroll_offset = (state.scroll_offset + 1).min(max_scroll);
            }
            KeyCode::PageUp => {
                state.scroll_offset = state.scroll_offset.saturating_sub(20);
            }
            KeyCode::PageDown => {
                state.scroll_offset = (state.scroll_offset + 20).min(max_scroll);
            }
            KeyCode::Home => {
                state.scroll_offset = 0;
            }
            KeyCode::End => {
                state.scroll_offset = max_scroll;
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                state.mode = ViewMode::Hex;
                state.scroll_offset = state.scroll_offset.min(state.max_scroll(20));
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('a') | KeyCode::Char('A') => {
                state.mode = ViewMode::Normal;
                state.scroll_offset = state.scroll_offset.min(state.max_scroll(20));
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                state.mode = ViewMode::Image;
                state.scroll_offset = 0;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                state.mode = ViewMode::Markdown;
                state.scroll_offset = state.scroll_offset.min(state.max_scroll(20));
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                state.filter = state.filter.next();
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                if state.is_git_repo {
                    if state.blame_lines.is_empty() {
                        state.blame_lines = load_file_blame(&state.file_path, cwd);
                    }
                    state.mode = ViewMode::Blame;
                    state.scroll_offset = 0;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if state.is_git_repo {
                    if state.diff_lines.is_empty() {
                        state.diff_lines = load_file_diff_against_head(&state.file_path, cwd);
                    }
                    state.mode = ViewMode::Diff;
                    state.scroll_offset = 0;
                }
            }
            KeyCode::F(4) => {
                if state.mode == ViewMode::Hex {
                    state.hex_side = !state.hex_side;
                }
            }
            KeyCode::Left => {
                // Go to older version in git history
                if state.has_older_version() {
                    let new_idx = match state.history_index {
                        None => state.git_history.len() - 1,
                        Some(idx) => idx.saturating_sub(1),
                    };
                    if let Some(entry) = state.git_history.get(new_idx) {
                        let commit_hash = entry.hash.clone();
                        if let Ok(content) =
                            load_file_at_commit(&state.file_path, &commit_hash, cwd)
                        {
                            state.content = content;
                            state.history_index = Some(new_idx);
                            state.scroll_offset = 0;
                        }
                    }
                }
            }
            KeyCode::Right => {
                // Go to newer version in git history
                if state.has_newer_version() {
                    if let Some(idx) = state.history_index {
                        if idx + 1 >= state.git_history.len() {
                            // Go to working copy
                            if let Ok(content) = std::fs::read(&state.file_path) {
                                state.content = content;
                                state.history_index = None;
                                state.scroll_offset = 0;
                            }
                        } else {
                            let new_idx = idx + 1;
                            if let Some(entry) = state.git_history.get(new_idx) {
                                let commit_hash = entry.hash.clone();
                                if let Ok(content) =
                                    load_file_at_commit(&state.file_path, &commit_hash, cwd)
                                {
                                    state.content = content;
                                    state.history_index = Some(new_idx);
                                    state.scroll_offset = 0;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        KeyHandleResult::Handled
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect) {
        let state = match &self.state {
            Some(s) => s,
            None => return,
        };

        // Clear the entire screen
        frame.render_widget(Clear, area);

        // Layout: title bar, separator, content, separator, status/help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title bar
                Constraint::Length(1), // Separator
                Constraint::Min(5),    // Content area
                Constraint::Length(1), // Separator
                Constraint::Length(1), // Status/help line
            ])
            .split(area);

        // Title bar
        let mode_str = match state.mode {
            ViewMode::Normal => "NORMAL",
            ViewMode::Hex => "HEX",
            ViewMode::Image => "IMAGE",
            ViewMode::Markdown => "MARKDOWN",
            ViewMode::Blame => "BLAME",
            ViewMode::Diff => "DIFF",
        };
        let filter_str = match state.filter {
            ViewFilter::Off => "",
            ViewFilter::Ascii => " [Filter: ASCII]",
            ViewFilter::WordStar => " [Filter: W/S]",
        };

        let version_str = if let Some(entry) = state.current_commit() {
            let short_hash = &entry.hash[..7.min(entry.hash.len())];
            let short_msg = if entry.message.len() > 25 {
                format!("{}...", &entry.message[..22])
            } else {
                entry.message.clone()
            };
            format!("  [{}] {} - {}", short_hash, entry.date, short_msg)
        } else if state.is_git_repo && !state.git_history.is_empty() {
            "  [working copy]".to_string()
        } else {
            String::new()
        };

        let title = format!(
            " VIEW: {}  Mode: {}{}{}",
            state.file_name.to_uppercase(),
            mode_str,
            filter_str,
            version_str
        );
        frame.render_widget(
            Paragraph::new(Span::styled(
                title,
                Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
            )),
            chunks[0],
        );

        // Separator
        let sep = "═".repeat(area.width as usize);
        frame.render_widget(
            Paragraph::new(Span::styled(&sep, Style::default().fg(COLOR_FG))),
            chunks[1],
        );

        // Content area
        let content_height = chunks[2].height as usize;
        match state.mode {
            ViewMode::Normal => self.draw_normal_view(frame, chunks[2], state, content_height),
            ViewMode::Hex => self.draw_hex_view(frame, chunks[2], state, content_height),
            ViewMode::Image => self.draw_image_view(frame, chunks[2], state),
            ViewMode::Markdown => self.draw_markdown_view(frame, chunks[2], state, content_height),
            ViewMode::Blame => self.draw_blame_view(frame, chunks[2], state, content_height),
            ViewMode::Diff => self.draw_diff_view(frame, chunks[2], state, content_height),
        }

        // Separator
        frame.render_widget(
            Paragraph::new(Span::styled(&sep, Style::default().fg(COLOR_FG))),
            chunks[3],
        );

        // Help line
        let mut help_spans = vec![
            Span::styled(" H", Style::default().fg(COLOR_BLUE)),
            Span::raw("ex "),
            Span::styled("N", Style::default().fg(COLOR_BLUE)),
            Span::raw("ormal "),
            Span::styled("I", Style::default().fg(COLOR_BLUE)),
            Span::raw("mage "),
            Span::styled("M", Style::default().fg(COLOR_BLUE)),
            Span::raw("arkdown "),
        ];

        if state.is_git_repo {
            help_spans.push(Span::styled("B", Style::default().fg(COLOR_BLUE)));
            help_spans.push(Span::raw("lame "));
            help_spans.push(Span::styled("D", Style::default().fg(COLOR_BLUE)));
            help_spans.push(Span::raw("iff "));
        }

        help_spans.push(Span::styled("F", Style::default().fg(COLOR_BLUE)));
        help_spans.push(Span::raw("ilter "));
        help_spans.push(Span::styled("↑↓", Style::default().fg(COLOR_BLUE)));
        help_spans.push(Span::raw(" scroll "));

        if state.has_older_version() {
            help_spans.push(Span::styled("←", Style::default().fg(COLOR_BLUE)));
            help_spans.push(Span::raw(" older "));
        }
        if state.has_newer_version() {
            help_spans.push(Span::styled("→", Style::default().fg(COLOR_BLUE)));
            help_spans.push(Span::raw(" newer "));
        }

        help_spans.push(Span::styled("Esc", Style::default().fg(COLOR_BLUE)));
        help_spans.push(Span::raw(" exit"));

        frame.render_widget(Paragraph::new(Line::from(help_spans)), chunks[4]);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "V or Enter - View File".to_string(),
            "  View the contents of any file on the screen".to_string(),
            "  H - Hex view".to_string(),
            "  N/A - Normal/ASCII view".to_string(),
            "  I - Image view".to_string(),
            "  M - Markdown view".to_string(),
            "  B - Git blame view".to_string(),
            "  D - Git diff view".to_string(),
            "  F - Toggle filter".to_string(),
            "  ←/→ - Navigate git history".to_string(),
            "  ↑↓ PgUp PgDn - Scroll".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Drawing helper methods
impl ViewerPlugin {
    fn draw_normal_view(&self, frame: &mut Frame, area: Rect, state: &ViewerState, height: usize) {
        let content_str = String::from_utf8_lossy(&state.content);
        let ss = get_syntax_set();
        let ts = get_theme_set();

        let syntax = ss
            .find_syntax_for_file(&state.file_name)
            .ok()
            .flatten()
            .filter(|s| s.name != "Plain Text");

        let all_lines: Vec<&str> = content_str.lines().collect();
        let max_scroll = all_lines.len().saturating_sub(height);
        let scroll = state.scroll_offset.min(max_scroll);

        let visible_lines: Vec<Line> = if let Some(syntax) = syntax {
            let theme = &ts.themes["base16-ocean.dark"];
            let mut highlighter = HighlightLines::new(syntax, theme);

            all_lines
                .iter()
                .skip(scroll)
                .take(height)
                .map(|line| {
                    let highlighted = highlighter.highlight_line(line, ss).unwrap_or_default();
                    let mut spans: Vec<Span> = vec![Span::raw(" ")];

                    for (style, text) in highlighted {
                        let fg = syntect_to_ratatui_color(style.foreground);
                        let mut ratatui_style = Style::default().fg(fg);

                        if style
                            .font_style
                            .contains(syntect::highlighting::FontStyle::BOLD)
                        {
                            ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                        }
                        if style
                            .font_style
                            .contains(syntect::highlighting::FontStyle::ITALIC)
                        {
                            ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                        }
                        if style
                            .font_style
                            .contains(syntect::highlighting::FontStyle::UNDERLINE)
                        {
                            ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
                        }

                        spans.push(Span::styled(text.to_string(), ratatui_style));
                    }

                    Line::from(spans)
                })
                .collect()
        } else {
            let lines: Vec<String> = state
                .content
                .split(|&b| b == b'\n')
                .map(|line| {
                    line.iter()
                        .map(|&b| match state.filter {
                            ViewFilter::Off => {
                                if (32..127).contains(&b) {
                                    b as char
                                } else if b == b'\t' || b == b'\r' {
                                    ' '
                                } else {
                                    '.'
                                }
                            }
                            ViewFilter::Ascii => {
                                if (32..127).contains(&b) {
                                    b as char
                                } else {
                                    ' '
                                }
                            }
                            ViewFilter::WordStar => {
                                let b = b & 0x7F;
                                if (32..127).contains(&b) {
                                    b as char
                                } else {
                                    ' '
                                }
                            }
                        })
                        .collect::<String>()
                })
                .collect();

            lines
                .iter()
                .skip(scroll)
                .take(height)
                .map(|line| {
                    Line::from(Span::styled(
                        format!(" {}", line),
                        Style::default().fg(COLOR_FG),
                    ))
                })
                .collect()
        };

        frame.render_widget(Paragraph::new(visible_lines), area);
    }

    fn draw_hex_view(&self, frame: &mut Frame, area: Rect, state: &ViewerState, height: usize) {
        let bytes_per_line: usize = 16;
        let total_lines = state.content.len().div_ceil(bytes_per_line);
        let max_scroll = total_lines.saturating_sub(height);
        let scroll = state.scroll_offset.min(max_scroll);

        let mut lines: Vec<Line> = Vec::new();

        for line_idx in scroll..(scroll + height).min(total_lines) {
            let offset = line_idx * bytes_per_line;
            let end = (offset + bytes_per_line).min(state.content.len());
            let chunk = &state.content[offset..end];

            let mut spans = Vec::new();

            spans.push(Span::styled(
                format!(" {:08X}  ", offset),
                Style::default().fg(COLOR_BLUE),
            ));

            for (i, &byte) in chunk.iter().enumerate() {
                if i == 8 {
                    spans.push(Span::raw(" "));
                }
                spans.push(Span::styled(
                    format!("{:02X} ", byte),
                    Style::default().fg(COLOR_FG),
                ));
            }

            for i in chunk.len()..bytes_per_line {
                if i == 8 {
                    spans.push(Span::raw(" "));
                }
                spans.push(Span::raw("   "));
            }

            spans.push(Span::raw("  "));
            let ascii: String = chunk
                .iter()
                .map(|&b| {
                    if (32..127).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            spans.push(Span::styled(ascii, Style::default().fg(COLOR_GREEN)));

            lines.push(Line::from(spans));
        }

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn draw_image_view(&self, frame: &mut Frame, area: Rect, state: &ViewerState) {
        if let Some(mut protocol) = get_or_create_image_protocol(&state.content) {
            let image_widget = StatefulImage::new(None);
            frame.render_stateful_widget(image_widget, area, &mut protocol);
        } else {
            let error_msg = vec![
                Line::from(""),
                Line::from(Span::styled(
                    " Cannot display image",
                    Style::default().fg(COLOR_RED).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!(" File: {}", state.file_path.display()),
                    Style::default().fg(COLOR_GREEN),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    " Press N for normal view or H for hex view",
                    Style::default().fg(COLOR_BLUE),
                )),
            ];
            frame.render_widget(Paragraph::new(error_msg), area);
        }
    }

    fn draw_markdown_view(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &ViewerState,
        height: usize,
    ) {
        let content_str = String::from_utf8_lossy(&state.content);
        let mut lines: Vec<Line> = Vec::new();

        for line in content_str.lines() {
            if line.starts_with("# ") {
                lines.push(Line::from(Span::styled(
                    format!(" {}", &line[2..]),
                    Style::default().fg(COLOR_BLUE).add_modifier(Modifier::BOLD),
                )));
            } else if line.starts_with("## ") {
                lines.push(Line::from(Span::styled(
                    format!(" {}", &line[3..]),
                    Style::default().fg(COLOR_BLUE).add_modifier(Modifier::BOLD),
                )));
            } else if line.starts_with("### ") {
                lines.push(Line::from(Span::styled(
                    format!(" {}", &line[4..]),
                    Style::default().fg(COLOR_BLUE),
                )));
            } else if line.starts_with("#### ")
                || line.starts_with("##### ")
                || line.starts_with("###### ")
            {
                let header_content = line.trim_start_matches('#').trim_start();
                lines.push(Line::from(Span::styled(
                    format!(" {}", header_content),
                    Style::default().fg(COLOR_BLUE),
                )));
            } else if line.starts_with("```") {
                lines.push(Line::from(Span::styled(
                    " ─────────────────────────────────────",
                    Style::default().fg(COLOR_GREEN),
                )));
            } else if line.starts_with("- ") || line.starts_with("* ") {
                lines.push(Line::from(Span::styled(
                    format!("  • {}", &line[2..]),
                    Style::default().fg(COLOR_FG),
                )));
            } else if line.chars().next().is_some_and(|c| c.is_ascii_digit()) && line.contains(". ")
            {
                lines.push(Line::from(Span::styled(
                    format!("  {}", line),
                    Style::default().fg(COLOR_FG),
                )));
            } else if line.starts_with("> ") {
                lines.push(Line::from(Span::styled(
                    format!(" │ {}", &line[2..]),
                    Style::default().fg(COLOR_GREEN),
                )));
            } else if line == "---" || line == "***" || line == "___" {
                lines.push(Line::from(Span::styled(
                    " ═════════════════════════════════════",
                    Style::default().fg(COLOR_FG),
                )));
            } else if line.contains("**") || line.contains("__") {
                let clean_line = line.replace("**", "").replace("__", "");
                lines.push(Line::from(Span::styled(
                    format!(" {}", clean_line),
                    Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
                )));
            } else if line.contains('*') || line.contains('_') {
                let clean_line = line
                    .chars()
                    .filter(|&c| c != '*' && c != '_')
                    .collect::<String>();
                lines.push(Line::from(Span::styled(
                    format!(" {}", clean_line),
                    Style::default().fg(COLOR_FG).add_modifier(Modifier::ITALIC),
                )));
            } else if line.contains('`') {
                lines.push(Line::from(Span::styled(
                    format!(" {}", line),
                    Style::default().fg(COLOR_GREEN),
                )));
            } else if line.trim().is_empty() {
                lines.push(Line::from(""));
            } else {
                lines.push(Line::from(Span::styled(
                    format!(" {}", line),
                    Style::default().fg(COLOR_FG),
                )));
            }
        }

        let max_scroll = lines.len().saturating_sub(height);
        let scroll = state.scroll_offset.min(max_scroll);
        let visible_lines: Vec<Line> = lines.into_iter().skip(scroll).take(height).collect();

        frame.render_widget(Paragraph::new(visible_lines), area);
    }

    fn draw_blame_view(&self, frame: &mut Frame, area: Rect, state: &ViewerState, height: usize) {
        if state.blame_lines.is_empty() {
            let error_msg = vec![
                Line::from(""),
                Line::from(Span::styled(
                    " No blame data available",
                    Style::default().fg(COLOR_RED),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    " File may not be tracked by git",
                    Style::default().fg(COLOR_FG),
                )),
            ];
            frame.render_widget(Paragraph::new(error_msg), area);
            return;
        }

        let max_scroll = state.blame_lines.len().saturating_sub(height);
        let scroll = state.scroll_offset.min(max_scroll);

        let visible_lines: Vec<Line> = state
            .blame_lines
            .iter()
            .enumerate()
            .skip(scroll)
            .take(height)
            .map(|(line_num, blame)| {
                let author = if blame.author.len() > 10 {
                    format!("{}…", &blame.author[..9])
                } else {
                    format!("{:10}", blame.author)
                };

                let spans = vec![
                    Span::styled(
                        format!(" {:>4} ", line_num + 1),
                        Style::default().fg(COLOR_BLUE),
                    ),
                    Span::styled(
                        format!("{} ", blame.hash),
                        Style::default().fg(Color::Rgb(128, 128, 128)),
                    ),
                    Span::styled(author, Style::default().fg(COLOR_GREEN)),
                    Span::styled(
                        format!(" {:>8} │ ", blame.time_ago),
                        Style::default().fg(Color::Rgb(128, 128, 128)),
                    ),
                    Span::styled(&blame.line_content, Style::default().fg(COLOR_FG)),
                ];

                Line::from(spans)
            })
            .collect();

        frame.render_widget(Paragraph::new(visible_lines), area);
    }

    fn draw_diff_view(&self, frame: &mut Frame, area: Rect, state: &ViewerState, height: usize) {
        if state.diff_lines.is_empty() {
            let error_msg = vec![
                Line::from(""),
                Line::from(Span::styled(
                    " No diff data available",
                    Style::default().fg(COLOR_FG),
                )),
            ];
            frame.render_widget(Paragraph::new(error_msg), area);
            return;
        }

        let max_scroll = state.diff_lines.len().saturating_sub(height);
        let scroll = state.scroll_offset.min(max_scroll);
        let color_cyan = Color::Rgb(0, 170, 170);

        let visible_lines: Vec<Line> = state
            .diff_lines
            .iter()
            .skip(scroll)
            .take(height)
            .map(|line| {
                let (style, prefix) = if line.starts_with('+') && !line.starts_with("+++") {
                    (Style::default().fg(COLOR_GREEN), "+")
                } else if line.starts_with('-') && !line.starts_with("---") {
                    (Style::default().fg(COLOR_RED), "-")
                } else if line.starts_with("@@") {
                    (Style::default().fg(color_cyan), "@")
                } else if line.starts_with("diff ") || line.starts_with("index ") {
                    (Style::default().fg(COLOR_BLUE), " ")
                } else if line.starts_with("+++") || line.starts_with("---") {
                    (Style::default().fg(COLOR_BLUE), " ")
                } else {
                    (Style::default().fg(COLOR_FG), " ")
                };

                Line::from(vec![
                    Span::styled(format!(" {} ", prefix), style),
                    Span::styled(line.as_str(), style),
                ])
            })
            .collect();

        frame.render_widget(Paragraph::new(visible_lines), area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewer_plugin_creation() {
        let plugin = ViewerPlugin::new();
        assert_eq!(plugin.id(), "viewer");
        assert!(!plugin.modal_open);
    }

    #[test]
    fn test_view_mode_detection() {
        assert_eq!(ViewerState::detect_mode("test.md"), ViewMode::Markdown);
        assert_eq!(ViewerState::detect_mode("test.png"), ViewMode::Image);
        assert_eq!(ViewerState::detect_mode("test.jpg"), ViewMode::Image);
        assert_eq!(ViewerState::detect_mode("test.rs"), ViewMode::Normal);
    }

    #[test]
    fn test_view_filter_cycle() {
        assert_eq!(ViewFilter::Off.next(), ViewFilter::Ascii);
        assert_eq!(ViewFilter::Ascii.next(), ViewFilter::WordStar);
        assert_eq!(ViewFilter::WordStar.next(), ViewFilter::Off);
    }
}
