use crate::app::{App, Modal, NavItem, ShellCommandState};
use crate::file_ops::get_disk_space;
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
    // File table column widths - name column expands to fill remaining space
    let size_col: u16 = 12;
    let date_col: u16 = 10;
    let time_col: u16 = 9;
    // Calculate name_col to fill remaining width (area.width - left_width - size - date - time - 4 separators - 1 right border)
    let fixed_cols = left_width + size_col + date_col + time_col + 4 + 1;
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
    // File table headers
    let mut x = file_table_x;
    frame.render_widget(
        Paragraph::new(Span::styled(format!(" {:<width$}", "File Name", width = name_col as usize - 1), header_style)),
        Rect::new(x, y, name_col, 1),
    );
    x += name_col;
    frame.render_widget(Paragraph::new(Span::styled("║", border_style)), Rect::new(x, y, 1, 1));
    x += 1;
    frame.render_widget(
        Paragraph::new(Span::styled(format!("{:>width$} ", "Size", width = size_col as usize - 1), header_style)),
        Rect::new(x, y, size_col, 1),
    );
    x += size_col;
    frame.render_widget(Paragraph::new(Span::styled("║", border_style)), Rect::new(x, y, 1, 1));
    x += 1;
    frame.render_widget(
        Paragraph::new(Span::styled(format!(" {:<width$}", "Date", width = date_col as usize - 1), header_style)),
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
    let right_border_x = file_table_x + name_col + 1 + size_col + 1 + date_col + 1 + time_col;

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

            let style = if is_selected {
                Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)
            } else if is_tagged {
                Style::default().fg(COLOR_YELLOW)
            } else if file.is_dir {
                Style::default().fg(COLOR_BLUE)
            } else {
                Style::default().fg(COLOR_FG)
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

            let size_str = if file.is_dir {
                if file.name == ".." { String::new() } else { "<DIR>".to_string() }
            } else {
                format_number(file.size)
            };

            let mut x = file_table_x;

            // File name (name + ext combined)
            frame.render_widget(
                Paragraph::new(Span::styled(format!(" {:<width$}", display_name, width = name_col as usize - 1), style)),
                Rect::new(x, row_y, name_col, 1),
            );
            x += name_col;
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
    let modal_area = centered_rect(60, 50, area);

    // Clear the modal area
    frame.render_widget(Clear, modal_area);

    match &app.modal {
        Modal::Help => draw_help_modal(frame, modal_area),
        Modal::Status(info) => draw_status_modal(frame, modal_area, info),
        Modal::Quit => draw_quit_modal(frame, modal_area),
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

/// Draw quit confirmation modal
fn draw_quit_modal(frame: &mut Frame, area: Rect) {
    let quit_area = centered_rect(40, 20, area);

    let quit_block = Block::default()
        .title(" Quit ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_RED))
        .style(Style::default().bg(COLOR_BG));

    let quit_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Are you sure you want to quit?",
            Style::default().fg(COLOR_FG),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Y]", Style::default().fg(COLOR_BLUE)),
            Span::raw("es  "),
            Span::styled("[N]", Style::default().fg(COLOR_BLUE)),
            Span::raw("o"),
        ]),
    ];

    let paragraph = Paragraph::new(quit_text)
        .block(quit_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, quit_area);
    frame.render_widget(paragraph, quit_area);
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
