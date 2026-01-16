//! 3D rendering for model viewer
//!
//! Software wireframe and rasterization rendering.

use super::state::{Camera, DrawStyle, Model};
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

/// A projected triangle for filled rendering
#[derive(Debug, Clone, Copy)]
pub struct Triangle2D {
    pub p0: Point2D,
    pub p1: Point2D,
    pub p2: Point2D,
    pub brightness: f32, // 0.0 to 1.0 based on face normal
}

/// Project 3D model to 2D screen space
pub fn project_model(model: &Model, camera: &Camera, width: u16, height: u16) -> Vec<Line2D> {
    let aspect = width as f32 / height as f32;

    // Account for terminal character aspect ratio (chars are ~2x taller than wide)
    let char_aspect = 2.0;

    let projection = Mat4::perspective_rh(
        std::f32::consts::FRAC_PI_4, // 45 degree FOV
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
                y: (1.0 - ndc.y) * 0.5 * height as f32, // Flip Y for screen coords
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
        '-' // Horizontal
    } else if angle_deg.abs() > 157.5 {
        '-' // Horizontal (opposite direction)
    } else if (angle_deg - 90.0).abs() < 22.5 || (angle_deg + 90.0).abs() < 22.5 {
        '|' // Vertical
    } else if angle_deg > 0.0 && angle_deg < 90.0 {
        '\\' // Diagonal down-right
    } else if angle_deg > 90.0 {
        '/' // Diagonal down-left
    } else if angle_deg < 0.0 && angle_deg > -90.0 {
        '/' // Diagonal up-right
    } else {
        '\\' // Diagonal up-left
    }
}

/// Render to RGB image buffer for image protocol display
pub fn render_image(model: &Model, camera: &Camera, width: u32, height: u32) -> Vec<u8> {
    let mut buffer = vec![0u8; (width * height * 3) as usize];

    // Fill with dark background
    for i in 0..(width * height) as usize {
        buffer[i * 3] = 20; // R
        buffer[i * 3 + 1] = 20; // G
        buffer[i * 3 + 2] = 30; // B
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

/// Project 3D model to triangles for filled rendering
pub fn project_triangles(
    model: &Model,
    camera: &Camera,
    width: u16,
    height: u16,
) -> Vec<Triangle2D> {
    let aspect = width as f32 / height as f32;
    let char_aspect = 2.0;

    let projection = Mat4::perspective_rh(
        std::f32::consts::FRAC_PI_4,
        aspect * char_aspect,
        0.1,
        100.0,
    );

    let view = camera.view_matrix();
    let mvp = projection * view;

    // Light direction (from camera)
    let light_dir = Vec3::new(0.5, 0.8, 0.5).normalize();

    // Transform all vertices to screen space
    let screen_verts: Vec<Option<Point2D>> = model
        .vertices
        .iter()
        .map(|v| {
            let clip = mvp * Vec4::new(v.position.x, v.position.y, v.position.z, 1.0);

            if clip.w <= 0.0 {
                return None;
            }

            let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);

            Some(Point2D {
                x: (ndc.x + 1.0) * 0.5 * width as f32,
                y: (1.0 - ndc.y) * 0.5 * height as f32,
                depth: ndc.z,
            })
        })
        .collect();

    // Generate triangles from faces
    let mut triangles: Vec<Triangle2D> = Vec::new();

    for face in &model.faces {
        let v0 = screen_verts.get(face.v0).and_then(|v| *v);
        let v1 = screen_verts.get(face.v1).and_then(|v| *v);
        let v2 = screen_verts.get(face.v2).and_then(|v| *v);

        if let (Some(p0), Some(p1), Some(p2)) = (v0, v1, v2) {
            // Calculate face normal for lighting
            let world_v0 = model.vertices[face.v0].position;
            let world_v1 = model.vertices[face.v1].position;
            let world_v2 = model.vertices[face.v2].position;

            let edge1 = world_v1 - world_v0;
            let edge2 = world_v2 - world_v0;
            let normal = edge1.cross(edge2).normalize();

            // Simple diffuse lighting
            let brightness = normal.dot(light_dir).clamp(0.1, 1.0);

            triangles.push(Triangle2D {
                p0,
                p1,
                p2,
                brightness,
            });
        }
    }

    // Sort by average depth (painter's algorithm - back to front)
    triangles.sort_by(|a, b| {
        let depth_a = (a.p0.depth + a.p1.depth + a.p2.depth) / 3.0;
        let depth_b = (b.p0.depth + b.p1.depth + b.p2.depth) / 3.0;
        depth_b
            .partial_cmp(&depth_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    triangles
}

/// Render filled ASCII
pub fn render_ascii_filled(model: &Model, camera: &Camera, width: u16, height: u16) -> Vec<String> {
    let mut buffer: Vec<Vec<char>> = vec![vec![' '; width as usize]; height as usize];
    let mut depth_buffer: Vec<Vec<f32>> = vec![vec![f32::MAX; width as usize]; height as usize];

    let triangles = project_triangles(model, camera, width, height);

    // Shade characters by brightness
    let shade_chars = ['.', ':', '-', '=', '+', '*', '#', '%', '@'];

    for tri in triangles {
        fill_triangle_ascii(
            &mut buffer,
            &mut depth_buffer,
            &tri,
            width,
            height,
            &shade_chars,
        );
    }

    buffer.iter().map(|row| row.iter().collect()).collect()
}

/// Fill a triangle in ASCII buffer
fn fill_triangle_ascii(
    buffer: &mut [Vec<char>],
    depth_buffer: &mut [Vec<f32>],
    tri: &Triangle2D,
    width: u16,
    height: u16,
    shade_chars: &[char],
) {
    // Get bounding box
    let min_x = tri.p0.x.min(tri.p1.x).min(tri.p2.x).max(0.0) as i32;
    let max_x = tri.p0.x.max(tri.p1.x).max(tri.p2.x).min(width as f32 - 1.0) as i32;
    let min_y = tri.p0.y.min(tri.p1.y).min(tri.p2.y).max(0.0) as i32;
    let max_y = tri
        .p0
        .y
        .max(tri.p1.y)
        .max(tri.p2.y)
        .min(height as f32 - 1.0) as i32;

    // Choose character based on brightness
    let char_idx =
        ((tri.brightness * (shade_chars.len() - 1) as f32) as usize).min(shade_chars.len() - 1);
    let ch = shade_chars[char_idx];

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            if point_in_triangle(px, py, tri) {
                // Interpolate depth
                let depth = interpolate_depth(px, py, tri);

                if depth < depth_buffer[y as usize][x as usize] {
                    depth_buffer[y as usize][x as usize] = depth;
                    buffer[y as usize][x as usize] = ch;
                }
            }
        }
    }
}

/// Render filled image
pub fn render_image_filled(model: &Model, camera: &Camera, width: u32, height: u32) -> Vec<u8> {
    let mut buffer = vec![0u8; (width * height * 3) as usize];
    let mut depth_buffer = vec![f32::MAX; (width * height) as usize];

    // Fill with dark background
    for i in 0..(width * height) as usize {
        buffer[i * 3] = 20;
        buffer[i * 3 + 1] = 20;
        buffer[i * 3 + 2] = 30;
    }

    let triangles = project_triangles(model, camera, width as u16, height as u16);

    for tri in triangles {
        fill_triangle_image(&mut buffer, &mut depth_buffer, &tri, width, height);
    }

    buffer
}

/// Fill a triangle in image buffer
fn fill_triangle_image(
    buffer: &mut [u8],
    depth_buffer: &mut [f32],
    tri: &Triangle2D,
    width: u32,
    height: u32,
) {
    // Get bounding box
    let min_x = tri.p0.x.min(tri.p1.x).min(tri.p2.x).max(0.0) as i32;
    let max_x = tri.p0.x.max(tri.p1.x).max(tri.p2.x).min(width as f32 - 1.0) as i32;
    let min_y = tri.p0.y.min(tri.p1.y).min(tri.p2.y).max(0.0) as i32;
    let max_y = tri
        .p0
        .y
        .max(tri.p1.y)
        .max(tri.p2.y)
        .min(height as f32 - 1.0) as i32;

    // Green with brightness
    let r = (50.0 + tri.brightness * 50.0) as u8;
    let g = (100.0 + tri.brightness * 155.0) as u8;
    let b = (50.0 + tri.brightness * 50.0) as u8;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            if point_in_triangle(px, py, tri) {
                let depth = interpolate_depth(px, py, tri);
                let idx = (y as u32 * width + x as u32) as usize;

                if depth < depth_buffer[idx] {
                    depth_buffer[idx] = depth;
                    buffer[idx * 3] = r;
                    buffer[idx * 3 + 1] = g;
                    buffer[idx * 3 + 2] = b;
                }
            }
        }
    }
}

