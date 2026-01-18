//! Q-DECK Image Support
//!
//! Handles loading and rendering images via sixel/kitty protocols.

use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

// =============================================================================
// IMAGE PICKER
// =============================================================================

/// Lazy-loaded image picker (detects Kitty/Sixel/iTerm2 protocols)
static IMAGE_PICKER: OnceLock<Mutex<Option<Picker>>> = OnceLock::new();

/// Get or initialize the image picker with terminal protocol detection
fn get_image_picker() -> &'static Mutex<Option<Picker>> {
    IMAGE_PICKER.get_or_init(|| {
        let picker = Picker::from_query_stdio().ok();
        Mutex::new(picker)
    })
}

// =============================================================================
// IMAGE LOADING
// =============================================================================

/// Load an image from a file path and create a protocol
fn load_image_protocol(path: &str, base_dir: &Path) -> Option<StatefulProtocol> {
    // Resolve path
    let full_path = if path.starts_with('/') {
        std::path::PathBuf::from(path)
    } else if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(rest)
        } else {
            base_dir.join(path)
        }
    } else {
        base_dir.join(path)
    };

    // Try to load the image
    let content = std::fs::read(&full_path).ok()?;
    let dyn_img = image::load_from_memory(&content).ok()?;

    // Create protocol from picker
    if let Ok(mut guard) = get_image_picker().lock() {
        if let Some(ref mut picker) = *guard {
            return Some(picker.new_resize_protocol(dyn_img));
        }
    }
    None
}

/// Render an image in the given area
///
/// Returns true if the image was rendered successfully
pub fn render_image(frame: &mut Frame, area: Rect, path: &str, base_dir: &Path) -> bool {
    if let Some(mut protocol) = load_image_protocol(path, base_dir) {
        let image_widget = StatefulImage::new(None);
        frame.render_stateful_widget(image_widget, area, &mut protocol);
        true
    } else {
        false
    }
}

/// Check if image rendering is available (terminal supports graphics protocol)
pub fn is_image_rendering_available() -> bool {
    if let Ok(guard) = get_image_picker().lock() {
        guard.is_some()
    } else {
        false
    }
}
