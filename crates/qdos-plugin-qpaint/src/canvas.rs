//! Q-PAINT Canvas Operations
//!
//! Drawing algorithms for shapes, fill, and other canvas manipulations.

use crate::state::{Canvas, Selection, Tool};

/// Draw a single pixel or brush stroke
pub fn draw_pixel(canvas: &mut Canvas, x: u32, y: u32, color: (u8, u8, u8), brush_size: u8) {
    if brush_size <= 1 {
        canvas.set_pixel(x, y, color);
    } else {
        // Draw a square brush
        let half = (brush_size / 2) as i32;
        for dy in -half..=half {
            for dx in -half..=half {
                let px = (x as i32 + dx).clamp(0, canvas.width as i32 - 1) as u32;
                let py = (y as i32 + dy).clamp(0, canvas.height as i32 - 1) as u32;
                canvas.set_pixel(px, py, color);
            }
        }
    }
}

/// Draw a line using Bresenham's algorithm
pub fn draw_line(canvas: &mut Canvas, x0: u32, y0: u32, x1: u32, y1: u32, color: (u8, u8, u8)) {
    let x0 = x0 as i32;
    let y0 = y0 as i32;
    let x1 = x1 as i32;
    let y1 = y1 as i32;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && x < canvas.width as i32 && y >= 0 && y < canvas.height as i32 {
            canvas.set_pixel(x as u32, y as u32, color);
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

/// Draw a rectangle (outline or filled)
#[allow(dead_code)]
pub fn draw_rectangle(
    canvas: &mut Canvas,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    color: (u8, u8, u8),
    filled: bool,
) {
    let min_x = x0.min(x1);
    let max_x = x0.max(x1);
    let min_y = y0.min(y1);
    let max_y = y0.max(y1);

    if filled {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                canvas.set_pixel(x, y, color);
            }
        }
    } else {
        // Top and bottom edges
        for x in min_x..=max_x {
            canvas.set_pixel(x, min_y, color);
            canvas.set_pixel(x, max_y, color);
        }
        // Left and right edges
        for y in min_y..=max_y {
            canvas.set_pixel(min_x, y, color);
            canvas.set_pixel(max_x, y, color);
        }
    }
}

/// Draw an ellipse using midpoint algorithm
#[allow(dead_code)]
pub fn draw_ellipse(
    canvas: &mut Canvas,
    cx: u32,
    cy: u32,
    rx: u32,
    ry: u32,
    color: (u8, u8, u8),
    filled: bool,
) {
    if rx == 0 || ry == 0 {
        canvas.set_pixel(cx, cy, color);
        return;
    }

    let rx = rx as i32;
    let ry = ry as i32;
    let cx = cx as i32;
    let cy = cy as i32;

    let mut x = 0i32;
    let mut y = ry;

    // Initial decision parameter for region 1
    let mut d1 = (ry * ry) - (rx * rx * ry) + (rx * rx) / 4;
    let mut dx = 2 * ry * ry * x;
    let mut dy = 2 * rx * rx * y;

    // Region 1
    while dx < dy {
        plot_ellipse_points(canvas, cx, cy, x, y, color, filled);

        if d1 < 0 {
            x += 1;
            dx += 2 * ry * ry;
            d1 += dx + ry * ry;
        } else {
            x += 1;
            y -= 1;
            dx += 2 * ry * ry;
            dy -= 2 * rx * rx;
            d1 += dx - dy + ry * ry;
        }
    }

    // Decision parameter for region 2
    let mut d2 = (ry * ry * ((2 * x + 1) * (2 * x + 1))) / 4 + (rx * rx * (y - 1) * (y - 1))
        - (rx * rx * ry * ry);

    // Region 2
    while y >= 0 {
        plot_ellipse_points(canvas, cx, cy, x, y, color, filled);

        if d2 > 0 {
            y -= 1;
            dy -= 2 * rx * rx;
            d2 += rx * rx - dy;
        } else {
            y -= 1;
            x += 1;
            dx += 2 * ry * ry;
            dy -= 2 * rx * rx;
            d2 += dx - dy + rx * rx;
        }
    }
}

