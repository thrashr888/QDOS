//! Q-PAINT File I/O
//!
//! Load and save images in PNG and BMP formats.

use crate::state::{Canvas, QPaintState};
use image::{ImageBuffer, Rgb, RgbImage};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Load an image file into the canvas
pub fn load_image(state: &mut QPaintState, path: &Path) -> Result<(), String> {
    let img = image::open(path).map_err(|e| format!("Failed to load image: {}", e))?;

    let rgb = img.to_rgb8();
    let (width, height) = rgb.dimensions();

    state.canvas = Canvas::new(width, height);

    for (x, y, pixel) in rgb.enumerate_pixels() {
        state.canvas.set_pixel(x, y, (pixel[0], pixel[1], pixel[2]));
    }

    state.file_path = Some(path.to_path_buf());
    state.modified = false;
    state.cursor_x = width / 2;
    state.cursor_y = height / 2;
    state.scroll_x = 0;
    state.scroll_y = 0;
    state.undo_stack.clear();
    state.redo_stack.clear();

    Ok(())
}

/// Save canvas to an image file
pub fn save_image(state: &QPaintState, path: &Path) -> Result<(), String> {
    let canvas = &state.canvas;
    let mut img: RgbImage = ImageBuffer::new(canvas.width, canvas.height);

    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let (r, g, b) = canvas.get_pixel(x, y);
            img.put_pixel(x, y, Rgb([r, g, b]));
        }
    }

    img.save(path)
        .map_err(|e| format!("Failed to save image: {}", e))
}

/// Export canvas as ANSI art
pub fn export_ansi(state: &QPaintState, path: &Path) -> Result<(), String> {
    let canvas = &state.canvas;
    let mut output = String::new();

    // Use half-block characters for better resolution
    for y in (0..canvas.height).step_by(2) {
        for x in 0..canvas.width {
            let top = canvas.get_pixel(x, y);
            let bottom = if y + 1 < canvas.height {
                canvas.get_pixel(x, y + 1)
            } else {
                (0, 0, 0)
            };

            // Use upper half block with foreground (top) and background (bottom)
            output.push_str(&format!(
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m\u{2580}",
                top.0, top.1, top.2, bottom.0, bottom.1, bottom.2
            ));
        }
        output.push_str("\x1b[0m\n");
    }

    let mut file = fs::File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;
    file.write_all(output.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))
}

/// List image files in a directory
pub fn list_images(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if matches!(ext.as_str(), "png" | "bmp" | "jpg" | "jpeg" | "gif") {
                        if let Some(name) = path.file_name() {
                            files.push(name.to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }
    }

    files.sort();
    files
}

/// Get config file path
fn config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("rdos")
        .join("qpaint.conf")
}

/// Config file structure
#[derive(Debug, Default)]
pub struct QPaintConfig {
    pub last_dir: Option<String>,
    pub last_fg_color: Option<(u8, u8, u8)>,
    pub last_bg_color: Option<(u8, u8, u8)>,
    pub default_width: u32,
    pub default_height: u32,
}

impl QPaintConfig {
    /// Load config from file
    pub fn load() -> Self {
        let config_path = config_path();
        if !config_path.exists() {
            return Self::default();
        }

        let Ok(content) = fs::read_to_string(&config_path) else {
            return Self::default();
        };

        let mut config = Self::default();

        for line in content.lines() {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }
            let key = parts[0].trim();
            let value = parts[1].trim();

            match key {
                "last_dir" => config.last_dir = Some(value.to_string()),
                "fg_color" => {
                    let rgb: Vec<u8> = value
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                    if rgb.len() == 3 {
                        config.last_fg_color = Some((rgb[0], rgb[1], rgb[2]));
                    }
                }
                "bg_color" => {
                    let rgb: Vec<u8> = value
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                    if rgb.len() == 3 {
                        config.last_bg_color = Some((rgb[0], rgb[1], rgb[2]));
                    }
                }
                "width" => config.default_width = value.parse().unwrap_or(32),
                "height" => config.default_height = value.parse().unwrap_or(32),
                _ => {}
            }
        }

        config
    }

    /// Save config to file
    pub fn save(&self) -> Result<(), String> {
        let config_path = config_path();

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }

        let mut file = fs::File::create(&config_path)
            .map_err(|e| format!("Failed to create config: {}", e))?;

        if let Some(dir) = &self.last_dir {
            writeln!(file, "last_dir={}", dir)
                .map_err(|e| format!("Failed to write config: {}", e))?;
        }
        if let Some((r, g, b)) = self.last_fg_color {
            writeln!(file, "fg_color={},{},{}", r, g, b)
                .map_err(|e| format!("Failed to write config: {}", e))?;
        }
        if let Some((r, g, b)) = self.last_bg_color {
            writeln!(file, "bg_color={},{},{}", r, g, b)
                .map_err(|e| format!("Failed to write config: {}", e))?;
        }
        if self.default_width > 0 {
            writeln!(file, "width={}", self.default_width)
                .map_err(|e| format!("Failed to write config: {}", e))?;
        }
        if self.default_height > 0 {
            writeln!(file, "height={}", self.default_height)
                .map_err(|e| format!("Failed to write config: {}", e))?;
        }

        Ok(())
    }
}

/// Apply config to state
pub fn apply_config(state: &mut QPaintState) {
    let config = QPaintConfig::load();

    if let Some((r, g, b)) = config.last_fg_color {
        state.fg_color = (r, g, b);
    }
    if let Some((r, g, b)) = config.last_bg_color {
        state.bg_color = (r, g, b);
    }
}

/// Save current settings to config
pub fn save_config(state: &QPaintState) -> Result<(), String> {
    let config = QPaintConfig {
        last_dir: state
            .file_path
            .as_ref()
            .and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned())),
        last_fg_color: Some(state.fg_color),
        last_bg_color: Some(state.bg_color),
        default_width: state.canvas.width,
        default_height: state.canvas.height,
    };
    config.save()
}
