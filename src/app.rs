use crate::event::EventHandler;
use crate::file_ops::{FileEntry, get_directory_contents, get_system_info, SystemInfo};
use crate::ui;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use std::fs;
use std::path::PathBuf;

/// Navigation menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavItem {
    Directory,
    Tag,
    View,
    Copy,
    Move,
    Find,
    Erase,
    Rename,
    Space,
    Attribute,
    Print,
}

impl NavItem {
    pub const ALL: [NavItem; 11] = [
        NavItem::Directory,
        NavItem::Tag,
        NavItem::View,
        NavItem::Copy,
        NavItem::Move,
        NavItem::Find,
        NavItem::Erase,
        NavItem::Rename,
        NavItem::Space,
        NavItem::Attribute,
        NavItem::Print,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            NavItem::Directory => "Directory",
            NavItem::Tag => "Tag",
            NavItem::View => "View",
            NavItem::Copy => "Copy",
            NavItem::Move => "Move",
            NavItem::Find => "Find",
            NavItem::Erase => "Erase",
            NavItem::Rename => "Rename",
            NavItem::Space => "Space",
            NavItem::Attribute => "Attribute",
            NavItem::Print => "Print",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            NavItem::Directory => "Change current directory, make or remove directory, see directory tree",
            NavItem::Tag => "Tag groups of files, or clear all tags -- SPACE BAR tags highlighted file",
            NavItem::View => "View the contents of any file on the screen (in \"ASCII\" or \"HEX\")",
            NavItem::Copy => "Copy one or several files to another disk or directory",
            NavItem::Move => "Move one or several files from this directory to another directory",
            NavItem::Find => "Search all directories on the disk to find specified file(s)",
            NavItem::Erase => "Erase one or several files from this directory",
            NavItem::Rename => "Rename one or several files in this directory",
            NavItem::Space => "Show the total, used, and free space on any disk",
            NavItem::Attribute => "Change/view file attributes",
            NavItem::Print => "Print one or several files on the printer",
        }
    }
}

/// Sort modes for file listing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    NameAsc,
    NameDesc,
    ExtAsc,
    ExtDesc,
    SizeAsc,
    SizeDesc,
    DateAsc,
    DateDesc,
    None,
}

impl SortMode {
    pub fn next(&self) -> SortMode {
        match self {
            SortMode::NameAsc => SortMode::NameDesc,
            SortMode::NameDesc => SortMode::ExtAsc,
            SortMode::ExtAsc => SortMode::ExtDesc,
            SortMode::ExtDesc => SortMode::SizeAsc,
            SortMode::SizeAsc => SortMode::SizeDesc,
            SortMode::SizeDesc => SortMode::DateAsc,
            SortMode::DateAsc => SortMode::DateDesc,
            SortMode::DateDesc => SortMode::None,
            SortMode::None => SortMode::NameAsc,
        }
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            SortMode::NameAsc => "Name ↑",
            SortMode::NameDesc => "Name ↓",
            SortMode::ExtAsc => "Ext ↑",
            SortMode::ExtDesc => "Ext ↓",
            SortMode::SizeAsc => "Size ↑",
            SortMode::SizeDesc => "Size ↓",
            SortMode::DateAsc => "Date ↑",
            SortMode::DateDesc => "Date ↓",
            SortMode::None => "None",
        }
    }
}

/// Active modal dialog
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    None,
    Help,
    Status(SystemInfo),
    Quit,
    SearchSpec,
    Space,
    Error(String),
    Success(String),
    PathInput(String),
    CopyTo(String),
    MoveTo(String),
    EraseConfirm,
    RenameInput(String),
}

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
}

