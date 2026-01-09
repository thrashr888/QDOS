//! Q-MIND: AI Intelligence Layer plugin
//!
//! Provides semantic search and natural language commands for QDOS.
//! Press `?` to open the command palette from anywhere.

pub mod api;
pub mod command;
pub mod indexer;
mod modal;
pub mod search;
mod state;
pub mod summary;
pub mod vector;

use crate::plugins::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use command::{CommandExecutor, CommandParser, ExecutionResult};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, Frame};
use search::SemanticSearch;
use state::{DryRunState, QMindState, QMindView, SearchResult};
use std::any::Any;
use std::path::PathBuf;
use summary::FileSummarizer;

/// Q-MIND AI Intelligence Layer plugin
pub struct QMindPlugin {
    pub state: QMindState,
    /// Whether data is currently being loaded
    loading: bool,
    /// Command parser for natural language commands
    parser: Option<CommandParser>,
    /// Semantic search engine
    searcher: Option<SemanticSearch>,
    /// File summarizer
    summarizer: Option<FileSummarizer>,
    /// Current working directory (for indexing)
    cwd: PathBuf,
    /// Selected file for summarization
    selected_file: Option<PathBuf>,
}

impl Default for QMindPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl QMindPlugin {
    pub fn new() -> Self {
        Self {
            state: QMindState::new(),
            loading: false,
            parser: None,
            searcher: None,
            summarizer: None,
            cwd: std::env::current_dir().unwrap_or_default(),
            selected_file: None,
        }
    }

    /// Start loading
    pub fn start_loading(&mut self) {
        self.loading = true;
    }

    /// Check if currently loading
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Initialize Q-MIND (check API availability, etc.)
    fn initialize(&mut self) {
        self.state.check_api_availability();

        // Initialize parser and searcher if API is available
        if self.state.api_available {
            if self.parser.is_none() {
                self.parser = Some(CommandParser::from_env());
            }
            if self.searcher.is_none() {
                // Load existing index from disk
                let searcher = SemanticSearch::from_env();
                self.state.indexed_count = searcher.indexed_count();
                if self.state.indexed_count > 0 {
                    self.state.status_message =
                        Some(format!("Loaded {} indexed files", self.state.indexed_count));
                }
                self.searcher = Some(searcher);
            }
        }

        self.loading = false;
    }

    /// Parse and execute a natural language command
    fn parse_command(&mut self) {
        let input = self.state.command_input.text().to_string();
        if input.is_empty() {
            return;
        }

        self.state.clear_error();
        self.state.found_files.clear();

        // Get or create parser
        if self.parser.is_none() {
            self.parser = Some(CommandParser::from_env());
        }

        if let Some(parser) = &self.parser {
            match parser.parse(&input) {
                Ok(cmd) => {
                    // Store the parsed command for display
                    self.state.last_parsed_command = Some(cmd.clone());

                    // Execute the command
                    let executor = CommandExecutor::new(self.cwd.clone());
                    match executor.execute(&cmd) {
                        ExecutionResult::Success(msg) => {
                            self.state.set_error(msg); // Use error field for status messages too
                        }
                        ExecutionResult::Found(files) => {
                            self.state.found_files = files;
                        }
                        ExecutionResult::NeedsDryRun(ops) => {
                            // Set up dry run state
                            self.state.dry_run = Some(DryRunState::new(
                                format!("Q-MIND: {}", cmd.explanation),
                                ops,
                            ));
                            self.state.view = QMindView::DryRun;
                        }
                        ExecutionResult::Error(e) => {
                            self.state.set_error(e);
                        }
                        ExecutionResult::Unsupported(msg) => {
                            self.state.set_error(msg);
                        }
                    }
                }
                Err(e) => {
                    self.state.set_error(format!("Parse error: {}", e));
                }
            }
        }
    }

