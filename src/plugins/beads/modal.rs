//! Beads modal drawing

use crate::app::App;
use crate::plugins::beads::{BeadsMenuItem, BeadsState, BeadsView};
use crate::ui::components::FullScreenView;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Draw Beads modal
pub fn draw_beads_modal(frame: &mut Frame, area: Rect, state: &BeadsState, app: &App) {
    let colors = app.colors();

    // Title based on current view
    let title = match state.view {
        BeadsView::Menu => " BEADS ISSUE TRACKER ",
        BeadsView::List => " BEADS - ALL ISSUES ",
        BeadsView::Ready => " BEADS - READY TO WORK ",
        BeadsView::Blocked => " BEADS - BLOCKED ISSUES ",
        BeadsView::Epics => " BEADS - EPICS ",
        BeadsView::Stats => " BEADS - STATISTICS ",
        BeadsView::Create => " BEADS - CREATE ISSUE ",
        BeadsView::Detail => " BEADS - ISSUE DETAIL ",
        BeadsView::Edit => " BEADS - EDIT ISSUE ",
        BeadsView::Comments => " BEADS - COMMENTS ",
        BeadsView::History => " BEADS - ISSUE HISTORY ",
        BeadsView::FileIssues => " BEADS - FILE ISSUES ",
        BeadsView::Dependencies => " BEADS - DEPENDENCY GRAPH ",
        BeadsView::Kanban => " BEADS - KANBAN BOARD ",
        BeadsView::Human => " BEADS - COMMAND HELP ",
        BeadsView::Doctor => " BEADS - HEALTH CHECK ",
    };

    // Create full screen view
    let view = FullScreenView::new(area, title, &colors);
    view.render_frame(frame);

    // Content area
    let content_area = view.content_area();

    if !state.is_beads_project {
        // Not a beads project
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Not a Beads project",
                Style::default().fg(colors.yellow()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Initialize beads with 'bd init'",
                Style::default().fg(colors.grey()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press any key to close",
                Style::default().fg(colors.green()),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), content_area);
    } else {
        match state.view {
            BeadsView::Menu => {
                let mut lines = vec![Line::from("")];

                let items = BeadsMenuItem::items(state.is_beads_project);
                for (i, item) in items.iter().enumerate() {
                    let is_selected = i == state.menu_selected;
                    let style = if is_selected {
                        Style::default().fg(colors.yellow()).bg(colors.red())
                    } else {
                        Style::default().fg(colors.fg())
                    };

                    let number = format!("{}. ", i + 1);

                    lines.push(Line::from(vec![
                        Span::styled("  ", style),
                        Span::styled(
                            number,
                            if is_selected {
                                style
                            } else {
                                Style::default().fg(colors.blue())
                            },
                        ),
                        Span::styled(format!("{:<12}", item.as_str()), style),
                        Span::styled(
                            item.description(),
                            if is_selected {
                                style
                            } else {
                                Style::default().fg(colors.grey())
                            },
                        ),
                    ]));
                }

                // Show top epics after menu items (navigable)
                let menu_count = items.len();
                if !state.top_epics.is_empty() {
                    lines.push(Line::from("")); // blank line separator
                    for (i, epic) in state.top_epics.iter().take(5).enumerate() {
                        let epic_idx = menu_count + i;
                        let is_selected = epic_idx == state.menu_selected;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        // Truncate title to fit
                        let max_title = content_area.width.saturating_sub(20) as usize;
                        let title = if epic.title.len() > max_title {
                            format!("{}…", &epic.title[..max_title.saturating_sub(1)])
                        } else {
                            epic.title.clone()
                        };

                        lines.push(Line::from(vec![
                            Span::styled("  ", style),
                            Span::styled(
                                "◆ ",
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.magenta())
                                },
                            ),
                            Span::styled(
                                format!("{:<12}", epic.id),
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.grey())
                                },
                            ),
                            Span::styled(title, style),
                        ]));
                    }
                }

                // Show recent issues if we have any
                if !state.recent_issues.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  Recent:",
                        Style::default()
                            .fg(colors.cyan())
                            .add_modifier(Modifier::BOLD),
                    )));
                    for issue in state.recent_issues.iter().take(5) {
                        let status_icon = match issue.status.as_str() {
                            "in_progress" => "●",
                            "open" => "○",
                            _ => "✓",
                        };
                        let status_color = match issue.status.as_str() {
                            "in_progress" => colors.yellow(),
                            "open" => colors.blue(),
                            _ => colors.green(),
                        };
                        // Truncate title to fit
                        let max_title = content_area.width.saturating_sub(22) as usize;
                        let title = if issue.title.len() > max_title {
                            format!("{}…", &issue.title[..max_title.saturating_sub(1)])
                        } else {
                            issue.title.clone()
                        };
                        lines.push(Line::from(vec![
                            Span::styled("    ", Style::default()),
                            Span::styled(status_icon, Style::default().fg(status_color)),
                            Span::styled(" ", Style::default()),
                            Span::styled(
                                format!("{:<12}", issue.id),
                                Style::default().fg(colors.grey()),
                            ),
                            Span::styled(title, Style::default().fg(colors.fg())),
                        ]));
                    }
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::List | BeadsView::Ready | BeadsView::Blocked | BeadsView::Epics => {
                // Account for search bar and header
                let search_height = if state.search_active || !state.search_query.is_empty() {
                    1
                } else {
                    0
                };
                let visible_height =
                    content_area.height.saturating_sub(1 + search_height as u16) as usize;
                let mut lines: Vec<Line> = vec![];

                // Show search bar if active or has query
                if state.search_active || !state.search_query.is_empty() {
                    let search_style = if state.search_active {
                        Style::default().fg(colors.yellow())
                    } else {
                        Style::default().fg(colors.blue())
                    };
                    let prompt = if state.search_active { "/" } else { "🔍" };
                    lines.push(Line::from(vec![
                        Span::styled(format!("{} ", prompt), search_style),
                        Span::styled(&state.search_query, search_style),
                        if state.search_active {
                            Span::styled("█", search_style)
                        } else {
                            Span::raw("")
                        },
                    ]));
                }

                // Filter issues based on search query
                let query_lower = state.search_query.to_lowercase();
                let filtered_issues: Vec<_> = if state.search_query.is_empty() {
                    state.issues.iter().collect()
                } else {
                    state
                        .issues
                        .iter()
                        .filter(|i| {
                            i.id.to_lowercase().contains(&query_lower)
                                || i.title.to_lowercase().contains(&query_lower)
                                || i.issue_type.to_lowercase().contains(&query_lower)
                                || i.status.to_lowercase().contains(&query_lower)
                        })
                        .collect()
                };

                if filtered_issues.is_empty() {
                    lines.push(Line::from(""));
                    let msg = if !state.search_query.is_empty() {
                        "No matching issues"
                    } else {
                        "No issues found"
                    };
                    lines.push(Line::from(Span::styled(
                        msg,
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    // Table header
                    let header_style = Style::default()
                        .fg(colors.blue())
                        .add_modifier(Modifier::BOLD);
                    // Column widths: ID=14, Type=8, Status=12, Pri=3, Title=rest
                    let id_w = 14;
                    let type_w = 8;
                    let status_w = 12;
                    let pri_w = 3;
                    let fixed_width = id_w + type_w + status_w + pri_w + 5; // +5 for spacing
                    let title_w = content_area
                        .width
                        .saturating_sub(fixed_width as u16)
                        .max(10) as usize;

                    lines.push(Line::from(vec![
                        Span::styled(format!(" {:<id_w$}", "ID"), header_style),
                        Span::styled(format!("{:<type_w$}", "TYPE"), header_style),
                        Span::styled(format!("{:<status_w$}", "STATUS"), header_style),
                        Span::styled(format!("{:<pri_w$}", "P"), header_style),
                        Span::styled(format!("{:<title_w$}", "TITLE"), header_style),
                    ]));

                    for (i, issue) in filtered_issues
                        .iter()
                        .skip(state.scroll_offset)
                        .enumerate()
                        .take(visible_height)
                    {
                        let is_selected = state.scroll_offset + i == state.selected_issue;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        let priority_style = if is_selected {
                            style
                        } else {
                            match issue.priority.as_str() {
                                "0" | "P0" => Style::default().fg(colors.red()),
                                "1" | "P1" => Style::default().fg(colors.yellow()),
                                _ => Style::default().fg(colors.grey()),
                            }
                        };

                        let type_style = if is_selected {
                            style
                        } else {
                            match issue.issue_type.as_str() {
                                "bug" => Style::default().fg(colors.red()),
                                "feature" => Style::default().fg(colors.green()),
                                _ => Style::default().fg(colors.grey()),
                            }
                        };

                        let status_style = if is_selected {
                            style
                        } else {
                            match issue.status.as_str() {
                                "open" => Style::default().fg(colors.green()),
                                "in_progress" => Style::default().fg(colors.yellow()),
                                "closed" => Style::default().fg(colors.grey()),
                                _ => Style::default().fg(colors.fg()),
                            }
                        };

                        let id_short = if issue.id.len() > id_w {
                            format!("{}…", &issue.id[..id_w - 1])
                        } else {
                            issue.id.clone()
                        };

                        let type_short = if issue.issue_type.len() > type_w {
                            format!("{}…", &issue.issue_type[..type_w - 1])
                        } else {
                            issue.issue_type.clone()
                        };

                        let status_short = if issue.status.len() > status_w {
                            format!("{}…", &issue.status[..status_w - 1])
                        } else {
                            issue.status.clone()
                        };

                        let pri = issue.priority.chars().last().unwrap_or('2');

                        // For epics, show progress bar
                        let is_epic = issue.issue_type == "epic";
                        let progress_str = if is_epic && !issue.dependents.is_empty() {
                            let total = issue.dependents.len();
                            let closed = issue
                                .dependents
                                .iter()
                                .filter(|d| d.status == "closed")
                                .count();
                            let pct = if total > 0 { (closed * 100) / total } else { 0 };
                            // Create progress bar: [████░░░░] 4/6
                            let bar_width = 8;
                            let filled = (pct * bar_width) / 100;
                            let empty = bar_width - filled;
                            format!(
                                " [{}{}] {}/{}",
                                "█".repeat(filled),
                                "░".repeat(empty),
                                closed,
                                total
                            )
                        } else {
                            String::new()
                        };

                        let progress_len = progress_str.len();
                        let available_title_w = title_w.saturating_sub(progress_len);
                        let title = if issue.title.len() > available_title_w {
                            format!("{}…", &issue.title[..available_title_w.saturating_sub(1)])
                        } else {
                            issue.title.clone()
                        };

                        if is_epic && !issue.dependents.is_empty() {
                            let closed_count = issue
                                .dependents
                                .iter()
                                .filter(|d| d.status == "closed")
                                .count();
                            let total = issue.dependents.len();
                            let pct = (closed_count * 100) / total.max(1);
                            let progress_color = if is_selected {
                                style
                            } else if pct == 100 {
                                Style::default().fg(colors.green())
                            } else if pct >= 50 {
                                Style::default().fg(colors.yellow())
                            } else {
                                Style::default().fg(colors.grey())
                            };

                            lines.push(Line::from(vec![
                                Span::styled(format!(" {:<id_w$}", id_short), style),
                                Span::styled(format!("{:<type_w$}", type_short), type_style),
                                Span::styled(format!("{:<status_w$}", status_short), status_style),
                                Span::styled(format!("{:<pri_w$}", pri), priority_style),
                                Span::styled(title, style),
                                Span::styled(progress_str, progress_color),
                            ]));
                        } else {
                            let title = if issue.title.len() > title_w {
                                format!("{}…", &issue.title[..title_w.saturating_sub(1)])
                            } else {
                                issue.title.clone()
                            };
                            lines.push(Line::from(vec![
                                Span::styled(format!(" {:<id_w$}", id_short), style),
                                Span::styled(format!("{:<type_w$}", type_short), type_style),
                                Span::styled(format!("{:<status_w$}", status_short), status_style),
                                Span::styled(format!("{:<pri_w$}", pri), priority_style),
                                Span::styled(format!("{:<title_w$}", title), style),
                            ]));
                        }
                    }
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Stats => {
                let lines = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  Total Issues:   ", Style::default().fg(colors.green())),
                        Span::styled(
                            state.stats.total.to_string(),
                            Style::default().fg(colors.fg()),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  Open:           ", Style::default().fg(colors.green())),
                        Span::styled(
                            state.stats.open.to_string(),
                            Style::default().fg(colors.fg()),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("  In Progress:    ", Style::default().fg(colors.yellow())),
                        Span::styled(
                            state.stats.in_progress.to_string(),
                            Style::default().fg(colors.fg()),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("  Blocked:        ", Style::default().fg(colors.red())),
                        Span::styled(
                            state.stats.blocked.to_string(),
                            Style::default().fg(colors.fg()),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("  Closed:         ", Style::default().fg(colors.grey())),
                        Span::styled(
                            state.stats.closed.to_string(),
                            Style::default().fg(colors.fg()),
                        ),
                    ]),
                ];

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Create => {
                let issue_types = ["task", "bug", "feature"];
                let priorities = ["P0", "P1", "P2", "P3", "P4"];
                let visible_height = content_area.height as usize;
                let label_width = 15; // "  Description: " length
                let max_text_width =
                    content_area.width.saturating_sub(label_width as u16 + 2) as usize; // -2 for cursor

                // Helper to wrap text into lines
                let wrap_text = |text: &str, width: usize| -> Vec<String> {
                    if text.is_empty() {
                        return vec![String::new()];
                    }
                    let mut wrapped = Vec::new();
                    for line in text.lines() {
                        if line.is_empty() {
                            wrapped.push(String::new());
                            continue;
                        }
                        let mut current = String::new();
                        for word in line.split_inclusive(char::is_whitespace) {
                            if current.len() + word.len() <= width {
                                current.push_str(word);
                            } else if current.is_empty() {
                                // Word is longer than width, break it
                                for chunk in word.chars().collect::<Vec<_>>().chunks(width.max(1)) {
                                    wrapped.push(chunk.iter().collect());
                                }
                            } else {
                                wrapped.push(current.trim_end().to_string());
                                current = word.to_string();
                            }
                        }
                        if !current.is_empty() {
                            wrapped.push(current.trim_end().to_string());
                        }
                    }
                    if wrapped.is_empty() {
                        wrapped.push(String::new());
                    }
                    wrapped
                };

                let mut lines: Vec<Line> = vec![];

                // Header
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Create New Issue",
                    Style::default()
                        .fg(colors.fg())
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));

                // Track line indices for scroll targeting
                let title_start_line = lines.len();

                // Title field (field 0) - with wrapping
                let title_style = if state.create_field == 0 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                let title_wrapped = wrap_text(&state.create_title, max_text_width);
                let title_len = title_wrapped.len();
                for (i, line_text) in title_wrapped.into_iter().enumerate() {
                    let is_last = i == title_len - 1;
                    let label = if i == 0 {
                        "  Title:       "
                    } else {
                        "               "
                    };
                    lines.push(Line::from(vec![
                        Span::styled(label, Style::default().fg(colors.green())),
                        Span::styled(line_text, title_style),
                        if state.create_field == 0 && is_last {
                            Span::styled("█", title_style)
                        } else {
                            Span::styled("", Style::default())
                        },
                    ]));
                }

                lines.push(Line::from(""));
                let desc_start_line = lines.len();

                // Description field (field 1) - with wrapping
                let desc_style = if state.create_field == 1 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                if state.create_description.is_empty() {
                    let placeholder = if state.create_field == 1 {
                        ""
                    } else {
                        "(optional)"
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  Description: ", Style::default().fg(colors.green())),
                        Span::styled(placeholder, desc_style),
                        if state.create_field == 1 {
                            Span::styled("█", desc_style)
                        } else {
                            Span::styled("", Style::default())
                        },
                    ]));
                } else {
                    let desc_wrapped = wrap_text(&state.create_description, max_text_width);
                    let desc_len = desc_wrapped.len();
                    for (i, line_text) in desc_wrapped.into_iter().enumerate() {
                        let is_last = i == desc_len - 1;
                        let label = if i == 0 {
                            "  Description: "
                        } else {
                            "               "
                        };
                        lines.push(Line::from(vec![
                            Span::styled(label, Style::default().fg(colors.green())),
                            Span::styled(line_text, desc_style),
                            if state.create_field == 1 && is_last {
                                Span::styled("█", desc_style)
                            } else {
                                Span::styled("", Style::default())
                            },
                        ]));
                    }
                }

                lines.push(Line::from(""));
                let type_line = lines.len();

                // Type field (field 2)
                let type_style = if state.create_field == 2 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                let type_value = format!("< {} >", issue_types[state.create_type]);
                lines.push(Line::from(vec![
                    Span::styled("  Type:        ", Style::default().fg(colors.green())),
                    Span::styled(type_value, type_style),
                ]));

                lines.push(Line::from(""));
                let priority_line = lines.len();

                // Priority field (field 3)
                let priority_style = if state.create_field == 3 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                let priority_value = format!("< {} >", priorities[state.create_priority]);
                lines.push(Line::from(vec![
                    Span::styled("  Priority:    ", Style::default().fg(colors.green())),
                    Span::styled(priority_value, priority_style),
                ]));

                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Tab: next field | ↑↓/←→: change value | Enter: submit",
                    Style::default().fg(colors.cyan()),
                )));

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                // Calculate target line for current field to ensure visibility
                let target_line = match state.create_field {
                    0 => title_start_line,
                    1 => desc_start_line,
                    2 => type_line,
                    3 => priority_line,
                    _ => 0,
                };

                // Calculate scroll to keep current field visible
                let total_lines = lines.len();
                let scroll = if total_lines <= visible_height {
                    0
                } else {
                    let max_scroll = total_lines.saturating_sub(visible_height);
                    // Ensure target line is visible
                    if target_line < state.create_scroll {
                        target_line
                    } else if target_line >= state.create_scroll + visible_height {
                        (target_line + 1)
                            .saturating_sub(visible_height)
                            .min(max_scroll)
                    } else {
                        state.create_scroll.min(max_scroll)
                    }
                };

                // Apply scrolling
                let visible_lines: Vec<Line> = lines
                    .into_iter()
                    .skip(scroll)
                    .take(visible_height)
                    .collect();

                // Show scroll indicator if content extends beyond view
                if total_lines > visible_height {
                    let indicator = format!(
                        " [{}/{}] ",
                        scroll + 1,
                        total_lines.saturating_sub(visible_height) + 1
                    );
                    let indicator_len = indicator.len() as u16;
                    let indicator_x =
                        content_area.x + content_area.width.saturating_sub(indicator_len + 1);
                    frame.render_widget(
                        Paragraph::new(Span::styled(indicator, Style::default().fg(colors.grey()))),
                        Rect::new(indicator_x, content_area.y, indicator_len + 1, 1),
                    );
                }

                frame.render_widget(Paragraph::new(visible_lines), content_area);
            }
            BeadsView::Detail => {
                let mut lines = vec![];

                // Use detail_issue if available, otherwise fall back to issues list
                let issue = state
                    .detail_issue
                    .as_ref()
                    .or_else(|| state.issues.get(state.selected_issue));

                if let Some(issue) = issue {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("  ID:       ", Style::default().fg(colors.green())),
                        Span::styled(&issue.id, Style::default().fg(colors.fg())),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("  Title:    ", Style::default().fg(colors.green())),
                        Span::styled(&issue.title, Style::default().fg(colors.fg())),
                    ]));

                    // Description if available (with text wrapping)
                    if let Some(ref desc) = issue.description {
                        if !desc.is_empty() {
                            lines.push(Line::from(""));
                            lines.push(Line::from(Span::styled(
                                "  Description:",
                                Style::default().fg(colors.green()),
                            )));
                            // Wrap text to fit content area (subtract 4 for indent)
                            let max_width = (content_area.width as usize).saturating_sub(6);
                            // Show all lines (no truncation)
                            for paragraph in desc.lines() {
                                // Simple word wrap
                                let words: Vec<&str> = paragraph.split_whitespace().collect();
                                if words.is_empty() {
                                    lines.push(Line::from(""));
                                    continue;
                                }
                                let mut current_line = String::new();
                                for word in words {
                                    if current_line.is_empty() {
                                        current_line = word.to_string();
                                    } else if current_line.len() + 1 + word.len() <= max_width {
                                        current_line.push(' ');
                                        current_line.push_str(word);
                                    } else {
                                        // Emit current line and start new one
                                        lines.push(Line::from(vec![
                                            Span::styled("    ", Style::default()),
                                            Span::styled(
                                                current_line.clone(),
                                                Style::default().fg(colors.grey()),
                                            ),
                                        ]));
                                        current_line = word.to_string();
                                    }
                                }
                                if !current_line.is_empty() {
                                    lines.push(Line::from(vec![
                                        Span::styled("    ", Style::default()),
                                        Span::styled(
                                            current_line.clone(),
                                            Style::default().fg(colors.grey()),
                                        ),
                                    ]));
                                }
                            }
                        }
                    }

                    lines.push(Line::from(""));

                    // Status with color
                    let status_color = match issue.status.as_str() {
                        "open" => colors.green(),
                        "in_progress" => colors.yellow(),
                        "closed" => colors.grey(),
                        _ => colors.fg(),
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  Status:   ", Style::default().fg(colors.green())),
                        Span::styled(&issue.status, Style::default().fg(status_color)),
                    ]));

                    // Type with color
                    let type_color = match issue.issue_type.as_str() {
                        "bug" => colors.red(),
                        "feature" => colors.green(),
                        "epic" => colors.cyan(),
                        _ => colors.fg(),
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  Type:     ", Style::default().fg(colors.green())),
                        Span::styled(&issue.issue_type, Style::default().fg(type_color)),
                    ]));

                    // Priority with color
                    let priority_color = match issue.priority.as_str() {
                        "0" | "P0" => colors.red(),
                        "1" | "P1" => colors.yellow(),
                        _ => colors.fg(),
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  Priority: ", Style::default().fg(colors.green())),
                        Span::styled(&issue.priority, Style::default().fg(priority_color)),
                    ]));

                    // Show blocked by if any
                    if !issue.blocked_by.is_empty() {
                        lines.push(Line::from(""));
                        lines.push(Line::from(vec![
                            Span::styled(
                                "  Blocked by: ",
                                Style::default()
                                    .fg(colors.red())
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                issue.blocked_by.join(", "),
                                Style::default().fg(colors.red()),
                            ),
                        ]));
                    }

                    // Show subtasks for epics
                    if !issue.dependents.is_empty() {
                        lines.push(Line::from(""));
                        let closed_count = issue
                            .dependents
                            .iter()
                            .filter(|d| d.status == "closed")
                            .count();
                        let total_count = issue.dependents.len();
                        lines.push(Line::from(Span::styled(
                            format!("  ─── Subtasks ({}/{}) ───", closed_count, total_count),
                            Style::default()
                                .fg(colors.cyan())
                                .add_modifier(Modifier::BOLD),
                        )));

                        for (i, subtask) in issue.dependents.iter().enumerate() {
                            let is_selected = i == state.selected_subtask;
                            let prefix = if is_selected { "▶ " } else { "  " };

                            let status_char = match subtask.status.as_str() {
                                "closed" => "✓",
                                "in_progress" => "◆",
                                _ => "○",
                            };
                            let status_color = match subtask.status.as_str() {
                                "closed" => colors.grey(),
                                "in_progress" => colors.yellow(),
                                _ => colors.green(),
                            };

                            let bg = if is_selected {
                                colors.red()
                            } else {
                                Color::Reset
                            };

                            lines.push(Line::from(vec![
                                Span::styled(
                                    format!("  {}", prefix),
                                    Style::default().fg(colors.yellow()).bg(bg),
                                ),
                                Span::styled(
                                    format!("{} ", status_char),
                                    Style::default().fg(status_color).bg(bg),
                                ),
                                Span::styled(
                                    format!("{} ", subtask.id),
                                    Style::default().fg(colors.blue()).bg(bg),
                                ),
                                Span::styled(
                                    &subtask.title,
                                    Style::default().fg(colors.fg()).bg(bg),
                                ),
                            ]));
                        }
                    }

                    // Show comments if any
                    if !issue.comments.is_empty() {
                        lines.push(Line::from(""));
                        lines.push(Line::from(Span::styled(
                            format!("  ─── Comments ({}) ───", issue.comments.len()),
                            Style::default()
                                .fg(colors.magenta())
                                .add_modifier(Modifier::BOLD),
                        )));

                        for comment in issue.comments.iter().take(3) {
                            lines.push(Line::from(vec![
                                Span::styled(
                                    format!("  {} ", comment.author),
                                    Style::default().fg(colors.blue()),
                                ),
                                Span::styled(
                                    &comment.created_at[..10], // Just date
                                    Style::default().fg(colors.grey()),
                                ),
                            ]));
                            // Truncate comment text if too long
                            let text = if comment.text.len() > 60 {
                                format!("{}...", &comment.text[..57])
                            } else {
                                comment.text.clone()
                            };
                            lines.push(Line::from(vec![
                                Span::styled("    ", Style::default()),
                                Span::styled(text, Style::default().fg(colors.fg())),
                            ]));
                        }
                        if issue.comments.len() > 3 {
                            lines.push(Line::from(Span::styled(
                                format!("    ... and {} more", issue.comments.len() - 3),
                                Style::default().fg(colors.grey()),
                            )));
                        }
                    }

                    // Actions section
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  ─── Actions ───",
                        Style::default()
                            .fg(colors.blue())
                            .add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));

                    // Show available actions based on status
                    if issue.status == "open" {
                        lines.push(Line::from(vec![
                            Span::styled("  [S] ", Style::default().fg(colors.yellow())),
                            Span::styled("Start working", Style::default().fg(colors.fg())),
                        ]));
                    }
                    if issue.status == "in_progress" || issue.status == "open" {
                        lines.push(Line::from(vec![
                            Span::styled("  [C] ", Style::default().fg(colors.yellow())),
                            Span::styled("Close issue", Style::default().fg(colors.fg())),
                        ]));
                    }
                    if issue.status == "closed" {
                        lines.push(Line::from(vec![
                            Span::styled("  [O] ", Style::default().fg(colors.yellow())),
                            Span::styled("Reopen issue", Style::default().fg(colors.fg())),
                        ]));
                    }
                    // Edit action always available
                    lines.push(Line::from(vec![
                        Span::styled("  [E] ", Style::default().fg(colors.yellow())),
                        Span::styled("Edit issue", Style::default().fg(colors.fg())),
                    ]));
                    // Subtask creation for epics
                    if issue.issue_type == "epic" {
                        lines.push(Line::from(vec![
                            Span::styled("  [N] ", Style::default().fg(colors.yellow())),
                            Span::styled("New subtask", Style::default().fg(colors.fg())),
                        ]));
                    }
                } else {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Issue not found",
                        Style::default().fg(colors.red()),
                    )));
                }

                // Apply scrolling - skip detail_scroll lines and limit to visible height
                let visible_height = content_area.height as usize;
                let total_lines = lines.len();
                let scroll = state
                    .detail_scroll
                    .min(total_lines.saturating_sub(visible_height));
                let visible_lines: Vec<Line> = lines
                    .into_iter()
                    .skip(scroll)
                    .take(visible_height)
                    .collect();

                // Show scroll indicator if content extends beyond view
                if total_lines > visible_height {
                    let indicator = format!(
                        " [{}/{}] ↑↓ scroll ",
                        scroll + 1,
                        total_lines.saturating_sub(visible_height) + 1
                    );
                    let indicator_len = indicator.len() as u16;
                    let indicator_x =
                        content_area.x + content_area.width.saturating_sub(indicator_len + 1);
                    frame.render_widget(
                        Paragraph::new(Span::styled(indicator, Style::default().fg(colors.grey()))),
                        Rect::new(indicator_x, content_area.y, indicator_len + 1, 1),
                    );
                }

                frame.render_widget(Paragraph::new(visible_lines), content_area);
            }
            BeadsView::Edit => {
                let statuses = ["open", "in_progress", "closed"];
                let priorities = ["P0", "P1", "P2", "P3", "P4"];

                let mut lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("Edit Issue: {}", state.edit_issue_id),
                        Style::default()
                            .fg(colors.fg())
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                ];

                // Title field
                let title_style = if state.edit_field == 0 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                lines.push(Line::from(vec![
                    Span::styled("  Title:       ", Style::default().fg(colors.green())),
                    Span::styled(&state.edit_title, title_style),
                    if state.edit_field == 0 {
                        Span::styled("█", title_style)
                    } else {
                        Span::styled("", Style::default())
                    },
                ]));

                lines.push(Line::from(""));

                // Description field
                let desc_style = if state.edit_field == 1 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                let desc_display = if state.edit_description.len() > 40 {
                    format!("{}...", &state.edit_description[..37])
                } else {
                    state.edit_description.clone()
                };
                lines.push(Line::from(vec![
                    Span::styled("  Description: ", Style::default().fg(colors.green())),
                    Span::styled(&desc_display, desc_style),
                    if state.edit_field == 1 {
                        Span::styled("█", desc_style)
                    } else {
                        Span::styled("", Style::default())
                    },
                ]));

                lines.push(Line::from(""));

                // Status field
                let status_style = if state.edit_field == 2 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                let status_value = format!("< {} >", statuses[state.edit_status]);
                lines.push(Line::from(vec![
                    Span::styled("  Status:      ", Style::default().fg(colors.green())),
                    Span::styled(status_value, status_style),
                ]));

                lines.push(Line::from(""));

                // Priority field
                let priority_style = if state.edit_field == 3 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                let priority_value = format!("< {} >", priorities[state.edit_priority]);
                lines.push(Line::from(vec![
                    Span::styled("  Priority:    ", Style::default().fg(colors.green())),
                    Span::styled(priority_value, priority_style),
                ]));

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Comments => {
                let mut lines = vec![];

                if let Some(ref issue) = state.detail_issue {
                    // Issue title at top
                    lines.push(Line::from(vec![
                        Span::styled("  Issue: ", Style::default().fg(colors.green())),
                        Span::styled(&issue.id, Style::default().fg(colors.blue())),
                        Span::styled(" - ", Style::default().fg(colors.grey())),
                        Span::styled(&issue.title, Style::default().fg(colors.fg())),
                    ]));
                    lines.push(Line::from(""));

                    // Comment input area at top if active
                    if state.comment_input_active {
                        lines.push(Line::from(Span::styled(
                            "  Add comment:",
                            Style::default()
                                .fg(colors.yellow())
                                .add_modifier(Modifier::BOLD),
                        )));
                        lines.push(Line::from(vec![
                            Span::styled("  > ", Style::default().fg(colors.green())),
                            Span::styled(&state.comment_input, Style::default().fg(colors.fg())),
                            Span::styled("█", Style::default().fg(colors.yellow())),
                        ]));
                        lines.push(Line::from(""));
                    }

                    if issue.comments.is_empty() {
                        lines.push(Line::from(""));
                        lines.push(Line::from(Span::styled(
                            "  No comments yet. Press 'A' to add one.",
                            Style::default().fg(colors.grey()),
                        )));
                    } else {
                        // Header
                        lines.push(Line::from(Span::styled(
                            format!("  ─── Comments ({}) ───", issue.comments.len()),
                            Style::default()
                                .fg(colors.magenta())
                                .add_modifier(Modifier::BOLD),
                        )));
                        lines.push(Line::from(""));

                        // List all comments with scrolling
                        for (i, comment) in issue.comments.iter().enumerate() {
                            let is_selected = i == state.selected_comment;
                            let bg = if is_selected {
                                colors.red()
                            } else {
                                Color::Reset
                            };
                            let prefix = if is_selected { "▶ " } else { "  " };

                            // Author and date line
                            lines.push(Line::from(vec![
                                Span::styled(prefix, Style::default().fg(colors.yellow()).bg(bg)),
                                Span::styled(
                                    format!("{} ", comment.author),
                                    Style::default().fg(colors.blue()).bg(bg),
                                ),
                                Span::styled(
                                    &comment.created_at,
                                    Style::default().fg(colors.grey()).bg(bg),
                                ),
                            ]));

                            // Comment text - wrap if too long
                            let text_style = Style::default().fg(colors.fg()).bg(bg);
                            let max_width = content_area.width.saturating_sub(6) as usize;
                            let text = &comment.text;
                            if text.len() <= max_width {
                                lines.push(Line::from(vec![
                                    Span::styled("    ", Style::default().bg(bg)),
                                    Span::styled(text.clone(), text_style),
                                ]));
                            } else {
                                // Wrap text
                                for chunk in text.as_bytes().chunks(max_width) {
                                    let line_text = String::from_utf8_lossy(chunk).to_string();
                                    lines.push(Line::from(vec![
                                        Span::styled("    ", Style::default().bg(bg)),
                                        Span::styled(line_text, text_style),
                                    ]));
                                }
                            }
                            lines.push(Line::from(""));
                        }
                    }
                } else {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No issue selected",
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::History => {
                let visible_height = content_area.height.saturating_sub(4) as usize;
                let mut lines = vec![];

                if let Some(ref issue) = state.detail_issue {
                    // Issue title at top
                    lines.push(Line::from(vec![
                        Span::styled("  Issue: ", Style::default().fg(colors.green())),
                        Span::styled(&issue.id, Style::default().fg(colors.blue())),
                        Span::styled(" - ", Style::default().fg(colors.grey())),
                        Span::styled(&issue.title, Style::default().fg(colors.fg())),
                    ]));
                    lines.push(Line::from(""));
                }

                if state.activity_entries.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  No activity history available.",
                        Style::default().fg(colors.grey()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  Activity is tracked when issues are modified.",
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    // Timeline header
                    lines.push(Line::from(Span::styled(
                        format!(
                            "  ─── Timeline ({} events) ───",
                            state.activity_entries.len()
                        ),
                        Style::default()
                            .fg(colors.magenta())
                            .add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));

                    // Calculate scroll offset for selected item visibility
                    let start = if state.selected_activity >= visible_height {
                        state.selected_activity - visible_height + 1
                    } else {
                        0
                    };

                    for (i, entry) in state
                        .activity_entries
                        .iter()
                        .enumerate()
                        .skip(start)
                        .take(visible_height)
                    {
                        let is_selected = i == state.selected_activity;
                        let bg = if is_selected {
                            colors.red()
                        } else {
                            Color::Reset
                        };
                        let prefix = if is_selected { "▶ " } else { "  " };

                        // Event type color
                        let event_color = match entry.event_type.as_str() {
                            "created" => colors.green(),
                            "status_change" => colors.yellow(),
                            "closed" => colors.grey(),
                            "reopened" => colors.cyan(),
                            "comment_added" => colors.blue(),
                            "priority_change" => colors.magenta(),
                            "assignment_change" => colors.cyan(),
                            _ => colors.fg(),
                        };

                        // Timeline connector
                        let connector = if i == 0 { "┌" } else { "├" };

                        // Main event line with timestamp
                        lines.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(colors.yellow()).bg(bg)),
                            Span::styled(
                                format!("{} ", connector),
                                Style::default().fg(colors.grey()).bg(bg),
                            ),
                            Span::styled(
                                format!("{} ", entry.symbol),
                                Style::default().fg(event_color).bg(bg),
                            ),
                            Span::styled(
                                &entry.timestamp,
                                Style::default().fg(colors.grey()).bg(bg),
                            ),
                        ]));

                        // Event detail line
                        let detail_prefix = if i == state.activity_entries.len() - 1 {
                            "└──"
                        } else {
                            "│  "
                        };
                        lines.push(Line::from(vec![
                            Span::styled("  ", Style::default().bg(bg)),
                            Span::styled(
                                format!("{} ", detail_prefix),
                                Style::default().fg(colors.grey()).bg(bg),
                            ),
                            Span::styled(&entry.message, Style::default().fg(colors.fg()).bg(bg)),
                        ]));

                        // Status transition if present
                        if let (Some(old), Some(new)) = (&entry.old_status, &entry.new_status) {
                            lines.push(Line::from(vec![
                                Span::styled("  ", Style::default().bg(bg)),
                                Span::styled("│     ", Style::default().fg(colors.grey()).bg(bg)),
                                Span::styled(old, Style::default().fg(colors.grey()).bg(bg)),
                                Span::styled(" → ", Style::default().fg(colors.yellow()).bg(bg)),
                                Span::styled(new, Style::default().fg(colors.green()).bg(bg)),
                            ]));
                        }

                        // Actor if present
                        if let Some(ref actor) = entry.actor {
                            lines.push(Line::from(vec![
                                Span::styled("  ", Style::default().bg(bg)),
                                Span::styled("│     ", Style::default().fg(colors.grey()).bg(bg)),
                                Span::styled("by ", Style::default().fg(colors.grey()).bg(bg)),
                                Span::styled(actor, Style::default().fg(colors.blue()).bg(bg)),
                            ]));
                        }

                        lines.push(Line::from(""));
                    }
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::FileIssues => {
                let visible_height = content_area.height.saturating_sub(4) as usize;
                let mut lines = vec![];

                // Show the file being queried
                lines.push(Line::from(vec![
                    Span::styled("  File: ", Style::default().fg(colors.green())),
                    Span::styled(&state.file_query_path, Style::default().fg(colors.blue())),
                ]));
                lines.push(Line::from(""));

                if state.file_related_issues.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  No issues found mentioning this file.",
                        Style::default().fg(colors.grey()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  Create an issue with the filename to link it.",
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!(
                            "  ─── Related Issues ({}) ───",
                            state.file_related_issues.len()
                        ),
                        Style::default()
                            .fg(colors.magenta())
                            .add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));

                    for (i, issue) in state
                        .file_related_issues
                        .iter()
                        .enumerate()
                        .take(visible_height)
                    {
                        let is_selected = i == state.file_issue_selected;
                        let bg = if is_selected {
                            colors.red()
                        } else {
                            Color::Reset
                        };
                        let prefix = if is_selected { "▶ " } else { "  " };

                        // Status indicator
                        let status_char = match issue.status.as_str() {
                            "closed" => "✓",
                            "in_progress" => "◆",
                            "open" => "○",
                            _ => "?",
                        };
                        let status_color = match issue.status.as_str() {
                            "closed" => colors.grey(),
                            "in_progress" => colors.yellow(),
                            "open" => colors.green(),
                            _ => colors.fg(),
                        };

                        // Issue line
                        lines.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(colors.yellow()).bg(bg)),
                            Span::styled(status_char, Style::default().fg(status_color).bg(bg)),
                            Span::styled(" ", Style::default().bg(bg)),
                            Span::styled(&issue.id, Style::default().fg(colors.blue()).bg(bg)),
                            Span::styled(" ", Style::default().bg(bg)),
                            Span::styled(
                                format!("[{}]", issue.priority),
                                Style::default().fg(colors.cyan()).bg(bg),
                            ),
                            Span::styled(" ", Style::default().bg(bg)),
                            Span::styled(&issue.title, Style::default().fg(colors.fg()).bg(bg)),
                        ]));
                    }
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Dependencies => {
                let visible_height = content_area.height.saturating_sub(2) as usize;
                let mut lines = vec![];

                if state.issues.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  No issues to display",
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        "  Issue ID          Status        Blocked By → Dependents",
                        Style::default()
                            .fg(colors.blue())
                            .add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));

                    for (i, issue) in state
                        .issues
                        .iter()
                        .skip(state.scroll_offset)
                        .enumerate()
                        .take(visible_height)
                    {
                        let is_selected = i == state.selected_issue;
                        let bg = if is_selected {
                            colors.red()
                        } else {
                            Color::Reset
                        };
                        let prefix = if is_selected { "▶ " } else { "  " };

                        // Status indicator
                        let status_char = match issue.status.as_str() {
                            "closed" => "✓",
                            "in_progress" => "◆",
                            "open" => "○",
                            _ => "?",
                        };
                        let status_color = match issue.status.as_str() {
                            "closed" => colors.grey(),
                            "in_progress" => colors.yellow(),
                            "open" => colors.green(),
                            _ => colors.fg(),
                        };

                        // Build dependency info
                        let blocked_by_str = if issue.blocked_by.is_empty() {
                            "none".to_string()
                        } else {
                            issue
                                .blocked_by
                                .iter()
                                .map(|b| {
                                    // Shorten the ID
                                    if b.len() > 8 {
                                        format!("{}…", &b[..7])
                                    } else {
                                        b.clone()
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        };

                        let dependents_str = if issue.dependents.is_empty() {
                            "none".to_string()
                        } else {
                            format!("{} items", issue.dependents.len())
                        };

                        // ID shortened if needed
                        let id_short = if issue.id.len() > 12 {
                            format!("{}…", &issue.id[..11])
                        } else {
                            issue.id.clone()
                        };

                        // Type indicator
                        let type_symbol = match issue.issue_type.as_str() {
                            "epic" => "⊞",
                            "bug" => "●",
                            "feature" => "★",
                            _ => "□",
                        };
                        let type_color = match issue.issue_type.as_str() {
                            "epic" => colors.cyan(),
                            "bug" => colors.red(),
                            "feature" => colors.green(),
                            _ => colors.grey(),
                        };

                        // First line: ID and status
                        lines.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(colors.yellow()).bg(bg)),
                            Span::styled(
                                format!("{} ", type_symbol),
                                Style::default().fg(type_color).bg(bg),
                            ),
                            Span::styled(
                                format!("{:<12} ", id_short),
                                Style::default().fg(colors.blue()).bg(bg),
                            ),
                            Span::styled(
                                format!("{} ", status_char),
                                Style::default().fg(status_color).bg(bg),
                            ),
                            Span::styled(
                                format!("{:<12}", issue.status),
                                Style::default().fg(status_color).bg(bg),
                            ),
                        ]));

                        // Second line: Dependencies
                        let block_color = if issue.blocked_by.is_empty() {
                            colors.grey()
                        } else {
                            colors.red()
                        };
                        let dep_color = if issue.dependents.is_empty() {
                            colors.grey()
                        } else {
                            colors.cyan()
                        };

                        lines.push(Line::from(vec![
                            Span::styled("      ← ", Style::default().fg(block_color).bg(bg)),
                            Span::styled(
                                format!("{:<20}", blocked_by_str),
                                Style::default().fg(block_color).bg(bg),
                            ),
                            Span::styled(" → ", Style::default().fg(dep_color).bg(bg)),
                            Span::styled(dependents_str, Style::default().fg(dep_color).bg(bg)),
                        ]));

                        // Third line: Title (truncated)
                        let max_title_w = content_area.width.saturating_sub(8) as usize;
                        let title = if issue.title.len() > max_title_w {
                            format!("{}…", &issue.title[..max_title_w.saturating_sub(1)])
                        } else {
                            issue.title.clone()
                        };
                        lines.push(Line::from(vec![
                            Span::styled("      ", Style::default().bg(bg)),
                            Span::styled(title, Style::default().fg(colors.fg()).bg(bg)),
                        ]));

                        // Separator
                        lines.push(Line::from(""));
                    }
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Kanban => {
                // Kanban board with three columns: Open, In Progress, Closed
                let column_width = content_area.width / 3;
                let visible_height = content_area.height.saturating_sub(3) as usize;

                // Filter issues by status
                let open_issues: Vec<_> =
                    state.issues.iter().filter(|i| i.status == "open").collect();
                let in_progress_issues: Vec<_> = state
                    .issues
                    .iter()
                    .filter(|i| i.status == "in_progress")
                    .collect();
                let closed_issues: Vec<_> = state
                    .issues
                    .iter()
                    .filter(|i| i.status == "closed")
                    .collect();

                let columns = [
                    ("OPEN", &open_issues, colors.green()),
                    ("IN PROGRESS", &in_progress_issues, colors.yellow()),
                    ("CLOSED", &closed_issues, colors.grey()),
                ];

                let mut lines = vec![];

                // Header row
                let mut header_spans = vec![];
                for (i, (title, issues, color)) in columns.iter().enumerate() {
                    let selected = i == state.kanban_column;
                    let style = if selected {
                        Style::default()
                            .fg(*color)
                            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                    } else {
                        Style::default().fg(*color).add_modifier(Modifier::BOLD)
                    };
                    header_spans.push(Span::styled(
                        format!(
                            " {:^width$}",
                            format!("{} ({})", title, issues.len()),
                            width = column_width.saturating_sub(2) as usize
                        ),
                        style,
                    ));
                }
                lines.push(Line::from(header_spans));
                lines.push(Line::from(""));

                // Render rows with scroll support
                let max_items = open_issues
                    .len()
                    .max(in_progress_issues.len())
                    .max(closed_issues.len());

                // Calculate scroll offset to keep kanban_row visible
                let scroll_offset = if state.kanban_row >= visible_height {
                    state.kanban_row.saturating_sub(visible_height - 1)
                } else {
                    0
                };

                for display_row in 0..visible_height.min(max_items.saturating_sub(scroll_offset)) {
                    let actual_row = scroll_offset + display_row;
                    let mut row_spans = vec![];

                    for (col_idx, (_, issues, color)) in columns.iter().enumerate() {
                        let is_selected_cell =
                            col_idx == state.kanban_column && actual_row == state.kanban_row;
                        let bg = if is_selected_cell {
                            colors.red()
                        } else {
                            Color::Reset
                        };

                        if actual_row < issues.len() {
                            let issue = &issues[actual_row];
                            let type_symbol = match issue.issue_type.as_str() {
                                "epic" => "⊞",
                                "bug" => "●",
                                "feature" => "★",
                                _ => "□",
                            };

                            // Truncate title to fit column
                            let max_title = column_width.saturating_sub(6) as usize;
                            let title = if issue.title.len() > max_title {
                                format!("{}…", &issue.title[..max_title.saturating_sub(1)])
                            } else {
                                issue.title.clone()
                            };

                            row_spans.push(Span::styled(
                                format!(
                                    " {} {:width$}",
                                    type_symbol,
                                    title,
                                    width = column_width.saturating_sub(4) as usize
                                ),
                                Style::default().fg(*color).bg(bg),
                            ));
                        } else {
                            // Empty cell
                            row_spans.push(Span::styled(
                                format!("{:width$}", "", width = column_width as usize),
                                Style::default().bg(bg),
                            ));
                        }
                    }

                    lines.push(Line::from(row_spans));
                }

                // If no issues at all
                if state.issues.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  No issues to display",
                        Style::default().fg(colors.grey()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Human | BeadsView::Doctor => {
                let visible_height = content_area.height as usize;
                let mut lines = vec![];

                // Title line
                let title_text = if state.view == BeadsView::Human {
                    "Common beads commands for human users:"
                } else {
                    "Beads installation health check:"
                };
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    title_text,
                    Style::default().fg(colors.yellow()),
                )));
                lines.push(Line::from(""));

                // Show output lines with scrolling
                let total_lines = state.output_lines.len();
                let start = state.scroll_offset;
                let end = (start + visible_height.saturating_sub(4)).min(total_lines);

                for line in state.output_lines.iter().skip(start).take(end - start) {
                    // Color lines based on content
                    let style =
                        if line.contains("✓") || line.contains("OK") || line.contains("passed") {
                            Style::default().fg(colors.green())
                        } else if line.contains("✗")
                            || line.contains("ERROR")
                            || line.contains("failed")
                        {
                            Style::default().fg(colors.red())
                        } else if line.contains("WARNING") {
                            Style::default().fg(colors.yellow())
                        } else if line.starts_with("  ") {
                            Style::default().fg(colors.grey())
                        } else {
                            Style::default().fg(colors.fg())
                        };
                    lines.push(Line::from(Span::styled(line.clone(), style)));
                }

                // Scroll indicator
                if total_lines > visible_height.saturating_sub(4) {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("-- Line {}/{} --", start + 1, total_lines.max(1)),
                        Style::default().fg(colors.grey()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
        }
    }

    // Render help line based on view
    let help_hints: Vec<(&str, &str)> = if !state.is_beads_project {
        vec![("Any key", "close")]
    } else {
        match state.view {
            BeadsView::Menu => vec![("↑↓", "select"), ("Enter", "open"), ("ESC", "close")],
            BeadsView::List | BeadsView::Ready | BeadsView::Blocked | BeadsView::Epics => {
                if state.search_active {
                    vec![("type", "search"), ("Enter", "finish"), ("ESC", "cancel")]
                } else if !state.search_query.is_empty() {
                    vec![
                        ("↑↓", "nav"),
                        ("/", "search"),
                        ("C", "close"),
                        ("S", "start"),
                        ("ESC", "clear"),
                    ]
                } else {
                    vec![
                        ("↑↓", "nav"),
                        ("/", "search"),
                        ("C", "close"),
                        ("S", "start"),
                        ("ESC", "back"),
                    ]
                }
            }
            BeadsView::Stats => vec![("R", "refresh"), ("ESC", "back")],
            BeadsView::Create => vec![
                ("↑↓", "field"),
                ("←→", "value"),
                ("Enter", "create"),
                ("ESC", "cancel"),
            ],
            BeadsView::Detail => vec![
                ("↑↓", "subtasks"),
                ("PgUp/Dn", "scroll"),
                ("E", "edit"),
                ("N", "new"),
                ("C", "close"),
            ],
            BeadsView::Edit => vec![
                ("↑↓", "field"),
                ("←→", "value"),
                ("Enter", "save"),
                ("ESC", "cancel"),
            ],
            BeadsView::Comments => {
                if state.comment_input_active {
                    vec![("type", "comment"), ("Enter", "submit"), ("ESC", "cancel")]
                } else {
                    vec![("↑↓", "navigate"), ("A", "add"), ("ESC", "back")]
                }
            }
            BeadsView::History => vec![
                ("↑↓", "navigate"),
                ("PgUp/Dn", "page"),
                ("R", "refresh"),
                ("ESC", "back"),
            ],
            BeadsView::FileIssues => vec![
                ("↑↓", "navigate"),
                ("Enter", "detail"),
                ("R", "refresh"),
                ("ESC", "back"),
            ],
            BeadsView::Dependencies => vec![
                ("↑↓", "navigate"),
                ("Enter", "detail"),
                ("R", "refresh"),
                ("ESC", "back"),
            ],
            BeadsView::Kanban => vec![
                ("←→", "columns"),
                ("↑↓", "rows"),
                ("Enter", "detail"),
                ("ESC", "back"),
            ],
            BeadsView::Human | BeadsView::Doctor => {
                vec![("↑↓", "scroll"), ("PgUp/Dn", "page"), ("ESC", "back")]
            }
        }
    };
    view.render_help(frame, help_hints);
}
