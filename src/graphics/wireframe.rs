//! Retro wireframe 3D rendering
//!
//! Provides Star Wars / Battlezone style vector graphics rendering.
//! Renders 3D wireframe models to pixel buffers with phosphor CRT effects.

#![allow(dead_code)]

/// Phosphor green colors (mimics CRT persistence)
pub const PHOSPHOR_GREEN: (u8, u8, u8) = (0, 255, 0); // Bright line
pub const PHOSPHOR_DIM: (u8, u8, u8) = (0, 128, 0); // Glow
pub const PHOSPHOR_DARK: (u8, u8, u8) = (0, 64, 0); // Fade

/// Wireframe 3D model for retro rendering
#[derive(Debug, Clone, Default)]
pub struct WireframeModel {
    /// 3D vertex positions (x, y, z)
    pub vertices: Vec<(f32, f32, f32)>,
    /// Edge connections (vertex index pairs)
    pub edges: Vec<(usize, usize)>,
}

impl WireframeModel {
    /// Create a new empty wireframe model
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a unit cube centered at origin
    pub fn cube() -> Self {
        Self {
            vertices: vec![
                (-1.0, -1.0, -1.0), // 0: back-bottom-left
                (1.0, -1.0, -1.0),  // 1: back-bottom-right
                (1.0, 1.0, -1.0),   // 2: back-top-right
                (-1.0, 1.0, -1.0),  // 3: back-top-left
                (-1.0, -1.0, 1.0),  // 4: front-bottom-left
                (1.0, -1.0, 1.0),   // 5: front-bottom-right
                (1.0, 1.0, 1.0),    // 6: front-top-right
                (-1.0, 1.0, 1.0),   // 7: front-top-left
            ],
            edges: vec![
                // Back face
                (0, 1),
                (1, 2),
                (2, 3),
                (3, 0),
                // Front face
                (4, 5),
                (5, 6),
                (6, 7),
                (7, 4),
                // Connecting edges
                (0, 4),
                (1, 5),
                (2, 6),
                (3, 7),
            ],
        }
    }

    /// Create a pyramid (tetrahedron-like)
    pub fn pyramid() -> Self {
        Self {
            vertices: vec![
                (0.0, 1.0, 0.0),    // 0: top
                (-1.0, -1.0, -1.0), // 1: back-left
                (1.0, -1.0, -1.0),  // 2: back-right
                (1.0, -1.0, 1.0),   // 3: front-right
                (-1.0, -1.0, 1.0),  // 4: front-left
            ],
            edges: vec![
                // Base
                (1, 2),
                (2, 3),
                (3, 4),
                (4, 1),
                // Sides to apex
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
            ],
        }
    }

    /// Create a simple spaceship shape (Star Wars style)
    pub fn spaceship() -> Self {
        Self {
            vertices: vec![
                // Nose
                (0.0, 0.0, 2.0), // 0: nose tip
                // Main body
                (-0.5, 0.3, 0.5),  // 1: top-left
                (0.5, 0.3, 0.5),   // 2: top-right
                (0.5, -0.3, 0.5),  // 3: bottom-right
                (-0.5, -0.3, 0.5), // 4: bottom-left
                // Rear
                (-0.8, 0.4, -1.5),  // 5: rear top-left
                (0.8, 0.4, -1.5),   // 6: rear top-right
                (0.8, -0.4, -1.5),  // 7: rear bottom-right
                (-0.8, -0.4, -1.5), // 8: rear bottom-left
                // Wings
                (-2.0, 0.0, -0.5), // 9: left wing tip
                (2.0, 0.0, -0.5),  // 10: right wing tip
            ],
            edges: vec![
                // Nose to body
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                // Body front
                (1, 2),
                (2, 3),
                (3, 4),
                (4, 1),
                // Body to rear
                (1, 5),
                (2, 6),
                (3, 7),
                (4, 8),
                // Rear
                (5, 6),
                (6, 7),
                (7, 8),
                (8, 5),
                // Wings
                (4, 9),
                (5, 9),
                (8, 9),
                (3, 10),
                (6, 10),
                (7, 10),
            ],
        }
    }

