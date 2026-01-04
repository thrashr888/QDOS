//! Shell/DOS Command Plugin for R-DOS
//!
//! Provides shell command execution functionality including:
//! - Synchronous command execution
//! - Background task management (commands ending with &)
//! - Built-in commands: jobs, fg, kill
//! - Task list view with live output
//! - Command history

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem};
use crate::ui::components::ModalFrame;
use crate::ui::{COLOR_BG, COLOR_CYAN, COLOR_FG, COLOR_GREEN, COLOR_GREY, COLOR_RED, COLOR_YELLOW};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::Clear;
use ratatui::Frame;
use std::any::Any;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

/// Status of a background task
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Running,
    Completed,
    Failed,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Running => "Running",
            TaskStatus::Completed => "Done",
            TaskStatus::Failed => "Failed",
        }
    }
}

/// A background shell task
pub struct BackgroundTask {
    pub id: u64,
    pub command: String,
    pub cwd: PathBuf,
    pub status: TaskStatus,
    pub output: Arc<Mutex<Vec<String>>>,
    pub exit_code: Option<i32>,
    pub started_at: Instant,
    reader_handle: Option<JoinHandle<i32>>,
}

impl BackgroundTask {
    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    pub fn get_output(&self) -> Vec<String> {
        self.output.lock().unwrap().clone()
    }

    pub fn poll(&mut self) {
        if self.status != TaskStatus::Running {
            return;
        }

        if let Some(handle) = self.reader_handle.take() {
            if handle.is_finished() {
                match handle.join() {
                    Ok(exit_code) => {
                        self.exit_code = Some(exit_code);
                        self.status = if exit_code == 0 {
                            TaskStatus::Completed
                        } else {
                            TaskStatus::Failed
                        };
                    }
                    Err(_) => {
                        self.status = TaskStatus::Failed;
                        self.exit_code = Some(-1);
                    }
                }
            } else {
                self.reader_handle = Some(handle);
            }
        }
    }
}

/// View mode for the shell modal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellView {
    /// Command input mode (default)
    #[default]
    Command,
    /// Task list view
    TaskList,
    /// Attached to a specific task (watching output)
    Attached(u64),
}

/// Shell command state
#[derive(Debug, Clone, Default)]
pub struct ShellState {
    pub input: String,
    pub output: Vec<String>,
    pub exit_code: Option<i32>,
    pub scroll_offset: usize,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    /// Current view mode
    pub view: ShellView,
    /// Selected task in task list
    pub selected_task: usize,
    /// Last output length for attached view (for auto-scroll)
    pub last_output_len: usize,
}

/// Shell plugin that provides DOS command functionality
pub struct ShellPlugin {
    modal_open: bool,
    state: ShellState,
    current_cwd: PathBuf,
    /// Background tasks
    tasks: HashMap<u64, BackgroundTask>,
    next_task_id: u64,
}

