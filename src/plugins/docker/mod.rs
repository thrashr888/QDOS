//! Docker plugin - Container management
//!
//! Provides Docker container, image, volume, and network management.

mod modal;
mod ops;
mod state;

use crate::app::ThemeColors;
use crate::plugins::{AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, Frame};
use std::any::Any;
use std::path::PathBuf;

pub use state::{DockerState, DockerTab, DockerView};

/// Docker plugin
pub struct DockerPlugin {
    state: DockerState,
    modal_open: bool,
    /// Active build process (if any)
    build_process: Option<ops::BuildProcess>,
    /// Last known output line count (for auto-scroll detection)
    last_output_count: usize,
}

impl DockerPlugin {
    pub fn new() -> Self {
        Self {
            state: DockerState::new(),
            modal_open: false,
            build_process: None,
            last_output_count: 0,
        }
    }

    /// Initialize and load data
    fn initialize(&mut self, cwd: &PathBuf) {
        self.state.cwd = Some(cwd.clone());
        self.state.docker_available = ops::check_docker();
        self.state.context = state::DockerContext::None;
        if self.state.docker_available {
            self.refresh_current_tab();
        }
    }

    /// Initialize with file context (Dockerfile or docker-compose.yml)
    fn initialize_with_context(&mut self, cwd: &PathBuf, file: &PathBuf) {
        self.state.cwd = Some(cwd.clone());
        self.state.docker_available = ops::check_docker();

        // Detect context from selected file
        let file_name = file
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if file_name == "dockerfile" || file_name.starts_with("dockerfile.") {
            self.state.context = state::DockerContext::Dockerfile(file.clone());
            // Set default tag from folder name
            if let Some(folder) = cwd.file_name() {
                self.state.build_tag = folder.to_string_lossy().to_lowercase().replace(' ', "-");
            }
            self.state.view = state::DockerView::Build;
        } else if file_name == "docker-compose.yml"
            || file_name == "docker-compose.yaml"
            || file_name == "compose.yml"
            || file_name == "compose.yaml"
        {
            self.state.context = state::DockerContext::Compose(file.clone());
            self.state.view = state::DockerView::Compose;
            self.load_compose_services();
        } else {
            self.state.context = state::DockerContext::None;
            if self.state.docker_available {
                self.refresh_current_tab();
            }
        }
    }

