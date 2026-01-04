//! Git modal drawing

use crate::app::App;
use crate::plugins::git::{GitMenuItem, GitState, GitView, RemoteAction};
use crate::ui::components::ModalFrame;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Draw Git modal
pub fn draw_git_modal(frame: &mut Frame, area: Rect, state: &GitState, app: &App) {
    let colors = app.colors();

    // Title based on current view
    let title = match state.view {
        GitView::Menu => " GIT INTEGRATION ",
        GitView::Status => " GIT STATUS ",
        GitView::Log => " GIT LOG ",
        GitView::Diff => " GIT DIFF ",
        GitView::Commit => " GIT COMMIT ",
        GitView::Branch => " GIT BRANCHES ",
        GitView::Stash => " GIT STASH ",
        GitView::Tag => " GIT TAGS ",
        GitView::Remote => match state.remote_action {
            RemoteAction::Push => " GIT PUSH TO REMOTE ",
            RemoteAction::Pull => " GIT PULL FROM REMOTE ",
        },
        GitView::Config => " GIT CONFIG ",
        GitView::Conflicts => " MERGE CONFLICTS ",
        GitView::Submodules => " GIT SUBMODULES ",
    };

    // Create modal frame
    let modal = ModalFrame::themed(area, title, &colors);
    modal.render_frame(frame);

    // Content area
    let content_area = modal.content_area();

    if !state.is_repo {
        // Not a git repo
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Not a Git repository",
                Style::default().fg(colors.yellow()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Initialize a git repository with 'git init'",
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
            GitView::Menu => {
                let mut lines = vec![Line::from("")];

                for (i, item) in GitMenuItem::ALL.iter().enumerate() {
                    let is_selected = i == state.menu_selected;
                    let style = if is_selected {
                        Style::default().fg(colors.yellow()).bg(colors.red())
                    } else {
                        Style::default().fg(colors.fg())
                    };

                    let key = match item {
                        GitMenuItem::Status => "S",
                        GitMenuItem::Log => "L",
                        GitMenuItem::Diff => "D",
                        GitMenuItem::Commit => "C",
                        GitMenuItem::Push => "P",
                        GitMenuItem::Pull => "U",
                        GitMenuItem::Branch => "B",
                        GitMenuItem::Stash => "H",
                        GitMenuItem::Tag => "T",
                        GitMenuItem::Config => "G",
                        GitMenuItem::Conflicts => "X",
                        GitMenuItem::Submodules => "M",
                    };

                    lines.push(Line::from(vec![
                        Span::styled("  ", style),
                        Span::styled(
                            format!("[{}] ", key),
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

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Status => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                if state.files.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Working tree clean",
                        Style::default().fg(colors.green()),
                    )));
                } else {
                    for (i, file) in state.files.iter().enumerate().take(visible_height) {
                        let is_selected = i == state.selected_file;
                        let status_char = match file.status {
                            'M' => "M",
                            'A' => "A",
                            'D' => "D",
                            'R' => "R",
                            '?' => "?",
                            _ => " ",
                        };
                        let staged_indicator = if file.staged { "+" } else { " " };

                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        let status_style = if is_selected {
                            style
                        } else {
                            match file.status {
                                'M' => Style::default().fg(colors.yellow()),
                                'A' => Style::default().fg(colors.green()),
                                'D' => Style::default().fg(colors.red()),
                                '?' => Style::default().fg(colors.grey()),
                                _ => Style::default().fg(colors.fg()),
                            }
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!(" {} ", staged_indicator), status_style),
                            Span::styled(format!("{} ", status_char), status_style),
                            Span::styled(&file.path, style),
                        ]));
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
            GitView::Log => {
                let visible_height = content_area.height as usize / 2; // Each entry takes 2 lines
                let mut lines: Vec<Line> = vec![];

                // Calculate scroll offset based on selection
                let scroll = if state.selected_log >= visible_height {
                    state.selected_log - visible_height + 1
                } else {
                    0
                };

                for (i, entry) in state
                    .log_entries
                    .iter()
                    .enumerate()
                    .skip(scroll)
                    .take(visible_height)
                {
                    let is_selected = i == state.selected_log;
                    let prefix = if is_selected { "▶ " } else { "  " };
                    let hash_style = if is_selected {
                        Style::default()
                            .fg(colors.yellow())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.yellow())
                    };
                    let msg_style = if is_selected {
                        Style::default()
                            .fg(colors.fg())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.fg())
                    };

                    lines.push(Line::from(vec![
                        Span::styled(prefix, hash_style),
                        Span::styled(format!("{} ", entry.hash), hash_style),
                        Span::styled(&entry.message, msg_style),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("         ", Style::default()),
                        Span::styled(
                            format!("{} - {}", entry.author, entry.date),
                            Style::default().fg(colors.grey()),
                        ),
                    ]));
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Diff => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                for line_text in state
                    .diff_content
                    .iter()
                    .skip(state.scroll_offset)
                    .take(visible_height)
                {
                    let style = if line_text.starts_with('+') && !line_text.starts_with("+++") {
                        Style::default().fg(colors.green())
                    } else if line_text.starts_with('-') && !line_text.starts_with("---") {
                        Style::default().fg(colors.red())
                    } else if line_text.starts_with("@@") {
                        Style::default().fg(colors.cyan())
                    } else if line_text.starts_with("diff") || line_text.starts_with("index") {
                        Style::default().fg(colors.yellow())
                    } else {
                        Style::default().fg(colors.fg())
                    };

                    lines.push(Line::from(Span::styled(line_text.clone(), style)));
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Commit => {
                let mut lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Enter commit message:",
                        Style::default().fg(colors.green()),
                    )),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(
                            &state.commit_message,
                            Style::default().fg(colors.yellow()).bg(colors.red()),
                        ),
                        Span::styled("█", Style::default().fg(colors.yellow()).bg(colors.red())),
                    ]),
                ];

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Branch => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                if state.branch_input_mode {
                    // Show branch name input
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Create new branch:",
                        Style::default().fg(colors.green()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled(
                            &state.branch_name_input,
                            Style::default().fg(colors.yellow()).bg(colors.red()),
                        ),
                        Span::styled("█", Style::default().fg(colors.yellow()).bg(colors.red())),
                    ]));
                } else if state.branches.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No branches found",
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    // List branches
                    for (i, branch) in state.branches.iter().enumerate() {
                        if i >= visible_height {
                            break;
                        }

                        let is_selected = i == state.selected_branch;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        let marker = if branch.is_current { "* " } else { "  " };
                        let marker_style = if branch.is_current {
                            Style::default().fg(colors.green())
                        } else {
                            style
                        };

                        // Truncate commit message
                        let commit_display = if branch.last_commit.len() > 40 {
                            format!("{}...", &branch.last_commit[..37])
                        } else {
                            branch.last_commit.clone()
                        };

                        lines.push(Line::from(vec![
                            Span::styled(marker, marker_style),
                            Span::styled(format!("{:<20} ", branch.name), style),
                            Span::styled(
                                commit_display,
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.grey())
                                },
                            ),
                        ]));
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
            GitView::Stash => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                if state.stash_input_mode {
                    // Show stash message input
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Stash message (optional):",
                        Style::default().fg(colors.green()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled(
                            &state.stash_message_input,
                            Style::default().fg(colors.yellow()).bg(colors.red()),
                        ),
                        Span::styled("█", Style::default().fg(colors.yellow()).bg(colors.red())),
                    ]));
                } else if state.stashes.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No stashes",
                        Style::default().fg(colors.grey()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Press S to stash current changes",
                        Style::default().fg(colors.green()),
                    )));
                } else {
                    // List stashes
                    for (i, stash) in state.stashes.iter().enumerate() {
                        if i >= visible_height {
                            break;
                        }

                        let is_selected = i == state.selected_stash;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        // Truncate message
                        let msg_display = if stash.message.len() > 50 {
                            format!("{}...", &stash.message[..47])
                        } else {
                            stash.message.clone()
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!("  stash@{{{}}}: ", stash.index), style),
                            Span::styled(
                                msg_display,
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.grey())
                                },
                            ),
                        ]));
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
            GitView::Tag => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                if state.tag_input_mode {
                    // Show tag name input
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Create new tag:",
                        Style::default().fg(colors.green()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled(
                            &state.tag_name_input,
                            Style::default().fg(colors.yellow()).bg(colors.red()),
                        ),
                        Span::styled("█", Style::default().fg(colors.yellow()).bg(colors.red())),
                    ]));
                } else if state.tags.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No tags",
                        Style::default().fg(colors.grey()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Press N to create a new tag",
                        Style::default().fg(colors.green()),
                    )));
                } else {
                    // List tags
                    for (i, tag) in state.tags.iter().enumerate() {
                        if i >= visible_height {
                            break;
                        }

                        let is_selected = i == state.selected_tag;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        let msg = tag
                            .message
                            .as_ref()
                            .map(|m| {
                                if m.len() > 40 {
                                    format!(" - {}...", &m[..37])
                                } else {
                                    format!(" - {}", m)
                                }
                            })
                            .unwrap_or_default();

                        lines.push(Line::from(vec![
                            Span::styled(format!("  {:<20} ", tag.name), style),
                            Span::styled(
                                tag.commit.clone(),
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.blue())
                                },
                            ),
                            Span::styled(
                                msg,
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.grey())
                                },
                            ),
                        ]));
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
            GitView::Remote => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                let action_text = match state.remote_action {
                    RemoteAction::Push => "Push to",
                    RemoteAction::Pull => "Pull from",
                };

                if state.remotes.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No remotes configured",
                        Style::default().fg(colors.grey()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Use 'git remote add <name> <url>' to add a remote",
                        Style::default().fg(colors.green()),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("Select remote to {}:", action_text),
                        Style::default().fg(colors.green()),
                    )));
                    lines.push(Line::from(""));

                    // List remotes
                    for (i, remote) in state.remotes.iter().enumerate() {
                        if i + 2 >= visible_height {
                            break;
                        }

                        let is_selected = i == state.selected_remote;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        // Truncate URL if too long
                        let url = if remote.url.len() > 50 {
                            format!("{}...", &remote.url[..47])
                        } else {
                            remote.url.clone()
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!("  {:<12} ", remote.name), style),
                            Span::styled(
                                url,
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.blue())
                                },
                            ),
                        ]));
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
            GitView::Config => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                if state.config_entries.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No config entries found",
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    // Calculate scroll offset
                    let start = if state.selected_config >= visible_height {
                        state.selected_config - visible_height + 1
                    } else {
                        0
                    };

                    // List config entries
                    for (idx, entry) in state.config_entries.iter().enumerate().skip(start) {
                        if idx >= start + visible_height {
                            break;
                        }

                        let is_selected = idx == state.selected_config;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        // Format: [scope] key = value
                        let scope_color = match entry.scope.as_str() {
                            "local" => colors.green(),
                            "global" => colors.blue(),
                            _ => colors.grey(),
                        };

                        // Truncate value if too long
                        let max_val_len = 40;
                        let value = if entry.value.len() > max_val_len {
                            format!("{}...", &entry.value[..max_val_len - 3])
                        } else {
                            entry.value.clone()
                        };

                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("[{:6}] ", entry.scope),
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(scope_color)
                                },
                            ),
                            Span::styled(format!("{:<30} ", entry.key), style),
                            Span::styled("= ", Style::default().fg(colors.grey())),
                            Span::styled(
                                value,
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.cyan())
                                },
                            ),
                        ]));
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
            GitView::Conflicts => {
                let mut lines = vec![];

                if state.conflict_files.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  No merge conflicts detected",
                        Style::default().fg(colors.green()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  All conflicts have been resolved or there is no merge in progress.",
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    // Header
                    lines.push(Line::from(vec![Span::styled(
                        format!(
                            "  {} conflicting file(s) - ←→ to switch files",
                            state.conflict_files.len()
                        ),
                        Style::default().fg(colors.yellow()),
                    )]));
                    lines.push(Line::from(""));

                    // Current file info
                    let file = &state.conflict_files[state.selected_conflict_file];
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!(
                                "  File [{}/{}]: ",
                                state.selected_conflict_file + 1,
                                state.conflict_files.len()
                            ),
                            Style::default().fg(colors.grey()),
                        ),
                        Span::styled(&file.path, Style::default().fg(colors.blue())),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(
                            format!("{} conflict section(s)", file.sections.len()),
                            Style::default().fg(colors.red()),
                        ),
                    ]));
                    lines.push(Line::from(""));

                    // Show sections
                    let visible_height = content_area.height.saturating_sub(8) as usize;
                    for (i, section) in file.sections.iter().enumerate().take(visible_height / 6) {
                        let is_selected = i == file.selected_section;
                        let bg = if is_selected {
                            colors.red()
                        } else {
                            Color::Reset
                        };
                        let prefix = if is_selected { "▶ " } else { "  " };

                        // Section header
                        lines.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(colors.yellow()).bg(bg)),
                            Span::styled(
                                format!("Conflict {} (line {})", i + 1, section.start_line),
                                Style::default()
                                    .fg(colors.cyan())
                                    .bg(bg)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]));

                        // Ours section (green)
                        lines.push(Line::from(vec![
                            Span::styled("    ", Style::default().bg(bg)),
                            Span::styled(
                                "<<<< OURS (current branch)",
                                Style::default().fg(colors.green()).bg(bg),
                            ),
                        ]));
                        for (j, line) in section.ours.iter().take(3).enumerate() {
                            let truncated = if line.len() > 60 {
                                format!("{}...", &line[..57])
                            } else {
                                line.clone()
                            };
                            lines.push(Line::from(vec![
                                Span::styled("      ", Style::default().bg(bg)),
                                Span::styled(
                                    format!("{}: {}", j + 1, truncated),
                                    Style::default().fg(colors.green()).bg(bg),
                                ),
                            ]));
                        }
                        if section.ours.len() > 3 {
                            lines.push(Line::from(vec![
                                Span::styled("      ", Style::default().bg(bg)),
                                Span::styled(
                                    format!("... and {} more lines", section.ours.len() - 3),
                                    Style::default().fg(colors.grey()).bg(bg),
                                ),
                            ]));
                        }

                        // Theirs section (red/yellow)
                        lines.push(Line::from(vec![
                            Span::styled("    ", Style::default().bg(bg)),
                            Span::styled(
                                ">>>> THEIRS (incoming)",
                                Style::default().fg(colors.yellow()).bg(bg),
                            ),
                        ]));
                        for (j, line) in section.theirs.iter().take(3).enumerate() {
                            let truncated = if line.len() > 60 {
                                format!("{}...", &line[..57])
                            } else {
                                line.clone()
                            };
                            lines.push(Line::from(vec![
                                Span::styled("      ", Style::default().bg(bg)),
                                Span::styled(
                                    format!("{}: {}", j + 1, truncated),
                                    Style::default().fg(colors.yellow()).bg(bg),
                                ),
                            ]));
                        }
                        if section.theirs.len() > 3 {
                            lines.push(Line::from(vec![
                                Span::styled("      ", Style::default().bg(bg)),
                                Span::styled(
                                    format!("... and {} more lines", section.theirs.len() - 3),
                                    Style::default().fg(colors.grey()).bg(bg),
                                ),
                            ]));
                        }

                        lines.push(Line::from(""));
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
            GitView::Submodules => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                if state.submodules.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No submodules found",
                        Style::default().fg(colors.grey()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Add submodules with 'git submodule add <url> <path>'",
                        Style::default().fg(colors.green()),
                    )));
                } else {
                    // List submodules
                    for (i, submodule) in state.submodules.iter().enumerate() {
                        if i >= visible_height {
                            break;
                        }

                        let is_selected = i == state.selected_submodule;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        // Status indicator
                        let status_indicator = match submodule.status {
                            crate::app::SubmoduleStatus::Initialized => "+",
                            crate::app::SubmoduleStatus::Uninitialized => "-",
                            crate::app::SubmoduleStatus::Modified => "*",
                            crate::app::SubmoduleStatus::Conflict => "!",
                            crate::app::SubmoduleStatus::OutOfDate => "^",
                        };

                        let status_color = match submodule.status {
                            crate::app::SubmoduleStatus::Initialized => colors.green(),
                            crate::app::SubmoduleStatus::Uninitialized => colors.grey(),
                            crate::app::SubmoduleStatus::Modified => colors.yellow(),
                            crate::app::SubmoduleStatus::Conflict => colors.red(),
                            crate::app::SubmoduleStatus::OutOfDate => colors.blue(),
                        };

                        // Truncate path if needed
                        let path_display = if submodule.path.len() > 40 {
                            format!("{}...", &submodule.path[..37])
                        } else {
                            submodule.path.clone()
                        };

                        lines.push(Line::from(vec![
                            Span::styled(
                                format!(" {} ", status_indicator),
                                Style::default().fg(status_color),
                            ),
                            Span::styled(format!("{:<42}", path_display), style),
                            Span::styled(
                                format!(" {}", &submodule.commit[..7.min(submodule.commit.len())]),
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.grey())
                                },
                            ),
                        ]));
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
        }
    }

    // Render help line based on view
    let help_hints: Vec<(&str, &str)> = if !state.is_repo {
        vec![("Any key", "close")]
    } else {
        match state.view {
            GitView::Menu => vec![
                ("↑↓", "select"),
                ("Enter", "open"),
                ("S/L/D/C/B/H/T/G", "quick"),
                ("ESC", "close"),
            ],
            GitView::Status => vec![
                ("↑↓", "navigate"),
                ("Enter", "diff"),
                ("A", "stage"),
                ("R", "refresh"),
                ("ESC", "back"),
            ],
            GitView::Log => vec![
                ("↑↓", "select"),
                ("Enter", "diff"),
                ("PgUp/Dn", "scroll"),
                ("ESC", "back"),
            ],
            GitView::Diff => vec![("↑↓", "scroll"), ("PgUp/Dn", "fast"), ("ESC", "back")],
            GitView::Commit => vec![
                ("type", "message"),
                ("Shift+Enter", "newline"),
                ("Enter", "commit"),
                ("ESC", "cancel"),
            ],
            GitView::Branch => {
                if state.branch_input_mode {
                    vec![("type", "name"), ("Enter", "create"), ("ESC", "cancel")]
                } else {
                    vec![
                        ("↑↓", "select"),
                        ("Enter", "switch"),
                        ("N", "new"),
                        ("D", "delete"),
                        ("ESC", "back"),
                    ]
                }
            }
            GitView::Stash => {
                if state.stash_input_mode {
                    vec![("type", "message"), ("Enter", "create"), ("ESC", "cancel")]
                } else {
                    vec![
                        ("↑↓", "select"),
                        ("S", "stash"),
                        ("P", "pop"),
                        ("D", "drop"),
                        ("ESC", "back"),
                    ]
                }
            }
            GitView::Tag => {
                if state.tag_input_mode {
                    vec![("type", "name"), ("Enter", "create"), ("ESC", "cancel")]
                } else {
                    vec![
                        ("↑↓", "select"),
                        ("N", "new"),
                        ("D", "delete"),
                        ("P", "push"),
                        ("ESC", "back"),
                    ]
                }
            }
            GitView::Remote => vec![("↑↓", "select"), ("Enter", "execute"), ("ESC", "back")],
            GitView::Config => vec![
                ("↑↓", "scroll"),
                ("PgUp/Dn", "fast"),
                ("R", "refresh"),
                ("ESC", "back"),
            ],
            GitView::Conflicts => vec![
                ("←→", "files"),
                ("↑↓", "sections"),
                ("O/T/B", "resolve"),
                ("M", "mark"),
                ("ESC", "back"),
            ],
            GitView::Submodules => vec![
                ("↑↓", "select"),
                ("I", "init"),
                ("U", "update"),
                ("S", "sync"),
                ("ESC", "back"),
            ],
        }
    };
    modal.render_help(frame, help_hints);
}
