//! Terraform plugin state

use std::path::PathBuf;

/// Get the folder name from a path for display
pub fn folder_name(path: &PathBuf) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

/// Command execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommandStatus {
    #[default]
    NotStarted,
    Running,
    Success,
    Failed,
}

impl CommandStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, CommandStatus::Running)
    }

    pub fn is_done(&self) -> bool {
        matches!(self, CommandStatus::Success | CommandStatus::Failed)
    }
}

/// Main view modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerraformView {
    #[default]
    Menu,
    Init,
    Plan,
    Apply,
    Workspaces,
    State,
    StateDetail,
    Output,
    Confirm,
}

/// Tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerraformTab {
    #[default]
    Operations,
    Workspaces,
    State,
}

impl TerraformTab {
    pub fn next(&self) -> Self {
        match self {
            TerraformTab::Operations => TerraformTab::Workspaces,
            TerraformTab::Workspaces => TerraformTab::State,
            TerraformTab::State => TerraformTab::Operations,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            TerraformTab::Operations => TerraformTab::State,
            TerraformTab::Workspaces => TerraformTab::Operations,
            TerraformTab::State => TerraformTab::Workspaces,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            TerraformTab::Operations => "Operations",
            TerraformTab::Workspaces => "Workspaces",
            TerraformTab::State => "State",
        }
    }
}

/// Menu item for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Init,
    Plan,
    Apply,
    Destroy,
    Refresh,
    Validate,
}

impl MenuItem {
    pub fn all() -> &'static [MenuItem] {
        &[
            MenuItem::Init,
            MenuItem::Plan,
            MenuItem::Apply,
            MenuItem::Destroy,
            MenuItem::Refresh,
            MenuItem::Validate,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            MenuItem::Init => "Init",
            MenuItem::Plan => "Plan",
            MenuItem::Apply => "Apply",
            MenuItem::Destroy => "Destroy",
            MenuItem::Refresh => "Refresh",
            MenuItem::Validate => "Validate",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            MenuItem::Init => "Initialize working directory",
            MenuItem::Plan => "Show changes to infrastructure",
            MenuItem::Apply => "Apply changes to infrastructure",
            MenuItem::Destroy => "Destroy all managed infrastructure",
            MenuItem::Refresh => "Update state with real resources",
            MenuItem::Validate => "Validate configuration",
        }
    }

    pub fn key(&self) -> char {
        match self {
            MenuItem::Init => 'i',
            MenuItem::Plan => 'p',
            MenuItem::Apply => 'a',
            MenuItem::Destroy => 'd',
            MenuItem::Refresh => 'r',
            MenuItem::Validate => 'v',
        }
    }
}

/// Workspace entry
#[derive(Debug, Clone, Default)]
pub struct WorkspaceEntry {
    pub name: String,
    pub is_current: bool,
}

/// State resource entry
#[derive(Debug, Clone, Default)]
pub struct StateResource {
    pub address: String,
    pub resource_type: String,
    pub name: String,
    pub provider: String,
}

impl StateResource {
    /// Parse from terraform state list output line
    pub fn from_address(address: &str) -> Self {
        // Format: module.name.type.name or type.name
        let parts: Vec<&str> = address.split('.').collect();

        let (resource_type, name) = if parts.len() >= 2 {
            let last_two = &parts[parts.len() - 2..];
            (last_two[0].to_string(), last_two[1].to_string())
        } else {
            (address.to_string(), String::new())
        };

        Self {
            address: address.to_string(),
            resource_type,
            name,
            provider: String::new(),
        }
    }
}

/// Action requiring confirmation
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    Apply,
    Destroy,
    StateRemove(String),
    WorkspaceDelete(String),
}

impl std::fmt::Display for ConfirmAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfirmAction::Apply => write!(f, "Apply changes to infrastructure?"),
            ConfirmAction::Destroy => write!(f, "DESTROY all managed infrastructure?"),
            ConfirmAction::StateRemove(addr) => write!(f, "Remove '{}' from state?", addr),
            ConfirmAction::WorkspaceDelete(name) => write!(f, "Delete workspace '{}'?", name),
        }
    }
}

