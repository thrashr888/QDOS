//! Dependency Manager CLI operations

use super::state::{PackageEntry, PackageManager, SearchResult};
use std::path::Path;
use std::process::Command;

/// Run a command and capture output
fn run_command(cmd: &str, args: &[&str], cwd: &Path) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", cmd, e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if stderr.is_empty() {
            Err(format!("{} failed with no output", cmd))
        } else {
            Err(stderr)
        }
    }
}

/// Check if a command is available
pub fn check_command(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// List installed packages
pub fn list_packages(pm: PackageManager, cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    match pm {
        PackageManager::Cargo => list_cargo_packages(cwd),
        PackageManager::Npm => list_npm_packages(cwd),
        PackageManager::Pnpm => list_pnpm_packages(cwd),
        PackageManager::Yarn => list_yarn_packages(cwd),
        PackageManager::Pip => list_pip_packages(cwd),
        PackageManager::Uv => list_uv_packages(cwd),
        PackageManager::Poetry => list_poetry_packages(cwd),
        PackageManager::GoMod => list_go_packages(cwd),
    }
}

/// Check for outdated packages
pub fn check_outdated(pm: PackageManager, cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    match pm {
        PackageManager::Cargo => check_cargo_outdated(cwd),
        PackageManager::Npm => check_npm_outdated(cwd),
        PackageManager::Pnpm => check_pnpm_outdated(cwd),
        PackageManager::Yarn => check_yarn_outdated(cwd),
        PackageManager::Pip => check_pip_outdated(cwd),
        PackageManager::Uv => check_uv_outdated(cwd),
        PackageManager::Poetry => check_poetry_outdated(cwd),
        PackageManager::GoMod => check_go_outdated(cwd),
    }
}

/// Install a package
pub fn install_package(
    pm: PackageManager,
    name: &str,
    dev: bool,
    cwd: &Path,
) -> Result<String, String> {
    let output = match pm {
        PackageManager::Cargo => {
            if dev {
                run_command("cargo", &["add", "--dev", name], cwd)?
            } else {
                run_command("cargo", &["add", name], cwd)?
            }
        }
        PackageManager::Npm => {
            if dev {
                run_command("npm", &["install", "-D", name], cwd)?
            } else {
                run_command("npm", &["install", name], cwd)?
            }
        }
        PackageManager::Pnpm => {
            if dev {
                run_command("pnpm", &["add", "-D", name], cwd)?
            } else {
                run_command("pnpm", &["add", name], cwd)?
            }
        }
        PackageManager::Yarn => {
            if dev {
                run_command("yarn", &["add", "-D", name], cwd)?
            } else {
                run_command("yarn", &["add", name], cwd)?
            }
        }
        PackageManager::Pip => run_command("pip", &["install", name], cwd)?,
        PackageManager::Uv => run_command("uv", &["pip", "install", name], cwd)?,
        PackageManager::Poetry => {
            if dev {
                run_command("poetry", &["add", "--dev", name], cwd)?
            } else {
                run_command("poetry", &["add", name], cwd)?
            }
        }
        PackageManager::GoMod => run_command("go", &["get", name], cwd)?,
    };
    Ok(output)
}

/// Uninstall a package
pub fn uninstall_package(pm: PackageManager, name: &str, cwd: &Path) -> Result<String, String> {
    let output = match pm {
        PackageManager::Cargo => run_command("cargo", &["remove", name], cwd)?,
        PackageManager::Npm => run_command("npm", &["uninstall", name], cwd)?,
        PackageManager::Pnpm => run_command("pnpm", &["remove", name], cwd)?,
        PackageManager::Yarn => run_command("yarn", &["remove", name], cwd)?,
        PackageManager::Pip => run_command("pip", &["uninstall", "-y", name], cwd)?,
        PackageManager::Uv => run_command("uv", &["pip", "uninstall", name], cwd)?,
        PackageManager::Poetry => run_command("poetry", &["remove", name], cwd)?,
        PackageManager::GoMod => {
            // Go doesn't have direct uninstall, edit go.mod
            run_command("go", &["mod", "edit", "-droprequire", name], cwd)?
        }
    };
    Ok(output)
}

/// Update a package
pub fn update_package(pm: PackageManager, name: &str, cwd: &Path) -> Result<String, String> {
    let output = match pm {
        PackageManager::Cargo => run_command("cargo", &["update", "-p", name], cwd)?,
        PackageManager::Npm => run_command("npm", &["update", name], cwd)?,
        PackageManager::Pnpm => run_command("pnpm", &["update", name], cwd)?,
        PackageManager::Yarn => run_command("yarn", &["upgrade", name], cwd)?,
        PackageManager::Pip => run_command("pip", &["install", "-U", name], cwd)?,
        PackageManager::Uv => run_command("uv", &["pip", "install", "-U", name], cwd)?,
        PackageManager::Poetry => run_command("poetry", &["update", name], cwd)?,
        PackageManager::GoMod => run_command("go", &["get", "-u", name], cwd)?,
    };
    Ok(output)
}

/// Update all packages
pub fn update_all(pm: PackageManager, cwd: &Path) -> Result<String, String> {
    let output = match pm {
        PackageManager::Cargo => run_command("cargo", &["update"], cwd)?,
        PackageManager::Npm => run_command("npm", &["update"], cwd)?,
        PackageManager::Pnpm => run_command("pnpm", &["update"], cwd)?,
        PackageManager::Yarn => run_command("yarn", &["upgrade"], cwd)?,
        PackageManager::Pip => {
            // pip doesn't have update all, need to list outdated first
            return Err("pip doesn't support update all directly".to_string());
        }
        PackageManager::Uv => run_command("uv", &["pip", "compile", "--upgrade"], cwd)?,
        PackageManager::Poetry => run_command("poetry", &["update"], cwd)?,
        PackageManager::GoMod => run_command("go", &["get", "-u", "all"], cwd)?,
    };
    Ok(output)
}

/// Search for packages (limited support)
pub fn search_packages(
    pm: PackageManager,
    query: &str,
    _cwd: &Path,
) -> Result<Vec<SearchResult>, String> {
    match pm {
        PackageManager::Cargo => search_cargo(query),
        PackageManager::Npm => search_npm(query),
        _ => Err(format!("{} doesn't support package search", pm.name())),
    }
}

// === Cargo ===

fn list_cargo_packages(cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    // Parse Cargo.toml directly for simplicity
    let cargo_toml = cwd.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml)
        .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;

    let mut packages = Vec::new();
    let mut in_deps = false;
    let mut in_dev_deps = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "[dependencies]" {
            in_deps = true;
            in_dev_deps = false;
            continue;
        }
        if line == "[dev-dependencies]" {
            in_deps = false;
            in_dev_deps = true;
            continue;
        }
        if line.starts_with('[') {
            in_deps = false;
            in_dev_deps = false;
            continue;
        }

        if (in_deps || in_dev_deps) && line.contains('=') {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let name = parts[0].trim().to_string();
                let version = extract_cargo_version(parts[1].trim());
                packages.push(PackageEntry {
                    name,
                    current_version: Some(version),
                    latest_version: None,
                    is_dev: in_dev_deps,
                    is_outdated: false,
                });
            }
        }
    }

    Ok(packages)
}