#[allow(dead_code)]
fn plot_ellipse_points(
    canvas: &mut Canvas,
    cx: i32,
    cy: i32,
    x: i32,
    y: i32,
    color: (u8, u8, u8),
    filled: bool,
) {
    if filled {
        // Draw horizontal lines for filled ellipse
        draw_horizontal_line(canvas, cx - x, cx + x, cy + y, color);
        draw_horizontal_line(canvas, cx - x, cx + x, cy - y, color);
    } else {
        // Plot the four symmetric points
        plot_point(canvas, cx + x, cy + y, color);
        plot_point(canvas, cx - x, cy + y, color);
        plot_point(canvas, cx + x, cy - y, color);
        plot_point(canvas, cx - x, cy - y, color);
    }
}

fn draw_horizontal_line(canvas: &mut Canvas, x1: i32, x2: i32, y: i32, color: (u8, u8, u8)) {
    if y < 0 || y >= canvas.height as i32 {
        return;
    }
    let min_x = x1.max(0) as u32;
    let max_x = x2.min(canvas.width as i32 - 1) as u32;
    for x in min_x..=max_x {
        canvas.set_pixel(x, y as u32, color);
    }
}

fn plot_point(canvas: &mut Canvas, x: i32, y: i32, color: (u8, u8, u8)) {
    if x >= 0 && x < canvas.width as i32 && y >= 0 && y < canvas.height as i32 {
        canvas.set_pixel(x as u32, y as u32, color);
    }
}

/// Flood fill using scanline algorithm
#[allow(dead_code)]
pub fn flood_fill(canvas: &mut Canvas, start_x: u32, start_y: u32, fill_color: (u8, u8, u8)) {
    let target_color = canvas.get_pixel(start_x, start_y);

    if target_color == fill_color {
        return;
    }

    let mut stack = vec![(start_x, start_y)];

    while let Some((x, y)) = stack.pop() {
        if x >= canvas.width || y >= canvas.height {
            continue;
        }

        if canvas.get_pixel(x, y) != target_color {
            continue;
        }

        // Find left and right bounds of this scanline segment
        let mut left = x;
        while left > 0 && canvas.get_pixel(left - 1, y) == target_color {
            left -= 1;
        }

        let mut right = x;
        while right < canvas.width - 1 && canvas.get_pixel(right + 1, y) == target_color {
            right += 1;
        }

        // Fill the scanline
        for fill_x in left..=right {
            canvas.set_pixel(fill_x, y, fill_color);
        }

        // Check lines above and below
        for fill_x in left..=right {
            if y > 0 && canvas.get_pixel(fill_x, y - 1) == target_color {
                stack.push((fill_x, y - 1));
            }
            if y < canvas.height - 1 && canvas.get_pixel(fill_x, y + 1) == target_color {
                stack.push((fill_x, y + 1));
            }
        }
    }
}

/// Apply a tool action at the given position
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn apply_tool(
    canvas: &mut Canvas,
    tool: Tool,
    x: u32,
    y: u32,
    fg_color: (u8, u8, u8),
    bg_color: (u8, u8, u8),
    brush_size: u8,
    start_pos: Option<(u32, u32)>,
) -> Option<(u8, u8, u8)> {
    match tool {
        Tool::Pencil | Tool::Brush => {
            draw_pixel(canvas, x, y, fg_color, brush_size);
            None
        }
        Tool::Eraser => {
            draw_pixel(canvas, x, y, bg_color, brush_size);
            None
        }
        Tool::Line => {
            if let Some((sx, sy)) = start_pos {
                draw_line(canvas, sx, sy, x, y, fg_color);
            }
            None
        }
        Tool::ColorPicker => Some(canvas.get_pixel(x, y)),
        Tool::Select => None, // Selection is handled separately
        Tool::Text => None,   // Text is handled separately
    }
}

/// Draw selection rectangle preview
#[allow(dead_code)]
pub fn draw_selection_preview(_canvas: &Canvas, selection: &Selection) -> Vec<(u32, u32)> {
    if !selection.active {
        return Vec::new();
    }

    let (min_x, min_y, max_x, max_y) = selection.bounds();
    let mut points = Vec::new();

    // Top and bottom edges
    for x in min_x..=max_x {
        points.push((x, min_y));
        points.push((x, max_y));
    }
    // Left and right edges (avoiding corners)
    for y in (min_y + 1)..max_y {
        points.push((min_x, y));
        points.push((max_x, y));
    }

    points
}
