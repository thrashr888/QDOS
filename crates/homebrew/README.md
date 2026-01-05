# homebrew

A Rust wrapper for the Homebrew package manager CLI.

## Features

- List installed packages (formulae and casks)
- Search for packages
- Get detailed package information (versions, dependencies, caveats)
- Install, uninstall, and upgrade packages
- Manage taps (third-party repositories)
- Check for outdated packages
- Run arbitrary brew commands

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
homebrew = { path = "../crates/homebrew" }  # or from crates.io when published
```

## Usage

### Basic Usage

```rust
use homebrew::Homebrew;

fn main() -> homebrew::Result<()> {
    let brew = Homebrew::new()?;

    // List installed packages
    for pkg in brew.list()? {
        println!("{} ({})", pkg.name, pkg.version);
    }

    Ok(())
}
```

### Search for Packages

```rust
let results = brew.search("json")?;
for r in results {
    let pkg_type = if r.is_cask { "cask" } else { "formula" };
    println!("{} ({})", r.name, pkg_type);
}
```

### Get Package Info

```rust
let info = brew.info("git")?;
println!("Name: {}", info.name);
println!("Version: {}", info.version);
println!("Description: {}", info.description);
println!("Installed: {}", info.installed);
println!("Dependencies: {:?}", info.dependencies);
```

### Install/Uninstall Packages

```rust
// Install a formula
let output = brew.install("ripgrep")?;
if output.success {
    println!("Installed successfully!");
}

// Install a cask
brew.install_cask("visual-studio-code")?;

// Uninstall
brew.uninstall("ripgrep")?;
```

### Check for Updates

```rust
// Update Homebrew
brew.update()?;

// Get outdated packages
let outdated = brew.outdated()?;
println!("{} packages need updating", outdated.len());

// Upgrade a specific package
brew.upgrade("git")?;

// Upgrade all packages
brew.upgrade_all()?;
```

### Manage Taps

```rust
// Add a tap
brew.tap("homebrew/cask-fonts")?;

// List taps
for tap in brew.list_taps()? {
    let status = if tap.official { "official" } else { "third-party" };
    println!("{} ({})", tap.name, status);
}

// Remove a tap
brew.untap("homebrew/cask-fonts")?;
```

### Utilities

```rust
// Open homepage in browser
brew.home("git")?;

// Clean up old versions
brew.cleanup()?;

// Check for problems
let doctor = brew.doctor()?;
println!("{}", doctor.stdout);
```

### Raw Command Execution

For commands not covered by the API:

```rust
let output = brew.run(&["deps", "--tree", "git"])?;
println!("{}", output.stdout);
```

## Data Types

### Package

Full package information from `brew info`:

```rust
pub struct Package {
    pub name: String,
    pub full_name: Option<String>,
    pub version: String,
    pub description: String,
    pub homepage: String,
    pub installed: bool,
    pub installed_version: Option<String>,
    pub outdated: bool,
    pub package_type: PackageType,  // Formula or Cask
    pub dependencies: Vec<String>,
    pub caveats: Option<String>,
    pub license: Option<String>,
}
```

### InstalledPackage

Basic info from `brew list`:

```rust
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
}
```

### SearchResult

Result from `brew search`:

```rust
pub struct SearchResult {
    pub name: String,
    pub is_cask: bool,
}
```

## Error Handling

All operations return `homebrew::Result<T>`, which is `Result<T, homebrew::Error>`:

```rust
match brew.info("nonexistent") {
    Ok(info) => println!("Found: {}", info.name),
    Err(homebrew::Error::CommandFailed(msg)) => eprintln!("Command failed: {}", msg),
    Err(homebrew::Error::NotInstalled) => eprintln!("Homebrew not installed"),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Platform Support

This crate only works on macOS where Homebrew is available. On other platforms, `Homebrew::new()` will return `Error::NotInstalled`.

## License

MIT
