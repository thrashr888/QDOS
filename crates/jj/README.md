# jj-cli

A Rust wrapper for the Jujutsu (jj) version control CLI.

## Features

- Status and working copy information
- Change log viewing and parsing
- Diff operations with multiple formats
- Change management (describe, new, squash, abandon, etc.)
- Bookmark (branch) operations
- Operation history and undo
- Git integration (push, fetch, clone)
- Conflict detection
- Workspace management
- Arbitrary command execution

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
jj-cli = { path = "../crates/jj" }  # or from crates.io when published
```

## Usage

### Basic Usage

```rust
use jj_cli::Jj;

fn main() -> jj_cli::Result<()> {
    let jj = Jj::new()?;

    // Get status
    let status = jj.status()?;
    println!("{}", status.stdout);

    // View log
    let changes = jj.log(Some(10))?;
    for change in changes {
        println!("{}: {}", change.change_id, change.description);
    }

    Ok(())
}
```

### Working Directory

```rust
// Create with specific working directory
let jj = Jj::with_workdir("/path/to/repo");

// Or set it later
let mut jj = Jj::new()?;
jj.set_workdir("/path/to/repo");
```

### Working Copy Status

```rust
let status = jj.working_copy_status()?;
println!("Change ID: {:?}", status.change_id);
println!("Empty: {}", status.is_empty);
println!("Bookmark: {:?}", status.bookmark);
println!("Modified files: {}", status.modified_count);
```

### Viewing Changes

```rust
// Get parsed change log
let changes = jj.log(Some(20))?;
for change in changes {
    let markers = if change.is_working_copy { "@" } else { " " };
    println!("{} {} - {}", markers, change.change_id, change.description);
}

// Get raw log output
let log = jj.log_raw(Some(10))?;
println!("{}", log.stdout);
```

### Diff Operations

```rust
use jj_cli::DiffFormat;

// Default diff
let diff = jj.diff(None, DiffFormat::Default)?;

// Git format diff
let git_diff = jj.diff(None, DiffFormat::Git)?;

// Just file stats
let stat_diff = jj.diff(None, DiffFormat::Stat)?;

// Diff for specific revision
let rev_diff = jj.diff(Some("abc123"), DiffFormat::Git)?;

// Get file changes as parsed structs
let files = jj.diff_files()?;
for file in files {
    println!("{} {}", file.status, file.path.display());
}
```

### Change Operations

```rust
// Update description
jj.describe("Fix: resolve authentication bug")?;

// Create new change
jj.new_change()?;

// Create new change with message
jj.new_change_with_message("WIP: new feature")?;

// Edit a specific change
jj.edit("abc123")?;

// Squash into parent
jj.squash()?;

// Abandon current change
jj.abandon(None)?;

// Abandon specific revision
jj.abandon(Some("abc123"))?;
```

### Bookmark Operations

```rust
// List bookmarks
let bookmarks = jj.bookmark_list()?;
for bookmark in bookmarks {
    let remote = bookmark.remote.as_deref().unwrap_or("");
    println!("{}{}", bookmark.name, if remote.is_empty() { "" } else { &format!("@{}", remote) });
}

// Create bookmark
jj.bookmark_create("feature-branch")?;

// Create bookmark at revision
jj.bookmark_create_at("release", "abc123")?;

// Delete bookmark
jj.bookmark_delete("old-branch")?;

// Move bookmark to current revision
jj.bookmark_set("main")?;
```

### Operation History

```rust
// List operations
let operations = jj.operation_log()?;
for op in operations {
    let marker = if op.is_current { "*" } else { " " };
    println!("{} {} ({}) {}", marker, op.id, op.time, op.description);
}

// Undo last operation
jj.undo()?;

// Restore to specific operation
jj.operation_restore("abc12345")?;
```

### Git Integration

```rust
// Fetch from remote
jj.git_fetch()?;

// Fetch from all remotes
jj.git_fetch_all()?;

// Push to remote
jj.git_push()?;

// Push all bookmarks
jj.git_push_all()?;

// Push specific bookmark
jj.git_push_bookmark("main")?;

// Initialize colocated repo
jj.git_init()?;

// Clone repository
jj.git_clone("https://github.com/user/repo", "repo")?;
```

### Conflict Detection

```rust
// Check for conflicts
if jj.has_conflicts()? {
    println!("Repository has conflicts!");

    // List conflicting files
    let conflicts = jj.conflict_list()?;
    for file in conflicts {
        println!("  Conflict: {}", file);
    }
}
```

### Workspace Operations

```rust
// List workspaces
let workspaces = jj.workspace_list()?;
println!("{}", workspaces.stdout);

// Add workspace
jj.workspace_add("../my-workspace", Some("dev"))?;

// Forget workspace
jj.workspace_forget("dev")?;
```

### Raw Command Execution

For commands not covered by the API:

```rust
let output = jj.run(&["config", "list"])?;
println!("{}", output.stdout);
```

## Data Types

### Change

```rust
pub struct Change {
    pub change_id: String,
    pub commit_id: String,
    pub description: String,
    pub author: String,
    pub timestamp: String,
    pub is_working_copy: bool,
    pub is_empty: bool,
    pub bookmarks: Vec<String>,
}
```

### FileStatus

```rust
pub struct FileStatus {
    pub path: PathBuf,
    pub status: char,  // M, A, D, R
    pub added: Option<usize>,
    pub removed: Option<usize>,
}
```

### Bookmark

```rust
pub struct Bookmark {
    pub name: String,
    pub remote: Option<String>,
    pub target: Option<String>,
    pub is_current: bool,
    pub is_tracking: bool,
}
```

### Operation

```rust
pub struct Operation {
    pub id: String,
    pub is_current: bool,
    pub time: String,
    pub description: String,
}
```

### WorkingCopyStatus

```rust
pub struct WorkingCopyStatus {
    pub change_id: Option<String>,
    pub is_empty: bool,
    pub bookmark: Option<String>,
    pub modified_count: usize,
    pub has_changes: bool,
}
```

### DiffFormat

```rust
pub enum DiffFormat {
    Default,
    Git,
    Summary,
    Stat,
    ColorWords,
}
```

## Error Handling

All operations return `jj_cli::Result<T>`, which is `Result<T, jj_cli::Error>`:

```rust
match jj.status() {
    Ok(output) => println!("{}", output.stdout),
    Err(jj_cli::Error::NotInstalled) => eprintln!("jj not installed"),
    Err(jj_cli::Error::NotInRepo) => eprintln!("Not in a jj repository"),
    Err(jj_cli::Error::CommandFailed(msg)) => eprintln!("Command failed: {}", msg),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Requirements

- jj (Jujutsu) must be installed and available in PATH
- Works on all platforms where jj is available

## License

MIT
