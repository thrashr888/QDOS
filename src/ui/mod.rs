pub mod components;
mod modals;

use crate::app::{App, Modal, NavItem, SortMode};
use crate::file_ops::GitStatus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

// Use submodule functions
use modals::draw_modal;

/// QDOS color scheme - exact DOS 16-color palette RGB values
pub(crate) const COLOR_BG: Color = Color::Reset; // Terminal default (transparent)
pub(crate) const COLOR_FG: Color = Color::Rgb(255, 255, 255); // White
pub(crate) const COLOR_BLUE: Color = Color::Rgb(102, 183, 179); // DOS Blue - borders, menu items
pub(crate) const COLOR_GREEN: Color = Color::Rgb(103, 204, 77); // DOS Green - help text, descriptions
pub(crate) const COLOR_RED: Color = Color::Rgb(157, 31, 20); // DOS Red - path bar, selection bg
pub(crate) const COLOR_YELLOW: Color = Color::Rgb(232, 218, 89); // DOS Yellow - tagged items
pub(crate) const COLOR_GREY: Color = Color::Rgb(128, 128, 128); // Grey - hidden files
pub(crate) const COLOR_CYAN: Color = Color::Rgb(0, 170, 170); // Cyan - git added
#[allow(dead_code)]
pub(crate) const COLOR_MAGENTA: Color = Color::Rgb(170, 0, 170); // Magenta - git untracked

/// Format file size with B, K, M, G, T, P suffixes (max 2 decimal places)
pub(crate) fn format_size_short(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;
    const PB: u64 = TB * 1024;

    if bytes >= PB {
        let val = bytes as f64 / PB as f64;
        if val >= 100.0 {
            format!("{:.0}P", val)
        } else if val >= 10.0 {
            format!("{:.1}P", val)
        } else {
            format!("{:.2}P", val)
        }
    } else if bytes >= TB {
        let val = bytes as f64 / TB as f64;
        if val >= 100.0 {
            format!("{:.0}T", val)
        } else if val >= 10.0 {
            format!("{:.1}T", val)
        } else {
            format!("{:.2}T", val)
        }
    } else if bytes >= GB {
        let val = bytes as f64 / GB as f64;
        if val >= 100.0 {
            format!("{:.0}G", val)
        } else if val >= 10.0 {
            format!("{:.1}G", val)
        } else {
            format!("{:.2}G", val)
        }
    } else if bytes >= MB {
        let val = bytes as f64 / MB as f64;
        if val >= 100.0 {
            format!("{:.0}M", val)
        } else if val >= 10.0 {
            format!("{:.1}M", val)
        } else {
            format!("{:.2}M", val)
        }
    } else if bytes >= KB {
        let val = bytes as f64 / KB as f64;
        if val >= 100.0 {
            format!("{:.0}K", val)
        } else if val >= 10.0 {
            format!("{:.1}K", val)
        } else {
            format!("{:.2}K", val)
        }
    } else {
        format!("{}B", bytes)
    }
}

/// Color usage (based on the original QDOS default color scheme):
///
/// L1: white on black. selected item is yellow on red.
/// L2: green on black.
/// L3: double line white on black
/// L4: white on black. path input is yellow on red.
/// L5: double line white on black.
/// L6: blue on black headers.
/// table: headers are blue on black. rows are white on black. selected row is yellow on red. borders are double line white on black.
/// stats: headers are blue on black. data are white on black, with double line white-on-black border.
/// keybindings: all text is white on black. single line white-on-black border above and below.
/// copyright: all text is blue on black.

/// Format a number with comma separators (DOS style)
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }
    result
}

/// Draw the entire UI
pub fn draw(frame: &mut Frame, app: &App) {
    let size = frame.area();
    let colors = app.colors();

    // Fill entire background with theme color
    let bg_block = ratatui::widgets::Block::default().style(Style::default().bg(colors.bg()));
    frame.render_widget(bg_block, size);

    // Minimum size check - show message if too small (must check before drawing anything)
    let min_width: u16 = 80;
    let min_height: u16 = 25;

    if size.width < min_width || size.height < min_height {
        let msg = format!(
            "Terminal too small. Need {}x{}, have {}x{}",
            min_width, min_height, size.width, size.height
        );
        frame.render_widget(
            Paragraph::new(Span::styled(msg, Style::default().fg(colors.red()))),
            size,
        );
        return;
    }

    // Main layout: nav bar at top, then content (no empty line after PATH)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Navigation bar + description
            Constraint::Length(1), // Separator line above path
            Constraint::Length(1), // Path bar
            Constraint::Min(10),   // Content area (directly after PATH)
        ])
        .split(size);

    // Draw navigation bar
    draw_nav_bar(frame, app, main_chunks[0]);

    // Draw separator line above path (double line white)
    draw_separator_line(frame, app, main_chunks[1]);

    // Draw path bar with red background on path text only
    draw_path_bar(frame, app, main_chunks[2]);

    // Draw integrated content area (stats panel + file table as one grid)
    draw_integrated_content(frame, app, main_chunks[3]);

    // Draw modal if active
    if !matches!(app.modal, Modal::None) {
        draw_modal(frame, app, size);
    }
}

