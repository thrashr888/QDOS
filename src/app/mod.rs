mod state;

// Re-export plugin ops for backwards compatibility within this module
use crate::plugins::beads::ops as beads_ops;
use crate::plugins::git::ops as git_ops;

// Re-export state types for external use (non-plugin types)
pub use state::{
    AttrValue, AttributeState, BatchRenameState, BeadsMenuItem, BeadsState, BeadsView,
    ClipboardItem, ClipboardState, ColorTheme, ColorThemeState, DirectoryMapState, FileViewerState,
    FindPhase, FindState, HelpState, Modal, NavItem, ProgressOperation, ProgressState,
    QdstartField, QdstartState, SearchSpecState, ShellCommandState, SortMode, ThemeColors,
    ViewFilter, ViewMode,
};

// Re-export Git types from the git plugin (now self-contained)
// These are used by ui/modals.rs and other modules
#[allow(unused_imports)]
pub use crate::plugins::git::{
    BlameLine, ConflictFile, ConflictResolution, ConflictSection, FileHistoryEntry, GitBranch,
    GitConfigEntry, GitFileStatus, GitLogEntry, GitMenuItem, GitRemote, GitStashEntry, GitState,
    GitSubmodule, GitTag, GitView, RemoteAction, SubmoduleStatus,
};

// Re-export types used by beads plugin ops (will move to beads plugin later)
pub use state::{BeadsActivityEntry, BeadsComment, BeadsIssue, BeadsSubIssue};

use crate::config::Config;
use crate::errors;
use crate::event::EventHandler;
use crate::file_ops::{apply_attributes, find_files_recursive, get_directory_contents, FileEntry};
use crate::plugins::{
    fileops::FileOperation, BeadsPlugin, DirMapPlugin, FileOpsPlugin, GitPlugin, HelpPlugin,
    KeyHandleResult, PluginManager, PluginMenuItem, PluginStatusInfo, PrintPlugin, ProcPlugin,
    QdconfigPlugin, SearchSpecPlugin, SpacePlugin, StatusPlugin, ThemePlugin,
};
use crate::ui;
use crate::watcher::DirWatcher;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use crossterm::execute;
use ratatui::prelude::*;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Application state
pub struct App {
    /// Current directory path
    pub current_path: PathBuf,
    /// Files in current directory
    pub files: Vec<FileEntry>,
    /// Currently selected file index
    pub selected_index: usize,
    /// Tagged files (by full path)
    pub tagged_files: Vec<PathBuf>,
    /// Current sort mode
    pub sort_mode: SortMode,
    /// Selected navigation menu item
    pub nav_index: usize,
    /// Scroll offset for navigation menu (horizontal scrolling)
    pub nav_scroll_offset: usize,
    /// Current active modal
    pub modal: Modal,
    /// Scroll offset for file list
    pub scroll_offset: usize,
    /// Should the app quit
    pub should_quit: bool,
    /// Search/filter specification
    pub search_spec: String,
    /// Navigation history (back stack)
    pub history: Vec<PathBuf>,
    /// Forward navigation stack
    pub forward_history: Vec<PathBuf>,
    /// Last find pattern (for Ctrl+R recall)
    pub last_find_pattern: String,
    /// Current color theme
    pub color_theme: ColorTheme,
    /// Application configuration
    pub config: Config,
    /// Show hidden files
    pub show_hidden: bool,
    /// Beads status bar info - None if not in beads project
    pub beads_status_info: Option<beads_ops::BeadsStatusInfo>,
    /// Git status bar info - None if not in git repo
    pub git_status_info: Option<git_ops::GitStatusInfo>,
    /// Plugin manager for extensibility
    pub plugin_manager: PluginManager,
    /// Last time the file list was refreshed (for auto-refresh)
    pub last_refresh: std::time::Instant,
    /// Directory watcher for event-based file updates
    pub dir_watcher: Option<DirWatcher>,
}

impl App {
    pub fn new(start_path: &str) -> Result<Self> {
        // Load configuration
        let config = Config::load().unwrap_or_default();

        // Apply config settings
        let sort_mode = config.to_sort_mode();
        let color_theme: ColorTheme = config.display.theme.clone().into();
        let search_spec = config.general.search_spec.clone();
        let show_hidden = config.general.show_hidden;

        // Initialize plugin manager with config and register built-in plugins
        let mut plugin_manager = PluginManager::with_config(config.plugins.clone());
        plugin_manager.register(Box::new(HelpPlugin::new()));
        plugin_manager.register(Box::new(DirMapPlugin::new()));
        plugin_manager.register(Box::new(GitPlugin::new()));
        plugin_manager.register(Box::new(BeadsPlugin::new()));
        plugin_manager.register(Box::new(StatusPlugin::new()));
        plugin_manager.register(Box::new(SpacePlugin::new()));
        plugin_manager.register(Box::new(ThemePlugin::new()));
        plugin_manager.register(Box::new(PrintPlugin::new()));
        plugin_manager.register(Box::new(ProcPlugin::new()));
        plugin_manager.register(Box::new(SearchSpecPlugin::new()));
        plugin_manager.register(Box::new(QdconfigPlugin::new()));
        plugin_manager.register(Box::new(FileOpsPlugin::new()));

        let current_path = PathBuf::from(start_path).canonicalize()?;
        let files = get_directory_contents(&current_path, sort_mode)?;
        let watcher = DirWatcher::new(&current_path).ok();

        let mut app = Self {
            current_path,
            files,
            selected_index: 0,
            tagged_files: Vec::new(),
            sort_mode,
            nav_index: 0,
            nav_scroll_offset: 0,
            modal: Modal::None,
            scroll_offset: 0,
            should_quit: false,
            search_spec,
            history: Vec::new(),
            forward_history: Vec::new(),
            last_find_pattern: String::new(),
            color_theme,
            config,
            show_hidden,
            beads_status_info: None,
            git_status_info: None,
            plugin_manager,
            last_refresh: std::time::Instant::now(),
            dir_watcher: watcher,
        };

        // Load status bar info
        app.refresh_status_bar();

        Ok(app)
    }

    /// Save current settings to config file
    pub fn save_config(&mut self) -> Result<()> {
        // Update config with current settings
        self.config.from_sort_mode(self.sort_mode);
        self.config.display.theme = self.color_theme.into();
        self.config.general.search_spec = self.search_spec.clone();
        self.config.general.show_hidden = self.show_hidden;

        self.config.save()
    }

    /// Main application loop
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let event_handler = EventHandler::new(100);

