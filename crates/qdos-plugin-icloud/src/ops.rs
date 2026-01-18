//! iCloud Drive Operations
//!
//! Functions for interacting with iCloud Drive on macOS.

use super::state::{ICloudFileEntry, ICloudState, ICloudSyncState};
use qdos_plugin_cloud::StorageInfo;
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "macos")]
use std::process::Command;

/// Get iCloud Drive root path
pub fn get_icloud_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir()?;
        let icloud_path = home.join("Library/Mobile Documents/com~apple~CloudDocs");
        if icloud_path.exists() {
            Some(icloud_path)
        } else {
            None
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        None // iCloud Drive not available on non-macOS
    }
}

/// Check if iCloud Drive is available
pub fn is_icloud_available() -> bool {
    get_icloud_path().is_some()
}

/// Get sync status for a file
pub fn get_file_sync_status(path: &PathBuf) -> ICloudSyncState {
    let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

    // Check if this is a .icloud placeholder file
    if filename.starts_with('.') && filename.ends_with(".icloud") {
        return ICloudSyncState::CloudOnly;
    }

    // Check extended attributes for sync status
    #[cfg(target_os = "macos")]
    {
        // Check for com.apple.icloud.itemDownloadRequested attribute
        if let Ok(output) = Command::new("xattr")
            .args(["-l", &path.to_string_lossy()])
            .output()
        {
            let attrs = String::from_utf8_lossy(&output.stdout);
            if attrs.contains("com.apple.icloud.downloadRequested") {
                return ICloudSyncState::Downloading;
            }
            if attrs.contains("com.apple.icloud.uploading") {
                return ICloudSyncState::Uploading;
            }
        }
    }

    if path.exists() {
        ICloudSyncState::Downloaded
    } else {
        ICloudSyncState::Unknown
    }
}

/// Parse .icloud filename to get original filename
fn parse_icloud_filename(filename: &str) -> Option<String> {
    // .icloud files are named like: .originalname.ext.icloud
    if filename.starts_with('.') && filename.ends_with(".icloud") {
        let without_prefix = &filename[1..];
        let without_suffix = &without_prefix[..without_prefix.len() - 7]; // Remove ".icloud"
        Some(without_suffix.to_string())
    } else {
        None
    }
}

/// Load files in a directory with their iCloud status
pub fn load_directory(state: &mut ICloudState, dir: &PathBuf) {
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

    let mut files: Vec<ICloudFileEntry> = Vec::new();
    let mut icloud_placeholders: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    // First pass: collect .icloud placeholder names
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let filename = entry.file_name().to_string_lossy().to_string();
        if let Some(original) = parse_icloud_filename(&filename) {
            icloud_placeholders.insert(original);
        }
    }

    for entry in entries.flatten() {
        let path = entry.path();
        let filename = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files unless they're .icloud placeholders
        if filename.starts_with('.') && !filename.ends_with(".icloud") {
            continue;
        }

        // Skip if this file has a corresponding .icloud placeholder
        // (meaning the real file exists but there's also a placeholder)
        if icloud_placeholders.contains(&filename) {
            continue;
        }

        let metadata = entry.metadata().ok();
        let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = metadata
            .as_ref()
            .and_then(|m| if m.is_file() { Some(m.len()) } else { None });

        let (display_name, original_name, sync_state) =
            if let Some(original) = parse_icloud_filename(&filename) {
                (original.clone(), Some(original), ICloudSyncState::CloudOnly)
            } else {
                (filename.clone(), None, get_file_sync_status(&path))
            };

        files.push(ICloudFileEntry {
            name: display_name,
            path,
            sync_state,
            size,
            is_dir,
            original_name,
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

/// Trigger download of a cloud-only file
#[cfg(target_os = "macos")]
pub fn download_file(path: &PathBuf) -> Result<(), String> {
    // Use brctl to download the file
    Command::new("brctl")
        .args(["download", &path.to_string_lossy()])
        .output()
        .map_err(|e| format!("Failed to download: {}", e))?;

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn download_file(_path: &PathBuf) -> Result<(), String> {
    Err("iCloud download not supported on this platform".to_string())
}

/// Evict a downloaded file (remove local copy, keep in cloud)
#[cfg(target_os = "macos")]
pub fn evict_file(path: &PathBuf) -> Result<(), String> {
    Command::new("brctl")
        .args(["evict", &path.to_string_lossy()])
        .output()
        .map_err(|e| format!("Failed to evict: {}", e))?;

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn evict_file(_path: &PathBuf) -> Result<(), String> {
    Err("iCloud evict not supported on this platform".to_string())
}

/// Get iCloud storage info
pub fn get_storage_info() -> StorageInfo {
    #[cfg(target_os = "macos")]
    {
        if let Some(icloud_path) = get_icloud_path() {
            if let Ok(output) = Command::new("df")
                .args(["-k", &icloud_path.to_string_lossy()])
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
                            account: None,
                            connected: true,
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
        connected: is_icloud_available(),
    }
}

/// Open iCloud Drive preferences
#[cfg(target_os = "macos")]
pub fn open_preferences() -> Result<(), String> {
    Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preferences.AppleIDPrefPane?iCloud")
        .spawn()
        .map_err(|e| format!("Failed to open preferences: {}", e))?;

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn open_preferences() -> Result<(), String> {
    Err("iCloud preferences not available on this platform".to_string())
}

/// Open file in Finder
#[cfg(target_os = "macos")]
pub fn reveal_in_finder(path: &PathBuf) -> Result<(), String> {
    Command::new("open")
        .args(["-R", &path.to_string_lossy()])
        .spawn()
        .map_err(|e| format!("Failed to reveal in Finder: {}", e))?;

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn reveal_in_finder(_path: &PathBuf) -> Result<(), String> {
    Err("Reveal in Finder not available on this platform".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_icloud_filename() {
        assert_eq!(
            parse_icloud_filename(".document.pdf.icloud"),
            Some("document.pdf".to_string())
        );
        assert_eq!(
            parse_icloud_filename(".photo.jpg.icloud"),
            Some("photo.jpg".to_string())
        );
        assert_eq!(parse_icloud_filename("normalfile.txt"), None);
        assert_eq!(parse_icloud_filename(".hiddenfile"), None);
    }

    #[test]
    fn test_icloud_path_detection() {
        // Just verify it doesn't panic
        let _ = get_icloud_path();
    }
}
