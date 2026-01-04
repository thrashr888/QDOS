use crate::app::SortMode;
use anyhow::Result;
use chrono::{DateTime, Local};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use sysinfo::System;

/// File type/kind classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileKind {
    Directory,
    Text,
    Code,
    Image,
    Audio,
    Video,
    Archive,
    Document,
    Executable,
    #[default]
    Binary,
}

impl FileKind {
    /// Determine file kind from extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            // Text files
            "txt" | "md" | "markdown" | "rst" | "log" | "csv" | "json" | "yaml" | "yml"
            | "toml" | "xml" | "html" | "htm" | "css" => FileKind::Text,
            // Code files
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "c" | "cpp" | "h" | "hpp" | "java"
            | "go" | "rb" | "php" | "swift" | "kt" | "scala" | "sh" | "bash" | "zsh" | "fish"
            | "ps1" | "sql" | "lua" | "vim" | "el" => FileKind::Code,
            // Image files
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" | "tiff" | "tif"
            | "psd" | "raw" | "heic" | "heif" => FileKind::Image,
            // Audio files
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" | "aiff" | "opus" => {
                FileKind::Audio
            }
            // Video files
            "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpeg" | "mpg" => {
                FileKind::Video
            }
            // Archive files
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "tgz" | "tbz2" | "txz" | "zst" => {
                FileKind::Archive
            }
            // Document files
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp"
            | "rtf" | "epub" => FileKind::Document,
            // Executable files
            "exe" | "msi" | "app" | "dmg" | "deb" | "rpm" | "apk" | "jar" | "out" => {
                FileKind::Executable
            }
            // Default to binary
            _ => FileKind::Binary,
        }
    }

    /// Get short display string for kind
    pub fn as_str(&self) -> &'static str {
        match self {
            FileKind::Directory => "DIR",
            FileKind::Text => "TEXT",
            FileKind::Code => "CODE",
            FileKind::Image => "IMG",
            FileKind::Audio => "AUDIO",
            FileKind::Video => "VIDEO",
            FileKind::Archive => "ARCH",
            FileKind::Document => "DOC",
            FileKind::Executable => "EXEC",
            FileKind::Binary => "BIN",
        }
    }
}

/// Git status for a file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitStatus {
    #[default]
    None,
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Ignored,
    Conflict,
}

impl GitStatus {
    /// Get single character indicator for git status
    pub fn indicator(&self) -> &'static str {
        match self {
            GitStatus::None => " ",
            GitStatus::Modified => "M",
            GitStatus::Added => "A",
            GitStatus::Deleted => "D",
            GitStatus::Renamed => "R",
            GitStatus::Untracked => "?",
            GitStatus::Ignored => "!",
            GitStatus::Conflict => "C",
        }
    }
}

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
    /// File type/kind
    pub kind: FileKind,
    /// Whether file is hidden (starts with .)
    pub is_hidden: bool,
    /// Git status
    pub git_status: GitStatus,
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
        let suffix = if self.modified.format("%P").to_string().starts_with('a') {
            "a"
        } else {
            "p"
        };
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

/// Get the git repository root directory
fn get_git_root(path: &PathBuf) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .ok()?;

    if output.status.success() {
        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(PathBuf::from(root))
    } else {
        None
    }
}

/// Get git status for all files in a directory
fn get_git_status_map(path: &PathBuf) -> HashMap<PathBuf, GitStatus> {
    let mut status_map = HashMap::new();

    // Get git root directory
    let git_root = match get_git_root(path) {
        Some(root) => root,
        None => return status_map, // Not a git repo
    };

    // Run git status --porcelain from the git root to get file statuses
    let output = Command::new("git")
        .args(["status", "--porcelain", "-uall"])
        .current_dir(&git_root)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.len() < 4 {
                    continue;
                }

                let status_code = &line[0..2];
                let file_path = line[3..].trim();

                // Handle renamed files (format: "R  old -> new")
                let file_path = if file_path.contains(" -> ") {
                    file_path.split(" -> ").last().unwrap_or(file_path)
                } else {
                    file_path
                };

                let status = match status_code {
                    "M " | " M" | "MM" => GitStatus::Modified,
                    "A " | "AM" => GitStatus::Added,
                    "D " | " D" => GitStatus::Deleted,
                    "R " | "RM" => GitStatus::Renamed,
                    "??" => GitStatus::Untracked,
                    "!!" => GitStatus::Ignored,
                    "UU" | "AA" | "DD" => GitStatus::Conflict,
                    _ => GitStatus::None,
                };

                if status != GitStatus::None {
                    // Store absolute path (git root + relative path)
                    let full_path = git_root.join(file_path);
                    status_map.insert(full_path, status);

                    // Also mark parent directories as modified if a file inside is modified
                    let mut parent = PathBuf::from(file_path);
                    while let Some(p) = parent.parent() {
                        if p.as_os_str().is_empty() {
                            break;
                        }
                        let parent_full = git_root.join(p);
                        status_map.entry(parent_full).or_insert(GitStatus::Modified);
                        parent = p.to_path_buf();
                    }
                }
            }
        }
    }

    status_map
}

