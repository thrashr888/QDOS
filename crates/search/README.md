# search-tools

A unified Rust wrapper for code search tools: ripgrep (rg), The Silver Searcher (ag), grep, and ack.

## Features

- Auto-detect best available tool
- Unified API across all tools
- Rich search options (case, hidden files, file types, globs, context)
- Match grouping and file listing
- Count and existence checks
- Working directory support

## Supported Tools

| Tool | Command | Description |
|------|---------|-------------|
| ripgrep | `rg` | Fastest, recommended |
| The Silver Searcher | `ag` | Fast code search |
| ack | `ack` | Perl-based code search |
| GNU grep | `grep` | Standard Unix grep |

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
search-tools = { path = "../crates/search" }  # or from crates.io when published
```

## Usage

### Basic Usage

```rust
use search_tools::{Search, SearchTool};

fn main() -> search_tools::Result<()> {
    // Auto-detect best available tool
    let search = Search::auto()?;

    // Search for a pattern
    let results = search.search("TODO", Some("."))?;

    println!("Found {} matches using {}", results.match_count, results.tool);

    for m in results.matches {
        println!("{}:{}: {}", m.file, m.line_number.unwrap_or(0), m.line);
    }

    Ok(())
}
```

### Specific Tool

```rust
// Use a specific search tool
let search = Search::new(SearchTool::Ripgrep)?;

// Or fallback to another
let search = Search::new(SearchTool::Grep)?;
```

### Working Directory

```rust
let search = Search::auto()?
    .with_working_dir("/path/to/project");

let results = search.search("pattern", None)?;
```

### Search Options

```rust
use search_tools::{Search, SearchOptions};

let search = Search::auto()?;

// Use preset options
let opts = SearchOptions::case_insensitive();
let results = search.search_with_options("error", Some("."), &opts)?;

// Or customize options
let opts = SearchOptions {
    ignore_case: true,
    hidden: true,
    file_type: Some("rust".to_string()),
    max_count: Some(100),
    before_context: Some(2),
    after_context: Some(2),
    ..Default::default()
};

let results = search.search_with_options("fn main", Some("src"), &opts)?;
```

### Preset Options

```rust
// Case insensitive
let opts = SearchOptions::case_insensitive();

// Include hidden files
let opts = SearchOptions::with_hidden();

// List files only (no content)
let opts = SearchOptions::files_list();

// With context lines
let opts = SearchOptions::with_context(3);

// Filter by file type
let opts = SearchOptions::for_file_type("rust");
```

### File Listing

```rust
// Get just file names
let files = search.search_files("pattern", Some("."))?;
for file in files {
    println!("{}", file);
}
```

### Count Matches

```rust
let count = search.count("TODO", Some("."))?;
println!("Found {} TODOs", count);
```

### Existence Check

```rust
// Fast check if pattern exists anywhere
if search.exists("FIXME", Some("."))? {
    println!("Found FIXME comments!");
}
```

### Working with Results

```rust
let results = search.search("error", Some("."))?;

// Check if empty
if results.is_empty() {
    println!("No matches found");
}

// Get unique files
let files = results.files();
println!("Matches in {} files", files.len());

// Group by file
let by_file = results.by_file();
for (file, matches) in by_file {
    println!("{}: {} matches", file, matches.len());
}
```

### Tool Information

```rust
use search_tools::{SearchTool, available_tools, best_tool};

// Check which tools are available
for tool in available_tools() {
    println!("{} is available", tool.name());
    if let Some(version) = tool.version() {
        println!("  Version: {}", version);
    }
}

// Get the best available tool
if let Some(tool) = best_tool() {
    println!("Best tool: {}", tool);
}

// Check specific tool
if SearchTool::Ripgrep.is_available() {
    println!("ripgrep is installed");
}
```

## Data Types

### Match

```rust
pub struct Match {
    pub file: String,
    pub line_number: Option<usize>,
    pub column: Option<usize>,
    pub line: String,
}
```

### SearchOutput

```rust
pub struct SearchOutput {
    pub tool: SearchTool,
    pub matches: Vec<Match>,
    pub match_count: usize,
    pub success: bool,
    pub raw_output: String,
}
```

### SearchOptions

```rust
pub struct SearchOptions {
    pub ignore_case: bool,
    pub case_sensitive: bool,
    pub hidden: bool,
    pub follow: bool,
    pub file_type: Option<String>,
    pub glob: Option<String>,
    pub exclude: Option<String>,
    pub max_count: Option<usize>,
    pub max_depth: Option<usize>,
    pub before_context: Option<usize>,
    pub after_context: Option<usize>,
    pub word_boundary: bool,
    pub fixed_string: bool,
    pub files_only: bool,
    pub line_numbers: bool,
    pub column_numbers: bool,
    pub count_only: bool,
    pub quiet: bool,
    pub sort: bool,
}
```

### SearchTool

```rust
pub enum SearchTool {
    Ripgrep,  // rg
    Ag,       // ag (The Silver Searcher)
    Grep,     // grep
    Ack,      // ack
}
```

## Error Handling

All operations return `search_tools::Result<T>`:

```rust
match Search::auto() {
    Ok(search) => println!("Using {}", search.tool()),
    Err(search_tools::Error::NoToolAvailable) => {
        eprintln!("No search tool installed");
    }
    Err(search_tools::Error::ToolNotInstalled(name)) => {
        eprintln!("{} is not installed", name);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Requirements

At least one of these tools must be installed:
- `rg` (ripgrep) - **recommended**
- `ag` (The Silver Searcher)
- `ack`
- `grep` (usually pre-installed on Unix)

## License

MIT