/// Main state container
#[derive(Debug, Clone, Default)]
pub struct TerraformState {
    pub view: TerraformView,
    pub tab: TerraformTab,
    pub loading: bool,
    pub loading_message: Option<String>,
    pub error: Option<String>,
    pub message: Option<String>,

    // Current working directory
    pub cwd: Option<PathBuf>,

    // Terraform availability
    pub terraform_available: bool,
    pub initialized: bool,

    // Menu
    pub selected_menu: usize,

    // Workspaces
    pub workspaces: Vec<WorkspaceEntry>,
    pub selected_workspace: usize,
    pub workspace_scroll: usize,

    // State
    pub resources: Vec<StateResource>,
    pub selected_resource: usize,
    pub resource_scroll: usize,

    // Resource detail
    pub current_resource: Option<StateResource>,
    pub resource_detail: Vec<String>,
    pub detail_scroll: usize,

    // Output (plan/apply output)
    pub output_lines: Vec<String>,
    pub output_scroll: usize,

    // Command execution status
    pub command_status: CommandStatus,

    // Confirmation
    pub confirm_action: Option<ConfirmAction>,
}

impl TerraformState {
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
        self.view = TerraformView::Menu;
        self.tab = TerraformTab::Operations;
        self.loading = false;
        self.loading_message = None;
        self.error = None;
        self.message = None;
        self.workspaces.clear();
        self.resources.clear();
        self.output_lines.clear();
        self.current_resource = None;
        self.resource_detail.clear();
        self.confirm_action = None;
    }

    /// Get current workspace name
    pub fn current_workspace(&self) -> Option<&str> {
        self.workspaces
            .iter()
            .find(|w| w.is_current)
            .map(|w| w.name.as_str())
    }

    /// Get selected workspace
    pub fn selected_workspace(&self) -> Option<&WorkspaceEntry> {
        self.workspaces.get(self.selected_workspace)
    }

    /// Get selected resource
    pub fn selected_resource(&self) -> Option<&StateResource> {
        self.resources.get(self.selected_resource)
    }

    /// Move selection up (based on current tab)
    pub fn move_up(&mut self) {
        match self.tab {
            TerraformTab::Operations => {
                if self.selected_menu > 0 {
                    self.selected_menu -= 1;
                }
            }
            TerraformTab::Workspaces => {
                if self.selected_workspace > 0 {
                    self.selected_workspace -= 1;
                    self.ensure_workspace_visible();
                }
            }
            TerraformTab::State => {
                if self.selected_resource > 0 {
                    self.selected_resource -= 1;
                    self.ensure_resource_visible();
                }
            }
        }
    }

    /// Move selection down (based on current tab)
    pub fn move_down(&mut self) {
        match self.tab {
            TerraformTab::Operations => {
                let max = MenuItem::all().len().saturating_sub(1);
                if self.selected_menu < max {
                    self.selected_menu += 1;
                }
            }
            TerraformTab::Workspaces => {
                let max = self.workspaces.len().saturating_sub(1);
                if self.selected_workspace < max {
                    self.selected_workspace += 1;
                    self.ensure_workspace_visible();
                }
            }
            TerraformTab::State => {
                let max = self.resources.len().saturating_sub(1);
                if self.selected_resource < max {
                    self.selected_resource += 1;
                    self.ensure_resource_visible();
                }
            }
        }
    }

    fn ensure_workspace_visible(&mut self) {
        let visible_lines = 16;
        if self.selected_workspace < self.workspace_scroll {
            self.workspace_scroll = self.selected_workspace;
        } else if self.selected_workspace >= self.workspace_scroll + visible_lines {
            self.workspace_scroll = self.selected_workspace - visible_lines + 1;
        }
    }

    fn ensure_resource_visible(&mut self) {
        let visible_lines = 16;
        if self.selected_resource < self.resource_scroll {
            self.resource_scroll = self.selected_resource;
        } else if self.selected_resource >= self.resource_scroll + visible_lines {
            self.resource_scroll = self.selected_resource - visible_lines + 1;
        }
    }
}
