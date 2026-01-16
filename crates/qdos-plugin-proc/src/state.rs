//! Proc plugin state types
//!
//! State types for process monitoring views.

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
    pub fn next(&self) -> Self {
        match self {
            ProcView::Cpu => ProcView::Memory,
            ProcView::Memory => ProcView::Disk,
            ProcView::Disk => ProcView::Network,
            ProcView::Network => ProcView::Cpu,
        }
    }

    pub fn as_str(&self) -> &'static str {
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
    pub fn next(&self) -> Self {
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

    pub fn as_str(&self) -> &'static str {
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
    pub cpu_time_ms: u64,
    pub memory: u64,
    pub status: String,
    pub user: String,
    pub bytes_read: u64,
    pub bytes_written: u64,
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

/// Extended process info for detail view
#[derive(Debug, Clone, Default)]
pub struct ProcessDetailInfo {
    pub pid: u32,
    pub name: String,
    pub cmd: Vec<String>,
    pub cwd: String,
    pub parent_pid: Option<u32>,
    pub start_time: u64,
    pub cpu_usage: f32,
    pub memory: u64,
    pub status: String,
    pub user: String,
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
    /// Show process detail overlay
    pub show_detail: bool,
    /// Process detail info
    pub detail_info: Option<ProcessDetailInfo>,
    /// Detail scroll offset
    pub detail_scroll: usize,
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
            show_detail: false,
            detail_info: None,
            detail_scroll: 0,
        }
    }
}
