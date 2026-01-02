use crate::app::{App, Modal, NavItem, ShellCommandState, FileViewerState, ViewMode, ViewFilter, SortMode};
use crate::file_ops::{get_disk_space, GitStatus};
use humansize::{format_size, DECIMAL};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// QDOS color scheme - exact DOS 16-color palette RGB values
const COLOR_BG: Color = Color::Rgb(0, 0, 0);         // Black
const COLOR_FG: Color = Color::Rgb(255, 255, 255);   // White
const COLOR_BLUE: Color = Color::Rgb(102, 183, 179);     // DOS Blue - borders, menu items
const COLOR_GREEN: Color = Color::Rgb(103, 204, 77);    // DOS Green - help text, descriptions
const COLOR_RED: Color = Color::Rgb(157, 31, 20);      // DOS Red - path bar, selection bg
const COLOR_YELLOW: Color = Color::Rgb(232, 218, 89); // DOS Yellow - tagged items
const COLOR_GREY: Color = Color::Rgb(128, 128, 128);   // Grey - hidden files
const COLOR_CYAN: Color = Color::Rgb(0, 170, 170);     // Cyan - git added
const COLOR_MAGENTA: Color = Color::Rgb(170, 0, 170);  // Magenta - git untracked

/// Color usage:
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

    // Minimum size check - show message if too small (must check before drawing anything)
    let min_width: u16 = 80;
    let min_height: u16 = 25;

    if size.width < min_width || size.height < min_height {
        let msg = format!("Terminal too small. Need {}x{}, have {}x{}",
            min_width, min_height, size.width, size.height);
        frame.render_widget(
            Paragraph::new(Span::styled(msg, Style::default().fg(COLOR_RED))),
            size,
        );
        return;
    }

    // Main layout: nav bar at top, then content (no empty line after PATH)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Navigation bar + description
            Constraint::Length(1),  // Separator line above path
            Constraint::Length(1),  // Path bar
            Constraint::Min(10),    // Content area (directly after PATH)
        ])
        .split(size);

    // Draw navigation bar
    draw_nav_bar(frame, app, main_chunks[0]);

    // Draw separator line above path (double line white)
    draw_separator_line(frame, main_chunks[1]);

    // Draw path bar with red background on path text only
    draw_path_bar(frame, app, main_chunks[2]);

    // Draw integrated content area (stats panel + file table as one grid)
    draw_integrated_content(frame, app, main_chunks[3]);

    // Draw modal if active
    if app.modal != Modal::None {
        draw_modal(frame, app, size);
    }
}

/// Draw the navigation menu bar
fn draw_nav_bar(frame: &mut Frame, app: &App, area: Rect) {
    // First line: menu items - white text, selected is yellow on red
    let nav_items: Vec<Span> = NavItem::ALL
        .iter()
        .enumerate()
        .flat_map(|(i, item)| {
            let style = if i == app.nav_index {
                Style::default()
                    .fg(COLOR_YELLOW)
                    .bg(COLOR_RED)
            } else {
                Style::default().fg(COLOR_FG) // White text for unselected
            };

            vec![
                Span::styled(item.as_str(), style),
                Span::raw("  "),
            ]
        })
        .collect();

    let nav_line = Line::from(nav_items);

    // Second line: description in green (like original)
    let description = NavItem::ALL[app.nav_index].description();
    let desc_line = Line::from(Span::styled(description, Style::default().fg(COLOR_GREEN)));

    let nav_text = vec![nav_line, desc_line];
    let nav_paragraph = Paragraph::new(nav_text);
    frame.render_widget(nav_paragraph, area);
}

/// Draw separator line above path (double line white on black)
fn draw_separator_line(frame: &mut Frame, area: Rect) {
    let line_char = "═";
    let line = line_char.repeat(area.width as usize);
    let separator = Paragraph::new(Line::from(Span::styled(
        line,
        Style::default().fg(COLOR_FG),
    )));
    frame.render_widget(separator, area);
}

/// Draw the path bar - "PATH  >> " is white, only the path value is yellow on red (extending to far right)
fn draw_path_bar(frame: &mut Frame, app: &App, area: Rect) {
    let label = " PATH  >>  ";
    let path_str = format!("{}", app.current_path.display());
    // Pad the path value to fill remaining width
    let remaining_width = area.width.saturating_sub(label.len() as u16) as usize;
    let padded_path = format!("{:<width$}", path_str, width = remaining_width);

    let path_line = Line::from(vec![
        Span::styled(label, Style::default().fg(COLOR_FG)), // White label
        Span::styled(
            padded_path,
            Style::default().fg(COLOR_YELLOW).bg(COLOR_RED), // Yellow on red for path value
        ),
    ]);

    let path_paragraph = Paragraph::new(path_line);
    frame.render_widget(path_paragraph, area);
}

