use crate::tui::app::{App, SelectedItem};
use ratatui::{prelude::*, widgets::*};

pub fn render_tree(f: &mut Frame, area: Rect, app: &App) {
    if app.repos.is_empty() {
        let empty = Paragraph::new("No repos found. Run 'ws --scan' first.")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    for (repo_idx, repo) in app.repos.iter().enumerate() {
        let is_selected_repo = repo_idx == app.selected_repo_idx;

        // Repo line
        let repo_selected = is_selected_repo && app.selected_item == SelectedItem::Repo;
        let expand_char = if repo.expanded { "▼" } else { "▶" };

        // Build worktree dots for repo line
        let worktree_spans: Vec<Span> = repo
            .data
            .worktrees
            .iter()
            .enumerate()
            .map(|(wt_idx, wt)| {
                let state = &repo.worktree_states[wt_idx];

                // Color logic: red if dirty, yellow if WIP, white otherwise
                let style = if state.is_dirty {
                    Style::default().fg(Color::Red)
                } else if state.has_wip {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };

                // Show [name] for selected worktree when repo is selected,
                // otherwise show dot (filled if has a branch checked out)
                if repo_selected {
                    Span::styled(format!("[{}] ", wt.name), style)
                } else {
                    let symbol = if wt.checked_out_branch.is_some() {
                        "●"
                    } else {
                        "○"
                    };
                    Span::styled(format!("{} ", symbol), style)
                }
            })
            .collect();

        let mut repo_spans = vec![Span::styled(
            format!("{} {} ", expand_char, repo.data.name),
            if repo_selected {
                Style::default().bold().fg(Color::White)
            } else {
                Style::default().fg(Color::Cyan)
            },
        )];
        repo_spans.extend(worktree_spans);

        let repo_line = Line::from(repo_spans);
        lines.push(if repo_selected {
            repo_line.patch_style(Style::default().bg(Color::DarkGray))
        } else {
            repo_line
        });

        // Branches (if expanded)
        if repo.expanded {
            for (branch_idx, branch) in repo.branches.iter().enumerate() {
                let is_selected_branch = is_selected_repo && branch_idx == app.selected_branch_idx;
                let branch_selected =
                    is_selected_branch && app.selected_item == SelectedItem::Branch;

                let branch_data = &repo.data.branches[branch_idx];
                let expand_char = if branch.expanded { "▼" } else { "▶" };

                // Build worktree selector - show ALL worktrees in repo
                let worktree_spans: Vec<Span> = repo
                    .data
                    .worktrees
                    .iter()
                    .enumerate()
                    .flat_map(|(wt_idx, wt)| {
                        let is_selected_wt = wt_idx == branch.selected_worktree_idx;
                        let state = &repo.worktree_states[wt_idx];

                        // Color logic:
                        // - Green: this branch IS checked out in this worktree
                        // - Red: worktree is dirty
                        // - Yellow: worktree has WIP commit
                        // - White: otherwise
                        let is_checked_out = wt
                            .checked_out_branch
                            .as_ref()
                            .map_or(false, |b| b == &branch_data.branch);

                        let style = if state.is_dirty {
                            Style::default().fg(Color::Red)
                        } else if state.has_wip {
                            Style::default().fg(Color::Yellow)
                        } else if is_checked_out {
                            Style::default().fg(Color::Green)
                        } else {
                            Style::default().fg(Color::White)
                        };

                        let symbol = if is_selected_wt { "●" } else { "○" };

                        // Show dot, then [name] for selected worktree when branch is selected
                        if is_selected_wt && is_selected_branch {
                            vec![
                                Span::styled(format!("{}", symbol), style),
                                Span::styled(format!("[{}] ", wt.name), style),
                            ]
                        } else {
                            vec![Span::styled(format!("{} ", symbol), style)]
                        }
                    })
                    .collect();

                let mut branch_spans = vec![
                    Span::raw("    "),
                    Span::styled(
                        format!("{} {} ", expand_char, branch_data.branch),
                        if branch_selected {
                            Style::default().bold().fg(Color::White)
                        } else {
                            Style::default()
                        },
                    ),
                ];
                branch_spans.extend(worktree_spans);

                let branch_line = Line::from(branch_spans);
                lines.push(if branch_selected {
                    branch_line.patch_style(Style::default().bg(Color::DarkGray))
                } else {
                    branch_line
                });

                // Sessions (if expanded)
                if branch.expanded {
                    for (session_idx, session) in branch_data.sessions.iter().enumerate() {
                        let session_selected = is_selected_branch
                            && app.selected_item == SelectedItem::Session(session_idx);
                        let is_checked = branch.selected_sessions.contains(&session.uuid);

                        let checkbox = if is_checked { "[x]" } else { "[ ]" };
                        let summary = session
                            .summary
                            .as_ref()
                            .or(session.first_prompt.as_ref())
                            .map(|s| truncate_str(s, 40))
                            .unwrap_or_else(|| "No summary".to_string());

                        // Format metadata: message count and relative time
                        let msg_count = session
                            .message_count
                            .map(|c| format!("{} msg", c))
                            .unwrap_or_default();
                        let relative_time = format_relative_time(session.modified);

                        let metadata = if msg_count.is_empty() {
                            relative_time
                        } else {
                            format!("{} • {}", msg_count, relative_time)
                        };

                        let summary_style = if session_selected {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default().fg(Color::Gray)
                        };
                        let metadata_style = if session_selected {
                            Style::default().fg(Color::Gray)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };

                        let session_line = Line::from(vec![
                            Span::styled(
                                format!("        {} ", checkbox),
                                summary_style,
                            ),
                            Span::styled(summary, summary_style),
                            Span::styled(format!(" • {}", metadata), metadata_style),
                        ]);

                        lines.push(if session_selected {
                            session_line.patch_style(Style::default().bg(Color::DarkGray))
                        } else {
                            session_line
                        });
                    }
                }
            }
        }
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

fn truncate_str(s: &str, max_len: usize) -> String {
    // Take first line only
    let first_line = s.lines().next().unwrap_or(s);
    if first_line.len() <= max_len {
        first_line.to_string()
    } else {
        format!("{}...", &first_line[..max_len - 3])
    }
}

fn format_relative_time(timestamp_ms: i64) -> String {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let diff_ms = now_ms - timestamp_ms;
    if diff_ms < 0 {
        return "just now".to_string();
    }

    let minutes = diff_ms / (1000 * 60);
    let hours = minutes / 60;
    let days = hours / 24;
    let weeks = days / 7;

    if minutes < 1 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{} min ago", minutes)
    } else if hours < 24 {
        format!("{} hours ago", hours)
    } else if days < 7 {
        format!("{} days ago", days)
    } else {
        format!("{} weeks ago", weeks)
    }
}
