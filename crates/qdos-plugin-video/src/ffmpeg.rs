//! FFmpeg detection and management utilities
//!
//! Checks for ffmpeg availability and provides install hints.

use std::process::Command;

/// Check if ffmpeg is available in PATH
pub fn is_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get ffmpeg version string
pub fn get_version() -> Option<String> {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("unknown")
                .to_string()
        })
}

/// Get install hint for the current platform
pub fn get_install_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "brew install ffmpeg"
    } else if cfg!(target_os = "linux") {
        "apt install ffmpeg  (or your distro's package manager)"
    } else if cfg!(target_os = "windows") {
        "winget install ffmpeg  (or download from ffmpeg.org)"
    } else {
        "Install ffmpeg from ffmpeg.org"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_hint() {
        let hint = get_install_hint();
        assert!(!hint.is_empty());
    }
}
