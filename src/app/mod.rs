mod state;

// Re-export plugin ops for backwards compatibility within this module
use crate::plugins::beads::ops as beads_ops;
use crate::plugins::git::ops as git_ops;
use crate::plugins::jj::ops as jj_ops;

// Re-export state types for external use (non-plugin types)
pub use state::{
    AttrValue, AttributeState, BatchRenameState, BeadsState, BeadsView, ClipboardItem,
    ClipboardState, ColorTheme, ColorThemeState, FindPhase, FindState, Modal, NavItem,
    ProgressOperation, ProgressState, SearchMode, SortMode, ThemeColors,
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
pub use state::{BeadsActivityEntry, BeadsComment, BeadsIssue};

use crate::config::Config;
use crate::errors;
use crate::event::EventHandler;
use crate::file_ops::{
    apply_attributes, find_files_recursive, get_directory_contents,
    get_directory_contents_with_provider, FileEntry,
};
use crate::plugins::{
    fileops::FileOperation, AIPlugin, AppsPlugin, AudioPlugin, BasicPlugin, BeadsPlugin,
    DatabasePlugin, DepsPlugin, DirMapPlugin, DockerPlugin, DrivesPlugin, DropboxPlugin,
    EmulatorPlugin, FileOpsPlugin, GDrivePlugin, GamesPlugin, GitPlugin, HelpPlugin,
    HomebrewPlugin, ICloudPlugin, JjPlugin, KeyHandleResult, MidiPlugin, Model3dPlugin,
    PalettePlugin, PluginManager, PluginMenuItem, PluginStatusInfo, PrintPlugin, ProcPlugin,
    QEditPlugin, QLinkPlugin, QMindPlugin, QTaskPlugin, QdconfigPlugin, RedisPlugin,
    SearchSpecPlugin, SftpPlugin, ShellPlugin, SpacePlugin, StatusPlugin, TerraformPlugin,
    ThemePlugin, VideoPlugin, ViewerPlugin,
};
use crate::ui;
use crate::vfs::{FileSystemProvider, RoutingFS};
use crate::watcher::DirWatcher;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use jumprs::Database as JumpDatabase;
use ratatui::prelude::*;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

/// Set terminal taskbar progress indicator (OSC 9;4)
/// Supported by: Windows Terminal, Kitty, Foot, GNOME Terminal, ConsoleZ
fn set_terminal_progress(percent: u8) {
    // OSC 9;4;1;<percent> ST - Set progress indicator
    let _ = write!(io::stdout(), "\x1b]9;4;1;{}\x07", percent.min(100));
    let _ = io::stdout().flush();
}

/// Clear terminal taskbar progress indicator
fn clear_terminal_progress() {
    // OSC 9;4;0 ST - Remove progress indicator
    let _ = write!(io::stdout(), "\x1b]9;4;0\x07");
    let _ = io::stdout().flush();
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
    /// JJ status bar info - None if not in jj repo
    pub jj_status_info: Option<jj_ops::JjStatusInfo>,
    /// Plugin manager for extensibility
    pub plugin_manager: PluginManager,
    /// Last time the file list was refreshed (for auto-refresh)
    pub last_refresh: std::time::Instant,
    /// Directory watcher for event-based file updates
    pub dir_watcher: Option<DirWatcher>,
    /// Terminal background luma (0.0 = black, 1.0 = white)
    /// Used for light/dark mode detection
    pub terminal_luma: Option<f32>,
    /// Directory jump database (zoxide, z, autojump, fasd)
    pub z_db: Option<JumpDatabase>,
    /// Virtual filesystem provider with routing for MCP mounts
    /// Used for Q-LINK MCP integration - routes to MCP providers for mounted paths
    #[allow(dead_code)] // Will be used when file operations are routed through VFS
    pub routing_fs: Arc<RoutingFS>,
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

        // Initialize the VFS provider with routing for MCP mounts
        // This must be created before Q-LINK plugin so it can share the routing filesystem
        let routing_fs = Arc::new(RoutingFS::new());

        // Initialize plugin manager with config and register built-in plugins
        let mut plugin_manager = PluginManager::with_config(config.plugins.clone());
        plugin_manager.register(Box::new(AIPlugin::new()));
        plugin_manager.register(Box::new(AppsPlugin::new()));
        plugin_manager.register(Box::new(BasicPlugin::new()));
        plugin_manager.register(Box::new(BeadsPlugin::new()));
        plugin_manager.register(Box::new(DatabasePlugin::new()));
        plugin_manager.register(Box::new(DepsPlugin::new()));
        plugin_manager.register(Box::new(DirMapPlugin::new()));
        plugin_manager.register(Box::new(DockerPlugin::new()));
        plugin_manager.register(Box::new(DrivesPlugin::new()));
        plugin_manager.register(Box::new(DropboxPlugin::new()));
        plugin_manager.register(Box::new(EmulatorPlugin::new()));
        plugin_manager.register(Box::new(GDrivePlugin::new()));
        plugin_manager.register(Box::new(ICloudPlugin::new()));
        plugin_manager.register(Box::new(FileOpsPlugin::new()));
        plugin_manager.register(Box::new(GamesPlugin::new()));
        plugin_manager.register(Box::new(GitPlugin::new()));
        plugin_manager.register(Box::new(HelpPlugin::new()));
        plugin_manager.register(Box::new(HomebrewPlugin::new()));
        plugin_manager.register(Box::new(JjPlugin::new()));
        plugin_manager.register(Box::new(MidiPlugin::new()));
        plugin_manager.register(Box::new(Model3dPlugin::new()));
        plugin_manager.register(Box::new(AudioPlugin::new()));
        plugin_manager.register(Box::new(PalettePlugin::new()));
        plugin_manager.register(Box::new(PrintPlugin::new()));
        plugin_manager.register(Box::new(ProcPlugin::new()));
        plugin_manager.register(Box::new(QdconfigPlugin::new()));
        plugin_manager.register(Box::new(QEditPlugin::new()));
        plugin_manager.register(Box::new(QLinkPlugin::with_routing_fs(Arc::clone(
            &routing_fs,
        ))));
        plugin_manager.register(Box::new(QMindPlugin::new()));
        plugin_manager.register(Box::new(QTaskPlugin::new()));
        plugin_manager.register(Box::new(RedisPlugin::new()));
        plugin_manager.register(Box::new(SearchSpecPlugin::new()));
        plugin_manager.register(Box::new(SftpPlugin::new()));
        plugin_manager.register(Box::new(ShellPlugin::new()));
        plugin_manager.register(Box::new(SpacePlugin::new()));
        plugin_manager.register(Box::new(StatusPlugin::new()));
        plugin_manager.register(Box::new(TerraformPlugin::new()));
        plugin_manager.register(Box::new(ThemePlugin::new()));
        plugin_manager.register(Box::new(VideoPlugin::new()));
        plugin_manager.register(Box::new(ViewerPlugin::new()));

        // Collect help content from plugins and pass to HelpPlugin
        let plugin_help = plugin_manager.collect_plugin_help();
        if let Some(help_plugin) = plugin_manager.help_plugin_mut() {
            help_plugin.load_plugin_help(plugin_help);
        }

        let current_path = PathBuf::from(start_path).canonicalize()?;
        let files = get_directory_contents(&current_path, sort_mode)?;
        let watcher = DirWatcher::new(&current_path).ok();

        // Load directory jump database (auto-detects zoxide, z, autojump, fasd)
        let z_db = JumpDatabase::detect().ok();

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
            jj_status_info: None,
            plugin_manager,
            last_refresh: std::time::Instant::now(),
            dir_watcher: watcher,
            terminal_luma: None,
            z_db,
            routing_fs,
        };

        // Detect terminal light/dark mode using OSC 10/11 query
        // This must be done before entering raw mode in main.rs
        app.terminal_luma = terminal_light::luma().ok();

        // Load status bar info
        app.refresh_status_bar();

        Ok(app)
    }

    /// Check if the terminal has a light background (luma > 0.6)
    #[allow(dead_code)]
    pub fn is_light_terminal(&self) -> bool {
        self.terminal_luma.is_some_and(|l| l > 0.6)
    }

    /// Get theme colors adjusted for the terminal's light/dark mode
    pub fn theme_colors(&self) -> ThemeColors {
        self.color_theme.colors_for_luma(self.terminal_luma)
    }

    /// Read directory contents using the routing filesystem
    /// This routes to MCP providers for mounted paths
    fn read_directory(&self, path: &std::path::Path) -> Result<Vec<FileEntry>> {
        // Check if this path is under a mount point
        if self.routing_fs.is_mounted_path(path) {
            get_directory_contents_with_provider(path, self.sort_mode, self.routing_fs.as_ref())
        } else {
            get_directory_contents(&path.to_path_buf(), self.sort_mode)
        }
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
                    crate::event::Event::Key(key) => {
                        // Debug: log key events to file
                        if let Ok(mut f) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("/tmp/rdos-keys.log")
                        {
                            use std::io::Write;
                            let _ = writeln!(f, "Key: {:?}", key.code);
                        }
                        self.handle_key(key)?
                    }
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
                if matches!(self.modal, Modal::None) && self.selected_index > 0 {
                    self.selected_index -= 1;
                    // Adjust scroll if needed
                    if self.selected_index < self.scroll_offset {
                        self.scroll_offset = self.selected_index;
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                // Scroll down in file list
                if matches!(self.modal, Modal::None) && self.selected_index + 1 < self.files.len() {
                    self.selected_index += 1;
                    // Note: scroll adjustment happens in UI render
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
            // Change drive - handled by DrivesPlugin via plugin system
            // F3 key is intercepted by handle_plugin_key() before reaching here
            // Previous directory
            KeyCode::F(4) => {
                self.go_to_parent()?;
            }
            // Change directory - start with empty input for z jumper style
            // Shift+F5 = reload config
            KeyCode::F(5) => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    match self.reload_config() {
                        Ok(msg) => {
                            self.modal = Modal::Success(msg);
                        }
                        Err(e) => {
                            self.modal = Modal::Error(format!("Config reload failed: {}", e));
                        }
                    }
                } else {
                    self.modal = Modal::PathInput(String::new());
                }
            }
            // Shell Command - handled by ShellPlugin via plugin system
            // Search spec - handled by SearchSpecPlugin via plugin system
            // F7 key is intercepted by handle_plugin_key() before reaching here
            // Sort
            KeyCode::F(8) => {
                self.cycle_sort_mode()?;
            }
            // Edit - open in Q-EDIT (built-in editor)
            KeyCode::F(9) => {
                self.edit_in_qedit()?;
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
            KeyCode::Char('t') | KeyCode::Char('T')
                if !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                // Tag - toggle tag on selected file (Ctrl+T is theme, handled by plugin)
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
            KeyCode::Char('o') | KeyCode::Char('O') => {
                // Open - open file in default application
                self.nav_index = NavItem::ALL
                    .iter()
                    .position(|n| *n == NavItem::Open)
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
            KeyCode::Char('K') => {
                // MkDir (uppercase K only - lowercase k is vim-up)
                self.nav_index = NavItem::ALL
                    .iter()
                    .position(|n| *n == NavItem::MkDir)
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
        let is_viewer = matches!(&self.modal, Modal::Plugin(id) if id == "viewer");
        if matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y'))
            && (matches!(self.modal, Modal::Beads(_) | Modal::Git(_)) || is_viewer)
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
                    // First try z match if it looks like a query (short text without slashes)
                    let input = path.clone();
                    let is_z_query =
                        !input.contains('/') && !input.contains('\\') && input.len() < 50;

                    if is_z_query {
                        if let Some(entry) = self.z_db.as_ref().and_then(|db| db.best_match(&input))
                        {
                            let z_path = entry.path.clone();
                            self.modal = Modal::None;
                            if let Err(e) = self.navigate_to(&z_path) {
                                self.modal = Modal::Error(format!("Cannot navigate: {}", e));
                            }
                            return Ok(());
                        }
                    }

                    // Fall back to literal path navigation
                    let new_path = PathBuf::from(input);
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
                    // Try z completion first for short queries without path separators
                    let is_z_query = !path.contains('/') && !path.contains('\\') && path.len() < 50;

                    if is_z_query {
                        if let Some(entry) = self.z_db.as_ref().and_then(|db| db.best_match(path)) {
                            *path = entry.path.to_string_lossy().to_string();
                            return Ok(());
                        }
                    }

                    // Fall back to filesystem completion
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
            Modal::MkDirInput(ref mut dir_name) => match key.code {
                KeyCode::Enter => {
                    let name = dir_name.trim().to_string();
                    if name.is_empty() {
                        self.modal = Modal::Error("Directory name cannot be empty".to_string());
                    } else {
                        let new_dir = self.current_path.join(&name);
                        match std::fs::create_dir(&new_dir) {
                            Ok(()) => {
                                self.modal = Modal::Success(format!("Created directory: {}", name));
                                let _ = self.refresh_files();
                            }
                            Err(e) => {
                                self.modal =
                                    Modal::Error(format!("Failed to create directory: {}", e));
                            }
                        }
                    }
                }
                KeyCode::Esc => {
                    self.modal = Modal::None;
                }
                KeyCode::Backspace => {
                    dir_name.pop();
                }
                KeyCode::Char(c) => {
                    dir_name.push(c);
                }
                _ => {}
            },
            Modal::Find(ref mut state) => {
                use crate::app::SearchMode;
                match state.phase {
                    FindPhase::SelectMode => {
                        match key.code {
                            KeyCode::Enter | KeyCode::Char('1') => {
                                // Use current mode (default: ByName)
                                state.phase = FindPhase::InputPattern;
                            }
                            KeyCode::Char('2') if state.search_tool_available => {
                                // Switch to content search
                                state.search_mode = SearchMode::ByContent;
                                state.phase = FindPhase::InputPattern;
                            }
                            KeyCode::Tab => {
                                // Toggle between modes (if search tool available)
                                if state.search_tool_available {
                                    state.search_mode = state.search_mode.toggle();
                                }
                            }
                            KeyCode::Esc => {
                                self.modal = Modal::None;
                            }
                            _ => {}
                        }
                    }
                    FindPhase::InputPattern => {
                        match key.code {
                            KeyCode::Enter => {
                                if state.pattern.is_empty() {
                                    // Use *.* if no pattern entered (for name search)
                                    if state.search_mode == SearchMode::ByName {
                                        state.pattern = "*.*".to_string();
                                    } else {
                                        // For content search, require a pattern
                                        return Ok(());
                                    }
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

                                // Use appropriate search function based on mode and tool
                                state.matches = match state.search_mode {
                                    SearchMode::ByName => {
                                        find_files_recursive(&root, &state.pattern)
                                    }
                                    SearchMode::ByContent => crate::rg::search_content_with_tool(
                                        &root,
                                        &state.pattern,
                                        state.search_tool,
                                    ),
                                };

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

                                // Use appropriate search function based on mode and tool
                                state.matches = match state.search_mode {
                                    SearchMode::ByName => {
                                        find_files_recursive(&root, &state.pattern)
                                    }
                                    SearchMode::ByContent => crate::rg::search_content_with_tool(
                                        &root,
                                        &state.pattern,
                                        state.search_tool,
                                    ),
                                };

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
                                    let file_path = path.clone();
                                    let cwd = self.current_path.clone();
                                    if let Some(plugin) = self.plugin_manager.viewer_plugin_mut() {
                                        if plugin.open_file(file_path, &cwd).is_ok() {
                                            self.plugin_manager.set_active_modal(Some("viewer"));
                                            self.modal = Modal::Plugin("viewer".to_string());
                                        }
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
                                    let file_path = path.clone();
                                    let cwd = self.current_path.clone();
                                    if let Some(plugin) = self.plugin_manager.viewer_plugin_mut() {
                                        if plugin.open_file(file_path, &cwd).is_ok() {
                                            self.plugin_manager.set_active_modal(Some("viewer"));
                                            self.modal = Modal::Plugin("viewer".to_string());
                                        }
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
            Modal::Error(_) | Modal::Success(_) => match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                    self.modal = Modal::None;
                }
                _ => {}
            },
            Modal::Git(ref mut state) => {
                // Delegate key handling to GitPlugin
                let cwd = self.current_path.clone();
                if let Some(git_plugin) = self.plugin_manager.git_plugin_mut() {
                    let result = git_plugin.handle_external_state_key(key, state, &cwd);
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

    /// Move file selection up or down
    fn move_selection(&mut self, delta: i32) {
        if self.files.is_empty() {
            // Log empty files
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rdos-keys.log")
            {
                use std::io::Write;
                let _ = writeln!(f, "move_selection: files is empty!");
            }
            return;
        }

        let new_index = if delta < 0 {
            self.selected_index.saturating_sub((-delta) as usize)
        } else {
            (self.selected_index + delta as usize).min(self.files.len() - 1)
        };

        self.selected_index = new_index;

        // Log selection change
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rdos-keys.log")
        {
            use std::io::Write;
            let _ = writeln!(
                f,
                "move_selection: delta={}, new_index={}, files.len={}",
                delta,
                new_index,
                self.files.len()
            );
        }

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

            // Update terminal taskbar progress (OSC 9;4)
            let percent = if state.files.is_empty() {
                100
            } else {
                ((state.current_index as f64 / state.files.len() as f64) * 100.0) as u8
            };
            set_terminal_progress(percent);

            // Check if done
            if state.is_done() {
                // Clear terminal progress indicator
                clear_terminal_progress();

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
        // Log navigation attempt
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rdos-keys.log")
        {
            use std::io::Write;
            let is_mounted = self.routing_fs.is_mounted_path(path);
            let _ = writeln!(f, "navigate_to: path={:?}, is_mounted={}", path, is_mounted);
        }

        // For MCP-mounted paths, use VFS operations instead of local filesystem
        let canonical = if self.routing_fs.is_mounted_path(path) {
            // For virtual paths, just use the path as-is (it's already "canonical" in VFS terms)
            path.clone()
        } else {
            path.canonicalize()?
        };

        // Check if it's a directory using the appropriate filesystem
        let is_dir = if self.routing_fs.is_mounted_path(&canonical) {
            self.routing_fs.is_dir(&canonical)
        } else {
            canonical.is_dir()
        };

        if !is_dir {
            anyhow::bail!("Not a directory");
        }

        // Save current path to history and clear forward stack
        self.history.push(self.current_path.clone());
        self.forward_history.clear();

        // Update state
        self.current_path = canonical.clone();
        self.files = self.read_directory(&self.current_path)?;
        self.selected_index = 0;
        self.scroll_offset = 0;

        // Update directory watcher (only for local paths)
        if !self.routing_fs.is_mounted_path(&canonical) {
            if let Some(ref mut watcher) = self.dir_watcher {
                let _ = watcher.watch_path(&canonical);
            }
        }

        // Refresh git/beads status for new directory
        self.refresh_status_bar();

        // Record visit to jump database for frecency tracking
        if let Some(ref mut db) = self.z_db {
            db.add_visit(canonical);
            let _ = db.save(); // Best effort save
        }

        Ok(())
    }

    /// Go back in navigation history
    fn go_back(&mut self) -> Result<()> {
        if let Some(prev_path) = self.history.pop() {
            // Save current to forward stack
            self.forward_history.push(self.current_path.clone());

            // Navigate to previous path
            self.current_path = prev_path.clone();
            self.files = self.read_directory(&self.current_path)?;
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
            self.files = self.read_directory(&self.current_path)?;
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
        self.files = self.read_directory(&self.current_path)?;
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
                if let Some(space_plugin) = self.plugin_manager.space_plugin_mut() {
                    space_plugin.open_modal(&self.current_path);
                    self.plugin_manager.set_active_modal(Some("space"));
                    self.modal = Modal::Plugin("space".to_string());
                }
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
            NavItem::MkDir => {
                // Open MkDir dialog to create a new directory
                self.modal = Modal::MkDirInput(String::new());
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
            NavItem::Jj => {
                // Open jj modal via plugin
                if let Some(jj_plugin) = self.plugin_manager.jj_plugin_mut() {
                    jj_plugin.open_modal(&self.current_path);
                    self.plugin_manager.set_active_modal(Some("jj"));
                    self.modal = Modal::Plugin("jj".to_string());
                }
            }
            NavItem::View => {
                if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else if self.files[self.selected_index].is_dir {
                    self.modal = Modal::Error(errors::file::CANNOT_VIEW_DIR.to_string());
                } else {
                    let file = &self.files[self.selected_index];
                    let file_path = file.path.clone();
                    let cwd = self.current_path.clone();

                    // Check for .taskpaper files - use Q-TASK plugin
                    let ext = file_path
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase());
                    if ext.as_deref() == Some("taskpaper") {
                        if let Some(plugin) = self.plugin_manager.qtask_plugin_mut() {
                            match plugin.open_file(file_path.clone()) {
                                Ok(()) => {
                                    self.plugin_manager.set_active_modal(Some("qtask"));
                                    self.modal = Modal::Plugin("qtask".to_string());
                                }
                                Err(e) => {
                                    self.modal = Modal::Error(format!("Failed to open: {}", e));
                                }
                            }
                        }
                    } else {
                        // Check if this is a VFS path - if so, read via VFS
                        let is_vfs = self.routing_fs.is_mounted_path(&file_path);

                        // Debug log
                        if let Ok(mut f) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("/tmp/rdos-keys.log")
                        {
                            use std::io::Write;
                            let _ = writeln!(
                                f,
                                "NavItem::View: file_path={:?}, is_vfs={}",
                                file_path, is_vfs
                            );
                        }

                        if let Some(plugin) = self.plugin_manager.viewer_plugin_mut() {
                            let result = if is_vfs {
                                // Read file content via VFS
                                match self.routing_fs.read_file(&file_path) {
                                    Ok(content) => {
                                        // Log success
                                        if let Ok(mut f) = std::fs::OpenOptions::new()
                                            .create(true)
                                            .append(true)
                                            .open("/tmp/rdos-keys.log")
                                        {
                                            use std::io::Write;
                                            let _ = writeln!(
                                                f,
                                                "VFS read success: {} bytes",
                                                content.len()
                                            );
                                        }
                                        plugin.open_file_with_content(file_path, content, &cwd)
                                    }
                                    Err(e) => {
                                        // Log error
                                        if let Ok(mut f) = std::fs::OpenOptions::new()
                                            .create(true)
                                            .append(true)
                                            .open("/tmp/rdos-keys.log")
                                        {
                                            use std::io::Write;
                                            let _ = writeln!(f, "VFS read error: {}", e);
                                        }
                                        Err(format!("VFS read error: {}", e))
                                    }
                                }
                            } else {
                                // Use standard file reading
                                plugin.open_file(file_path, &cwd)
                            };

                            match result {
                                Ok(()) => {
                                    self.plugin_manager.set_active_modal(Some("viewer"));
                                    self.modal = Modal::Plugin("viewer".to_string());
                                }
                                Err(_e) => {
                                    self.modal = Modal::Error(
                                        errors::file::CANNOT_OPEN_HIGHLIGHTED.to_string(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            NavItem::Open => {
                if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else {
                    let file = &self.files[self.selected_index];
                    match crate::file_ops::open_in_default_app(&file.path) {
                        Ok(()) => {
                            self.modal = Modal::Success(format!(
                                "Opening {} in default application...",
                                file.name
                            ));
                        }
                        Err(e) => {
                            self.modal = Modal::Error(format!("Failed to open file: {}", e));
                        }
                    }
                }
            }
            NavItem::Find => {
                // Open Find dialog with configured search tool
                let search_tool = self.config.general.search_tool;
                let state = FindState::new(self.last_find_pattern.clone(), search_tool);
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

    /// Get the current theme colors (adjusted for terminal light/dark mode)
    pub fn colors(&self) -> ThemeColors {
        self.theme_colors()
    }

    /// Get total size of all files
    pub fn total_size(&self) -> u64 {
        self.files
            .iter()
            .filter(|f| !f.is_dir)
            .map(|f| f.size)
            .sum()
    }

    /// Reload configuration from disk
    /// Returns a message describing what was reloaded
    pub fn reload_config(&mut self) -> Result<String> {
        let config = Config::load().unwrap_or_default();

        // Track what changed
        let mut changes = Vec::new();

        // Update sort mode
        let new_sort_mode = config.to_sort_mode();
        if self.sort_mode != new_sort_mode {
            self.sort_mode = new_sort_mode;
            changes.push("sort");
            // Re-sort files
            self.files = self.read_directory(&self.current_path)?;
        }

        // Update theme
        let new_theme: ColorTheme = config.display.theme.clone().into();
        if self.color_theme != new_theme {
            self.color_theme = new_theme;
            changes.push("theme");
        }

        // Update search spec
        if self.search_spec != config.general.search_spec {
            self.search_spec = config.general.search_spec.clone();
            changes.push("search_spec");
        }

        // Update show hidden
        if self.show_hidden != config.general.show_hidden {
            self.show_hidden = config.general.show_hidden;
            changes.push("show_hidden");
            // Refresh file list
            self.files = self.read_directory(&self.current_path)?;
        }

        // Update plugin config
        self.plugin_manager.set_config(config.plugins.clone());

        // Store updated config
        self.config = config;

        let msg = if changes.is_empty() {
            "Config reloaded (no changes)".to_string()
        } else {
            format!("Config reloaded: {}", changes.join(", "))
        };

        Ok(msg)
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
        self.files = self.read_directory(&self.current_path)?;
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

    /// Open the selected file in Q-EDIT (built-in editor)
    fn edit_in_qedit(&mut self) -> Result<()> {
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

        let file_path = file.path.clone();
        if let Some(plugin) = self.plugin_manager.qedit_plugin_mut() {
            match plugin.open(Some(file_path)) {
                Ok(()) => {
                    self.plugin_manager.set_active_modal(Some("qedit"));
                    self.modal = Modal::Plugin("qedit".to_string());
                }
                Err(e) => {
                    self.modal = Modal::Error(format!("Failed to open file: {}", e));
                }
            }
        } else {
            self.modal = Modal::Error("Q-EDIT plugin not available".to_string());
        }

        Ok(())
    }

    /// Open the selected file in the default editor (external)
    #[allow(dead_code)]
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

    /// Check if current directory is in a jj repository
    pub fn is_jj_repo(&self) -> bool {
        // Walk up the directory tree looking for .jj
        let mut path = self.current_path.clone();
        loop {
            if path.join(".jj").is_dir() {
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
            Modal::Plugin(ref plugin_id) if plugin_id == "viewer" => {
                // Viewer plugin - add file path and commit info if viewing history
                if let Some(viewer) = self.plugin_manager.viewer_plugin() {
                    if let Some(state) = viewer.get_state() {
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

    /// Refresh status bar info (beads, git, and jj)
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

        // Refresh jj status
        if self.is_jj_repo() {
            self.jj_status_info = jj_ops::get_jj_status_bar_info(&self.current_path);
        } else {
            self.jj_status_info = None;
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
        let selected_file = self.files.get(self.selected_index).map(|e| e.path.clone());
        let result =
            self.plugin_manager
                .handle_global_key(key, &self.current_path, selected_file.as_ref());
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
                        let plugins = self.plugin_manager.plugin_list();
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
                                plugins,
                            );
                        }
                    }

                    // Collect app entries from all plugins and pass to AppsPlugin
                    if plugin_id == "apps" {
                        let entries = self.plugin_manager.collect_app_entries(&self.current_path);
                        if let Some(apps_plugin) = self.plugin_manager.apps_plugin_mut() {
                            apps_plugin.set_entries(entries);
                        }
                    }

                    // Collect app entries for PalettePlugin
                    if plugin_id == "palette" {
                        let entries = self.plugin_manager.collect_app_entries(&self.current_path);
                        let palette_apps: Vec<_> = entries
                            .into_iter()
                            .map(|e| crate::plugins::palette::PaletteApp {
                                id: e.id,
                                name: e.name,
                                description: e.description,
                            })
                            .collect();
                        if let Some(palette_plugin) = self.plugin_manager.palette_plugin_mut() {
                            palette_plugin.set_apps(palette_apps);
                            palette_plugin.update_results();
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
                } else if msg == "drives:navigate" {
                    // Handle drives navigation
                    if let Some(drives_plugin) = self.plugin_manager.drives_plugin_mut() {
                        if let Some(path) = drives_plugin.take_navigate_path() {
                            let _ = self.navigate_to(&path);
                        }
                    }
                    self.modal = Modal::None;
                } else if msg.starts_with("homebrew:install:") {
                    // Handle Homebrew package install
                    // For now, just close the modal - user can run brew install manually
                    // Future: could integrate with shell plugin to run the command
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
                } else if let Some(plugin_id) = msg.strip_prefix("launch:") {
                    // Launch a plugin modal from Apps launcher or Command Palette
                    self.launch_plugin_modal(plugin_id);
                } else if let Some(command_name) = msg.strip_prefix("command:") {
                    // Execute a NavItem command from Command Palette
                    self.modal = Modal::None;
                    if let Some(idx) = NavItem::ALL.iter().position(|n| n.as_str() == command_name)
                    {
                        self.nav_index = idx;
                        let _ = self.execute_action();
                    }
                } else if msg.starts_with("Copied:") {
                    // Clipboard copy from Command Palette - clipboard handled by the palette
                    // Just close modal and show success
                    self.modal = Modal::Success(msg);
                } else if let Some(toggle_info) = msg.strip_prefix("plugin_toggle:") {
                    // Handle plugin enable/disable toggle from Apps launcher
                    // Format: "plugin_toggle:plugin_id:true/false"
                    let parts: Vec<&str> = toggle_info.split(':').collect();
                    if parts.len() == 2 {
                        let plugin_id = parts[0];
                        let enabled = parts[1] == "true";

                        // Update config
                        if enabled {
                            // Remove from disabled list
                            self.config.plugins.disabled.retain(|id| id != plugin_id);
                        } else {
                            // Add to disabled list if not already there
                            if !self
                                .config
                                .plugins
                                .disabled
                                .contains(&plugin_id.to_string())
                            {
                                self.config.plugins.disabled.push(plugin_id.to_string());
                            }
                        }

                        // Save config to disk
                        if let Err(e) = self.save_config() {
                            self.modal = Modal::Error(format!("Failed to save config: {}", e));
                            return true;
                        }

                        let status = if enabled { "enabled" } else { "disabled" };
                        self.modal = Modal::Success(format!("Plugin '{}' {}", plugin_id, status));
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
            KeyHandleResult::NavigateToFile(path) => {
                // Close any active modal (both regular and plugin modals)
                self.modal = Modal::None;
                self.plugin_manager.set_active_modal(None);

                // Navigate to the file's parent directory
                if let Some(parent) = path.parent() {
                    if parent != self.current_path && parent.is_dir() {
                        self.current_path = parent.to_path_buf();
                        let _ = self.refresh_files();
                    }
                }

                // Try to select the file in the file list
                if let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy().to_string();
                    if let Some(idx) = self.files.iter().position(|f| f.name == filename_str) {
                        self.selected_index = idx;
                    }
                }

                true
            }
            KeyHandleResult::NavigateToDir(path) => {
                // Close any active modal (both regular and plugin modals)
                self.modal = Modal::None;
                self.plugin_manager.set_active_modal(None);

                // Navigate directly into the directory
                if let Err(e) = self.navigate_to(&path) {
                    // Log navigation error
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/rdos-keys.log")
                    {
                        use std::io::Write;
                        let _ = writeln!(f, "NavigateToDir error: {:?} -> {}", path, e);
                    }
                } else if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/rdos-keys.log")
                {
                    use std::io::Write;
                    let _ = writeln!(
                        f,
                        "NavigateToDir success: {:?}, files={}",
                        path,
                        self.files.len()
                    );
                }

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

    /// Launch a plugin modal by ID (from Apps launcher)
    fn launch_plugin_modal(&mut self, plugin_id: &str) {
        // Special cases for plugins that use dedicated Modal types
        match plugin_id {
            "beads" => {
                if self.is_beads_project() {
                    let state = BeadsState::new(true);
                    self.modal = Modal::Beads(state);
                } else {
                    self.modal = Modal::Error("Beads not initialized in this project".to_string());
                }
                return;
            }
            "git" => {
                if self.is_git_repo() {
                    let state = GitState::new(true);
                    self.modal = Modal::Git(state);
                } else {
                    self.modal = Modal::Error("Not a Git repository".to_string());
                }
                return;
            }
            "fileops" => {
                self.modal = Modal::Error("FileOps is used via file operations".to_string());
                return;
            }
            _ => {}
        }

        // Generic plugin launch - plugins implement their own launch() method
        let selected_file = self.files.get(self.selected_index).map(|e| e.path.clone());
        let cwd = self.current_path.clone();

        match self
            .plugin_manager
            .launch_plugin(plugin_id, &cwd, selected_file.as_ref())
        {
            Ok(id) => {
                self.modal = Modal::Plugin(id);
            }
            Err(e) => {
                self.modal = Modal::Error(e);
            }
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
