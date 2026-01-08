//! 3D rendering for model viewer
//!
//! Software wireframe and rasterization rendering.

use super::state::{Camera, Model};
use glam::{Mat4, Vec3, Vec4};

/// A 2D point after projection
#[derive(Debug, Clone, Copy)]
pub struct Point2D {
    pub x: f32,
    pub y: f32,
    pub depth: f32,
}

/// A line segment to draw
#[derive(Debug, Clone, Copy)]
pub struct Line2D {
    pub p0: Point2D,
    pub p1: Point2D,
}

/// Project 3D model to 2D screen space
pub fn project_model(model: &Model, camera: &Camera, width: u16, height: u16) -> Vec<Line2D> {
    let aspect = width as f32 / height as f32;

    // Account for terminal character aspect ratio (chars are ~2x taller than wide)
    let char_aspect = 2.0;

    let projection = Mat4::perspective_rh(
        std::f32::consts::FRAC_PI_4,  // 45 degree FOV
        aspect * char_aspect,
        0.1,
        100.0,
    );

    let view = camera.view_matrix();
    let mvp = projection * view;

    // Transform all vertices to screen space
    let screen_verts: Vec<Option<Point2D>> = model
        .vertices
        .iter()
        .map(|v| {
            let clip = mvp * Vec4::new(v.position.x, v.position.y, v.position.z, 1.0);

            // Perspective divide
            if clip.w <= 0.0 {
                return None;
            }

            let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);

            // NDC to screen coordinates
            Some(Point2D {
                x: (ndc.x + 1.0) * 0.5 * width as f32,
                y: (1.0 - ndc.y) * 0.5 * height as f32,  // Flip Y for screen coords
                depth: ndc.z,
            })
        })
        .collect();

    // Generate lines from faces
    let mut lines = Vec::new();

    for face in &model.faces {
        let v0 = screen_verts.get(face.v0).and_then(|v| *v);
        let v1 = screen_verts.get(face.v1).and_then(|v| *v);
        let v2 = screen_verts.get(face.v2).and_then(|v| *v);

        if let (Some(p0), Some(p1)) = (v0, v1) {
            lines.push(Line2D { p0, p1 });
        }
        if let (Some(p1), Some(p2)) = (v1, v2) {
            lines.push(Line2D { p0: p1, p1: p2 });
        }
        if let (Some(p2), Some(p0)) = (v2, v0) {
            lines.push(Line2D { p0: p2, p1: p0 });
        }
    }

    lines
}

/// Render wireframe to ASCII character buffer
pub fn render_ascii(model: &Model, camera: &Camera, width: u16, height: u16) -> Vec<String> {
    let mut buffer: Vec<Vec<char>> = vec![vec![' '; width as usize]; height as usize];

    let lines = project_model(model, camera, width, height);

    // Draw lines using Bresenham's algorithm
    for line in lines {
        draw_line(&mut buffer, line.p0, line.p1, width, height);
    }

    // Convert to strings
    buffer.iter().map(|row| row.iter().collect()).collect()
}

/// Draw a line using Bresenham's algorithm
fn draw_line(buffer: &mut [Vec<char>], p0: Point2D, p1: Point2D, width: u16, height: u16) {
    let x0 = p0.x as i32;
    let y0 = p0.y as i32;
    let x1 = p1.x as i32;
    let y1 = p1.y as i32;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        // Plot point if in bounds
        if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
            let ch = get_line_char(x0, y0, x1, y1, x, y);
            buffer[y as usize][x as usize] = ch;
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Get appropriate character for line direction
fn get_line_char(x0: i32, y0: i32, x1: i32, y1: i32, _x: i32, _y: i32) -> char {
    let dx = (x1 - x0) as f32;
    let dy = (y1 - y0) as f32;

    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return '*';
    }

    let angle = dy.atan2(dx);
    let angle_deg = angle.to_degrees();

    // Choose character based on angle
    if angle_deg.abs() < 22.5 {
        '-'  // Horizontal
    } else if angle_deg.abs() > 157.5 {
        '-'  // Horizontal (opposite direction)
    } else if (angle_deg - 90.0).abs() < 22.5 || (angle_deg + 90.0).abs() < 22.5 {
        '|'  // Vertical
    } else if angle_deg > 0.0 && angle_deg < 90.0 {
        '\\'  // Diagonal down-right
    } else if angle_deg > 90.0 {
        '/'  // Diagonal down-left
    } else if angle_deg < 0.0 && angle_deg > -90.0 {
        '/'  // Diagonal up-right
    } else {
        '\\'  // Diagonal up-left
    }
}

/// Render to RGB image buffer for image protocol display
pub fn render_image(
    model: &Model,
    camera: &Camera,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let mut buffer = vec![0u8; (width * height * 3) as usize];

    // Fill with dark background
    for i in 0..(width * height) as usize {
        buffer[i * 3] = 20;      // R
        buffer[i * 3 + 1] = 20;  // G
        buffer[i * 3 + 2] = 30;  // B
    }

    let lines = project_model(model, camera, width as u16, height as u16);

    // Draw lines
    for line in lines {
        draw_line_image(&mut buffer, line.p0, line.p1, width, height);
    }

    buffer
}

/// Draw a line to RGB buffer
fn draw_line_image(buffer: &mut [u8], p0: Point2D, p1: Point2D, width: u32, height: u32) {
    let x0 = p0.x as i32;
    let y0 = p0.y as i32;
    let x1 = p1.x as i32;
    let y1 = p1.y as i32;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    // Line color (green wireframe)
    let color = [100, 255, 100];

    loop {
        if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
            let idx = ((y as u32 * width + x as u32) * 3) as usize;
            buffer[idx] = color[0];
            buffer[idx + 1] = color[1];
            buffer[idx + 2] = color[2];
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}
