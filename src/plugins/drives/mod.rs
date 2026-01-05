//! Drives plugin (F3)
//!
//! Shows mounted volumes and network drives. F3 is the classic "Change Drive"
//! key from DOS.

mod modal;
pub mod state;

use crate::plugins::{
    KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem, PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{DrivesState, VolumeEntry, VolumeType};
use std::any::Any;
use std::fs;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::Command;

/// Drives plugin for browsing mounted volumes
pub struct DrivesPlugin {
    initialized: bool,
    pub state: DrivesState,
}

impl Default for DrivesPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DrivesPlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
            state: DrivesState::new(),
        }
    }

    /// Refresh the list of mounted volumes
    fn refresh_volumes(&mut self) {
        self.state.volumes.clear();

        #[cfg(target_os = "macos")]
        {
            self.refresh_macos_volumes();
        }

        #[cfg(not(target_os = "macos"))]
        {
            self.refresh_unix_volumes();
        }
    }

    #[cfg(target_os = "macos")]
    fn refresh_macos_volumes(&mut self) {
        // Read /Volumes directory
        if let Ok(entries) = fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Skip "Macintosh HD" symlink if it points to /
                if name == "Macintosh HD" && path.read_link().is_ok() {
                    continue;
                }

                // Determine volume type
                let volume_type = self.detect_volume_type(&path);

                // Get filesystem info
                let (total_size, free_space, filesystem) = self.get_fs_info(&path);

                // Check if writable
                let writable = self.check_writable(&path);

                self.state.volumes.push(VolumeEntry {
                    name,
                    path: path.clone(),
                    volume_type,
                    mount_point: path.to_string_lossy().to_string(),
                    filesystem,
                    total_size,
                    free_space,
                    writable,
                });
            }
        }

        // Add root filesystem
        self.state.volumes.insert(
            0,
            VolumeEntry {
                name: "Macintosh HD".to_string(),
                path: PathBuf::from("/"),
                volume_type: VolumeType::Local,
                mount_point: "/".to_string(),
                filesystem: "apfs".to_string(),
                total_size: self.get_fs_info(&PathBuf::from("/")).0,
                free_space: self.get_fs_info(&PathBuf::from("/")).1,
                writable: true,
            },
        );

        // Sort: Local first, then Network, then others
        self.state.volumes.sort_by(|a, b| {
            let type_order = |t: &VolumeType| match t {
                VolumeType::Local => 0,
                VolumeType::Network => 1,
                VolumeType::DiskImage => 2,
                VolumeType::TimeMachine => 3,
                VolumeType::Unknown => 4,
            };
            type_order(&a.volume_type)
                .cmp(&type_order(&b.volume_type))
                .then_with(|| a.name.cmp(&b.name))
        });
    }

    #[cfg(target_os = "macos")]
    fn detect_volume_type(&self, path: &PathBuf) -> VolumeType {
        // Try to detect volume type using diskutil
        if let Ok(output) = Command::new("diskutil")
            .args(["info", &path.to_string_lossy()])
            .output()
        {
            let info = String::from_utf8_lossy(&output.stdout);
            if info.contains("Network: Yes")
                || info.contains("Protocol: AFP")
                || info.contains("Protocol: SMB")
            {
                return VolumeType::Network;
            }
            if info.contains("Time Machine") {
                return VolumeType::TimeMachine;
            }
            if info.contains("Disk Image") {
                return VolumeType::DiskImage;
            }
            if info.contains("Internal: Yes") || info.contains("APFS") {
                return VolumeType::Local;
            }
        }

        // Fallback heuristics
        let path_str = path.to_string_lossy();
        if path_str.contains(".dmg") || path_str.contains("disk image") {
            VolumeType::DiskImage
        } else if path_str.contains("Time Machine") {
            VolumeType::TimeMachine
        } else {
            VolumeType::Local
        }
    }

    #[cfg(target_os = "macos")]
    fn get_fs_info(&self, path: &PathBuf) -> (Option<u64>, Option<u64>, String) {
        // Use df command to get filesystem info
        if let Ok(output) = Command::new("df")
            .args(["-k", &path.to_string_lossy()])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            if lines.len() >= 2 {
                let parts: Vec<&str> = lines[1].split_whitespace().collect();
                if parts.len() >= 4 {
                    let total = parts[1].parse::<u64>().ok().map(|k| k * 1024);
                    let free = parts[3].parse::<u64>().ok().map(|k| k * 1024);

                    // Get filesystem type from mount
                    let fs_type = if let Ok(mount_out) = Command::new("mount").output() {
                        let mount_str = String::from_utf8_lossy(&mount_out.stdout);
                        mount_str
                            .lines()
                            .find(|l| l.contains(&path.to_string_lossy().to_string()))
                            .and_then(|l| {
                                l.split('(')
                                    .nth(1)
                                    .and_then(|s| s.split(',').next())
                                    .map(|s| s.trim().to_string())
                            })
                            .unwrap_or_else(|| "unknown".to_string())
                    } else {
                        "unknown".to_string()
                    };

                    return (total, free, fs_type);
                }
            }
        }
        (None, None, "unknown".to_string())
    }

    #[cfg(not(target_os = "macos"))]
    fn refresh_unix_volumes(&mut self) {
        // On Linux, read /proc/mounts or use findmnt
        // For now, just show root
        self.state.volumes.push(VolumeEntry {
            name: "Root".to_string(),
            path: PathBuf::from("/"),
            volume_type: VolumeType::Local,
            mount_point: "/".to_string(),
            filesystem: "ext4".to_string(),
            total_size: None,
            free_space: None,
            writable: true,
        });
    }

    fn check_writable(&self, path: &PathBuf) -> bool {
        // Try to get metadata and check permissions
        fs::metadata(path)
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
    }

    /// Take the navigate path (consumes it)
    pub fn take_navigate_path(&mut self) -> Option<PathBuf> {
        self.state.navigate_path.take()
    }
}

impl Plugin for DrivesPlugin {
    fn id(&self) -> &str {
        "drives"
    }

    fn name(&self) -> &str {
        "Drives"
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
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.initialized = false;
        Ok(())
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true // Always available
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Drives".to_string(),
            key: '6', // F6 key mapping
            description: "Browse mounted volumes".to_string(),
            priority: 60,
        })
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn handle_global_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            // F3 = Change Drive (classic DOS)
            KeyCode::F(3) => {
                // Refresh volumes and open modal
                self.refresh_volumes();
                self.state.selected_index = 0;
                self.state.navigate_path = None;
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => KeyHandleResult::CloseModal,
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_prev();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Navigate to selected volume
                if let Some(vol) = self.state.selected_volume() {
                    self.state.navigate_path = Some(vol.path.clone());
                    return KeyHandleResult::CloseWithSuccess("drives:navigate".to_string());
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Refresh volume list
                self.refresh_volumes();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_drives_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "F3 - Drives & Volumes".to_string(),
            "".to_string(),
            "Browse mounted volumes and network drives.".to_string(),
            "F3 is the classic DOS 'Change Drive' key.".to_string(),
            "".to_string(),
            "Keys:".to_string(),
            "  F3        Open Drives modal".to_string(),
            "  ↑↓/jk     Navigate list".to_string(),
            "  Enter     Go to selected volume".to_string(),
            "  R         Refresh volume list".to_string(),
            "  Esc       Close".to_string(),
            "".to_string(),
            "Volume Types:".to_string(),
            "  HD        Local disk".to_string(),
            "  NET       Network share (SMB/AFP/NFS)".to_string(),
            "  DMG       Disk image".to_string(),
            "  TM        Time Machine backup".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
