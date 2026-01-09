//! Proc Plugin for R-DOS
//!
//! System/process monitoring plugin similar to top/htop/Activity Monitor.
//! Triggered via F12, provides multiple views for CPU, Memory, Disk, Network.

mod modal;
mod state;

use super::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;
use sysinfo::{Disks, Networks, Pid, ProcessStatus, System};

// Re-export state types for external use
pub use state::{
    DiskInfo, NetworkInfo, ProcSort, ProcState, ProcView, ProcessDetailInfo, ProcessInfo,
};

/// Proc plugin for system/process monitoring
pub struct ProcPlugin {
    /// Whether modal is open
    modal_open: bool,
    /// Current state
    state: ProcState,
    /// System info handle
    system: System,
    /// Disk info handle
    disks: Disks,
    /// Network info handle
    networks: Networks,
}

impl ProcPlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: ProcState {
                auto_refresh: true,
                last_refresh: std::time::Instant::now(),
                ..Default::default()
            },
            system: System::new_all(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
        }
    }

    /// Refresh process list and system info
    fn refresh(&mut self) {
        // Save selected PID before refresh (for process views)
        if (self.state.view == ProcView::Cpu || self.state.view == ProcView::Memory)
            && self.state.selected < self.state.processes.len()
        {
            self.state.selected_pid = Some(self.state.processes[self.state.selected].pid);
        }

        self.system.refresh_all();
        self.disks.refresh(true);
        self.networks.refresh(true);

        // System totals
        self.state.total_memory = self.system.total_memory();
        self.state.used_memory = self.system.used_memory();
        self.state.cpu_count = self.system.cpus().len();
        self.state.cpu_usage = self.system.global_cpu_usage();

        // Collect process info
        self.state.processes.clear();
        for (pid, process) in self.system.processes() {
            let status = match process.status() {
                ProcessStatus::Run => "R",
                ProcessStatus::Sleep => "S",
                ProcessStatus::Idle => "I",
                ProcessStatus::Zombie => "Z",
                ProcessStatus::Stop => "T",
                _ => "?",
            };

            let disk_usage = process.disk_usage();

            self.state.processes.push(ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage(),
                cpu_time_ms: process.accumulated_cpu_time(),
                memory: process.memory(),
                status: status.to_string(),
                user: process
                    .user_id()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                bytes_read: disk_usage.total_read_bytes,
                bytes_written: disk_usage.total_written_bytes,
            });
        }

        // Sort processes based on current sort mode
        self.sort_processes();

        // Restore selection to same PID if it still exists
        if let Some(target_pid) = self.state.selected_pid {
            if let Some(idx) = self
                .state
                .processes
                .iter()
                .position(|p| p.pid == target_pid)
            {
                self.state.selected = idx;
                // Adjust scroll offset to keep selection visible
                if self.state.selected < self.state.scroll_offset {
                    self.state.scroll_offset = self.state.selected;
                }
            }
        }

        // Collect disk info
        self.state.disks.clear();
        for disk in self.disks.list() {
            let total = disk.total_space();
            let available = disk.available_space();
            self.state.disks.push(DiskInfo {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total,
                available,
                used: total.saturating_sub(available),
                file_system: disk.file_system().to_string_lossy().to_string(),
            });
        }

        // Collect network info
        self.state.networks.clear();
        for (name, data) in self.networks.list() {
            self.state.networks.push(NetworkInfo {
                name: name.clone(),
                received: data.total_received(),
                transmitted: data.total_transmitted(),
                packets_in: data.total_packets_received(),
                packets_out: data.total_packets_transmitted(),
            });
        }

        self.state.last_refresh = std::time::Instant::now();
    }

    /// Sort processes based on current sort mode
    fn sort_processes(&mut self) {
        match self.state.sort {
            ProcSort::CpuDesc => self
                .state
                .processes
                .sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap()),
            ProcSort::CpuAsc => self
                .state
                .processes
                .sort_by(|a, b| a.cpu_usage.partial_cmp(&b.cpu_usage).unwrap()),
            ProcSort::MemDesc => self.state.processes.sort_by(|a, b| b.memory.cmp(&a.memory)),
            ProcSort::MemAsc => self.state.processes.sort_by(|a, b| a.memory.cmp(&b.memory)),
            ProcSort::PidAsc => self.state.processes.sort_by(|a, b| a.pid.cmp(&b.pid)),
            ProcSort::PidDesc => self.state.processes.sort_by(|a, b| b.pid.cmp(&a.pid)),
            ProcSort::NameAsc => self.state.processes.sort_by(|a, b| a.name.cmp(&b.name)),
            ProcSort::NameDesc => self.state.processes.sort_by(|a, b| b.name.cmp(&a.name)),
        }
    }

    /// Open the modal
    pub fn open_modal(&mut self) {
        self.modal_open = true;
        self.refresh();
    }

    /// Close the modal
    fn close_modal(&mut self) {
        self.modal_open = false;
        self.state.selected = 0;
        self.state.scroll_offset = 0;
    }

    /// Kill selected process
    fn kill_selected(&mut self) {
        if self.state.view == ProcView::Cpu || self.state.view == ProcView::Memory {
            if let Some(proc) = self.state.processes.get(self.state.selected) {
                let pid = Pid::from_u32(proc.pid);
                if let Some(process) = self.system.process(pid) {
                    process.kill();
                    self.refresh();
                }
            }
        }
    }

    /// Show process detail overlay
    fn show_process_info(&mut self) {
        // Only show detail for process views
        if self.state.view != ProcView::Cpu
            && self.state.view != ProcView::Memory
            && self.state.view != ProcView::Disk
        {
            return;
        }

        if let Some(proc) = self.state.processes.get(self.state.selected) {
            let pid = Pid::from_u32(proc.pid);
            if let Some(process) = self.system.process(pid) {
                self.state.detail_info = Some(ProcessDetailInfo {
                    pid: proc.pid,
                    name: proc.name.clone(),
                    cmd: process
                        .cmd()
                        .iter()
                        .map(|s| s.to_string_lossy().to_string())
                        .collect(),
                    cwd: process
                        .cwd()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    parent_pid: process.parent().map(|p| p.as_u32()),
                    start_time: process.start_time(),
                    cpu_usage: proc.cpu_usage,
                    memory: proc.memory,
                    status: proc.status.clone(),
                    user: proc.user.clone(),
                });
                self.state.show_detail = true;
                self.state.detail_scroll = 0;
            }
        }
    }

    /// Close process detail overlay
    fn close_process_info(&mut self) {
        self.state.show_detail = false;
        self.state.detail_info = None;
        self.state.detail_scroll = 0;
    }

    /// Get current list length based on view
    fn current_list_len(&self) -> usize {
        match self.state.view {
            ProcView::Cpu | ProcView::Memory | ProcView::Disk => self.state.processes.len(),
            ProcView::Network => self.state.networks.len(),
        }
    }
}

