//! Docker plugin state

use std::path::PathBuf;

/// Build process status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildStatus {
    #[default]
    NotStarted,
    Running,
    Success,
    Failed,
}

impl BuildStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, BuildStatus::Running)
    }

    pub fn is_done(&self) -> bool {
        matches!(self, BuildStatus::Success | BuildStatus::Failed)
    }
}

/// Get the folder name from a path for display
pub fn folder_name(path: &PathBuf) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

/// Docker context type - what file was selected when opening
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DockerContext {
    #[default]
    None,
    /// Dockerfile selected - can build image
    Dockerfile(PathBuf),
    /// docker-compose.yml selected - can manage services
    Compose(PathBuf),
}

impl DockerContext {
    /// Check if we have a context file
    pub fn has_context(&self) -> bool {
        !matches!(self, DockerContext::None)
    }

    /// Get the context file path
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            DockerContext::None => None,
            DockerContext::Dockerfile(p) | DockerContext::Compose(p) => Some(p),
        }
    }
}

/// Main view modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockerView {
    #[default]
    Containers,
    Images,
    Volumes,
    Networks,
    Logs,
    Inspect,
    Pull,
    Exec,
    Confirm,
    /// Building an image from Dockerfile
    Build,
    /// Building output
    BuildOutput,
    /// Compose services list
    Compose,
    /// Compose service logs
    ComposeLogs,
}

/// Docker tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockerTab {
    #[default]
    Containers,
    Images,
    Volumes,
    Networks,
}

impl DockerTab {
    pub fn next(&self) -> Self {
        match self {
            DockerTab::Containers => DockerTab::Images,
            DockerTab::Images => DockerTab::Volumes,
            DockerTab::Volumes => DockerTab::Networks,
            DockerTab::Networks => DockerTab::Containers,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            DockerTab::Containers => DockerTab::Networks,
            DockerTab::Images => DockerTab::Containers,
            DockerTab::Volumes => DockerTab::Images,
            DockerTab::Networks => DockerTab::Volumes,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            DockerTab::Containers => "Containers",
            DockerTab::Images => "Images",
            DockerTab::Volumes => "Volumes",
            DockerTab::Networks => "Networks",
        }
    }
}

/// Container status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContainerStatus {
    Running,
    Paused,
    #[default]
    Stopped,
    Restarting,
    Dead,
    Created,
}

impl ContainerStatus {
    pub fn from_str(s: &str) -> Self {
        let s = s.to_lowercase();
        if s.starts_with("up") {
            ContainerStatus::Running
        } else if s.contains("paused") {
            ContainerStatus::Paused
        } else if s.starts_with("exited") || s.starts_with("stopped") {
            ContainerStatus::Stopped
        } else if s.contains("restarting") {
            ContainerStatus::Restarting
        } else if s.contains("dead") {
            ContainerStatus::Dead
        } else if s.contains("created") {
            ContainerStatus::Created
        } else {
            ContainerStatus::Stopped
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            ContainerStatus::Running => "▶",
            ContainerStatus::Paused => "⏸",
            ContainerStatus::Stopped => "■",
            ContainerStatus::Restarting => "↻",
            ContainerStatus::Dead => "✖",
            ContainerStatus::Created => "○",
        }
    }
}

/// Container entry from docker ps -a
#[derive(Debug, Clone, Default)]
pub struct ContainerEntry {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
    pub status_text: String,
    pub ports: String,
    pub created: String,
}

/// Image entry from docker images
#[derive(Debug, Clone, Default)]
pub struct ImageEntry {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub size: String,
    pub created: String,
}

impl ImageEntry {
    pub fn full_name(&self) -> String {
        if self.tag.is_empty() || self.tag == "<none>" {
            self.repository.clone()
        } else {
            format!("{}:{}", self.repository, self.tag)
        }
    }
}

/// Volume entry
#[derive(Debug, Clone, Default)]
pub struct VolumeEntry {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
}

/// Network entry
#[derive(Debug, Clone, Default)]
pub struct NetworkEntry {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
}