/// Draw the navigation menu bar with horizontal scrolling
fn draw_nav_bar(frame: &mut Frame, app: &App, area: Rect) {
    let colors = app.colors();
    let available_width = area.width as usize;

    // Calculate item widths and positions
    let item_widths: Vec<usize> = NavItem::ALL
        .iter()
        .map(|item| item.as_str().len() + 2)
        .collect();
    let total_width: usize = item_widths.iter().sum();

    // Calculate item start positions
    let mut item_positions: Vec<usize> = Vec::new();
    let mut pos = 0;
    for width in &item_widths {
        item_positions.push(pos);
        pos += width;
    }

    // Determine scroll offset needed to show selected item
    let selected_start = item_positions[app.nav_index];
    let selected_end = selected_start + item_widths[app.nav_index];

    // Use app's stored scroll offset, but adjust if selected item is not visible
    let mut scroll_offset = app.nav_scroll_offset;
    let indicator_space = if scroll_offset > 0 || total_width > available_width {
        2
    } else {
        0
    };
    let visible_width = available_width.saturating_sub(indicator_space * 2);

    // Ensure selected item is visible (scroll when needed pattern)
    if selected_start < scroll_offset {
        scroll_offset = selected_start;
    } else if selected_end > scroll_offset + visible_width {
        scroll_offset = selected_end.saturating_sub(visible_width);
    }

    // Build visible menu items
    let has_left = scroll_offset > 0;
    let has_right = scroll_offset + visible_width < total_width;

    let mut nav_spans: Vec<Span> = Vec::new();

    // Left scroll indicator
    if has_left {
        nav_spans.push(Span::styled("< ", Style::default().fg(colors.blue())));
    }

    // Render visible items
    for (i, item) in NavItem::ALL.iter().enumerate() {
        let item_start = item_positions[i];
        let item_end = item_start + item_widths[i];

        // Skip items that are fully before the visible area
        if item_end <= scroll_offset {
            continue;
        }
        // Stop if item is fully after the visible area
        if item_start >= scroll_offset + visible_width {
            break;
        }

        let name = item.as_str();
        let first_char = &name[..1];
        let rest = &name[1..];

        let is_selected = i == app.nav_index;

        // First letter style: green normally, yellow on red when selected
        let first_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.green())
        };

        // Rest of name style: white normally, yellow on red when selected
        let rest_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg())
        };

        nav_spans.push(Span::styled(first_char, first_style));
        nav_spans.push(Span::styled(rest, rest_style));
        nav_spans.push(Span::raw("  "));
    }

    // Right scroll indicator
    if has_right {
        nav_spans.push(Span::styled(" >", Style::default().fg(colors.blue())));
    }

    let nav_line = Line::from(nav_spans);

    // Second line: description in green (like original)
    let description = NavItem::ALL[app.nav_index].description();
    let desc_line = Line::from(Span::styled(
        description,
        Style::default().fg(colors.green()),
    ));

    let nav_text = vec![nav_line, desc_line];
    let nav_paragraph = Paragraph::new(nav_text);
    frame.render_widget(nav_paragraph, area);
}

/// Draw separator line above path (double line white on black)
fn draw_separator_line(frame: &mut Frame, app: &App, area: Rect) {
    let colors = app.colors();
    let line_char = "═";
    let line = line_char.repeat(area.width as usize);
    let separator = Paragraph::new(Line::from(Span::styled(
        line,
        Style::default().fg(colors.fg()),
    )));
    frame.render_widget(separator, area);
}