/// Check if point is inside triangle using barycentric coordinates
fn point_in_triangle(px: f32, py: f32, tri: &Triangle2D) -> bool {
    let v0 = Vec3::new(tri.p2.x - tri.p0.x, tri.p1.x - tri.p0.x, tri.p0.x - px);
    let v1 = Vec3::new(tri.p2.y - tri.p0.y, tri.p1.y - tri.p0.y, tri.p0.y - py);

    let u = v0.cross(v1);

    if u.z.abs() < 1.0 {
        return false;
    }

    let u = u / u.z;
    u.x >= 0.0 && u.y >= 0.0 && (u.x + u.y) <= 1.0
}

/// Interpolate depth at a point inside triangle
fn interpolate_depth(px: f32, py: f32, tri: &Triangle2D) -> f32 {
    // Calculate barycentric coordinates
    let v0 = (tri.p1.x - tri.p0.x, tri.p1.y - tri.p0.y);
    let v1 = (tri.p2.x - tri.p0.x, tri.p2.y - tri.p0.y);
    let v2 = (px - tri.p0.x, py - tri.p0.y);

    let dot00 = v0.0 * v0.0 + v0.1 * v0.1;
    let dot01 = v0.0 * v1.0 + v0.1 * v1.1;
    let dot02 = v0.0 * v2.0 + v0.1 * v2.1;
    let dot11 = v1.0 * v1.0 + v1.1 * v1.1;
    let dot12 = v1.0 * v2.0 + v1.1 * v2.1;

    let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

    tri.p0.depth * (1.0 - u - v) + tri.p1.depth * u + tri.p2.depth * v
}

/// Main ASCII render function with draw style support
pub fn render_ascii_with_style(
    model: &Model,
    camera: &Camera,
    width: u16,
    height: u16,
    style: DrawStyle,
) -> Vec<String> {
    match style {
        DrawStyle::Wireframe => render_ascii(model, camera, width, height),
        DrawStyle::Filled => render_ascii_filled(model, camera, width, height),
    }
}

/// Main image render function with draw style support
pub fn render_image_with_style(
    model: &Model,
    camera: &Camera,
    width: u32,
    height: u32,
    style: DrawStyle,
) -> Vec<u8> {
    match style {
        DrawStyle::Wireframe => render_image(model, camera, width, height),
        DrawStyle::Filled => render_image_filled(model, camera, width, height),
    }
}