/// Draw integrated content area - stats panel and file table as one grid
/// This matches the original QDOS layout where panels share borders
fn draw_integrated_content(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = Style::default().fg(COLOR_FG);
    let header_style = Style::default().fg(COLOR_BLUE);

    // Left panel width (stats/keybindings/copyright)
    let left_width: u16 = 30;

    // Calculate dynamic size column width based on file sizes
    let max_size_width = app.files.iter()
        .filter(|f| !f.is_dir)
        .map(|f| format_number(f.size).len())
        .max()
        .unwrap_or(4); // "Size" header minimum
    let size_col: u16 = (max_size_width.max(4) + 2) as u16; // +2 for padding

    // File table column widths - name column expands to fill remaining space
    let kind_col: u16 = 6;  // "KIND" + padding
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
    top_line.push_str(&"═".repeat(left_width as usize - 1));
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
    let left_header = format!(" {:>5}  {:>13}   ", "Count", "Total Size");
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
        Paragraph::new(Span::styled(format!("{:<width$}", name_header, width = name_col as usize), header_style)),
        Rect::new(x, y, name_col, 1),
    );
    x += name_col;
    frame.render_widget(Paragraph::new(Span::styled("║", border_style)), Rect::new(x, y, 1, 1));
    x += 1;
    frame.render_widget(
        Paragraph::new(Span::styled(format!(" {:<width$}", "Kind", width = kind_col as usize - 1), header_style)),
        Rect::new(x, y, kind_col, 1),
    );
    x += kind_col;
    frame.render_widget(Paragraph::new(Span::styled("║", border_style)), Rect::new(x, y, 1, 1));
    x += 1;
    let size_header = format!("Size{}", size_arrow);
    frame.render_widget(
        Paragraph::new(Span::styled(format!("{:>width$} ", size_header, width = size_col as usize - 1), header_style)),
        Rect::new(x, y, size_col, 1),
    );
    x += size_col;
    frame.render_widget(Paragraph::new(Span::styled("║", border_style)), Rect::new(x, y, 1, 1));
    x += 1;
    let date_header = format!(" Date{}", date_arrow);
    frame.render_widget(
        Paragraph::new(Span::styled(format!("{:<width$}", date_header, width = date_col as usize), header_style)),
        Rect::new(x, y, date_col, 1),
    );
    x += date_col;
    frame.render_widget(Paragraph::new(Span::styled("║", border_style)), Rect::new(x, y, 1, 1));
    x += 1;
    frame.render_widget(
        Paragraph::new(Span::styled(format!("{:>width$} ", "Time", width = time_col as usize - 1), header_style)),
        Rect::new(x, y, time_col, 1),
    );
    x += time_col;
    frame.render_widget(Paragraph::new(Span::styled("║", border_style)), Rect::new(x, y, 1, 1));

    // ROW 2: Header underline with stats boxes start
    let y = area.y + 2;
    // Left side: stats box tops
    let file_count = app.file_count();
    let total_size = app.total_size();
    let left_line = format!("╔════╗        ╔═══════════╗   ");
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
    let files_line = format!("║{:>4}║ Files  ║{:>11}║   ", file_count, format_number(total_size));
    frame.render_widget(
        Paragraph::new(Span::styled(&files_line, border_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROW 4: Dirs row (close total size box)
    let y = area.y + 4;
    let dir_count = app.dir_count();
    let dirs_line = format!("╠════╣        ╚═══════════╝   ");
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
    let tagged_line1 = format!("╠════╣        ╔═══════════╗   ");
    frame.render_widget(
        Paragraph::new(Span::styled(&tagged_line1, border_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROW 7: Tagged count row (white text like other stats)
    let y = area.y + 7;
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("║", border_style),
            Span::styled(format!("{:>4}", tagged_count), Style::default().fg(COLOR_FG)),
            Span::styled("║ ", border_style),
            Span::styled("Tagged", Style::default().fg(COLOR_FG)),
            Span::styled(" ║", border_style),
            Span::styled(format!("{:>11}", format_number(tagged_size)), Style::default().fg(COLOR_FG)),
            Span::styled("║   ", border_style),
        ])),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROW 8: Close both boxes
    let y = area.y + 8;
    let close_boxes = format!("╚════╝        ╚═══════════╝   ");
    frame.render_widget(
        Paragraph::new(Span::styled(&close_boxes, border_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROW 9: Single line divider
    let y = area.y + 9;
    let divider = "─".repeat(left_width as usize - 1);
    frame.render_widget(
        Paragraph::new(Span::styled(&divider, border_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // ROWS 10-17: Keybindings
    let white_style = Style::default().fg(COLOR_FG);
    let keybindings = [
        " F1- Help       F2- Status   ",
        " F3- Chg Drive  F4- Prev Dir ",
        " F5- Chg Dir    F6- DOS Cmd  ",
        " F7- Srch Spec  F8- Sort     ",
        " F9- Edit      F10- Quit     ",
        "   SPACE BAR- Tag file       ",
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

    // ROWS 18-20: Copyright
    let blue_style = Style::default().fg(COLOR_BLUE);
    let y = area.y + 18;
    frame.render_widget(
        Paragraph::new(Span::styled(" R-DOS — Version 0.1.0       ", blue_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );
    let y = area.y + 19;
    frame.render_widget(
        Paragraph::new(Span::styled("   Rust remake of Q-DOS II   ", blue_style)),
        Rect::new(area.x, y, left_width - 1, 1),
    );

    // Calculate file table dimensions
    let data_start_y = area.y + 3; // After top border, header, header underline
    let data_height = area.height.saturating_sub(4) as usize; // Leave room for bottom border
    let scroll_offset = calculate_scroll_offset(app.selected_index, app.scroll_offset, data_height);

    // Calculate right border x position
    let right_border_x = file_table_x + name_col + 1 + kind_col + 1 + size_col + 1 + date_col + 1 + time_col;

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
                Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)
            } else if is_tagged {
                Style::default().fg(COLOR_YELLOW)
            } else if file.is_hidden {
                Style::default().fg(COLOR_GREY)
            } else if file.is_dir {
                Style::default().fg(COLOR_BLUE)
            } else {
                Style::default().fg(COLOR_FG)
            };

            // Git status indicator style
            let git_style = if is_selected {
                style
            } else {
                match file.git_status {
                    GitStatus::Modified => Style::default().fg(COLOR_YELLOW),
                    GitStatus::Added => Style::default().fg(COLOR_CYAN),
                    GitStatus::Deleted => Style::default().fg(COLOR_RED),
                    GitStatus::Renamed => Style::default().fg(COLOR_CYAN),
                    GitStatus::Untracked => Style::default().fg(COLOR_MAGENTA),
                    GitStatus::Conflict => Style::default().fg(COLOR_RED).add_modifier(Modifier::BOLD),
                    GitStatus::Ignored => Style::default().fg(COLOR_GREY),
                    GitStatus::None => style,
                }
            };

            let (sep_char, sep_style) = if is_selected {
                (" ", style)
            } else {
                ("║", border_style)
            };

            let name = if file.name == ".." { "..".to_string() } else { file.name.to_uppercase() };
            let ext = if file.is_dir && file.name != ".." {
                String::new()
            } else if file.extension.is_empty() {
                String::new()
            } else {
                format!(".{}", file.extension)
            };

            let display_name = format!("{}{}", name, ext);
            let git_indicator = file.git_status.indicator();

            let size_str = if file.is_dir {
                if file.name == ".." { String::new() } else { "<DIR>".to_string() }
            } else {
                format_number(file.size)
            };

            let kind_str = file.kind.as_str();

            let mut x = file_table_x;

            // File name with git status indicator on the right
            // Format: " NAME...           M" (name left-aligned, git status flush right)
            let name_width = name_col as usize - 2; // -2 for leading space and trailing git indicator
            let truncated_name = if display_name.len() > name_width {
                format!("{}", &display_name[..name_width])
            } else {
                format!("{:<width$}", display_name, width = name_width)
            };
            let name_content = Line::from(vec![
                Span::styled(" ", style),
                Span::styled(truncated_name, style),
                Span::styled(git_indicator, git_style),
            ]);
            frame.render_widget(
                Paragraph::new(name_content),
                Rect::new(x, row_y, name_col, 1),
            );
            x += name_col;
            frame.render_widget(Paragraph::new(Span::styled(sep_char, sep_style)), Rect::new(x, row_y, 1, 1));
            x += 1;

            // Kind column
            frame.render_widget(
                Paragraph::new(Span::styled(format!(" {:<width$}", kind_str, width = kind_col as usize - 1), style)),
                Rect::new(x, row_y, kind_col, 1),
            );
            x += kind_col;
            frame.render_widget(Paragraph::new(Span::styled(sep_char, sep_style)), Rect::new(x, row_y, 1, 1));
            x += 1;

            // Size (right-aligned with space on right)
            frame.render_widget(
                Paragraph::new(Span::styled(format!("{:>width$} ", size_str, width = size_col as usize - 1), style)),
                Rect::new(x, row_y, size_col, 1),
            );
            x += size_col;
            frame.render_widget(Paragraph::new(Span::styled(sep_char, sep_style)), Rect::new(x, row_y, 1, 1));
            x += 1;

            // Date
            frame.render_widget(
                Paragraph::new(Span::styled(format!(" {:<width$}", file.date_string(), width = date_col as usize - 1), style)),
                Rect::new(x, row_y, date_col, 1),
            );
            x += date_col;
            frame.render_widget(Paragraph::new(Span::styled(sep_char, sep_style)), Rect::new(x, row_y, 1, 1));
            x += 1;

            // Time (right-aligned with space on right)
            frame.render_widget(
                Paragraph::new(Span::styled(format!("{:>width$} ", file.time_string(), width = time_col as usize - 1), style)),
                Rect::new(x, row_y, time_col, 1),
            );
        } else {
            // Empty row - just draw column separators
            let mut x = file_table_x + name_col;
            frame.render_widget(Paragraph::new(Span::styled("║", border_style)), Rect::new(x, row_y, 1, 1));
            x += 1 + kind_col;
            frame.render_widget(Paragraph::new(Span::styled("║", border_style)), Rect::new(x, row_y, 1, 1));
            x += 1 + size_col;
            frame.render_widget(Paragraph::new(Span::styled("║", border_style)), Rect::new(x, row_y, 1, 1));
            x += 1 + date_col;
            frame.render_widget(Paragraph::new(Span::styled("║", border_style)), Rect::new(x, row_y, 1, 1));
        }

        // Right border for all rows
        frame.render_widget(Paragraph::new(Span::styled("║", border_style)), Rect::new(right_border_x, row_y, 1, 1));
    }

    // Bottom border for file table
    let bottom_y = area.y + area.height - 1;
    let mut bottom_line = String::new();
    bottom_line.push_str(&" ".repeat(left_width as usize - 1));
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

/// Draw modal dialogs
fn draw_modal(frame: &mut Frame, app: &App, area: Rect) {
    // Quit modal handles its own area - no Clear needed (overlays directly)
    if matches!(app.modal, Modal::Quit) {
        draw_quit_modal(frame, area);
        return;
    }

    let modal_area = centered_rect(60, 50, area);

    // Clear the modal area for other modals
    frame.render_widget(Clear, modal_area);

    match &app.modal {
        Modal::Help => draw_help_modal(frame, modal_area),
        Modal::Status(info) => draw_status_modal(frame, modal_area, info),
        Modal::Quit => {} // Handled above
        Modal::SearchSpec => draw_search_spec_modal(frame, modal_area, app),
        Modal::Space => draw_space_modal(frame, modal_area, app),
        Modal::Error(msg) => draw_error_modal(frame, modal_area, msg),
        Modal::Success(msg) => draw_success_modal(frame, modal_area, msg),
        Modal::PathInput(path) => draw_path_input_modal(frame, modal_area, path),
        Modal::CopyTo(dest) => draw_copy_modal(frame, modal_area, dest, app),
        Modal::MoveTo(dest) => draw_move_modal(frame, modal_area, dest, app),
        Modal::EraseConfirm => draw_erase_modal(frame, modal_area, app),
        Modal::RenameInput(name) => draw_rename_modal(frame, modal_area, name),
        Modal::ShellCommand(state) => draw_shell_command(frame, area, state, app),
        Modal::FileViewer(state) => draw_file_viewer(frame, area, state),
        Modal::None => {}
    }
}

/// Draw help modal
fn draw_help_modal(frame: &mut Frame, area: Rect) {
    let help_block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled("R-DOS File Manager", Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Navigation:", Style::default().fg(COLOR_BLUE))),
        Line::from("  ↑/↓ or j/k    - Move selection"),
        Line::from("  ←/→ or h/l    - Navigate menu"),
        Line::from("  Enter         - Open directory / Execute action"),
        Line::from("  PgUp/PgDn     - Scroll page"),
        Line::from("  Home/End      - Jump to start/end"),
        Line::from(""),
        Line::from(Span::styled("Actions:", Style::default().fg(COLOR_BLUE))),
        Line::from("  Space         - Tag/untag file"),
        Line::from("  F8            - Cycle sort mode"),
        Line::from("  F10 or q      - Quit"),
        Line::from(""),
        Line::from(Span::styled("Press any key to close", Style::default().fg(COLOR_GREEN))),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(help_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw status modal
fn draw_status_modal(frame: &mut Frame, area: Rect, info: &crate::file_ops::SystemInfo) {
    let status_block = Block::default()
        .title(" System Status ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let label_style = Style::default().fg(COLOR_GREEN);
    let value_style = Style::default().fg(COLOR_FG);

    let status_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Hostname: ", label_style),
            Span::styled(&info.hostname, value_style),
        ]),
        Line::from(vec![
            Span::styled("OS: ", label_style),
            Span::styled(format!("{} {}", info.os_name, info.os_version), value_style),
        ]),
        Line::from(vec![
            Span::styled("CPUs: ", label_style),
            Span::styled(format!("{}", info.cpu_count), value_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Total Memory: ", label_style),
            Span::styled(format_size(info.total_memory, DECIMAL), value_style),
        ]),
        Line::from(vec![
            Span::styled("Used Memory: ", label_style),
            Span::styled(format_size(info.used_memory, DECIMAL), value_style),
        ]),
        Line::from(vec![
            Span::styled("Total Swap: ", label_style),
            Span::styled(format_size(info.total_swap, DECIMAL), value_style),
        ]),
        Line::from(vec![
            Span::styled("Used Swap: ", label_style),
            Span::styled(format_size(info.used_swap, DECIMAL), value_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("Press any key to close", Style::default().fg(COLOR_GREEN))),
    ];

    let paragraph = Paragraph::new(status_text)
        .block(status_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw quit confirmation modal (Q-DOS II style with double-line box)
fn draw_quit_modal(frame: &mut Frame, area: Rect) {
    // Modal is 60 chars wide, 8 lines tall (matching spec/ui.md)
    let width: u16 = 60;
    let height: u16 = 8;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let quit_area = Rect::new(x, y, width.min(area.width), height.min(area.height));

    let style = Style::default().fg(COLOR_FG).bg(COLOR_BG);

    // Build the modal line by line with double-line box characters
    // Line 0: Top border
    let top = format!("╔{}╗", "═".repeat(width as usize - 2));
    frame.render_widget(Paragraph::new(Span::styled(&top, style)), Rect::new(quit_area.x, quit_area.y, width, 1));

    // Line 1: Title row
    let title = "F10 - Quit Q-DOS II";
    let title_line = format!("║{:^w$}║", title, w = width as usize - 2);
    frame.render_widget(Paragraph::new(Span::styled(&title_line, style)), Rect::new(quit_area.x, quit_area.y + 1, width, 1));

    // Line 2: Separator (double line)
    let sep = format!("╠{}╣", "═".repeat(width as usize - 2));
    frame.render_widget(Paragraph::new(Span::styled(&sep, style)), Rect::new(quit_area.x, quit_area.y + 2, width, 1));

    // Line 3: Empty row
    let empty = format!("║{:w$}║", "", w = width as usize - 2);
    frame.render_widget(Paragraph::new(Span::styled(&empty, style)), Rect::new(quit_area.x, quit_area.y + 3, width, 1));

    // Line 4: First message
    let msg1 = "Press F10 again to quit, or RETURN for options";
    let msg1_line = format!("║{:^w$}║", msg1, w = width as usize - 2);
    frame.render_widget(Paragraph::new(Span::styled(&msg1_line, style)), Rect::new(quit_area.x, quit_area.y + 4, width, 1));

    // Line 5: Empty row
    frame.render_widget(Paragraph::new(Span::styled(&empty, style)), Rect::new(quit_area.x, quit_area.y + 5, width, 1));

    // Line 6: Second message
    let msg2 = "Press ESC to return to Q-DOS II";
    let msg2_line = format!("║{:^w$}║", msg2, w = width as usize - 2);
    frame.render_widget(Paragraph::new(Span::styled(&msg2_line, style)), Rect::new(quit_area.x, quit_area.y + 6, width, 1));

    // Line 7: Bottom border
    let bottom = format!("╚{}╝", "═".repeat(width as usize - 2));
    frame.render_widget(Paragraph::new(Span::styled(&bottom, style)), Rect::new(quit_area.x, quit_area.y + 7, width, 1));
}

/// Draw search specification modal
fn draw_search_spec_modal(frame: &mut Frame, area: Rect, app: &App) {
    let search_block = Block::default()
        .title(" Search Specification ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let search_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Current spec: ", Style::default().fg(COLOR_GREEN)),
            Span::styled(&app.search_spec, Style::default().fg(COLOR_FG)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Search specification not yet implemented",
            Style::default().fg(COLOR_GREEN),
        )),
        Line::from(""),
        Line::from(Span::styled("Press any key to close", Style::default().fg(COLOR_GREEN))),
    ];

    let paragraph = Paragraph::new(search_text)
        .block(search_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw disk space modal
fn draw_space_modal(frame: &mut Frame, area: Rect, app: &App) {
    let space_block = Block::default()
        .title(" Disk Space ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let (available, total) = get_disk_space(&app.current_path).unwrap_or((0, 0));
    let used = total.saturating_sub(available);
    let percent = if total > 0 {
        (used as f64 / total as f64 * 100.0) as u64
    } else {
        0
    };

    let space_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Total: ", Style::default().fg(COLOR_GREEN)),
            Span::styled(format_size(total, DECIMAL), Style::default().fg(COLOR_FG)),
        ]),
        Line::from(vec![
            Span::styled("Used: ", Style::default().fg(COLOR_GREEN)),
            Span::styled(
                format!("{} ({}%)", format_size(used, DECIMAL), percent),
                Style::default().fg(COLOR_FG),
            ),
        ]),
        Line::from(vec![
            Span::styled("Available: ", Style::default().fg(COLOR_GREEN)),
            Span::styled(format_size(available, DECIMAL), Style::default().fg(COLOR_BLUE)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Press any key to close", Style::default().fg(COLOR_GREEN))),
    ];

    let paragraph = Paragraph::new(space_text)
        .block(space_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw error modal
fn draw_error_modal(frame: &mut Frame, area: Rect, message: &str) {
    let error_area = centered_rect(50, 25, area);

    let error_block = Block::default()
        .title(" Error ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_RED))
        .style(Style::default().bg(COLOR_BG));

    let error_text = vec![
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(COLOR_FG))),
        Line::from(""),
        Line::from(Span::styled("Press any key to close", Style::default().fg(COLOR_GREEN))),
    ];

    let paragraph = Paragraph::new(error_text)
        .block(error_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, error_area);
    frame.render_widget(paragraph, error_area);
}

/// Draw path input modal
fn draw_path_input_modal(frame: &mut Frame, area: Rect, path: &str) {
    let input_area = centered_rect(70, 30, area);

    let input_block = Block::default()
        .title(" Change Directory ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let input_text = vec![
        Line::from(""),
        Line::from(Span::styled("Enter path (Tab to complete):", Style::default().fg(COLOR_GREEN))),
        Line::from(""),
        Line::from(Span::styled(format!("{}_", path), Style::default().fg(COLOR_FG))),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(COLOR_BLUE)),
            Span::raw(" complete, "),
            Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
            Span::raw(" confirm, "),
            Span::styled("Esc", Style::default().fg(COLOR_BLUE)),
            Span::raw(" cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(input_text)
        .block(input_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, input_area);
    frame.render_widget(paragraph, input_area);
}

/// Draw success modal
fn draw_success_modal(frame: &mut Frame, area: Rect, message: &str) {
    let success_area = centered_rect(50, 25, area);

    let success_block = Block::default()
        .title(" Success ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_GREEN))
        .style(Style::default().bg(COLOR_BG));

    let success_text = vec![
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(COLOR_FG))),
        Line::from(""),
        Line::from(Span::styled("Press any key to close", Style::default().fg(COLOR_GREEN))),
    ];

    let paragraph = Paragraph::new(success_text)
        .block(success_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, success_area);
    frame.render_widget(paragraph, success_area);
}

/// Draw copy modal
fn draw_copy_modal(frame: &mut Frame, area: Rect, dest: &str, app: &App) {
    let copy_area = centered_rect(70, 35, area);

    let copy_block = Block::default()
        .title(" Copy Files ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let copy_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Copying {} tagged file(s)", app.tagged_files.len()),
            Style::default().fg(COLOR_YELLOW),
        )),
        Line::from(""),
        Line::from(Span::styled("Destination (Tab to complete):", Style::default().fg(COLOR_GREEN))),
        Line::from(""),
        Line::from(Span::styled(format!("{}_", dest), Style::default().fg(COLOR_FG))),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(COLOR_BLUE)),
            Span::raw(" complete, "),
            Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
            Span::raw(" copy, "),
            Span::styled("Esc", Style::default().fg(COLOR_BLUE)),
            Span::raw(" cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(copy_text)
        .block(copy_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, copy_area);
    frame.render_widget(paragraph, copy_area);
}

/// Draw move modal
fn draw_move_modal(frame: &mut Frame, area: Rect, dest: &str, app: &App) {
    let move_area = centered_rect(70, 35, area);

    let move_block = Block::default()
        .title(" Move Files ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let move_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Moving {} tagged file(s)", app.tagged_files.len()),
            Style::default().fg(COLOR_YELLOW),
        )),
        Line::from(""),
        Line::from(Span::styled("Destination (Tab to complete):", Style::default().fg(COLOR_GREEN))),
        Line::from(""),
        Line::from(Span::styled(format!("{}_", dest), Style::default().fg(COLOR_FG))),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(COLOR_BLUE)),
            Span::raw(" complete, "),
            Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
            Span::raw(" move, "),
            Span::styled("Esc", Style::default().fg(COLOR_BLUE)),
            Span::raw(" cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(move_text)
        .block(move_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, move_area);
    frame.render_widget(paragraph, move_area);
}

/// Draw erase confirmation modal
fn draw_erase_modal(frame: &mut Frame, area: Rect, app: &App) {
    let erase_area = centered_rect(50, 30, area);

    let erase_block = Block::default()
        .title(" Erase Files ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_RED))
        .style(Style::default().bg(COLOR_BG));

    let erase_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Delete {} tagged file(s)?", app.tagged_files.len()),
            Style::default().fg(COLOR_YELLOW),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "This cannot be undone!",
            Style::default().fg(COLOR_RED),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Y]", Style::default().fg(COLOR_BLUE)),
            Span::raw("es  "),
            Span::styled("[N]", Style::default().fg(COLOR_BLUE)),
            Span::raw("o"),
        ]),
    ];

    let paragraph = Paragraph::new(erase_text)
        .block(erase_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, erase_area);
    frame.render_widget(paragraph, erase_area);
}

/// Draw rename modal
fn draw_rename_modal(frame: &mut Frame, area: Rect, name: &str) {
    let rename_area = centered_rect(60, 30, area);

    let rename_block = Block::default()
        .title(" Rename File ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let rename_text = vec![
        Line::from(""),
        Line::from(Span::styled("Enter new name:", Style::default().fg(COLOR_GREEN))),
        Line::from(""),
        Line::from(Span::styled(format!("{}_", name), Style::default().fg(COLOR_FG))),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
            Span::raw(" rename, "),
            Span::styled("Esc", Style::default().fg(COLOR_BLUE)),
            Span::raw(" cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(rename_text)
        .block(rename_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, rename_area);
    frame.render_widget(paragraph, rename_area);
}

/// Draw file viewer screen (full screen)
fn draw_file_viewer(frame: &mut Frame, area: Rect, state: &FileViewerState) {
    // Clear the entire screen
    frame.render_widget(Clear, area);

    // Layout: title bar, separator, content, separator, status/help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Title bar with file name and mode
            Constraint::Length(1),  // Separator
            Constraint::Min(5),     // Content area
            Constraint::Length(1),  // Separator
            Constraint::Length(1),  // Status/help line
        ])
        .split(area);

    // Title bar: file name and mode/filter info
    let mode_str = match state.mode {
        ViewMode::Normal => "NORMAL",
        ViewMode::Hex => "HEX",
        ViewMode::Image => "IMAGE",
        ViewMode::Markdown => "MARKDOWN",
    };
    let filter_str = match state.filter {
        ViewFilter::Off => "",
        ViewFilter::Ascii => " [Filter: ASCII]",
        ViewFilter::WordStar => " [Filter: W/S]",
    };
    let title = format!(" VIEW: {}  Mode: {}{}", state.file_name.to_uppercase(), mode_str, filter_str);
    frame.render_widget(
        Paragraph::new(Span::styled(title, Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD))),
        chunks[0],
    );

    // Separator
    let sep = "═".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(COLOR_FG))),
        chunks[1],
    );

    // Content area
    let content_height = chunks[2].height as usize;
    match state.mode {
        ViewMode::Normal => draw_normal_view(frame, chunks[2], state, content_height),
        ViewMode::Hex => draw_hex_view(frame, chunks[2], state, content_height),
        ViewMode::Image => draw_image_view(frame, chunks[2], state),
        ViewMode::Markdown => draw_markdown_view(frame, chunks[2], state, content_height),
    }

    // Separator
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(COLOR_FG))),
        chunks[3],
    );

    // Help line
    let help_spans = vec![
        Span::styled(" H", Style::default().fg(COLOR_BLUE)),
        Span::raw("ex "),
        Span::styled("N", Style::default().fg(COLOR_BLUE)),
        Span::raw("ormal "),
        Span::styled("I", Style::default().fg(COLOR_BLUE)),
        Span::raw("mage "),
        Span::styled("M", Style::default().fg(COLOR_BLUE)),
        Span::raw("arkdown "),
        Span::styled("F", Style::default().fg(COLOR_BLUE)),
        Span::raw("ilter "),
        Span::styled("↑↓", Style::default().fg(COLOR_BLUE)),
        Span::raw(" scroll "),
        Span::styled("Esc", Style::default().fg(COLOR_BLUE)),
        Span::raw(" exit"),
    ];
    frame.render_widget(
        Paragraph::new(Line::from(help_spans)),
        chunks[4],
    );
}

/// Draw normal/ASCII view mode
fn draw_normal_view(frame: &mut Frame, area: Rect, state: &FileViewerState, height: usize) {
    // Convert content to lines based on filter
    let lines: Vec<String> = state.content
        .split(|&b| b == b'\n')
        .map(|line| {
            line.iter()
                .map(|&b| {
                    match state.filter {
                        ViewFilter::Off => {
                            if b >= 32 && b < 127 {
                                b as char
                            } else if b == b'\t' {
                                ' '
                            } else if b == b'\r' {
                                ' '
                            } else {
                                '.'
                            }
                        }
                        ViewFilter::Ascii => {
                            if b >= 32 && b < 127 {
                                b as char
                            } else {
                                ' '
                            }
                        }
                        ViewFilter::WordStar => {
                            let b = b & 0x7F; // Strip high bit
                            if b >= 32 && b < 127 {
                                b as char
                            } else {
                                ' '
                            }
                        }
                    }
                })
                .collect::<String>()
        })
        .collect();

    // Calculate max scroll
    let max_scroll = lines.len().saturating_sub(height);
    let scroll = state.scroll_offset.min(max_scroll);

    // Render visible lines
    let visible_lines: Vec<Line> = lines
        .iter()
        .skip(scroll)
        .take(height)
        .map(|line| Line::from(Span::styled(format!(" {}", line), Style::default().fg(COLOR_FG))))
        .collect();

    frame.render_widget(
        Paragraph::new(visible_lines),
        area,
    );
}

/// Draw hex view mode
fn draw_hex_view(frame: &mut Frame, area: Rect, state: &FileViewerState, height: usize) {
    let bytes_per_line: usize = 16;
    let total_lines = (state.content.len() + bytes_per_line - 1) / bytes_per_line;

    // Calculate max scroll
    let max_scroll = total_lines.saturating_sub(height);
    let scroll = state.scroll_offset.min(max_scroll);

    let mut lines: Vec<Line> = Vec::new();

    for line_idx in scroll..(scroll + height).min(total_lines) {
        let offset = line_idx * bytes_per_line;
        let end = (offset + bytes_per_line).min(state.content.len());
        let chunk = &state.content[offset..end];

        // Build the hex line
        let mut spans = Vec::new();

        // Offset (8 hex digits)
        spans.push(Span::styled(
            format!(" {:08X}  ", offset),
            Style::default().fg(COLOR_BLUE),
        ));

        // Hex bytes (two groups of 8)
        for (i, &byte) in chunk.iter().enumerate() {
            if i == 8 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                format!("{:02X} ", byte),
                Style::default().fg(COLOR_FG),
            ));
        }

        // Pad if less than 16 bytes
        for i in chunk.len()..bytes_per_line {
            if i == 8 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::raw("   "));
        }

        // ASCII representation
        spans.push(Span::raw("  "));
        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if b >= 32 && b < 127 {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        spans.push(Span::styled(ascii, Style::default().fg(COLOR_GREEN)));

        lines.push(Line::from(spans));
    }

    frame.render_widget(
        Paragraph::new(lines),
        area,
    );
}