        loop {
            // Draw UI with synchronized update to reduce flicker
            // BeginSynchronizedUpdate tells the terminal to buffer rendering
            // until EndSynchronizedUpdate, preventing tearing
            let _ = execute!(io::stdout(), BeginSynchronizedUpdate);
            terminal.draw(|frame| ui::draw(frame, self))?;
            let _ = execute!(io::stdout(), EndSynchronizedUpdate);

            // Handle events
            if let Some(event) = event_handler.next().await? {
                match event {
                    crate::event::Event::Key(key) => self.handle_key(key)?,
                    crate::event::Event::Tick => {
                        // Auto-process progress on tick
                        if matches!(self.modal, Modal::Progress(_)) {
                            self.process_next_progress_file();
                        }

                        // Tick active plugin modal for auto-refresh (e.g., Proc plugin)
                        if self.plugin_manager.has_active_modal() {
                            self.plugin_manager.tick_active_modal();
                        }

                        // Check for directory changes from watcher (event-based refresh)
                        let watcher_changed = self
                            .dir_watcher
                            .as_ref()
                            .map(|w| w.has_changes())
                            .unwrap_or(false);

                        if watcher_changed && matches!(self.modal, Modal::None) {
                            let _ = self.refresh_files();
                            self.last_refresh = std::time::Instant::now();
                        }

                        // Auto-refresh for plugin status updates (fallback when watcher unavailable)
                        let refresh_interval = self.config.general.auto_refresh_interval;
                        if refresh_interval > 0
                            && matches!(self.modal, Modal::None)
                            && self.last_refresh.elapsed().as_secs() >= refresh_interval
                        {
                            // Only refresh status bar, not files (watcher handles files)
                            if self.dir_watcher.is_some() {
                                self.refresh_status_bar();
                            } else {
                                // No watcher, do full refresh
                                let _ = self.refresh_files();
                            }
                            self.last_refresh = std::time::Instant::now();
                        }
                    }
                    crate::event::Event::Mouse(mouse) => {
                        // Only handle mouse events if enabled in config
                        if self.config.general.mouse_support {
                            self.handle_mouse(mouse)?;
                        }
                    }
                    crate::event::Event::Resize(_, _) => {}
                    crate::event::Event::DirChanged => {
                        // Handled by watcher.has_changes() in Tick
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Handle mouse input
    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        // Layout constants (must match ui/mod.rs layout)
        // Nav bar: 2 lines, Separator: 1 line, Path bar: 1 line
        const CONTENT_START_Y: u16 = 4;
        // Stats panel on right takes ~20 chars, status bar at bottom takes 1 line

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Only handle clicks in file list area when no modal is open
                if matches!(self.modal, Modal::None) && mouse.row >= CONTENT_START_Y {
                    // Calculate which file was clicked
                    // row - CONTENT_START_Y gives the line in the content area
                    let clicked_line = (mouse.row - CONTENT_START_Y) as usize;

                    // Account for scroll offset
                    let clicked_index = self.scroll_offset + clicked_line;

                    // Only select if valid index
                    if clicked_index < self.files.len() {
                        self.selected_index = clicked_index;
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                // Scroll up in file list
                if matches!(self.modal, Modal::None) {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                        // Adjust scroll if needed
                        if self.selected_index < self.scroll_offset {
                            self.scroll_offset = self.selected_index;
                        }
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                // Scroll down in file list
                if matches!(self.modal, Modal::None) {
                    if self.selected_index + 1 < self.files.len() {
                        self.selected_index += 1;
                        // Note: scroll adjustment happens in UI render
                    }
                }
            }
            _ => {
                // Ignore other mouse events (moves, releases, etc.)
            }
        }

        Ok(())
    }

    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Global quit shortcuts - F10 and Ctrl+C can open quit modal from anywhere
        let is_quit_key = key.code == KeyCode::F(10)
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL));

        if is_quit_key {
            if matches!(self.modal, Modal::Quit) {
                // Already in quit modal - quit immediately
                self.should_quit = true;
            } else {
                // Open quit modal from any state
                self.modal = Modal::Quit;
            }
            return Ok(());
        }

        // Color theme shortcut (Ctrl+T) - handled by ThemePlugin via plugin system
        // Ctrl+T is intercepted by handle_plugin_key() before reaching here

        // QDSTART configuration shortcut (Ctrl+S) - handled by QdconfigPlugin via plugin system
        // Ctrl+S is intercepted by handle_plugin_key() before reaching here

        // Refresh shortcut (Ctrl+R)
        if key.code == KeyCode::Char('r') && key.modifiers.contains(KeyModifiers::CONTROL) {
            let _ = self.refresh_files();
            return Ok(());
        }

        // Handle modal-specific input
        if !matches!(self.modal, Modal::None) {
            return self.handle_modal_key(key);
        }

        // Let plugins handle keys first
        if self.handle_plugin_key(key) {
            return Ok(());
        }

        match key.code {
            // Quit (also handles 'q' which is not a global shortcut)
            KeyCode::Char('q') => {
                self.modal = Modal::Quit;
            }
            // Git menu (G key)
            KeyCode::Char('g') | KeyCode::Char('G') => {
                let is_repo = self.is_git_repo();
                let mut state = GitState::new(is_repo);
                if is_repo {
                    git_ops::load_git_status(&mut state, &self.current_path);
                }
                self.modal = Modal::Git(state);
            }
            // Beads menu (B key)
            KeyCode::Char('b') | KeyCode::Char('B') => {
                let is_beads = self.is_beads_project();
                let mut state = BeadsState::new(is_beads);
                if is_beads {
                    beads_ops::load_recent_issues(&mut state, &self.current_path);
                    beads_ops::load_top_epics(&mut state, &self.current_path);
                }
                self.modal = Modal::Beads(state);
            }
            // File issues (I key) - show beads issues related to selected file
            KeyCode::Char('i') | KeyCode::Char('I') => {
                if self.is_beads_project() {
                    let mut state = BeadsState::new(true);
                    if let Some(entry) = self.files.get(self.selected_index) {
                        let file_path = entry.path.to_string_lossy().to_string();
                        beads_ops::find_issues_for_file(&mut state, &file_path, &self.current_path);
                        state.view = BeadsView::FileIssues;
                    }
                    self.modal = Modal::Beads(state);
                } else {
                    self.modal = Modal::Error("Beads not initialized in this project".to_string());
                }
            }
            // Yank (copy to clipboard) - Y key
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.open_clipboard_menu();
            }
            // Go back in directory history - like VS Code
            KeyCode::Char('-') => {
                let _ = self.go_back();
            }
            // Go forward in directory history - like VS Code
            KeyCode::Char('=') | KeyCode::Char('+') => {
                let _ = self.go_forward();
            }
            // Help - handled by HelpPlugin via plugin system
            // F1 key is intercepted by handle_plugin_key() before reaching here
            // Status - handled by StatusPlugin via plugin system
            // F2 key is intercepted by handle_plugin_key() before reaching here
            // Change drive (not applicable on Unix, show error)
            KeyCode::F(3) => {
                self.modal =
                    Modal::Error("Drive selection not available on this platform".to_string());
            }
            // Previous directory
            KeyCode::F(4) => {
                self.go_to_parent()?;
            }
            // Change directory
            KeyCode::F(5) => {
                let path = self.current_path.to_string_lossy().to_string();
                self.modal = Modal::PathInput(path);
            }
            // Shell Command
            KeyCode::F(6) => {
                self.modal = Modal::ShellCommand(ShellCommandState::default());
            }
            // Search spec - handled by SearchSpecPlugin via plugin system
            // F7 key is intercepted by handle_plugin_key() before reaching here
            // Sort
            KeyCode::F(8) => {
                self.cycle_sort_mode()?;
            }
            // Edit - open in default editor
            KeyCode::F(9) => {
                self.edit_selected_file()?;
            }
            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
            }
            KeyCode::PageUp => {
                self.move_selection(-20);
            }
            KeyCode::PageDown => {
                self.move_selection(20);
            }
            KeyCode::Home => {
                self.selected_index = 0;
                self.scroll_offset = 0;
            }
            KeyCode::End => {
                if !self.files.is_empty() {
                    self.selected_index = self.files.len() - 1;
                    // Scroll to bottom (use conservative visible_height of 15)
                    self.scroll_offset = self.files.len().saturating_sub(15);
                }
            }
            // Menu navigation
            KeyCode::Left | KeyCode::Char('h') => {
                if self.nav_index > 0 {
                    self.nav_index -= 1;
                } else {
                    self.nav_index = NavItem::ALL.len() - 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.nav_index = (self.nav_index + 1) % NavItem::ALL.len();
            }
            // Tag file
            KeyCode::Char(' ') => {
                self.toggle_tag();
                self.move_selection(1);
            }
            // Enter directory or execute menu action
            KeyCode::Enter => {
                self.execute_action()?;
            }
            // Escape
            KeyCode::Esc => {
                // Do nothing in main view
            }
            // Ctrl+C opens quit confirmation (same as F10)
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.modal = Modal::Quit;
            }
            // Main menu letter shortcuts (first letter of each item)
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // Directory - same as Enter on a directory
                self.execute_action()?;
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                // Tag - toggle tag on selected file
                self.toggle_tag();
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                // View - view selected file
                self.nav_index = NavItem::ALL
                    .iter()
                    .position(|n| *n == NavItem::View)
                    .unwrap_or(0);
                self.execute_action()?;
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                // Copy - select nav item and execute
                self.nav_index = NavItem::ALL
                    .iter()
                    .position(|n| *n == NavItem::Copy)
                    .unwrap_or(0);
                self.execute_action()?;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                // Move
                self.nav_index = NavItem::ALL
                    .iter()
                    .position(|n| *n == NavItem::Move)
                    .unwrap_or(0);
                self.execute_action()?;
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                // Find
                self.nav_index = NavItem::ALL
                    .iter()
                    .position(|n| *n == NavItem::Find)
                    .unwrap_or(0);
                self.execute_action()?;
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                // Erase
                self.nav_index = NavItem::ALL
                    .iter()
                    .position(|n| *n == NavItem::Erase)
                    .unwrap_or(0);
                self.execute_action()?;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Rename
                self.nav_index = NavItem::ALL
                    .iter()
                    .position(|n| *n == NavItem::Rename)
                    .unwrap_or(0);
                self.execute_action()?;
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                // Attribute
                self.nav_index = NavItem::ALL
                    .iter()
                    .position(|n| *n == NavItem::Attribute)
                    .unwrap_or(0);
                self.execute_action()?;
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                // Print
                self.nav_index = NavItem::ALL
                    .iter()
                    .position(|n| *n == NavItem::Print)
                    .unwrap_or(0);
                self.execute_action()?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle keyboard input in modal dialogs
    fn handle_modal_key(&mut self, key: KeyEvent) -> Result<()> {
        // Handle 'y' (yank/copy) in specific modals that should support clipboard
        if matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y'))
            && matches!(
                self.modal,
                Modal::Beads(_) | Modal::Git(_) | Modal::FileViewer(_)
            )
        {
            self.open_clipboard_menu();
            return Ok(());
        }

        match &mut self.modal {
            Modal::Quit => {
                match key.code {
                    // F10 again or Ctrl+C again quits immediately
                    KeyCode::F(10) => {
                        self.should_quit = true;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.should_quit = true;
                    }
                    // RETURN for options (for now, just quit)
                    KeyCode::Enter => {
                        self.should_quit = true;
                    }
                    // ESC returns to Q-DOS II
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    // Y/N still work for convenience
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.should_quit = true;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                }
            }
            Modal::PathInput(ref mut path) => match key.code {
                KeyCode::Enter => {
                    let new_path = PathBuf::from(path.clone());
                    self.modal = Modal::None;
                    if let Err(e) = self.navigate_to(&new_path) {
                        self.modal = Modal::Error(format!("Cannot navigate: {}", e));
                    }
                }
                KeyCode::Esc => {
                    self.modal = Modal::None;
                }
                KeyCode::Backspace => {
                    path.pop();
                }
                KeyCode::Tab => {
                    if let Some(completed) = Self::tab_complete(path) {
                        *path = completed;
                    }
                }
                KeyCode::Char(c) => {
                    path.push(c);
                }
                _ => {}
            },
            Modal::CopyTo(ref mut dest) => match key.code {
                KeyCode::Enter => {
                    let dest_path = PathBuf::from(dest.clone());
                    let files = self.tagged_files.clone();
                    if files.is_empty() {
                        self.modal = Modal::Error("No files to copy".to_string());
                    } else {
                        // Start progress modal
                        let state =
                            ProgressState::new(ProgressOperation::Copy, files, Some(dest_path));
                        self.modal = Modal::Progress(state);
                    }
                }
                KeyCode::Esc => {
                    // Clear any temporarily tagged files
                    self.tagged_files.clear();
                    self.modal = Modal::None;
                }
                KeyCode::Backspace => {
                    dest.pop();
                }
                KeyCode::Tab => {
                    if let Some(completed) = Self::tab_complete(dest) {
                        *dest = completed;
                    }
                }
                KeyCode::Char(c) => {
                    dest.push(c);
                }
                _ => {}
            },
            Modal::MoveTo(ref mut dest) => {
                match key.code {
                    KeyCode::Enter => {
                        let dest_path = PathBuf::from(dest.clone());
                        let files = self.tagged_files.clone();
                        if files.is_empty() {
                            self.modal = Modal::Error("No files to move".to_string());
                        } else {
                            // Start progress modal
                            let state =
                                ProgressState::new(ProgressOperation::Move, files, Some(dest_path));
                            self.modal = Modal::Progress(state);
                        }
                    }
                    KeyCode::Esc => {
                        // Clear any temporarily tagged files
                        self.tagged_files.clear();
                        self.modal = Modal::None;
                    }
                    KeyCode::Backspace => {
                        dest.pop();
                    }
                    KeyCode::Tab => {
                        if let Some(completed) = Self::tab_complete(dest) {
                            *dest = completed;
                        }
                    }
                    KeyCode::Char(c) => {
                        dest.push(c);
                    }
                    _ => {}
                }
            }
            Modal::EraseConfirm => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let files = self.tagged_files.clone();
                    if files.is_empty() {
                        self.modal = Modal::Error("No files to erase".to_string());
                    } else {
                        // Start progress modal
                        let state = ProgressState::new(ProgressOperation::Erase, files, None);
                        self.modal = Modal::Progress(state);
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.modal = Modal::None;
                }
                _ => {}
            },
            Modal::RenameInput(ref mut new_name) => match key.code {
                KeyCode::Enter => {
                    let name = new_name.clone();
                    self.modal = Modal::None;
                    match self.rename_selected_file(&name) {
                        Ok(()) => {
                            self.modal = Modal::Success("File renamed".to_string());
                            let _ = self.refresh_files();
                        }
                        Err(e) => {
                            self.modal = Modal::Error(format!("Rename failed: {}", e));
                        }
                    }
                }
                KeyCode::Esc => {
                    self.modal = Modal::None;
                }
                KeyCode::Backspace => {
                    new_name.pop();
                }
                KeyCode::Tab => {
                    if let Some(completed) = Self::tab_complete(new_name) {
                        *new_name = completed;
                    }
                }
                KeyCode::Char(c) => {
                    new_name.push(c);
                }
                _ => {}
            },
            Modal::FileViewer(ref mut state) => {
                // Calculate max scroll based on mode and content
                let max_scroll = state.max_scroll(20); // Assume ~20 lines visible, will be recalculated in UI

                match key.code {
                    KeyCode::Esc => {
                        self.modal = Modal::None;
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
                        // Clamp scroll for new mode
                        state.scroll_offset = state.scroll_offset.min(state.max_scroll(20));
                    }
                    KeyCode::Char('n')
                    | KeyCode::Char('N')
                    | KeyCode::Char('a')
                    | KeyCode::Char('A') => {
                        state.mode = ViewMode::Normal;
                        // Clamp scroll for new mode
                        state.scroll_offset = state.scroll_offset.min(state.max_scroll(20));
                    }
                    KeyCode::Char('i') | KeyCode::Char('I') => {
                        state.mode = ViewMode::Image;
                        state.scroll_offset = 0;
                    }
                    KeyCode::Char('m') | KeyCode::Char('M') => {
                        state.mode = ViewMode::Markdown;
                        // Clamp scroll for new mode
                        state.scroll_offset = state.scroll_offset.min(state.max_scroll(20));
                    }
                    KeyCode::Char('f') | KeyCode::Char('F') => {
                        state.filter = state.filter.next();
                    }
                    KeyCode::Char('b') | KeyCode::Char('B') => {
                        // Switch to blame view (only if in git repo)
                        if state.is_git_repo {
                            // Load blame data if not already loaded
                            if state.blame_lines.is_empty() {
                                state.blame_lines =
                                    git_ops::load_file_blame(&state.file_path, &self.current_path);
                            }
                            state.mode = ViewMode::Blame;
                            state.scroll_offset = 0;
                        }
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        // Switch to diff view (only if in git repo)
                        if state.is_git_repo {
                            // Load diff data if not already loaded
                            if state.diff_lines.is_empty() {
                                state.diff_lines = git_ops::load_file_diff_against_head(
                                    &state.file_path,
                                    &self.current_path,
                                );
                            }
                            state.mode = ViewMode::Diff;
                            state.scroll_offset = 0;
                        }
                    }
                    KeyCode::F(4) => {
                        // Toggle hex/ascii side in hex mode
                        if state.mode == ViewMode::Hex {
                            state.hex_side = !state.hex_side;
                        }
                    }
                    KeyCode::Left => {
                        // Go to older version in git history
                        if state.has_older_version() {
                            let new_idx = match state.history_index {
                                None => {
                                    // Currently at working copy, go to most recent commit
                                    state.git_history.len() - 1
                                }
                                Some(idx) => idx.saturating_sub(1),
                            };
                            if let Some(entry) = state.git_history.get(new_idx) {
                                let commit_hash = entry.hash.clone();
                                let file_path = state.file_path.clone();
                                if let Ok(content) = git_ops::load_file_at_commit(
                                    &file_path,
                                    &commit_hash,
                                    &self.current_path,
                                ) {
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
                                    // Go to next commit
                                    let new_idx = idx + 1;
                                    if let Some(entry) = state.git_history.get(new_idx) {
                                        let commit_hash = entry.hash.clone();
                                        let file_path = state.file_path.clone();
                                        if let Ok(content) = git_ops::load_file_at_commit(
                                            &file_path,
                                            &commit_hash,
                                            &self.current_path,
                                        ) {
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
            }
            Modal::ShellCommand(ref mut state) => {
                match key.code {
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    KeyCode::Enter => {
                        if !state.input.is_empty() {
                            let cmd = state.input.clone();
                            let cwd = self.current_path.clone();
                            // Add to history
                            state.history.push(cmd.clone());
                            state.history_index = None;
                            // Execute command (use standalone function to avoid borrow issues)
                            let output = execute_shell_command_impl(&cmd, &cwd);
                            state.output = output.0;
                            state.exit_code = Some(output.1);
                            state.input.clear();
                            state.scroll_offset = 0;
                        }
                    }
                    KeyCode::Backspace => {
                        state.input.pop();
                    }
                    KeyCode::Up => {
                        // Navigate history up
                        if !state.history.is_empty() {
                            let new_idx = match state.history_index {
                                Some(idx) if idx > 0 => idx - 1,
                                Some(idx) => idx,
                                None => state.history.len() - 1,
                            };
                            state.history_index = Some(new_idx);
                            state.input = state.history[new_idx].clone();
                        }
                    }
                    KeyCode::Down => {
                        // Navigate history down
                        if let Some(idx) = state.history_index {
                            if idx + 1 < state.history.len() {
                                let new_idx = idx + 1;
                                state.history_index = Some(new_idx);
                                state.input = state.history[new_idx].clone();
                            } else {
                                state.history_index = None;
                                state.input.clear();
                            }
                        }
                    }
                    KeyCode::PageUp => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        let max_scroll = state.output.len().saturating_sub(10);
                        state.scroll_offset = (state.scroll_offset + 10).min(max_scroll);
                    }
                    KeyCode::Tab => {
                        // Tab completion for commands/paths
                        if let Some(completed) = Self::tab_complete(&state.input) {
                            state.input = completed;
                        }
                    }
                    KeyCode::Char(c) => {
                        state.input.push(c);
                    }
                    _ => {}
                }
            }
            Modal::DirectoryMap(ref mut state) => {
                // Handle delete confirmation mode
                if let Some(ref path_to_delete) = state.confirm_delete.clone() {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            // Confirmed - delete the directory
                            match fs::remove_dir(path_to_delete) {
                                Ok(()) => {
                                    // Refresh tree
                                    let parent_idx = if state.selected_index > 0 {
                                        state.selected_index - 1
                                    } else {
                                        0
                                    };
                                    state.confirm_delete = None;
                                    state.rebuild_flat_list();
                                    state.selected_index =
                                        parent_idx.min(state.flat_list.len().saturating_sub(1));
                                }
                                Err(e) => {
                                    state.confirm_delete = None;
                                    self.modal =
                                        Modal::Error(format!("Cannot remove directory: {}", e));
                                    return Ok(());
                                }
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            // Cancelled
                            state.confirm_delete = None;
                        }
                        _ => {}
                    }
                }
                // Handle input mode (for make directory)
                else if state.input_mode.is_some() {
                    match key.code {
                        KeyCode::Enter => {
                            let dir_name = state.input_buffer.clone();
                            if !dir_name.is_empty() {
                                if let Some(parent_path) = state.selected_path() {
                                    let new_dir = parent_path.join(&dir_name);
                                    match fs::create_dir(&new_dir) {
                                        Ok(()) => {
                                            // Refresh the tree
                                            state.input_mode = None;
                                            state.input_buffer.clear();
                                            // Reload children of selected node
                                            state.toggle_expand(state.selected_index);
                                            state.toggle_expand(state.selected_index);
                                        }
                                        Err(e) => {
                                            state.input_mode = None;
                                            state.input_buffer.clear();
                                            self.modal = Modal::Error(format!(
                                                "Failed to create directory: {}",
                                                e
                                            ));
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                            state.input_mode = None;
                            state.input_buffer.clear();
                        }
                        KeyCode::Esc => {
                            state.input_mode = None;
                            state.input_buffer.clear();
                        }
                        KeyCode::Backspace => {
                            state.input_buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            state.input_buffer.push(c);
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Esc => {
                            self.modal = Modal::None;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if state.selected_index > 0 {
                                state.selected_index -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if state.selected_index + 1 < state.flat_list.len() {
                                state.selected_index += 1;
                            }
                        }
                        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                            // Navigate to directory or expand
                            if let Some((_, _, expanded, has_children)) =
                                state.flat_list.get(state.selected_index)
                            {
                                if *has_children && !*expanded {
                                    state.toggle_expand(state.selected_index);
                                } else if let Some(path) = state.selected_path() {
                                    // Navigate to the selected directory
                                    let path = path.clone();
                                    self.modal = Modal::None;
                                    let _ = self.navigate_to(&path);
                                }
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                            // Collapse if expanded, otherwise go to parent
                            if let Some((_, _, expanded, _)) =
                                state.flat_list.get(state.selected_index)
                            {
                                if *expanded {
                                    state.toggle_expand(state.selected_index);
                                } else if state.selected_index > 0 {
                                    // Find parent (look for item with depth - 1)
                                    let current_depth = state
                                        .flat_list
                                        .get(state.selected_index)
                                        .map(|(_, d, _, _)| *d)
                                        .unwrap_or(0);
                                    if current_depth > 0 {
                                        for i in (0..state.selected_index).rev() {
                                            if let Some((_, d, _, _)) = state.flat_list.get(i) {
                                                if *d < current_depth {
                                                    state.selected_index = i;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('m') | KeyCode::Char('M') => {
                            // Make directory mode
                            state.input_mode = Some("New directory name".to_string());
                            state.input_buffer.clear();
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            // Request delete confirmation
                            if let Some(path) = state.selected_path() {
                                // Don't allow deleting root
                                if path != state.root.path {
                                    state.confirm_delete = Some(path);
                                }
                            }
                        }
                        KeyCode::Home => {
                            state.selected_index = 0;
                        }
                        KeyCode::End => {
                            state.selected_index = state.flat_list.len().saturating_sub(1);
                        }
                        _ => {}
                    }
                }
            }
            Modal::Find(ref mut state) => {
                match state.phase {
                    FindPhase::InputPattern => {
                        match key.code {
                            KeyCode::Enter => {
                                if state.pattern.is_empty() {
                                    // Use *.* if no pattern entered
                                    state.pattern = "*.*".to_string();
                                }
                                // Move to ask pause phase
                                state.phase = FindPhase::AskPause;
                            }
                            KeyCode::Esc => {
                                self.modal = Modal::None;
                            }
                            KeyCode::Backspace => {
                                state.pattern.pop();
                            }
                            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Recall last pattern
                                state.pattern = state.last_pattern.clone();
                            }
                            KeyCode::Char(c) => {
                                state.pattern.push(c);
                            }
                            _ => {}
                        }
                    }
                    FindPhase::AskPause => {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                state.pause_on_match = true;
                                // Start searching
                                state.phase = FindPhase::Searching;
                                let root = self.current_path.clone();
                                state.matches = find_files_recursive(&root, &state.pattern);
                                self.last_find_pattern = state.pattern.clone();
                                state.search_complete = true;

                                if state.matches.is_empty() {
                                    state.phase = FindPhase::NoResults;
                                } else {
                                    state.phase = FindPhase::ShowResult;
                                    state.current_match = 0;
                                }
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                state.pause_on_match = false;
                                // Start searching
                                state.phase = FindPhase::Searching;
                                let root = self.current_path.clone();
                                state.matches = find_files_recursive(&root, &state.pattern);
                                self.last_find_pattern = state.pattern.clone();
                                state.search_complete = true;

                                if state.matches.is_empty() {
                                    state.phase = FindPhase::NoResults;
                                } else {
                                    state.phase = FindPhase::ShowAllResults;
                                    state.scroll_offset = 0;
                                }
                            }
                            KeyCode::Esc => {
                                self.modal = Modal::None;
                            }
                            _ => {}
                        }
                    }
                    FindPhase::Searching => {
                        // Searching is synchronous for now, so this won't be hit
                        // In future could make async with progress display
                    }
                    FindPhase::ShowResult => {
                        match key.code {
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                // Continue to next match
                                if state.current_match + 1 < state.matches.len() {
                                    state.current_match += 1;
                                } else {
                                    // No more matches
                                    state.phase = FindPhase::NoResults;
                                }
                            }
                            KeyCode::Char('j') | KeyCode::Char('J') => {
                                // Jump to directory containing the file
                                if let Some((path, _)) = state.matches.get(state.current_match) {
                                    if let Some(parent) = path.parent() {
                                        let parent = parent.to_path_buf();
                                        let file_name = path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string());
                                        self.modal = Modal::None;
                                        if let Ok(()) = self.navigate_to(&parent) {
                                            // Try to select the file
                                            if let Some(name) = file_name {
                                                if let Some(idx) = self.files.iter().position(|f| {
                                                    let full_name = if f.extension.is_empty() {
                                                        f.name.clone()
                                                    } else {
                                                        format!(
                                                            "{}.{}",
                                                            f.name,
                                                            f.extension.to_lowercase()
                                                        )
                                                    };
                                                    full_name.eq_ignore_ascii_case(&name)
                                                }) {
                                                    self.selected_index = idx;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('v') | KeyCode::Char('V') => {
                                // View the file
                                if let Some((path, _)) = state.matches.get(state.current_match) {
                                    if let Ok(content) = std::fs::read(path) {
                                        let file_name = path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| "file".to_string());
                                        let file_path = path.clone();
                                        let mut viewer_state = FileViewerState::new(
                                            file_name,
                                            file_path.clone(),
                                            content,
                                        );
                                        // Load git history if in a git repo
                                        let is_repo = git_ops::is_git_repo(&self.current_path);
                                        if is_repo {
                                            let history = git_ops::load_file_history(
                                                &file_path,
                                                &self.current_path,
                                            );
                                            viewer_state.set_git_history(history, true);
                                        }
                                        self.modal = Modal::FileViewer(viewer_state);
                                    }
                                }
                            }
                            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::ALT) => {
                                // Alt-E: Erase the found file (with confirmation in future)
                                // For now, skip this - requires confirmation workflow
                            }
                            KeyCode::Esc => {
                                self.modal = Modal::None;
                            }
                            _ => {}
                        }
                    }
                    FindPhase::ShowAllResults => {
                        let visible_height = 15; // Approximate visible lines

                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                // Move selection up
                                if state.current_match > 0 {
                                    state.current_match -= 1;
                                    // Adjust scroll to keep selection visible
                                    if state.current_match < state.scroll_offset {
                                        state.scroll_offset = state.current_match;
                                    }
                                }
                            }
                            KeyCode::Down => {
                                // Move selection down
                                if state.current_match + 1 < state.matches.len() {
                                    state.current_match += 1;
                                    // Adjust scroll to keep selection visible
                                    if state.current_match >= state.scroll_offset + visible_height {
                                        state.scroll_offset =
                                            state.current_match - visible_height + 1;
                                    }
                                }
                            }
                            KeyCode::PageUp => {
                                state.current_match =
                                    state.current_match.saturating_sub(visible_height);
                                state.scroll_offset =
                                    state.scroll_offset.saturating_sub(visible_height);
                            }
                            KeyCode::PageDown => {
                                let max = state.matches.len().saturating_sub(1);
                                state.current_match =
                                    (state.current_match + visible_height).min(max);
                                let max_scroll = state.matches.len().saturating_sub(visible_height);
                                state.scroll_offset =
                                    (state.scroll_offset + visible_height).min(max_scroll);
                            }
                            KeyCode::Home => {
                                state.current_match = 0;
                                state.scroll_offset = 0;
                            }
                            KeyCode::End => {
                                state.current_match = state.matches.len().saturating_sub(1);
                                let max_scroll = state.matches.len().saturating_sub(visible_height);
                                state.scroll_offset = max_scroll;
                            }
                            KeyCode::Char('j') | KeyCode::Char('J') => {
                                // Jump to directory containing the selected file
                                if let Some((path, _)) = state.matches.get(state.current_match) {
                                    if let Some(parent) = path.parent() {
                                        let parent = parent.to_path_buf();
                                        let file_name = path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string());
                                        self.modal = Modal::None;
                                        if let Ok(()) = self.navigate_to(&parent) {
                                            // Try to select the file
                                            if let Some(name) = file_name {
                                                if let Some(idx) = self.files.iter().position(|f| {
                                                    let full_name = if f.extension.is_empty() {
                                                        f.name.clone()
                                                    } else {
                                                        format!(
                                                            "{}.{}",
                                                            f.name,
                                                            f.extension.to_lowercase()
                                                        )
                                                    };
                                                    full_name.eq_ignore_ascii_case(&name)
                                                }) {
                                                    self.selected_index = idx;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('v') | KeyCode::Char('V') => {
                                // View the selected file
                                if let Some((path, _)) = state.matches.get(state.current_match) {
                                    if let Ok(content) = std::fs::read(path) {
                                        let file_name = path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| "file".to_string());
                                        let file_path = path.clone();
                                        let mut viewer_state = FileViewerState::new(
                                            file_name,
                                            file_path.clone(),
                                            content,
                                        );
                                        // Load git history if in a git repo
                                        let is_repo = git_ops::is_git_repo(&self.current_path);
                                        if is_repo {
                                            let history = git_ops::load_file_history(
                                                &file_path,
                                                &self.current_path,
                                            );
                                            viewer_state.set_git_history(history, true);
                                        }
                                        self.modal = Modal::FileViewer(viewer_state);
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                // Jump to the selected file (same as J)
                                if let Some((path, _)) = state.matches.get(state.current_match) {
                                    if let Some(parent) = path.parent() {
                                        let parent = parent.to_path_buf();
                                        let file_name = path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string());
                                        self.modal = Modal::None;
                                        if let Ok(()) = self.navigate_to(&parent) {
                                            if let Some(name) = file_name {
                                                if let Some(idx) = self.files.iter().position(|f| {
                                                    let full_name = if f.extension.is_empty() {
                                                        f.name.clone()
                                                    } else {
                                                        format!(
                                                            "{}.{}",
                                                            f.name,
                                                            f.extension.to_lowercase()
                                                        )
                                                    };
                                                    full_name.eq_ignore_ascii_case(&name)
                                                }) {
                                                    self.selected_index = idx;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Esc => {
                                self.modal = Modal::None;
                            }
                            _ => {}
                        }
                    }
                    FindPhase::NoResults => {
                        // Any key closes
                        self.modal = Modal::None;
                    }
                }
            }
            Modal::BatchRename(ref mut state) => {
                match key.code {
                    KeyCode::Enter => {
                        // Rename the current file
                        if let Some((path, _)) = state.current_file().cloned() {
                            let new_name = state.input.clone();
                            if !new_name.is_empty()
                                && new_name
                                    != path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default()
                            {
                                let new_path = path.parent().unwrap_or(&path).join(&new_name);
                                match fs::rename(&path, &new_path) {
                                    Ok(()) => {
                                        state.renamed_count += 1;
                                        state.last_error = None;
                                        // Remove from tagged files
                                        self.tagged_files.retain(|p| *p != path);
                                    }
                                    Err(e) => {
                                        state.last_error = Some(format!("Error: {}", e));
                                    }
                                }
                            }
                        }
                        // Move to next file
                        state.next();
                        if state.is_complete() {
                            // Done - refresh and show summary
                            let count = state.renamed_count;
                            self.modal = Modal::None;
                            let _ = self.refresh_files();
                            if count > 0 {
                                self.modal = Modal::Success(format!("Renamed {} file(s)", count));
                            }
                        }
                    }
                    KeyCode::Esc => {
                        // Exit batch rename
                        let count = state.renamed_count;
                        self.modal = Modal::None;
                        let _ = self.refresh_files();
                        if count > 0 {
                            self.modal = Modal::Success(format!("Renamed {} file(s)", count));
                        }
                    }
                    KeyCode::Backspace => {
                        state.input.pop();
                        state.last_error = None;
                    }
                    KeyCode::Char(c) => {
                        state.input.push(c);
                        state.last_error = None;
                    }
                    KeyCode::Tab => {
                        // Skip this file without renaming
                        state.next();
                        if state.is_complete() {
                            let count = state.renamed_count;
                            self.modal = Modal::None;
                            let _ = self.refresh_files();
                            if count > 0 {
                                self.modal = Modal::Success(format!("Renamed {} file(s)", count));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Modal::Attribute(ref mut state) => {
                match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        state.prev_attr();
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        state.next_attr();
                    }
                    KeyCode::Char(' ') => {
                        // Toggle current attribute
                        state.toggle_current();
                    }
                    KeyCode::Enter => {
                        if state.display_only {
                            // Just close in display mode
                            self.modal = Modal::None;
                        } else {
                            // Apply attribute changes
                            let path = state.path.clone();
                            let attrs = state.attrs;
                            let for_tagged = state.for_tagged;

                            // Apply to all tagged files if for_tagged, otherwise just the one file
                            let files_to_update: Vec<PathBuf> = if for_tagged {
                                self.tagged_files.clone()
                            } else {
                                vec![path]
                            };

                            let mut success_count = 0;
                            let mut error_msg = None;

                            for file_path in files_to_update {
                                match apply_attributes(&file_path, &attrs) {
                                    Ok(()) => success_count += 1,
                                    Err(e) => {
                                        if error_msg.is_none() {
                                            error_msg = Some(format!("Error: {}", e));
                                        }
                                    }
                                }
                            }

                            self.modal = Modal::None;
                            let _ = self.refresh_files();

                            if let Some(err) = error_msg {
                                self.modal = Modal::Error(err);
                            } else if success_count > 0 {
                                self.modal = Modal::Success(format!(
                                    "Updated attributes for {} file(s)",
                                    success_count
                                ));
                            }
                        }
                    }
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                }
            }
            Modal::SearchSpec(ref mut state) => {
                match state.phase {
                    0 => {
                        // Phase 0: Editing pattern
                        match key.code {
                            KeyCode::Enter => {
                                // Move to attribute selection phase
                                state.phase = 1;
                            }
                            KeyCode::Esc => {
                                self.modal = Modal::None;
                            }
                            KeyCode::Backspace => {
                                state.pattern.pop();
                            }
                            KeyCode::Char(c) => {
                                state.pattern.push(c);
                            }
                            _ => {}
                        }
                    }
                    1 => {
                        // Phase 1: Editing attributes
                        match key.code {
                            KeyCode::Left | KeyCode::Char('h') => {
                                if state.selected_attr > 0 {
                                    state.selected_attr -= 1;
                                }
                            }
                            KeyCode::Right | KeyCode::Char('l') => {
                                if state.selected_attr < 5 {
                                    state.selected_attr += 1;
                                }
                            }
                            KeyCode::Char(' ') => {
                                state.toggle_current();
                            }
                            KeyCode::Enter => {
                                // Apply the search specification
                                let pattern = state.pattern.clone();
                                self.search_spec = pattern;
                                self.modal = Modal::None;
                                let _ = self.refresh_files();
                            }
                            KeyCode::Esc => {
                                // Go back to pattern phase
                                state.phase = 0;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Modal::Help(ref mut state) => {
                match key.code {
                    KeyCode::Esc => {
                        if state.current_topic == 0 {
                            // On index page, close help
                            self.modal = Modal::None;
                        } else {
                            // On topic page, go back to index
                            state.current_topic = 0;
                            state.scroll_offset = 0;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if state.scroll_offset > 0 {
                            state.scroll_offset -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        state.scroll_offset += 1;
                    }
                    KeyCode::PageUp => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        state.scroll_offset += 10;
                    }
                    KeyCode::Home => {
                        state.scroll_offset = 0;
                    }
                    KeyCode::Char(c) => {
                        // Check if character matches a topic key
                        let c_upper = c.to_ascii_uppercase();
                        for (i, topic) in state.topics.iter().enumerate() {
                            if topic.key == c_upper {
                                state.current_topic = i + 1;
                                state.scroll_offset = 0;
                                break;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if state.current_topic == 0 {
                            // On index, go to first topic
                            state.current_topic = 1;
                            state.scroll_offset = 0;
                        }
                    }
                    _ => {}
                }
            }
            Modal::Progress(ref mut state) => {
                match key.code {
                    KeyCode::Esc => {
                        // Cancel operation
                        let completed = state.completed;
                        self.modal = Modal::Success(format!(
                            "Cancelled. {} completed, {} remaining.",
                            completed,
                            state.files.len() - state.current_index
                        ));
                        // Clear tagged files and refresh
                        self.tagged_files.clear();
                        let _ = self.refresh_files();
                    }
                    _ => {
                        // Process next file on any key press
                        self.process_next_progress_file();
                    }
                }
            }
            Modal::Status(_) | Modal::Space | Modal::Error(_) | Modal::Success(_) => {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                }
            }
            Modal::ColorTheme(state) => match key.code {
                KeyCode::Esc => {
                    // Cancel - restore original theme
                    self.color_theme = state.original_theme;
                    self.modal = Modal::None;
                }
                KeyCode::Enter => {
                    // Apply selected theme
                    self.color_theme = state.selected_theme();
                    self.modal = Modal::None;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if state.selected > 0 {
                        state.selected -= 1;
                        // Live preview
                        self.color_theme = state.selected_theme();
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.selected < ColorTheme::ALL.len() - 1 {
                        state.selected += 1;
                        // Live preview
                        self.color_theme = state.selected_theme();
                    }
                }
                KeyCode::Char('1') => {
                    state.selected = 0;
                    self.color_theme = state.selected_theme();
                }
                KeyCode::Char('2') => {
                    state.selected = 1;
                    self.color_theme = state.selected_theme();
                }
                KeyCode::Char('3') => {
                    state.selected = 2;
                    self.color_theme = state.selected_theme();
                }
                KeyCode::Char('4') => {
                    state.selected = 3;
                    self.color_theme = state.selected_theme();
                }
                KeyCode::Char('5') => {
                    state.selected = 4;
                    self.color_theme = state.selected_theme();
                }
                _ => {}
            },
            Modal::Qdstart(ref mut state) => {
                if state.editing {
                    // Handle text input mode
                    match key.code {
                        KeyCode::Enter => {
                            // Apply the edited value
                            let current_field = state.current_field();
                            match current_field {
                                QdstartField::SearchSpec => {
                                    state.search_spec = state.input_buffer.clone();
                                }
                                QdstartField::Editor => {
                                    if state.input_buffer.is_empty()
                                        || state.input_buffer == "$EDITOR"
                                    {
                                        state.editor = None;
                                    } else {
                                        state.editor = Some(state.input_buffer.clone());
                                    }
                                }
                                _ => {}
                            }
                            state.editing = false;
                            state.input_buffer.clear();
                        }
                        KeyCode::Esc => {
                            state.editing = false;
                            state.input_buffer.clear();
                        }
                        KeyCode::Backspace => {
                            state.input_buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            state.input_buffer.push(c);
                        }
                        _ => {}
                    }
                } else {
                    // Handle navigation/selection mode
                    match key.code {
                        KeyCode::Esc => {
                            self.modal = Modal::None;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if state.selected > 0 {
                                state.selected -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if state.selected < QdstartField::ALL.len() - 1 {
                                state.selected += 1;
                            }
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            // Toggle or edit based on field type
                            let current_field = state.current_field();
                            match current_field {
                                QdstartField::SearchSpec | QdstartField::Editor => {
                                    // Enter editing mode
                                    state.editing = true;
                                    match current_field {
                                        QdstartField::SearchSpec => {
                                            state.input_buffer = state.search_spec.clone();
                                        }
                                        QdstartField::Editor => {
                                            state.input_buffer = state
                                                .editor
                                                .clone()
                                                .unwrap_or_else(|| "$EDITOR".to_string());
                                        }
                                        _ => {}
                                    }
                                }
                                QdstartField::SortMethod => {
                                    state.cycle_sort_method();
                                }
                                QdstartField::SortDirection => {
                                    state.toggle_sort_direction();
                                }
                                QdstartField::ShowHidden => {
                                    state.show_hidden = !state.show_hidden;
                                }
                                QdstartField::ConfirmDelete => {
                                    state.confirm_delete = !state.confirm_delete;
                                }
                                QdstartField::ColorTheme => {
                                    state.cycle_theme();
                                    // Live preview
                                    self.color_theme = state.theme();
                                }
                                QdstartField::MouseSupport => {
                                    state.mouse_support = !state.mouse_support;
                                }
                                QdstartField::UppercaseNames => {
                                    state.uppercase_names = !state.uppercase_names;
                                }
                            }
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            // Save configuration - clone state to avoid borrow issues
                            let state_clone = state.clone();
                            self.apply_qdstart_settings(&state_clone);
                            if let Err(e) = self.save_config() {
                                self.modal = Modal::Error(format!("Failed to save config: {}", e));
                            } else {
                                self.modal =
                                    Modal::Success("Configuration saved successfully".to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            Modal::Git(ref mut state) => {
                if !state.is_repo {
                    // Not a git repo - any key closes
                    match key.code {
                        KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                            self.modal = Modal::None;
                        }
                        _ => {}
                    }
                } else {
                    match state.view {
                        GitView::Menu => match key.code {
                            KeyCode::Esc => {
                                self.modal = Modal::None;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.menu_selected > 0 {
                                    state.menu_selected -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if state.menu_selected < GitMenuItem::ALL.len() - 1 {
                                    state.menu_selected += 1;
                                }
                            }
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                let item = GitMenuItem::ALL[state.menu_selected];
                                let path = self.current_path.clone();
                                match item {
                                    GitMenuItem::Status => {
                                        state.view = GitView::Status;
                                        git_ops::load_git_status(state, &path);
                                    }
                                    GitMenuItem::Log => {
                                        state.view = GitView::Log;
                                        git_ops::load_git_log(state, &path);
                                    }
                                    GitMenuItem::Diff => {
                                        state.view = GitView::Diff;
                                        git_ops::load_git_diff(state, &path);
                                    }
                                    GitMenuItem::Commit => {
                                        state.view = GitView::Commit;
                                        state.commit_input_mode = true;
                                    }
                                    GitMenuItem::Push => {
                                        state.remote_action = RemoteAction::Push;
                                        git_ops::load_remotes(state, &path);
                                        state.view = GitView::Remote;
                                    }
                                    GitMenuItem::Pull => {
                                        state.remote_action = RemoteAction::Pull;
                                        git_ops::load_remotes(state, &path);
                                        state.view = GitView::Remote;
                                    }
                                    GitMenuItem::Branch => {
                                        state.view = GitView::Branch;
                                        git_ops::load_branches(state, &path);
                                    }
                                    GitMenuItem::Stash => {
                                        state.view = GitView::Stash;
                                        git_ops::load_stashes(state, &path);
                                    }
                                    GitMenuItem::Tag => {
                                        state.view = GitView::Tag;
                                        git_ops::load_tags(state, &path);
                                    }
                                    GitMenuItem::Config => {
                                        state.view = GitView::Config;
                                        git_ops::load_git_config(state, &path);
                                    }
                                    GitMenuItem::Conflicts => {
                                        state.view = GitView::Conflicts;
                                        git_ops::load_conflict_files(state, &path);
                                    }
                                    GitMenuItem::Submodules => {
                                        state.view = GitView::Submodules;
                                        git_ops::load_submodules(state, &path);
                                    }
                                }
                            }
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                state.view = GitView::Status;
                                let path = self.current_path.clone();
                                git_ops::load_git_status(state, &path);
                            }
                            KeyCode::Char('l') | KeyCode::Char('L') => {
                                state.view = GitView::Log;
                                let path = self.current_path.clone();
                                git_ops::load_git_log(state, &path);
                            }
                            KeyCode::Char('d') | KeyCode::Char('D') => {
                                state.view = GitView::Diff;
                                let path = self.current_path.clone();
                                git_ops::load_git_diff(state, &path);
                            }
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                state.view = GitView::Commit;
                                state.commit_input_mode = true;
                            }
                            KeyCode::Char('p') | KeyCode::Char('P') => {
                                state.remote_action = RemoteAction::Push;
                                let path = self.current_path.clone();
                                git_ops::load_remotes(state, &path);
                                state.view = GitView::Remote;
                            }
                            KeyCode::Char('u') | KeyCode::Char('U') => {
                                state.remote_action = RemoteAction::Pull;
                                let path = self.current_path.clone();
                                git_ops::load_remotes(state, &path);
                                state.view = GitView::Remote;
                            }
                            KeyCode::Char('b') | KeyCode::Char('B') => {
                                state.view = GitView::Branch;
                                let path = self.current_path.clone();
                                git_ops::load_branches(state, &path);
                            }
                            KeyCode::Char('h') | KeyCode::Char('H') => {
                                state.view = GitView::Stash;
                                let path = self.current_path.clone();
                                git_ops::load_stashes(state, &path);
                            }
                            KeyCode::Char('t') | KeyCode::Char('T') => {
                                state.view = GitView::Tag;
                                let path = self.current_path.clone();
                                git_ops::load_tags(state, &path);
                            }
                            KeyCode::Char('g') | KeyCode::Char('G') => {
                                state.view = GitView::Config;
                                let path = self.current_path.clone();
                                git_ops::load_git_config(state, &path);
                            }
                            KeyCode::Char('x') | KeyCode::Char('X') => {
                                state.view = GitView::Conflicts;
                                let path = self.current_path.clone();
                                git_ops::load_conflict_files(state, &path);
                            }
                            KeyCode::Char('m') | KeyCode::Char('M') => {
                                state.view = GitView::Submodules;
                                let path = self.current_path.clone();
                                git_ops::load_submodules(state, &path);
                            }
                            _ => {}
                        },
                        GitView::Status => match key.code {
                            KeyCode::Esc => {
                                state.view = GitView::Menu;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.selected_file > 0 {
                                    state.selected_file -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if state.selected_file + 1 < state.files.len() {
                                    state.selected_file += 1;
                                }
                            }
                            KeyCode::Enter => {
                                // Show diff for selected file
                                if !state.files.is_empty() {
                                    let file_path = state.files[state.selected_file].path.clone();
                                    let path = self.current_path.clone();
                                    state.prev_view = Some(GitView::Status);
                                    git_ops::load_file_diff(state, &path, &file_path);
                                    state.view = GitView::Diff;
                                }
                            }
                            KeyCode::Char('a') | KeyCode::Char('A') => {
                                let path = self.current_path.clone();
                                git_ops::toggle_git_stage(state, &path);
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                let path = self.current_path.clone();
                                git_ops::load_git_status(state, &path);
                            }
                            _ => {}
                        },
                        GitView::Log => match key.code {
                            KeyCode::Esc => {
                                state.view = GitView::Menu;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.selected_log > 0 {
                                    state.selected_log -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if state.selected_log + 1 < state.log_entries.len() {
                                    state.selected_log += 1;
                                }
                            }
                            KeyCode::Enter => {
                                // Show diff for selected commit
                                if !state.log_entries.is_empty() {
                                    let commit_hash =
                                        state.log_entries[state.selected_log].hash.clone();
                                    let path = self.current_path.clone();
                                    state.prev_view = Some(GitView::Log);
                                    git_ops::load_commit_diff(state, &path, &commit_hash);
                                    state.view = GitView::Diff;
                                }
                            }
                            KeyCode::PageUp => {
                                state.selected_log = state.selected_log.saturating_sub(10);
                            }
                            KeyCode::PageDown => {
                                let max = state.log_entries.len().saturating_sub(1);
                                state.selected_log = (state.selected_log + 10).min(max);
                            }
                            _ => {}
                        },
                        GitView::Diff => match key.code {
                            KeyCode::Esc => {
                                // Return to previous view if set, otherwise menu
                                state.view = state.prev_view.take().unwrap_or(GitView::Menu);
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.scroll_offset > 0 {
                                    state.scroll_offset -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                state.scroll_offset += 1;
                            }
                            KeyCode::PageUp => {
                                state.scroll_offset = state.scroll_offset.saturating_sub(10);
                            }
                            KeyCode::PageDown => {
                                state.scroll_offset += 10;
                            }
                            _ => {}
                        },
                        GitView::Commit => {
                            if state.commit_input_mode {
                                match key.code {
                                    KeyCode::Esc => {
                                        state.commit_input_mode = false;
                                        state.view = GitView::Menu;
                                    }
                                    KeyCode::Enter => {
                                        // Shift+Enter adds a newline, Enter commits
                                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                                            state.commit_message.push('\n');
                                        } else if !state.commit_message.is_empty() {
                                            let msg = state.commit_message.clone();
                                            let path = self.current_path.clone();
                                            match git_ops::execute_git_commit(&msg, &path) {
                                                Ok(_) => {
                                                    state.commit_message.clear();
                                                    state.commit_input_mode = false;
                                                    self.modal = Modal::Success(
                                                        "Commit successful".to_string(),
                                                    );
                                                }
                                                Err(e) => {
                                                    state.error = Some(e);
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Backspace => {
                                        state.commit_message.pop();
                                    }
                                    KeyCode::Char(c) => {
                                        state.commit_message.push(c);
                                    }
                                    _ => {}
                                }
                            } else if key.code == KeyCode::Esc {
                                state.view = GitView::Menu;
                            }
                        }
                        GitView::Branch => {
                            if state.branch_input_mode {
                                match key.code {
                                    KeyCode::Esc => {
                                        state.branch_input_mode = false;
                                        state.branch_name_input.clear();
                                    }
                                    KeyCode::Enter => {
                                        if !state.branch_name_input.is_empty() {
                                            let name = state.branch_name_input.clone();
                                            let path = self.current_path.clone();
                                            match git_ops::create_branch(&name, &path) {
                                                Ok(msg) => {
                                                    state.branch_name_input.clear();
                                                    state.branch_input_mode = false;
                                                    git_ops::load_branches(state, &path);
                                                    self.modal = Modal::Success(msg);
                                                }
                                                Err(e) => {
                                                    state.error = Some(e);
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Backspace => {
                                        state.branch_name_input.pop();
                                    }
                                    KeyCode::Char(c) => {
                                        state.branch_name_input.push(c);
                                    }
                                    _ => {}
                                }
                            } else {
                                match key.code {
                                    KeyCode::Esc => {
                                        state.view = GitView::Menu;
                                    }
                                    KeyCode::Up | KeyCode::Char('k') => {
                                        if state.selected_branch > 0 {
                                            state.selected_branch -= 1;
                                        }
                                    }
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        if state.selected_branch + 1 < state.branches.len() {
                                            state.selected_branch += 1;
                                        }
                                    }
                                    KeyCode::Enter => {
                                        // Switch to selected branch
                                        if !state.branches.is_empty() {
                                            let branch = &state.branches[state.selected_branch];
                                            if !branch.is_current && !branch.is_remote {
                                                let name = branch.name.clone();
                                                let path = self.current_path.clone();
                                                match git_ops::switch_branch(&name, &path) {
                                                    Ok(msg) => {
                                                        git_ops::load_branches(state, &path);
                                                        self.modal = Modal::Success(msg);
                                                    }
                                                    Err(e) => {
                                                        state.error = Some(e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char('n') | KeyCode::Char('N') => {
                                        // New branch
                                        state.branch_input_mode = true;
                                        state.branch_name_input.clear();
                                    }
                                    KeyCode::Char('d') | KeyCode::Char('D') => {
                                        // Delete branch
                                        if !state.branches.is_empty() {
                                            let branch = &state.branches[state.selected_branch];
                                            if !branch.is_current && !branch.is_remote {
                                                let name = branch.name.clone();
                                                let path = self.current_path.clone();
                                                match git_ops::delete_branch(&name, &path) {
                                                    Ok(msg) => {
                                                        git_ops::load_branches(state, &path);
                                                        self.modal = Modal::Success(msg);
                                                    }
                                                    Err(e) => {
                                                        state.error = Some(e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char('r') | KeyCode::Char('R') => {
                                        // Refresh
                                        let path = self.current_path.clone();
                                        git_ops::load_branches(state, &path);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        GitView::Stash => {
                            if state.stash_input_mode {
                                match key.code {
                                    KeyCode::Esc => {
                                        state.stash_input_mode = false;
                                        state.stash_message_input.clear();
                                    }
                                    KeyCode::Enter => {
                                        let path = self.current_path.clone();
                                        let msg = if state.stash_message_input.is_empty() {
                                            None
                                        } else {
                                            Some(state.stash_message_input.as_str())
                                        };
                                        match git_ops::create_stash(msg, &path) {
                                            Ok(result) => {
                                                state.stash_message_input.clear();
                                                state.stash_input_mode = false;
                                                git_ops::load_stashes(state, &path);
                                                self.modal = Modal::Success(result);
                                            }
                                            Err(e) => {
                                                state.error = Some(e);
                                            }
                                        }
                                    }
                                    KeyCode::Backspace => {
                                        state.stash_message_input.pop();
                                    }
                                    KeyCode::Char(c) => {
                                        state.stash_message_input.push(c);
                                    }
                                    _ => {}
                                }
                            } else {
                                match key.code {
                                    KeyCode::Esc => {
                                        state.view = GitView::Menu;
                                    }
                                    KeyCode::Up | KeyCode::Char('k') => {
                                        if state.selected_stash > 0 {
                                            state.selected_stash -= 1;
                                        }
                                    }
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        if state.selected_stash + 1 < state.stashes.len() {
                                            state.selected_stash += 1;
                                        }
                                    }
                                    KeyCode::Char('s') | KeyCode::Char('S') => {
                                        // Create new stash
                                        state.stash_input_mode = true;
                                        state.stash_message_input.clear();
                                    }
                                    KeyCode::Char('p') | KeyCode::Char('P') => {
                                        // Pop stash (apply and remove)
                                        let path = self.current_path.clone();
                                        match git_ops::pop_stash(&path) {
                                            Ok(msg) => {
                                                git_ops::load_stashes(state, &path);
                                                self.modal = Modal::Success(msg);
                                            }
                                            Err(e) => {
                                                state.error = Some(e);
                                            }
                                        }
                                    }
                                    KeyCode::Char('a') | KeyCode::Char('A') => {
                                        // Apply stash (keep stash)
                                        if !state.stashes.is_empty() {
                                            let idx = state.stashes[state.selected_stash].index;
                                            let path = self.current_path.clone();
                                            match git_ops::apply_stash(idx, &path) {
                                                Ok(msg) => {
                                                    self.modal = Modal::Success(msg);
                                                }
                                                Err(e) => {
                                                    state.error = Some(e);
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char('d') | KeyCode::Char('D') => {
                                        // Drop stash
                                        if !state.stashes.is_empty() {
                                            let idx = state.stashes[state.selected_stash].index;
                                            let path = self.current_path.clone();
                                            match git_ops::drop_stash(idx, &path) {
                                                Ok(msg) => {
                                                    git_ops::load_stashes(state, &path);
                                                    self.modal = Modal::Success(msg);
                                                }
                                                Err(e) => {
                                                    state.error = Some(e);
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char('r') | KeyCode::Char('R') => {
                                        // Refresh
                                        let path = self.current_path.clone();
                                        git_ops::load_stashes(state, &path);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        GitView::Tag => {
                            if state.tag_input_mode {
                                match key.code {
                                    KeyCode::Esc => {
                                        state.tag_input_mode = false;
                                        state.tag_name_input.clear();
                                    }
                                    KeyCode::Enter => {
                                        if !state.tag_name_input.is_empty() {
                                            let name = state.tag_name_input.clone();
                                            let path = self.current_path.clone();
                                            match git_ops::create_tag(&name, &path) {
                                                Ok(msg) => {
                                                    state.tag_name_input.clear();
                                                    state.tag_input_mode = false;
                                                    git_ops::load_tags(state, &path);
                                                    self.modal = Modal::Success(msg);
                                                }
                                                Err(e) => {
                                                    state.error = Some(e);
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Backspace => {
                                        state.tag_name_input.pop();
                                    }
                                    KeyCode::Char(c) => {
                                        state.tag_name_input.push(c);
                                    }
                                    _ => {}
                                }
                            } else {
                                match key.code {
                                    KeyCode::Esc => {
                                        state.view = GitView::Menu;
                                    }
                                    KeyCode::Up | KeyCode::Char('k') => {
                                        if state.selected_tag > 0 {
                                            state.selected_tag -= 1;
                                        }
                                    }
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        if state.selected_tag + 1 < state.tags.len() {
                                            state.selected_tag += 1;
                                        }
                                    }
                                    KeyCode::Char('n') | KeyCode::Char('N') => {
                                        // New tag
                                        state.tag_input_mode = true;
                                        state.tag_name_input.clear();
                                    }
                                    KeyCode::Char('d') | KeyCode::Char('D') => {
                                        // Delete tag
                                        if !state.tags.is_empty() {
                                            let name = state.tags[state.selected_tag].name.clone();
                                            let path = self.current_path.clone();
                                            match git_ops::delete_tag(&name, &path) {
                                                Ok(msg) => {
                                                    git_ops::load_tags(state, &path);
                                                    self.modal = Modal::Success(msg);
                                                }
                                                Err(e) => {
                                                    state.error = Some(e);
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char('p') | KeyCode::Char('P') => {
                                        // Push tags to remote
                                        let path = self.current_path.clone();
                                        match git_ops::push_tags(&path) {
                                            Ok(msg) => {
                                                self.modal = Modal::Success(msg);
                                            }
                                            Err(e) => {
                                                state.error = Some(e);
                                            }
                                        }
                                    }
                                    KeyCode::Char('r') | KeyCode::Char('R') => {
                                        // Refresh
                                        let path = self.current_path.clone();
                                        git_ops::load_tags(state, &path);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        GitView::Remote => match key.code {
                            KeyCode::Esc => {
                                state.view = GitView::Menu;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.selected_remote > 0 {
                                    state.selected_remote -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if state.selected_remote + 1 < state.remotes.len() {
                                    state.selected_remote += 1;
                                }
                            }
                            KeyCode::Enter => {
                                // Execute push/pull to selected remote
                                if !state.remotes.is_empty() {
                                    let remote_name =
                                        state.remotes[state.selected_remote].name.clone();
                                    let path = self.current_path.clone();
                                    let result = match state.remote_action {
                                        RemoteAction::Push => {
                                            git_ops::execute_git_push_to(&remote_name, &path)
                                        }
                                        RemoteAction::Pull => {
                                            git_ops::execute_git_pull_from(&remote_name, &path)
                                        }
                                    };
                                    match result {
                                        Ok(msg) => {
                                            self.modal = Modal::Success(msg);
                                        }
                                        Err(e) => {
                                            state.error = Some(e);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        },
                        GitView::Config => match key.code {
                            KeyCode::Esc => {
                                state.view = GitView::Menu;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.selected_config > 0 {
                                    state.selected_config -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if state.selected_config + 1 < state.config_entries.len() {
                                    state.selected_config += 1;
                                }
                            }
                            KeyCode::PageUp => {
                                state.selected_config = state.selected_config.saturating_sub(10);
                            }
                            KeyCode::PageDown => {
                                let max = state.config_entries.len().saturating_sub(1);
                                state.selected_config = (state.selected_config + 10).min(max);
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                // Refresh
                                let path = self.current_path.clone();
                                git_ops::load_git_config(state, &path);
                            }
                            _ => {}
                        },
                        GitView::Conflicts => match key.code {
                            KeyCode::Esc => {
                                state.view = GitView::Menu;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                // Navigate sections within current file
                                if !state.conflict_files.is_empty() {
                                    let file =
                                        &mut state.conflict_files[state.selected_conflict_file];
                                    if file.selected_section > 0 {
                                        file.selected_section -= 1;
                                    }
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if !state.conflict_files.is_empty() {
                                    let file =
                                        &mut state.conflict_files[state.selected_conflict_file];
                                    if file.selected_section + 1 < file.sections.len() {
                                        file.selected_section += 1;
                                    }
                                }
                            }
                            KeyCode::Left | KeyCode::Char('h') => {
                                // Previous file
                                if state.selected_conflict_file > 0 {
                                    state.selected_conflict_file -= 1;
                                }
                            }
                            KeyCode::Right | KeyCode::Char('l') => {
                                // Next file
                                if state.selected_conflict_file + 1 < state.conflict_files.len() {
                                    state.selected_conflict_file += 1;
                                }
                            }
                            KeyCode::Char('o') | KeyCode::Char('O') => {
                                // Choose ours
                                if !state.conflict_files.is_empty() {
                                    let file_idx = state.selected_conflict_file;
                                    let file = &state.conflict_files[file_idx];
                                    let section_idx = file.selected_section;
                                    let file_path = file.path.clone();
                                    let path = self.current_path.clone();
                                    match git_ops::resolve_conflict_section(
                                        &file_path,
                                        section_idx,
                                        ConflictResolution::Ours,
                                        &path,
                                    ) {
                                        Ok(_) => {
                                            state.error = None;
                                            // Reload conflicts
                                            git_ops::load_conflict_files(state, &path);
                                        }
                                        Err(e) => {
                                            state.error = Some(e);
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('t') | KeyCode::Char('T') => {
                                // Choose theirs
                                if !state.conflict_files.is_empty() {
                                    let file_idx = state.selected_conflict_file;
                                    let file = &state.conflict_files[file_idx];
                                    let section_idx = file.selected_section;
                                    let file_path = file.path.clone();
                                    let path = self.current_path.clone();
                                    match git_ops::resolve_conflict_section(
                                        &file_path,
                                        section_idx,
                                        ConflictResolution::Theirs,
                                        &path,
                                    ) {
                                        Ok(_) => {
                                            state.error = None;
                                            git_ops::load_conflict_files(state, &path);
                                        }
                                        Err(e) => {
                                            state.error = Some(e);
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('b') | KeyCode::Char('B') => {
                                // Choose both
                                if !state.conflict_files.is_empty() {
                                    let file_idx = state.selected_conflict_file;
                                    let file = &state.conflict_files[file_idx];
                                    let section_idx = file.selected_section;
                                    let file_path = file.path.clone();
                                    let path = self.current_path.clone();
                                    match git_ops::resolve_conflict_section(
                                        &file_path,
                                        section_idx,
                                        ConflictResolution::Both,
                                        &path,
                                    ) {
                                        Ok(_) => {
                                            state.error = None;
                                            git_ops::load_conflict_files(state, &path);
                                        }
                                        Err(e) => {
                                            state.error = Some(e);
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('m') | KeyCode::Char('M') => {
                                // Mark file as resolved
                                if !state.conflict_files.is_empty() {
                                    let file = &state.conflict_files[state.selected_conflict_file];
                                    let file_path = file.path.clone();
                                    let path = self.current_path.clone();
                                    match git_ops::mark_conflict_resolved(&file_path, &path) {
                                        Ok(_) => {
                                            state.error = None;
                                            git_ops::load_conflict_files(state, &path);
                                        }
                                        Err(e) => {
                                            state.error = Some(e);
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('a') | KeyCode::Char('A') => {
                                // Abort merge
                                let path = self.current_path.clone();
                                match git_ops::abort_merge(&path) {
                                    Ok(_) => {
                                        state.error = None;
                                        state.view = GitView::Menu;
                                    }
                                    Err(e) => {
                                        state.error = Some(e);
                                    }
                                }
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                // Refresh
                                let path = self.current_path.clone();
                                git_ops::load_conflict_files(state, &path);
                            }
                            _ => {}
                        },
                        GitView::Submodules => match key.code {
                            KeyCode::Esc => {
                                state.view = GitView::Menu;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.selected_submodule > 0 {
                                    state.selected_submodule -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if state.selected_submodule + 1 < state.submodules.len() {
                                    state.selected_submodule += 1;
                                }
                            }
                            KeyCode::Char('i') | KeyCode::Char('I') => {
                                // Init selected submodule or all if none selected
                                let path = self.current_path.clone();
                                let submodule_path = if !state.submodules.is_empty() {
                                    Some(state.submodules[state.selected_submodule].path.clone())
                                } else {
                                    None
                                };
                                match git_ops::init_submodule(submodule_path.as_deref(), &path) {
                                    Ok(msg) => {
                                        state.error = Some(msg);
                                        git_ops::load_submodules(state, &path);
                                    }
                                    Err(e) => {
                                        state.error = Some(e);
                                    }
                                }
                            }
                            KeyCode::Char('u') | KeyCode::Char('U') => {
                                // Update selected submodule or all
                                let path = self.current_path.clone();
                                let submodule_path = if !state.submodules.is_empty() {
                                    Some(state.submodules[state.selected_submodule].path.clone())
                                } else {
                                    None
                                };
                                match git_ops::update_submodule(submodule_path.as_deref(), &path) {
                                    Ok(msg) => {
                                        state.error = Some(msg);
                                        git_ops::load_submodules(state, &path);
                                    }
                                    Err(e) => {
                                        state.error = Some(e);
                                    }
                                }
                            }
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                // Sync submodule URLs
                                let path = self.current_path.clone();
                                match git_ops::sync_submodules(&path) {
                                    Ok(msg) => {
                                        state.error = Some(msg);
                                        git_ops::load_submodules(state, &path);
                                    }
                                    Err(e) => {
                                        state.error = Some(e);
                                    }
                                }
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                // Refresh
                                let path = self.current_path.clone();
                                git_ops::load_submodules(state, &path);
                            }
                            _ => {}
                        },
                    }
                }
            }
            Modal::Beads(ref mut state) => {
                // Delegate key handling to BeadsPlugin
                let cwd = self.current_path.clone();
                if let Some(beads_plugin) = self.plugin_manager.beads_plugin_mut() {
                    let result = beads_plugin.handle_external_state_key(key, state, &cwd);
                    match result {
                        KeyHandleResult::CloseModal => {
                            self.modal = Modal::None;
                        }
                        KeyHandleResult::CloseWithSuccess(msg) => {
                            self.modal = Modal::Success(msg);
                        }
                        KeyHandleResult::CloseWithError(msg) => {
                            self.modal = Modal::Error(msg);
                        }
                        _ => {}
                    }
                }
            }
            Modal::Clipboard(ref mut state) => {
                use crate::clipboard::copy_to_clipboard;
                match key.code {
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if state.selected > 0 {
                            state.selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if state.selected + 1 < state.items.len() {
                            state.selected += 1;
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(item) = state.selected_item() {
                            let value = item.value.clone();
                            if let Err(e) = copy_to_clipboard(&value) {
                                self.modal = Modal::Error(e);
                            } else {
                                self.modal = Modal::Success(format!("Copied: {}", value));
                            }
                        }
                    }
                    // Quick keys: 1-9 to select item directly
                    KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                        let idx = (c as usize) - ('1' as usize);
                        if idx < state.items.len() {
                            let value = state.items[idx].value.clone();
                            if let Err(e) = copy_to_clipboard(&value) {
                                self.modal = Modal::Error(e);
                            } else {
                                self.modal = Modal::Success(format!("Copied: {}", value));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Modal::Plugin(ref plugin_id) => {
                // Plugin modals are handled by handle_plugin_key before this function is called
                // This is a fallback - delegate to plugin manager
                let is_theme = plugin_id == "theme";
                let is_qdconfig = plugin_id == "qdconfig";
                let result = self
                    .plugin_manager
                    .handle_modal_key(key, &self.current_path);

                // Live preview for theme plugin - update theme as user navigates
                if is_theme && result == KeyHandleResult::Handled {
                    if let Some(theme_plugin) = self.plugin_manager.theme_plugin_mut() {
                        if let Some(theme) = theme_plugin.selected_theme() {
                            self.color_theme = theme;
                        }
                    }
                }

                // Live preview for qdconfig plugin - update theme as user cycles through themes
                if is_qdconfig && result == KeyHandleResult::Handled {
                    if let Some(qdconfig_plugin) = self.plugin_manager.qdconfig_plugin_mut() {
                        if let Some(theme) = qdconfig_plugin.preview_theme() {
                            self.color_theme = theme;
                        }
                    }
                }

                self.handle_plugin_result(result);
            }
            Modal::None => {}
        }

        Ok(())
    }

    /// Apply QDSTART settings to app state
    fn apply_qdstart_settings(&mut self, state: &QdstartState) {
        self.search_spec = state.search_spec.clone();
        self.show_hidden = state.show_hidden;
        self.color_theme = state.theme();

        // Convert sort settings to SortMode
        let method = state.sort_method;
        let asc = state.sort_asc;
        self.sort_mode = match (method, asc) {
            (0, true) => SortMode::NameAsc,
            (0, false) => SortMode::NameDesc,
            (1, true) => SortMode::ExtAsc,
            (1, false) => SortMode::ExtDesc,
            (2, true) => SortMode::SizeAsc,
            (2, false) => SortMode::SizeDesc,
            (3, true) => SortMode::DateAsc,
            (3, false) => SortMode::DateDesc,
            _ => SortMode::None,
        };

        // Update config
        self.config.general.search_spec = state.search_spec.clone();
        self.config.general.show_hidden = state.show_hidden;
        self.config.general.confirm_delete = state.confirm_delete;
        self.config.general.mouse_support = state.mouse_support;
        self.config.display.uppercase_names = state.uppercase_names;
        self.config.display.theme = state.theme().into();
        self.config.editor.command = state.editor.clone();
        self.config.from_sort_mode(self.sort_mode);
    }

    /// Create QDSTART state from current settings
    #[allow(dead_code)] // Legacy - QdconfigPlugin now handles this
    pub fn create_qdstart_state(&self) -> QdstartState {
        // Convert current SortMode to method + direction
        let (sort_method, sort_asc) = match self.sort_mode {
            SortMode::NameAsc => (0, true),
            SortMode::NameDesc => (0, false),
            SortMode::ExtAsc => (1, true),
            SortMode::ExtDesc => (1, false),
            SortMode::SizeAsc => (2, true),
            SortMode::SizeDesc => (2, false),
            SortMode::DateAsc => (3, true),
            SortMode::DateDesc => (3, false),
            SortMode::None => (4, true),
        };

        // Get current theme index
        let theme_index = ColorTheme::ALL
            .iter()
            .position(|&t| t == self.color_theme)
            .unwrap_or(0);

        QdstartState::new(
            self.search_spec.clone(),
            sort_method,
            sort_asc,
            self.show_hidden,
            self.config.general.confirm_delete,
            self.config.editor.command.clone(),
            theme_index,
            self.config.general.mouse_support,
            self.config.display.uppercase_names,
        )
    }

    /// Move file selection up or down
    fn move_selection(&mut self, delta: i32) {
        if self.files.is_empty() {
            return;
        }

        let new_index = if delta < 0 {
            self.selected_index.saturating_sub((-delta) as usize)
        } else {
            (self.selected_index + delta as usize).min(self.files.len() - 1)
        };

        self.selected_index = new_index;

        // Only adjust scroll when selection goes outside visible window
        // UI will use this as starting point and make final adjustments
        // Use conservative estimate - actual UI may show more rows
        let visible_height = 15usize; // Conservative estimate
        if new_index < self.scroll_offset {
            // Selection moved above visible area - scroll up to show it
            self.scroll_offset = new_index;
        } else if new_index >= self.scroll_offset + visible_height {
            // Selection moved below visible area - scroll down minimally
            self.scroll_offset = new_index.saturating_sub(visible_height - 1);
        }
        // If selection is within visible area, don't change scroll_offset
        // This prevents scrolling when changing direction
    }

    /// Toggle tag on currently selected file
    fn toggle_tag(&mut self) {
        if self.files.is_empty() {
            return;
        }

        let file = &self.files[self.selected_index];
        let path = file.path.clone();

        if let Some(pos) = self.tagged_files.iter().position(|p| *p == path) {
            self.tagged_files.remove(pos);
        } else {
            self.tagged_files.push(path);
        }
    }

    /// Process the next file in the progress queue
    fn process_next_progress_file(&mut self) {
        let (op, file, dest) = {
            if let Modal::Progress(ref state) = self.modal {
                if state.is_done() {
                    return;
                }
                (
                    state.operation.clone(),
                    state.current_file().cloned(),
                    state.destination.clone(),
                )
            } else {
                return;
            }
        };

        let Some(file_path) = file else {
            return;
        };

        // Process the current file
        let result = match op {
            ProgressOperation::Copy => {
                if let Some(ref dest) = dest {
                    let file_name = file_path.file_name().unwrap_or_default();
                    let dest_path = dest.join(file_name);
                    fs::copy(&file_path, &dest_path).map(|_| ())
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "No destination",
                    ))
                }
            }
            ProgressOperation::Move => {
                if let Some(ref dest) = dest {
                    let file_name = file_path.file_name().unwrap_or_default();
                    let dest_path = dest.join(file_name);
                    fs::rename(&file_path, &dest_path)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "No destination",
                    ))
                }
            }
            ProgressOperation::Erase => fs::remove_file(&file_path),
        };

        // Update progress state
        if let Modal::Progress(ref mut state) = self.modal {
            state.current_index += 1;
            match result {
                Ok(_) => state.completed += 1,
                Err(e) => {
                    state.failed += 1;
                    state.last_error = Some(e.to_string());
                }
            }

            // Check if done
            if state.is_done() {
                let completed = state.completed;
                let failed = state.failed;
                let op_name = match state.operation {
                    ProgressOperation::Copy => "copied",
                    ProgressOperation::Move => "moved",
                    ProgressOperation::Erase => "erased",
                };

                self.tagged_files.clear();
                let _ = self.refresh_files();

                if failed == 0 {
                    self.modal = Modal::Success(format!("{} file(s) {}", completed, op_name));
                } else {
                    self.modal = Modal::Success(format!(
                        "{} file(s) {}, {} failed",
                        completed, op_name, failed
                    ));
                }
            }
        }
    }

    /// Check if a file is tagged
    pub fn is_tagged(&self, path: &PathBuf) -> bool {
        self.tagged_files.contains(path)
    }

    /// Get total size of tagged files
    pub fn tagged_size(&self) -> u64 {
        self.tagged_files
            .iter()
            .filter_map(|p| self.files.iter().find(|f| &f.path == p).map(|f| f.size))
            .sum()
    }

    /// Go to parent directory
    fn go_to_parent(&mut self) -> Result<()> {
        if let Some(parent) = self.current_path.parent() {
            let parent = parent.to_path_buf();
            self.navigate_to(&parent)?;
        }
        Ok(())
    }

    /// Navigate to a new directory
    fn navigate_to(&mut self, path: &PathBuf) -> Result<()> {
        let canonical = path.canonicalize()?;

        if !canonical.is_dir() {
            anyhow::bail!("Not a directory");
        }

        // Save current path to history and clear forward stack
        self.history.push(self.current_path.clone());
        self.forward_history.clear();

        // Update state
        self.current_path = canonical.clone();
        self.files = get_directory_contents(&self.current_path, self.sort_mode)?;
        self.selected_index = 0;
        self.scroll_offset = 0;

        // Update directory watcher
        if let Some(ref mut watcher) = self.dir_watcher {
            let _ = watcher.watch_path(&canonical);
        }

        // Refresh git/beads status for new directory
        self.refresh_status_bar();

        Ok(())
    }

    /// Go back in navigation history
    fn go_back(&mut self) -> Result<()> {
        if let Some(prev_path) = self.history.pop() {
            // Save current to forward stack
            self.forward_history.push(self.current_path.clone());

            // Navigate to previous path
            self.current_path = prev_path.clone();
            self.files = get_directory_contents(&self.current_path, self.sort_mode)?;
            self.selected_index = 0;
            self.scroll_offset = 0;

            // Update directory watcher
            if let Some(ref mut watcher) = self.dir_watcher {
                let _ = watcher.watch_path(&prev_path);
            }

            self.refresh_status_bar();
        }
        Ok(())
    }

    /// Go forward in navigation history
    fn go_forward(&mut self) -> Result<()> {
        if let Some(next_path) = self.forward_history.pop() {
            // Save current to back stack
            self.history.push(self.current_path.clone());

            // Navigate to next path
            self.current_path = next_path.clone();
            self.files = get_directory_contents(&self.current_path, self.sort_mode)?;
            self.selected_index = 0;
            self.scroll_offset = 0;

            // Update directory watcher
            if let Some(ref mut watcher) = self.dir_watcher {
                let _ = watcher.watch_path(&next_path);
            }

            self.refresh_status_bar();
        }
        Ok(())
    }

    /// Cycle through sort modes
    fn cycle_sort_mode(&mut self) -> Result<()> {
        self.sort_mode = self.sort_mode.next();
        self.files = get_directory_contents(&self.current_path, self.sort_mode)?;
        Ok(())
    }

    /// Execute the current menu action or enter directory
    fn execute_action(&mut self) -> Result<()> {
        // If a directory is selected, enter it
        if !self.files.is_empty() {
            let file = &self.files[self.selected_index];
            if file.is_dir {
                let path = file.path.clone();
                return self.navigate_to(&path);
            }
        }

        // Otherwise, execute menu action
        let nav_item = NavItem::ALL[self.nav_index];
        match nav_item {
            NavItem::Directory => {
                // Open Directory Map plugin
                if let Some(dirmap_plugin) = self.plugin_manager.dirmap_plugin_mut() {
                    dirmap_plugin.open_modal(&self.current_path);
                    self.plugin_manager.set_active_modal(Some("dirmap"));
                    self.modal = Modal::Plugin("dirmap".to_string());
                }
            }
            NavItem::Tag => {
                self.toggle_tag();
            }
            NavItem::Space => {
                self.modal = Modal::Space;
            }
            NavItem::Copy => {
                if !self.tagged_files.is_empty() {
                    // Copy tagged files
                    let dest = self.current_path.to_string_lossy().to_string();
                    let files = self.tagged_files.clone();
                    if let Some(plugin) = self.plugin_manager.fileops_plugin_mut() {
                        plugin.open_modal(FileOperation::Copy, files, dest);
                        self.plugin_manager.set_active_modal(Some("fileops"));
                        self.modal = Modal::Plugin("fileops".to_string());
                    }
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else {
                    // Copy the highlighted file
                    let file = &self.files[self.selected_index];
                    let files = vec![file.path.clone()];
                    let dest = self.current_path.to_string_lossy().to_string();
                    // Temporarily tag for the operation
                    self.tagged_files.push(file.path.clone());
                    if let Some(plugin) = self.plugin_manager.fileops_plugin_mut() {
                        plugin.open_modal(FileOperation::Copy, files, dest);
                        self.plugin_manager.set_active_modal(Some("fileops"));
                        self.modal = Modal::Plugin("fileops".to_string());
                    }
                }
            }
            NavItem::Move => {
                if !self.tagged_files.is_empty() {
                    // Move tagged files
                    let dest = self.current_path.to_string_lossy().to_string();
                    let files = self.tagged_files.clone();
                    if let Some(plugin) = self.plugin_manager.fileops_plugin_mut() {
                        plugin.open_modal(FileOperation::Move, files, dest);
                        self.plugin_manager.set_active_modal(Some("fileops"));
                        self.modal = Modal::Plugin("fileops".to_string());
                    }
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else {
                    // Move the highlighted file
                    let file = &self.files[self.selected_index];
                    let files = vec![file.path.clone()];
                    let dest = self.current_path.to_string_lossy().to_string();
                    // Temporarily tag for the operation
                    self.tagged_files.push(file.path.clone());
                    if let Some(plugin) = self.plugin_manager.fileops_plugin_mut() {
                        plugin.open_modal(FileOperation::Move, files, dest);
                        self.plugin_manager.set_active_modal(Some("fileops"));
                        self.modal = Modal::Plugin("fileops".to_string());
                    }
                }
            }
            NavItem::Erase => {
                if !self.tagged_files.is_empty() {
                    // Erase tagged files
                    let files = self.tagged_files.clone();
                    if let Some(plugin) = self.plugin_manager.fileops_plugin_mut() {
                        plugin.open_modal(FileOperation::Erase, files, String::new());
                        self.plugin_manager.set_active_modal(Some("fileops"));
                        self.modal = Modal::Plugin("fileops".to_string());
                    }
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else if self.files[self.selected_index].is_dir {
                    self.modal = Modal::Error(errors::file::CANNOT_ERASE_DIR.to_string());
                } else {
                    // Erase the highlighted file
                    let file = &self.files[self.selected_index];
                    let files = vec![file.path.clone()];
                    // Temporarily tag for the operation
                    self.tagged_files.push(file.path.clone());
                    if let Some(plugin) = self.plugin_manager.fileops_plugin_mut() {
                        plugin.open_modal(FileOperation::Erase, files, String::new());
                        self.plugin_manager.set_active_modal(Some("fileops"));
                        self.modal = Modal::Plugin("fileops".to_string());
                    }
                }
            }
            NavItem::Rename => {
                if !self.tagged_files.is_empty() {
                    // Batch rename for tagged files (keep old modal for now)
                    let files = self.tagged_files.clone();
                    let state = BatchRenameState::new(files);
                    self.modal = Modal::BatchRename(state);
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else {
                    // Single file rename
                    let file = &self.files[self.selected_index];
                    let current_name = file.name.clone();
                    let ext = &file.extension;
                    let full_name = if ext.is_empty() {
                        current_name
                    } else {
                        format!("{}.{}", current_name, ext.to_lowercase())
                    };
                    let files = vec![file.path.clone()];
                    if let Some(plugin) = self.plugin_manager.fileops_plugin_mut() {
                        plugin.open_modal(FileOperation::Rename, files, full_name);
                        self.plugin_manager.set_active_modal(Some("fileops"));
                        self.modal = Modal::Plugin("fileops".to_string());
                    }
                }
            }
            NavItem::Git => {
                let is_repo = self.is_git_repo();
                self.modal = Modal::Git(GitState::new(is_repo));
            }
            NavItem::Beads => {
                let is_beads = self.is_beads_project();
                self.modal = Modal::Beads(BeadsState::new(is_beads));
            }
            NavItem::View => {
                if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else if self.files[self.selected_index].is_dir {
                    self.modal = Modal::Error(errors::file::CANNOT_VIEW_DIR.to_string());
                } else {
                    let file = &self.files[self.selected_index];
                    match std::fs::read(&file.path) {
                        Ok(content) => {
                            let file_name = if file.extension.is_empty() {
                                file.name.clone()
                            } else {
                                format!("{}.{}", file.name, file.extension)
                            };
                            let file_path = file.path.clone();
                            let mut viewer_state =
                                FileViewerState::new(file_name, file_path.clone(), content);
                            // Load git history if in a git repo
                            let is_repo = git_ops::is_git_repo(&self.current_path);
                            if is_repo {
                                let history =
                                    git_ops::load_file_history(&file_path, &self.current_path);
                                viewer_state.set_git_history(history, true);
                            }
                            self.modal = Modal::FileViewer(viewer_state);
                        }
                        Err(_e) => {
                            self.modal =
                                Modal::Error(errors::file::CANNOT_OPEN_HIGHLIGHTED.to_string());
                        }
                    }
                }
            }
            NavItem::Find => {
                // Open Find dialog
                let state = FindState::new(self.last_find_pattern.clone());
                self.modal = Modal::Find(state);
            }
            NavItem::Attribute => {
                if !self.tagged_files.is_empty() {
                    // For tagged files, show attribute editor with N/C option
                    // We'll use the first tagged file as reference
                    let path = self.tagged_files[0].clone();
                    let state = AttributeState::new(path, true);
                    self.modal = Modal::Attribute(state);
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    // No file selected - show display mode for current file or error
                    self.modal =
                        Modal::Error("No file selected for attribute display.".to_string());
                } else {
                    // Single file - show attribute editor
                    let file = &self.files[self.selected_index];
                    let state = AttributeState::new(file.path.clone(), false);
                    self.modal = Modal::Attribute(state);
                }
            }
            NavItem::Print => {
                // Delegate to Print plugin
                if self.files.is_empty() {
                    if let Some(print_plugin) = self.plugin_manager.print_plugin_mut() {
                        print_plugin.open_modal_error("No file selected".to_string());
                        self.plugin_manager.set_active_modal(Some("print"));
                        self.modal = Modal::Plugin("print".to_string());
                    }
                } else {
                    let file = &self.files[self.selected_index];
                    if file.name == ".." {
                        if let Some(print_plugin) = self.plugin_manager.print_plugin_mut() {
                            print_plugin
                                .open_modal_error("Cannot print parent directory".to_string());
                            self.plugin_manager.set_active_modal(Some("print"));
                            self.modal = Modal::Plugin("print".to_string());
                        }
                    } else if file.is_dir {
                        if let Some(print_plugin) = self.plugin_manager.print_plugin_mut() {
                            print_plugin.open_modal_error("Cannot print a directory".to_string());
                            self.plugin_manager.set_active_modal(Some("print"));
                            self.modal = Modal::Plugin("print".to_string());
                        }
                    } else {
                        let path = file.path.clone();
                        let name = file.name.clone();
                        if let Some(print_plugin) = self.plugin_manager.print_plugin_mut() {
                            print_plugin.open_modal(path, name);
                            self.plugin_manager.set_active_modal(Some("print"));
                            self.modal = Modal::Plugin("print".to_string());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get count of directories in file list
    pub fn dir_count(&self) -> usize {
        self.files.iter().filter(|f| f.is_dir).count()
    }

    /// Get count of regular files in file list
    pub fn file_count(&self) -> usize {
        self.files.iter().filter(|f| !f.is_dir).count()
    }

    /// Get the current theme colors
    pub fn colors(&self) -> ThemeColors {
        self.color_theme.colors()
    }

    /// Get total size of all files
    pub fn total_size(&self) -> u64 {
        self.files
            .iter()
            .filter(|f| !f.is_dir)
            .map(|f| f.size)
            .sum()
    }

    /// Tab completion for paths
    fn tab_complete(partial: &str) -> Option<String> {
        let path = PathBuf::from(partial);

        // Determine the directory to search and the prefix to match
        let (search_dir, prefix) = if partial.ends_with('/') || partial.ends_with('\\') {
            (path.clone(), String::new())
        } else if let Some(parent) = path.parent() {
            let file_name = path.file_name()?.to_string_lossy().to_string();
            (parent.to_path_buf(), file_name)
        } else {
            (PathBuf::from("."), partial.to_string())
        };

        // Read directory and find matches
        let entries = fs::read_dir(&search_dir).ok()?;
        let mut matches: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    let full_path = search_dir.join(&name);
                    let mut result = full_path.to_string_lossy().to_string();
                    // Add trailing slash for directories
                    if full_path.is_dir() && !result.ends_with('/') {
                        result.push('/');
                    }
                    Some(result)
                } else {
                    None
                }
            })
            .collect();

        matches.sort();

        // Return first match, or find common prefix if multiple matches
        if matches.len() == 1 {
            Some(matches.remove(0))
        } else if matches.len() > 1 {
            // Find common prefix among all matches
            let first = &matches[0];
            let mut common_len = first.len();
            for m in &matches[1..] {
                common_len = first
                    .chars()
                    .zip(m.chars())
                    .take_while(|(a, b)| a == b)
                    .count()
                    .min(common_len);
            }
            if common_len > partial.len() {
                Some(first[..common_len].to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Refresh the current file list and status bar
    fn refresh_files(&mut self) -> Result<()> {
        self.files = get_directory_contents(&self.current_path, self.sort_mode)?;
        if self.selected_index >= self.files.len() && !self.files.is_empty() {
            self.selected_index = self.files.len() - 1;
        }
        // Refresh status bar when files change (e.g., after git/beads operations)
        self.refresh_status_bar();
        // Reset auto-refresh timer
        self.last_refresh = std::time::Instant::now();
        Ok(())
    }

    /// Copy tagged files to destination directory
    #[allow(dead_code)] // For future batch operations
    fn copy_tagged_files(&mut self, dest: &PathBuf) -> Result<usize> {
        if self.tagged_files.is_empty() {
            anyhow::bail!("No files tagged");
        }

        let dest_dir = if dest.is_dir() {
            dest.clone()
        } else {
            dest.parent().unwrap_or(dest).to_path_buf()
        };

        if !dest_dir.exists() {
            fs::create_dir_all(&dest_dir)?;
        }

        let mut count = 0;
        for src_path in &self.tagged_files.clone() {
            if let Some(file_name) = src_path.file_name() {
                let dest_path = dest_dir.join(file_name);
                if src_path.is_dir() {
                    copy_dir_recursive(src_path, &dest_path)?;
                } else {
                    fs::copy(src_path, &dest_path)?;
                }
                count += 1;
            }
        }

        self.tagged_files.clear();
        Ok(count)
    }

    /// Move tagged files to destination directory
    #[allow(dead_code)] // For future batch operations
    fn move_tagged_files(&mut self, dest: &PathBuf) -> Result<usize> {
        if self.tagged_files.is_empty() {
            anyhow::bail!("No files tagged");
        }

        let dest_dir = if dest.is_dir() {
            dest.clone()
        } else {
            dest.parent().unwrap_or(dest).to_path_buf()
        };

        if !dest_dir.exists() {
            fs::create_dir_all(&dest_dir)?;
        }

        let mut count = 0;
        for src_path in &self.tagged_files.clone() {
            if let Some(file_name) = src_path.file_name() {
                let dest_path = dest_dir.join(file_name);
                fs::rename(src_path, &dest_path)?;
                count += 1;
            }
        }

        self.tagged_files.clear();
        Ok(count)
    }

    /// Erase (delete) tagged files
    #[allow(dead_code)] // For future batch operations
    fn erase_tagged_files(&mut self) -> Result<usize> {
        if self.tagged_files.is_empty() {
            anyhow::bail!("No files tagged");
        }

        let mut count = 0;
        for path in &self.tagged_files.clone() {
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
            count += 1;
        }

        self.tagged_files.clear();
        Ok(count)
    }

    /// Rename the selected file
    fn rename_selected_file(&mut self, new_name: &str) -> Result<()> {
        if self.files.is_empty() {
            anyhow::bail!("No file selected");
        }

        let file = &self.files[self.selected_index];
        if file.name == ".." {
            anyhow::bail!("Cannot rename parent directory");
        }

        let new_path = file
            .path
            .parent()
            .unwrap_or(&self.current_path)
            .join(new_name);
        fs::rename(&file.path, &new_path)?;

        Ok(())
    }

    /// Get files to operate on (tagged files or selected file)
    #[allow(dead_code)]
    pub fn get_operation_targets(&self) -> Vec<PathBuf> {
        if !self.tagged_files.is_empty() {
            self.tagged_files.clone()
        } else if !self.files.is_empty() && self.files[self.selected_index].name != ".." {
            vec![self.files[self.selected_index].path.clone()]
        } else {
            Vec::new()
        }
    }

    /// Open the selected file in the default editor
    fn edit_selected_file(&mut self) -> Result<()> {
        if self.files.is_empty() {
            self.modal = Modal::Error("No file selected".to_string());
            return Ok(());
        }

        let file = &self.files[self.selected_index];
        if file.name == ".." {
            self.modal = Modal::Error("Cannot edit parent directory".to_string());
            return Ok(());
        }

        if file.is_dir {
            self.modal = Modal::Error("Cannot edit a directory".to_string());
            return Ok(());
        }

        // Use the 'open' command on macOS to open in default editor
        // This works without suspending the TUI since it opens a separate window
        let result = std::process::Command::new("open")
            .arg("-t") // Open in default text editor
            .arg(&file.path)
            .spawn();

        match result {
            Ok(_) => {
                self.modal = Modal::Success(format!("Opening {} in editor...", file.name));
            }
            Err(e) => {
                self.modal = Modal::Error(format!("Failed to open editor: {}", e));
            }
        }

        Ok(())
    }

    // print_selected_file removed - now handled by PrintPlugin

    /// Check if current directory is in a git repository
    pub fn is_git_repo(&self) -> bool {
        // Walk up the directory tree looking for .git
        let mut path = self.current_path.clone();
        loop {
            if path.join(".git").exists() {
                return true;
            }
            if !path.pop() {
                break;
            }
        }
        false
    }

    /// Check if current directory is in a beads-enabled project
    pub fn is_beads_project(&self) -> bool {
        // Walk up the directory tree looking for .beads
        let mut path = self.current_path.clone();
        loop {
            if path.join(".beads").exists() {
                return true;
            }
            if !path.pop() {
                break;
            }
        }
        false
    }

    /// Open clipboard menu with context-appropriate items
    fn open_clipboard_menu(&mut self) {
        use crate::clipboard::copy_to_clipboard;

        let mut items = Vec::new();

        // Check modal context for specific items
        match &self.modal {
            Modal::Beads(state) => {
                // In beads modal - prioritize issue ID
                if let Some(issue) = &state.detail_issue {
                    items.push(ClipboardItem {
                        label: "Issue ID".to_string(),
                        value: issue.id.clone(),
                    });
                    items.push(ClipboardItem {
                        label: "Issue Title".to_string(),
                        value: issue.title.clone(),
                    });
                } else if let Some(issue) = state.issues.get(state.selected_issue) {
                    items.push(ClipboardItem {
                        label: "Issue ID".to_string(),
                        value: issue.id.clone(),
                    });
                    items.push(ClipboardItem {
                        label: "Issue Title".to_string(),
                        value: issue.title.clone(),
                    });
                }
            }
            Modal::Git(state) => {
                // In git modal - prioritize commit hash if in log view
                if !state.log_entries.is_empty() {
                    if let Some(entry) = state.log_entries.get(state.selected_log) {
                        items.push(ClipboardItem {
                            label: "Commit SHA".to_string(),
                            value: entry.hash.clone(),
                        });
                        items.push(ClipboardItem {
                            label: "Commit Message".to_string(),
                            value: entry.message.clone(),
                        });
                    }
                }
                // Add current branch
                if !state.branches.is_empty() {
                    if let Some(branch) = state.branches.get(state.selected_branch) {
                        items.push(ClipboardItem {
                            label: "Branch Name".to_string(),
                            value: branch.name.clone(),
                        });
                    }
                }
            }
            Modal::FileViewer(state) => {
                // In file viewer - add file path and commit info if viewing history
                items.push(ClipboardItem {
                    label: "File Path".to_string(),
                    value: state.file_path.to_string_lossy().to_string(),
                });
                items.push(ClipboardItem {
                    label: "File Name".to_string(),
                    value: state.file_name.clone(),
                });
                if let Some(commit) = state.current_commit() {
                    items.push(ClipboardItem {
                        label: "Viewing Commit".to_string(),
                        value: commit.hash.clone(),
                    });
                }
            }
            _ => {
                // Default: current directory and selected file
                items.push(ClipboardItem {
                    label: "Current Directory".to_string(),
                    value: self.current_path.to_string_lossy().to_string(),
                });

                // Selected file (if any)
                if let Some(entry) = self.files.get(self.selected_index) {
                    if entry.name != ".." {
                        items.push(ClipboardItem {
                            label: "File Path".to_string(),
                            value: entry.path.to_string_lossy().to_string(),
                        });
                        items.push(ClipboardItem {
                            label: "File Name".to_string(),
                            value: entry.name.clone(),
                        });
                    }
                }

                // Git info (if in repo)
                if self.is_git_repo() {
                    if let Some(ref git_info) = self.git_status_info {
                        items.push(ClipboardItem {
                            label: "Git Branch".to_string(),
                            value: git_info.branch.clone(),
                        });
                    }
                    // Add HEAD commit SHA
                    if let Some(sha) = git_ops::get_head_commit_sha(&self.current_path) {
                        items.push(ClipboardItem {
                            label: "HEAD Commit".to_string(),
                            value: sha,
                        });
                    }
                }
            }
        }

        // If only one item, copy immediately
        if items.len() == 1 {
            if let Err(e) = copy_to_clipboard(&items[0].value) {
                self.modal = Modal::Error(e);
            } else {
                self.modal = Modal::Success(format!("Copied: {}", items[0].value));
            }
        } else if items.is_empty() {
            // No items - just copy current directory
            let dir = self.current_path.to_string_lossy().to_string();
            if let Err(e) = copy_to_clipboard(&dir) {
                self.modal = Modal::Error(e);
            } else {
                self.modal = Modal::Success(format!("Copied: {}", dir));
            }
        } else {
            self.modal = Modal::Clipboard(ClipboardState::new(items));
        }
    }

    /// Refresh status bar info (beads and git)
    pub fn refresh_status_bar(&mut self) {
        // Refresh beads status
        if self.is_beads_project() {
            self.beads_status_info = beads_ops::get_beads_status_info(&self.current_path);
        } else {
            self.beads_status_info = None;
        }

        // Refresh git status
        if self.is_git_repo() {
            self.git_status_info = git_ops::get_git_status_info(&self.current_path);
        } else {
            self.git_status_info = None;
        }
    }

    /// Get menu items from all registered plugins
    #[allow(dead_code)] // Will be used by UI in future
    pub fn get_plugin_menu_items(&self) -> Vec<PluginMenuItem> {
        self.plugin_manager
            .menu_plugins()
            .into_iter()
            .map(|(_, item)| item)
            .collect()
    }

    /// Let plugins handle a key event first
    /// Returns true if a plugin handled the key
    pub fn handle_plugin_key(&mut self, key: KeyEvent) -> bool {
        // Check if a plugin modal is active
        if self.plugin_manager.has_active_modal() {
            let result = self
                .plugin_manager
                .handle_modal_key(key, &self.current_path);
            return self.handle_plugin_result(result);
        }

        // Let plugins try to handle global keys
        let result = self
            .plugin_manager
            .handle_global_key(key, &self.current_path);
        self.handle_plugin_result(result)
    }

    /// Handle a KeyHandleResult from a plugin, updating app state accordingly
    fn handle_plugin_result(&mut self, result: KeyHandleResult) -> bool {
        match result {
            KeyHandleResult::NotHandled => false,
            KeyHandleResult::Handled => true,
            KeyHandleResult::OpenModal => {
                // Plugin opened its modal - set the Modal::Plugin variant
                if let Some(plugin) = self.plugin_manager.active_modal() {
                    let plugin_id = plugin.id().to_string();

                    // Pass plugin list to StatusPlugin when it opens
                    if plugin_id == "status" {
                        let plugin_list = self.plugin_manager.plugin_list();
                        if let Some(status_plugin) = self.plugin_manager.status_plugin_mut() {
                            status_plugin.set_plugins(plugin_list);
                        }
                    }

                    // Sync current theme to ThemePlugin when it opens
                    if plugin_id == "theme" {
                        if let Some(theme_plugin) = self.plugin_manager.theme_plugin_mut() {
                            theme_plugin.set_current_theme(self.color_theme);
                            // Re-open with correct theme (since it was opened with default)
                            theme_plugin.open_modal(self.color_theme);
                        }
                    }

                    // Sync current search_spec to SearchSpecPlugin when it opens
                    if plugin_id == "searchspec" {
                        if let Some(searchspec_plugin) = self.plugin_manager.searchspec_plugin_mut()
                        {
                            // Re-open with correct search spec (since it was opened with default)
                            searchspec_plugin.open_modal(&self.search_spec);
                        }
                    }

                    // Sync current settings to QdconfigPlugin when it opens
                    if plugin_id == "qdconfig" {
                        if let Some(qdconfig_plugin) = self.plugin_manager.qdconfig_plugin_mut() {
                            // Re-open with correct settings (since it was opened with defaults)
                            qdconfig_plugin.open_modal(
                                self.search_spec.clone(),
                                self.sort_mode,
                                self.show_hidden,
                                self.config.general.confirm_delete,
                                self.config.editor.command.clone(),
                                self.color_theme,
                                self.config.general.mouse_support,
                                self.config.display.uppercase_names,
                                self.config.general.auto_refresh_interval,
                            );
                        }
                    }

                    self.modal = Modal::Plugin(plugin_id);
                }
                true
            }
            KeyHandleResult::CloseModal => {
                self.plugin_manager.set_active_modal(None);
                self.modal = Modal::None;
                true
            }
            KeyHandleResult::CloseWithSuccess(msg) => {
                self.plugin_manager.set_active_modal(None);
                // Handle theme selection (format: "theme:ThemeName")
                if let Some(theme_name) = msg.strip_prefix("theme:") {
                    if let Some(theme) = ColorTheme::ALL.iter().find(|t| t.name() == theme_name) {
                        self.color_theme = *theme;
                    }
                    self.modal = Modal::None;
                } else if msg == "dirmap:navigate" {
                    // Handle directory map navigation
                    if let Some(dirmap_plugin) = self.plugin_manager.dirmap_plugin_mut() {
                        if let Some(path) = dirmap_plugin.take_navigate_path() {
                            let _ = self.navigate_to(&path);
                        }
                    }
                    self.modal = Modal::None;
                } else if msg == "searchspec:applied" {
                    // Handle search spec update
                    if let Some(searchspec_plugin) = self.plugin_manager.searchspec_plugin_mut() {
                        if let Some(pattern) = searchspec_plugin.take_result() {
                            self.search_spec = pattern;
                            let _ = self.refresh_files();
                        }
                    }
                    self.modal = Modal::None;
                } else if msg == "qdconfig:saved" {
                    // Handle configuration save
                    if let Some(qdconfig_plugin) = self.plugin_manager.qdconfig_plugin_mut() {
                        if let Some(state) = qdconfig_plugin.take_result() {
                            // Apply settings to app state
                            self.search_spec = state.search_spec.clone();
                            self.show_hidden = state.show_hidden;
                            self.color_theme = state.theme();
                            self.sort_mode = state.sort_mode();

                            // Update config
                            self.config.general.search_spec = state.search_spec.clone();
                            self.config.general.show_hidden = state.show_hidden;
                            self.config.general.confirm_delete = state.confirm_delete;
                            self.config.general.mouse_support = state.mouse_support;
                            self.config.general.auto_refresh_interval = state.auto_refresh_interval;
                            self.config.display.uppercase_names = state.uppercase_names;
                            self.config.display.theme = state.theme().into();
                            self.config.editor.command = state.editor.clone();
                            self.config.from_sort_mode(self.sort_mode);

                            // Save config to disk
                            if let Err(e) = self.save_config() {
                                self.modal = Modal::Error(format!("Failed to save config: {}", e));
                                return true;
                            }

                            let _ = self.refresh_files();
                            self.modal =
                                Modal::Success("Configuration saved successfully".to_string());
                        }
                    } else {
                        self.modal = Modal::None;
                    }
                } else if msg.starts_with("fileops:") {
                    // Handle file operations
                    if let Some(fileops_plugin) = self.plugin_manager.fileops_plugin_mut() {
                        if let Some(result) = fileops_plugin.take_result() {
                            match result.operation {
                                FileOperation::Copy => {
                                    if let Some(dest_str) = result.destination {
                                        let dest = PathBuf::from(dest_str);
                                        let dest_dir = if dest.is_dir() {
                                            dest.clone()
                                        } else {
                                            dest.parent().unwrap_or(&dest).to_path_buf()
                                        };
                                        if !dest_dir.exists() {
                                            if let Err(e) = fs::create_dir_all(&dest_dir) {
                                                self.modal = Modal::Error(format!(
                                                    "Failed to create directory: {}",
                                                    e
                                                ));
                                                return true;
                                            }
                                        }
                                        let mut count = 0;
                                        for src_path in &result.files {
                                            if let Some(file_name) = src_path.file_name() {
                                                let dest_path = dest_dir.join(file_name);
                                                let copy_result: Result<()> = if src_path.is_dir() {
                                                    copy_dir_recursive(src_path, &dest_path)
                                                } else {
                                                    fs::copy(src_path, &dest_path)
                                                        .map(|_| ())
                                                        .map_err(|e| e.into())
                                                };
                                                if let Err(e) = copy_result {
                                                    self.modal =
                                                        Modal::Error(format!("Copy failed: {}", e));
                                                    self.tagged_files.clear();
                                                    let _ = self.refresh_files();
                                                    return true;
                                                }
                                                count += 1;
                                            }
                                        }
                                        self.tagged_files.clear();
                                        let _ = self.refresh_files();
                                        self.modal =
                                            Modal::Success(format!("Copied {} file(s)", count));
                                    }
                                }
                                FileOperation::Move => {
                                    if let Some(dest_str) = result.destination {
                                        let dest = PathBuf::from(dest_str);
                                        let dest_dir = if dest.is_dir() {
                                            dest.clone()
                                        } else {
                                            dest.parent().unwrap_or(&dest).to_path_buf()
                                        };
                                        if !dest_dir.exists() {
                                            if let Err(e) = fs::create_dir_all(&dest_dir) {
                                                self.modal = Modal::Error(format!(
                                                    "Failed to create directory: {}",
                                                    e
                                                ));
                                                return true;
                                            }
                                        }
                                        let mut count = 0;
                                        for src_path in &result.files {
                                            if let Some(file_name) = src_path.file_name() {
                                                let dest_path = dest_dir.join(file_name);
                                                if let Err(e) = fs::rename(src_path, &dest_path) {
                                                    self.modal =
                                                        Modal::Error(format!("Move failed: {}", e));
                                                    self.tagged_files.clear();
                                                    let _ = self.refresh_files();
                                                    return true;
                                                }
                                                count += 1;
                                            }
                                        }
                                        self.tagged_files.clear();
                                        let _ = self.refresh_files();
                                        self.modal =
                                            Modal::Success(format!("Moved {} file(s)", count));
                                    }
                                }
                                FileOperation::Erase => {
                                    let mut count = 0;
                                    for path in &result.files {
                                        let remove_result = if path.is_dir() {
                                            fs::remove_dir_all(path)
                                        } else {
                                            fs::remove_file(path)
                                        };
                                        if let Err(e) = remove_result {
                                            self.modal =
                                                Modal::Error(format!("Erase failed: {}", e));
                                            self.tagged_files.clear();
                                            let _ = self.refresh_files();
                                            return true;
                                        }
                                        count += 1;
                                    }
                                    self.tagged_files.clear();
                                    let _ = self.refresh_files();
                                    self.modal =
                                        Modal::Success(format!("Erased {} file(s)", count));
                                }
                                FileOperation::Rename => {
                                    if let Some(new_name) = result.destination {
                                        if let Some(old_path) = result.files.first() {
                                            let new_path = old_path
                                                .parent()
                                                .unwrap_or(&self.current_path)
                                                .join(&new_name);
                                            if let Err(e) = fs::rename(old_path, &new_path) {
                                                self.modal =
                                                    Modal::Error(format!("Rename failed: {}", e));
                                                let _ = self.refresh_files();
                                                return true;
                                            }
                                            let _ = self.refresh_files();
                                            self.modal = Modal::Success(format!(
                                                "Renamed to \"{}\"",
                                                new_name
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        self.modal = Modal::None;
                    }
                } else {
                    self.modal = Modal::Success(msg);
                }
                true
            }
            KeyHandleResult::CloseWithError(msg) => {
                self.plugin_manager.set_active_modal(None);
                // Handle theme cancel (format: "theme:ThemeName" - restore original)
                if let Some(theme_name) = msg.strip_prefix("theme:") {
                    if let Some(theme) = ColorTheme::ALL.iter().find(|t| t.name() == theme_name) {
                        self.color_theme = *theme;
                    }
                    self.modal = Modal::None;
                } else {
                    self.modal = Modal::Error(msg);
                }
                true
            }
            KeyHandleResult::RefreshFiles => {
                let _ = self.refresh_files();
                true
            }
        }
    }

    /// Get status bar info from all active plugins
    #[allow(dead_code)] // Will be used by UI when plugins are implemented
    pub fn get_plugin_status_info(&self) -> Vec<(String, PluginStatusInfo)> {
        self.plugin_manager
            .status_plugins(&self.current_path)
            .into_iter()
            .map(|(plugin, info)| (plugin.id().to_string(), info))
            .collect()
    }
}

/// Recursively copy a directory
#[allow(dead_code)] // Used by copy_tagged_files
fn copy_dir_recursive(src: &PathBuf, dest: &PathBuf) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

/// Execute a shell command and return (output lines, exit code)
fn execute_shell_command_impl(cmd: &str, cwd: &PathBuf) -> (Vec<String>, i32) {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    let result = Command::new(&shell)
        .arg("-c")
        .arg(cmd)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match result {
        Ok(output) => {
            let mut lines = Vec::new();

            // Add stdout lines
            let stdout_reader = BufReader::new(&output.stdout[..]);
            for l in stdout_reader.lines().flatten() {
                lines.push(l);
            }

            // Add stderr lines
            let stderr_reader = BufReader::new(&output.stderr[..]);
            for l in stderr_reader.lines().flatten() {
                lines.push(format!("stderr: {}", l));
            }

            let exit_code = output.status.code().unwrap_or(-1);
            (lines, exit_code)
        }
        Err(e) => (vec![format!("Error executing command: {}", e)], -1),
    }
}
