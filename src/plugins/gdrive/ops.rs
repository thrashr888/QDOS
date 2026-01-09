//! Google Drive Operations

use super::state::{GDriveFileEntry, GDriveState, GDriveSyncState, GDriveVariant};
use crate::plugins::cloud::StorageInfo;
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "macos")]
use std::process::Command;

/// Check if Google Drive is installed and find which variant
pub fn detect_gdrive() -> (bool, GDriveVariant, Option<PathBuf>) {
    // Check for home folder variants first (most common now)
    if let Some(home) = dirs::home_dir() {
        // Check for macOS CloudStorage location (Google Drive for Desktop)
        // Format: ~/Library/CloudStorage/GoogleDrive-email@domain.com
        let cloud_storage = home.join("Library/CloudStorage");
        if cloud_storage.exists() {
            if let Ok(entries) = fs::read_dir(&cloud_storage) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("GoogleDrive") {
                        let path = entry.path();
                        // Check for "My Drive" subfolder
                        let my_drive = path.join("My Drive");
                        if my_drive.exists() {
                            return (true, GDriveVariant::HomeFolder, Some(my_drive));
                        }
                        // Otherwise return the root Google Drive folder
                        if path.is_dir() {
                            return (true, GDriveVariant::HomeFolder, Some(path));
                        }
                    }
                }
            }
        }

        // Check for traditional home folder paths
        let home_path = home.join("Google Drive");
        if home_path.exists() {
            return (true, GDriveVariant::HomeFolder, Some(home_path));
        }

        // Check for "My Drive" folder (Google Drive for Desktop default)
        let my_drive = home.join("My Drive");
        if my_drive.exists() {
            return (true, GDriveVariant::HomeFolder, Some(my_drive));
        }
    }

    // Check for Google Drive for Desktop mounted as volume
    let volumes_path = PathBuf::from("/Volumes/GoogleDrive");
    if volumes_path.exists() {
        return (true, GDriveVariant::VolumesMount, Some(volumes_path));
    }

    // Check for Google Drive Stream (older)
    let stream_path = PathBuf::from("/Volumes/GoogleDrive/My Drive");
    if stream_path.exists() {
        return (true, GDriveVariant::Stream, Some(stream_path));
    }

    (false, GDriveVariant::None, None)
}

/// Get Google Drive root path
pub fn get_gdrive_path() -> Option<PathBuf> {
    let (installed, _, path) = detect_gdrive();
    if installed {
        path
    } else {
        None
    }
}

/// Check if Google Drive is installed
pub fn is_gdrive_installed() -> bool {
    let (installed, _, _) = detect_gdrive();
    installed
}

/// Check if Google Drive is running
#[cfg(target_os = "macos")]
pub fn is_gdrive_running() -> bool {
    Command::new("pgrep")
        .args(["-x", "Google Drive"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
pub fn is_gdrive_running() -> bool {
    is_gdrive_installed()
}

/// Get sync status for a file
pub fn get_file_sync_status(path: &PathBuf) -> GDriveSyncState {
    // Google Drive for Desktop uses virtual files
    // Check file type and existence

    let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

    // Check for Google Docs formats (these are always "streaming")
    if filename.ends_with(".gdoc")
        || filename.ends_with(".gsheet")
        || filename.ends_with(".gslides")
    {
        return GDriveSyncState::Streaming;
    }

    // Check extended attributes on macOS
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("xattr")
            .args(["-l", &path.to_string_lossy()])
            .output()
        {
            let attrs = String::from_utf8_lossy(&output.stdout);
            if attrs.contains("com.google") {
                // Has Google Drive attributes
                if attrs.contains("sync") {
                    return GDriveSyncState::Syncing;
                }
            }
        }
    }

    if path.exists() {
        GDriveSyncState::Available
    } else {
        GDriveSyncState::Unknown
    }
}

/// Check if file is a Google Docs file type
fn is_google_doc(filename: &str) -> bool {
    filename.ends_with(".gdoc")
        || filename.ends_with(".gsheet")
        || filename.ends_with(".gslides")
        || filename.ends_with(".gform")
        || filename.ends_with(".gdraw")
}

/// Load files in directory
pub fn load_directory(state: &mut GDriveState, dir: &PathBuf) {
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

    let mut files: Vec<GDriveFileEntry> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let filename = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files
        if filename.starts_with('.') {
            continue;
        }

        let metadata = entry.metadata().ok();
        let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = metadata
            .as_ref()
            .and_then(|m| if m.is_file() { Some(m.len()) } else { None });

        let sync_state = get_file_sync_status(&path);
        let is_gdoc = is_google_doc(&filename);

        files.push(GDriveFileEntry {
            name: filename,
            path,
            sync_state,
            size,
            is_dir,
            is_google_doc: is_gdoc,
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

/// Get storage info
pub fn get_storage_info() -> StorageInfo {
    #[cfg(target_os = "macos")]
    {
        if let Some(gdrive_path) = get_gdrive_path() {
            if let Ok(output) = Command::new("df")
                .args(["-k", &gdrive_path.to_string_lossy()])
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
                            connected: is_gdrive_running(),
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
        connected: is_gdrive_running(),
    }
}

/// Open file/folder in browser (Google Drive web)
pub fn open_in_browser(path: &PathBuf) -> Result<(), String> {
    // For Google Docs files, we can open them directly
    let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

    if is_google_doc(filename) {
        // Open the file directly - macOS will handle it
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg(path)
                .spawn()
                .map_err(|e| format!("Failed to open: {}", e))?;
            return Ok(());
        }
    }

    // For regular files, open Google Drive web
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("https://drive.google.com")
            .spawn()
            .map_err(|e| format!("Failed to open browser: {}", e))?;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("Opening browser not supported on this platform".to_string())
    }
}

/// Open Google Drive preferences
#[cfg(target_os = "macos")]
pub fn open_preferences() -> Result<(), String> {
    Command::new("open")
        .arg("-a")
        .arg("Google Drive")
        .spawn()
        .map_err(|e| format!("Failed to open Google Drive: {}", e))?;

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn open_preferences() -> Result<(), String> {
    Err("Not supported on this platform".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdrive_detection() {
        let (_, variant, _) = detect_gdrive();
        // Just verify it doesn't panic
        let _ = variant;
    }

    #[test]
    fn test_is_google_doc() {
        assert!(is_google_doc("document.gdoc"));
        assert!(is_google_doc("spreadsheet.gsheet"));
        assert!(is_google_doc("presentation.gslides"));
        assert!(!is_google_doc("regular.txt"));
        assert!(!is_google_doc("image.png"));
    }
}