/// Draw shell command screen (full screen)
fn draw_shell_command(frame: &mut Frame, area: Rect, state: &ShellCommandState, app: &App) {
    // Clear the entire screen
    frame.render_widget(Clear, area);

    // Layout: title, separator, working dir, input, separator, output, separator, help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Title
            Constraint::Length(1),  // Separator
            Constraint::Length(1),  // Working directory
            Constraint::Length(1),  // Empty
            Constraint::Length(1),  // Input prompt
            Constraint::Length(1),  // Separator
            Constraint::Min(5),     // Output area
            Constraint::Length(1),  // Separator
            Constraint::Length(1),  // Help line
        ])
        .split(area);

    // Title (centered)
    let title = "R-DOS Shell Command";
    let padding = (area.width as usize).saturating_sub(title.len()) / 2;
    let title_line = format!("{:>width$}{}", "", title, width = padding);
    frame.render_widget(
        Paragraph::new(Span::styled(title_line, Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD))),
        chunks[0],
    );

    // Separator
    let sep = "═".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(COLOR_FG))),
        chunks[1],
    );

    // Working directory
    let wd_line = format!(" Working Directory: {}", app.current_path.display());
    frame.render_widget(
        Paragraph::new(Span::styled(wd_line, Style::default().fg(COLOR_GREEN))),
        chunks[2],
    );

    // Input prompt with cursor
    let input_line = format!(" $ {}_", state.input);
    frame.render_widget(
        Paragraph::new(Span::styled(input_line, Style::default().fg(COLOR_FG))),
        chunks[4],
    );

    // Separator
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(COLOR_FG))),
        chunks[5],
    );

    // Output area
    let output_height = chunks[6].height as usize;
    let visible_lines: Vec<Line> = state.output
        .iter()
        .skip(state.scroll_offset)
        .take(output_height)
        .map(|line| {
            let style = if line.starts_with("stderr:") {
                Style::default().fg(COLOR_RED)
            } else {
                Style::default().fg(COLOR_FG)
            };
            Line::from(Span::styled(format!(" {}", line), style))
        })
        .collect();

    // Show exit code at bottom if command completed
    let mut output_lines = visible_lines;
    if let Some(code) = state.exit_code {
        if output_lines.len() < output_height {
            output_lines.push(Line::from(""));
            let exit_style = if code == 0 {
                Style::default().fg(COLOR_GREEN)
            } else {
                Style::default().fg(COLOR_RED)
            };
            output_lines.push(Line::from(Span::styled(
                format!(" [Exit code: {}]", code),
                exit_style,
            )));
        }
    }

    frame.render_widget(
        Paragraph::new(output_lines),
        chunks[6],
    );

    // Separator
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(COLOR_FG))),
        chunks[7],
    );

    // Help line
    let help_spans = vec![
        Span::styled(" Enter", Style::default().fg(COLOR_BLUE)),
        Span::raw(" run, "),
        Span::styled("↑/↓", Style::default().fg(COLOR_BLUE)),
        Span::raw(" history, "),
        Span::styled("PgUp/PgDn", Style::default().fg(COLOR_BLUE)),
        Span::raw(" scroll, "),
        Span::styled("Tab", Style::default().fg(COLOR_BLUE)),
        Span::raw(" complete, "),
        Span::styled("Esc", Style::default().fg(COLOR_BLUE)),
        Span::raw(" exit"),
    ];
    frame.render_widget(
        Paragraph::new(Line::from(help_spans)),
        chunks[8],
    );
}

