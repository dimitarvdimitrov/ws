use crate::actions;
use crate::config::Config;
use crate::db::{BranchData, Database};
use crate::scanner::git::{self, Worktree};
use crossterm::event::KeyCode;
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;

pub enum Action {
    Continue,
    Launch,
}

#[derive(Clone)]
pub struct ConfirmDialog {
    pub message: String,
    pub worktree_path: PathBuf,
}

pub struct BranchNode {
    pub data: BranchData,
    pub selected_worktree_idx: usize,
    pub selected_sessions: HashSet<String>, // UUIDs of selected sessions
    pub expanded: bool,
    // Runtime state (not from DB)
    pub worktree_states: Vec<WorktreeState>,
}

#[derive(Clone)]
pub struct WorktreeState {
    pub is_dirty: bool,
    pub has_wip: bool,
}

pub struct App {
    pub db: Database,
    pub config: Config,
    pub filter: String,
    pub branches: Vec<BranchNode>,
    pub selected_branch_idx: usize,
    pub selected_item: SelectedItem,
    pub confirm_dialog: Option<ConfirmDialog>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SelectedItem {
    Branch,
    Session(usize), // Index within branch's sessions
}

impl App {
    pub fn new(db: Database, config: Config, filter: String) -> Result<Self, Box<dyn Error>> {
        let mut app = App {
            db,
            config,
            filter,
            branches: Vec::new(),
            selected_branch_idx: 0,
            selected_item: SelectedItem::Branch,
            confirm_dialog: None,
        };

        app.refresh_data()?;
        Ok(app)
    }

