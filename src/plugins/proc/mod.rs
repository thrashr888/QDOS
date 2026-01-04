//! Proc Plugin for R-DOS
//!
//! System/process monitoring plugin similar to top/htop/Activity Monitor.
//! Triggered via F12, provides multiple views for CPU, Memory, Disk, Network.

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem};
use crate::ui::{COLOR_BG, COLOR_BLUE, COLOR_GREEN, COLOR_RED, COLOR_YELLOW};
use crossterm::event::{KeyCode, KeyEvent};
use humansize::{format_size, DECIMAL};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;
use sysinfo::{Disks, Networks, Pid, ProcessStatus, System};

/// View modes for the proc plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcView {
    #[default]
    Cpu,
    Memory,
    Disk,
    Network,
}

impl ProcView {
    fn next(&self) -> Self {
        match self {
            ProcView::Cpu => ProcView::Memory,
            ProcView::Memory => ProcView::Disk,
            ProcView::Disk => ProcView::Network,
            ProcView::Network => ProcView::Cpu,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            ProcView::Cpu => "CPU",
            ProcView::Memory => "Memory",
            ProcView::Disk => "Disk",
            ProcView::Network => "Network",
        }
    }
}

/// Sort mode for process list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcSort {
    #[default]
    CpuDesc,
    CpuAsc,
    MemDesc,
    MemAsc,
    PidAsc,
    PidDesc,
    NameAsc,
    NameDesc,
}

impl ProcSort {
    fn next(&self) -> Self {
        match self {
            ProcSort::CpuDesc => ProcSort::CpuAsc,
            ProcSort::CpuAsc => ProcSort::MemDesc,
            ProcSort::MemDesc => ProcSort::MemAsc,
            ProcSort::MemAsc => ProcSort::PidAsc,
            ProcSort::PidAsc => ProcSort::PidDesc,
            ProcSort::PidDesc => ProcSort::NameAsc,
            ProcSort::NameAsc => ProcSort::NameDesc,
            ProcSort::NameDesc => ProcSort::CpuDesc,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            ProcSort::CpuDesc => "CPU ↓",
            ProcSort::CpuAsc => "CPU ↑",
            ProcSort::MemDesc => "Mem ↓",
            ProcSort::MemAsc => "Mem ↑",
            ProcSort::PidAsc => "PID ↑",
            ProcSort::PidDesc => "PID ↓",
            ProcSort::NameAsc => "Name ↑",
            ProcSort::NameDesc => "Name ↓",
        }
    }
}

/// Process info for display
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
    pub status: String,
    pub user: String,
}

/// Disk info for display
#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total: u64,
    pub available: u64,
    pub used: u64,
    pub file_system: String,
}

/// Network interface info
#[derive(Debug, Clone)]
pub struct NetworkInfo {
    pub name: String,
    pub received: u64,
    pub transmitted: u64,
    pub packets_in: u64,
    pub packets_out: u64,
}

/// Proc plugin state
#[derive(Debug, Clone)]
pub struct ProcState {
    /// Current view mode
    pub view: ProcView,
    /// Sort mode
    pub sort: ProcSort,
    /// Process list
    pub processes: Vec<ProcessInfo>,
    /// Disk list
    pub disks: Vec<DiskInfo>,
    /// Network interfaces
    pub networks: Vec<NetworkInfo>,
    /// Selected item index
    pub selected: usize,
    /// Scroll offset
    pub scroll_offset: usize,
    /// System totals
    pub total_memory: u64,
    pub used_memory: u64,
    pub cpu_count: usize,
    pub cpu_usage: f32,
    /// Auto-refresh enabled
    pub auto_refresh: bool,
    /// Last refresh time
    pub last_refresh: std::time::Instant,
    /// Selected PID (for preserving selection across refreshes)
    pub selected_pid: Option<u32>,
}