/// Draw image view mode
fn draw_image_view(frame: &mut Frame, area: Rect, state: &FileViewerState) {
    // Try to load and display the image
    match image::load_from_memory(&state.content) {
        Ok(img) => {
            // Get the image dimensions
            let img_width = img.width();
            let img_height = img.height();

            // Calculate the aspect ratio and fit to area
            // Terminal characters are typically 2:1 height:width ratio
            let term_aspect = (area.width as f64) / (area.height as f64 * 2.0);
            let img_aspect = img_width as f64 / img_height as f64;

            let (display_width, display_height) = if img_aspect > term_aspect {
                // Image is wider than area
                let w = area.width as u32;
                let h = ((w as f64 / img_aspect) / 2.0).max(1.0) as u32;
                (w, h.min(area.height as u32))
            } else {
                // Image is taller than area
                let h = area.height as u32;
                let w = (h as f64 * img_aspect * 2.0).max(1.0) as u32;
                (w.min(area.width as u32), h)
            };

            // Center the image in the area
            let x_offset = (area.width.saturating_sub(display_width as u16)) / 2;
            let y_offset = (area.height.saturating_sub(display_height as u16)) / 2;

            // Convert image to RGBA and resize
            let rgba_img = img.to_rgba8();
            let resized = image::imageops::resize(
                &rgba_img,
                display_width,
                display_height,
                image::imageops::FilterType::Triangle,
            );

            // Render as half-block characters (▀ upper half, ▄ lower half)
            // Each character cell represents 2 vertical pixels
            let mut lines: Vec<Line> = Vec::new();

            for y in (0..display_height).step_by(2) {
                let mut spans: Vec<Span> = Vec::new();

                // Add left padding
                if x_offset > 0 {
                    spans.push(Span::raw(" ".repeat(x_offset as usize)));
                }

                for x in 0..display_width {
                    let top_pixel = resized.get_pixel(x, y);
                    let bottom_pixel = if y + 1 < display_height {
                        resized.get_pixel(x, y + 1)
                    } else {
                        top_pixel
                    };

                    // Use half-block character with top color as foreground, bottom as background
                    let fg = Color::Rgb(top_pixel[0], top_pixel[1], top_pixel[2]);
                    let bg = Color::Rgb(bottom_pixel[0], bottom_pixel[1], bottom_pixel[2]);

                    spans.push(Span::styled("▀", Style::default().fg(fg).bg(bg)));
                }

                lines.push(Line::from(spans));
            }

            // Add top padding
            let mut padded_lines: Vec<Line> = Vec::new();
            for _ in 0..y_offset {
                padded_lines.push(Line::from(""));
            }
            padded_lines.extend(lines);

            frame.render_widget(
                Paragraph::new(padded_lines),
                area,
            );
        }
        Err(e) => {
            // Show error if image can't be loaded
            let error_msg = vec![
                Line::from(""),
                Line::from(Span::styled(
                    " Cannot display image",
                    Style::default().fg(COLOR_RED).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!(" Error: {}", e),
                    Style::default().fg(COLOR_FG),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!(" File: {}", state.file_path.display()),
                    Style::default().fg(COLOR_GREEN),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    " Press N for normal view or H for hex view",
                    Style::default().fg(COLOR_BLUE),
                )),
            ];
            frame.render_widget(
                Paragraph::new(error_msg),
                area,
            );
        }
    }
}