    /// Execute semantic search
    fn execute_search(&mut self) {
        let query = self.state.search_input.text().to_string();
        if query.is_empty() {
            return;
        }

        self.state.clear_error();

        // Get or create searcher
        if self.searcher.is_none() {
            self.searcher = Some(SemanticSearch::from_env());
        }

        if let Some(searcher) = &self.searcher {
            match searcher.search(&query) {
                Ok(results) => {
                    self.state.search_results = results
                        .into_iter()
                        .map(|r| SearchResult {
                            path: r.path,
                            score: r.score,
                            summary: Some(r.summary),
                        })
                        .collect();
                    self.state.search_selected = 0;
                }
                Err(e) => {
                    self.state.set_error(format!("Search error: {}", e));
                }
            }
        }
    }

    /// Index the current directory tree (recursive, respects .gitignore)
    fn index_directory(&mut self) {
        self.state.clear_error();
        self.state.indexing = true;
        self.state.status_message = Some("Indexing directory tree...".to_string());

        // Get or create searcher
        if self.searcher.is_none() {
            self.searcher = Some(SemanticSearch::from_env());
        }

        if let Some(searcher) = &mut self.searcher {
            // Use index_tree for recursive indexing that respects .gitignore
            match searcher.index_tree(&self.cwd) {
                Ok(count) => {
                    self.state.indexed_count = searcher.indexed_count();
                    self.state.status_message = Some(format!(
                        "Indexed {} new files ({} total)",
                        count, self.state.indexed_count
                    ));
                }
                Err(e) => {
                    self.state.status_message = Some(format!("Index error: {}", e));
                    self.state.set_error(format!("Index error: {}", e));
                }
            }
        }

        self.state.indexing = false;
    }

    /// Execute confirmed dry run operations
    fn execute_dry_run(&mut self) {
        if let Some(dry_run) = self.state.dry_run.take() {
            match CommandExecutor::execute_confirmed(&dry_run.operations) {
                Ok(count) => {
                    self.state
                        .set_error(format!("Executed {} operations successfully", count));
                }
                Err(e) => {
                    self.state.set_error(format!("Execution failed: {}", e));
                }
            }
        }
        self.state.view = QMindView::CommandPalette;
    }

    /// Generate summary for the selected file
    fn summarize_file(&mut self) {
        let path = match &self.selected_file {
            Some(p) => p.clone(),
            None => {
                self.state.set_error("No file selected".to_string());
                return;
            }
        };

        self.state.clear_error();

        // Get or create summarizer
        if self.summarizer.is_none() {
            self.summarizer = Some(FileSummarizer::from_env());
        }

        if let Some(summarizer) = &self.summarizer {
            match summarizer.summarize(&path) {
                Ok(summary) => {
                    self.state.file_summary = Some(summary);
                    self.state.view = QMindView::FileSummary;
                }
                Err(e) => {
                    self.state.set_error(format!("Summary error: {}", e));
                }
            }
        }
    }
}

impl Plugin for QMindPlugin {
    fn id(&self) -> &str {
        "qmind"
    }