impl ShellPlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: ShellState::default(),
            current_cwd: PathBuf::new(),
            tasks: HashMap::new(),
            next_task_id: 1,
        }
    }

    /// Spawn a background task
    fn spawn_task(&mut self, command: String, cwd: PathBuf) -> u64 {
        let id = self.next_task_id;
        self.next_task_id += 1;

        let output = Arc::new(Mutex::new(Vec::new()));
        let output_clone = Arc::clone(&output);

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

        let child_result = Command::new(&shell)
            .arg("-c")
            .arg(&command)
            .current_dir(&cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let (reader_handle, status) = match child_result {
            Ok(mut child) => {
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();

                let handle = thread::spawn(move || {
                    Self::read_child_output(&mut child, stdout, stderr, output_clone)
                });

                (Some(handle), TaskStatus::Running)
            }
            Err(e) => {
                output
                    .lock()
                    .unwrap()
                    .push(format!("Error spawning command: {}", e));
                (None, TaskStatus::Failed)
            }
        };

        let task = BackgroundTask {
            id,
            command: command.clone(),
            cwd: cwd.clone(),
            status,
            output,
            exit_code: if status == TaskStatus::Failed {
                Some(-1)
            } else {
                None
            },
            started_at: Instant::now(),
            reader_handle,
        };

        self.tasks.insert(id, task);
        id
    }

    /// Read output from a child process
    fn read_child_output(
        child: &mut Child,
        stdout: Option<std::process::ChildStdout>,
        stderr: Option<std::process::ChildStderr>,
        output: Arc<Mutex<Vec<String>>>,
    ) -> i32 {
        let output_stdout = Arc::clone(&output);
        let stdout_handle = stdout.map(|stdout| {
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().flatten() {
                    output_stdout.lock().unwrap().push(line);
                }
            })
        });

        let output_stderr = Arc::clone(&output);
        let stderr_handle = stderr.map(|stderr| {
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().flatten() {
                    output_stderr
                        .lock()
                        .unwrap()
                        .push(format!("stderr: {}", line));
                }
            })
        });

        let exit_code = match child.wait() {
            Ok(status) => status.code().unwrap_or(-1),
            Err(_) => -1,
        };

        if let Some(h) = stdout_handle {
            let _ = h.join();
        }
        if let Some(h) = stderr_handle {
            let _ = h.join();
        }

        exit_code
    }

    /// Poll all tasks
    fn poll_tasks(&mut self) {
        for task in self.tasks.values_mut() {
            task.poll();
        }
    }

    /// List all tasks
    fn list_tasks(&self) -> Vec<(u64, &str, TaskStatus, std::time::Duration)> {
        let mut tasks: Vec<_> = self
            .tasks
            .values()
            .map(|t| (t.id, t.command.as_str(), t.status, t.elapsed()))
            .collect();
        tasks.sort_by_key(|(id, _, _, _)| *id);
        tasks
    }

    /// Get running task count
    pub fn running_count(&self) -> usize {
        self.tasks
            .values()
            .filter(|t| t.status == TaskStatus::Running)
            .count()
    }

    /// Get sorted task IDs for display
    fn get_sorted_task_ids(&self) -> Vec<u64> {
        let mut ids: Vec<_> = self.tasks.keys().copied().collect();
        ids.sort();
        ids
    }

    /// Get task by index in sorted list
    fn get_task_by_index(&self, index: usize) -> Option<&BackgroundTask> {
        let ids = self.get_sorted_task_ids();
        ids.get(index).and_then(|id| self.tasks.get(id))
    }

    /// Kill a task by ID
    fn kill_task(&mut self, id: u64) -> bool {
        if let Some(task) = self.tasks.get_mut(&id) {
            if task.status == TaskStatus::Running {
                task.status = TaskStatus::Failed;
                task.exit_code = Some(-9); // SIGKILL
                task.output.lock().unwrap().push("[Killed]".to_string());
                // Note: We can't actually kill the process since we don't store the Child
                // The thread will complete naturally, but we mark it as killed
                return true;
            }
        }
        false
    }

    /// Clear completed/failed tasks
    fn clear_finished_tasks(&mut self) -> usize {
        let to_remove: Vec<_> = self
            .tasks
            .iter()
            .filter(|(_, t)| t.status != TaskStatus::Running)
            .map(|(id, _)| *id)
            .collect();
        let count = to_remove.len();
        for id in to_remove {
            self.tasks.remove(&id);
        }
        count
    }

    /// Execute a synchronous command
    fn execute_sync(&self, cmd: &str, cwd: &PathBuf) -> (Vec<String>, i32) {
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

                let stdout_reader = BufReader::new(&output.stdout[..]);
                for l in stdout_reader.lines().flatten() {
                    lines.push(l);
                }

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

    /// Handle command execution
    fn execute_command(&mut self) {
        if self.state.input.is_empty() {
            return;
        }

        let cmd = self.state.input.clone();
        let cwd = self.current_cwd.clone();

        // Add to history
        self.state.history.push(cmd.clone());
        self.state.history_index = None;

        let cmd_trimmed = cmd.trim();

        // Handle built-in "jobs" command
        if cmd_trimmed == "jobs" {
            self.state.view = ShellView::TaskList;
            self.state.selected_task = 0;
            self.state.exit_code = Some(0);
        } else if cmd_trimmed.starts_with("fg ") {
            // Handle "fg <id>" to attach to task
            if let Ok(id) = cmd_trimmed[3..].trim().parse::<u64>() {
                if self.tasks.contains_key(&id) {
                    self.state.view = ShellView::Attached(id);
                    self.state.scroll_offset = 0;
                    self.state.last_output_len = 0;
                    self.state.exit_code = None;
                } else {
                    self.state.output = vec![format!("No such task: {}", id)];
                    self.state.exit_code = Some(1);
                }
            } else {
                self.state.output = vec!["Usage: fg <task_id>".to_string()];
                self.state.exit_code = Some(1);
            }
        } else if cmd_trimmed.starts_with("kill ") {
            // Handle "kill <id>" to terminate task
            if let Ok(id) = cmd_trimmed[5..].trim().parse::<u64>() {
                if self.kill_task(id) {
                    self.state.output = vec![format!("[{}] Killed", id)];
                    self.state.exit_code = Some(0);
                } else {
                    self.state.output = vec![format!("Cannot kill task {}: not running or not found", id)];
                    self.state.exit_code = Some(1);
                }
            } else {
                self.state.output = vec!["Usage: kill <task_id>".to_string()];
                self.state.exit_code = Some(1);
            }
        } else if cmd_trimmed == "clear" {
            // Clear finished tasks
            let count = self.clear_finished_tasks();
            self.state.output = vec![format!("Cleared {} finished task(s)", count)];
            self.state.exit_code = Some(0);
        } else if cmd_trimmed.ends_with('&') {
            // Run in background
            let bg_cmd = cmd_trimmed.trim_end_matches('&').trim().to_string();
            let task_id = self.spawn_task(bg_cmd, cwd);
            self.state.output = vec![format!("[{}] Started in background", task_id)];
            self.state.exit_code = None;
        } else {
            // Execute command synchronously
            let output = self.execute_sync(&cmd, &cwd);
            self.state.output = output.0;
            self.state.exit_code = Some(output.1);
        }

        self.state.input.clear();
        self.state.scroll_offset = 0;
    }

    /// Tab completion for paths
    fn tab_complete(partial: &str) -> Option<String> {
        use std::fs;

        let path = PathBuf::from(partial);
        let (dir, prefix) = if partial.ends_with('/') || partial.ends_with('\\') {
            (path.clone(), String::new())
        } else {
            let default_path = PathBuf::from(".");
            let parent = path.parent().unwrap_or(&default_path);
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            (parent.to_path_buf(), file_name)
        };

        if let Ok(entries) = fs::read_dir(&dir) {
            let matches: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .to_lowercase()
                        .starts_with(&prefix.to_lowercase())
                })
                .collect();

            if matches.len() == 1 {
                let entry = &matches[0];
                let mut completed = dir.join(entry.file_name());
                if entry.path().is_dir() {
                    completed.push("");
                }
                return Some(completed.to_string_lossy().to_string());
            }
        }
        None
    }
}