    /// Create a simple tank shape (Battlezone style)
    pub fn tank() -> Self {
        Self {
            vertices: vec![
                // Body (lower)
                (-1.0, -0.3, -1.5), // 0
                (1.0, -0.3, -1.5),  // 1
                (1.0, -0.3, 1.5),   // 2
                (-1.0, -0.3, 1.5),  // 3
                // Body (upper)
                (-1.0, 0.2, -1.5), // 4
                (1.0, 0.2, -1.5),  // 5
                (1.0, 0.2, 1.5),   // 6
                (-1.0, 0.2, 1.5),  // 7
                // Turret
                (-0.5, 0.2, -0.3), // 8
                (0.5, 0.2, -0.3),  // 9
                (0.5, 0.5, -0.3),  // 10
                (-0.5, 0.5, -0.3), // 11
                (-0.5, 0.5, 0.3),  // 12
                (0.5, 0.5, 0.3),   // 13
                // Cannon
                (0.0, 0.35, 0.3), // 14: cannon base
                (0.0, 0.35, 1.8), // 15: cannon tip
            ],
            edges: vec![
                // Body bottom
                (0, 1),
                (1, 2),
                (2, 3),
                (3, 0),
                // Body top
                (4, 5),
                (5, 6),
                (6, 7),
                (7, 4),
                // Body sides
                (0, 4),
                (1, 5),
                (2, 6),
                (3, 7),
                // Turret
                (8, 9),
                (9, 10),
                (10, 11),
                (11, 8),
                (11, 12),
                (10, 13),
                (12, 13),
                // Cannon
                (14, 15),
            ],
        }
    }

    /// Scale the model uniformly
    pub fn scale(&mut self, factor: f32) {
        for v in &mut self.vertices {
            v.0 *= factor;
            v.1 *= factor;
            v.2 *= factor;
        }
    }

    /// Translate the model
    pub fn translate(&mut self, dx: f32, dy: f32, dz: f32) {
        for v in &mut self.vertices {
            v.0 += dx;
            v.1 += dy;
            v.2 += dz;
        }
    }
}

/// Project a 3D vertex to 2D screen coordinates with rotation
fn project_vertex(
    v: (f32, f32, f32),
    rotation: (f32, f32, f32),
    width: u32,
    height: u32,
) -> (i32, i32) {
    let (rx, ry, _rz) = rotation;
    let (x, y, z) = v;

    // Rotate around X axis
    let cos_x = rx.cos();
    let sin_x = rx.sin();
    let y1 = y * cos_x - z * sin_x;
    let z1 = y * sin_x + z * cos_x;

    // Rotate around Y axis
    let cos_y = ry.cos();
    let sin_y = ry.sin();
    let x1 = x * cos_y + z1 * sin_y;
    let z2 = -x * sin_y + z1 * cos_y;

    // Perspective projection
    let fov = 200.0;
    let distance = 400.0;
    let scale = fov / (z2 + distance);

    let px = (x1 * scale + width as f32 / 2.0) as i32;
    let py = (y1 * scale + height as f32 / 2.0) as i32;

    (px, py)
}

/// Draw a line using Bresenham's algorithm
fn draw_line(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: (u8, u8, u8),
    alpha: u8,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        // Plot pixel if within bounds
        if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
            let idx = ((y as u32 * width + x as u32) * 4) as usize;
            if idx + 3 < pixels.len() {
                pixels[idx] = color.0;
                pixels[idx + 1] = color.1;
                pixels[idx + 2] = color.2;
                pixels[idx + 3] = alpha;
            }
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            if x == x1 {
                break;
            }
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            if y == y1 {
                break;
            }
            err += dx;
            y += sy;
        }
    }
}

/// Draw a line with anti-aliased glow effect
fn draw_line_with_glow(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: (u8, u8, u8),
    glow_color: (u8, u8, u8),
    glow_radius: i32,
) {
    // Draw glow (wider, dimmer)
    for offset in -glow_radius..=glow_radius {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();

        if dx > dy {
            draw_line(
                pixels,
                width,
                height,
                x0,
                y0 + offset,
                x1,
                y1 + offset,
                glow_color,
                128,
            );
        } else {
            draw_line(
                pixels,
                width,
                height,
                x0 + offset,
                y0,
                x1 + offset,
                y1,
                glow_color,
                128,
            );
        }
    }

    // Draw core line (bright)
    draw_line(pixels, width, height, x0, y0, x1, y1, color, 255);
}

/// Render a wireframe model to a pixel buffer with phosphor effect
///
/// # Arguments
/// * `model` - The wireframe model to render
/// * `width` - Output buffer width in pixels
/// * `height` - Output buffer height in pixels
/// * `rotation` - Rotation angles (rx, ry, rz) in radians
///
/// # Returns
/// RGBA pixel buffer (width * height * 4 bytes)
pub fn render_wireframe(
    model: &WireframeModel,
    width: u32,
    height: u32,
    rotation: (f32, f32, f32),
) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    // Project all vertices to 2D
    let projected: Vec<(i32, i32)> = model
        .vertices
        .iter()
        .map(|v| project_vertex(*v, rotation, width, height))
        .collect();

    // Draw all edges with phosphor glow
    for &(a, b) in &model.edges {
        if a < projected.len() && b < projected.len() {
            let (x0, y0) = projected[a];
            let (x1, y1) = projected[b];

            draw_line_with_glow(
                &mut pixels,
                width,
                height,
                x0,
                y0,
                x1,
                y1,
                PHOSPHOR_GREEN,
                PHOSPHOR_DIM,
                1,
            );
        }
    }

    pixels
}

