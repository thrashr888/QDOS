//! Package manager detection

use super::state::PackageManager;
use std::path::Path;

/// Detect the package manager for a directory
pub fn detect_package_manager(cwd: &Path) -> Option<PackageManager> {
    // Check in priority order

    // Rust - Cargo
    if cwd.join("Cargo.toml").exists() {
        return Some(PackageManager::Cargo);
    }

    // Go
    if cwd.join("go.mod").exists() {
        return Some(PackageManager::GoMod);
    }

    // Python - check for uv first (most modern)
    if cwd.join("uv.lock").exists() || has_uv_config(cwd) {
        return Some(PackageManager::Uv);
    }
    if cwd.join("poetry.lock").exists() || has_poetry_config(cwd) {
        return Some(PackageManager::Poetry);
    }
    if cwd.join("requirements.txt").exists() {
        return Some(PackageManager::Pip);
    }

    // Node.js - check lockfiles first
    if cwd.join("pnpm-lock.yaml").exists() {
        return Some(PackageManager::Pnpm);
    }
    if cwd.join("yarn.lock").exists() {
        return Some(PackageManager::Yarn);
    }
    if cwd.join("package.json").exists() || cwd.join("package-lock.json").exists() {
        return Some(PackageManager::Npm);
    }

    None
}

/// Check for [tool.uv] in pyproject.toml
fn has_uv_config(cwd: &Path) -> bool {
    let pyproject = cwd.join("pyproject.toml");
    if pyproject.exists() {
        if let Ok(content) = std::fs::read_to_string(&pyproject) {
            return content.contains("[tool.uv]");
        }
    }
    false
}

/// Check for [tool.poetry] in pyproject.toml
fn has_poetry_config(cwd: &Path) -> bool {
    let pyproject = cwd.join("pyproject.toml");
    if pyproject.exists() {
        if let Ok(content) = std::fs::read_to_string(&pyproject) {
            return content.contains("[tool.poetry]");
        }
    }
    false
}

/// Get the project name from package manager files
pub fn get_project_name(pm: PackageManager, cwd: &Path) -> Option<String> {
    match pm {
        PackageManager::Cargo => {
            let cargo_toml = cwd.join("Cargo.toml");
            if let Ok(content) = std::fs::read_to_string(cargo_toml) {
                // Simple parsing - look for name = "..."
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("name") {
                        if let Some(name) = extract_toml_string(line) {
                            return Some(name);
                        }
                    }
                }
            }
        }
        PackageManager::Npm | PackageManager::Pnpm | PackageManager::Yarn => {
            let package_json = cwd.join("package.json");
            if let Ok(content) = std::fs::read_to_string(package_json) {
                // Simple JSON parsing - look for "name": "..."
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                        return Some(name.to_string());
                    }
                }
            }
        }
        PackageManager::GoMod => {
            let go_mod = cwd.join("go.mod");
            if let Ok(content) = std::fs::read_to_string(go_mod) {
                // First line: module <name>
                if let Some(first_line) = content.lines().next() {
                    if let Some(name) = first_line.strip_prefix("module ") {
                        return Some(name.trim().to_string());
                    }
                }
            }
        }
        PackageManager::Pip => {
            // No standard project name for pip
            return None;
        }
        PackageManager::Uv | PackageManager::Poetry => {
            let pyproject = cwd.join("pyproject.toml");
            if let Ok(content) = std::fs::read_to_string(pyproject) {
                // Look for name under [project] or [tool.poetry]
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("name") {
                        if let Some(name) = extract_toml_string(line) {
                            return Some(name);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Extract string value from TOML line like: name = "value"
fn extract_toml_string(line: &str) -> Option<String> {
    if let Some(eq_pos) = line.find('=') {
        let value_part = line[eq_pos + 1..].trim();
        if value_part.starts_with('"') && value_part.ends_with('"') && value_part.len() > 2 {
            return Some(value_part[1..value_part.len() - 1].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_detect_cargo() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        assert_eq!(
            detect_package_manager(dir.path()),
            Some(PackageManager::Cargo)
        );
    }

    #[test]
    fn test_detect_npm() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert_eq!(
            detect_package_manager(dir.path()),
            Some(PackageManager::Npm)
        );
    }

    #[test]
    fn test_detect_go() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("go.mod"), "module test").unwrap();
        assert_eq!(
            detect_package_manager(dir.path()),
            Some(PackageManager::GoMod)
        );
    }

    #[test]
    fn test_detect_none() {
        let dir = tempdir().unwrap();
        assert_eq!(detect_package_manager(dir.path()), None);
    }

    #[test]
    fn test_extract_toml_string() {
        assert_eq!(
            extract_toml_string("name = \"myapp\""),
            Some("myapp".to_string())
        );
        assert_eq!(
            extract_toml_string("version = \"1.0.0\""),
            Some("1.0.0".to_string())
        );
        assert_eq!(extract_toml_string("invalid"), None);
    }
}
