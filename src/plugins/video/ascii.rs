//! ASCII art rendering for video frames
//!
//! Converts raw RGB video frames to ASCII art for terminal display.

/// Character set for ASCII art (darkest to brightest)
const ASCII_CHARS: &[char] = &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

/// A colored ASCII character with RGB color tuple
pub type ColoredAsciiLine = Vec<(char, (u8, u8, u8))>;

/// Convert RGB frame data to ASCII art lines
pub fn frame_to_ascii(
    rgb_data: &[u8],
    width: u32,
    height: u32,
    target_width: u16,
    target_height: u16,
) -> Vec<String> {
    if rgb_data.is_empty() || width == 0 || height == 0 {
        return vec!["[No frame data]".to_string()];
    }

    let mut lines = Vec::with_capacity(target_height as usize);

    // Calculate sampling step
    let x_step = width as f32 / target_width as f32;
    let y_step = height as f32 / target_height as f32;

    for ty in 0..target_height {
        let mut line = String::with_capacity(target_width as usize);
        let src_y = ((ty as f32 * y_step) as u32).min(height - 1);

        for tx in 0..target_width {
            let src_x = ((tx as f32 * x_step) as u32).min(width - 1);

            // Get pixel from RGB data (3 bytes per pixel)
            let idx = ((src_y * width + src_x) * 3) as usize;
            if idx + 2 < rgb_data.len() {
                let r = rgb_data[idx] as f32;
                let g = rgb_data[idx + 1] as f32;
                let b = rgb_data[idx + 2] as f32;

                // Calculate luminance (perceived brightness)
                let luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;

                // Map to ASCII character
                let char_idx = (luminance * (ASCII_CHARS.len() - 1) as f32) as usize;
                line.push(ASCII_CHARS[char_idx.min(ASCII_CHARS.len() - 1)]);
            } else {
                line.push(' ');
            }
        }
        lines.push(line);
    }

    lines
}

/// Convert RGB frame to colored ASCII using ANSI colors
/// Returns (character, RGB color) pairs for ratatui rendering
pub fn frame_to_colored_ascii(
    rgb_data: &[u8],
    width: u32,
    height: u32,
    target_width: u16,
    target_height: u16,
) -> Vec<ColoredAsciiLine> {
    if rgb_data.is_empty() || width == 0 || height == 0 {
        return vec![];
    }

    let mut lines = Vec::with_capacity(target_height as usize);

    let x_step = width as f32 / target_width as f32;
    let y_step = height as f32 / target_height as f32;

    for ty in 0..target_height {
        let mut line = Vec::with_capacity(target_width as usize);
        let src_y = ((ty as f32 * y_step) as u32).min(height - 1);

        for tx in 0..target_width {
            let src_x = ((tx as f32 * x_step) as u32).min(width - 1);

            let idx = ((src_y * width + src_x) * 3) as usize;
            if idx + 2 < rgb_data.len() {
                let r = rgb_data[idx];
                let g = rgb_data[idx + 1];
                let b = rgb_data[idx + 2];

                let luminance = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0;
                let char_idx = (luminance * (ASCII_CHARS.len() - 1) as f32) as usize;
                let ch = ASCII_CHARS[char_idx.min(ASCII_CHARS.len() - 1)];

                line.push((ch, (r, g, b)));
            } else {
                line.push((' ', (0, 0, 0)));
            }
        }
        lines.push(line);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_to_ascii_empty() {
        let result = frame_to_ascii(&[], 0, 0, 10, 10);
        assert_eq!(result, vec!["[No frame data]"]);
    }

    #[test]
    fn test_frame_to_ascii_white() {
        // 2x2 white image
        let data = vec![255u8; 12]; // 4 pixels * 3 RGB bytes
        let result = frame_to_ascii(&data, 2, 2, 4, 2);
        assert_eq!(result.len(), 2);
        // White should map to '@' (brightest)
        assert!(result[0].contains('@'));
    }

    #[test]
    fn test_frame_to_ascii_black() {
        // 2x2 black image
        let data = vec![0u8; 12];
        let result = frame_to_ascii(&data, 2, 2, 4, 2);
        assert_eq!(result.len(), 2);
        // Black should map to ' ' (darkest)
        assert!(result[0].contains(' '));
    }
}