    fn refresh_data(&mut self) -> Result<(), Box<dyn Error>> {
        let branch_data = self.db.get_branches_with_data(&self.filter)?;

        self.branches = branch_data
            .into_iter()
            .map(|data| {
                // Check dirty/WIP state for each worktree
                let worktree_states: Vec<WorktreeState> = data
                    .worktrees
                    .iter()
                    .map(|wt| {
                        let worktree = Worktree {
                            path: wt.path.clone(),
                            branch: Some(wt.branch.clone()),
                        };
                        WorktreeState {
                            is_dirty: worktree.is_dirty(),
                            has_wip: worktree.has_wip_commit(),
                        }
                    })
                    .collect();

                BranchNode {
                    data,
                    selected_worktree_idx: 0,
                    selected_sessions: HashSet::new(),
                    expanded: true,
                    worktree_states,
                }
            })
            .collect();

        // Reset selection if out of bounds
        if self.selected_branch_idx >= self.branches.len() {
            self.selected_branch_idx = 0;
        }

        Ok(())
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Action {
        // Handle confirmation dialog
        if self.confirm_dialog.is_some() {
            return self.handle_confirm_key(key);
        }

        match key {
            KeyCode::Up => {
                self.move_up();
                Action::Continue
            }
            KeyCode::Down => {
                self.move_down();
                Action::Continue
            }
            KeyCode::Left => {
                self.cycle_worktree(-1);
                Action::Continue
            }
            KeyCode::Right => {
                self.cycle_worktree(1);
                Action::Continue
            }
            KeyCode::Char(' ') if self.filter.is_empty() => {
                self.toggle_session();
                Action::Continue
            }
            KeyCode::Enter => self.confirm_selection(),
            KeyCode::Esc => {
                if !self.filter.is_empty() {
                    self.filter.clear();
                    let _ = self.refresh_data();
                }
                Action::Continue
            }
            KeyCode::Backspace => {
                self.filter.pop();
                let _ = self.refresh_data();
                Action::Continue
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                let _ = self.refresh_data();
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    fn handle_confirm_key(&mut self, key: KeyCode) -> Action {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(ref dialog) = self.confirm_dialog {
                    // Create WIP commit
                    let _ = git::create_wip_commit(&dialog.worktree_path);
                }
                self.confirm_dialog = None;
                // Re-check and proceed with launch
                self.do_launch()
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.confirm_dialog = None;
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    fn move_up(&mut self) {
        if self.branches.is_empty() {
            return;
        }

        match self.selected_item {
            SelectedItem::Branch => {
                if self.selected_branch_idx > 0 {
                    self.selected_branch_idx -= 1;
                    // Move to last session of previous branch if expanded
                    let branch = &self.branches[self.selected_branch_idx];
                    if branch.expanded && !branch.data.sessions.is_empty() {
                        self.selected_item = SelectedItem::Session(branch.data.sessions.len() - 1);
                    }
                }
            }
            SelectedItem::Session(idx) => {
                if idx > 0 {
                    self.selected_item = SelectedItem::Session(idx - 1);
                } else {
                    self.selected_item = SelectedItem::Branch;
                }
            }
        }
    }

    fn move_down(&mut self) {
        if self.branches.is_empty() {
            return;
        }

        let branch = &self.branches[self.selected_branch_idx];

        match self.selected_item {
            SelectedItem::Branch => {
                if branch.expanded && !branch.data.sessions.is_empty() {
                    self.selected_item = SelectedItem::Session(0);
                } else if self.selected_branch_idx < self.branches.len() - 1 {
                    self.selected_branch_idx += 1;
                    self.selected_item = SelectedItem::Branch;
                }
            }
            SelectedItem::Session(idx) => {
                if idx < branch.data.sessions.len() - 1 {
                    self.selected_item = SelectedItem::Session(idx + 1);
                } else if self.selected_branch_idx < self.branches.len() - 1 {
                    self.selected_branch_idx += 1;
                    self.selected_item = SelectedItem::Branch;
                }
            }
        }
    }

    fn cycle_worktree(&mut self, delta: i32) {
        if self.branches.is_empty() {
            return;
        }

        let branch = &mut self.branches[self.selected_branch_idx];
        if branch.data.worktrees.is_empty() {
            return;
        }

        let len = branch.data.worktrees.len() as i32;
        let new_idx = (branch.selected_worktree_idx as i32 + delta).rem_euclid(len);
        branch.selected_worktree_idx = new_idx as usize;
    }

    fn toggle_session(&mut self) {
        if let SelectedItem::Session(idx) = self.selected_item {
            if let Some(branch) = self.branches.get_mut(self.selected_branch_idx) {
                if let Some(session) = branch.data.sessions.get(idx) {
                    let uuid = session.uuid.clone();
                    if branch.selected_sessions.contains(&uuid) {
                        branch.selected_sessions.remove(&uuid);
                    } else {
                        branch.selected_sessions.insert(uuid);
                    }
                }
            }
        }
    }

    fn confirm_selection(&mut self) -> Action {
        if self.branches.is_empty() {
            return Action::Continue;
        }

        let branch = &self.branches[self.selected_branch_idx];
        if branch.data.worktrees.is_empty() {
            return Action::Continue;
        }

        let worktree = &branch.data.worktrees[branch.selected_worktree_idx];
        let state = &branch.worktree_states[branch.selected_worktree_idx];

        // If has WIP commit, undo it first
        if state.has_wip {
            let _ = git::undo_wip_commit(&worktree.path);
        }

        // If dirty, show confirmation dialog
        if state.is_dirty {
            self.confirm_dialog = Some(ConfirmDialog {
                message: format!(
                    "Worktree '{}' has uncommitted changes.\nCreate WIP commit?",
                    worktree.repo_name
                ),
                worktree_path: worktree.path.clone(),
            });
            return Action::Continue;
        }

        self.do_launch()
    }

    fn do_launch(&self) -> Action {
        Action::Launch
    }

    pub fn launch_selection(&self) -> Result<(), Box<dyn Error>> {
        if self.branches.is_empty() {
            return Ok(());
        }

        let branch = &self.branches[self.selected_branch_idx];
        if branch.data.worktrees.is_empty() {
            return Ok(());
        }

        let worktree = &branch.data.worktrees[branch.selected_worktree_idx];

        // Generate and launch editor config
        let editor_config = actions::generate_editor_config(&worktree.path, &self.config.editor)?;
        actions::open_config(&editor_config)?;

        // Generate and launch session configs
        for uuid in &branch.selected_sessions {
            if let Some(session) = branch.data.sessions.iter().find(|s| &s.uuid == uuid) {
                let title = session
                    .summary
                    .as_ref()
                    .or(session.first_prompt.as_ref())
                    .map(|s| truncate(s, 30))
                    .unwrap_or_else(|| "Claude session".to_string());

                let session_config =
                    actions::generate_session_config(&session.uuid, &worktree.path, &title)?;
                actions::open_config(&session_config)?;
            }
        }

        Ok(())
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
