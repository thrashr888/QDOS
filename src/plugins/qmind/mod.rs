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
use std::sync::{Arc, Mutex};
use std::thread;
use summary::{FileSummarizer, FileSummary};

/// Result from async operations
#[derive(Clone)]
enum AsyncResult {
    /// Indexing completed with (new_count, total_count)
    IndexComplete(usize, usize),
    /// Indexing failed with error message
    IndexError(String),
    /// Summary completed
    SummaryComplete(FileSummary),
    /// Summary failed with error message
    SummaryError(String),
}

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
    /// Shared state for async operation results (None = no result yet)
    async_result: Arc<Mutex<Option<AsyncResult>>>,
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
            async_result: Arc::new(Mutex::new(None)),
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

    /// Start async indexing of the current directory tree
    /// Spawns a thread and stores result in async_result for tick() to pick up
    fn start_async_indexing(&mut self) {
        self.state.clear_error();

        let cwd = self.cwd.clone();
        let result_holder = Arc::clone(&self.async_result);

        // Spawn thread to do indexing in background
        thread::spawn(move || {
            // Create searcher in thread (loads existing index from disk)
            let mut searcher = SemanticSearch::from_env();

            let result = match searcher.index_tree(&cwd) {
                Ok(count) => {
                    let total = searcher.indexed_count();
                    AsyncResult::IndexComplete(count, total)
                }
                Err(e) => AsyncResult::IndexError(format!("{}", e)),
            };

            // Store result for main thread to pick up
            if let Ok(mut holder) = result_holder.lock() {
                *holder = Some(result);
            }
        });
    }

    /// Start async file summary generation
    fn start_async_summary(&mut self) {
        let path = match &self.selected_file {
            Some(p) => p.clone(),
            None => {
                self.state.set_error("No file selected".to_string());
                self.state.generating_summary = false;
                return;
            }
        };

        let result_holder = Arc::clone(&self.async_result);

        // Spawn thread to generate summary in background
        thread::spawn(move || {
            let summarizer = FileSummarizer::from_env();

            let result = match summarizer.summarize(&path) {
                Ok(summary) => AsyncResult::SummaryComplete(summary),
                Err(e) => AsyncResult::SummaryError(format!("{}", e)),
            };

            // Store result for main thread to pick up
            if let Ok(mut holder) = result_holder.lock() {
                *holder = Some(result);
            }
        });
    }

    /// Check for and handle async operation results
    fn poll_async_result(&mut self) {
        // Try to get result without blocking
        let result = {
            if let Ok(mut holder) = self.async_result.try_lock() {
                holder.take()
            } else {
                None
            }
        };

        if let Some(result) = result {
            match result {
                AsyncResult::IndexComplete(new_count, total_count) => {
                    self.state.indexed_count = total_count;
                    self.state.status_message = Some(format!(
                        "Indexed {} new files ({} total)",
                        new_count, total_count
                    ));
                    self.state.indexing = false;
                    // Reload searcher to get updated index
                    self.searcher = Some(SemanticSearch::from_env());
                }
                AsyncResult::IndexError(e) => {
                    self.state.status_message = Some(format!("Index error: {}", e));
                    self.state.set_error(format!("Index error: {}", e));
                    self.state.indexing = false;
                }
                AsyncResult::SummaryComplete(summary) => {
                    self.state.file_summary = Some(summary);
                    self.state.generating_summary = false;
                }
                AsyncResult::SummaryError(e) => {
                    self.state.set_error(format!("Summary error: {}", e));
                    self.state.generating_summary = false;
                }
            }
        }
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
                    // Refresh indexed count when opening status
                    if let Some(searcher) = &self.searcher {
                        self.state.indexed_count = searcher.indexed_count();
                    }
                    self.state.view = QMindView::IndexStatus;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    // Start file summary generation (happens in tick())
                    if self.selected_file.is_some() {
                        self.state.generating_summary = true;
                        self.state.file_summary = None;
                        self.state.clear_error();
                        self.state.view = QMindView::FileSummary;
                    } else {
                        self.state.set_error("No file selected".to_string());
                    }
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
                    // Start indexing (actual work happens in tick())
                    if !self.state.indexing {
                        self.state.indexing = true;
                        self.state.status_message = Some("Starting index...".to_string());
                    }
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

        // Check if we need to start async operations
        let should_start_indexing = self.state.indexing && {
            // Only start if no result pending
            self.async_result
                .try_lock()
                .map(|r| r.is_none())
                .unwrap_or(false)
        };
        let should_start_summary = self.state.generating_summary && {
            self.async_result
                .try_lock()
                .map(|r| r.is_none())
                .unwrap_or(false)
        };

        // Start async operations (only once per request)
        if should_start_indexing {
            self.start_async_indexing();
        }
        if should_start_summary {
            self.start_async_summary();
        }

        // Poll for async results
        self.poll_async_result();
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