    fn name(&self) -> &str {
        "Q-MIND"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: false, // Not in old menu system
            has_keys: true,  // Has ? global key
            has_modal: true,
            has_status: true,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        // Q-MIND is always available (shows setup info if no API key)
        true
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        // No menu item - accessed via ? key or App Launcher
        None
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        if self.state.api_available {
            Some(PluginStatusInfo {
                text: format!("Q-MIND {} files", self.state.indexed_count),
                active: true,
            })
        } else {
            None
        }
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        cwd: &PathBuf,
        selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        // ? key opens command palette directly
        if let KeyCode::Char('?') = key.code {
            self.cwd = cwd.clone();
            self.selected_file = selected_file.cloned();
            self.state.view = QMindView::CommandPalette;
            self.state.command_input.reset();
            self.start_loading(); // Trigger API check on first tick
            return KeyHandleResult::OpenModal;
        }
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            QMindView::Overview => match key.code {
                KeyCode::Esc => KeyHandleResult::CloseModal,
                KeyCode::Char('?') | KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.state.view = QMindView::CommandPalette;
                    self.state.command_input.reset();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.state.view = QMindView::SemanticSearch;
                    self.state.search_input.reset();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.state.view = QMindView::IndexStatus;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    // Trigger file summary for selected file
                    self.summarize_file();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            QMindView::CommandPalette => {
                match key.code {
                    KeyCode::Esc => {
                        self.state.view = QMindView::Overview;
                        KeyHandleResult::Handled
                    }
                    KeyCode::Enter => {
                        // Shift+Enter inserts newline, plain Enter parses command
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            self.state.command_input.insert_char('\n');
                        } else {
                            self.parse_command();
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Backspace => {
                        self.state.command_input.backspace();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Delete => {
                        self.state.command_input.delete();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Left => {
                        self.state.command_input.cursor_left();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Right => {
                        self.state.command_input.cursor_right();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Home => {
                        self.state.command_input.cursor_home();
                        KeyHandleResult::Handled
                    }
                    KeyCode::End => {
                        self.state.command_input.cursor_end();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char(c) => {
                        self.state.command_input.insert_char(c);
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                }
            }
            QMindView::SemanticSearch => {
                match key.code {
                    KeyCode::Esc => {
                        self.state.view = QMindView::Overview;
                        KeyHandleResult::Handled
                    }
                    KeyCode::Enter => {
                        // Shift+Enter inserts newline, plain Enter searches
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            self.state.search_input.insert_char('\n');
                        } else {
                            self.execute_search();
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Backspace => {
                        self.state.search_input.backspace();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Delete => {
                        self.state.search_input.delete();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Left => {
                        self.state.search_input.cursor_left();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Right => {
                        self.state.search_input.cursor_right();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Up => {
                        // Navigate search results
                        if self.state.search_selected > 0 {
                            self.state.search_selected -= 1;
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Down => {
                        // Navigate search results
                        if self.state.search_selected
                            < self.state.search_results.len().saturating_sub(1)
                        {
                            self.state.search_selected += 1;
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char(c) => {
                        self.state.search_input.insert_char(c);
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                }
            }
            QMindView::IndexStatus => match key.code {
                KeyCode::Esc => {
                    self.state.view = QMindView::Overview;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.index_directory();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            QMindView::FileSummary => match key.code {
                KeyCode::Esc => {
                    self.state.view = QMindView::Overview;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            QMindView::DryRun => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                        // Cancel - clear dry run and return to command palette
                        if let Some(ref mut dr) = self.state.dry_run {
                            dr.cancelled = true;
                        }
                        self.state.dry_run = None;
                        self.state.view = QMindView::CommandPalette;
                        self.state.set_error("Operation cancelled".to_string());
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        // Confirm and execute
                        self.execute_dry_run();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Enter => {
                        // Enter only confirms for non-destructive operations
                        let has_destructive = self
                            .state
                            .dry_run
                            .as_ref()
                            .map(|dr| dr.has_destructive())
                            .unwrap_or(false);
                        if !has_destructive {
                            self.execute_dry_run();
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if let Some(ref mut dr) = self.state.dry_run {
                            dr.select_prev();
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if let Some(ref mut dr) = self.state.dry_run {
                            dr.select_next();
                        }
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                }
            }
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_qmind_modal(frame, area, &self.state, self.loading, colors);
    }

    fn tick(&mut self) {
        if self.loading {
            self.initialize();
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-MIND - AI Intelligence Layer".to_string(),
            "".to_string(),
            "Natural language commands and semantic search".to_string(),
            "for your files.".to_string(),
            "".to_string(),
            "Global Keys:".to_string(),
            "  ?       Open command palette (from anywhere)".to_string(),
            "".to_string(),
            "Features:".to_string(),
            "  C       Command palette (natural language)".to_string(),
            "  S       Semantic search".to_string(),
            "  I       Index status".to_string(),
            "  Esc     Close/Back".to_string(),
            "".to_string(),
            "Examples:".to_string(),
            "  'copy *.txt to backup'".to_string(),
            "  'find that config file for rust'".to_string(),
            "  'delete old log files'".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Q-MIND".to_string(),
            description: "AI-powered commands & search".to_string(),
            category: PluginCategory::Tools,
            key: '?',
        })
    }

    fn launch(&mut self, cwd: &PathBuf, selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.cwd = cwd.clone();
        self.selected_file = selected_file.cloned();
        self.start_loading();
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
