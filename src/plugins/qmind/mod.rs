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
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use summary::{FileSummarizer, FileSummary};

/// Format a duration as "Xm Ys" or "Xs" for short durations
fn format_duration(secs: u64) -> String {
    if secs >= 60 {
        let mins = secs / 60;
        let remaining_secs = secs % 60;
        format!("{}m{}s", mins, remaining_secs)
    } else {
        format!("{}s", secs)
    }
}

/// Maximum entries to keep in the progress log
const PROGRESS_LOG_SIZE: usize = 15;

/// Helper to write to the Q-MIND log file
fn log_to_file(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;

    if let Some(config_dir) = dirs::config_dir() {
        let log_path = config_dir.join("rdos").join("qmind.log");
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(file, "[{}] {}", timestamp, msg);
        }
    }
}

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
    /// Progress log for indexing (shared with background thread)
    progress_log: Arc<Mutex<VecDeque<String>>>,
    /// Whether an async operation thread has been spawned (prevents duplicate spawns)
    async_thread_running: bool,
    /// Real-time indexed count (shared with background thread)
    live_indexed_count: Arc<Mutex<usize>>,
    /// Total files to index (shared with background thread)
    total_files_to_index: Arc<Mutex<usize>>,
    /// Total tokens used (shared with background thread)
    tokens_used: Arc<Mutex<u32>>,
    /// Indexing start time (shared with background thread)
    indexing_start_time: Arc<Mutex<Option<Instant>>>,
    /// Cancellation flag for background indexing
    cancel_indexing: Arc<AtomicBool>,
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
            progress_log: Arc::new(Mutex::new(VecDeque::with_capacity(PROGRESS_LOG_SIZE))),
            async_thread_running: false,
            live_indexed_count: Arc::new(Mutex::new(0)),
            total_files_to_index: Arc::new(Mutex::new(0)),
            tokens_used: Arc::new(Mutex::new(0)),
            indexing_start_time: Arc::new(Mutex::new(None)),
            cancel_indexing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get indexing stats for display
    fn get_indexing_stats(&self) -> (usize, usize, u32) {
        let indexed = self.live_indexed_count.try_lock().map(|c| *c).unwrap_or(0);
        let total = self
            .total_files_to_index
            .try_lock()
            .map(|c| *c)
            .unwrap_or(0);
        let tokens = self.tokens_used.try_lock().map(|c| *c).unwrap_or(0);
        (indexed, total, tokens)
    }

    /// Get a copy of the current progress log
    pub fn get_progress_log(&self) -> Vec<String> {
        self.progress_log
            .lock()
            .map(|log| log.iter().cloned().collect())
            .unwrap_or_default()
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

                // Get provider info from the index
                self.state.provider = searcher.provider().to_string();
                self.state.embedding_model = searcher.embedding_model().to_string();

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
            // Check if index is empty
            if searcher.indexed_count() == 0 {
                self.state
                    .set_error("No files indexed. Press I to index first.".to_string());
                return;
            }

            match searcher.search(&query) {
                Ok(results) => {
                    if results.is_empty() {
                        self.state.set_error("No matching files found".to_string());
                    } else {
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
                }
                Err(e) => {
                    self.state.set_error(format!("Search error: {}", e));
                }
            }
        } else {
            self.state.set_error("Search not available".to_string());
        }
    }

    /// Start async indexing of the current directory tree
    /// Spawns a thread and stores result in async_result for tick() to pick up
    fn start_async_indexing(&mut self) {
        use ignore::WalkBuilder;

        self.state.clear_error();

        // Clear progress log
        if let Ok(mut log) = self.progress_log.lock() {
            log.clear();
        }

        // Reset all counters
        if let Ok(mut count) = self.live_indexed_count.lock() {
            *count = 0;
        }
        if let Ok(mut count) = self.total_files_to_index.lock() {
            *count = 0;
        }
        if let Ok(mut tokens) = self.tokens_used.lock() {
            *tokens = 0;
        }
        if let Ok(mut start) = self.indexing_start_time.lock() {
            *start = None;
        }

        // Reset cancellation flag
        self.cancel_indexing.store(false, Ordering::SeqCst);

        let cwd = self.cwd.clone();
        let result_holder = Arc::clone(&self.async_result);
        let progress_log = Arc::clone(&self.progress_log);
        let live_count = Arc::clone(&self.live_indexed_count);
        let total_count = Arc::clone(&self.total_files_to_index);
        let tokens_used = Arc::clone(&self.tokens_used);
        let start_time = Arc::clone(&self.indexing_start_time);
        let cancel_flag = Arc::clone(&self.cancel_indexing);

        // Spawn thread to do indexing in background with progress reporting
        thread::spawn(move || {
            log_to_file(&format!("=== Starting index of: {} ===", cwd.display()));

            // Directories to always skip (not useful for semantic search)
            const SKIP_DIRS: &[&str] = &[
                ".git",
                ".hg",
                ".svn",
                "node_modules",
                "target",
                ".beads",
                "__pycache__",
                ".venv",
                "venv",
                ".tox",
                "dist",
                "build",
                ".next",
                ".nuxt",
            ];

            // Helper to check if path should be skipped
            let should_skip = |path: &std::path::Path| {
                path.components().any(|c| {
                    let name = c.as_os_str().to_string_lossy();
                    SKIP_DIRS.contains(&name.as_ref())
                })
            };

            // Pre-count files to index
            if let Ok(mut log) = progress_log.lock() {
                log.push_back("Counting files...".to_string());
            }

            let pre_walker = WalkBuilder::new(&cwd)
                .hidden(false)
                .git_ignore(true)
                .git_global(true)
                .git_exclude(true)
                .build();

            let file_count: usize = pre_walker
                .flatten()
                .filter(|e| e.path().is_file() && !should_skip(e.path()))
                .count();

            if let Ok(mut count) = total_count.lock() {
                *count = file_count;
            }
            log_to_file(&format!("Found {} files to process", file_count));

            // Create searcher in thread (loads existing index from disk)
            let mut searcher = SemanticSearch::from_env();
            let initial_count = searcher.indexed_count();
            log_to_file(&format!(
                "Loaded existing index with {} entries",
                initial_count
            ));

            // Set initial count
            if let Ok(mut count) = live_count.lock() {
                *count = initial_count;
            }

            // Update progress log
            if let Ok(mut log) = progress_log.lock() {
                log.clear();
                log.push_back(format!("Found {} files to process", file_count));
                log.push_back(format!("Existing index: {} files", initial_count));
            }

            // Record start time for actual indexing
            let indexing_start = Instant::now();
            if let Ok(mut start) = start_time.lock() {
                *start = Some(indexing_start);
            }

            // Walk tree and index each file with progress updates
            let walker = WalkBuilder::new(&cwd)
                .hidden(false) // Include hidden files
                .git_ignore(true)
                .git_global(true)
                .git_exclude(true)
                .filter_entry(|entry| {
                    // Skip excluded directories
                    !entry.path().components().any(|c| {
                        let name = c.as_os_str().to_string_lossy();
                        SKIP_DIRS.contains(&name.as_ref())
                    })
                })
                .build();

            let mut indexed = 0;
            let mut skipped = 0;
            let mut errors = 0;
            let mut total_tokens: u32 = 0;
            let mut cancelled = false;

            for entry in walker.flatten() {
                // Check for cancellation
                if cancel_flag.load(Ordering::SeqCst) {
                    cancelled = true;
                    log_to_file("=== Indexing cancelled by user ===");
                    if let Ok(mut log) = progress_log.lock() {
                        log.clear();
                        log.push_back("Cancelled by user".to_string());
                        log.push_back(format!("Indexed: {} files before cancel", indexed));
                    }
                    break;
                }

                let path = entry.path();

                if path.is_file() {
                    let file_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    // Index the file
                    match searcher.index_file_with_tokens(path) {
                        Ok((true, tokens)) => {
                            indexed += 1;
                            total_tokens += tokens;
                            log_to_file(&format!(
                                "Indexed: {} ({} tokens)",
                                path.display(),
                                tokens
                            ));

                            // Update live count and tokens
                            if let Ok(mut count) = live_count.lock() {
                                *count += 1;
                            }
                            if let Ok(mut t) = tokens_used.lock() {
                                *t = total_tokens;
                            }

                            // Calculate cost: $0.02 per 1M tokens for text-embedding-3-small
                            let cost_dollars = (total_tokens as f64) * 0.02 / 1_000_000.0;

                            // Calculate time estimates
                            let elapsed = indexing_start.elapsed();
                            let elapsed_secs = elapsed.as_secs();
                            let processed = indexed + skipped;
                            let time_display = if processed > 0 && file_count > 0 {
                                // Estimate total time based on progress
                                let avg_time_per_file = elapsed.as_secs_f64() / processed as f64;
                                let estimated_total_secs =
                                    (avg_time_per_file * file_count as f64) as u64;
                                format!(
                                    "{}/~{}",
                                    format_duration(elapsed_secs),
                                    format_duration(estimated_total_secs)
                                )
                            } else {
                                format_duration(elapsed_secs)
                            };

                            // Update progress
                            if let Ok(mut log) = progress_log.lock() {
                                log.clear();
                                log.push_back(format!(
                                    "Progress: {}/{} files ({})",
                                    processed, file_count, time_display
                                ));
                                log.push_back(format!(
                                    "Indexed: {} | Skipped: {} | Errors: {}",
                                    indexed, skipped, errors
                                ));
                                log.push_back(format!(
                                    "Tokens: {} (${:.4})",
                                    total_tokens, cost_dollars
                                ));
                                log.push_back(format!("Last: {}", file_name));
                            }
                        }
                        Ok((false, _)) => {
                            skipped += 1;
                            // Update progress occasionally for skipped files too
                            if skipped % 20 == 0 {
                                let elapsed = indexing_start.elapsed();
                                let elapsed_secs = elapsed.as_secs();
                                let processed = indexed + skipped;
                                let time_display = if processed > 0 && file_count > 0 {
                                    let avg_time_per_file =
                                        elapsed.as_secs_f64() / processed as f64;
                                    let estimated_total_secs =
                                        (avg_time_per_file * file_count as f64) as u64;
                                    format!(
                                        "{}/~{}",
                                        format_duration(elapsed_secs),
                                        format_duration(estimated_total_secs)
                                    )
                                } else {
                                    format_duration(elapsed_secs)
                                };

                                if let Ok(mut log) = progress_log.lock() {
                                    log.clear();
                                    log.push_back(format!(
                                        "Progress: {}/{} files ({})",
                                        processed, file_count, time_display
                                    ));
                                    log.push_back(format!(
                                        "Indexed: {} | Skipped: {} | Errors: {}",
                                        indexed, skipped, errors
                                    ));
                                    if total_tokens > 0 {
                                        let cost_dollars =
                                            (total_tokens as f64) * 0.02 / 1_000_000.0;
                                        log.push_back(format!(
                                            "Tokens: {} (${:.4})",
                                            total_tokens, cost_dollars
                                        ));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            errors += 1;
                            let err_msg = format!("{}", e);
                            log_to_file(&format!("Error indexing {}: {}", path.display(), err_msg));

                            if let Ok(mut log) = progress_log.lock() {
                                log.push_back(format!("Error: {} - {}", file_name, err_msg));
                                while log.len() > PROGRESS_LOG_SIZE {
                                    log.pop_front();
                                }
                            }
                        }
                    }
                }
            }

            // Log completion (only if not cancelled - cancelled already logged)
            let total = searcher.indexed_count();
            let total_time = format_duration(indexing_start.elapsed().as_secs());

            if !cancelled {
                log_to_file(&format!(
                    "=== Index complete: {} new, {} total, {} skipped, {} errors in {} ===",
                    indexed, total, skipped, errors, total_time
                ));

                if let Ok(mut log) = progress_log.lock() {
                    log.clear();
                    log.push_back(format!("Done in {}", total_time));
                    log.push_back(format!("Indexed: {} new files", indexed));
                    log.push_back(format!("Total: {} in index", total));
                    log.push_back(format!("Skipped: {} (already indexed)", skipped));
                    if errors > 0 {
                        log.push_back(format!("Errors: {} (see qmind.log)", errors));
                    }
                }
            }

            // Store result for main thread to pick up
            let result = AsyncResult::IndexComplete(indexed, total);
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
            // Thread completed, reset the flag
            self.async_thread_running = false;

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
                        // Shift+Enter inserts newline
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            self.state.search_input.insert_char('\n');
                            KeyHandleResult::Handled
                        } else if !self.state.search_results.is_empty() {
                            // If we have results, open the selected one
                            if let Some(result) =
                                self.state.search_results.get(self.state.search_selected)
                            {
                                let path = result.path.clone();
                                // Clear search state before navigating
                                self.state.search_results.clear();
                                self.state.search_selected = 0;
                                return KeyHandleResult::NavigateToFile(path);
                            }
                            KeyHandleResult::Handled
                        } else {
                            // No results yet, execute search
                            self.execute_search();
                            KeyHandleResult::Handled
                        }
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
                    if self.state.indexing && !self.cancel_indexing.load(Ordering::SeqCst) {
                        // First Esc: request cancellation
                        self.cancel_indexing.store(true, Ordering::SeqCst);
                        self.state.status_message =
                            Some("Cancelling... (Esc again to exit)".to_string());
                    } else {
                        // Second Esc (or not indexing): go back to overview
                        // If still technically indexing, mark as not for UI purposes
                        self.state.indexing = false;
                        self.state.view = QMindView::Overview;
                    }
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
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    // Clear the index
                    if !self.state.indexing {
                        if let Some(ref mut searcher) = self.searcher {
                            searcher.clear();
                            self.state.indexed_count = 0;
                            self.state.provider = String::new();
                            self.state.embedding_model = String::new();
                            self.state.status_message = Some("Index cleared".to_string());
                            // Clear progress log
                            if let Ok(mut log) = self.progress_log.lock() {
                                log.clear();
                            }
                        }
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
        let progress_log = self.get_progress_log();
        modal::draw_qmind_modal(
            frame,
            area,
            &self.state,
            self.loading,
            &progress_log,
            colors,
        );
    }

    fn tick(&mut self) {
        if self.loading {
            self.initialize();
        }

        // Check if we need to start async operations (only if no thread already running)
        let should_start_indexing = self.state.indexing && !self.async_thread_running;
        let should_start_summary = self.state.generating_summary && !self.async_thread_running;

        // Start async operations (only once per request)
        if should_start_indexing {
            self.async_thread_running = true;
            self.start_async_indexing();
        }
        if should_start_summary {
            self.async_thread_running = true;
            self.start_async_summary();
        }

        // Update live indexed count while indexing
        if self.state.indexing {
            if let Ok(count) = self.live_indexed_count.try_lock() {
                self.state.indexed_count = *count;
            }
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
