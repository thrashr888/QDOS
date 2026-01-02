mod beads_ops;
mod git_ops;
mod state;

// Re-export state types for external use
pub use state::{
    AttrValue, AttributeState, BatchRenameState, BeadsMenuItem, BeadsState, BeadsView, ColorTheme,
    ColorThemeState, DirectoryMapState, FileViewerState, FindPhase, FindState, GitMenuItem,
    GitState, GitView, HelpState, Modal, NavItem, ProgressOperation, ProgressState, QdstartField,
    QdstartState, SearchSpecState, ShellCommandState, SortMode, ThemeColors, ViewFilter, ViewMode,
};
// Internal types used by git_ops and beads_ops modules:
// BeadsIssue, BeadsStats, GitFileStatus, GitLogEntry

use crate::config::Config;
use crate::errors;
use crate::event::EventHandler;
use crate::file_ops::{
    apply_attributes, find_files_recursive, get_directory_contents, get_system_info, FileEntry,
};
use crate::ui;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use std::fs;
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
    /// Current active modal
    pub modal: Modal,
    /// Scroll offset for file list
    pub scroll_offset: usize,
    /// Should the app quit
    pub should_quit: bool,
    /// Search/filter specification
    pub search_spec: String,
    /// Navigation history
    pub history: Vec<PathBuf>,
    /// Last find pattern (for Ctrl+R recall)
    pub last_find_pattern: String,
    /// Current color theme
    pub color_theme: ColorTheme,
    /// Application configuration
    pub config: Config,
    /// Show hidden files
    pub show_hidden: bool,
    /// Beads status bar info: (open_count, ready_count) - None if not in beads project
    pub beads_status: Option<(usize, usize)>,
    /// Git status bar info: (modified_count, staged_count) - None if not in git repo
    pub git_status_counts: Option<(usize, usize)>,
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

        let current_path = PathBuf::from(start_path).canonicalize()?;
        let files = get_directory_contents(&current_path, sort_mode)?;

        let mut app = Self {
            current_path,
            files,
            selected_index: 0,
            tagged_files: Vec::new(),
            sort_mode,
            nav_index: 0,
            modal: Modal::None,
            scroll_offset: 0,
            should_quit: false,
            search_spec,
            history: Vec::new(),
            last_find_pattern: String::new(),
            color_theme,
            config,
            show_hidden,
            beads_status: None,
            git_status_counts: None,
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
            // Draw UI
            terminal.draw(|frame| ui::draw(frame, self))?;

            // Handle events
            if let Some(event) = event_handler.next().await? {
                match event {
                    crate::event::Event::Key(key) => self.handle_key(key)?,
                    crate::event::Event::Tick => {
                        // Auto-process progress on tick
                        if matches!(self.modal, Modal::Progress(_)) {
                            self.process_next_progress_file();
                        }
                    }
                    crate::event::Event::Resize(_, _) => {}
                }
            }

            if self.should_quit {
                break;
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

        // Color theme shortcut (Ctrl+T)
        if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.modal = Modal::ColorTheme(ColorThemeState::new(self.color_theme));
            return Ok(());
        }

        // QDSTART configuration shortcut (Ctrl+S)
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.modal = Modal::Qdstart(self.create_qdstart_state());
            return Ok(());
        }

        // Handle modal-specific input
        if !matches!(self.modal, Modal::None) {
            return self.handle_modal_key(key);
        }

        match key.code {
            // Quit (also handles 'q' which is not a global shortcut)
            KeyCode::Char('q') => {
                self.modal = Modal::Quit;
            }
            // Git menu (G key)
            KeyCode::Char('g') | KeyCode::Char('G') => {
                let is_repo = self.is_git_repo();
                self.modal = Modal::Git(GitState::new(is_repo));
            }
            // Beads menu (B key)
            KeyCode::Char('b') | KeyCode::Char('B') => {
                let is_beads = self.is_beads_project();
                self.modal = Modal::Beads(BeadsState::new(is_beads));
            }
            // Help
            KeyCode::F(1) => {
                self.modal = Modal::Help(HelpState::new());
            }
            // Status
            KeyCode::F(2) => {
                let info = get_system_info()?;
                self.modal = Modal::Status(info);
            }
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
            // Search spec
            KeyCode::F(7) => {
                let state = SearchSpecState::new(&self.search_spec);
                self.modal = Modal::SearchSpec(state);
            }
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
            _ => {}
        }

        Ok(())
    }

    /// Handle keyboard input in modal dialogs
    fn handle_modal_key(&mut self, key: KeyEvent) -> Result<()> {
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
                                    GitMenuItem::Push => match git_ops::execute_git_push(&path) {
                                        Ok(msg) => {
                                            self.modal = Modal::Success(msg);
                                        }
                                        Err(e) => {
                                            state.error = Some(e);
                                        }
                                    },
                                    GitMenuItem::Pull => match git_ops::execute_git_pull(&path) {
                                        Ok(msg) => {
                                            self.modal = Modal::Success(msg);
                                        }
                                        Err(e) => {
                                            state.error = Some(e);
                                        }
                                    },
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
                                let path = self.current_path.clone();
                                match git_ops::execute_git_push(&path) {
                                    Ok(msg) => {
                                        self.modal = Modal::Success(msg);
                                    }
                                    Err(e) => {
                                        state.error = Some(e);
                                    }
                                }
                            }
                            KeyCode::Char('u') | KeyCode::Char('U') => {
                                let path = self.current_path.clone();
                                match git_ops::execute_git_pull(&path) {
                                    Ok(msg) => {
                                        self.modal = Modal::Success(msg);
                                    }
                                    Err(e) => {
                                        state.error = Some(e);
                                    }
                                }
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
                    }
                }
            }
            Modal::Beads(ref mut state) => {
                if !state.is_beads_project {
                    match key.code {
                        KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                            self.modal = Modal::None;
                        }
                        _ => {}
                    }
                } else {
                    match state.view {
                        BeadsView::Menu => match key.code {
                            KeyCode::Esc => {
                                self.modal = Modal::None;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.menu_selected > 0 {
                                    state.menu_selected -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                let items = BeadsMenuItem::items(state.is_beads_project);
                                if state.menu_selected < items.len() - 1 {
                                    state.menu_selected += 1;
                                }
                            }
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                let items = BeadsMenuItem::items(state.is_beads_project);
                                let item = items[state.menu_selected];
                                let path = self.current_path.clone();
                                match item {
                                    BeadsMenuItem::List => {
                                        state.view = BeadsView::List;
                                        beads_ops::load_beads_list(state, &path, None);
                                    }
                                    BeadsMenuItem::Ready => {
                                        state.view = BeadsView::Ready;
                                        beads_ops::load_beads_ready(state, &path);
                                    }
                                    BeadsMenuItem::Blocked => {
                                        state.view = BeadsView::Blocked;
                                        beads_ops::load_beads_blocked(state, &path);
                                    }
                                    BeadsMenuItem::Stats => {
                                        state.view = BeadsView::Stats;
                                        beads_ops::load_beads_stats(state, &path);
                                    }
                                    BeadsMenuItem::Create => {
                                        state.view = BeadsView::Create;
                                        state.create_title.clear();
                                        state.create_type = 0;
                                        state.create_priority = 2;
                                        state.create_field = 0;
                                    }
                                    BeadsMenuItem::Sync => {
                                        let path = self.current_path.clone();
                                        match beads_ops::execute_beads_sync(&path) {
                                            Ok(msg) => {
                                                state.success_message = Some(msg);
                                            }
                                            Err(e) => {
                                                state.error = Some(e);
                                            }
                                        }
                                    }
                                    BeadsMenuItem::Human => {
                                        let path = self.current_path.clone();
                                        match beads_ops::execute_beads_human(&path) {
                                            Ok(lines) => {
                                                state.output_lines = lines;
                                                state.scroll_offset = 0;
                                                state.view = BeadsView::Human;
                                            }
                                            Err(e) => {
                                                state.error = Some(e);
                                            }
                                        }
                                    }
                                    BeadsMenuItem::Init => {
                                        let path = self.current_path.clone();
                                        match beads_ops::execute_beads_init(&path) {
                                            Ok(msg) => {
                                                state.success_message = Some(msg);
                                                state.is_beads_project = true;
                                                state.menu_selected = 0;
                                            }
                                            Err(e) => {
                                                state.error = Some(e);
                                            }
                                        }
                                    }
                                    BeadsMenuItem::Doctor => {
                                        let path = self.current_path.clone();
                                        match beads_ops::execute_beads_doctor(&path) {
                                            Ok(lines) => {
                                                state.output_lines = lines;
                                                state.scroll_offset = 0;
                                                state.view = BeadsView::Doctor;
                                            }
                                            Err(e) => {
                                                state.error = Some(e);
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        },
                        BeadsView::List | BeadsView::Ready | BeadsView::Blocked => {
                            let current_view = state.view;
                            match key.code {
                                KeyCode::Esc => {
                                    state.view = BeadsView::Menu;
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if state.selected_issue > 0 {
                                        state.selected_issue -= 1;
                                    }
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if state.selected_issue + 1 < state.issues.len() {
                                        state.selected_issue += 1;
                                    }
                                }
                                KeyCode::Enter => {
                                    // Load detailed issue info before showing detail view
                                    if !state.issues.is_empty() {
                                        let issue_id =
                                            state.issues[state.selected_issue].id.clone();
                                        let path = self.current_path.clone();
                                        match beads_ops::load_beads_issue_detail(&issue_id, &path) {
                                            Ok(issue) => {
                                                state.detail_issue = Some(issue);
                                                state.selected_subtask = 0;
                                            }
                                            Err(e) => {
                                                state.error = Some(e);
                                            }
                                        }
                                    }
                                    state.view = BeadsView::Detail;
                                }
                                KeyCode::Char('r') | KeyCode::Char('R') => {
                                    let path = self.current_path.clone();
                                    match current_view {
                                        BeadsView::List => {
                                            beads_ops::load_beads_list(state, &path, None)
                                        }
                                        BeadsView::Ready => {
                                            beads_ops::load_beads_ready(state, &path)
                                        }
                                        BeadsView::Blocked => {
                                            beads_ops::load_beads_blocked(state, &path)
                                        }
                                        _ => {}
                                    }
                                }
                                KeyCode::Char('c') | KeyCode::Char('C') => {
                                    // Close the selected issue
                                    if !state.issues.is_empty() {
                                        let issue_id =
                                            state.issues[state.selected_issue].id.clone();
                                        let path = self.current_path.clone();
                                        match beads_ops::execute_beads_close(&issue_id, &path) {
                                            Ok(msg) => {
                                                self.modal = Modal::Success(msg);
                                            }
                                            Err(e) => {
                                                state.error = Some(e);
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('s') | KeyCode::Char('S') => {
                                    // Start working on the selected issue (set to in_progress)
                                    if !state.issues.is_empty() {
                                        let issue_id =
                                            state.issues[state.selected_issue].id.clone();
                                        let path = self.current_path.clone();
                                        match beads_ops::execute_beads_update_status(
                                            &issue_id,
                                            "in_progress",
                                            &path,
                                        ) {
                                            Ok(msg) => {
                                                // Refresh the list
                                                match current_view {
                                                    BeadsView::List => beads_ops::load_beads_list(
                                                        state, &path, None,
                                                    ),
                                                    BeadsView::Ready => {
                                                        beads_ops::load_beads_ready(state, &path)
                                                    }
                                                    BeadsView::Blocked => {
                                                        beads_ops::load_beads_blocked(state, &path)
                                                    }
                                                    _ => {}
                                                }
                                                state.error = Some(msg);
                                            }
                                            Err(e) => {
                                                state.error = Some(e);
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        BeadsView::Stats => match key.code {
                            KeyCode::Esc => {
                                state.view = BeadsView::Menu;
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                let path = self.current_path.clone();
                                beads_ops::load_beads_stats(state, &path);
                            }
                            _ => {}
                        },
                        BeadsView::Create => match key.code {
                            KeyCode::Esc => {
                                state.view = BeadsView::Menu;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.create_field > 0 {
                                    state.create_field -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if state.create_field < 2 {
                                    state.create_field += 1;
                                }
                            }
                            KeyCode::Left | KeyCode::Char('h') => match state.create_field {
                                1 => {
                                    if state.create_type > 0 {
                                        state.create_type -= 1;
                                    }
                                }
                                2 => {
                                    if state.create_priority > 0 {
                                        state.create_priority -= 1;
                                    }
                                }
                                _ => {}
                            },
                            KeyCode::Right | KeyCode::Char('l') => match state.create_field {
                                1 => {
                                    if state.create_type < 2 {
                                        state.create_type += 1;
                                    }
                                }
                                2 => {
                                    if state.create_priority < 4 {
                                        state.create_priority += 1;
                                    }
                                }
                                _ => {}
                            },
                            KeyCode::Backspace => {
                                if state.create_field == 0 {
                                    state.create_title.pop();
                                }
                            }
                            KeyCode::Char(c) => {
                                if state.create_field == 0 {
                                    state.create_title.push(c);
                                }
                            }
                            KeyCode::Enter => {
                                if !state.create_title.is_empty() {
                                    let title = state.create_title.clone();
                                    let issue_type = state.create_type;
                                    let priority = state.create_priority;
                                    let path = self.current_path.clone();
                                    match beads_ops::execute_beads_create(
                                        &title, issue_type, priority, &path,
                                    ) {
                                        Ok(_) => {
                                            self.modal =
                                                Modal::Success("Issue created".to_string());
                                        }
                                        Err(e) => {
                                            state.error = Some(e);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        },
                        BeadsView::Detail => match key.code {
                            KeyCode::Esc => {
                                state.detail_issue = None;
                                state.view = BeadsView::List;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                // Navigate subtasks for epics
                                if state.selected_subtask > 0 {
                                    state.selected_subtask -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                // Navigate subtasks for epics
                                if let Some(ref issue) = state.detail_issue {
                                    if state.selected_subtask + 1 < issue.dependents.len() {
                                        state.selected_subtask += 1;
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                // Open selected subtask
                                if let Some(ref issue) = state.detail_issue {
                                    if !issue.dependents.is_empty() {
                                        let subtask_id =
                                            issue.dependents[state.selected_subtask].id.clone();
                                        let path = self.current_path.clone();
                                        match beads_ops::load_beads_issue_detail(&subtask_id, &path)
                                        {
                                            Ok(new_issue) => {
                                                state.detail_issue = Some(new_issue);
                                                state.selected_subtask = 0;
                                            }
                                            Err(e) => {
                                                state.error = Some(e);
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                // Start working on issue
                                if let Some(issue) = state.issues.get(state.selected_issue) {
                                    let issue_id = issue.id.clone();
                                    match beads_ops::execute_beads_update_status(
                                        &issue_id,
                                        "in_progress",
                                        &self.current_path,
                                    ) {
                                        Ok(msg) => {
                                            state.success_message = Some(msg);
                                            // Refresh the issue list
                                            beads_ops::load_beads_list(
                                                state,
                                                &self.current_path,
                                                None,
                                            );
                                        }
                                        Err(e) => {
                                            state.error = Some(e);
                                        }
                                    }
                                    state.view = BeadsView::List;
                                }
                            }
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                // Close issue
                                if let Some(issue) = state.issues.get(state.selected_issue) {
                                    let issue_id = issue.id.clone();
                                    match beads_ops::execute_beads_close(
                                        &issue_id,
                                        &self.current_path,
                                    ) {
                                        Ok(msg) => {
                                            state.success_message = Some(msg);
                                            beads_ops::load_beads_list(
                                                state,
                                                &self.current_path,
                                                None,
                                            );
                                        }
                                        Err(e) => {
                                            state.error = Some(e);
                                        }
                                    }
                                    state.view = BeadsView::List;
                                }
                            }
                            KeyCode::Char('o') | KeyCode::Char('O') => {
                                // Reopen issue
                                if let Some(issue) = state.issues.get(state.selected_issue) {
                                    let issue_id = issue.id.clone();
                                    match beads_ops::execute_beads_reopen(
                                        &issue_id,
                                        &self.current_path,
                                    ) {
                                        Ok(msg) => {
                                            state.success_message = Some(msg);
                                            beads_ops::load_beads_list(
                                                state,
                                                &self.current_path,
                                                None,
                                            );
                                        }
                                        Err(e) => {
                                            state.error = Some(e);
                                        }
                                    }
                                    state.view = BeadsView::List;
                                }
                            }
                            _ => {}
                        },
                        BeadsView::Human | BeadsView::Doctor => match key.code {
                            KeyCode::Esc => {
                                state.view = BeadsView::Menu;
                                state.output_lines.clear();
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.scroll_offset > 0 {
                                    state.scroll_offset -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if state.scroll_offset + 1 < state.output_lines.len() {
                                    state.scroll_offset += 1;
                                }
                            }
                            KeyCode::PageUp => {
                                state.scroll_offset = state.scroll_offset.saturating_sub(10);
                            }
                            KeyCode::PageDown => {
                                let max = state.output_lines.len().saturating_sub(1);
                                state.scroll_offset = (state.scroll_offset + 10).min(max);
                            }
                            _ => {}
                        },
                    }
                }
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

        // Save current path to history
        self.history.push(self.current_path.clone());

        // Update state
        self.current_path = canonical;
        self.files = get_directory_contents(&self.current_path, self.sort_mode)?;
        self.selected_index = 0;
        self.scroll_offset = 0;

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
                // Open Directory Map tree view
                let state = DirectoryMapState::new(&self.current_path);
                self.modal = Modal::DirectoryMap(state);
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
                    self.modal = Modal::CopyTo(dest);
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else {
                    // Copy the highlighted file - temporarily tag it
                    let file = &self.files[self.selected_index];
                    self.tagged_files.push(file.path.clone());
                    let dest = self.current_path.to_string_lossy().to_string();
                    self.modal = Modal::CopyTo(dest);
                }
            }
            NavItem::Move => {
                if !self.tagged_files.is_empty() {
                    // Move tagged files
                    let dest = self.current_path.to_string_lossy().to_string();
                    self.modal = Modal::MoveTo(dest);
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else {
                    // Move the highlighted file - temporarily tag it
                    let file = &self.files[self.selected_index];
                    self.tagged_files.push(file.path.clone());
                    let dest = self.current_path.to_string_lossy().to_string();
                    self.modal = Modal::MoveTo(dest);
                }
            }
            NavItem::Erase => {
                if !self.tagged_files.is_empty() {
                    // Erase tagged files
                    self.modal = Modal::EraseConfirm;
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else if self.files[self.selected_index].is_dir {
                    self.modal = Modal::Error(errors::file::CANNOT_ERASE_DIR.to_string());
                } else {
                    // Erase the highlighted file - temporarily tag it for the confirm dialog
                    let file = &self.files[self.selected_index];
                    self.tagged_files.push(file.path.clone());
                    self.modal = Modal::EraseConfirm;
                }
            }
            NavItem::Rename => {
                if !self.tagged_files.is_empty() {
                    // Batch rename for tagged files
                    let files = self.tagged_files.clone();
                    let state = BatchRenameState::new(files);
                    self.modal = Modal::BatchRename(state);
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else {
                    // Single file rename
                    let current_name = self.files[self.selected_index].name.clone();
                    let ext = &self.files[self.selected_index].extension;
                    let full_name = if ext.is_empty() {
                        current_name
                    } else {
                        format!("{}.{}", current_name, ext.to_lowercase())
                    };
                    self.modal = Modal::RenameInput(full_name);
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
                self.print_selected_file()?;
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

    /// Print the selected file
    fn print_selected_file(&mut self) -> Result<()> {
        if self.files.is_empty() {
            self.modal = Modal::Error("No file selected".to_string());
            return Ok(());
        }

        let file = &self.files[self.selected_index];
        if file.name == ".." {
            self.modal = Modal::Error("Cannot print parent directory".to_string());
            return Ok(());
        }

        if file.is_dir {
            self.modal = Modal::Error("Cannot print a directory".to_string());
            return Ok(());
        }

        // Use the 'lpr' command to print the file
        let result = std::process::Command::new("lpr").arg(&file.path).spawn();

        match result {
            Ok(_) => {
                self.modal = Modal::Success(format!("Sent {} to printer", file.name));
            }
            Err(e) => {
                self.modal = Modal::Error(format!("Failed to print: {}", e));
            }
        }

        Ok(())
    }

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

    /// Refresh status bar info (beads and git)
    pub fn refresh_status_bar(&mut self) {
        // Refresh beads status
        if self.is_beads_project() {
            self.beads_status = beads_ops::get_beads_quick_counts(&self.current_path);
        } else {
            self.beads_status = None;
        }

        // Refresh git status
        if self.is_git_repo() {
            self.git_status_counts = git_ops::get_git_quick_counts(&self.current_path);
        } else {
            self.git_status_counts = None;
        }
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
