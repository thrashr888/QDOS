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

#![allow(clippy::ptr_arg)]

use crossterm::event::{KeyCode, KeyEvent};
use qdos_plugin_api::prelude::*;
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_git::ops::{
    load_file_at_commit, load_file_blame, load_file_diff_against_head, load_file_history,
};
use qdos_plugin_git::{BlameLine, FileHistoryEntry};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;
use std::any::Any;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

// Lazy-loaded syntax highlighting resources
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

// Lazy-loaded image picker (detects Kitty/Sixel/iTerm2 protocols)
static IMAGE_PICKER: OnceLock<Mutex<Option<Picker>>> = OnceLock::new();
static IMAGE_FONT_SIZE: OnceLock<(u16, u16)> = OnceLock::new();

fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn get_theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

/// Select without terminal queries: ratatui-image 3 can leave its stdin probe
/// running after timeout. The application's event reader must own stdin alone.
fn initialize_image_picker() {
    IMAGE_PICKER.get_or_init(|| {
        let tty = io::stdin().is_terminal() && io::stdout().is_terminal();
        let term = std::env::var("TERM").unwrap_or_default();
        let program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        let direct = direct_terminal(
            tty,
            &term,
            &program,
            std::env::var_os("TMUX").is_some() || std::env::var_os("STY").is_some(),
        );
        let override_name = std::env::var("RDOS_IMAGE_PROTOCOL").unwrap_or_default();
        let font_size = if tty {
            crossterm::terminal::window_size()
                .ok()
                .map(|size| font_size_from_window(size.width, size.height, size.columns, size.rows))
                .unwrap_or((8, 16))
        } else {
            (8, 16)
        };
        IMAGE_FONT_SIZE.get_or_init(|| font_size);
        // Avoid the library's automatic tmux passthrough configuration unless the
        // user has explicitly requested a graphics protocol for that session.
        if !tty
            || override_name == "halfblocks"
            || (!direct && protocol_override(&override_name).is_none())
        {
            return Mutex::new(None);
        }
        let mut picker = Picker::from_fontsize(font_size);
        let selected = select_protocol(
            tty,
            direct,
            &term,
            &program,
            &override_name,
            picker.protocol_type(),
        );
        picker.set_protocol_type(selected);
        Mutex::new(Some(picker))
    });
}

fn font_size_from_window(width: u16, height: u16, columns: u16, rows: u16) -> (u16, u16) {
    if width == 0 || height == 0 || columns == 0 || rows == 0 {
        return (8, 16);
    }
    let size = (width / columns, height / rows);
    if size.0 == 0 || size.1 == 0 {
        (8, 16)
    } else {
        size
    }
}

fn protocol_override(name: &str) -> Option<ProtocolType> {
    match name {
        "kitty" => Some(ProtocolType::Kitty),
        "sixel" => Some(ProtocolType::Sixel),
        "iterm2" => Some(ProtocolType::Iterm2),
        "halfblocks" => Some(ProtocolType::Halfblocks),
        _ => None,
    }
}

fn select_protocol(
    tty: bool,
    direct: bool,
    term: &str,
    program: &str,
    override_name: &str,
    fallback: ProtocolType,
) -> ProtocolType {
    if !tty {
        return ProtocolType::Halfblocks;
    }
    if let Some(protocol) = protocol_override(override_name) {
        return protocol;
    }
    if !direct {
        return ProtocolType::Halfblocks;
    }
    if matches!(program, "ghostty" | "kitty") || matches!(term, "xterm-ghostty" | "xterm-kitty") {
        ProtocolType::Kitty
    } else {
        fallback
    }
}

/// Decode image content and create its terminal protocol once per cache entry.
fn create_image_protocol(content: &[u8]) -> Option<OwnedImageProtocol> {
    image_protocol_from_picker(content, IMAGE_PICKER.get())
}

