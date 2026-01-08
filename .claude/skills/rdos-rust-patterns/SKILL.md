---
name: rdos-rust-patterns
description: Rust best practices and overall approach for R-DOS development. Use when writing Rust code, implementing features, handling errors, or structuring code in the R-DOS codebase.
---

# R-DOS Rust Patterns

R-DOS is a retro DOS-style file manager TUI in Rust using ratatui. It recreates Q-DOS II (1991, Gazelle Systems) with modern Rust patterns.

## Error Handling

```rust
// Use Result for fallible operations
pub fn load_data(&mut self) -> Result<(), String> {
    let output = Command::new("tool")
        .output()
        .map_err(|e| format!("Failed to run: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}
```

## State Types

Always derive common traits and use Default:

```rust
#[derive(Debug, Clone, Default)]
pub struct MyState {
    pub view: MyView,
    pub items: Vec<MyItem>,
    pub selected: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MyView {
    #[default]
    List,
    Detail,
    Error,
}
```

## External Processes

Suppress stdout/stderr for background processes:

```rust
use std::process::{Command, Stdio};

Command::new("player")
    .arg(&file_path)
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn();
```

## File Operations

Pattern for sibling file detection:

```rust
pub fn detect_siblings(&mut self) {
    let Some(ref file_path) = self.file_path else { return };
    let Some(parent) = file_path.parent() else { return };

    let mut siblings: Vec<PathBuf> = std::fs::read_dir(parent)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_valid_file(p))
        .collect();

    siblings.sort();
    self.current_index = siblings.iter().position(|p| p == file_path).unwrap_or(0);
    self.sibling_files = siblings;
}
```

## Documentation

```rust
//! Module-level doc comment
//!
//! Extended description.

/// Function documentation
pub fn my_function() { ... }
```

## Quality Checks

Run before committing:

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## Avoid

- Sentinel values (use Option<T>)
- Ignoring clippy warnings
- Hardcoded paths (use config or cwd)
- Blocking the main thread (use background threads for long operations)