impl App {
    pub fn new(start_path: &str) -> Result<Self> {
        let current_path = PathBuf::from(start_path).canonicalize()?;
        let files = get_directory_contents(&current_path, SortMode::NameAsc)?;

        Ok(Self {
            current_path,
            files,
            selected_index: 0,
            tagged_files: Vec::new(),
            sort_mode: SortMode::NameAsc,
            nav_index: 0,
            modal: Modal::None,
            scroll_offset: 0,
            should_quit: false,
            search_spec: "*.*".to_string(),
            history: Vec::new(),
        })
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
                    crate::event::Event::Tick => {}
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
        // Handle modal-specific input first
        if self.modal != Modal::None {
            return self.handle_modal_key(key);
        }

        match key.code {
            // Quit
            KeyCode::F(10) | KeyCode::Char('q') => {
                self.modal = Modal::Quit;
            }
            // Help
            KeyCode::F(1) => {
                self.modal = Modal::Help;
            }
            // Status
            KeyCode::F(2) => {
                let info = get_system_info()?;
                self.modal = Modal::Status(info);
            }
            // Change drive (not applicable on Unix, show error)
            KeyCode::F(3) => {
                self.modal = Modal::Error("Drive selection not available on this platform".to_string());
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
            // DOS Command (not implemented)
            KeyCode::F(6) => {
                self.modal = Modal::Error("DOS Command not implemented".to_string());
            }
            // Search spec
            KeyCode::F(7) => {
                self.modal = Modal::SearchSpec;
            }
            // Sort
            KeyCode::F(8) => {
                self.cycle_sort_mode()?;
            }
            // Edit (not implemented)
            KeyCode::F(9) => {
                self.modal = Modal::Error("Edit not implemented".to_string());
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
            // Ctrl+C to quit
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
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
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                        self.should_quit = true;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                }
            }
            Modal::PathInput(ref mut path) => {
                match key.code {
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
                }
            }
            Modal::CopyTo(ref mut dest) => {
                match key.code {
                    KeyCode::Enter => {
                        let dest_path = PathBuf::from(dest.clone());
                        self.modal = Modal::None;
                        match self.copy_tagged_files(&dest_path) {
                            Ok(count) => {
                                self.modal = Modal::Success(format!("Copied {} file(s)", count));
                            }
                            Err(e) => {
                                self.modal = Modal::Error(format!("Copy failed: {}", e));
                            }
                        }
                    }
                    KeyCode::Esc => {
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
            Modal::MoveTo(ref mut dest) => {
                match key.code {
                    KeyCode::Enter => {
                        let dest_path = PathBuf::from(dest.clone());
                        self.modal = Modal::None;
                        match self.move_tagged_files(&dest_path) {
                            Ok(count) => {
                                self.modal = Modal::Success(format!("Moved {} file(s)", count));
                                // Refresh the file list
                                let _ = self.refresh_files();
                            }
                            Err(e) => {
                                self.modal = Modal::Error(format!("Move failed: {}", e));
                            }
                        }
                    }
                    KeyCode::Esc => {
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
            Modal::EraseConfirm => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.modal = Modal::None;
                        match self.erase_tagged_files() {
                            Ok(count) => {
                                self.modal = Modal::Success(format!("Erased {} file(s)", count));
                                let _ = self.refresh_files();
                            }
                            Err(e) => {
                                self.modal = Modal::Error(format!("Erase failed: {}", e));
                            }
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                }
            }
            Modal::RenameInput(ref mut new_name) => {
                match key.code {
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
                }
            }
            Modal::Help | Modal::Status(_) | Modal::SearchSpec | Modal::Space
            | Modal::Error(_) | Modal::Success(_) => {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                }
            }
            Modal::None => {}
        }

        Ok(())
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

    /// Check if a file is tagged
    pub fn is_tagged(&self, path: &PathBuf) -> bool {
        self.tagged_files.contains(path)
    }

    /// Get total size of tagged files
    pub fn tagged_size(&self) -> u64 {
        self.tagged_files
            .iter()
            .filter_map(|p| {
                self.files.iter().find(|f| &f.path == p).map(|f| f.size)
            })
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
                let path = self.current_path.to_string_lossy().to_string();
                self.modal = Modal::PathInput(path);
            }
            NavItem::Tag => {
                self.toggle_tag();
            }
            NavItem::Space => {
                self.modal = Modal::Space;
            }
            NavItem::Copy => {
                if self.tagged_files.is_empty() {
                    self.modal = Modal::Error("No files tagged. Tag files with SPACE first.".to_string());
                } else {
                    let dest = self.current_path.to_string_lossy().to_string();
                    self.modal = Modal::CopyTo(dest);
                }
            }
            NavItem::Move => {
                if self.tagged_files.is_empty() {
                    self.modal = Modal::Error("No files tagged. Tag files with SPACE first.".to_string());
                } else {
                    let dest = self.current_path.to_string_lossy().to_string();
                    self.modal = Modal::MoveTo(dest);
                }
            }
            NavItem::Erase => {
                if self.tagged_files.is_empty() {
                    self.modal = Modal::Error("No files tagged. Tag files with SPACE first.".to_string());
                } else {
                    self.modal = Modal::EraseConfirm;
                }
            }
            NavItem::Rename => {
                if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error("No file selected for rename.".to_string());
                } else {
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
            NavItem::View | NavItem::Find | NavItem::Attribute | NavItem::Print => {
                self.modal = Modal::Error(format!("{} not yet implemented", nav_item.as_str()));
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

    /// Get total size of all files
    pub fn total_size(&self) -> u64 {
        self.files.iter().filter(|f| !f.is_dir).map(|f| f.size).sum()
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

    /// Refresh the current file list
    fn refresh_files(&mut self) -> Result<()> {
        self.files = get_directory_contents(&self.current_path, self.sort_mode)?;
        if self.selected_index >= self.files.len() && !self.files.is_empty() {
            self.selected_index = self.files.len() - 1;
        }
        Ok(())
    }

    /// Copy tagged files to destination directory
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

        let new_path = file.path.parent().unwrap_or(&self.current_path).join(new_name);
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
}

/// Recursively copy a directory
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