/// Render wireframe with custom colors (for different CRT phosphors)
pub fn render_wireframe_colored(
    model: &WireframeModel,
    width: u32,
    height: u32,
    rotation: (f32, f32, f32),
    line_color: (u8, u8, u8),
    glow_color: (u8, u8, u8),
) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    let projected: Vec<(i32, i32)> = model
        .vertices
        .iter()
        .map(|v| project_vertex(*v, rotation, width, height))
        .collect();

    for &(a, b) in &model.edges {
        if a < projected.len() && b < projected.len() {
            let (x0, y0) = projected[a];
            let (x1, y1) = projected[b];

            draw_line_with_glow(
                &mut pixels,
                width,
                height,
                x0,
                y0,
                x1,
                y1,
                line_color,
                glow_color,
                1,
            );
        }
    }

    pixels
}

/// Apply CRT scanline effect to pixel buffer
pub fn apply_scanlines(pixels: &mut [u8], width: u32, height: u32, intensity: f32) {
    for y in 0..height {
        // Darken every other line
        if y % 2 == 1 {
            let row_start = (y * width * 4) as usize;
            for x in 0..width {
                let idx = row_start + (x * 4) as usize;
                if idx + 2 < pixels.len() {
                    pixels[idx] = (pixels[idx] as f32 * intensity) as u8;
                    pixels[idx + 1] = (pixels[idx + 1] as f32 * intensity) as u8;
                    pixels[idx + 2] = (pixels[idx + 2] as f32 * intensity) as u8;
                }
            }
        }
    }
}

/// Convert RGBA pixels to ratatui Spans for ASCII rendering
/// This is a fallback for terminals without sixel support
pub fn wireframe_to_ascii(
    model: &WireframeModel,
    width: u32,
    height: u32,
    rotation: (f32, f32, f32),
) -> Vec<String> {
    // Create a character buffer
    let char_width = width as usize;
    let char_height = height as usize;
    let mut buffer = vec![vec![' '; char_width]; char_height];

    // Project vertices
    let projected: Vec<(i32, i32)> = model
        .vertices
        .iter()
        .map(|v| project_vertex(*v, rotation, width, height))
        .collect();

    // Draw edges as ASCII
    for &(a, b) in &model.edges {
        if a < projected.len() && b < projected.len() {
            let (x0, y0) = projected[a];
            let (x1, y1) = projected[b];

            // Simple ASCII line drawing
            draw_ascii_line(&mut buffer, char_width, char_height, x0, y0, x1, y1);
        }
    }

    // Convert to strings
    buffer
        .into_iter()
        .map(|row| row.into_iter().collect())
        .collect()
}

/// Draw an ASCII line using simple characters
fn draw_ascii_line(
    buffer: &mut [Vec<char>],
    width: usize,
    height: usize,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
            let c = if dx > dy * 2 {
                '-'
            } else if dy.abs() > dx * 2 {
                '|'
            } else if (sx > 0 && sy > 0) || (sx < 0 && sy < 0) {
                '\\'
            } else {
                '/'
            };
            buffer[y as usize][x as usize] = c;
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            if x == x1 {
                break;
            }
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            if y == y1 {
                break;
            }
            err += dx;
            y += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube_creation() {
        let cube = WireframeModel::cube();
        assert_eq!(cube.vertices.len(), 8);
        assert_eq!(cube.edges.len(), 12);
    }

    #[test]
    fn test_pyramid_creation() {
        let pyramid = WireframeModel::pyramid();
        assert_eq!(pyramid.vertices.len(), 5);
        assert_eq!(pyramid.edges.len(), 8);
    }

    #[test]
    fn test_render_wireframe() {
        let cube = WireframeModel::cube();
        let pixels = render_wireframe(&cube, 100, 100, (0.0, 0.0, 0.0));
        assert_eq!(pixels.len(), 100 * 100 * 4);
    }

    #[test]
    fn test_ascii_rendering() {
        let cube = WireframeModel::cube();
        let ascii = wireframe_to_ascii(&cube, 40, 20, (0.5, 0.5, 0.0));
        assert_eq!(ascii.len(), 20);
        assert!(ascii.iter().all(|row| row.len() == 40));
    }

    #[test]
    fn test_scale() {
        let mut cube = WireframeModel::cube();
        cube.scale(2.0);
        assert_eq!(cube.vertices[0], (-2.0, -2.0, -2.0));
    }

    #[test]
    fn test_translate() {
        let mut cube = WireframeModel::cube();
        cube.translate(1.0, 2.0, 3.0);
        assert_eq!(cube.vertices[0], (0.0, 1.0, 2.0));
    }
}