impl Default for ProcPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ProcPlugin {
    fn id(&self) -> &str {
        "proc"
    }

    fn name(&self) -> &str {
        "Proc"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true // Always available
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Proc".to_string(),
            key: 'P', // F12 shown as P in menu
            description: "System/process monitor".to_string(),
            priority: 110, // After other features
        })
    }

    fn handle_global_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Proc is accessed via F12 Apps launcher, no direct global key
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Handle detail overlay if open
        if self.state.show_detail {
            return match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.close_process_info();
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.state.detail_scroll > 0 {
                        self.state.detail_scroll -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.state.detail_scroll += 1;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            };
        }

        match key.code {
            KeyCode::Esc => {
                self.close_modal();
                KeyHandleResult::CloseModal
            }
            KeyCode::Enter | KeyCode::Char('i') | KeyCode::Char('I') => {
                self.show_process_info();
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.selected > 0 {
                    self.state.selected -= 1;
                    // Adjust scroll offset
                    if self.state.selected < self.state.scroll_offset {
                        self.state.scroll_offset = self.state.selected;
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.current_list_len().saturating_sub(1);
                if self.state.selected < max {
                    self.state.selected += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                // Switch view and reset selection
                self.state.view = self.state.view.next();
                self.state.selected = 0;
                self.state.scroll_offset = 0;
                KeyHandleResult::Handled
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                // Only cycle sort in process views
                if self.state.view == ProcView::Cpu || self.state.view == ProcView::Memory {
                    self.state.sort = self.state.sort.next();
                    self.sort_processes();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.refresh();
                KeyHandleResult::Handled
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.state.auto_refresh = !self.state.auto_refresh;
                KeyHandleResult::Handled
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.kill_selected();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn tick(&mut self) {
        // Auto-refresh every 3 seconds when enabled and modal is open
        if self.modal_open
            && self.state.auto_refresh
            && self.state.last_refresh.elapsed().as_secs() >= 3
        {
            self.refresh();
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_proc_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "           P -- PROCESS MONITOR".to_string(),
            "".to_string(),
            "Purpose:   View and manage system processes, memory usage, disk".to_string(),
            "           space, and network activity. Similar to top or htop.".to_string(),
            "".to_string(),
            "To use:    Press F12 for Apps, then P for Processes. Use Tab to".to_string(),
            "           switch between views.".to_string(),
            "".to_string(),
            "Views:".to_string(),
            "  CPU      - Process list sorted by CPU usage".to_string(),
            "  Memory   - Process list sorted by memory usage".to_string(),
            "  Disk     - Disk usage and mount points".to_string(),
            "  Network  - Network interface statistics".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  ↑↓       - Navigate process/item list".to_string(),
            "  Tab      - Switch between views".to_string(),
            "  S        - Cycle sort mode (CPU/Memory views)".to_string(),
            "  Enter/i  - Show detailed process information".to_string(),
            "".to_string(),
            "Actions:".to_string(),
            "  R        - Refresh data manually".to_string(),
            "  A        - Toggle auto-refresh (every 3 seconds)".to_string(),
            "  X        - Kill selected process".to_string(),
            "  Esc      - Close monitor".to_string(),
            "".to_string(),
            "Tip:       Process detail view shows command line, working".to_string(),
            "           directory, parent process, and runtime info.".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Processes".to_string(),
            description: "System process monitor".to_string(),
            category: PluginCategory::System,
            key: 'P',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.open_modal();
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_view_cycle() {
        assert_eq!(ProcView::Cpu.next(), ProcView::Memory);
        assert_eq!(ProcView::Memory.next(), ProcView::Disk);
        assert_eq!(ProcView::Disk.next(), ProcView::Network);
        assert_eq!(ProcView::Network.next(), ProcView::Cpu);
    }

    #[test]
    fn test_proc_sort_cycle() {
        let sort = ProcSort::CpuDesc;
        assert_eq!(sort.next(), ProcSort::CpuAsc);
    }
}