impl Default for ProcState {
    fn default() -> Self {
        Self {
            view: ProcView::default(),
            sort: ProcSort::default(),
            processes: Vec::new(),
            disks: Vec::new(),
            networks: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            total_memory: 0,
            used_memory: 0,
            cpu_count: 0,
            cpu_usage: 0.0,
            auto_refresh: true,
            last_refresh: std::time::Instant::now(),
            selected_pid: None,
        }
    }
}

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

            self.state.processes.push(ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
                status: status.to_string(),
                user: process
                    .user_id()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "-".to_string()),
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
    fn open_modal(&mut self) {
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

    /// Get current list length based on view
    fn current_list_len(&self) -> usize {
        match self.state.view {
            ProcView::Cpu | ProcView::Memory => self.state.processes.len(),
            ProcView::Disk => self.state.disks.len(),
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

    fn init(&mut self, _cwd: &PathBuf) -> Result<(), String> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
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

    fn handle_global_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::F(12) => {
                self.open_modal();
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc | KeyCode::F(12) => {
                self.close_modal();
                KeyHandleResult::CloseModal
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

    fn draw_modal(&self, frame: &mut Frame, area: Rect) {
        // Full screen modal - cover entire screen
        let modal_area = Rect::new(0, 0, area.width, area.height);

        // Clear the modal area
        frame.render_widget(Clear, modal_area);

        // Use theme colors
        let bg = COLOR_BG;
        let fg = Color::White;
        let blue = COLOR_BLUE;
        let green = COLOR_GREEN;
        let yellow = COLOR_YELLOW;
        let red = COLOR_RED;

        let border_style = Style::default().fg(fg).bg(bg);
        let title_style = Style::default().fg(yellow).bg(bg).add_modifier(Modifier::BOLD);
        let header_style = Style::default().fg(blue).bg(bg);
        let normal_style = Style::default().fg(fg).bg(bg);
        let _highlight_style = Style::default().fg(yellow).bg(red);

        // Calculate dimensions
        let width = modal_area.width as usize;
        let inner_width = width.saturating_sub(2);

        // Draw top border: ╔═══╗
        let top_border = format!("╔{}╗", "═".repeat(inner_width));
        frame.render_widget(
            Paragraph::new(Span::styled(&top_border, border_style)),
            Rect::new(modal_area.x, modal_area.y, modal_area.width, 1),
        );

        // Draw title row with view tabs
        let mut y = modal_area.y + 1;
        let views = [ProcView::Cpu, ProcView::Memory, ProcView::Disk, ProcView::Network];
        let mut title_spans: Vec<Span> = vec![Span::styled("║ ", border_style)];

        for view in views.iter() {
            let is_selected = *view == self.state.view;
            let style = if is_selected {
                title_style
            } else {
                header_style
            };
            let label = format!(" {} ", view.as_str());
            title_spans.push(Span::styled(label, style));
            title_spans.push(Span::styled(" ", normal_style));
        }

        // Add system stats
        let mem_pct = if self.state.total_memory > 0 {
            (self.state.used_memory as f64 / self.state.total_memory as f64) * 100.0
        } else {
            0.0
        };

        title_spans.push(Span::styled(" │ ", border_style));
        title_spans.push(Span::styled(
            format!("CPU: {:.1}% ", self.state.cpu_usage),
            Style::default().fg(green).bg(bg),
        ));
        title_spans.push(Span::styled(
            format!(
                "Mem: {:.1}% ",
                mem_pct
            ),
            Style::default().fg(blue).bg(bg),
        ));

        if self.state.auto_refresh {
            title_spans.push(Span::styled("[Auto]", Style::default().fg(green).bg(bg)));
        } else {
            title_spans.push(Span::styled("[Manual]", Style::default().fg(red).bg(bg)));
        }

        // Pad and close - calculate remaining space for padding
        let title_content_width: usize = title_spans.iter().map(|s| s.width()).sum();
        // Total line width is modal_area.width. We need: content + padding + "║"
        let padding = (width).saturating_sub(title_content_width + 1);
        title_spans.push(Span::styled(" ".repeat(padding), normal_style));
        title_spans.push(Span::styled("║", border_style));

        frame.render_widget(
            Paragraph::new(Line::from(title_spans)),
            Rect::new(modal_area.x, y, modal_area.width, 1),
        );
        y += 1;

        // Draw separator: ╠═══╣
        let sep = format!("╠{}╣", "═".repeat(inner_width));
        frame.render_widget(
            Paragraph::new(Span::styled(&sep, border_style)),
            Rect::new(modal_area.x, y, modal_area.width, 1),
        );
        y += 1;

        // Draw content based on view
        let content_height = modal_area.height.saturating_sub(6) as usize; // 3 header + 2 footer + 1 border

        match self.state.view {
            ProcView::Cpu | ProcView::Memory => {
                self.draw_process_view(frame, modal_area.x, y, modal_area.width, content_height, inner_width);
            }
            ProcView::Disk => {
                self.draw_disk_view(frame, modal_area.x, y, modal_area.width, content_height, inner_width);
            }
            ProcView::Network => {
                self.draw_network_view(frame, modal_area.x, y, modal_area.width, content_height, inner_width);
            }
        }

        y += content_height as u16;

        // Draw separator before footer
        let sep = format!("╠{}╣", "═".repeat(inner_width));
        frame.render_widget(
            Paragraph::new(Span::styled(&sep, border_style)),
            Rect::new(modal_area.x, y, modal_area.width, 1),
        );
        y += 1;

        // Draw footer with help
        let help_spans = vec![
            Span::styled("║ ", border_style),
            Span::styled("↑↓", Style::default().fg(green).bg(bg)),
            Span::styled(" nav  ", normal_style),
            Span::styled("Tab", Style::default().fg(green).bg(bg)),
            Span::styled(" view  ", normal_style),
            Span::styled("S", Style::default().fg(green).bg(bg)),
            Span::styled(" sort  ", normal_style),
            Span::styled("R", Style::default().fg(green).bg(bg)),
            Span::styled(" refresh  ", normal_style),
            Span::styled("A", Style::default().fg(green).bg(bg)),
            Span::styled(" auto  ", normal_style),
            Span::styled("X", Style::default().fg(red).bg(bg)),
            Span::styled(" kill  ", normal_style),
            Span::styled("Esc", Style::default().fg(green).bg(bg)),
            Span::styled(" close", normal_style),
        ];
        let help_content_width: usize = help_spans.iter().map(|s| s.width()).sum();
        let mut footer_spans = help_spans;
        // Total line width is modal_area.width. We need: content + padding + "║"
        let padding = (width).saturating_sub(help_content_width + 1);
        footer_spans.push(Span::styled(" ".repeat(padding), normal_style));
        footer_spans.push(Span::styled("║", border_style));

        frame.render_widget(
            Paragraph::new(Line::from(footer_spans)),
            Rect::new(modal_area.x, y, modal_area.width, 1),
        );
        y += 1;

        // Draw bottom border: ╚═══╝
        let bottom_border = format!("╚{}╝", "═".repeat(inner_width));
        frame.render_widget(
            Paragraph::new(Span::styled(&bottom_border, border_style)),
            Rect::new(modal_area.x, y, modal_area.width, 1),
        );
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "F12 - Open Process Monitor".to_string(),
            "  ↑↓  Navigate list".to_string(),
            "  Tab Switch view (CPU/Memory/Disk/Network)".to_string(),
            "  S   Cycle sort mode (in process views)".to_string(),
            "  R   Refresh data".to_string(),
            "  A   Toggle auto-refresh".to_string(),
            "  X   Kill selected process".to_string(),
            "  Esc Close monitor".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ProcPlugin {
    /// Draw process list view (CPU or Memory)
    fn draw_process_view(
        &self,
        frame: &mut Frame,
        x: u16,
        start_y: u16,
        width: u16,
        height: usize,
        inner_width: usize,
    ) {
        let bg = COLOR_BG;
        let fg = Color::White;
        let blue = COLOR_BLUE;
        let yellow = COLOR_YELLOW;
        let red = COLOR_RED;

        let border_style = Style::default().fg(fg).bg(bg);
        let header_style = Style::default().fg(blue).bg(bg);
        let normal_style = Style::default().fg(fg).bg(bg);
        let highlight_style = Style::default().fg(yellow).bg(red);

        // Header row - use spans so borders are styled correctly
        let sort_indicator = self.state.sort.as_str();
        let header_content = format!(
            "{:>7}  {:<30}  {:>8}  {:>12}  {:>6}  (sorted by {})",
            "PID", "Name", "CPU %", "Memory", "Status", sort_indicator
        );
        let mut header_spans = vec![
            Span::styled("║ ", border_style),
            Span::styled(&header_content, header_style),
        ];
        let header_width: usize = header_spans.iter().map(|s| s.width()).sum();
        let padding = (width as usize).saturating_sub(header_width + 1);
        header_spans.push(Span::styled(" ".repeat(padding), header_style));
        header_spans.push(Span::styled("║", border_style));
        frame.render_widget(
            Paragraph::new(Line::from(header_spans)),
            Rect::new(x, start_y, width, 1),
        );

        // Visible processes
        let visible_height = height.saturating_sub(1);
        let mut scroll_offset = self.state.scroll_offset;
        if self.state.selected >= scroll_offset + visible_height {
            scroll_offset = self.state.selected.saturating_sub(visible_height - 1);
        }
        if self.state.selected < scroll_offset {
            scroll_offset = self.state.selected;
        }

        for (i, proc) in self
            .state
            .processes
            .iter()
            .skip(scroll_offset)
            .take(visible_height)
            .enumerate()
        {
            let y = start_y + 1 + i as u16;
            let actual_idx = scroll_offset + i;
            let is_selected = actual_idx == self.state.selected;

            let style = if is_selected {
                highlight_style
            } else {
                normal_style
            };

            // Color CPU usage based on value
            let cpu_style = if proc.cpu_usage > 50.0 {
                style.fg(red)
            } else if proc.cpu_usage > 10.0 {
                style.fg(yellow)
            } else {
                style
            };

            // Build spans with different colors for CPU
            let mut spans = vec![Span::styled("║ ", border_style)];
            spans.push(Span::styled(format!("{:>7}  ", proc.pid), style));
            spans.push(Span::styled(
                format!(
                    "{:<30}  ",
                    if proc.name.len() > 30 {
                        &proc.name[..30]
                    } else {
                        &proc.name
                    }
                ),
                style,
            ));
            spans.push(Span::styled(format!("{:>8.1}  ", proc.cpu_usage), cpu_style));
            spans.push(Span::styled(
                format!("{:>12}  ", format_size(proc.memory, DECIMAL)),
                style,
            ));
            spans.push(Span::styled(format!("{:>6}", proc.status), style));

            // Padding to fill line - total width minus content and closing border
            let content_width: usize = spans.iter().map(|s| s.width()).sum();
            let padding = (width as usize).saturating_sub(content_width + 1);
            spans.push(Span::styled(" ".repeat(padding), style));
            spans.push(Span::styled("║", border_style));

            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(x, y, width, 1),
            );
        }

        // Fill remaining lines
        for i in self.state.processes.len().min(visible_height)..visible_height {
            let y = start_y + 1 + i as u16;
            let empty_line = format!("║{:width$}║", "", width = inner_width);
            frame.render_widget(
                Paragraph::new(Span::styled(&empty_line, normal_style)),
                Rect::new(x, y, width, 1),
            );
        }
    }

    /// Draw disk view
    fn draw_disk_view(
        &self,
        frame: &mut Frame,
        x: u16,
        start_y: u16,
        width: u16,
        height: usize,
        inner_width: usize,
    ) {
        let bg = COLOR_BG;
        let fg = Color::White;
        let blue = COLOR_BLUE;
        let yellow = COLOR_YELLOW;
        let red = COLOR_RED;
        let green = COLOR_GREEN;

        let border_style = Style::default().fg(fg).bg(bg);
        let header_style = Style::default().fg(blue).bg(bg);
        let normal_style = Style::default().fg(fg).bg(bg);
        let highlight_style = Style::default().fg(yellow).bg(red);

        // Header row - use spans so borders are styled correctly
        let header_content = format!(
            "{:<20}  {:<20}  {:>12}  {:>12}  {:>12}  {:>6}",
            "Name", "Mount Point", "Total", "Used", "Available", "Use%"
        );
        let mut header_spans = vec![
            Span::styled("║ ", border_style),
            Span::styled(&header_content, header_style),
        ];
        let header_width: usize = header_spans.iter().map(|s| s.width()).sum();
        let padding = (width as usize).saturating_sub(header_width + 1);
        header_spans.push(Span::styled(" ".repeat(padding), header_style));
        header_spans.push(Span::styled("║", border_style));
        frame.render_widget(
            Paragraph::new(Line::from(header_spans)),
            Rect::new(x, start_y, width, 1),
        );

        // Visible disks
        let visible_height = height.saturating_sub(1);

        for (i, disk) in self.state.disks.iter().take(visible_height).enumerate() {
            let y = start_y + 1 + i as u16;
            let is_selected = i == self.state.selected;

            let style = if is_selected {
                highlight_style
            } else {
                normal_style
            };

            let usage_pct = if disk.total > 0 {
                (disk.used as f64 / disk.total as f64) * 100.0
            } else {
                0.0
            };

            // Color usage percentage based on value
            let usage_style = if usage_pct > 90.0 {
                style.fg(red)
            } else if usage_pct > 70.0 {
                style.fg(yellow)
            } else {
                style.fg(green)
            };

            let mut spans = vec![Span::styled("║ ", border_style)];
            spans.push(Span::styled(
                format!(
                    "{:<20}  ",
                    if disk.name.len() > 20 {
                        &disk.name[..20]
                    } else {
                        &disk.name
                    }
                ),
                style,
            ));
            spans.push(Span::styled(
                format!(
                    "{:<20}  ",
                    if disk.mount_point.len() > 20 {
                        &disk.mount_point[..20]
                    } else {
                        &disk.mount_point
                    }
                ),
                style,
            ));
            spans.push(Span::styled(
                format!("{:>12}  ", format_size(disk.total, DECIMAL)),
                style,
            ));
            spans.push(Span::styled(
                format!("{:>12}  ", format_size(disk.used, DECIMAL)),
                style,
            ));
            spans.push(Span::styled(
                format!("{:>12}  ", format_size(disk.available, DECIMAL)),
                style,
            ));
            spans.push(Span::styled(format!("{:>5.1}%", usage_pct), usage_style));

            // Padding - total width minus content and closing border
            let content_width: usize = spans.iter().map(|s| s.width()).sum();
            let padding = (width as usize).saturating_sub(content_width + 1);
            spans.push(Span::styled(" ".repeat(padding), style));
            spans.push(Span::styled("║", border_style));

            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(x, y, width, 1),
            );
        }

        // Fill remaining lines
        for i in self.state.disks.len().min(visible_height)..visible_height {
            let y = start_y + 1 + i as u16;
            let empty_line = format!("║{:width$}║", "", width = inner_width);
            frame.render_widget(
                Paragraph::new(Span::styled(&empty_line, normal_style)),
                Rect::new(x, y, width, 1),
            );
        }
    }

    /// Draw network view
    fn draw_network_view(
        &self,
        frame: &mut Frame,
        x: u16,
        start_y: u16,
        width: u16,
        height: usize,
        inner_width: usize,
    ) {
        let bg = COLOR_BG;
        let fg = Color::White;
        let blue = COLOR_BLUE;
        let yellow = COLOR_YELLOW;
        let red = COLOR_RED;

        let border_style = Style::default().fg(fg).bg(bg);
        let header_style = Style::default().fg(blue).bg(bg);
        let normal_style = Style::default().fg(fg).bg(bg);
        let highlight_style = Style::default().fg(yellow).bg(red);

        // Header row - use spans so borders are styled correctly
        let header_content = format!(
            "{:<20}  {:>14}  {:>14}  {:>12}  {:>12}",
            "Interface", "Received", "Transmitted", "Pkts In", "Pkts Out"
        );
        let mut header_spans = vec![
            Span::styled("║ ", border_style),
            Span::styled(&header_content, header_style),
        ];
        let header_width: usize = header_spans.iter().map(|s| s.width()).sum();
        let padding = (width as usize).saturating_sub(header_width + 1);
        header_spans.push(Span::styled(" ".repeat(padding), header_style));
        header_spans.push(Span::styled("║", border_style));
        frame.render_widget(
            Paragraph::new(Line::from(header_spans)),
            Rect::new(x, start_y, width, 1),
        );

        // Visible networks
        let visible_height = height.saturating_sub(1);

        for (i, net) in self.state.networks.iter().take(visible_height).enumerate() {
            let y = start_y + 1 + i as u16;
            let is_selected = i == self.state.selected;

            let style = if is_selected {
                highlight_style
            } else {
                normal_style
            };

            let mut spans = vec![Span::styled("║ ", border_style)];
            spans.push(Span::styled(
                format!(
                    "{:<20}  ",
                    if net.name.len() > 20 {
                        &net.name[..20]
                    } else {
                        &net.name
                    }
                ),
                style,
            ));
            spans.push(Span::styled(
                format!("{:>14}  ", format_size(net.received, DECIMAL)),
                style,
            ));
            spans.push(Span::styled(
                format!("{:>14}  ", format_size(net.transmitted, DECIMAL)),
                style,
            ));
            spans.push(Span::styled(format!("{:>12}  ", net.packets_in), style));
            spans.push(Span::styled(format!("{:>12}", net.packets_out), style));

            // Padding - total width minus content and closing border
            let content_width: usize = spans.iter().map(|s| s.width()).sum();
            let padding = (width as usize).saturating_sub(content_width + 1);
            spans.push(Span::styled(" ".repeat(padding), style));
            spans.push(Span::styled("║", border_style));

            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(x, y, width, 1),
            );
        }

        // Fill remaining lines
        for i in self.state.networks.len().min(visible_height)..visible_height {
            let y = start_y + 1 + i as u16;
            let empty_line = format!("║{:width$}║", "", width = inner_width);
            frame.render_widget(
                Paragraph::new(Span::styled(&empty_line, normal_style)),
                Rect::new(x, y, width, 1),
            );
        }
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