/// Compose service entry
#[derive(Debug, Clone, Default)]
pub struct ComposeService {
    pub name: String,
    pub status: String,
    pub ports: String,
}

/// Compose service status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComposeStatus {
    #[default]
    Unknown,
    Running,
    Stopped,
    Starting,
}

impl ComposeStatus {
    pub fn from_str(s: &str) -> Self {
        let s = s.to_lowercase();
        if s.contains("running") || s.contains("up") {
            ComposeStatus::Running
        } else if s.contains("exited") || s.contains("stopped") {
            ComposeStatus::Stopped
        } else if s.contains("starting") {
            ComposeStatus::Starting
        } else {
            ComposeStatus::Unknown
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            ComposeStatus::Running => "▶",
            ComposeStatus::Stopped => "■",
            ComposeStatus::Starting => "↻",
            ComposeStatus::Unknown => "?",
        }
    }
}

/// Action requiring confirmation
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    StopContainer(String),
    RemoveContainer(String),
    RemoveImage(String),
    RemoveVolume(String),
    RemoveNetwork(String),
    PruneContainers,
    PruneImages,
    ComposeUp,
    ComposeDown,
    ComposeRestart(String),
}

impl std::fmt::Display for ConfirmAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfirmAction::StopContainer(name) => write!(f, "Stop container {}?", name),
            ConfirmAction::RemoveContainer(name) => write!(f, "Remove container {}?", name),
            ConfirmAction::RemoveImage(name) => write!(f, "Remove image {}?", name),
            ConfirmAction::RemoveVolume(name) => write!(f, "Remove volume {}?", name),
            ConfirmAction::RemoveNetwork(name) => write!(f, "Remove network {}?", name),
            ConfirmAction::PruneContainers => write!(f, "Remove all stopped containers?"),
            ConfirmAction::PruneImages => write!(f, "Remove all unused images?"),
            ConfirmAction::ComposeUp => write!(f, "Start all compose services?"),
            ConfirmAction::ComposeDown => write!(f, "Stop all compose services?"),
            ConfirmAction::ComposeRestart(name) => write!(f, "Restart service {}?", name),
        }
    }
}

/// Main state container
#[derive(Debug, Clone, Default)]
pub struct DockerState {
    pub view: DockerView,
    pub tab: DockerTab,
    pub loading: bool,
    pub loading_message: Option<String>,
    pub error: Option<String>,
    pub message: Option<String>,

    // Current working directory
    pub cwd: Option<PathBuf>,

    // Containers
    pub containers: Vec<ContainerEntry>,
    pub selected_container: usize,
    pub container_scroll: usize,
    pub show_all_containers: bool,

    // Images
    pub images: Vec<ImageEntry>,
    pub selected_image: usize,
    pub image_scroll: usize,

    // Volumes
    pub volumes: Vec<VolumeEntry>,
    pub selected_volume: usize,
    pub volume_scroll: usize,

    // Networks
    pub networks: Vec<NetworkEntry>,
    pub selected_network: usize,
    pub network_scroll: usize,

    // Logs/Inspect output
    pub output_lines: Vec<String>,
    pub output_scroll: usize,
    pub following_logs: bool,

    // Exec command input
    pub exec_command: String,
    pub exec_cursor: usize,

    // Pull image input
    pub pull_image_name: String,
    pub pull_cursor: usize,

    // Confirmation
    pub confirm_action: Option<ConfirmAction>,

    // Docker availability
    pub docker_available: bool,

    // Context (Dockerfile or docker-compose.yml)
    pub context: DockerContext,

    // Build settings
    pub build_tag: String,
    pub build_cursor: usize,
    pub build_status: BuildStatus,

    // Compose services
    pub compose_services: Vec<ComposeService>,
    pub selected_service: usize,
    pub service_scroll: usize,
}

