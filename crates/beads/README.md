# beads-cli

A Rust wrapper for the Beads (bd) git-native issue tracker CLI.

## Features

- List issues with status/type filters
- View ready and blocked issues
- Create, update, and close issues
- Dependency management
- Comments and labels
- Project statistics and activity logs
- Sync with git remote
- Admin operations (init, doctor)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
beads-cli = { path = "../crates/beads" }  # or from crates.io when published
```

## Usage

### Basic Usage

```rust
use beads_cli::Beads;

fn main() -> beads_cli::Result<()> {
    let bd = Beads::new()?;

    // List all open issues
    let issues = bd.list_open()?;
    for issue in issues {
        println!("{}: {}", issue.id, issue.title);
    }

    // Get ready issues (no blockers)
    let ready = bd.ready()?;
    println!("{} issues ready to work on", ready.len());

    Ok(())
}
```

### Working Directory

```rust
// Create with specific working directory
let bd = Beads::with_workdir("/path/to/repo");

// Or set it later
let mut bd = Beads::new()?;
bd.set_workdir("/path/to/repo");
```

### Listing Issues

```rust
// List with filters
let open = bd.list(Some("open"), None)?;
let bugs = bd.list(None, Some("bug"))?;
let open_bugs = bd.list(Some("open"), Some("bug"))?;

// Convenience methods
let open = bd.list_open()?;
let in_progress = bd.list_in_progress()?;
let closed = bd.list_closed()?;
let epics = bd.list_epics()?;
let open_epics = bd.list_open_epics()?;

// Ready (no blockers) and blocked
let ready = bd.ready()?;
let blocked = bd.blocked()?;
```

### Issue Details

```rust
// Show single issue
let issue = bd.show("PROJ-123")?;
println!("Title: {}", issue.title);
println!("Status: {}", issue.status);
println!("Type: {}", issue.issue_type);
println!("Priority: {:?}", issue.priority);

// Search issues
let results = bd.search("authentication")?;
```

### Creating Issues

```rust
// Simple creation
bd.create("Fix login bug", "bug", Some(2), None)?;

// Create with parent
bd.create("Implement feature X", "task", Some(2), Some("EPIC-1"))?;

// Create epic
bd.create_epic("Q1 Release", Some(1))?;

// Create child (auto-adds dependency)
bd.create_child("Write tests", "task", "PROJ-123", Some(3))?;

// Full creation with all options
bd.create_full(
    "Detailed task",
    "task",
    Some(2),                          // priority
    Some("Full description here"),    // description
    Some("user@example.com"),         // assignee
    Some("EPIC-1"),                   // parent
    Some(&["backend", "urgent"]),     // labels
)?;
```

### Updating Issues

```rust
// Update status
bd.update_status("PROJ-123", "in_progress")?;

// Update with multiple fields
bd.update(
    "PROJ-123",
    Some("closed"),    // status
    Some(1),           // priority
    Some("user@example.com"), // assignee
    None,              // title (unchanged)
)?;

// Close issues
bd.close("PROJ-123")?;
bd.close_with_reason("PROJ-124", "Duplicate of PROJ-100")?;
bd.close_multiple(&["PROJ-125", "PROJ-126", "PROJ-127"])?;

// Reopen
bd.reopen("PROJ-123")?;
```

### Dependencies

```rust
// Add dependency (PROJ-124 depends on PROJ-123)
bd.dep_add("PROJ-124", "PROJ-123")?;

// Remove dependency
bd.dep_remove("PROJ-124", "PROJ-123")?;
```

### Comments

```rust
// Get comments
let comments = bd.comments("PROJ-123")?;
for comment in comments {
    println!("{}: {}", comment.author, comment.content);
}

// Add comment
bd.comment_add("PROJ-123", "Working on this now")?;
```

### Labels

```rust
// Add label
bd.label_add("PROJ-123", "urgent")?;

// Remove label
bd.label_remove("PROJ-123", "urgent")?;
```

### Statistics

```rust
// Get stats
let stats = bd.stats()?;
println!("Total: {}", stats.total);
println!("Open: {}", stats.open);
println!("In Progress: {}", stats.in_progress);
println!("Closed: {}", stats.closed);
println!("Blocked: {}", stats.blocked);

// Get combined status info
let info = bd.status_info()?;
println!("Open: {}, Ready: {}", info.open, info.ready);
```

### Activity

```rust
// Get global activity
let activity = bd.activity(Some(50))?;
for event in activity {
    println!("{}: {} on {:?}", event.timestamp, event.action, event.issue_id);
}

// Get activity for specific issue
let issue_activity = bd.activity_for_issue("PROJ-123", Some(20))?;
```

### Sync and Admin

```rust
// Sync with remote
bd.sync()?;

// Check sync status
let status = bd.sync_status()?;
println!("{}", status.stdout);

// Initialize beads
bd.init()?;

// Run health checks
let doctor = bd.doctor()?;
println!("{}", doctor.combined());

// Get help
let help = bd.human()?;
println!("{}", help.stdout);
```

### Raw Command Execution

For commands not covered by the API:

```rust
let output = bd.run(&["export", "--format", "csv"])?;
println!("{}", output.stdout);
```

## Data Types

### Issue

```rust
pub struct Issue {
    pub id: String,
    pub title: String,
    pub status: String,
    pub issue_type: String,
    pub priority: Option<u8>,
    pub description: Option<String>,
    pub assignee: Option<String>,
    pub parent: Option<String>,
    pub labels: Vec<String>,
    pub depends_on: Vec<String>,
    pub blocks: Vec<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
```

### Stats

```rust
pub struct Stats {
    pub total: usize,
    pub open: usize,
    pub in_progress: usize,
    pub closed: usize,
    pub blocked: usize,
    pub epics: usize,
}
```

### Comment

```rust
pub struct Comment {
    pub id: Option<String>,
    pub author: String,
    pub content: String,
    pub created_at: Option<String>,
}
```

### Activity

```rust
pub struct Activity {
    pub timestamp: String,
    pub action: String,
    pub issue_id: Option<String>,
    pub details: Option<String>,
}
```

### Status and IssueType Enums

```rust
pub enum Status {
    Open,
    InProgress,
    Closed,
}

pub enum IssueType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
}
```

## Error Handling

All operations return `beads_cli::Result<T>`, which is `Result<T, beads_cli::Error>`:

```rust
match bd.show("PROJ-999") {
    Ok(issue) => println!("{}", issue.title),
    Err(beads_cli::Error::NotInstalled) => eprintln!("bd not installed"),
    Err(beads_cli::Error::NotInRepo) => eprintln!("Not in a beads repository"),
    Err(beads_cli::Error::IssueNotFound(id)) => eprintln!("Issue {} not found", id),
    Err(beads_cli::Error::CommandFailed(msg)) => eprintln!("Command failed: {}", msg),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Requirements

- bd (beads) must be installed and available in PATH
- Repository must be initialized with beads (`bd init`)

## License

MIT
