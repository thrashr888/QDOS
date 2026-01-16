#![allow(dead_code)]
#![allow(clippy::if_same_then_else, clippy::needless_borrow)]
#![allow(clippy::ptr_arg)]

//! Terraform plugin - Infrastructure as code management
//!
//! Provides Terraform operations, workspace management, and state browsing.

mod modal;
mod ops;
mod state;

use crossterm::event::{KeyCode, KeyEvent};
use qdos_plugin_api::prelude::ThemeColors;
use qdos_plugin_api::prelude::*;
use ratatui::{layout::Rect, Frame};
use std::any::Any;
use std::path::PathBuf;

pub use state::TerraformState;

/// Terraform plugin
pub struct TerraformPlugin {
    state: TerraformState,
    modal_open: bool,
    cwd: Option<PathBuf>,
    /// Active terraform process (if any)
    active_process: Option<ops::TerraformProcess>,
    /// Last known output line count (for auto-scroll detection)
    last_output_count: usize,
}

impl TerraformPlugin {
    pub fn new() -> Self {
        Self {
            state: TerraformState::new(),
            modal_open: false,
            cwd: None,
            active_process: None,
            last_output_count: 0,
        }
    }

    /// Initialize plugin with current directory
    fn initialize(&mut self, cwd: &PathBuf) {
        self.cwd = Some(cwd.clone());
        self.state.cwd = Some(cwd.clone());
        self.state.terraform_available = ops::check_terraform();
        self.state.initialized = ops::is_initialized(cwd);

        if self.state.terraform_available && self.state.initialized {
            self.load_workspaces();
        }
    }

    /// Get current working directory
    fn cwd(&self) -> Option<&PathBuf> {
        self.cwd.as_ref()
    }

