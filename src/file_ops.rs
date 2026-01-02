use crate::app::SortMode;
use anyhow::Result;
use chrono::{DateTime, Local};
use std::fs;
use std::path::PathBuf;
use sysinfo::System;

/// Represents a file or directory entry
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FileEntry {
    /// File name (without path)
    pub name: String,
    /// File extension (if any)
    pub extension: String,
    /// Full path
    pub path: PathBuf,
    /// File size in bytes (0 for directories)
    pub size: u64,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Modification time
    pub modified: DateTime<Local>,
    /// Creation time
    pub created: DateTime<Local>,
}

impl FileEntry {
    /// Format size for display (human-readable)
    #[allow(dead_code)]
    pub fn size_string(&self) -> String {
        if self.is_dir {
            "<DIR>".to_string()
        } else {
            humansize::format_size(self.size, humansize::DECIMAL)
        }
    }

    /// Format date for display (M-DD-YY with space padding, like " 1- 2-26")
    pub fn date_string(&self) -> String {
        let month = self.modified.format("%-m").to_string();
        let day = self.modified.format("%-d").to_string();
        let year = self.modified.format("%y").to_string();
        format!("{:>2}-{:>2}-{}", month, day, year)
    }

    /// Format time for display (HH:MMp) - just "a" or "p", not "am"/"pm"
    pub fn time_string(&self) -> String {
        let time = self.modified.format("%-I:%M").to_string();
        let suffix = if self.modified.format("%P").to_string().starts_with('a') { "a" } else { "p" };
        format!("{}{}", time, suffix)
    }

    /// Get the display name (name + extension)
    #[allow(dead_code)]
    pub fn display_name(&self) -> String {
        if self.extension.is_empty() {
            self.name.clone()
        } else {
            format!("{}.{}", self.name, self.extension)
        }
    }
}

/// Get directory contents with sorting
pub fn get_directory_contents(path: &PathBuf, sort_mode: SortMode) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    // Add parent directory entry if not at root
    if let Some(parent) = path.parent() {
        entries.push(FileEntry {
            name: "..".to_string(),
            extension: String::new(),
            path: parent.to_path_buf(),
            size: 0,
            is_dir: true,
            modified: DateTime::from(std::time::SystemTime::now()),
            created: DateTime::from(std::time::SystemTime::now()),
        });
    }

    // Read directory entries
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files (starting with .)
        if file_name.starts_with('.') {
            continue;
        }

        // Extract name and extension
        let (name, extension) = if metadata.is_dir() {
            (file_name.clone(), String::new())
        } else {
            match file_name.rfind('.') {
                Some(pos) if pos > 0 => {
                    (file_name[..pos].to_string(), file_name[pos + 1..].to_string())
                }
                _ => (file_name.clone(), String::new()),
            }
        };

        let modified = metadata
            .modified()
            .map(DateTime::from)
            .unwrap_or_else(|_| Local::now());

        let created = metadata
            .created()
            .map(DateTime::from)
            .unwrap_or_else(|_| Local::now());

        entries.push(FileEntry {
            name,
            extension: extension.to_uppercase(),
            path: entry.path(),
            size: metadata.len(),
            is_dir: metadata.is_dir(),
            modified,
            created,
        });
    }

    // Sort entries (directories first, then by sort mode)
    sort_entries(&mut entries, sort_mode);

    Ok(entries)
}

/// Sort file entries according to the given mode
fn sort_entries(entries: &mut [FileEntry], sort_mode: SortMode) {
    // Keep ".." at the top
    let has_parent = entries.first().map(|e| e.name == "..").unwrap_or(false);
    let start_idx = if has_parent { 1 } else { 0 };

    let slice = &mut entries[start_idx..];

    // Sort directories first, then files
    slice.sort_by(|a, b| {
        // Directories always come first
        match (a.is_dir, b.is_dir) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }

        // Then sort by the selected mode
        match sort_mode {
            SortMode::NameAsc => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortMode::NameDesc => b.name.to_lowercase().cmp(&a.name.to_lowercase()),
            SortMode::ExtAsc => a.extension.cmp(&b.extension),
            SortMode::ExtDesc => b.extension.cmp(&a.extension),
            SortMode::SizeAsc => a.size.cmp(&b.size),
            SortMode::SizeDesc => b.size.cmp(&a.size),
            SortMode::DateAsc => a.modified.cmp(&b.modified),
            SortMode::DateDesc => b.modified.cmp(&a.modified),
            SortMode::None => std::cmp::Ordering::Equal,
        }
    });
}

/// System information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemInfo {
    pub total_memory: u64,
    pub used_memory: u64,
    pub total_swap: u64,
    pub used_swap: u64,
    pub cpu_count: usize,
    pub os_name: String,
    pub os_version: String,
    pub hostname: String,
}

/// Get system information
pub fn get_system_info() -> Result<SystemInfo> {
    let mut sys = System::new_all();
    sys.refresh_all();

    Ok(SystemInfo {
        total_memory: sys.total_memory(),
        used_memory: sys.used_memory(),
        total_swap: sys.total_swap(),
        used_swap: sys.used_swap(),
        cpu_count: sys.cpus().len(),
        os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
        os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
        hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
    })
}

/// Get disk space information for a path
pub fn get_disk_space(path: &PathBuf) -> Result<(u64, u64)> {
    // This is a simplified version - in production you'd use statvfs or similar
    // For now, we'll use sysinfo's disk info
    let mut sys = System::new_all();
    sys.refresh_all();

    // Try to find the disk that contains this path
    for disk in sysinfo::Disks::new_with_refreshed_list().iter() {
        let mount_point = disk.mount_point();
        if path.starts_with(mount_point) {
            return Ok((disk.available_space(), disk.total_space()));
        }
    }

    // Fallback to first disk
    if let Some(disk) = sysinfo::Disks::new_with_refreshed_list().iter().next() {
        return Ok((disk.available_space(), disk.total_space()));
    }

    Ok((0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_directory_contents() {
        let current_dir = env::current_dir().unwrap();
        let result = get_directory_contents(&current_dir, SortMode::NameAsc);
        assert!(result.is_ok());
        let entries = result.unwrap();
        // Should have at least the parent directory entry
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_get_system_info() {
        let result = get_system_info();
        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(info.cpu_count > 0);
        assert!(info.total_memory > 0);
    }

    #[test]
    fn test_get_disk_space() {
        let current_dir = env::current_dir().unwrap();
        let result = get_disk_space(&current_dir);
        assert!(result.is_ok());
        let (available, total) = result.unwrap();
        assert!(total >= available);
    }

    #[test]
    fn test_sort_modes() {
        assert_eq!(SortMode::NameAsc.next(), SortMode::NameDesc);
        assert_eq!(SortMode::None.next(), SortMode::NameAsc);
    }
}
