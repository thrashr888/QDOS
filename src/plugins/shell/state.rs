//! Shell plugin state types
//!
//! State types for shell/DOS command execution.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
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
    pub reader_handle: Option<JoinHandle<i32>>,
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