fn extract_cargo_version(value: &str) -> String {
    // Handle both "1.0" and { version = "1.0", ... }
    if value.starts_with('"') {
        value.trim_matches('"').to_string()
    } else if value.contains("version") {
        // Table format
        if let Some(start) = value.find("version") {
            let rest = &value[start..];
            if let Some(eq) = rest.find('=') {
                let version_part = rest[eq + 1..].trim();
                if let Some(quote_start) = version_part.find('"') {
                    let after_quote = &version_part[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find('"') {
                        return after_quote[..quote_end].to_string();
                    }
                }
            }
        }
        value.to_string()
    } else {
        value.to_string()
    }
}

fn check_cargo_outdated(cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    // Try cargo-outdated if available
    let output = run_command("cargo", &["outdated", "--format", "json"], cwd);
    match output {
        Ok(json_str) => {
            // Parse JSON output
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                let mut packages = Vec::new();
                if let Some(deps) = json.get("dependencies").and_then(|d| d.as_array()) {
                    for dep in deps {
                        let name = dep.get("name").and_then(|n| n.as_str()).unwrap_or_default();
                        let current = dep.get("project").and_then(|v| v.as_str());
                        let latest = dep.get("latest").and_then(|v| v.as_str());
                        if current != latest {
                            packages.push(PackageEntry {
                                name: name.to_string(),
                                current_version: current.map(String::from),
                                latest_version: latest.map(String::from),
                                is_dev: false,
                                is_outdated: true,
                            });
                        }
                    }
                }
                return Ok(packages);
            }
            Ok(Vec::new())
        }
        Err(_) => {
            // cargo-outdated not installed
            Err("cargo-outdated not installed. Run: cargo install cargo-outdated".to_string())
        }
    }
}

