use crate::tui::app::{App, SelectedItem};
use ratatui::{prelude::*, widgets::*};

pub fn render_tree(f: &mut Frame, area: Rect, app: &App) {
    if app.branches.is_empty() {
        let empty = Paragraph::new("No branches found. Run 'ws --scan' first.")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    for (branch_idx, branch) in app.branches.iter().enumerate() {
        let is_selected_branch = branch_idx == app.selected_branch_idx;

        // Build worktree selector string
        let worktree_parts: Vec<Span> = branch
            .data
            .worktrees
            .iter()
            .enumerate()
            .map(|(wt_idx, wt)| {
                let is_selected = wt_idx == branch.selected_worktree_idx;
                let state = &branch.worktree_states[wt_idx];

                let checkbox = if is_selected { "[*]" } else { "[ ]" };
                let display = format!(
                    "{} {} ({})",
                    checkbox,
                    wt.repo_name,
                    truncate_branch(&wt.branch, 12)
                );

                let style = if state.is_dirty {
                    Style::default().fg(Color::Red)
                } else if state.has_wip {
                    Style::default().fg(Color::Yellow)
                } else if is_selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                Span::styled(display, style)
            })
            .collect();

        // Branch line
        let branch_selected = is_selected_branch && app.selected_item == SelectedItem::Branch;
        let expand_char = if branch.expanded { "▼" } else { "▶" };

        let mut branch_spans = vec![
            Span::styled(
                format!("{} {} ", expand_char, branch.data.branch),
                if branch_selected {
                    Style::default().bold().fg(Color::White)
                } else {
                    Style::default()
                },
            ),
            Span::raw("("),
        ];

        for (i, wt_span) in worktree_parts.into_iter().enumerate() {
            if i > 0 {
                branch_spans.push(Span::raw(" "));
            }
            branch_spans.push(wt_span);
        }
        branch_spans.push(Span::raw(")"));

        let branch_line = Line::from(branch_spans);
        lines.push(if branch_selected {
            branch_line.patch_style(Style::default().bg(Color::DarkGray))
        } else {
            branch_line
        });

        // Sessions (if expanded)
        if branch.expanded {
            for (session_idx, session) in branch.data.sessions.iter().enumerate() {
                let session_selected =
                    is_selected_branch && app.selected_item == SelectedItem::Session(session_idx);
                let is_checked = branch.selected_sessions.contains(&session.uuid);

                let checkbox = if is_checked { "[x]" } else { "[ ]" };
                let summary = session
                    .summary
                    .as_ref()
                    .or(session.first_prompt.as_ref())
                    .map(|s| truncate_str(s, 50))
                    .unwrap_or_else(|| "No summary".to_string());

                let session_line = Line::from(vec![Span::styled(
                    format!("    {} {}", checkbox, summary),
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

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

fn truncate_branch(branch: &str, max_len: usize) -> String {
    if branch.len() <= max_len {
        branch.to_string()
    } else {
        format!("{}...", &branch[..max_len - 3])
    }
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