    /// Load compose services
    fn load_compose_services(&mut self) {
        if let state::DockerContext::Compose(ref path) = self.state.context.clone() {
            self.state.set_loading("Loading services...");
            match ops::compose_ps(&path) {
                Ok(services) => {
                    self.state.compose_services = services;
                    self.state.selected_service = 0;
                    self.state.service_scroll = 0;
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
            self.state.clear_loading();
        }
    }

    /// Run docker build (streaming version)
    fn run_build(&mut self) {
        if let state::DockerContext::Dockerfile(ref dockerfile) = self.state.context.clone() {
            let dir = dockerfile.parent().unwrap_or(std::path::Path::new("."));
            let tag = self.state.build_tag.clone();
            if tag.is_empty() {
                self.state.error = Some("Tag is required".to_string());
                return;
            }

            // Start streaming build
            match ops::start_build(dir, &tag) {
                Ok(process) => {
                    self.build_process = Some(process);
                    self.state.build_status = state::BuildStatus::Running;
                    self.state.output_lines.clear();
                    self.state.output_scroll = 0;
                    self.last_output_count = 0;
                    self.state.view = state::DockerView::BuildOutput;
                    // Add initial message
                    self.state
                        .output_lines
                        .push(format!("Building image: {}...", tag));
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
        }
    }

    /// Poll build process for new output
    fn poll_build(&mut self) {
        if let Some(ref process) = self.build_process {
            // Get current output
            let output = process.get_output();
            let new_count = output.len();

            // If we have new output, update state
            if new_count > self.last_output_count {
                // Append new lines
                for line in output.iter().skip(self.last_output_count) {
                    self.state.output_lines.push(line.clone());
                }
                self.last_output_count = new_count;

                // Auto-scroll to bottom if following
                let total_lines = self.state.output_lines.len();
                if total_lines > 0 {
                    // Auto-scroll to keep bottom visible (show last ~18 lines)
                    let visible_lines = 18;
                    if total_lines > visible_lines {
                        self.state.output_scroll = total_lines - visible_lines;
                    }
                }
            }

            // Check if build completed
            if let Some(success) = process.succeeded() {
                if success {
                    self.state.build_status = state::BuildStatus::Success;
                    self.state.output_lines.push(String::new());
                    self.state
                        .output_lines
                        .push("Build completed successfully!".to_string());
                    self.state.message = Some(format!("Built image: {}", self.state.build_tag));
                } else {
                    self.state.build_status = state::BuildStatus::Failed;
                    self.state.output_lines.push(String::new());
                    self.state.output_lines.push("Build failed!".to_string());
                    self.state.error = Some("Build failed".to_string());
                }
                // Final scroll to bottom
                let total = self.state.output_lines.len();
                if total > 18 {
                    self.state.output_scroll = total - 18;
                }
                // Clear the process
                self.build_process = None;
            }
        }
    }

    /// Run compose up
    fn run_compose_up(&mut self) {
        if let state::DockerContext::Compose(ref path) = self.state.context.clone() {
            self.state.set_loading("Starting services...");
            match ops::compose_up(&path) {
                Ok(_) => {
                    self.state.message = Some("Services started".to_string());
                    self.load_compose_services();
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
            self.state.clear_loading();
        }
    }

    /// Run compose down
    fn run_compose_down(&mut self) {
        if let state::DockerContext::Compose(ref path) = self.state.context.clone() {
            self.state.set_loading("Stopping services...");
            match ops::compose_down(&path) {
                Ok(_) => {
                    self.state.message = Some("Services stopped".to_string());
                    self.load_compose_services();
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
            self.state.clear_loading();
        }
    }

    /// Load compose service logs
    fn load_compose_logs(&mut self, service: &str) {
        if let state::DockerContext::Compose(ref path) = self.state.context.clone() {
            self.state.set_loading("Loading logs...");
            match ops::compose_logs(&path, service, 100) {
                Ok(lines) => {
                    self.state.output_lines = lines;
                    self.state.output_scroll = 0;
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
            self.state.clear_loading();
        }
    }

    /// Restart a compose service
    fn restart_compose_service(&mut self, service: &str) {
        if let state::DockerContext::Compose(ref path) = self.state.context.clone() {
            self.state.set_loading("Restarting service...");
            match ops::compose_restart(&path, service) {
                Ok(_) => {
                    self.state.message = Some(format!("Restarted {}", service));
                    self.load_compose_services();
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
            self.state.clear_loading();
        }
    }

    /// Refresh data for current tab
    fn refresh_current_tab(&mut self) {
        match self.state.tab {
            DockerTab::Containers => self.load_containers(),
            DockerTab::Images => self.load_images(),
            DockerTab::Volumes => self.load_volumes(),
            DockerTab::Networks => self.load_networks(),
        }
    }

    /// Load containers
    fn load_containers(&mut self) {
        self.state.set_loading("Loading containers...");
        match ops::list_containers(self.state.show_all_containers) {
            Ok(containers) => {
                self.state.containers = containers;
                self.state.selected_container = 0;
                self.state.container_scroll = 0;
            }
            Err(e) => {
                self.state.error = Some(e);
            }
        }
        self.state.clear_loading();
    }

    /// Load images
    fn load_images(&mut self) {
        self.state.set_loading("Loading images...");
        match ops::list_images() {
            Ok(images) => {
                self.state.images = images;
                self.state.selected_image = 0;
                self.state.image_scroll = 0;
            }
            Err(e) => {
                self.state.error = Some(e);
            }
        }
        self.state.clear_loading();
    }

    /// Load volumes
    fn load_volumes(&mut self) {
        self.state.set_loading("Loading volumes...");
        match ops::list_volumes() {
            Ok(volumes) => {
                self.state.volumes = volumes;
                self.state.selected_volume = 0;
                self.state.volume_scroll = 0;
            }
            Err(e) => {
                self.state.error = Some(e);
            }
        }
        self.state.clear_loading();
    }

    /// Load networks
    fn load_networks(&mut self) {
        self.state.set_loading("Loading networks...");
        match ops::list_networks() {
            Ok(networks) => {
                self.state.networks = networks;
                self.state.selected_network = 0;
                self.state.network_scroll = 0;
            }
            Err(e) => {
                self.state.error = Some(e);
            }
        }
        self.state.clear_loading();
    }

    /// Handle container actions
    fn handle_container_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                self.state.move_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.state.move_down();
                KeyHandleResult::Handled
            }
            KeyCode::Char('s') => {
                // Start container
                if let Some(container) = self.state.selected_container() {
                    let id = container.id.clone();
                    self.state.set_loading("Starting container...");
                    match ops::start_container(&id) {
                        Ok(_) => {
                            self.state.message = Some("Container started".to_string());
                            self.load_containers();
                        }
                        Err(e) => {
                            self.state.error = Some(e);
                        }
                    }
                    self.state.clear_loading();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('t') => {
                // Stop container (with confirmation)
                if let Some(container) = self.state.selected_container() {
                    self.state.confirm_action =
                        Some(state::ConfirmAction::StopContainer(container.name.clone()));
                    self.state.view = DockerView::Confirm;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::NONE) => {
                // Restart container
                if let Some(container) = self.state.selected_container() {
                    let id = container.id.clone();
                    self.state.set_loading("Restarting container...");
                    match ops::restart_container(&id) {
                        Ok(_) => {
                            self.state.message = Some("Container restarted".to_string());
                            self.load_containers();
                        }
                        Err(e) => {
                            self.state.error = Some(e);
                        }
                    }
                    self.state.clear_loading();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('l') => {
                // View logs
                if let Some(container) = self.state.selected_container() {
                    let id = container.id.clone();
                    self.state.set_loading("Loading logs...");
                    match ops::get_logs(&id, 100) {
                        Ok(lines) => {
                            self.state.output_lines = lines;
                            self.state.output_scroll = 0;
                            self.state.view = DockerView::Logs;
                        }
                        Err(e) => {
                            self.state.error = Some(e);
                        }
                    }
                    self.state.clear_loading();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('x') => {
                // Exec command
                if self.state.selected_container().is_some() {
                    self.state.exec_command.clear();
                    self.state.exec_cursor = 0;
                    self.state.view = DockerView::Exec;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('i') => {
                // Inspect
                if let Some(container) = self.state.selected_container() {
                    let id = container.id.clone();
                    self.state.set_loading("Inspecting...");
                    match ops::inspect(&id) {
                        Ok(output) => {
                            self.state.output_lines = output.lines().map(String::from).collect();
                            self.state.output_scroll = 0;
                            self.state.view = DockerView::Inspect;
                        }
                        Err(e) => {
                            self.state.error = Some(e);
                        }
                    }
                    self.state.clear_loading();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') => {
                // Remove container (with confirmation)
                if let Some(container) = self.state.selected_container() {
                    self.state.confirm_action = Some(state::ConfirmAction::RemoveContainer(
                        container.name.clone(),
                    ));
                    self.state.view = DockerView::Confirm;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('a') => {
                // Toggle show all containers
                self.state.show_all_containers = !self.state.show_all_containers;
                self.load_containers();
                KeyHandleResult::Handled
            }
            KeyCode::Char('p') => {
                // Pull image
                self.state.pull_image_name.clear();
                self.state.pull_cursor = 0;
                self.state.view = DockerView::Pull;
                KeyHandleResult::Handled
            }
            KeyCode::Char('P') => {
                // Prune stopped containers
                self.state.confirm_action = Some(state::ConfirmAction::PruneContainers);
                self.state.view = DockerView::Confirm;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle image actions
    fn handle_image_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                self.state.move_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.state.move_down();
                KeyHandleResult::Handled
            }
            KeyCode::Char('p') => {
                // Pull image
                self.state.pull_image_name.clear();
                self.state.pull_cursor = 0;
                self.state.view = DockerView::Pull;
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') => {
                // Remove image (with confirmation)
                if let Some(image) = self.state.selected_image() {
                    self.state.confirm_action =
                        Some(state::ConfirmAction::RemoveImage(image.full_name()));
                    self.state.view = DockerView::Confirm;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('i') => {
                // Inspect
                if let Some(image) = self.state.selected_image() {
                    let id = image.id.clone();
                    self.state.set_loading("Inspecting...");
                    match ops::inspect(&id) {
                        Ok(output) => {
                            self.state.output_lines = output.lines().map(String::from).collect();
                            self.state.output_scroll = 0;
                            self.state.view = DockerView::Inspect;
                        }
                        Err(e) => {
                            self.state.error = Some(e);
                        }
                    }
                    self.state.clear_loading();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('P') => {
                // Prune unused images
                self.state.confirm_action = Some(state::ConfirmAction::PruneImages);
                self.state.view = DockerView::Confirm;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle volume actions
    fn handle_volume_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                self.state.move_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.state.move_down();
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') => {
                // Remove volume (with confirmation)
                if let Some(volume) = self.state.selected_volume() {
                    self.state.confirm_action =
                        Some(state::ConfirmAction::RemoveVolume(volume.name.clone()));
                    self.state.view = DockerView::Confirm;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle network actions
    fn handle_network_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                self.state.move_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.state.move_down();
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') => {
                // Remove network (with confirmation)
                if let Some(network) = self.state.selected_network() {
                    self.state.confirm_action =
                        Some(state::ConfirmAction::RemoveNetwork(network.name.clone()));
                    self.state.view = DockerView::Confirm;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle logs view keys
    fn handle_logs_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                if self.state.output_scroll > 0 {
                    self.state.output_scroll -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                if self.state.output_scroll < self.state.output_lines.len().saturating_sub(1) {
                    self.state.output_scroll += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('f') => {
                // Toggle follow mode (would need async implementation)
                self.state.following_logs = !self.state.following_logs;
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.state.view = DockerView::Containers;
                self.state.output_lines.clear();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle inspect view keys
    fn handle_inspect_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                if self.state.output_scroll > 0 {
                    self.state.output_scroll -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                if self.state.output_scroll < self.state.output_lines.len().saturating_sub(1) {
                    self.state.output_scroll += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                // Go back to appropriate list view
                self.state.view = match self.state.tab {
                    DockerTab::Containers => DockerView::Containers,
                    DockerTab::Images => DockerView::Images,
                    DockerTab::Volumes => DockerView::Volumes,
                    DockerTab::Networks => DockerView::Networks,
                };
                self.state.output_lines.clear();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle pull input keys
    fn handle_pull_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Char(c) => {
                self.state.insert_pull_char(c);
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.backspace_pull();
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                if self.state.pull_cursor > 0 {
                    self.state.pull_cursor -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                if self.state.pull_cursor < self.state.pull_image_name.len() {
                    self.state.pull_cursor += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if !self.state.pull_image_name.is_empty() {
                    let name = self.state.pull_image_name.clone();
                    self.state.set_loading(&format!("Pulling {}...", name));
                    match ops::pull_image(&name) {
                        Ok(_) => {
                            self.state.message = Some(format!("Pulled {}", name));
                            self.load_images();
                        }
                        Err(e) => {
                            self.state.error = Some(e);
                        }
                    }
                    self.state.clear_loading();
                    self.state.pull_image_name.clear();
                    self.state.view = DockerView::Images;
                    self.state.tab = DockerTab::Images;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.state.pull_image_name.clear();
                self.state.view = match self.state.tab {
                    DockerTab::Containers => DockerView::Containers,
                    DockerTab::Images => DockerView::Images,
                    DockerTab::Volumes => DockerView::Volumes,
                    DockerTab::Networks => DockerView::Networks,
                };
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle exec input keys
    fn handle_exec_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Char(c) => {
                self.state.insert_exec_char(c);
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.backspace_exec();
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                if self.state.exec_cursor > 0 {
                    self.state.exec_cursor -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                if self.state.exec_cursor < self.state.exec_command.len() {
                    self.state.exec_cursor += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if !self.state.exec_command.is_empty() {
                    if let Some(container) = self.state.selected_container() {
                        let id = container.id.clone();
                        let cmd = self.state.exec_command.clone();
                        self.state.set_loading("Executing...");
                        match ops::exec_command(&id, &cmd) {
                            Ok(output) => {
                                self.state.output_lines =
                                    output.lines().map(String::from).collect();
                                self.state.output_scroll = 0;
                                self.state.view = DockerView::Inspect; // Reuse inspect view for output
                            }
                            Err(e) => {
                                self.state.error = Some(e);
                                self.state.view = DockerView::Containers;
                            }
                        }
                        self.state.clear_loading();
                        self.state.exec_command.clear();
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.state.exec_command.clear();
                self.state.view = DockerView::Containers;
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
                    self.execute_confirmed_action(action);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.state.confirm_action = None;
                self.state.view = match self.state.tab {
                    DockerTab::Containers => DockerView::Containers,
                    DockerTab::Images => DockerView::Images,
                    DockerTab::Volumes => DockerView::Volumes,
                    DockerTab::Networks => DockerView::Networks,
                };
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle keys in Build view
    fn handle_build_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Enter => {
                self.run_build();
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.state.view = DockerView::Containers;
                self.state.context = state::DockerContext::None;
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.build_tag.insert(self.state.build_cursor, c);
                self.state.build_cursor += 1;
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                if self.state.build_cursor > 0 {
                    self.state.build_cursor -= 1;
                    self.state.build_tag.remove(self.state.build_cursor);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                if self.state.build_cursor > 0 {
                    self.state.build_cursor -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                if self.state.build_cursor < self.state.build_tag.len() {
                    self.state.build_cursor += 1;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle keys in BuildOutput view
    fn handle_build_output_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        // Only allow navigation when build is complete
        let build_done = self.state.build_status.is_done();

        match key.code {
            KeyCode::Tab if build_done => {
                // Switch to Images tab when build is done
                self.state.tab = DockerTab::Images;
                self.state.view = DockerView::Images;
                self.state.context = state::DockerContext::None;
                self.state.build_status = state::BuildStatus::NotStarted;
                self.load_images();
                KeyHandleResult::Handled
            }
            KeyCode::BackTab if build_done => {
                // Switch to Containers tab
                self.state.tab = DockerTab::Containers;
                self.state.view = DockerView::Containers;
                self.state.context = state::DockerContext::None;
                self.state.build_status = state::BuildStatus::NotStarted;
                self.load_containers();
                KeyHandleResult::Handled
            }
            KeyCode::Enter if build_done => {
                // Go to Images view
                self.state.tab = DockerTab::Images;
                self.state.view = DockerView::Images;
                self.state.context = state::DockerContext::None;
                self.state.build_status = state::BuildStatus::NotStarted;
                self.load_images();
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                if build_done {
                    // Close and go to Images
                    self.state.tab = DockerTab::Images;
                    self.state.view = DockerView::Images;
                    self.state.context = state::DockerContext::None;
                    self.state.build_status = state::BuildStatus::NotStarted;
                    self.load_images();
                }
                // If build is still running, Esc does nothing (can't cancel yet)
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
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle keys in Compose view
    fn handle_compose_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = DockerView::Containers;
                self.state.context = state::DockerContext::None;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.selected_service > 0 {
                    self.state.selected_service -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.state.selected_service < self.state.compose_services.len().saturating_sub(1)
                {
                    self.state.selected_service += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('u') => {
                self.state.confirm_action = Some(state::ConfirmAction::ComposeUp);
                self.state.view = DockerView::Confirm;
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') => {
                self.state.confirm_action = Some(state::ConfirmAction::ComposeDown);
                self.state.view = DockerView::Confirm;
                KeyHandleResult::Handled
            }
            KeyCode::Char('l') => {
                if let Some(service) = self.state.compose_services.get(self.state.selected_service)
                {
                    let name = service.name.clone();
                    self.load_compose_logs(&name);
                    self.state.view = DockerView::ComposeLogs;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('R') => {
                if let Some(service) = self.state.compose_services.get(self.state.selected_service)
                {
                    let name = service.name.clone();
                    self.state.confirm_action = Some(state::ConfirmAction::ComposeRestart(name));
                    self.state.view = DockerView::Confirm;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') => {
                self.load_compose_services();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle keys in ComposeLogs view
    fn handle_compose_logs_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = DockerView::Compose;
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
            _ => KeyHandleResult::Handled,
        }
    }

    /// Execute a confirmed action
    fn execute_confirmed_action(&mut self, action: state::ConfirmAction) {
        match action {
            state::ConfirmAction::StopContainer(name) => {
                // Find container by name
                if let Some(container) = self
                    .state
                    .containers
                    .iter()
                    .find(|c| c.name == name)
                    .cloned()
                {
                    self.state.set_loading("Stopping container...");
                    match ops::stop_container(&container.id) {
                        Ok(_) => {
                            self.state.message = Some("Container stopped".to_string());
                            self.load_containers();
                        }
                        Err(e) => {
                            self.state.error = Some(e);
                        }
                    }
                    self.state.clear_loading();
                }
                self.state.view = DockerView::Containers;
            }
            state::ConfirmAction::RemoveContainer(name) => {
                if let Some(container) = self
                    .state
                    .containers
                    .iter()
                    .find(|c| c.name == name)
                    .cloned()
                {
                    self.state.set_loading("Removing container...");
                    match ops::remove_container(&container.id, true) {
                        Ok(_) => {
                            self.state.message = Some("Container removed".to_string());
                            self.load_containers();
                        }
                        Err(e) => {
                            self.state.error = Some(e);
                        }
                    }
                    self.state.clear_loading();
                }
                self.state.view = DockerView::Containers;
            }
            state::ConfirmAction::RemoveImage(name) => {
                if let Some(image) = self
                    .state
                    .images
                    .iter()
                    .find(|i| i.full_name() == name)
                    .cloned()
                {
                    self.state.set_loading("Removing image...");
                    match ops::remove_image(&image.id) {
                        Ok(_) => {
                            self.state.message = Some("Image removed".to_string());
                            self.load_images();
                        }
                        Err(e) => {
                            self.state.error = Some(e);
                        }
                    }
                    self.state.clear_loading();
                }
                self.state.view = DockerView::Images;
            }
            state::ConfirmAction::RemoveVolume(name) => {
                self.state.set_loading("Removing volume...");
                match ops::remove_volume(&name) {
                    Ok(_) => {
                        self.state.message = Some("Volume removed".to_string());
                        self.load_volumes();
                    }
                    Err(e) => {
                        self.state.error = Some(e);
                    }
                }
                self.state.clear_loading();
                self.state.view = DockerView::Volumes;
            }
            state::ConfirmAction::RemoveNetwork(name) => {
                self.state.set_loading("Removing network...");
                match ops::remove_network(&name) {
                    Ok(_) => {
                        self.state.message = Some("Network removed".to_string());
                        self.load_networks();
                    }
                    Err(e) => {
                        self.state.error = Some(e);
                    }
                }
                self.state.clear_loading();
                self.state.view = DockerView::Networks;
            }
            state::ConfirmAction::PruneContainers => {
                self.state.set_loading("Pruning containers...");
                match ops::prune_containers() {
                    Ok(_) => {
                        self.state.message = Some("Containers pruned".to_string());
                        self.load_containers();
                    }
                    Err(e) => {
                        self.state.error = Some(e);
                    }
                }
                self.state.clear_loading();
                self.state.view = DockerView::Containers;
            }
            state::ConfirmAction::PruneImages => {
                self.state.set_loading("Pruning images...");
                match ops::prune_images() {
                    Ok(_) => {
                        self.state.message = Some("Images pruned".to_string());
                        self.load_images();
                    }
                    Err(e) => {
                        self.state.error = Some(e);
                    }
                }
                self.state.clear_loading();
                self.state.view = DockerView::Images;
            }
            state::ConfirmAction::ComposeUp => {
                self.run_compose_up();
                self.state.view = DockerView::Compose;
            }
            state::ConfirmAction::ComposeDown => {
                self.run_compose_down();
                self.state.view = DockerView::Compose;
            }
            state::ConfirmAction::ComposeRestart(name) => {
                self.restart_compose_service(&name);
                self.state.view = DockerView::Compose;
            }
        }
    }
}

impl Default for DockerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for DockerPlugin {
    fn id(&self) -> &str {
        "docker"
    }

    fn name(&self) -> &str {
        "Docker"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_modal: true,
            has_menu: true,
            ..Default::default()
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        ops::check_docker()
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: "docker".to_string(),
            name: "Docker".to_string(),
            description: "Container management".to_string(),
            category: PluginCategory::Tools,
            key: 'O',
        })
    }

    fn launch(&mut self, cwd: &PathBuf, selected_file: Option<&PathBuf>) -> Result<(), String> {
        if let Some(file) = selected_file {
            self.initialize_with_context(cwd, file);
        } else {
            self.initialize(cwd);
        }
        self.modal_open = true;
        Ok(())
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        // Open with 'o' key
        if key.code == KeyCode::Char('o') && key.modifiers.is_empty() {
            self.modal_open = true;
            self.initialize(cwd);
            return KeyHandleResult::OpenModal;
        }
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Clear messages on any key
        self.state.message = None;
        self.state.error = None;

        // Global modal keys
        match key.code {
            KeyCode::Esc => {
                // Esc behavior depends on current view
                match self.state.view {
                    DockerView::Containers
                    | DockerView::Images
                    | DockerView::Volumes
                    | DockerView::Networks => {
                        self.state.reset();
                        self.modal_open = false;
                        return KeyHandleResult::CloseModal;
                    }
                    _ => {
                        // Handle in view-specific handler
                    }
                }
            }
            KeyCode::Tab => {
                // Tab switching only in list views
                match self.state.view {
                    DockerView::Containers
                    | DockerView::Images
                    | DockerView::Volumes
                    | DockerView::Networks => {
                        self.state.tab = self.state.tab.next();
                        self.state.view = match self.state.tab {
                            DockerTab::Containers => DockerView::Containers,
                            DockerTab::Images => DockerView::Images,
                            DockerTab::Volumes => DockerView::Volumes,
                            DockerTab::Networks => DockerView::Networks,
                        };
                        self.refresh_current_tab();
                        return KeyHandleResult::Handled;
                    }
                    _ => {}
                }
            }
            KeyCode::BackTab => {
                // Shift+Tab for reverse tab switching
                match self.state.view {
                    DockerView::Containers
                    | DockerView::Images
                    | DockerView::Volumes
                    | DockerView::Networks => {
                        self.state.tab = self.state.tab.prev();
                        self.state.view = match self.state.tab {
                            DockerTab::Containers => DockerView::Containers,
                            DockerTab::Images => DockerView::Images,
                            DockerTab::Volumes => DockerView::Volumes,
                            DockerTab::Networks => DockerView::Networks,
                        };
                        self.refresh_current_tab();
                        return KeyHandleResult::Handled;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // View-specific key handling
        match self.state.view {
            DockerView::Containers => self.handle_container_key(key),
            DockerView::Images => self.handle_image_key(key),
            DockerView::Volumes => self.handle_volume_key(key),
            DockerView::Networks => self.handle_network_key(key),
            DockerView::Logs => self.handle_logs_key(key),
            DockerView::Inspect => self.handle_inspect_key(key),
            DockerView::Pull => self.handle_pull_key(key),
            DockerView::Exec => self.handle_exec_key(key),
            DockerView::Confirm => self.handle_confirm_key(key),
            DockerView::Build => self.handle_build_key(key),
            DockerView::BuildOutput => self.handle_build_output_key(key),
            DockerView::Compose => self.handle_compose_key(key),
            DockerView::ComposeLogs => self.handle_compose_logs_key(key),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn tick(&mut self) {
        // Poll build process for streaming output
        self.poll_build();
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_docker_modal(frame, area, &self.state, colors);
    }
}