fn search_cargo(query: &str) -> Result<Vec<SearchResult>, String> {
    let output = run_command(
        "cargo",
        &["search", query, "--limit", "10"],
        &std::env::current_dir().unwrap(),
    )?;
    let mut results = Vec::new();

    for line in output.lines() {
        // Format: name = "version" # description
        if let Some(eq_pos) = line.find('=') {
            let name = line[..eq_pos].trim().to_string();
            let rest = &line[eq_pos + 1..];
            let (version, description) = if let Some(hash_pos) = rest.find('#') {
                let v = rest[..hash_pos].trim().trim_matches('"').to_string();
                let d = rest[hash_pos + 1..].trim().to_string();
                (v, d)
            } else {
                (rest.trim().trim_matches('"').to_string(), String::new())
            };
            results.push(SearchResult {
                name,
                version,
                description,
            });
        }
    }

    Ok(results)
}

// === npm ===

fn list_npm_packages(cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    let output = run_command("npm", &["list", "--json", "--depth=0"], cwd)?;

    let mut packages = Vec::new();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
            for (name, info) in deps {
                let version = info.get("version").and_then(|v| v.as_str());
                packages.push(PackageEntry {
                    name: name.clone(),
                    current_version: version.map(String::from),
                    latest_version: None,
                    is_dev: false,
                    is_outdated: false,
                });
            }
        }
        if let Some(deps) = json.get("devDependencies").and_then(|d| d.as_object()) {
            for (name, info) in deps {
                let version = info.get("version").and_then(|v| v.as_str());
                packages.push(PackageEntry {
                    name: name.clone(),
                    current_version: version.map(String::from),
                    latest_version: None,
                    is_dev: true,
                    is_outdated: false,
                });
            }
        }
    }

    Ok(packages)
}

fn check_npm_outdated(cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    let output = run_command("npm", &["outdated", "--json"], cwd);
    // npm outdated returns non-zero if there are outdated packages
    let json_str = match output {
        Ok(s) => s,
        Err(s) => s, // stderr might have the JSON
    };

    let mut packages = Vec::new();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
        if let Some(deps) = json.as_object() {
            for (name, info) in deps {
                let current = info.get("current").and_then(|v| v.as_str());
                let latest = info.get("latest").and_then(|v| v.as_str());
                packages.push(PackageEntry {
                    name: name.clone(),
                    current_version: current.map(String::from),
                    latest_version: latest.map(String::from),
                    is_dev: false,
                    is_outdated: true,
                });
            }
        }
    }

    Ok(packages)
}

fn search_npm(query: &str) -> Result<Vec<SearchResult>, String> {
    let output = run_command(
        "npm",
        &["search", query, "--json"],
        &std::env::current_dir().unwrap(),
    )?;

    let mut results = Vec::new();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        if let Some(arr) = json.as_array() {
            for item in arr.iter().take(10) {
                let name = item
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_default();
                let desc = item
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or_default();
                let version = item
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                results.push(SearchResult {
                    name: name.to_string(),
                    description: desc.to_string(),
                    version: version.to_string(),
                });
            }
        }
    }

    Ok(results)
}