fn halfblock_protocol(image: image::DynamicImage) -> StatefulProtocol {
    use ratatui_image::protocol::{halfblocks::StatefulHalfblocks, ImageSource};
    let size = IMAGE_FONT_SIZE.get().copied().unwrap_or((8, 16));
    StatefulProtocol::Halfblocks(StatefulHalfblocks::new(ImageSource::new(image, size), size))
}

fn image_protocol_from_picker(
    content: &[u8],
    picker: Option<&Mutex<Option<Picker>>>,
) -> Option<OwnedImageProtocol> {
    let image = image::load_from_memory(content).ok()?;
    // Embedders that omit startup initialization still get a safe preview.
    // Rendering must never initialize a picker or read terminal input.
    let protocol = match picker.and_then(|picker| picker.lock().ok()) {
        Some(mut picker) => match picker.as_mut() {
            Some(picker) => picker.new_resize_protocol(image),
            None => halfblock_protocol(image),
        },
        None => halfblock_protocol(image),
    };
    Some(OwnedImageProtocol::new(protocol))
}

/// A viewer-owned image; never delete another app's terminal images.
struct OwnedImageProtocol {
    protocol: StatefulProtocol,
    cleanup_id: Option<u32>,
}

impl OwnedImageProtocol {
    fn new(protocol: StatefulProtocol) -> Self {
        // ratatui-image 3 keeps its tmux routing private. Do not guess a passthrough
        // envelope or write escape sequences to redirected output.
        let term = std::env::var("TERM").unwrap_or_default();
        let program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        let direct = direct_terminal(
            io::stdout().is_terminal(),
            &term,
            &program,
            std::env::var_os("TMUX").is_some() || std::env::var_os("STY").is_some(),
        );
        let cleanup_id = match &protocol {
            StatefulProtocol::Kitty(kitty) if direct => Some(kitty.unique_id),
            _ => None,
        };
        Self {
            protocol,
            cleanup_id,
        }
    }
}

fn direct_terminal(is_terminal: bool, term: &str, program: &str, multiplexed: bool) -> bool {
    is_terminal
        && !multiplexed
        && !term.starts_with("screen")
        && !term.starts_with("tmux")
        && program != "tmux"
        && program != "screen"
}

fn delete_kitty_image(id: u32) -> String {
    // Uppercase I frees only this image's data and placements; q=2 suppresses replies.
    format!("\x1b_Ga=d,d=I,i={id},q=2;\x1b\\")
}

impl Drop for OwnedImageProtocol {
    fn drop(&mut self) {
        if let Some(id) = self.cleanup_id {
            let mut stdout = io::stdout().lock();
            let _ = stdout.write_all(delete_kitty_image(id).as_bytes());
            let _ = stdout.flush();
        }
    }
}

/// Retain protocol state across frames, including Kitty image IDs and resize state.
/// Keep failures too, so corrupt files are not decoded on every UI tick.
struct ImageCache<T> {
    entry: Option<(ViewMode, Vec<u8>, Option<T>)>,
}

impl<T> Default for ImageCache<T> {
    fn default() -> Self {
        Self { entry: None }
    }
}

impl<T> ImageCache<T> {
    fn clear(&mut self) {
        self.entry = None;
    }

    fn get_or_insert_with(
        &mut self,
        content: &[u8],
        mode: ViewMode,
        build: impl FnOnce() -> Option<T>,
    ) -> Option<&mut T> {
        let matches = self
            .entry
            .as_ref()
            .is_some_and(|(cached_mode, cached_content, _)| {
                *cached_mode == mode && cached_content == content
            });
        if !matches {
            self.entry = Some((mode, content.to_vec(), build()));
        }
        self.entry
            .as_mut()
            .and_then(|(_, _, protocol)| protocol.as_mut())
    }
}