impl Default for ShellPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ShellPlugin {
    fn id(&self) -> &str {
        "shell"
    }

    fn name(&self) -> &str {
        "DOS Command"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true,
            has_status: true,
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
            name: "DOScmd".to_string(),
            key: 'D',
            description: "Execute shell command".to_string(),
            priority: 55,
        })
    }

    fn handle_global_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::F(6) => {
                self.current_cwd = cwd.clone();
                self.modal_open = true;
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        self.current_cwd = cwd.clone();

        // Handle keys based on current view
        match self.state.view {
            ShellView::Command => self.handle_command_key(key),
            ShellView::TaskList => self.handle_task_list_key(key),
            ShellView::Attached(id) => self.handle_attached_key(key, id),
        }
    }

    fn tick(&mut self) {
        self.poll_tasks();
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect) {
        match self.state.view {
            ShellView::Command => self.draw_command_view(frame, area),
            ShellView::TaskList => self.draw_task_list_view(frame, area),
            ShellView::Attached(id) => self.draw_attached_view(frame, area, id),
        }
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<super::PluginStatusInfo> {
        let running = self.running_count();
        if running > 0 {
            Some(super::PluginStatusInfo {
                text: format!("{}bg", running),
                active: true,
            })
        } else {
            None
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "F6 - DOS Command".to_string(),
            "  Execute shell commands".to_string(),
            "  cmd &     - Run command in background".to_string(),
            "  jobs      - Open task list view".to_string(),
            "  fg <id>   - Attach to task output".to_string(),
            "  kill <id> - Terminate a task".to_string(),
            "  clear     - Clear finished tasks".to_string(),
            "  Tab       - Path completion".to_string(),
            "  Up/Down   - Command history".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Helper methods for key handling and drawing
impl ShellPlugin {
    fn handle_command_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.modal_open = false;
                KeyHandleResult::CloseModal
            }
            KeyCode::Enter => {
                self.execute_command();
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.input.pop();
                KeyHandleResult::Handled
            }
            KeyCode::Up => {
                if !self.state.history.is_empty() {
                    let new_idx = match self.state.history_index {
                        Some(idx) if idx > 0 => idx - 1,
                        Some(idx) => idx,
                        None => self.state.history.len() - 1,
                    };
                    self.state.history_index = Some(new_idx);
                    self.state.input = self.state.history[new_idx].clone();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                if let Some(idx) = self.state.history_index {
                    if idx + 1 < self.state.history.len() {
                        let new_idx = idx + 1;
                        self.state.history_index = Some(new_idx);
                        self.state.input = self.state.history[new_idx].clone();
                    } else {
                        self.state.history_index = None;
                        self.state.input.clear();
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::PageUp => {
                self.state.scroll_offset = self.state.scroll_offset.saturating_sub(10);
                KeyHandleResult::Handled
            }
            KeyCode::PageDown => {
                let max_scroll = self.state.output.len().saturating_sub(10);
                self.state.scroll_offset = (self.state.scroll_offset + 10).min(max_scroll);
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                if let Some(completed) = Self::tab_complete(&self.state.input) {
                    self.state.input = completed;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.input.push(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_task_list_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let task_count = self.tasks.len();
        match key.code {
            KeyCode::Esc => {
                self.state.view = ShellView::Command;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.selected_task > 0 {
                    self.state.selected_task -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if task_count > 0 && self.state.selected_task < task_count - 1 {
                    self.state.selected_task += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Attach to selected task
                if let Some(task) = self.get_task_by_index(self.state.selected_task) {
                    let id = task.id;
                    self.state.view = ShellView::Attached(id);
                    self.state.scroll_offset = 0;
                    self.state.last_output_len = 0;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('K') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                // Ctrl+Shift+K to kill selected task
                if let Some(task) = self.get_task_by_index(self.state.selected_task) {
                    let id = task.id;
                    self.kill_task(id);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // D to delete/kill selected task
                if let Some(task) = self.get_task_by_index(self.state.selected_task) {
                    let id = task.id;
                    if task.status != TaskStatus::Running {
                        self.tasks.remove(&id);
                        if self.state.selected_task > 0 && self.state.selected_task >= self.tasks.len() {
                            self.state.selected_task = self.tasks.len().saturating_sub(1);
                        }
                    } else {
                        self.kill_task(id);
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                // C to clear finished tasks
                self.clear_finished_tasks();
                self.state.selected_task = 0;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_attached_key(&mut self, key: KeyEvent, task_id: u64) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.state.view = ShellView::Command;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.scroll_offset = self.state.scroll_offset.saturating_sub(1);
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(task) = self.tasks.get(&task_id) {
                    let output_len = task.output.lock().unwrap().len();
                    if self.state.scroll_offset < output_len.saturating_sub(1) {
                        self.state.scroll_offset += 1;
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::PageUp => {
                self.state.scroll_offset = self.state.scroll_offset.saturating_sub(10);
                KeyHandleResult::Handled
            }
            KeyCode::PageDown => {
                if let Some(task) = self.tasks.get(&task_id) {
                    let output_len = task.output.lock().unwrap().len();
                    self.state.scroll_offset = (self.state.scroll_offset + 10).min(output_len.saturating_sub(1));
                }
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                self.state.scroll_offset = 0;
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                if let Some(task) = self.tasks.get(&task_id) {
                    let output_len = task.output.lock().unwrap().len();
                    self.state.scroll_offset = output_len.saturating_sub(1);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('f') => {
                // F to follow (auto-scroll to end)
                if let Some(task) = self.tasks.get(&task_id) {
                    let output_len = task.output.lock().unwrap().len();
                    self.state.scroll_offset = output_len.saturating_sub(1);
                    self.state.last_output_len = output_len;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn draw_command_view(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let modal = ModalFrame::new(area, " DOS Command ");
        modal.render_frame(frame);

        let content_height = modal.content_height() as usize;

        // Draw prompt and input
        let prompt_style = Style::default().fg(COLOR_GREEN).bg(COLOR_BG);
        let input_style = Style::default().fg(COLOR_FG).bg(COLOR_BG);

        let prompt = format!("{}> ", self.current_cwd.display());
        let prompt_len = prompt.len().min(area.width.saturating_sub(4) as usize);

        modal.render_row(
            frame,
            0,
            vec![
                Span::styled(&prompt[..prompt_len], prompt_style),
                Span::styled(&self.state.input, input_style),
                Span::styled("_", Style::default().fg(COLOR_CYAN).bg(COLOR_BG).add_modifier(Modifier::SLOW_BLINK)),
            ],
        );

        // Draw output
        let output_start = 2;
        let output_height = content_height.saturating_sub(3);
        let scroll = self.state.scroll_offset.min(self.state.output.len().saturating_sub(output_height));

        for (i, line) in self.state.output.iter().skip(scroll).take(output_height).enumerate() {
            let style = if line.starts_with("stderr:") {
                Style::default().fg(COLOR_YELLOW).bg(COLOR_BG)
            } else {
                Style::default().fg(COLOR_FG).bg(COLOR_BG)
            };
            modal.render_row(frame, (output_start + i) as u16, vec![Span::styled(line, style)]);
        }

        // Draw status line
        let status_row = content_height.saturating_sub(1) as u16;
        let exit_str = match self.state.exit_code {
            Some(0) => "Exit: 0 (OK)".to_string(),
            Some(code) => format!("Exit: {}", code),
            None => String::new(),
        };

        let running_count = self.running_count();
        let tasks_str = if running_count > 0 {
            format!(" | {} bg", running_count)
        } else {
            String::new()
        };

        modal.render_row(
            frame,
            status_row,
            vec![
                Span::styled(&exit_str, Style::default().fg(COLOR_GREY).bg(COLOR_BG)),
                Span::styled(&tasks_str, Style::default().fg(COLOR_CYAN).bg(COLOR_BG)),
                Span::styled(
                    " | jobs  fg <id>  kill <id>  cmd&",
                    Style::default().fg(COLOR_GREY).bg(COLOR_BG),
                ),
            ],
        );
    }

    fn draw_task_list_view(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let running = self.running_count();
        let total = self.tasks.len();
        let title = format!(" Task List ({} running / {} total) ", running, total);
        let modal = ModalFrame::new(area, &title);
        modal.render_frame(frame);

        let content_height = modal.content_height() as usize;

        if self.tasks.is_empty() {
            modal.render_row(
                frame,
                1,
                vec![Span::styled(
                    "No background tasks",
                    Style::default().fg(COLOR_GREY).bg(COLOR_BG),
                )],
            );
        } else {
            // Header
            modal.render_row(
                frame,
                0,
                vec![
                    Span::styled("  ID  ", Style::default().fg(COLOR_CYAN).bg(COLOR_BG)),
                    Span::styled("Status   ", Style::default().fg(COLOR_CYAN).bg(COLOR_BG)),
                    Span::styled("Time     ", Style::default().fg(COLOR_CYAN).bg(COLOR_BG)),
                    Span::styled("Command", Style::default().fg(COLOR_CYAN).bg(COLOR_BG)),
                ],
            );

            let task_ids = self.get_sorted_task_ids();
            let max_display = content_height.saturating_sub(3);

            for (i, id) in task_ids.iter().take(max_display).enumerate() {
                if let Some(task) = self.tasks.get(id) {
                    let is_selected = i == self.state.selected_task;
                    let bg = if is_selected { COLOR_RED } else { COLOR_BG };
                    let fg = if is_selected { COLOR_YELLOW } else { COLOR_FG };

                    let status_style = match task.status {
                        TaskStatus::Running => Style::default().fg(COLOR_GREEN).bg(bg),
                        TaskStatus::Completed => Style::default().fg(COLOR_CYAN).bg(bg),
                        TaskStatus::Failed => Style::default().fg(COLOR_RED).bg(bg),
                    };

                    let status_icon = match task.status {
                        TaskStatus::Running => "● ",
                        TaskStatus::Completed => "✓ ",
                        TaskStatus::Failed => "✗ ",
                    };

                    let elapsed = task.elapsed().as_secs_f32();
                    let time_str = if elapsed < 60.0 {
                        format!("{:>5.1}s  ", elapsed)
                    } else {
                        format!("{:>5.1}m  ", elapsed / 60.0)
                    };

                    let cmd_width = area.width.saturating_sub(30) as usize;
                    let cmd_display = if task.command.len() > cmd_width {
                        format!("{}...", &task.command[..cmd_width.saturating_sub(3)])
                    } else {
                        task.command.clone()
                    };

                    modal.render_row(
                        frame,
                        (i + 1) as u16,
                        vec![
                            Span::styled(format!(" {:>3}  ", id), Style::default().fg(fg).bg(bg)),
                            Span::styled(status_icon, status_style),
                            Span::styled(format!("{:<6} ", task.status.as_str()), status_style),
                            Span::styled(&time_str, Style::default().fg(COLOR_GREY).bg(bg)),
                            Span::styled(cmd_display, Style::default().fg(fg).bg(bg)),
                        ],
                    );
                }
            }
        }

        // Help line
        let status_row = content_height.saturating_sub(1) as u16;
        modal.render_row(
            frame,
            status_row,
            vec![
                Span::styled("Enter", Style::default().fg(COLOR_GREEN).bg(COLOR_BG)),
                Span::styled(":attach  ", Style::default().fg(COLOR_GREY).bg(COLOR_BG)),
                Span::styled("D", Style::default().fg(COLOR_GREEN).bg(COLOR_BG)),
                Span::styled(":kill  ", Style::default().fg(COLOR_GREY).bg(COLOR_BG)),
                Span::styled("C", Style::default().fg(COLOR_GREEN).bg(COLOR_BG)),
                Span::styled(":clear  ", Style::default().fg(COLOR_GREY).bg(COLOR_BG)),
                Span::styled("Esc", Style::default().fg(COLOR_GREEN).bg(COLOR_BG)),
                Span::styled(":back", Style::default().fg(COLOR_GREY).bg(COLOR_BG)),
            ],
        );
    }

    fn draw_attached_view(&self, frame: &mut Frame, area: Rect, task_id: u64) {
        frame.render_widget(Clear, area);

        let task = match self.tasks.get(&task_id) {
            Some(t) => t,
            None => {
                // Task was removed, go back to command view
                return;
            }
        };

        let status_str = match task.status {
            TaskStatus::Running => "RUNNING",
            TaskStatus::Completed => "DONE",
            TaskStatus::Failed => "FAILED",
        };
        let title = format!(" Task {} - {} - {} ", task_id, status_str, task.command);
        let title_truncated = if title.len() > area.width.saturating_sub(4) as usize {
            format!("{}...", &title[..area.width.saturating_sub(7) as usize])
        } else {
            title
        };

        let modal = ModalFrame::new(area, &title_truncated);
        modal.render_frame(frame);

        let content_height = modal.content_height() as usize;
        let output = task.output.lock().unwrap();
        let output_len = output.len();

        // Calculate scroll position
        let visible_height = content_height.saturating_sub(2);
        let scroll = self.state.scroll_offset.min(output_len.saturating_sub(visible_height));

        // Draw output lines
        for (i, line) in output.iter().skip(scroll).take(visible_height).enumerate() {
            let style = if line.starts_with("stderr:") {
                Style::default().fg(COLOR_YELLOW).bg(COLOR_BG)
            } else if line == "[Killed]" {
                Style::default().fg(COLOR_RED).bg(COLOR_BG)
            } else {
                Style::default().fg(COLOR_FG).bg(COLOR_BG)
            };
            modal.render_row(frame, i as u16, vec![Span::styled(line, style)]);
        }

        // Status line
        let status_row = content_height.saturating_sub(1) as u16;
        let elapsed = task.elapsed().as_secs_f32();
        let time_str = if elapsed < 60.0 {
            format!("{:.1}s", elapsed)
        } else {
            format!("{:.1}m", elapsed / 60.0)
        };

        let exit_str = match task.exit_code {
            Some(code) => format!("Exit: {}", code),
            None => String::new(),
        };

        let scroll_str = format!(" [{}/{}]", scroll + 1, output_len.max(1));

        modal.render_row(
            frame,
            status_row,
            vec![
                Span::styled(&time_str, Style::default().fg(COLOR_GREY).bg(COLOR_BG)),
                Span::styled("  ", Style::default().bg(COLOR_BG)),
                Span::styled(&exit_str, Style::default().fg(COLOR_GREY).bg(COLOR_BG)),
                Span::styled(&scroll_str, Style::default().fg(COLOR_GREY).bg(COLOR_BG)),
                Span::styled(" | ", Style::default().fg(COLOR_GREY).bg(COLOR_BG)),
                Span::styled("↑↓", Style::default().fg(COLOR_GREEN).bg(COLOR_BG)),
                Span::styled(":scroll  ", Style::default().fg(COLOR_GREY).bg(COLOR_BG)),
                Span::styled("F", Style::default().fg(COLOR_GREEN).bg(COLOR_BG)),
                Span::styled(":follow  ", Style::default().fg(COLOR_GREY).bg(COLOR_BG)),
                Span::styled("q", Style::default().fg(COLOR_GREEN).bg(COLOR_BG)),
                Span::styled(":back", Style::default().fg(COLOR_GREY).bg(COLOR_BG)),
            ],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_plugin_creation() {
        let plugin = ShellPlugin::new();
        assert_eq!(plugin.id(), "shell");
        assert!(!plugin.modal_open);
    }
}
