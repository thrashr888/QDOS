//! Command executor for Q-MIND
//!
//! Takes parsed commands and executes them, using Dry Run UI for destructive operations.

use super::parser::{CommandAction, ParsedCommand};
use crate::state::{DryRunOpType, DryRunOperation};
use glob::glob;
use std::fs;
use std::path::PathBuf;

/// Result of command execution
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    /// Command executed successfully
    Success(String),
    /// Command requires confirmation via Dry Run UI
    NeedsDryRun(Vec<DryRunOperation>),
    /// Files found (for Find action)
    Found(Vec<PathBuf>),
    /// Error during execution
    Error(String),
    /// Action not supported or unknown
    Unsupported(String),
}

/// Command executor
pub struct CommandExecutor {
    /// Current working directory
    cwd: PathBuf,
}

impl CommandExecutor {
    /// Create a new executor with the given working directory
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }

    /// Execute a parsed command
    pub fn execute(&self, cmd: &ParsedCommand) -> ExecutionResult {
        match cmd.action {
            CommandAction::Find => self.execute_find(cmd),
            CommandAction::List => self.execute_list(cmd),
            CommandAction::Copy => self.prepare_copy(cmd),
            CommandAction::Move => self.prepare_move(cmd),
            CommandAction::Delete => self.prepare_delete(cmd),
            CommandAction::Rename => self.prepare_rename(cmd),
            CommandAction::CreateFile => self.execute_create_file(cmd),
            CommandAction::CreateDir => self.execute_create_dir(cmd),
            CommandAction::Info => self.execute_info(cmd),
            CommandAction::View | CommandAction::Edit => {
                ExecutionResult::Unsupported("Use file browser to view/edit files".to_string())
            }
            CommandAction::ChangeDir => {
                ExecutionResult::Unsupported("Use file browser to navigate".to_string())
            }
            CommandAction::Sort => {
                ExecutionResult::Unsupported("Use file browser sort options".to_string())
            }
            CommandAction::Search => self.execute_search(cmd),
            CommandAction::Unknown => {
                ExecutionResult::Error("Could not understand command".to_string())
            }
        }
    }

    /// Find files matching pattern
    fn execute_find(&self, cmd: &ParsedCommand) -> ExecutionResult {
        let pattern = cmd.pattern.as_deref().unwrap_or("*");
        let search_path = if cmd.targets.is_empty() {
            self.cwd.clone()
        } else {
            self.cwd.join(&cmd.targets[0])
        };

        let glob_pattern = search_path.join(pattern).to_string_lossy().to_string();

        match glob(&glob_pattern) {
            Ok(paths) => {
                let mut found: Vec<PathBuf> = paths.filter_map(|p| p.ok()).collect();

                // Sort by size if looking for "largest"
                if cmd.original.to_lowercase().contains("large") {
                    found.sort_by(|a: &PathBuf, b: &PathBuf| {
                        let size_a = fs::metadata(a).map(|m| m.len()).unwrap_or(0);
                        let size_b = fs::metadata(b).map(|m| m.len()).unwrap_or(0);
                        size_b.cmp(&size_a) // Descending
                    });
                }

                // Sort by date if looking for "recent" or "newest"
                if cmd.original.to_lowercase().contains("recent")
                    || cmd.original.to_lowercase().contains("newest")
                    || cmd.original.to_lowercase().contains("latest")
                {
                    found.sort_by(|a: &PathBuf, b: &PathBuf| {
                        let time_a = fs::metadata(a)
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        let time_b = fs::metadata(b)
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        time_b.cmp(&time_a) // Descending (newest first)
                    });
                }

                // Sort by date ascending if looking for "oldest"
                if cmd.original.to_lowercase().contains("oldest") {
                    found.sort_by(|a: &PathBuf, b: &PathBuf| {
                        let time_a = fs::metadata(a)
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        let time_b = fs::metadata(b)
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        time_a.cmp(&time_b) // Ascending (oldest first)
                    });
                }

                ExecutionResult::Found(found)
            }
            Err(e) => ExecutionResult::Error(format!("Invalid pattern: {}", e)),
        }
    }

    /// List directory contents
    fn execute_list(&self, cmd: &ParsedCommand) -> ExecutionResult {
        let dir = if cmd.targets.is_empty() {
            self.cwd.clone()
        } else {
            self.cwd.join(&cmd.targets[0])
        };

        match fs::read_dir(&dir) {
            Ok(entries) => {
                let found: Vec<PathBuf> =
                    entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
                ExecutionResult::Found(found)
            }
            Err(e) => ExecutionResult::Error(format!("Cannot list {}: {}", dir.display(), e)),
        }
    }

    /// Search file contents (grep-like)
    fn execute_search(&self, _cmd: &ParsedCommand) -> ExecutionResult {
        // For now, use semantic search instead
        ExecutionResult::Unsupported("Use Semantic Search (S) for content search".to_string())
    }

    /// Prepare copy operation (returns dry run)
    fn prepare_copy(&self, cmd: &ParsedCommand) -> ExecutionResult {
        let files = self.resolve_targets(cmd);
        if files.is_empty() {
            return ExecutionResult::Error("No files found to copy".to_string());
        }

        let dest = match &cmd.destination {
            Some(d) => self.cwd.join(d),
            None => return ExecutionResult::Error("No destination specified".to_string()),
        };

        let ops: Vec<DryRunOperation> = files
            .into_iter()
            .map(|f| {
                let dest_path = if dest.is_dir() {
                    dest.join(f.file_name().unwrap_or_default())
                } else {
                    dest.clone()
                };
                DryRunOperation::new(
                    DryRunOpType::Copy,
                    f.clone(),
                    format!("Copy to {}", dest_path.display()),
                )
                .with_dest(dest_path)
            })
            .collect();

        ExecutionResult::NeedsDryRun(ops)
    }

    /// Prepare move operation (returns dry run)
    fn prepare_move(&self, cmd: &ParsedCommand) -> ExecutionResult {
        let files = self.resolve_targets(cmd);
        if files.is_empty() {
            return ExecutionResult::Error("No files found to move".to_string());
        }

        let dest = match &cmd.destination {
            Some(d) => self.cwd.join(d),
            None => return ExecutionResult::Error("No destination specified".to_string()),
        };

        let ops: Vec<DryRunOperation> = files
            .into_iter()
            .map(|f| {
                let dest_path = if dest.is_dir() {
                    dest.join(f.file_name().unwrap_or_default())
                } else {
                    dest.clone()
                };
                DryRunOperation::new(
                    DryRunOpType::Rename,
                    f.clone(),
                    format!("Move to {}", dest_path.display()),
                )
                .with_dest(dest_path)
            })
            .collect();

        ExecutionResult::NeedsDryRun(ops)
    }

    /// Prepare delete operation (returns dry run)
    fn prepare_delete(&self, cmd: &ParsedCommand) -> ExecutionResult {
        let files = self.resolve_targets(cmd);
        if files.is_empty() {
            return ExecutionResult::Error("No files found to delete".to_string());
        }

        let ops: Vec<DryRunOperation> = files
            .into_iter()
            .map(|f| {
                let size = fs::metadata(&f).map(|m| m.len()).unwrap_or(0);
                DryRunOperation::new(
                    DryRunOpType::Delete,
                    f.clone(),
                    format!("Delete ({} bytes)", size),
                )
            })
            .collect();

        ExecutionResult::NeedsDryRun(ops)
    }

    /// Prepare rename operation (returns dry run)
    fn prepare_rename(&self, cmd: &ParsedCommand) -> ExecutionResult {
        if cmd.targets.is_empty() {
            return ExecutionResult::Error("No file specified to rename".to_string());
        }

        let source = self.cwd.join(&cmd.targets[0]);
        if !source.exists() {
            return ExecutionResult::Error(format!("File not found: {}", cmd.targets[0]));
        }

        let dest = match &cmd.destination {
            Some(d) => self.cwd.join(d),
            None => return ExecutionResult::Error("No new name specified".to_string()),
        };

        let op = DryRunOperation::new(
            DryRunOpType::Rename,
            source,
            format!("Rename to {}", dest.display()),
        )
        .with_dest(dest);

        ExecutionResult::NeedsDryRun(vec![op])
    }

    /// Execute create file
    fn execute_create_file(&self, cmd: &ParsedCommand) -> ExecutionResult {
        if cmd.targets.is_empty() {
            return ExecutionResult::Error("No filename specified".to_string());
        }

        let path = self.cwd.join(&cmd.targets[0]);
        if path.exists() {
            return ExecutionResult::Error(format!("File already exists: {}", cmd.targets[0]));
        }

        // Use dry run for safety
        let op = DryRunOperation::new(DryRunOpType::Create, path, "Create new file");
        ExecutionResult::NeedsDryRun(vec![op])
    }

    /// Execute create directory
    fn execute_create_dir(&self, cmd: &ParsedCommand) -> ExecutionResult {
        if cmd.targets.is_empty() {
            return ExecutionResult::Error("No directory name specified".to_string());
        }

        let path = self.cwd.join(&cmd.targets[0]);
        if path.exists() {
            return ExecutionResult::Error(format!("Directory already exists: {}", cmd.targets[0]));
        }

        // Use dry run for safety
        let op = DryRunOperation::new(DryRunOpType::Create, path, "Create new directory");
        ExecutionResult::NeedsDryRun(vec![op])
    }

    /// Show file info
    fn execute_info(&self, cmd: &ParsedCommand) -> ExecutionResult {
        if cmd.targets.is_empty() {
            return ExecutionResult::Error("No file specified".to_string());
        }

        let path = self.cwd.join(&cmd.targets[0]);
        match fs::metadata(&path) {
            Ok(meta) => {
                let info = format!(
                    "{}: {} bytes, {}",
                    path.display(),
                    meta.len(),
                    if meta.is_dir() { "directory" } else { "file" }
                );
                ExecutionResult::Success(info)
            }
            Err(e) => ExecutionResult::Error(format!("Cannot read {}: {}", path.display(), e)),
        }
    }

    /// Resolve target files from command (handles patterns)
    fn resolve_targets(&self, cmd: &ParsedCommand) -> Vec<PathBuf> {
        let mut files = Vec::new();

        // First try pattern
        if let Some(pattern) = &cmd.pattern {
            let glob_pattern = self.cwd.join(pattern).to_string_lossy().to_string();
            if let Ok(paths) = glob(&glob_pattern) {
                files.extend(paths.filter_map(|p| p.ok()));
            }
        }

        // Then try explicit targets
        for target in &cmd.targets {
            let path = self.cwd.join(target);
            if path.exists() {
                files.push(path);
            } else {
                // Try as pattern
                let glob_pattern = self.cwd.join(target).to_string_lossy().to_string();
                if let Ok(paths) = glob(&glob_pattern) {
                    files.extend(paths.filter_map(|p| p.ok()));
                }
            }
        }

        files
    }

    /// Execute confirmed dry run operations
    pub fn execute_confirmed(ops: &[DryRunOperation]) -> Result<usize, String> {
        let mut executed = 0;

        for op in ops {
            match op.op_type {
                DryRunOpType::Copy => {
                    if let Some(dest) = &op.dest_path {
                        fs::copy(&op.path, dest).map_err(|e| format!("Copy failed: {}", e))?;
                        executed += 1;
                    }
                }
                DryRunOpType::Rename => {
                    if let Some(dest) = &op.dest_path {
                        fs::rename(&op.path, dest)
                            .map_err(|e| format!("Rename/move failed: {}", e))?;
                        executed += 1;
                    }
                }
                DryRunOpType::Delete => {
                    if op.path.is_dir() {
                        fs::remove_dir_all(&op.path)
                            .map_err(|e| format!("Delete failed: {}", e))?;
                    } else {
                        fs::remove_file(&op.path).map_err(|e| format!("Delete failed: {}", e))?;
                    }
                    executed += 1;
                }
                DryRunOpType::Create => {
                    if op.description.contains("directory") {
                        fs::create_dir_all(&op.path)
                            .map_err(|e| format!("Create dir failed: {}", e))?;
                    } else {
                        fs::File::create(&op.path)
                            .map_err(|e| format!("Create file failed: {}", e))?;
                    }
                    executed += 1;
                }
                DryRunOpType::Modify | DryRunOpType::Execute => {
                    // Not implemented yet
                }
            }
        }

        Ok(executed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_executor_creation() {
        let executor = CommandExecutor::new(PathBuf::from("/tmp"));
        assert_eq!(executor.cwd, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_find_in_current_dir() {
        let executor = CommandExecutor::new(env::current_dir().unwrap());
        let cmd = ParsedCommand {
            action: CommandAction::Find,
            targets: vec![],
            destination: None,
            pattern: Some("*".to_string()), // Find all files
            original: "find files".to_string(),
            confidence: 0.9,
            explanation: "Find files".to_string(),
        };

        match executor.execute(&cmd) {
            ExecutionResult::Found(_files) => {
                // Found is the expected result type (may be empty)
            }
            other => panic!("Expected Found, got {:?}", other),
        }
    }

    #[test]
    fn test_delete_needs_dry_run() {
        let executor = CommandExecutor::new(env::current_dir().unwrap());
        let cmd = ParsedCommand {
            action: CommandAction::Delete,
            targets: vec!["*.tmp".to_string()],
            destination: None,
            pattern: Some("*.tmp".to_string()),
            original: "delete temp files".to_string(),
            confidence: 0.9,
            explanation: "Delete temporary files".to_string(),
        };

        // Should return NeedsDryRun or Error (no files)
        match executor.execute(&cmd) {
            ExecutionResult::NeedsDryRun(_) | ExecutionResult::Error(_) => {}
            other => panic!("Expected NeedsDryRun or Error, got {:?}", other),
        }
    }
}
