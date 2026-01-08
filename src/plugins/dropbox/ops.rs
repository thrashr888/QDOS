//! Dropbox Operations
//!
//! Functions for interacting with Dropbox and reading sync status.

use super::state::{DropboxFileEntry, DropboxState, DropboxSyncState};
use crate::plugins::cloud::StorageInfo;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Check if Dropbox is installed
pub fn is_dropbox_installed() -> bool {
    #[cfg(target_os = "macos")]
    {
        // Check for Dropbox.app
        PathBuf::from("/Applications/Dropbox.app").exists()
            || dirs::home_dir()
                .map(|h| h.join("Applications/Dropbox.app").exists())
                .unwrap_or(false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Check for dropbox command or folder
        dirs::home_dir()
            .map(|h| h.join("Dropbox").exists())
            .unwrap_or(false)
    }
}

/// Check if Dropbox is running
pub fn is_dropbox_running() -> bool {
    #[cfg(target_os = "macos")]
    {
        Command::new("pgrep")
            .args(["-x", "Dropbox"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Command::new("pgrep")
            .args(["-x", "dropbox"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Get Dropbox root path
pub fn get_dropbox_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let dropbox_path = home.join("Dropbox");
    if dropbox_path.exists() {
        Some(dropbox_path)
    } else {
        None
    }
}

/// Read sync status for a file using extended attributes (macOS)
#[cfg(target_os = "macos")]
pub fn get_file_sync_status(path: &PathBuf) -> DropboxSyncState {
    use std::process::Command;

    // Try to read Dropbox extended attribute
    let output = Command::new("xattr")
        .args(["-p", "com.dropbox.attrs", &path.to_string_lossy()])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let attr_str = String::from_utf8_lossy(&result.stdout);
            parse_dropbox_attr(&attr_str)
        }
        _ => {
            // No Dropbox attribute - check if file exists in Dropbox folder
            if path.exists() {
                DropboxSyncState::UpToDate // Assume synced if no special status
            } else {
                DropboxSyncState::Unknown
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_file_sync_status(path: &PathBuf) -> DropboxSyncState {
    // On other platforms, we can't easily read sync status
    if path.exists() {
        DropboxSyncState::UpToDate
    } else {
        DropboxSyncState::Unknown
    }
}

/// Parse Dropbox extended attribute value
#[cfg(target_os = "macos")]
fn parse_dropbox_attr(attr: &str) -> DropboxSyncState {
    // Dropbox uses a binary/hex format for attributes
    // Common patterns:
    // - Files with no special status are synced
    // - Syncing files have specific byte patterns
    // For simplicity, we'll use a heuristic approach

    let lower = attr.to_lowercase();
    if lower.contains("sync") || lower.contains("upload") || lower.contains("download") {
        DropboxSyncState::Syncing
    } else if lower.contains("error") || lower.contains("unsync") {
        DropboxSyncState::Unsyncable
    } else if lower.contains("selective") || lower.contains("ignore") {
        DropboxSyncState::SelectiveSync
    } else {
        DropboxSyncState::UpToDate
    }
}

/// Load files in a directory with their sync status
pub fn load_directory(state: &mut DropboxState, dir: &PathBuf) {
    state.files.clear();
    state.selected = 0;
    state.scroll_offset = 0;
    state.error = None;

    if !dir.exists() {
        state.error = Some("Directory not found".to_string());
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            state.error = Some(format!("Failed to read directory: {}", e));
            return;
        }
    };

    let mut files: Vec<DropboxFileEntry> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files
        if name.starts_with('.') {
            continue;
        }

        let metadata = entry.metadata().ok();
        let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = metadata
            .as_ref()
            .and_then(|m| if m.is_file() { Some(m.len()) } else { None });

        let sync_state = get_file_sync_status(&path);

        // Check if shared (Dropbox creates .shared files or folders)
        let is_shared = name.contains("(") && name.contains(")"); // Shared folders often have names like "Folder (1)"

        files.push(DropboxFileEntry {
            name,
            path,
            sync_state,
            size,
            is_dir,
            is_shared,
        });
    }

    // Sort: directories first, then by name
    files.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    state.files = files;
    state.current_dir = dir.clone();
}

/// Get Dropbox storage info
pub fn get_storage_info() -> StorageInfo {
    // Dropbox doesn't expose storage info via local files easily
    // We'd need to use the Dropbox API for accurate info
    // For now, return the filesystem info for the Dropbox folder

    if let Some(dropbox_path) = get_dropbox_path() {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("df")
                .args(["-k", &dropbox_path.to_string_lossy()])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines().collect();
                if lines.len() >= 2 {
                    let parts: Vec<&str> = lines[1].split_whitespace().collect();
                    if parts.len() >= 4 {
                        let total = parts[1].parse::<u64>().ok().map(|k| k * 1024);
                        let free = parts[3].parse::<u64>().ok().map(|k| k * 1024);
                        let used = total.and_then(|t| free.map(|f| t.saturating_sub(f)));

                        return StorageInfo {
                            total_bytes: total,
                            used_bytes: used,
                            account: None, // Would need API access
                            connected: is_dropbox_running(),
                        };
                    }
                }
            }
        }
    }

    StorageInfo {
        total_bytes: None,
        used_bytes: None,
        account: None,
        connected: is_dropbox_running(),
    }
}

/// Open file in Dropbox web interface
pub fn open_in_browser(path: &PathBuf) -> Result<(), String> {
    // Dropbox web URLs follow a pattern, but we need the relative path
    if let Some(dropbox_root) = get_dropbox_path() {
        if let Ok(relative) = path.strip_prefix(&dropbox_root) {
            let url = format!(
                "https://www.dropbox.com/home/{}",
                relative.to_string_lossy()
            );

            #[cfg(target_os = "macos")]
            {
                Command::new("open")
                    .arg(&url)
                    .spawn()
                    .map_err(|e| format!("Failed to open browser: {}", e))?;
            }

            #[cfg(not(target_os = "macos"))]
            {
                Command::new("xdg-open")
                    .arg(&url)
                    .spawn()
                    .map_err(|e| format!("Failed to open browser: {}", e))?;
            }

            return Ok(());
        }
    }

    Err("File not in Dropbox folder".to_string())
}

/// Get shareable link for a file (requires Dropbox CLI or API)
pub fn get_share_link(path: &PathBuf) -> Result<String, String> {
    // This would ideally use the Dropbox API
    // For now, construct the web URL
    if let Some(dropbox_root) = get_dropbox_path() {
        if let Ok(relative) = path.strip_prefix(&dropbox_root) {
            // Note: This URL won't work for sharing, it's just for viewing
            // Real sharing requires Dropbox API
            let url = format!(
                "https://www.dropbox.com/home/{}",
                relative.to_string_lossy()
            );
            return Ok(url);
        }
    }

    Err("File not in Dropbox folder".to_string())
}

/// Open Dropbox preferences/settings
pub fn open_dropbox_preferences() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // Open Dropbox preferences via AppleScript
        Command::new("osascript")
            .args(["-e", "tell application \"Dropbox\" to activate"])
            .spawn()
            .map_err(|e| format!("Failed to open Dropbox: {}", e))?;

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("Opening Dropbox preferences not supported on this platform".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dropbox_path_detection() {
        // Just verify the function doesn't panic
        let _ = get_dropbox_path();
    }

    #[test]
    fn test_dropbox_running_check() {
        // Just verify the function doesn't panic
        let _ = is_dropbox_running();
    }
}