/// Rasterize SVG content to PNG bytes
fn rasterize_svg(svg_data: &[u8]) -> Option<Vec<u8>> {
    // Parse SVG
    let tree = resvg::usvg::Tree::from_data(svg_data, &resvg::usvg::Options::default()).ok()?;

    // Get size and create pixmap
    let size = tree.size();
    let width = size.width() as u32;
    let height = size.height() as u32;

    // Limit size for performance (max 2048x2048)
    let max_dim = 2048u32;
    let (render_width, render_height) = if width > max_dim || height > max_dim {
        let scale = (max_dim as f32) / (width.max(height) as f32);
        (
            (width as f32 * scale) as u32,
            (height as f32 * scale) as u32,
        )
    } else {
        (width, height)
    };

    let mut pixmap = tiny_skia::Pixmap::new(render_width, render_height)?;

    // Render SVG
    let scale_x = render_width as f32 / width as f32;
    let scale_y = render_height as f32 / height as f32;
    let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Encode as PNG
    pixmap.encode_png().ok()
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
    Vector, // SVG files
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
            ViewMode::Image | ViewMode::Vector => 0,
            ViewMode::Blame => self.blame_lines.len().saturating_sub(visible_height),
            ViewMode::Diff => self.diff_lines.len().saturating_sub(visible_height),
        }
    }

    /// Detect the best view mode based on file extension
    pub fn detect_mode(file_name: &str) -> ViewMode {
        let lower = file_name.to_lowercase();
        if lower.ends_with(".md") || lower.ends_with(".markdown") {
            ViewMode::Markdown
        } else if lower.ends_with(".svg") {
            ViewMode::Vector
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
    image_cache: Mutex<ImageCache<OwnedImageProtocol>>,
}

impl ViewerPlugin {
    /// Select graphics once during terminal startup without reading stdin.
    /// Known direct terminals use their advertised identity; other terminals can
    /// opt in using RDOS_IMAGE_PROTOCOL. Embedders can initialize explicitly; otherwise previews use half-blocks.
    pub fn initialize_graphics() {
        initialize_image_picker();
    }

    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: None,
            current_cwd: PathBuf::new(),
            image_cache: Mutex::new(ImageCache::default()),
        }
    }

    /// Open the viewer with a file (reads content using std::fs)
    pub fn open_file(&mut self, file_path: PathBuf, cwd: &PathBuf) -> Result<(), String> {
        let content =
            std::fs::read(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;
        self.open_file_with_content(file_path, content, cwd)
    }

    /// Open the viewer with file content already read (for VFS files)
    pub fn open_file_with_content(
        &mut self,
        file_path: PathBuf,
        content: Vec<u8>,
        cwd: &PathBuf,
    ) -> Result<(), String> {
        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let mut state = ViewerState::new(file_name, file_path.clone(), content);

        // Load git history if in a git repo
        let history = load_file_history(&file_path, cwd);
        let is_git_repo = !history.is_empty();
        state.set_git_history(history, is_git_repo);

        self.image_cache
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
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

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
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

        let previous_mode = state.mode;
        let max_scroll = state.max_scroll(20);

        match key.code {
            KeyCode::Esc => {
                self.image_cache
                    .get_mut()
                    .unwrap_or_else(|e| e.into_inner())
                    .clear();
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
            // Go to newer version in git history
            KeyCode::Right if state.has_newer_version() => {
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
            _ => {}
        }

        if state.mode != previous_mode {
            self.image_cache
                .get_mut()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
        }
        KeyHandleResult::Handled
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let state = match &self.state {
            Some(s) => s,
            None => return,
        };

        // Build dynamic title
        let mode_str = match state.mode {
            ViewMode::Normal => "NORMAL",
            ViewMode::Hex => "HEX",
            ViewMode::Image => "IMAGE",
            ViewMode::Vector => "VECTOR",
            ViewMode::Markdown => "MARKDOWN",
            ViewMode::Blame => "BLAME",
            ViewMode::Diff => "DIFF",
        };
        let filter_str = match state.filter {
            ViewFilter::Off => "",
            ViewFilter::Ascii => " [ASCII]",
            ViewFilter::WordStar => " [W/S]",
        };

        let version_str = if let Some(entry) = state.current_commit() {
            let short_hash = &entry.hash[..7.min(entry.hash.len())];
            format!(" [{}]", short_hash)
        } else if state.is_git_repo && !state.git_history.is_empty() {
            " [HEAD]".to_string()
        } else {
            String::new()
        };

        let title = format!(
            " VIEW: {} - {}{}{}",
            state.file_name.to_uppercase(),
            mode_str,
            filter_str,
            version_str
        );

        // Create full screen view
        let view = FullScreenView::new(area, &title, colors);
        view.render_frame(frame);

        // Content area
        let content_area = view.content_area();
        let content_height = content_area.height as usize;
        match state.mode {
            ViewMode::Normal => {
                self.draw_normal_view(frame, content_area, state, content_height, colors)
            }
            ViewMode::Hex => self.draw_hex_view(frame, content_area, state, content_height, colors),
            ViewMode::Image => self.draw_image_view(frame, content_area, state, colors),
            ViewMode::Vector => self.draw_vector_view(frame, content_area, state, colors),
            ViewMode::Markdown => {
                self.draw_markdown_view(frame, content_area, state, content_height, colors)
            }
            ViewMode::Blame => {
                self.draw_blame_view(frame, content_area, state, content_height, colors)
            }
            ViewMode::Diff => {
                self.draw_diff_view(frame, content_area, state, content_height, colors)
            }
        }

        // Build dynamic help hints based on mode
        let mut help_hints: Vec<(&str, &str)> = vec![
            ("H", "hex"),
            ("N", "normal"),
            ("I", "image"),
            ("M", "markdown"),
        ];

        if state.is_git_repo {
            help_hints.push(("B", "blame"));
            help_hints.push(("D", "diff"));
        }

        help_hints.push(("F", "filter"));
        help_hints.push(("↑↓", "scroll"));

        if state.has_older_version() {
            help_hints.push(("←", "older"));
        }
        if state.has_newer_version() {
            help_hints.push(("→", "newer"));
        }

        help_hints.push(("Esc", "exit"));

        view.render_help(frame, help_hints);
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

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Viewer".to_string(),
            description: "File content viewer".to_string(),
            category: PluginCategory::Files,
            key: 'V',
        })
    }

    fn launch(&mut self, cwd: &PathBuf, selected_file: Option<&PathBuf>) -> Result<(), String> {
        if let Some(file_path) = selected_file {
            self.open_file(file_path.clone(), cwd)
        } else {
            Err("No file selected".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Self-registration for automatic plugin discovery
inventory::submit! {
    PluginRegistration::new("viewer", || Box::new(ViewerPlugin::new()))
}

// Drawing helper methods
impl ViewerPlugin {
    fn draw_normal_view(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &ViewerState,
        height: usize,
        colors: &ThemeColors,
    ) {
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
                        Style::default().fg(colors.fg()),
                    ))
                })
                .collect()
        };

        frame.render_widget(Paragraph::new(visible_lines), area);
    }

    fn draw_hex_view(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &ViewerState,
        height: usize,
        colors: &ThemeColors,
    ) {
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
                Style::default().fg(colors.blue()),
            ));

            for (i, &byte) in chunk.iter().enumerate() {
                if i == 8 {
                    spans.push(Span::raw(" "));
                }
                spans.push(Span::styled(
                    format!("{:02X} ", byte),
                    Style::default().fg(colors.fg()),
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
            spans.push(Span::styled(ascii, Style::default().fg(colors.green())));

            lines.push(Line::from(spans));
        }

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn draw_image_view(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &ViewerState,
        colors: &ThemeColors,
    ) {
        let mut cache = self.image_cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(protocol) = cache.get_or_insert_with(&state.content, ViewMode::Image, || {
            create_image_protocol(&state.content)
        }) {
            let image_widget = StatefulImage::new(None);
            frame.render_stateful_widget(image_widget, area, &mut protocol.protocol);
        } else {
            let error_msg = vec![
                Line::from(""),
                Line::from(Span::styled(
                    " Cannot display image",
                    Style::default()
                        .fg(colors.red())
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!(" File: {}", state.file_path.display()),
                    Style::default().fg(colors.green()),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    " Press N for normal view or H for hex view",
                    Style::default().fg(colors.blue()),
                )),
            ];
            frame.render_widget(Paragraph::new(error_msg), area);
        }
    }

    fn draw_vector_view(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &ViewerState,
        colors: &ThemeColors,
    ) {
        // Rasterize only when the source changes, not on every redraw/resize.
        let mut cache = self.image_cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(protocol) = cache.get_or_insert_with(&state.content, ViewMode::Vector, || {
            rasterize_svg(&state.content).and_then(|png| create_image_protocol(&png))
        }) {
            let image_widget = StatefulImage::new(None);
            frame.render_stateful_widget(image_widget, area, &mut protocol.protocol);
            return;
        }

        // Fallback error message
        let error_msg = vec![
            Line::from(""),
            Line::from(Span::styled(
                " Cannot display SVG",
                Style::default()
                    .fg(colors.red())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!(" File: {}", state.file_path.display()),
                Style::default().fg(colors.green()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " Press N for normal view or H for hex view",
                Style::default().fg(colors.blue()),
            )),
        ];
        frame.render_widget(Paragraph::new(error_msg), area);
    }

    fn draw_markdown_view(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &ViewerState,
        height: usize,
        colors: &ThemeColors,
    ) {
        let content_str = String::from_utf8_lossy(&state.content);
        let mut lines: Vec<Line> = Vec::new();

        for line in content_str.lines() {
            if let Some(rest) = line.strip_prefix("# ") {
                lines.push(Line::from(Span::styled(
                    format!(" {}", rest),
                    Style::default()
                        .fg(colors.blue())
                        .add_modifier(Modifier::BOLD),
                )));
            } else if let Some(rest) = line.strip_prefix("## ") {
                lines.push(Line::from(Span::styled(
                    format!(" {}", rest),
                    Style::default()
                        .fg(colors.blue())
                        .add_modifier(Modifier::BOLD),
                )));
            } else if let Some(rest) = line.strip_prefix("### ") {
                lines.push(Line::from(Span::styled(
                    format!(" {}", rest),
                    Style::default().fg(colors.blue()),
                )));
            } else if line.starts_with("#### ")
                || line.starts_with("##### ")
                || line.starts_with("###### ")
            {
                let header_content = line.trim_start_matches('#').trim_start();
                lines.push(Line::from(Span::styled(
                    format!(" {}", header_content),
                    Style::default().fg(colors.blue()),
                )));
            } else if line.starts_with("```") {
                lines.push(Line::from(Span::styled(
                    " ─────────────────────────────────────",
                    Style::default().fg(colors.green()),
                )));
            } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
                lines.push(Line::from(Span::styled(
                    format!("  • {}", rest),
                    Style::default().fg(colors.fg()),
                )));
            } else if line.chars().next().is_some_and(|c| c.is_ascii_digit()) && line.contains(". ")
            {
                lines.push(Line::from(Span::styled(
                    format!("  {}", line),
                    Style::default().fg(colors.fg()),
                )));
            } else if let Some(rest) = line.strip_prefix("> ") {
                lines.push(Line::from(Span::styled(
                    format!(" │ {}", rest),
                    Style::default().fg(colors.green()),
                )));
            } else if line == "---" || line == "***" || line == "___" {
                lines.push(Line::from(Span::styled(
                    " ═════════════════════════════════════",
                    Style::default().fg(colors.fg()),
                )));
            } else if line.contains("**") || line.contains("__") {
                let clean_line = line.replace("**", "").replace("__", "");
                lines.push(Line::from(Span::styled(
                    format!(" {}", clean_line),
                    Style::default()
                        .fg(colors.fg())
                        .add_modifier(Modifier::BOLD),
                )));
            } else if line.contains('*') || line.contains('_') {
                let clean_line = line
                    .chars()
                    .filter(|&c| c != '*' && c != '_')
                    .collect::<String>();
                lines.push(Line::from(Span::styled(
                    format!(" {}", clean_line),
                    Style::default()
                        .fg(colors.fg())
                        .add_modifier(Modifier::ITALIC),
                )));
            } else if line.contains('`') {
                lines.push(Line::from(Span::styled(
                    format!(" {}", line),
                    Style::default().fg(colors.green()),
                )));
            } else if line.trim().is_empty() {
                lines.push(Line::from(""));
            } else {
                lines.push(Line::from(Span::styled(
                    format!(" {}", line),
                    Style::default().fg(colors.fg()),
                )));
            }
        }

        let max_scroll = lines.len().saturating_sub(height);
        let scroll = state.scroll_offset.min(max_scroll);
        let visible_lines: Vec<Line> = lines.into_iter().skip(scroll).take(height).collect();

        frame.render_widget(Paragraph::new(visible_lines), area);
    }

    fn draw_blame_view(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &ViewerState,
        height: usize,
        colors: &ThemeColors,
    ) {
        if state.blame_lines.is_empty() {
            let error_msg = vec![
                Line::from(""),
                Line::from(Span::styled(
                    " No blame data available",
                    Style::default().fg(colors.red()),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    " File may not be tracked by git",
                    Style::default().fg(colors.fg()),
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
                        Style::default().fg(colors.blue()),
                    ),
                    Span::styled(
                        format!("{} ", blame.hash),
                        Style::default().fg(colors.grey()),
                    ),
                    Span::styled(author, Style::default().fg(colors.green())),
                    Span::styled(
                        format!(" {:>8} │ ", blame.time_ago),
                        Style::default().fg(colors.grey()),
                    ),
                    Span::styled(&blame.line_content, Style::default().fg(colors.fg())),
                ];

                Line::from(spans)
            })
            .collect();

        frame.render_widget(Paragraph::new(visible_lines), area);
    }

    fn draw_diff_view(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &ViewerState,
        height: usize,
        colors: &ThemeColors,
    ) {
        if state.diff_lines.is_empty() {
            let error_msg = vec![
                Line::from(""),
                Line::from(Span::styled(
                    " No diff data available",
                    Style::default().fg(colors.fg()),
                )),
            ];
            frame.render_widget(Paragraph::new(error_msg), area);
            return;
        }

        let max_scroll = state.diff_lines.len().saturating_sub(height);
        let scroll = state.scroll_offset.min(max_scroll);

        let visible_lines: Vec<Line> = state
            .diff_lines
            .iter()
            .skip(scroll)
            .take(height)
            .map(|line| {
                let (style, prefix) = if line.starts_with('+') && !line.starts_with("+++") {
                    (Style::default().fg(colors.green()), "+")
                } else if line.starts_with('-') && !line.starts_with("---") {
                    (Style::default().fg(colors.red()), "-")
                } else if line.starts_with("@@") {
                    (Style::default().fg(colors.cyan()), "@")
                } else if line.starts_with("diff ")
                    || line.starts_with("index ")
                    || line.starts_with("+++")
                    || line.starts_with("---")
                {
                    (Style::default().fg(colors.blue()), " ")
                } else {
                    (Style::default().fg(colors.fg()), " ")
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
    fn graphics_selection_is_conservative_and_override_is_explicit() {
        let fallback = ProtocolType::Halfblocks;
        assert_eq!(
            select_protocol(true, true, "xterm-ghostty", "", "", fallback),
            ProtocolType::Kitty
        );
        assert_eq!(
            select_protocol(true, true, "xterm", "ghostty", "", fallback),
            ProtocolType::Kitty
        );
        assert_eq!(
            select_protocol(true, true, "xterm-kitty", "", "", fallback),
            ProtocolType::Kitty
        );
        assert_eq!(
            select_protocol(true, false, "screen", "ghostty", "", fallback),
            ProtocolType::Halfblocks
        );
        assert_eq!(
            select_protocol(false, true, "xterm-ghostty", "ghostty", "kitty", fallback),
            ProtocolType::Halfblocks
        );
        assert_eq!(
            select_protocol(true, false, "tmux", "ghostty", "sixel", fallback),
            ProtocolType::Sixel
        );
        assert_eq!(
            select_protocol(true, true, "unknown", "", "", ProtocolType::Iterm2),
            ProtocolType::Iterm2
        );
        assert_eq!(
            select_protocol(true, true, "unknown", "", "invalid", fallback),
            ProtocolType::Halfblocks
        );
        assert_eq!(
            select_protocol(
                true,
                true,
                "xterm-ghostty",
                "ghostty",
                "halfblocks",
                fallback
            ),
            ProtocolType::Halfblocks
        );
    }

    #[test]
    fn uninitialized_embedder_gets_halfblock_preview_without_query() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(8, 8)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let image = image_protocol_from_picker(png.get_ref(), None).unwrap();
        assert!(matches!(image.protocol, StatefulProtocol::Halfblocks(_)));
    }

    #[test]
    fn standalone_viewer_open_and_draw_without_startup_initialization() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(8, 8)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let mut viewer = ViewerPlugin::new();
        viewer
            .open_file_with_content(
                PathBuf::from("rdos-uninitialized-preview.png"),
                png.into_inner(),
                &std::env::temp_dir(),
            )
            .unwrap();
        let backend = ratatui::backend::TestBackend::new(80, 25);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| viewer.draw_modal(frame, frame.area(), &ThemeColors::default()))
            .unwrap();
        let cache = viewer.image_cache.lock().unwrap();
        let protocol = &cache.entry.as_ref().unwrap().2.as_ref().unwrap().protocol;
        assert!(matches!(protocol, StatefulProtocol::Halfblocks(_)));
    }

    #[test]
    fn graphics_font_size_uses_window_metrics_or_safe_default() {
        assert_eq!(font_size_from_window(800, 600, 100, 30), (8, 20));
        assert_eq!(font_size_from_window(0, 600, 100, 30), (8, 16));
        assert_eq!(font_size_from_window(800, 600, 0, 30), (8, 16));
        assert_eq!(font_size_from_window(10, 10, 100, 30), (8, 16));
    }

    #[test]
    fn kitty_cleanup_is_scoped_and_direct_terminal_only() {
        assert_eq!(delete_kitty_image(42), "\x1b_Ga=d,d=I,i=42,q=2;\x1b\\");
        assert!(direct_terminal(true, "xterm-ghostty", "ghostty", false));
        assert!(!direct_terminal(false, "xterm-ghostty", "ghostty", false));
        assert!(!direct_terminal(true, "xterm-ghostty", "ghostty", true));
        assert!(!direct_terminal(true, "screen-256color", "ghostty", false));
        assert!(!direct_terminal(true, "tmux-256color", "ghostty", false));
        assert!(!direct_terminal(true, "xterm", "tmux", false));
    }

    #[test]
    fn kitty_protocol_transmits_once_and_resizes_with_same_id() {
        use ratatui::buffer::Buffer;
        use ratatui_image::protocol::{kitty::StatefulKitty, ImageSource};
        use ratatui_image::Resize;

        let mut cache = ImageCache::default();
        let protocol = cache
            .get_or_insert_with(b"image", ViewMode::Image, || {
                let source = ImageSource::new(image::DynamicImage::new_rgb8(80, 80), (8, 16));
                Some(StatefulProtocol::Kitty(StatefulKitty::new(
                    source,
                    (8, 16),
                    42,
                    false,
                )))
            })
            .unwrap();
        let area = Rect::new(0, 0, 10, 5);
        let mut first = Buffer::empty(area);
        protocol.resize_encode_render(&Resize::Fit(None), None, area, &mut first);
        assert!(first
            .content()
            .iter()
            .any(|cell| cell.symbol().contains("_Gq=2,i=42,a=T")));

        let protocol = cache
            .get_or_insert_with(b"image", ViewMode::Image, || panic!("rebuilt"))
            .unwrap();
        let mut repeated = Buffer::empty(area);
        protocol.resize_encode_render(&Resize::Fit(None), None, area, &mut repeated);
        assert!(!repeated
            .content()
            .iter()
            .any(|cell| cell.symbol().contains("_G")));

        let small = Rect::new(0, 0, 5, 2);
        let mut resized = Buffer::empty(small);
        protocol.resize_encode_render(&Resize::Fit(None), None, small, &mut resized);
        assert!(resized
            .content()
            .iter()
            .any(|cell| cell.symbol().contains("_Gq=2,i=42,a=T")));
    }

    #[test]
    fn image_cache_reuses_mutated_protocol_state() {
        let mut cache = ImageCache::default();
        *cache
            .get_or_insert_with(b"image", ViewMode::Image, || Some(10))
            .unwrap() = 11;
        assert_eq!(
            cache.get_or_insert_with(b"image", ViewMode::Image, || panic!("rebuilt")),
            Some(&mut 11)
        );
    }

    #[test]
    fn image_cache_invalidates_content_and_mode() {
        let mut cache = ImageCache::default();
        cache.get_or_insert_with(b"first", ViewMode::Image, || Some(1));
        assert_eq!(
            cache.get_or_insert_with(b"second", ViewMode::Image, || Some(2)),
            Some(&mut 2)
        );
        assert_eq!(
            cache.get_or_insert_with(b"second", ViewMode::Vector, || Some(3)),
            Some(&mut 3)
        );
    }

    #[test]
    fn image_cache_remembers_failure_until_cleared() {
        let mut cache: ImageCache<u8> = ImageCache::default();
        assert!(cache
            .get_or_insert_with(b"invalid", ViewMode::Vector, || None)
            .is_none());
        assert!(cache
            .get_or_insert_with(b"invalid", ViewMode::Vector, || panic!("retried"))
            .is_none());
        cache.clear();
        assert_eq!(
            cache.get_or_insert_with(b"invalid", ViewMode::Vector, || Some(1)),
            Some(&mut 1)
        );
    }

    #[test]
    fn image_cache_is_per_viewer() {
        let first = ViewerPlugin::new();
        let second = ViewerPlugin::new();
        first.image_cache.lock().unwrap().entry = Some((ViewMode::Image, vec![1], None));
        assert!(second.image_cache.lock().unwrap().entry.is_none());
    }

    #[test]
    fn leaving_image_mode_and_closing_clear_cache() {
        let mut plugin = ViewerPlugin::new();
        plugin.state = Some(ViewerState::new(
            "test.png".into(),
            "test.png".into(),
            vec![1],
        ));
        plugin.image_cache.lock().unwrap().entry = Some((ViewMode::Image, vec![1], None));
        plugin.handle_modal_key(KeyEvent::from(KeyCode::Char('n')), &PathBuf::new());
        assert!(plugin.image_cache.lock().unwrap().entry.is_none());
        plugin.image_cache.lock().unwrap().entry = Some((ViewMode::Image, vec![1], None));
        plugin.handle_modal_key(KeyEvent::from(KeyCode::Esc), &PathBuf::new());
        assert!(plugin.image_cache.lock().unwrap().entry.is_none());
    }

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
        // Audio files are viewed as normal files (audio playback via Audio app)
        assert_eq!(ViewerState::detect_mode("test.mp3"), ViewMode::Normal);
        assert_eq!(ViewerState::detect_mode("test.wav"), ViewMode::Normal);
    }

    #[test]
    fn test_view_filter_cycle() {
        assert_eq!(ViewFilter::Off.next(), ViewFilter::Ascii);
        assert_eq!(ViewFilter::Ascii.next(), ViewFilter::WordStar);
        assert_eq!(ViewFilter::WordStar.next(), ViewFilter::Off);
    }
}