// === pnpm ===

fn list_pnpm_packages(cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    let output = run_command("pnpm", &["list", "--json", "--depth=0"], cwd)?;

    let mut packages = Vec::new();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        // pnpm list --json returns an array
        if let Some(arr) = json.as_array() {
            for project in arr {
                if let Some(deps) = project.get("dependencies").and_then(|d| d.as_object()) {
                    for (name, info) in deps {
                        let version = info.get("version").and_then(|v| v.as_str());
                        packages.push(PackageEntry {
                            name: name.clone(),
                            current_version: version.map(String::from),
                            latest_version: None,
                            is_dev: false,
                            is_outdated: false,
                        });
                    }
                }
                if let Some(deps) = project.get("devDependencies").and_then(|d| d.as_object()) {
                    for (name, info) in deps {
                        let version = info.get("version").and_then(|v| v.as_str());
                        packages.push(PackageEntry {
                            name: name.clone(),
                            current_version: version.map(String::from),
                            latest_version: None,
                            is_dev: true,
                            is_outdated: false,
                        });
                    }
                }
            }
        }
    }

    Ok(packages)
}

fn check_pnpm_outdated(cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    let output = run_command("pnpm", &["outdated", "--format", "json"], cwd);
    let json_str = match output {
        Ok(s) => s,
        Err(s) => s,
    };

    let mut packages = Vec::new();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
        if let Some(deps) = json.as_object() {
            for (name, info) in deps {
                let current = info.get("current").and_then(|v| v.as_str());
                let latest = info.get("latest").and_then(|v| v.as_str());
                packages.push(PackageEntry {
                    name: name.clone(),
                    current_version: current.map(String::from),
                    latest_version: latest.map(String::from),
                    is_dev: false,
                    is_outdated: true,
                });
            }
        }
    }

    Ok(packages)
}

// === Yarn ===

