//! SoundFont management for software MIDI playback
//!
//! Handles detection and downloading of SoundFont files needed for FluidSynth.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Default soundfont to download if none found
/// GeneralUser GS is a good quality General MIDI soundfont (~30MB)
const SOUNDFONT_URL: &str =
    "https://archive.org/download/free-soundfonts-sf2-2019-04/GeneralUser%20GS%20v1.471.sf2";
const SOUNDFONT_FILENAME: &str = "GeneralUser_GS.sf2";

/// Get the RDOS data directory for soundfonts
pub fn soundfont_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rdos")
        .join("soundfonts")
}

/// Get the path to the default RDOS soundfont
pub fn default_soundfont_path() -> PathBuf {
    soundfont_dir().join(SOUNDFONT_FILENAME)
}

/// Find an existing soundfont on the system
pub fn find_soundfont() -> Option<PathBuf> {
    // Check RDOS soundfont directory first
    let rdos_sf = default_soundfont_path();
    if rdos_sf.exists() {
        return Some(rdos_sf);
    }

    // Common soundfont locations
    let search_paths = [
        // macOS Homebrew (Apple Silicon)
        "/opt/homebrew/share/soundfonts",
        "/opt/homebrew/Cellar/fluid-synth/2.5.2/share/fluid-synth/sf2",
        // macOS Homebrew (Intel)
        "/usr/local/share/soundfonts",
        "/usr/local/Cellar/fluid-synth",
        // macOS system
        "/Library/Audio/Sounds/Banks",
        // Linux common locations
        "/usr/share/soundfonts",
        "/usr/share/sounds/sf2",
        "/usr/local/share/soundfonts",
    ];

    // Preferred soundfont names (larger/better quality first)
    let preferred_names = [
        "GeneralUser",
        "FluidR3_GM",
        "FluidR3",
        "MuseScore_General",
        "TimGM6mb",
        "default",
    ];

    // Search for soundfonts
    for path in &search_paths {
        let dir = PathBuf::from(path);
        if dir.exists() {
            if let Ok(entries) = fs::read_dir(&dir) {
                let mut found_files: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.extension()
                            .map(|ext| ext == "sf2" || ext == "sf3")
                            .unwrap_or(false)
                    })
                    .collect();

                // Sort by preference (preferred names first, then by size descending)
                found_files.sort_by(|a, b| {
                    let a_name = a.file_stem().unwrap_or_default().to_string_lossy();
                    let b_name = b.file_stem().unwrap_or_default().to_string_lossy();

                    let a_pref = preferred_names
                        .iter()
                        .position(|&n| a_name.contains(n))
                        .unwrap_or(usize::MAX);
                    let b_pref = preferred_names
                        .iter()
                        .position(|&n| b_name.contains(n))
                        .unwrap_or(usize::MAX);

                    if a_pref != b_pref {
                        return a_pref.cmp(&b_pref);
                    }

                    // Fall back to file size (larger = more complete)
                    let a_size = fs::metadata(a).map(|m| m.len()).unwrap_or(0);
                    let b_size = fs::metadata(b).map(|m| m.len()).unwrap_or(0);
                    b_size.cmp(&a_size)
                });

                // Return best soundfont that's at least 1MB (skip tiny demo fonts)
                for sf in found_files {
                    if let Ok(meta) = fs::metadata(&sf) {
                        if meta.len() > 1_000_000 {
                            return Some(sf);
                        }
                    }
                }
            }
        }
    }

    // Also check user's home directory
    if let Some(home) = dirs::home_dir() {
        let user_soundfonts = [
            home.join("Library/Audio/Sounds/Banks"),
            home.join(".local/share/soundfonts"),
            home.join(".soundfonts"),
        ];

        for dir in &user_soundfonts {
            if dir.exists() {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path
                            .extension()
                            .map(|ext| ext == "sf2" || ext == "sf3")
                            .unwrap_or(false)
                        {
                            if let Ok(meta) = fs::metadata(&path) {
                                if meta.len() > 1_000_000 {
                                    return Some(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Download a soundfont to the RDOS data directory
pub fn download_soundfont() -> Result<PathBuf, String> {
    let sf_dir = soundfont_dir();
    let sf_path = default_soundfont_path();

    // Create directory if needed
    fs::create_dir_all(&sf_dir)
        .map_err(|e| format!("Failed to create soundfont directory: {}", e))?;

    // Download using curl
    let status = Command::new("curl")
        .arg("-L")
        .arg("-o")
        .arg(&sf_path)
        .arg("--progress-bar")
        .arg(SOUNDFONT_URL)
        .status()
        .map_err(|e| format!("Failed to run curl: {}", e))?;

    if !status.success() {
        return Err("Download failed".to_string());
    }

    // Verify the download (should be at least 10MB for GeneralUser GS)
    let meta = fs::metadata(&sf_path).map_err(|e| format!("Failed to verify download: {}", e))?;

    if meta.len() < 10_000_000 {
        fs::remove_file(&sf_path).ok();
        return Err("Downloaded file too small, may be corrupted".to_string());
    }

    Ok(sf_path)
}

/// Get a soundfont path, downloading if necessary
pub fn get_soundfont() -> Option<PathBuf> {
    find_soundfont()
}

/// Check if we need to download a soundfont
pub fn needs_download() -> bool {
    find_soundfont().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soundfont_dir() {
        let dir = soundfont_dir();
        assert!(dir.to_string_lossy().contains("rdos"));
    }

    #[test]
    fn test_find_soundfont() {
        // This test is environment-dependent
        let _ = find_soundfont();
    }
}