/// Draw markdown view mode
fn draw_markdown_view(frame: &mut Frame, area: Rect, state: &FileViewerState, height: usize) {
    // Convert content to string
    let content_str = String::from_utf8_lossy(&state.content);

    // Parse and render markdown manually (simplified version)
    // termimad is primarily for printing directly; we'll do a simplified render for ratatui
    let mut lines: Vec<Line> = Vec::new();

    for raw_line in content_str.lines() {
        let line = raw_line;

        // Headers
        if line.starts_with("# ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", &line[2..]),
                Style::default().fg(COLOR_BLUE).add_modifier(Modifier::BOLD),
            )));
        } else if line.starts_with("## ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", &line[3..]),
                Style::default().fg(COLOR_BLUE).add_modifier(Modifier::BOLD),
            )));
        } else if line.starts_with("### ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", &line[4..]),
                Style::default().fg(COLOR_BLUE),
            )));
        } else if line.starts_with("#### ") || line.starts_with("##### ") || line.starts_with("###### ") {
            let header_content = line.trim_start_matches('#').trim_start();
            lines.push(Line::from(Span::styled(
                format!(" {}", header_content),
                Style::default().fg(COLOR_BLUE),
            )));
        }
        // Code blocks
        else if line.starts_with("```") {
            lines.push(Line::from(Span::styled(
                " ───────────────────────────────────",
                Style::default().fg(COLOR_GREEN),
            )));
        }
        // Bullet points
        else if line.starts_with("- ") || line.starts_with("* ") {
            lines.push(Line::from(Span::styled(
                format!("  • {}", &line[2..]),
                Style::default().fg(COLOR_FG),
            )));
        }
        // Numbered lists
        else if line.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
            && line.contains(". ")
        {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(COLOR_FG),
            )));
        }
        // Blockquotes
        else if line.starts_with("> ") {
            lines.push(Line::from(Span::styled(
                format!(" │ {}", &line[2..]),
                Style::default().fg(COLOR_GREEN),
            )));
        }
        // Horizontal rules
        else if line == "---" || line == "***" || line == "___" {
            lines.push(Line::from(Span::styled(
                " ════════════════════════════════════",
                Style::default().fg(COLOR_FG),
            )));
        }
        // Links and emphasis (simplified - just show as-is with color hints)
        else if line.contains("**") || line.contains("__") {
            // Bold text - simple approach, just highlight the whole line
            let clean_line = line.replace("**", "").replace("__", "");
            lines.push(Line::from(Span::styled(
                format!(" {}", clean_line),
                Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
            )));
        } else if line.contains('*') || line.contains('_') {
            // Italic text - show with different color
            let clean_line = line
                .chars()
                .filter(|&c| c != '*' && c != '_')
                .collect::<String>();
            lines.push(Line::from(Span::styled(
                format!(" {}", clean_line),
                Style::default().fg(COLOR_FG).add_modifier(Modifier::ITALIC),
            )));
        }
        // Code inline
        else if line.contains('`') {
            lines.push(Line::from(Span::styled(
                format!(" {}", line),
                Style::default().fg(COLOR_GREEN),
            )));
        }
        // Regular text
        else if line.trim().is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(Span::styled(
                format!(" {}", line),
                Style::default().fg(COLOR_FG),
            )));
        }
    }

    // Calculate max scroll
    let max_scroll = lines.len().saturating_sub(height);
    let scroll = state.scroll_offset.min(max_scroll);

    // Render visible lines
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll)
        .take(height)
        .collect();

    frame.render_widget(
        Paragraph::new(visible_lines),
        area,
    );
}

/// Create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