fn list_yarn_packages(cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    let output = run_command("yarn", &["list", "--json", "--depth=0"], cwd)?;

    let mut packages = Vec::new();
    // Yarn outputs NDJSON
    for line in output.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if json.get("type").and_then(|t| t.as_str()) == Some("tree") {
                if let Some(data) = json
                    .get("data")
                    .and_then(|d| d.get("trees"))
                    .and_then(|t| t.as_array())
                {
                    for item in data {
                        if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                            // Format: name@version
                            let parts: Vec<&str> = name.rsplitn(2, '@').collect();
                            if parts.len() == 2 {
                                packages.push(PackageEntry {
                                    name: parts[1].to_string(),
                                    current_version: Some(parts[0].to_string()),
                                    latest_version: None,
                                    is_dev: false,
                                    is_outdated: false,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(packages)
}

fn check_yarn_outdated(cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    let output = run_command("yarn", &["outdated", "--json"], cwd);
    let json_str = match output {
        Ok(s) => s,
        Err(s) => s,
    };

    let mut packages = Vec::new();
    for line in json_str.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if json.get("type").and_then(|t| t.as_str()) == Some("table") {
                if let Some(data) = json
                    .get("data")
                    .and_then(|d| d.get("body"))
                    .and_then(|b| b.as_array())
                {
                    for row in data {
                        if let Some(arr) = row.as_array() {
                            if arr.len() >= 4 {
                                let name = arr[0].as_str().unwrap_or_default();
                                let current = arr[1].as_str();
                                let latest = arr[3].as_str();
                                packages.push(PackageEntry {
                                    name: name.to_string(),
                                    current_version: current.map(String::from),
                                    latest_version: latest.map(String::from),
                                    is_dev: false,
                                    is_outdated: true,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(packages)
}

// === pip ===

fn list_pip_packages(_cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    let output = run_command(
        "pip",
        &["list", "--format=json"],
        &std::env::current_dir().unwrap(),
    )?;

    let mut packages = Vec::new();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        if let Some(arr) = json.as_array() {
            for item in arr {
                let name = item
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_default();
                let version = item.get("version").and_then(|v| v.as_str());
                packages.push(PackageEntry {
                    name: name.to_string(),
                    current_version: version.map(String::from),
                    latest_version: None,
                    is_dev: false,
                    is_outdated: false,
                });
            }
        }
    }

    Ok(packages)
}

fn check_pip_outdated(_cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    let output = run_command(
        "pip",
        &["list", "--outdated", "--format=json"],
        &std::env::current_dir().unwrap(),
    )?;

    let mut packages = Vec::new();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        if let Some(arr) = json.as_array() {
            for item in arr {
                let name = item
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_default();
                let version = item.get("version").and_then(|v| v.as_str());
                let latest = item.get("latest_version").and_then(|v| v.as_str());
                packages.push(PackageEntry {
                    name: name.to_string(),
                    current_version: version.map(String::from),
                    latest_version: latest.map(String::from),
                    is_dev: false,
                    is_outdated: true,
                });
            }
        }
    }

    Ok(packages)
}

// === uv ===

fn list_uv_packages(_cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    let output = run_command(
        "uv",
        &["pip", "list", "--format=json"],
        &std::env::current_dir().unwrap(),
    )?;

    let mut packages = Vec::new();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        if let Some(arr) = json.as_array() {
            for item in arr {
                let name = item
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_default();
                let version = item.get("version").and_then(|v| v.as_str());
                packages.push(PackageEntry {
                    name: name.to_string(),
                    current_version: version.map(String::from),
                    latest_version: None,
                    is_dev: false,
                    is_outdated: false,
                });
            }
        }
    }

    Ok(packages)
}

fn check_uv_outdated(_cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    // uv doesn't have a built-in outdated command yet
    Err("uv doesn't support checking outdated packages yet".to_string())
}

// === Poetry ===

fn list_poetry_packages(cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    let output = run_command("poetry", &["show", "--no-ansi"], cwd)?;

    let mut packages = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            packages.push(PackageEntry {
                name: parts[0].to_string(),
                current_version: Some(parts[1].to_string()),
                latest_version: None,
                is_dev: false,
                is_outdated: false,
            });
        }
    }

    Ok(packages)
}

fn check_poetry_outdated(cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    let output = run_command("poetry", &["show", "--outdated", "--no-ansi"], cwd)?;

    let mut packages = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            packages.push(PackageEntry {
                name: parts[0].to_string(),
                current_version: Some(parts[1].to_string()),
                latest_version: Some(parts[2].to_string()),
                is_dev: false,
                is_outdated: true,
            });
        }
    }

    Ok(packages)
}

// === Go ===

fn list_go_packages(cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    let output = run_command("go", &["list", "-m", "all"], cwd)?;

    let mut packages = Vec::new();
    for line in output.lines().skip(1) {
        // Skip first line (main module)
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            packages.push(PackageEntry {
                name: parts[0].to_string(),
                current_version: Some(parts[1].to_string()),
                latest_version: None,
                is_dev: false,
                is_outdated: false,
            });
        }
    }

    Ok(packages)
}

fn check_go_outdated(cwd: &Path) -> Result<Vec<PackageEntry>, String> {
    let output = run_command("go", &["list", "-m", "-u", "all"], cwd)?;

    let mut packages = Vec::new();
    for line in output.lines().skip(1) {
        // Skip first line (main module)
        // Format: module version [new_version]
        if line.contains('[') {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let latest = parts[2].trim_start_matches('[').trim_end_matches(']');
                packages.push(PackageEntry {
                    name: parts[0].to_string(),
                    current_version: Some(parts[1].to_string()),
                    latest_version: Some(latest.to_string()),
                    is_dev: false,
                    is_outdated: true,
                });
            }
        }
    }

    Ok(packages)
}