/// Draw the path bar - "PATH  >> " is white, only the path value is yellow on red (extending to far right)
fn draw_path_bar(frame: &mut Frame, app: &App, area: Rect) {
    let colors = app.colors();
    let label = " PATH  >>  ";
    let path_str = format!("{}", app.current_path.display());
    // Pad the path value to fill remaining width
    let remaining_width = area.width.saturating_sub(label.len() as u16) as usize;
    let padded_path = format!("{:<width$}", path_str, width = remaining_width);

    let path_line = Line::from(vec![
        Span::styled(label, Style::default().fg(colors.fg())), // White label
        Span::styled(
            padded_path,
            Style::default().fg(colors.yellow()).bg(colors.red()), // Yellow on red for path value
        ),
    ]);

    let path_paragraph = Paragraph::new(path_line);
    frame.render_widget(path_paragraph, area);
}

/// Draw integrated content area - stats panel and file table as one grid
/// This matches the original QDOS layout where panels share borders
fn draw_integrated_content(frame: &mut Frame, app: &App, area: Rect) {
    let colors = app.colors();
    let border_style = Style::default().fg(colors.fg());
    let header_style = Style::default().fg(colors.blue());

    // Left panel width (stats/keybindings/copyright)
    let left_width: u16 = 30;

    // Calculate dynamic size column width based on file sizes
    let max_size_width = app
        .files
        .iter()
        .filter(|f| !f.is_dir)
        .map(|f| format_number(f.size).len())
        .max()
        .unwrap_or(4); // "Size" header minimum
    let size_col: u16 = (max_size_width.max(4) + 2) as u16; // +2 for padding

    // File table column widths - name column expands to fill remaining space
    let kind_col: u16 = 6; // "KIND" + padding
    let date_col: u16 = 10;
    let time_col: u16 = 9;
    // Calculate name_col to fill remaining width (area.width - left_width - kind - size - date - time - 5 separators - 1 right border)
    let fixed_cols = left_width + kind_col + size_col + date_col + time_col + 5 + 1;
    let name_col: u16 = area.width.saturating_sub(fixed_cols).max(14);

    // Calculate positions
    let file_table_x = area.x + left_width;
    let file_table_width = area.width.saturating_sub(left_width);

    // ROW 0: Top border line
    let mut top_line = String::new();
    top_line.push_str(&"═".repeat((left_width as usize).saturating_sub(1)));
    top_line.push('╦');
    top_line.push_str(&"═".repeat(name_col as usize));
    top_line.push('╦');
    top_line.push_str(&"═".repeat(kind_col as usize));
    top_line.push('╦');
    top_line.push_str(&"═".repeat(size_col as usize));
    top_line.push('╦');
    top_line.push_str(&"═".repeat(date_col as usize));
    top_line.push('╦');
    top_line.push_str(&"═".repeat(time_col as usize));
    top_line.push('╗');
    frame.render_widget(
        Paragraph::new(Span::styled(&top_line, border_style)),
        Rect::new(area.x, area.y, area.width, 1),
    );

    // ROW 1: Headers row
    let y = area.y + 1;
    // Left side headers
    let left_header = format!(" {:>5}  {:>18}   ", "Count", "Total Size");
    frame.render_widget(
        Paragraph::new(Span::styled(&left_header, header_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );
    frame.render_widget(
        Paragraph::new(Span::styled("║", border_style)),
        Rect::new(file_table_x - 1, y, 1, 1),
    );
    // File table headers with sort indicators
    let mut x = file_table_x;
    // Helper to get sort arrow for a column
    let name_arrow = match app.sort_mode {
        SortMode::NameAsc => " ↑",
        SortMode::NameDesc => " ↓",
        _ => "",
    };
    let size_arrow = match app.sort_mode {
        SortMode::SizeAsc => " ↑",
        SortMode::SizeDesc => " ↓",
        _ => "",
    };
    let date_arrow = match app.sort_mode {
        SortMode::DateAsc => " ↑",
        SortMode::DateDesc => " ↓",
        _ => "",
    };
    // Extension sort shows on Name column since we don't have a separate ext column
    let ext_arrow = match app.sort_mode {
        SortMode::ExtAsc => " (Ext↑)",
        SortMode::ExtDesc => " (Ext↓)",
        _ => "",
    };
    let name_header = if !ext_arrow.is_empty() {
        format!(" File Name{}", ext_arrow)
    } else {
        format!(" File Name{}", name_arrow)
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{:<width$}", name_header, width = name_col as usize),
            header_style,
        )),
        Rect::new(x, y, name_col.min(area.width.saturating_sub(x - area.x)), 1),
    );
    x += name_col;
    let max_x = area.x + area.width;
    // All header column separators with bounds checking
    if x < max_x {
        frame.render_widget(
            Paragraph::new(Span::styled("║", border_style)),
            Rect::new(x, y, 1, 1),
        );
    }
    x += 1;
    if x < max_x {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!(" {:<width$}", "Kind", width = kind_col as usize - 1),
                header_style,
            )),
            Rect::new(x, y, kind_col.min(max_x - x), 1),
        );
    }
    x += kind_col;
    if x < max_x {
        frame.render_widget(
            Paragraph::new(Span::styled("║", border_style)),
            Rect::new(x, y, 1, 1),
        );
    }
    x += 1;
    let size_header = format!("Size{}", size_arrow);
    if x < max_x {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("{:>width$} ", size_header, width = size_col as usize - 1),
                header_style,
            )),
            Rect::new(x, y, size_col.min(max_x - x), 1),
        );
    }
    x += size_col;
    if x < max_x {
        frame.render_widget(
            Paragraph::new(Span::styled("║", border_style)),
            Rect::new(x, y, 1, 1),
        );
    }
    x += 1;
    let date_header = format!(" Date{}", date_arrow);
    if x < max_x {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("{:<width$}", date_header, width = date_col as usize),
                header_style,
            )),
            Rect::new(x, y, date_col.min(max_x - x), 1),
        );
    }
    x += date_col;
    if x < max_x {
        frame.render_widget(
            Paragraph::new(Span::styled("║", border_style)),
            Rect::new(x, y, 1, 1),
        );
    }
    x += 1;
    if x < max_x {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("{:>width$} ", "Time", width = time_col as usize - 1),
                header_style,
            )),
            Rect::new(x, y, time_col.min(max_x - x), 1),
        );
    }
    x += time_col;
    // Right border for header row (only if within bounds)
    if x < max_x {
        frame.render_widget(
            Paragraph::new(Span::styled("║", border_style)),
            Rect::new(x, y, 1, 1),
        );
    }

    // ROW 2: Header underline with stats boxes start
    let y = area.y + 2;
    // Left side: stats box tops
    let file_count = app.file_count();
    let total_size = app.total_size();
    let left_line = "╔════╗        ╔═══════════╗   ".to_string();
    frame.render_widget(
        Paragraph::new(Span::styled(&left_line, border_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );
    // File table header underline
    let mut header_ul = String::new();
    header_ul.push('╠');
    header_ul.push_str(&"═".repeat(name_col as usize));
    header_ul.push('╬');
    header_ul.push_str(&"═".repeat(kind_col as usize));
    header_ul.push('╬');
    header_ul.push_str(&"═".repeat(size_col as usize));
    header_ul.push('╬');
    header_ul.push_str(&"═".repeat(date_col as usize));
    header_ul.push('╬');
    header_ul.push_str(&"═".repeat(time_col as usize));
    header_ul.push('╣');
    frame.render_widget(
        Paragraph::new(Span::styled(&header_ul, border_style)),
        Rect::new(file_table_x - 1, y, file_table_width + 1, 1),
    );

    // ROW 3: Files count row
    let y = area.y + 3;
    let files_line = format!(
        "║{:>4}║ Files  ║{:>11}║   ",
        file_count,
        format_number(total_size)
    );
    frame.render_widget(
        Paragraph::new(Span::styled(&files_line, border_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROW 4: Dirs row (close total size box)
    let y = area.y + 4;
    let dir_count = app.dir_count();
    let dirs_line = "╠════╣        ╚═══════════╝   ".to_string();
    frame.render_widget(
        Paragraph::new(Span::styled(&dirs_line, border_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROW 5: Dirs count
    let y = area.y + 5;
    let dirs_line2 = format!("║{:>4}║ Directories            ", dir_count);
    frame.render_widget(
        Paragraph::new(Span::styled(&dirs_line2, border_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROW 6: Tagged box top
    let y = area.y + 6;
    let tagged_count = app.tagged_files.len();
    let tagged_size = app.tagged_size();
    let tagged_line1 = "╠════╣        ╔═══════════╗   ".to_string();
    frame.render_widget(
        Paragraph::new(Span::styled(&tagged_line1, border_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROW 7: Tagged count row (white text like other stats)
    let y = area.y + 7;
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("║", border_style),
            Span::styled(
                format!("{:>4}", tagged_count),
                Style::default().fg(colors.fg()),
            ),
            Span::styled("║ ", border_style),
            Span::styled("Tagged", Style::default().fg(colors.fg())),
            Span::styled(" ║", border_style),
            Span::styled(
                format!("{:>11}", format_number(tagged_size)),
                Style::default().fg(colors.fg()),
            ),
            Span::styled("║   ", border_style),
        ])),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROW 8: Close both boxes
    let y = area.y + 8;
    let close_boxes = "╚════╝        ╚═══════════╝   ".to_string();
    frame.render_widget(
        Paragraph::new(Span::styled(&close_boxes, border_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROW 9: Single line divider
    let y = area.y + 9;
    let divider = "─".repeat((left_width as usize).saturating_sub(1));
    frame.render_widget(
        Paragraph::new(Span::styled(&divider, border_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROWS 10-17: Keybindings
    let white_style = Style::default().fg(colors.fg());
    let keybindings = [
        " F1- Help       F2- Status   ",
        " F3- Chg Drive  F4- Prev Dir ",
        " F5- Chg Dir    F6- DOS Cmd  ",
        " F7- Srch Spec  F8- Sort     ",
        " F9- Edit      F10- Quit     ",
        "F12- Proc      ⌃S- Config    ",
        " ⌃T- Color    SPC- Tag file  ",
        "   ESC- Abort Command        ",
    ];
    for (i, kb) in keybindings.iter().enumerate() {
        let y = area.y + 10 + i as u16;
        frame.render_widget(
            Paragraph::new(Span::styled(*kb, white_style)),
            Rect::new(area.x, y, left_width - 1, 1),
        );
    }

    // ROW 17: Another divider
    let y = area.y + 17;
    frame.render_widget(
        Paragraph::new(Span::styled(&divider, border_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROWS 18-19: Copyright (always shown)
    let blue_style = Style::default().fg(colors.blue());
    let y = area.y + 18;
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!(" R-DOS — Version {}       ", env!("CARGO_PKG_VERSION")),
            blue_style,
        )),
        Rect::new(area.x, y, left_width - 1, 1),
    );
    let y = area.y + 19;
    frame.render_widget(
        Paragraph::new(Span::styled(" Paul Thrasher - SF, CA      ", blue_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROWS 20-21: Git and Beads status indicators (only when applicable)
    // NOTE: These rows are only rendered if they fit within the area bounds
    let green_style = Style::default().fg(colors.green());
    let cyan_style = Style::default().fg(colors.cyan());
    let status_width = (left_width as usize).saturating_sub(1);
    let max_y = area.y + area.height; // Maximum valid y (exclusive)

    // Row 20: Git status (only if in git repo and row fits)
    // Format: " ↑0↓5 +3 !2 branch-name..."
    if let Some(ref info) = app.git_status_info {
        let y = area.y + 20;
        if y < max_y {
            // Build compact status: ↑ahead↓behind +staged !modified branch
            let mut parts = Vec::new();
            if info.ahead > 0 || info.behind > 0 {
                parts.push(format!("↑{}↓{}", info.ahead, info.behind));
            }
            if info.staged > 0 {
                parts.push(format!("+{}", info.staged));
            }
            if info.modified > 0 {
                parts.push(format!("!{}", info.modified));
            }
            let status_prefix = if parts.is_empty() {
                String::new()
            } else {
                format!("{} ", parts.join(" "))
            };
            // Calculate remaining space for branch name
            let prefix_len = status_prefix.len() + 1; // +1 for leading space
            let branch_space = status_width.saturating_sub(prefix_len);
            let branch_display = if info.branch.len() > branch_space {
                format!("{}…", &info.branch[..branch_space.saturating_sub(1)])
            } else {
                info.branch.clone()
            };
            let git_status = format!(" git: {}{}", status_prefix, branch_display);
            let padded = format!("{:<width$}", git_status, width = status_width);
            frame.render_widget(
                Paragraph::new(Span::styled(padded, cyan_style)),
                Rect::new(area.x, y, left_width - 1, 1),
            );
        }
    }

    // Row 21: Beads status (only if in beads project and row fits)
    // Format: " bd: ○19 ●3 ✓12" (open, in-progress, ready)
    if let Some(ref info) = app.beads_status_info {
        let y = area.y + 21;
        if y < max_y {
            let mut parts = Vec::new();
            if info.open > 0 {
                parts.push(format!("○{}", info.open));
            }
            if info.in_progress > 0 {
                parts.push(format!("●{}", info.in_progress));
            }
            if info.ready > 0 {
                parts.push(format!("✓{}", info.ready));
            }
            let status = if parts.is_empty() {
                " bd: ✓".to_string() // All clear
            } else {
                format!(" bd: {}", parts.join(" "))
            };
            let padded = format!("{:<width$}", status, width = status_width);
            frame.render_widget(
                Paragraph::new(Span::styled(padded, green_style)),
                Rect::new(area.x, y, left_width - 1, 1),
            );
        }
    }

    // Calculate file table dimensions
    let data_start_y = area.y + 3; // After top border, header, header underline
    let data_height = area.height.saturating_sub(4) as usize; // Leave room for bottom border
    let scroll_offset = calculate_scroll_offset(app.selected_index, app.scroll_offset, data_height);

    // Calculate right border x position
    let right_border_x =
        file_table_x + name_col + 1 + kind_col + 1 + size_col + 1 + date_col + 1 + time_col;

    // Draw all content rows (files + empty rows to fill height)
    for row_idx in 0..data_height {
        let row_y = data_start_y + row_idx as u16;
        let file_idx = scroll_offset + row_idx;

        // Draw left separator column
        frame.render_widget(
            Paragraph::new(Span::styled("║", border_style)),
            Rect::new(file_table_x - 1, row_y, 1, 1),
        );

        if file_idx < app.files.len() {
            let file = &app.files[file_idx];
            let is_selected = file_idx == app.selected_index;
            let is_tagged = app.is_tagged(&file.path);

            // Determine base style based on selection, tag, hidden, dir status
            let style = if is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else if is_tagged {
                Style::default().fg(colors.yellow())
            } else if file.is_hidden {
                Style::default().fg(colors.grey())
            } else if file.is_dir {
                Style::default().fg(colors.blue())
            } else {
                Style::default().fg(colors.fg())
            };

            // Git status indicator style
            let git_style = if is_selected {
                style
            } else {
                match file.git_status {
                    GitStatus::Modified => Style::default().fg(colors.yellow()),
                    GitStatus::Added => Style::default().fg(colors.cyan()),
                    GitStatus::Deleted => Style::default().fg(colors.red()),
                    GitStatus::Renamed => Style::default().fg(colors.cyan()),
                    GitStatus::Untracked => Style::default().fg(colors.magenta()),
                    GitStatus::Conflict => Style::default()
                        .fg(colors.red())
                        .add_modifier(Modifier::BOLD),
                    GitStatus::Ignored => Style::default().fg(colors.grey()),
                    GitStatus::None => style,
                }
            };

            let (sep_char, sep_style) = if is_selected {
                (" ", style)
            } else {
                ("║", border_style)
            };

            let name = if file.name == ".." {
                "..".to_string()
            } else if app.config.display.uppercase_names {
                file.name.to_uppercase()
            } else {
                file.name.clone()
            };
            let ext = if file.is_dir && file.name != ".." {
                String::new()
            } else if file.extension.is_empty() {
                String::new()
            } else if app.config.display.uppercase_names {
                format!(".{}", file.extension.to_uppercase())
            } else {
                format!(".{}", file.extension)
            };

            let display_name = format!("{}{}", name, ext);
            let git_indicator = file.git_status.indicator();

            let size_str = if file.is_dir {
                if file.name == ".." {
                    String::new()
                } else {
                    "<DIR>".to_string()
                }
            } else {
                format_size_short(file.size)
            };

            let kind_str = file.kind.as_str();

            let mut x = file_table_x;

            // File name with git status indicator on the right
            // Format: " NAME...           M" (name left-aligned, git status flush right)
            let name_width = name_col as usize - 2; // -2 for leading space and trailing git indicator
            let truncated_name = if display_name.len() > name_width {
                (&display_name[..name_width]).to_string()
            } else {
                format!("{:<width$}", display_name, width = name_width)
            };
            let name_content = Line::from(vec![
                Span::styled(" ", style),
                Span::styled(truncated_name, style),
                Span::styled(git_indicator, git_style),
            ]);
            let max_x = area.x + area.width;
            frame.render_widget(
                Paragraph::new(name_content),
                Rect::new(x, row_y, name_col.min(max_x.saturating_sub(x)), 1),
            );
            x += name_col;
            if x < max_x {
                frame.render_widget(
                    Paragraph::new(Span::styled(sep_char, sep_style)),
                    Rect::new(x, row_y, 1, 1),
                );
            }
            x += 1;

            // Kind column
            if x < max_x {
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        format!(" {:<width$}", kind_str, width = kind_col as usize - 1),
                        style,
                    )),
                    Rect::new(x, row_y, kind_col.min(max_x - x), 1),
                );
            }
            x += kind_col;
            if x < max_x {
                frame.render_widget(
                    Paragraph::new(Span::styled(sep_char, sep_style)),
                    Rect::new(x, row_y, 1, 1),
                );
            }
            x += 1;

            // Size (right-aligned with space on right)
            if x < max_x {
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        format!("{:>width$} ", size_str, width = size_col as usize - 1),
                        style,
                    )),
                    Rect::new(x, row_y, size_col.min(max_x - x), 1),
                );
            }
            x += size_col;
            if x < max_x {
                frame.render_widget(
                    Paragraph::new(Span::styled(sep_char, sep_style)),
                    Rect::new(x, row_y, 1, 1),
                );
            }
            x += 1;

            // Date
            if x < max_x {
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        format!(
                            " {:<width$}",
                            file.date_string(),
                            width = date_col as usize - 1
                        ),
                        style,
                    )),
                    Rect::new(x, row_y, date_col.min(max_x - x), 1),
                );
            }
            x += date_col;
            if x < max_x {
                frame.render_widget(
                    Paragraph::new(Span::styled(sep_char, sep_style)),
                    Rect::new(x, row_y, 1, 1),
                );
            }
            x += 1;

            // Time (right-aligned with space on right)
            if x < max_x {
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        format!(
                            "{:>width$} ",
                            file.time_string(),
                            width = time_col as usize - 1
                        ),
                        style,
                    )),
                    Rect::new(x, row_y, time_col.min(max_x - x), 1),
                );
            }
        } else {
            // Empty row - just draw column separators (with bounds checks)
            let max_x = area.x + area.width;
            let mut x = file_table_x + name_col;
            if x < max_x {
                frame.render_widget(
                    Paragraph::new(Span::styled("║", border_style)),
                    Rect::new(x, row_y, 1, 1),
                );
            }
            x += 1 + kind_col;
            if x < max_x {
                frame.render_widget(
                    Paragraph::new(Span::styled("║", border_style)),
                    Rect::new(x, row_y, 1, 1),
                );
            }
            x += 1 + size_col;
            if x < max_x {
                frame.render_widget(
                    Paragraph::new(Span::styled("║", border_style)),
                    Rect::new(x, row_y, 1, 1),
                );
            }
            x += 1 + date_col;
            if x < max_x {
                frame.render_widget(
                    Paragraph::new(Span::styled("║", border_style)),
                    Rect::new(x, row_y, 1, 1),
                );
            }
        }

        // Right border for all rows (only if within bounds)
        let max_x = area.x + area.width;
        if right_border_x < max_x {
            frame.render_widget(
                Paragraph::new(Span::styled("║", border_style)),
                Rect::new(right_border_x, row_y, 1, 1),
            );
        }
    }

    // Bottom border for file table
    let bottom_y = area.y.saturating_add(area.height.saturating_sub(1));
    let mut bottom_line = String::new();
    bottom_line.push_str(&" ".repeat((left_width as usize).saturating_sub(1)));
    bottom_line.push('╚');
    bottom_line.push_str(&"═".repeat(name_col as usize));
    bottom_line.push('╩');
    bottom_line.push_str(&"═".repeat(kind_col as usize));
    bottom_line.push('╩');
    bottom_line.push_str(&"═".repeat(size_col as usize));
    bottom_line.push('╩');
    bottom_line.push_str(&"═".repeat(date_col as usize));
    bottom_line.push('╩');
    bottom_line.push_str(&"═".repeat(time_col as usize));
    bottom_line.push('╝');
    frame.render_widget(
        Paragraph::new(Span::styled(&bottom_line, border_style)),
        Rect::new(area.x, bottom_y, area.width, 1),
    );
}

/// Calculate scroll offset to keep selection visible
fn calculate_scroll_offset(selected: usize, current_offset: usize, visible_height: usize) -> usize {
    if visible_height == 0 {
        return 0;
    }

    if selected < current_offset {
        selected
    } else if selected >= current_offset + visible_height {
        selected - visible_height + 1
    } else {
        current_offset
    }
}
