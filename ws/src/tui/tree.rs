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

        let repo_line = Line::from(vec![Span::styled(
            format!("{} {}", expand_char, repo.data.name),
            if repo_selected {
                Style::default().bold().fg(Color::White)
            } else {
                Style::default().fg(Color::Cyan)
            },
        )]);

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

                // Build worktree status indicator
                let state = &branch.worktree_state;
                let status_style = if state.is_dirty {
                    Style::default().fg(Color::Red)
                } else if state.has_wip {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                };

                let expand_char = if branch.expanded { "▼" } else { "▶" };

                let branch_spans = vec![
                    Span::raw("    "),
                    Span::styled(
                        format!("{} {}", expand_char, branch.data.branch),
                        if branch_selected {
                            Style::default().bold().fg(Color::White)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::raw(" "),
                    Span::styled("●", status_style),
                ];

                let branch_line = Line::from(branch_spans);
                lines.push(if branch_selected {
                    branch_line.patch_style(Style::default().bg(Color::DarkGray))
                } else {
                    branch_line
                });

                // Sessions (if expanded)
                if branch.expanded {
                    for (session_idx, session) in branch.data.sessions.iter().enumerate() {
                        let session_selected = is_selected_branch
                            && app.selected_item == SelectedItem::Session(session_idx);
                        let is_checked = branch.selected_sessions.contains(&session.uuid);

                        let checkbox = if is_checked { "[x]" } else { "[ ]" };
                        let summary = session
                            .summary
                            .as_ref()
                            .or(session.first_prompt.as_ref())
                            .map(|s| truncate_str(s, 50))
                            .unwrap_or_else(|| "No summary".to_string());

                        let session_line = Line::from(vec![Span::styled(
                            format!("        {} {}", checkbox, summary),
                            if session_selected {
                                Style::default().fg(Color::Cyan)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            },
                        )]);

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