impl DockerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_loading(&mut self, msg: &str) {
        self.loading = true;
        self.loading_message = Some(msg.to_string());
    }

    pub fn clear_loading(&mut self) {
        self.loading = false;
        self.loading_message = None;
    }

    pub fn reset(&mut self) {
        self.view = DockerView::Containers;
        self.tab = DockerTab::Containers;
        self.loading = false;
        self.loading_message = None;
        self.error = None;
        self.message = None;
        self.containers.clear();
        self.images.clear();
        self.volumes.clear();
        self.networks.clear();
        self.output_lines.clear();
        self.exec_command.clear();
        self.pull_image_name.clear();
        self.confirm_action = None;
    }

    /// Get current list length based on tab
    pub fn current_list_len(&self) -> usize {
        match self.tab {
            DockerTab::Containers => self.containers.len(),
            DockerTab::Images => self.images.len(),
            DockerTab::Volumes => self.volumes.len(),
            DockerTab::Networks => self.networks.len(),
        }
    }

    /// Get current selection index
    pub fn current_index(&self) -> usize {
        match self.tab {
            DockerTab::Containers => self.selected_container,
            DockerTab::Images => self.selected_image,
            DockerTab::Volumes => self.selected_volume,
            DockerTab::Networks => self.selected_network,
        }
    }

    /// Set current selection index
    pub fn set_current_index(&mut self, idx: usize) {
        match self.tab {
            DockerTab::Containers => self.selected_container = idx,
            DockerTab::Images => self.selected_image = idx,
            DockerTab::Volumes => self.selected_volume = idx,
            DockerTab::Networks => self.selected_network = idx,
        }
    }

    /// Get current scroll offset
    pub fn current_scroll(&self) -> usize {
        match self.tab {
            DockerTab::Containers => self.container_scroll,
            DockerTab::Images => self.image_scroll,
            DockerTab::Volumes => self.volume_scroll,
            DockerTab::Networks => self.network_scroll,
        }
    }

    /// Set current scroll offset
    pub fn set_current_scroll(&mut self, offset: usize) {
        match self.tab {
            DockerTab::Containers => self.container_scroll = offset,
            DockerTab::Images => self.image_scroll = offset,
            DockerTab::Volumes => self.volume_scroll = offset,
            DockerTab::Networks => self.network_scroll = offset,
        }
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        let idx = self.current_index();
        if idx > 0 {
            self.set_current_index(idx - 1);
            self.ensure_visible();
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let max = self.current_list_len().saturating_sub(1);
        let idx = self.current_index();
        if idx < max {
            self.set_current_index(idx + 1);
            self.ensure_visible();
        }
    }

    /// Ensure selected item is visible
    fn ensure_visible(&mut self) {
        let visible_lines = 16;
        let idx = self.current_index();
        let scroll = self.current_scroll();
        if idx < scroll {
            self.set_current_scroll(idx);
        } else if idx >= scroll + visible_lines {
            self.set_current_scroll(idx - visible_lines + 1);
        }
    }

    /// Get selected container
    pub fn selected_container(&self) -> Option<&ContainerEntry> {
        self.containers.get(self.selected_container)
    }

    /// Get selected image
    pub fn selected_image(&self) -> Option<&ImageEntry> {
        self.images.get(self.selected_image)
    }

    /// Get selected volume
    pub fn selected_volume(&self) -> Option<&VolumeEntry> {
        self.volumes.get(self.selected_volume)
    }

    /// Get selected network
    pub fn selected_network(&self) -> Option<&NetworkEntry> {
        self.networks.get(self.selected_network)
    }

    /// Insert character in pull input
    pub fn insert_pull_char(&mut self, c: char) {
        self.pull_image_name.insert(self.pull_cursor, c);
        self.pull_cursor += 1;
    }

    /// Backspace in pull input
    pub fn backspace_pull(&mut self) {
        if self.pull_cursor > 0 {
            self.pull_cursor -= 1;
            self.pull_image_name.remove(self.pull_cursor);
        }
    }

    /// Insert character in exec input
    pub fn insert_exec_char(&mut self, c: char) {
        self.exec_command.insert(self.exec_cursor, c);
        self.exec_cursor += 1;
    }

    /// Backspace in exec input
    pub fn backspace_exec(&mut self) {
        if self.exec_cursor > 0 {
            self.exec_cursor -= 1;
            self.exec_command.remove(self.exec_cursor);
        }
    }
}
