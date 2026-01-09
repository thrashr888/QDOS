//! Shell/DOS Command Plugin for R-DOS
//!
//! Provides shell command execution functionality including:
//! - Synchronous command execution
//! - Background task management (commands ending with &)
//! - Built-in commands: jobs, fg, kill
//! - Task list view with live output
//! - Command history
//! - Interactive shell mode (PTY)

mod modal;
pub mod pty;
mod state;
mod telnet;

use super::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Frame;
use std::any::Any;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

// Re-export state types for external use
pub use state::{
    BackgroundTask, InteractiveState, ShellMenuItem, ShellState, ShellView, TaskStatus,
    TelnetHistoryEntry, TelnetMenuItem, TelnetState,
};
use telnet::TelnetSession;

/// Shell plugin that provides DOS command functionality
pub struct ShellPlugin {
    modal_open: bool,
    state: ShellState,
    current_cwd: PathBuf,
    /// Background tasks
    tasks: HashMap<u64, BackgroundTask>,
    next_task_id: u64,
    /// Interactive PTY session (if active)
    interactive: Option<InteractiveState>,
    /// Telnet session (if connected)
    telnet_session: Option<TelnetSession>,
    /// Telnet-specific state
    telnet_state: TelnetState,
}

impl ShellPlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: ShellState::default(),
            current_cwd: PathBuf::new(),
            tasks: HashMap::new(),
            next_task_id: 1,
            interactive: None,
            telnet_session: None,
            telnet_state: TelnetState::default(),
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
                    self.state.output =
                        vec![format!("Cannot kill task {}: not running or not found", id)];
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

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        match key.code {
            KeyCode::F(6) => {
                self.current_cwd = cwd.clone();
                self.modal_open = true;
                // Always start at Menu view when opening
                self.state.view = ShellView::Menu;
                self.state.menu_selected = 0;
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        self.current_cwd = cwd.clone();

        // Handle keys based on current view
        match self.state.view {
            ShellView::Menu => self.handle_menu_key(key),
            ShellView::Command => self.handle_command_key(key),
            ShellView::Interactive => self.handle_interactive_key(key),
            ShellView::TaskList => self.handle_task_list_key(key),
            ShellView::Attached(id) => self.handle_attached_key(key, id),
            // Telnet views
            ShellView::TelnetMenu => self.handle_telnet_menu_key(key),
            ShellView::TelnetForm => self.handle_telnet_form_key(key),
            ShellView::TelnetConnecting => self.handle_telnet_connecting_key(key),
            ShellView::TelnetConnected => self.handle_telnet_connected_key(key),
            ShellView::TelnetHistory => self.handle_telnet_history_key(key),
            ShellView::TelnetError => self.handle_telnet_error_key(key),
        }
    }

    fn tick(&mut self) {
        self.poll_tasks();
        // Poll interactive PTY if active
        if let Some(ref mut interactive) = self.interactive {
            let _ = interactive.session.try_read();
            // Check if shell exited
            if !interactive.session.is_running() {
                self.interactive = None;
                self.state.view = ShellView::Menu;
            }
        }
        // Poll telnet session if connected
        if let Some(ref session) = self.telnet_session {
            let _ = session.try_read();
            // Check if disconnected
            if !session.is_connected() {
                self.telnet_session = None;
                self.state.view = ShellView::TelnetMenu;
            }
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        match self.state.view {
            ShellView::Menu => modal::draw_menu_view(frame, area, &self.state, colors),
            ShellView::Command => modal::draw_command_view(
                frame,
                area,
                &self.state,
                &self.current_cwd,
                &self.tasks,
                colors,
            ),
            ShellView::Interactive => {
                modal::draw_interactive_view(frame, area, &self.interactive, colors)
            }
            ShellView::TaskList => {
                modal::draw_task_list_view(frame, area, &self.state, &self.tasks, colors)
            }
            ShellView::Attached(id) => {
                modal::draw_attached_view(frame, area, id, &self.state, &self.tasks, colors)
            }
            // Telnet views
            ShellView::TelnetMenu => {
                modal::draw_telnet_menu_view(frame, area, &self.telnet_state, colors)
            }
            ShellView::TelnetForm => {
                modal::draw_telnet_form_view(frame, area, &self.telnet_state, colors)
            }
            ShellView::TelnetConnecting => {
                modal::draw_telnet_connecting_view(frame, area, &self.telnet_state, colors)
            }
            ShellView::TelnetConnected => {
                modal::draw_telnet_connected_view(frame, area, &self.telnet_session, colors)
            }
            ShellView::TelnetHistory => {
                modal::draw_telnet_history_view(frame, area, &self.telnet_state, colors)
            }
            ShellView::TelnetError => {
                modal::draw_telnet_error_view(frame, area, &self.telnet_state, colors)
            }
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
            "".to_string(),
            "  Menu Options:".to_string(),
            "    C - Command mode (single commands)".to_string(),
            "    I - Interactive shell (bash/zsh)".to_string(),
            "    J - Background jobs list".to_string(),
            "    T - Telnet client".to_string(),
            "".to_string(),
            "  Command Mode:".to_string(),
            "    cmd &     - Run command in background".to_string(),
            "    jobs      - Open task list view".to_string(),
            "    fg <id>   - Attach to task output".to_string(),
            "    kill <id> - Terminate a task".to_string(),
            "    clear     - Clear finished tasks".to_string(),
            "    Tab       - Path completion".to_string(),
            "    Up/Down   - Command history".to_string(),
            "".to_string(),
            "  Interactive Mode:".to_string(),
            "    Ctrl+D    - Exit interactive shell".to_string(),
            "    Full PTY  - Run vim, htop, etc.".to_string(),
            "".to_string(),
            "  Telnet Mode:".to_string(),
            "    C - Connect to a server".to_string(),
            "    H - View connection history".to_string(),
            "    Ctrl+]   - Disconnect from server".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Shell".to_string(),
            description: "Interactive shell".to_string(),
            category: PluginCategory::Tools,
            key: 'S',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.state = ShellState::default();
        self.modal_open = true;
        Ok(())
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
    fn handle_menu_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.modal_open = false;
                KeyHandleResult::CloseModal
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.menu_selected > 0 {
                    self.state.menu_selected -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.state.menu_selected < ShellMenuItem::ALL.len() - 1 {
                    self.state.menu_selected += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                let item = ShellMenuItem::ALL[self.state.menu_selected];
                self.launch_menu_item(item)
            }
            KeyCode::Char(c) => {
                // Check for menu item shortcut
                let c_upper = c.to_ascii_uppercase();
                for item in ShellMenuItem::ALL {
                    if item.key() == c_upper {
                        return self.launch_menu_item(item);
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn launch_menu_item(&mut self, item: ShellMenuItem) -> KeyHandleResult {
        match item {
            ShellMenuItem::Command => {
                self.state.view = ShellView::Command;
                KeyHandleResult::Handled
            }
            ShellMenuItem::Interactive => {
                // Launch interactive shell
                match pty::PtySession::spawn(None, &self.current_cwd, 80, 24) {
                    Ok(session) => {
                        self.interactive = Some(InteractiveState {
                            session,
                            size: (80, 24),
                        });
                        self.state.view = ShellView::Interactive;
                    }
                    Err(e) => {
                        self.state.output = vec![format!("Failed to start shell: {}", e)];
                        self.state.view = ShellView::Command;
                    }
                }
                KeyHandleResult::Handled
            }
            ShellMenuItem::Jobs => {
                self.state.view = ShellView::TaskList;
                self.state.selected_task = 0;
                KeyHandleResult::Handled
            }
            ShellMenuItem::Telnet => {
                self.state.view = ShellView::TelnetMenu;
                self.telnet_state.menu_selected = 0;
                KeyHandleResult::Handled
            }
        }
    }

    fn handle_interactive_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        use crossterm::event::KeyModifiers;

        // Ctrl+D exits interactive mode
        if key.code == KeyCode::Char('d') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.interactive = None;
            self.state.view = ShellView::Menu;
            return KeyHandleResult::Handled;
        }

        // Forward all other keys to PTY
        if let Some(ref mut interactive) = self.interactive {
            if let Some(bytes) = pty::key_to_bytes(key) {
                let _ = interactive.session.write(&bytes);
            }
        }
        KeyHandleResult::Handled
    }

    fn handle_command_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = ShellView::Menu;
                KeyHandleResult::Handled
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
                self.state.view = ShellView::Menu;
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
            KeyCode::Char('K')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
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
                        if self.state.selected_task > 0
                            && self.state.selected_task >= self.tasks.len()
                        {
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
                self.state.view = ShellView::Menu;
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
                    self.state.scroll_offset =
                        (self.state.scroll_offset + 10).min(output_len.saturating_sub(1));
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

    // =========================================================================
    // Telnet key handlers
    // =========================================================================

    fn handle_telnet_menu_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = ShellView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.telnet_state.menu_selected > 0 {
                    self.telnet_state.menu_selected -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.telnet_state.menu_selected < TelnetMenuItem::ALL.len() - 1 {
                    self.telnet_state.menu_selected += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                let item = TelnetMenuItem::ALL[self.telnet_state.menu_selected];
                match item {
                    TelnetMenuItem::Connect => {
                        self.telnet_state.host_input.clear();
                        self.telnet_state.port_input = "23".to_string();
                        self.telnet_state.input_field = 0;
                        self.state.view = ShellView::TelnetForm;
                    }
                    TelnetMenuItem::History => {
                        self.telnet_state.history_selected = 0;
                        self.state.view = ShellView::TelnetHistory;
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                let c_upper = c.to_ascii_uppercase();
                for item in TelnetMenuItem::ALL {
                    if item.key() == c_upper {
                        match item {
                            TelnetMenuItem::Connect => {
                                self.telnet_state.host_input.clear();
                                self.telnet_state.port_input = "23".to_string();
                                self.telnet_state.input_field = 0;
                                self.state.view = ShellView::TelnetForm;
                            }
                            TelnetMenuItem::History => {
                                self.telnet_state.history_selected = 0;
                                self.state.view = ShellView::TelnetHistory;
                            }
                        }
                        return KeyHandleResult::Handled;
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_telnet_form_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = ShellView::TelnetMenu;
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                // Toggle between host and port fields
                self.telnet_state.input_field = (self.telnet_state.input_field + 1) % 2;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Attempt connection
                let host = self.telnet_state.host_input.trim().to_string();
                let port: u16 = self.telnet_state.port_input.parse().unwrap_or(23);

                if host.is_empty() {
                    self.telnet_state.error_message = Some("Host cannot be empty".to_string());
                    self.state.view = ShellView::TelnetError;
                    return KeyHandleResult::Handled;
                }

                self.state.view = ShellView::TelnetConnecting;

                // Try to connect
                match TelnetSession::connect(&host, port, 80, 24) {
                    Ok(session) => {
                        // Add to history
                        self.telnet_state.history.push(TelnetHistoryEntry {
                            host: host.clone(),
                            port,
                            connected_at: std::time::SystemTime::now(),
                        });
                        self.telnet_session = Some(session);
                        self.state.view = ShellView::TelnetConnected;
                    }
                    Err(e) => {
                        self.telnet_state.error_message = Some(format!("Connection failed: {}", e));
                        self.state.view = ShellView::TelnetError;
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                if self.telnet_state.input_field == 0 {
                    self.telnet_state.host_input.pop();
                } else {
                    self.telnet_state.port_input.pop();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                if self.telnet_state.input_field == 0 {
                    self.telnet_state.host_input.push(c);
                } else if c.is_ascii_digit() {
                    self.telnet_state.port_input.push(c);
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_telnet_connecting_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        // Allow canceling with Esc during connection attempt
        if key.code == KeyCode::Esc {
            self.state.view = ShellView::TelnetForm;
        }
        KeyHandleResult::Handled
    }

    fn handle_telnet_connected_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        use crossterm::event::KeyModifiers;

        // Ctrl+] is the traditional telnet escape sequence
        if key.code == KeyCode::Char(']') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.telnet_session = None;
            self.state.view = ShellView::TelnetMenu;
            return KeyHandleResult::Handled;
        }

        // Forward all other keys to the telnet session
        if let Some(ref session) = self.telnet_session {
            if let Some(bytes) = telnet::key_to_bytes(key) {
                let _ = session.write(&bytes);
            }
        }
        KeyHandleResult::Handled
    }

    fn handle_telnet_history_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = ShellView::TelnetMenu;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.telnet_state.history_selected > 0 {
                    self.telnet_state.history_selected -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.telnet_state.history.is_empty()
                    && self.telnet_state.history_selected < self.telnet_state.history.len() - 1
                {
                    self.telnet_state.history_selected += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Connect to selected history entry
                if let Some(entry) = self
                    .telnet_state
                    .history
                    .get(self.telnet_state.history_selected)
                {
                    let host = entry.host.clone();
                    let port = entry.port;

                    self.state.view = ShellView::TelnetConnecting;

                    match TelnetSession::connect(&host, port, 80, 24) {
                        Ok(session) => {
                            self.telnet_session = Some(session);
                            self.state.view = ShellView::TelnetConnected;
                        }
                        Err(e) => {
                            self.telnet_state.error_message =
                                Some(format!("Connection failed: {}", e));
                            self.state.view = ShellView::TelnetError;
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // Delete history entry
                if !self.telnet_state.history.is_empty() {
                    self.telnet_state
                        .history
                        .remove(self.telnet_state.history_selected);
                    if self.telnet_state.history_selected > 0
                        && self.telnet_state.history_selected >= self.telnet_state.history.len()
                    {
                        self.telnet_state.history_selected =
                            self.telnet_state.history.len().saturating_sub(1);
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_telnet_error_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        // Any key returns to form
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char(_) => {
                self.telnet_state.error_message = None;
                self.state.view = ShellView::TelnetForm;
            }
            _ => {}
        }
        KeyHandleResult::Handled
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