    /// Load workspaces
    fn load_workspaces(&mut self) {
        if let Some(cwd) = self.cwd.as_ref() {
            self.state.set_loading("Loading workspaces...");
            match ops::list_workspaces(cwd) {
                Ok(workspaces) => {
                    self.state.workspaces = workspaces;
                    self.state.selected_workspace = 0;
                    self.state.workspace_scroll = 0;
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
            self.state.clear_loading();
        }
    }

    /// Load state resources
    fn load_state(&mut self) {
        if let Some(cwd) = self.cwd.as_ref() {
            self.state.set_loading("Loading state...");
            match ops::list_state(cwd) {
                Ok(resources) => {
                    self.state.resources = resources;
                    self.state.selected_resource = 0;
                    self.state.resource_scroll = 0;
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
            self.state.clear_loading();
        }
    }

    /// Run terraform init (streaming)
    fn do_init(&mut self) {
        if let Some(cwd) = self.cwd.clone() {
            match ops::start_command(&cwd, &["init", "-input=false", "-no-color"]) {
                Ok(process) => {
                    self.active_process = Some(process);
                    self.state.command_status = state::CommandStatus::Running;
                    self.state.output_lines.clear();
                    self.state
                        .output_lines
                        .push("Initializing Terraform...".to_string());
                    self.state.output_scroll = 0;
                    self.last_output_count = 0;
                    self.state.view = state::TerraformView::Init;
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
        }
    }

    /// Poll active process for new output
    fn poll_process(&mut self) {
        if let Some(ref process) = self.active_process {
            // Get current output
            let output = process.get_output();
            let new_count = output.len();

            // If we have new output, update state
            if new_count > self.last_output_count {
                for line in output.iter().skip(self.last_output_count) {
                    self.state.output_lines.push(line.clone());
                }
                self.last_output_count = new_count;

                // Auto-scroll to bottom
                let total_lines = self.state.output_lines.len();
                let visible_lines = 18;
                if total_lines > visible_lines {
                    self.state.output_scroll = total_lines - visible_lines;
                }
            }

            // Check if process completed
            if let Some(success) = process.succeeded() {
                self.state.output_lines.push(String::new());
                if success {
                    self.state.command_status = state::CommandStatus::Success;
                    // Update state based on which command was running
                    match self.state.view {
                        state::TerraformView::Init => {
                            self.state.initialized = true;
                            self.state
                                .output_lines
                                .push("Initialization completed successfully!".to_string());
                            self.state.message = Some("Initialized successfully".to_string());
                        }
                        state::TerraformView::Plan => {
                            self.state.output_lines.push("Plan completed.".to_string());
                        }
                        state::TerraformView::Apply => {
                            self.state
                                .output_lines
                                .push("Apply completed successfully!".to_string());
                            self.state.message = Some("Apply completed".to_string());
                        }
                        _ => {
                            self.state
                                .output_lines
                                .push("Command completed.".to_string());
                        }
                    }
                } else {
                    self.state.command_status = state::CommandStatus::Failed;
                    self.state.output_lines.push("Command failed!".to_string());
                    self.state.error = Some("Command failed".to_string());
                }
                // Final scroll
                let total = self.state.output_lines.len();
                if total > 18 {
                    self.state.output_scroll = total - 18;
                }
                self.active_process = None;
            }
        }
    }

    /// Run terraform plan (streaming)
    fn do_plan(&mut self) {
        if let Some(cwd) = self.cwd.clone() {
            match ops::start_command(&cwd, &["plan", "-no-color", "-input=false"]) {
                Ok(process) => {
                    self.active_process = Some(process);
                    self.state.command_status = state::CommandStatus::Running;
                    self.state.output_lines.clear();
                    self.state
                        .output_lines
                        .push("Running terraform plan...".to_string());
                    self.state.output_scroll = 0;
                    self.last_output_count = 0;
                    self.state.view = state::TerraformView::Plan;
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
        }
    }

    /// Run terraform apply (streaming)
    fn do_apply(&mut self) {
        if let Some(cwd) = self.cwd.clone() {
            match ops::start_command(
                &cwd,
                &["apply", "-auto-approve", "-no-color", "-input=false"],
            ) {
                Ok(process) => {
                    self.active_process = Some(process);
                    self.state.command_status = state::CommandStatus::Running;
                    self.state.output_lines.clear();
                    self.state
                        .output_lines
                        .push("Running terraform apply...".to_string());
                    self.state.output_scroll = 0;
                    self.last_output_count = 0;
                    self.state.view = state::TerraformView::Apply;
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
        }
    }

    /// Run terraform destroy
    fn do_destroy(&mut self) {
        if let Some(cwd) = self.cwd.clone() {
            self.state.set_loading("Destroying...");
            self.state.view = state::TerraformView::Output;
            match ops::destroy(&cwd) {
                Ok(lines) => {
                    self.state.output_lines = lines;
                    self.state.output_scroll = 0;
                    self.state.message = Some("Destroy completed".to_string());
                }
                Err(e) => {
                    self.state.output_lines = e.lines().map(String::from).collect();
                    self.state.output_scroll = 0;
                    self.state.error = Some("Destroy failed".to_string());
                }
            }
            self.state.clear_loading();
        }
    }

    /// Run terraform refresh
    fn do_refresh(&mut self) {
        if let Some(cwd) = self.cwd.clone() {
            self.state.set_loading("Refreshing...");
            self.state.view = state::TerraformView::Output;
            match ops::refresh(&cwd) {
                Ok(lines) => {
                    self.state.output_lines = lines;
                    self.state.output_scroll = 0;
                    self.state.message = Some("Refresh completed".to_string());
                }
                Err(e) => {
                    self.state.output_lines = e.lines().map(String::from).collect();
                    self.state.output_scroll = 0;
                    self.state.error = Some("Refresh failed".to_string());
                }
            }
            self.state.clear_loading();
        }
    }

    /// Run terraform validate
    fn do_validate(&mut self) {
        if let Some(cwd) = self.cwd.clone() {
            self.state.set_loading("Validating...");
            self.state.view = state::TerraformView::Output;
            match ops::validate(&cwd) {
                Ok(lines) => {
                    self.state.output_lines = lines;
                    self.state.output_scroll = 0;
                    self.state.message = Some("Validation passed".to_string());
                }
                Err(e) => {
                    self.state.output_lines = e.lines().map(String::from).collect();
                    self.state.output_scroll = 0;
                    self.state.error = Some("Validation failed".to_string());
                }
            }
            self.state.clear_loading();
        }
    }

    /// Select workspace
    fn select_workspace(&mut self, name: &str) {
        if let Some(cwd) = self.cwd.clone() {
            self.state.set_loading("Selecting workspace...");
            match ops::select_workspace(&cwd, name) {
                Ok(_) => {
                    self.state.message = Some(format!("Selected workspace: {}", name));
                    self.load_workspaces();
                    self.load_state();
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
            self.state.clear_loading();
        }
    }

    /// Delete workspace
    fn delete_workspace(&mut self, name: &str) {
        if let Some(cwd) = self.cwd.clone() {
            self.state.set_loading("Deleting workspace...");
            match ops::delete_workspace(&cwd, name) {
                Ok(_) => {
                    self.state.message = Some(format!("Deleted workspace: {}", name));
                    self.load_workspaces();
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
            self.state.clear_loading();
        }
    }

    /// Show resource detail
    fn show_resource(&mut self) {
        if let Some(resource) = self.state.selected_resource().cloned() {
            if let Some(cwd) = self.cwd.clone() {
                self.state.set_loading("Loading resource...");
                match ops::show_state_resource(&cwd, &resource.address) {
                    Ok(detail) => {
                        self.state.current_resource = Some(resource);
                        self.state.resource_detail = detail;
                        self.state.detail_scroll = 0;
                        self.state.view = state::TerraformView::StateDetail;
                    }
                    Err(e) => {
                        self.state.error = Some(e);
                    }
                }
                self.state.clear_loading();
            }
        }
    }

    /// Remove resource from state
    fn state_remove(&mut self, address: &str) {
        if let Some(cwd) = self.cwd.clone() {
            self.state.set_loading("Removing from state...");
            match ops::state_remove(&cwd, address) {
                Ok(_) => {
                    self.state.message = Some(format!("Removed: {}", address));
                    self.load_state();
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
            self.state.clear_loading();
        }
    }

    /// Handle menu keys
    fn handle_menu_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                self.state.move_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.state.move_down();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Execute selected menu item
                if let Some(item) = state::MenuItem::all().get(self.state.selected_menu) {
                    match item {
                        state::MenuItem::Init => self.do_init(),
                        state::MenuItem::Plan => self.do_plan(),
                        state::MenuItem::Apply => {
                            self.state.confirm_action = Some(state::ConfirmAction::Apply);
                            self.state.view = state::TerraformView::Confirm;
                        }
                        state::MenuItem::Destroy => {
                            self.state.confirm_action = Some(state::ConfirmAction::Destroy);
                            self.state.view = state::TerraformView::Confirm;
                        }
                        state::MenuItem::Refresh => self.do_refresh(),
                        state::MenuItem::Validate => self.do_validate(),
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('i') => {
                self.do_init();
                KeyHandleResult::Handled
            }
            KeyCode::Char('p') => {
                self.do_plan();
                KeyHandleResult::Handled
            }
            KeyCode::Char('a') => {
                self.state.confirm_action = Some(state::ConfirmAction::Apply);
                self.state.view = state::TerraformView::Confirm;
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') => {
                self.state.confirm_action = Some(state::ConfirmAction::Destroy);
                self.state.view = state::TerraformView::Confirm;
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') => {
                self.do_refresh();
                KeyHandleResult::Handled
            }
            KeyCode::Char('v') => {
                self.do_validate();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle workspaces keys
    fn handle_workspaces_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                self.state.move_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.state.move_down();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if let Some(workspace) = self.state.selected_workspace() {
                    if !workspace.is_current {
                        let name = workspace.name.clone();
                        self.select_workspace(&name);
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') => {
                if let Some(workspace) = self.state.selected_workspace() {
                    if !workspace.is_current {
                        self.state.confirm_action = Some(state::ConfirmAction::WorkspaceDelete(
                            workspace.name.clone(),
                        ));
                        self.state.view = state::TerraformView::Confirm;
                    } else {
                        self.state.error = Some("Cannot delete current workspace".to_string());
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') => {
                self.load_workspaces();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle state keys
    fn handle_state_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                self.state.move_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.state.move_down();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.show_resource();
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') => {
                if let Some(resource) = self.state.selected_resource() {
                    self.state.confirm_action =
                        Some(state::ConfirmAction::StateRemove(resource.address.clone()));
                    self.state.view = state::TerraformView::Confirm;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') => {
                self.load_state();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle state detail keys
    fn handle_state_detail_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                if self.state.detail_scroll > 0 {
                    self.state.detail_scroll -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                if self.state.detail_scroll < self.state.resource_detail.len().saturating_sub(1) {
                    self.state.detail_scroll += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.state.current_resource = None;
                self.state.resource_detail.clear();
                self.state.view = state::TerraformView::State;
                self.state.tab = state::TerraformTab::State;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle output view keys
    fn handle_output_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let command_done = self.state.command_status.is_done();

        match key.code {
            KeyCode::Tab if command_done => {
                // Switch to Workspaces tab when command is done
                self.state.tab = state::TerraformTab::Workspaces;
                self.state.view = state::TerraformView::Workspaces;
                self.state.command_status = state::CommandStatus::NotStarted;
                self.load_workspaces();
                KeyHandleResult::Handled
            }
            KeyCode::BackTab if command_done => {
                // Switch to State tab
                self.state.tab = state::TerraformTab::State;
                self.state.view = state::TerraformView::State;
                self.state.command_status = state::CommandStatus::NotStarted;
                self.load_state();
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.output_scroll > 0 {
                    self.state.output_scroll -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.state.output_scroll < self.state.output_lines.len().saturating_sub(1) {
                    self.state.output_scroll += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::PageUp => {
                self.state.output_scroll = self.state.output_scroll.saturating_sub(10);
                KeyHandleResult::Handled
            }
            KeyCode::PageDown => {
                let max = self.state.output_lines.len().saturating_sub(18);
                self.state.output_scroll = (self.state.output_scroll + 10).min(max);
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                self.state.output_scroll = 0;
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                let max = self.state.output_lines.len().saturating_sub(18);
                self.state.output_scroll = max;
                KeyHandleResult::Handled
            }
            KeyCode::Enter if command_done => {
                self.state.output_lines.clear();
                self.state.output_scroll = 0;
                self.state.view = state::TerraformView::Menu;
                self.state.tab = state::TerraformTab::Operations;
                self.state.command_status = state::CommandStatus::NotStarted;
                KeyHandleResult::Handled
            }
            KeyCode::Esc if command_done => {
                self.state.output_lines.clear();
                self.state.output_scroll = 0;
                self.state.view = state::TerraformView::Menu;
                self.state.tab = state::TerraformTab::Operations;
                self.state.command_status = state::CommandStatus::NotStarted;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle confirm dialog keys
    fn handle_confirm_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(action) = self.state.confirm_action.take() {
                    match action {
                        state::ConfirmAction::Apply => {
                            self.do_apply();
                        }
                        state::ConfirmAction::Destroy => {
                            self.do_destroy();
                        }
                        state::ConfirmAction::StateRemove(addr) => {
                            self.state_remove(&addr);
                            self.state.view = state::TerraformView::State;
                            self.state.tab = state::TerraformTab::State;
                        }
                        state::ConfirmAction::WorkspaceDelete(name) => {
                            self.delete_workspace(&name);
                            self.state.view = state::TerraformView::Workspaces;
                            self.state.tab = state::TerraformTab::Workspaces;
                        }
                    }
                } else {
                    self.state.view = state::TerraformView::Menu;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.state.confirm_action = None;
                self.state.view = state::TerraformView::Menu;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}

impl Default for TerraformPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for TerraformPlugin {
    fn id(&self) -> &str {
        "terraform"
    }

    fn name(&self) -> &str {
        "Terraform"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_modal: true,
            has_menu: true,
            ..Default::default()
        }
    }

    fn is_available(&self, cwd: &PathBuf) -> bool {
        // Check for terraform files in directory
        cwd.join("main.tf").exists()
            || cwd.join("terraform.tf").exists()
            || cwd
                .read_dir()
                .map(|d| {
                    d.filter_map(|e| e.ok())
                        .any(|e| e.path().extension().map(|ext| ext == "tf").unwrap_or(false))
                })
                .unwrap_or(false)
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: "terraform".to_string(),
            name: "Terraform".to_string(),
            description: "Infrastructure as code".to_string(),
            category: PluginCategory::Tools,
            key: 'T',
        })
    }

    fn launch(&mut self, cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.initialize(cwd);
        self.modal_open = true;
        Ok(())
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        // Open with 't' key
        if key.code == KeyCode::Char('t') {
            self.initialize(cwd);
            self.modal_open = true;
            return KeyHandleResult::OpenModal;
        }
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Clear messages
        self.state.message = None;
        self.state.error = None;

        // Tab switching
        match key.code {
            KeyCode::Esc => {
                match self.state.view {
                    state::TerraformView::Menu
                    | state::TerraformView::Workspaces
                    | state::TerraformView::State => {
                        self.state.reset();
                        self.modal_open = false;
                        return KeyHandleResult::CloseModal;
                    }
                    _ => {
                        // Handle in view-specific handler
                    }
                }
            }
            KeyCode::Tab => match self.state.view {
                state::TerraformView::Menu
                | state::TerraformView::Workspaces
                | state::TerraformView::State => {
                    self.state.tab = self.state.tab.next();
                    self.state.view = match self.state.tab {
                        state::TerraformTab::Operations => state::TerraformView::Menu,
                        state::TerraformTab::Workspaces => {
                            self.load_workspaces();
                            state::TerraformView::Workspaces
                        }
                        state::TerraformTab::State => {
                            self.load_state();
                            state::TerraformView::State
                        }
                    };
                    return KeyHandleResult::Handled;
                }
                _ => {}
            },
            KeyCode::BackTab => match self.state.view {
                state::TerraformView::Menu
                | state::TerraformView::Workspaces
                | state::TerraformView::State => {
                    self.state.tab = self.state.tab.prev();
                    self.state.view = match self.state.tab {
                        state::TerraformTab::Operations => state::TerraformView::Menu,
                        state::TerraformTab::Workspaces => {
                            self.load_workspaces();
                            state::TerraformView::Workspaces
                        }
                        state::TerraformTab::State => {
                            self.load_state();
                            state::TerraformView::State
                        }
                    };
                    return KeyHandleResult::Handled;
                }
                _ => {}
            },
            _ => {}
        }

        // View-specific handling
        match self.state.view {
            state::TerraformView::Menu => self.handle_menu_key(key),
            state::TerraformView::Workspaces => self.handle_workspaces_key(key),
            state::TerraformView::State => self.handle_state_key(key),
            state::TerraformView::StateDetail => self.handle_state_detail_key(key),
            state::TerraformView::Init
            | state::TerraformView::Plan
            | state::TerraformView::Apply
            | state::TerraformView::Output => self.handle_output_key(key),
            state::TerraformView::Confirm => self.handle_confirm_key(key),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn tick(&mut self) {
        // Poll active terraform process for streaming output
        self.poll_process();
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_terraform_modal(frame, area, &self.state, colors);
    }
}

inventory::submit! { PluginRegistration::new("terraform", || Box::new(TerraformPlugin::new())) }
