//! Shell plugin state types
//!
//! State types for shell/DOS command execution.

use super::pty::PtySession;
#[allow(unused_imports)]
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

/// Menu items for shell plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellMenuItem {
    /// Single command execution
    #[default]
    Command,
    /// Interactive shell (PTY)
    Interactive,
    /// Background jobs list
    Jobs,
    /// Telnet client
    Telnet,
}

impl ShellMenuItem {
    /// All menu items in order
    pub const ALL: [ShellMenuItem; 4] = [
        ShellMenuItem::Command,
        ShellMenuItem::Interactive,
        ShellMenuItem::Jobs,
        ShellMenuItem::Telnet,
    ];

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            ShellMenuItem::Command => "Command",
            ShellMenuItem::Interactive => "Interactive Shell",
            ShellMenuItem::Jobs => "Background Jobs",
            ShellMenuItem::Telnet => "Telnet",
        }
    }

    /// Get keyboard shortcut
    pub fn key(&self) -> char {
        match self {
            ShellMenuItem::Command => 'C',
            ShellMenuItem::Interactive => 'I',
            ShellMenuItem::Jobs => 'J',
            ShellMenuItem::Telnet => 'T',
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            ShellMenuItem::Command => "Execute a single shell command",
            ShellMenuItem::Interactive => "Launch interactive shell (bash/zsh)",
            ShellMenuItem::Jobs => "View and manage background tasks",
            ShellMenuItem::Telnet => "Connect to remote telnet servers",
        }
    }
}

/// View mode for the shell modal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellView {
    /// Menu selection (default entry point)
    #[default]
    Menu,
    /// Command input mode
    Command,
    /// Interactive shell (PTY)
    Interactive,
    /// Task list view
    TaskList,
    /// Attached to a specific task (watching output)
    Attached(u64),
    /// Telnet submenu
    TelnetMenu,
    /// Telnet connection form
    TelnetForm,
    /// Telnet connecting (loading)
    TelnetConnecting,
    /// Telnet active session
    TelnetConnected,
    /// Telnet connection history
    TelnetHistory,
    /// Telnet error display
    TelnetError,
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
    /// Selected menu item (for Menu view)
    pub menu_selected: usize,
    /// Selected task in task list
    pub selected_task: usize,
    /// Last output length for attached view (for auto-scroll)
    pub last_output_len: usize,
    /// Last terminal size for interactive view
    pub last_size: (u16, u16),
}

/// Interactive shell state (separate because PtySession is not Clone)
pub struct InteractiveState {
    /// The PTY session
    pub session: PtySession,
    /// Terminal size
    pub size: (u16, u16),
}

// ============================================================================
// Telnet types
// ============================================================================

/// Menu items for telnet submenu
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TelnetMenuItem {
    /// New connection
    #[default]
    Connect,
    /// Connection history
    History,
}

impl TelnetMenuItem {
    /// All menu items in order
    pub const ALL: [TelnetMenuItem; 2] = [
        TelnetMenuItem::Connect,
        TelnetMenuItem::History,
    ];

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            TelnetMenuItem::Connect => "Connect",
            TelnetMenuItem::History => "History",
        }
    }

    /// Get keyboard shortcut
    pub fn key(&self) -> char {
        match self {
            TelnetMenuItem::Connect => 'C',
            TelnetMenuItem::History => 'H',
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            TelnetMenuItem::Connect => "Connect to a telnet server",
            TelnetMenuItem::History => "View recent connections",
        }
    }
}

/// A telnet connection history entry
#[derive(Debug, Clone)]
pub struct TelnetHistoryEntry {
    pub host: String,
    pub port: u16,
    pub connected_at: std::time::SystemTime,
}

/// Telnet-specific state
#[derive(Debug, Clone, Default)]
pub struct TelnetState {
    /// Selected menu item (for TelnetMenu view)
    pub menu_selected: usize,
    /// Host input for connection form
    pub host_input: String,
    /// Port input for connection form
    pub port_input: String,
    /// Which field is active (0 = host, 1 = port)
    pub input_field: usize,
    /// Connection history
    pub history: Vec<TelnetHistoryEntry>,
    /// Selected history entry
    pub history_selected: usize,
    /// Error message (for TelnetError view)
    pub error_message: Option<String>,
}