/// Get directory contents with sorting
pub fn get_directory_contents(path: &PathBuf, sort_mode: SortMode) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    // Get git status for the directory
    let git_status_map = get_git_status_map(path);

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
            kind: FileKind::Directory,
            is_hidden: false,
            git_status: GitStatus::None,
        });
    }

    // Read directory entries
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Check if file is hidden (starting with .)
        let is_hidden = file_name.starts_with('.');

        // Extract name and extension
        let (name, extension) = if metadata.is_dir() {
            (file_name.clone(), String::new())
        } else {
            match file_name.rfind('.') {
                Some(pos) if pos > 0 => (
                    file_name[..pos].to_string(),
                    file_name[pos + 1..].to_string(),
                ),
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

        // Determine file kind
        let kind = if metadata.is_dir() {
            FileKind::Directory
        } else {
            FileKind::from_extension(&extension)
        };

        // Get git status for this file
        let file_path = entry.path();
        let git_status = git_status_map
            .get(&file_path)
            .copied()
            .unwrap_or(GitStatus::None);

        entries.push(FileEntry {
            name,
            extension,
            path: file_path,
            size: metadata.len(),
            is_dir: metadata.is_dir(),
            modified,
            created,
            kind,
            is_hidden,
            git_status,
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

    // Sort: directories first, then non-hidden before hidden, then by sort mode
    slice.sort_by(|a, b| {
        // Directories always come first
        match (a.is_dir, b.is_dir) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }

        // Hidden files come after non-hidden (within dirs and files)
        match (a.is_hidden, b.is_hidden) {
            (false, true) => return std::cmp::Ordering::Less,
            (true, false) => return std::cmp::Ordering::Greater,
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

/// Match a filename against a DOS-style wildcard pattern
/// Supports * (any characters) and ? (single character)
pub fn match_pattern(name: &str, pattern: &str) -> bool {
    let name = name.to_lowercase();
    let pattern = pattern.to_lowercase();

    // Split name and pattern into base and extension
    let (name_base, name_ext) = if let Some(pos) = name.rfind('.') {
        (&name[..pos], &name[pos + 1..])
    } else {
        (name.as_str(), "")
    };

    let (pat_base, pat_ext) = if let Some(pos) = pattern.rfind('.') {
        (&pattern[..pos], &pattern[pos + 1..])
    } else {
        // If no extension in pattern, match any extension
        (pattern.as_str(), "*")
    };

    match_wildcard(name_base, pat_base) && match_wildcard(name_ext, pat_ext)
}

/// Match a string against a wildcard pattern
fn match_wildcard(s: &str, pattern: &str) -> bool {
    let s_chars: Vec<char> = s.chars().collect();
    let p_chars: Vec<char> = pattern.chars().collect();

    fn helper(s: &[char], p: &[char]) -> bool {
        match (s.is_empty(), p.is_empty()) {
            (true, true) => true,
            (_, true) => false,
            (true, false) => p.iter().all(|&c| c == '*'),
            (false, false) => {
                match p[0] {
                    '*' => {
                        // * matches zero or more characters
                        helper(s, &p[1..]) || helper(&s[1..], p)
                    }
                    '?' => {
                        // ? matches exactly one character
                        helper(&s[1..], &p[1..])
                    }
                    c => {
                        // Exact character match
                        s[0].eq_ignore_ascii_case(&c) && helper(&s[1..], &p[1..])
                    }
                }
            }
        }
    }

    helper(&s_chars, &p_chars)
}

/// Recursively find files matching a pattern
/// Returns a list of (full_path, display_string) tuples
pub fn find_files_recursive(root: &PathBuf, pattern: &str) -> Vec<(PathBuf, String)> {
    let mut results = Vec::new();
    find_files_recursive_impl(root, pattern, &mut results);
    results
}

fn find_files_recursive_impl(dir: &PathBuf, pattern: &str, results: &mut Vec<(PathBuf, String)>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files/dirs (starting with .)
            if name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                // Recurse into subdirectories
                find_files_recursive_impl(&path, pattern, results);
            } else {
                // Check if file matches pattern
                if match_pattern(&name, pattern) {
                    let display =
                        format!("{} - {}", name, path.parent().unwrap_or(&path).display());
                    results.push((path, display));
                }
            }
        }
    }
}

/// Apply attribute changes to a file
/// Only R/O (read-only) is actually modifiable on Unix
#[cfg(unix)]
pub fn apply_attributes(path: &PathBuf, attrs: &[crate::app::AttrValue; 4]) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::metadata(path)?;
    let mut perms = metadata.permissions();
    let current_mode = perms.mode();

    // Only R/O (index 2) is modifiable
    let readonly_attr = attrs[2];

    match readonly_attr {
        crate::app::AttrValue::On => {
            // Remove all write permissions
            perms.set_mode(current_mode & !0o222);
            fs::set_permissions(path, perms)?;
        }
        crate::app::AttrValue::Off => {
            // Add user write permission
            perms.set_mode(current_mode | 0o200);
            fs::set_permissions(path, perms)?;
        }
        crate::app::AttrValue::NoChange => {
            // Do nothing
        }
    }

    Ok(())
}

/// Apply attribute changes to a file (Windows version)
/// On Windows, we use the readonly property directly
#[cfg(not(unix))]
pub fn apply_attributes(path: &PathBuf, attrs: &[crate::app::AttrValue; 4]) -> Result<()> {
    let metadata = fs::metadata(path)?;
    let mut perms = metadata.permissions();

    // Only R/O (index 2) is modifiable
    let readonly_attr = attrs[2];

    match readonly_attr {
        crate::app::AttrValue::On => {
            perms.set_readonly(true);
            fs::set_permissions(path, perms)?;
        }
        crate::app::AttrValue::Off => {
            perms.set_readonly(false);
            fs::set_permissions(path, perms)?;
        }
        crate::app::AttrValue::NoChange => {
            // Do nothing
        }
    }

    Ok(())
}

/// Open a file in its default application using the system's open command
/// Uses 'open' on macOS and 'xdg-open' on Linux
pub fn open_in_default_app(path: &PathBuf) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to open file: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to open file: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to open file: {}", e))?;
    }

    Ok(())
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
