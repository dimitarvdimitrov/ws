use crate::actions;
use crate::config::Config;
use crate::db::{BranchData, Database, RepoData};
use crate::migrate;
use crate::scanner::git::Worktree;
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
}

#[derive(Clone, Default)]
pub struct PendingLaunch {
    pub pre_commands: Vec<String>,
}

pub struct RepoNode {
    pub data: RepoData,
    pub branches: Vec<BranchNode>,
    pub worktree_states: Vec<WorktreeState>, // Runtime state for each worktree
    pub expanded: bool,
}

pub struct BranchNode {
    pub selected_worktree_idx: usize, // Index into repo's worktrees
    pub selected_sessions: HashSet<String>, // UUIDs of selected sessions
    pub expanded: bool,
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
    pub repos: Vec<RepoNode>,
    pub selected_repo_idx: usize,
    pub selected_branch_idx: usize,
    pub selected_item: SelectedItem,
    pub confirm_dialog: Option<ConfirmDialog>,
    pub pending_launch: PendingLaunch,
    pub scroll_offset: u16,
    pub viewport_height: u16,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SelectedItem {
    Repo,
    Branch,
    Session(usize), // Index within branch's sessions
}

impl App {
    pub fn new(db: Database, config: Config, filter: String) -> Result<Self, Box<dyn Error>> {
        let mut app = App {
            db,
            config,
            filter,
            repos: Vec::new(),
            selected_repo_idx: 0,
            selected_branch_idx: 0,
            selected_item: SelectedItem::Repo,
            confirm_dialog: None,
            pending_launch: PendingLaunch::default(),
            scroll_offset: 0,
            viewport_height: 0,
        };

        app.refresh_data()?;
        Ok(app)
    }

    fn refresh_data(&mut self) -> Result<(), Box<dyn Error>> {
        let repo_data = self.db.get_repos_with_data(&self.filter)?;

        self.repos = repo_data
            .into_iter()
            .map(|data| {
                // Compute worktree states for all worktrees in this repo
                let worktree_states: Vec<WorktreeState> = data
                    .worktrees
                    .iter()
                    .map(|wt| {
                        let worktree = Worktree {
                            path: wt.path.clone(),
                            branch: wt.checked_out_branch.clone(),
                        };
                        WorktreeState {
                            is_dirty: worktree.is_dirty(),
                            has_wip: worktree.has_wip_commit(),
                        }
                    })
                    .collect();

                let branches: Vec<BranchNode> = data
                    .branches
                    .iter()
                    .map(|branch_data| {
                        // Pre-select worktree where this branch is checked out
                        let selected_worktree_idx = data
                            .worktrees
                            .iter()
                            .position(|wt| {
                                wt.checked_out_branch
                                    .as_ref()
                                    .map_or(false, |b| b == &branch_data.branch)
                            })
                            .unwrap_or(0);
                        BranchNode {
                            selected_worktree_idx,
                            selected_sessions: HashSet::new(),
                            expanded: true,
                        }
                    })
                    .collect();

                RepoNode {
                    data,
                    branches,
                    worktree_states,
                    expanded: true,
                }
            })
            .collect();

        // Reset selection if out of bounds
        if self.selected_repo_idx >= self.repos.len() {
            self.selected_repo_idx = 0;
        }
        if let Some(repo) = self.repos.get(self.selected_repo_idx) {
            if self.selected_branch_idx >= repo.branches.len() {
                self.selected_branch_idx = 0;
            }
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
                self.ensure_selection_visible();
                Action::Continue
            }
            KeyCode::Down => {
                self.move_down();
                self.ensure_selection_visible();
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
            KeyCode::Char(' ') => {
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
                // Add WIP commit command
                self.pending_launch
                    .pre_commands
                    .push("git add -A && git commit -m 'WIP: paused work'".to_string());
                self.confirm_dialog = None;
                // Proceed with launch
                self.do_launch()
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.confirm_dialog = None;
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    fn current_repo(&self) -> Option<&RepoNode> {
        self.repos.get(self.selected_repo_idx)
    }

    fn current_repo_mut(&mut self) -> Option<&mut RepoNode> {
        self.repos.get_mut(self.selected_repo_idx)
    }

    fn current_branch(&self) -> Option<&BranchNode> {
        self.current_repo()
            .and_then(|repo| repo.branches.get(self.selected_branch_idx))
    }

    fn current_branch_mut(&mut self) -> Option<&mut BranchNode> {
        let branch_idx = self.selected_branch_idx;
        self.current_repo_mut()
            .and_then(|repo| repo.branches.get_mut(branch_idx))
    }

    /// Get branch data from repo.data.branches
    fn current_branch_data(&self) -> Option<&BranchData> {
        self.current_repo()
            .and_then(|repo| repo.data.branches.get(self.selected_branch_idx))
    }

    fn move_up(&mut self) {
        if self.repos.is_empty() {
            return;
        }

        match self.selected_item {
            SelectedItem::Repo => {
                if self.selected_repo_idx > 0 {
                    self.selected_repo_idx -= 1;
                    // Move to last item of previous repo if expanded
                    if let Some(repo) = self.repos.get(self.selected_repo_idx) {
                        if repo.expanded && !repo.branches.is_empty() {
                            let last_branch_idx = repo.branches.len() - 1;
                            let branch = &repo.branches[last_branch_idx];
                            let branch_data = &repo.data.branches[last_branch_idx];
                            self.selected_branch_idx = last_branch_idx;
                            if branch.expanded && !branch_data.sessions.is_empty() {
                                self.selected_item =
                                    SelectedItem::Session(branch_data.sessions.len() - 1);
                            } else {
                                self.selected_item = SelectedItem::Branch;
                            }
                        }
                    }
                }
            }
            SelectedItem::Branch => {
                if self.selected_branch_idx > 0 {
                    self.selected_branch_idx -= 1;
                    // Move to last session of previous branch if expanded
                    if let (Some(branch), Some(branch_data)) =
                        (self.current_branch(), self.current_branch_data())
                    {
                        if branch.expanded && !branch_data.sessions.is_empty() {
                            self.selected_item =
                                SelectedItem::Session(branch_data.sessions.len() - 1);
                        }
                    }
                } else {
                    // Move to repo
                    self.selected_item = SelectedItem::Repo;
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
        if self.repos.is_empty() {
            return;
        }

        match self.selected_item {
            SelectedItem::Repo => {
                if let Some(repo) = self.current_repo() {
                    if repo.expanded && !repo.branches.is_empty() {
                        self.selected_branch_idx = 0;
                        self.selected_item = SelectedItem::Branch;
                    } else if self.selected_repo_idx < self.repos.len() - 1 {
                        self.selected_repo_idx += 1;
                    }
                }
            }
            SelectedItem::Branch => {
                let should_go_to_session = self.current_branch().map_or(false, |b| b.expanded)
                    && self
                        .current_branch_data()
                        .map_or(false, |bd| !bd.sessions.is_empty());

                if should_go_to_session {
                    self.selected_item = SelectedItem::Session(0);
                } else if let Some(repo) = self.current_repo() {
                    if self.selected_branch_idx < repo.branches.len() - 1 {
                        self.selected_branch_idx += 1;
                    } else if self.selected_repo_idx < self.repos.len() - 1 {
                        self.selected_repo_idx += 1;
                        self.selected_branch_idx = 0;
                        self.selected_item = SelectedItem::Repo;
                    }
                }
            }
            SelectedItem::Session(idx) => {
                let sessions_len = self.current_branch_data().map_or(0, |bd| bd.sessions.len());
                if idx < sessions_len - 1 {
                    self.selected_item = SelectedItem::Session(idx + 1);
                } else if let Some(repo) = self.current_repo() {
                    if self.selected_branch_idx < repo.branches.len() - 1 {
                        self.selected_branch_idx += 1;
                        self.selected_item = SelectedItem::Branch;
                    } else if self.selected_repo_idx < self.repos.len() - 1 {
                        self.selected_repo_idx += 1;
                        self.selected_branch_idx = 0;
                        self.selected_item = SelectedItem::Repo;
                    }
                }
            }
        }
    }

    pub fn scroll_up(&mut self, lines: u16) {
        for _ in 0..lines {
            self.move_up();
        }
        self.ensure_selection_visible();
    }

    pub fn scroll_down(&mut self, lines: u16) {
        for _ in 0..lines {
            self.move_down();
        }
        self.ensure_selection_visible();
    }

    pub fn scroll_to_top(&mut self) {
        self.selected_repo_idx = 0;
        self.selected_branch_idx = 0;
        self.selected_item = SelectedItem::Repo;
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        if self.repos.is_empty() {
            return;
        }
        // Navigate to the last visible item
        loop {
            let prev_repo = self.selected_repo_idx;
            let prev_branch = self.selected_branch_idx;
            let prev_item = self.selected_item;
            self.move_down();
            if self.selected_repo_idx == prev_repo
                && self.selected_branch_idx == prev_branch
                && self.selected_item == prev_item
            {
                break;
            }
        }
        self.ensure_selection_visible();
    }

    /// Compute the line index of the current selection within the rendered tree
    pub fn selected_line_index(&self) -> usize {
        let mut line = 0;
        for (repo_idx, repo) in self.repos.iter().enumerate() {
            if repo_idx == self.selected_repo_idx && self.selected_item == SelectedItem::Repo {
                return line;
            }
            line += 1; // repo line

            if repo.expanded {
                for (branch_idx, branch) in repo.branches.iter().enumerate() {
                    let is_selected_branch = repo_idx == self.selected_repo_idx
                        && branch_idx == self.selected_branch_idx;

                    if is_selected_branch && self.selected_item == SelectedItem::Branch {
                        return line;
                    }
                    line += 1; // branch line

                    if branch.expanded {
                        let branch_data = &repo.data.branches[branch_idx];
                        for (session_idx, _) in branch_data.sessions.iter().enumerate() {
                            if is_selected_branch
                                && self.selected_item == SelectedItem::Session(session_idx)
                            {
                                return line;
                            }
                            line += 1;
                        }
                    }
                }
            }
        }
        line
    }

    /// Adjust scroll offset to keep selection visible
    pub fn ensure_selection_visible(&mut self) {
        let selected_line = self.selected_line_index() as u16;
        let viewport_height = self.viewport_height;

        if viewport_height == 0 {
            return;
        }

        // If selection is above viewport, scroll up to show it
        if selected_line < self.scroll_offset {
            self.scroll_offset = selected_line;
        }
        // If selection is below viewport, scroll down to show it
        else if selected_line >= self.scroll_offset + viewport_height {
            self.scroll_offset = selected_line - viewport_height + 1;
        }
    }

    fn cycle_worktree(&mut self, delta: i32) {
        // Get worktree count from repo
        let worktree_count = self
            .current_repo()
            .map_or(0, |repo| repo.data.worktrees.len());
        if worktree_count == 0 {
            return;
        }
        if let Some(branch) = self.current_branch_mut() {
            let len = worktree_count as i32;
            let new_idx = (branch.selected_worktree_idx as i32 + delta).rem_euclid(len);
            branch.selected_worktree_idx = new_idx as usize;
        }
    }

    fn toggle_session(&mut self) {
        if let SelectedItem::Session(idx) = self.selected_item {
            // Get session UUID from branch data
            let uuid = self
                .current_branch_data()
                .and_then(|bd| bd.sessions.get(idx))
                .map(|s| s.uuid.clone());

            if let Some(uuid) = uuid {
                if let Some(branch) = self.current_branch_mut() {
                    if branch.selected_sessions.contains(&uuid) {
                        branch.selected_sessions.remove(&uuid);
                    } else {
                        branch.selected_sessions.insert(uuid);
                    }
                }
            }
        }
    }

    fn toggle_expand(&mut self) {
        match self.selected_item {
            SelectedItem::Repo => {
                if let Some(repo) = self.current_repo_mut() {
                    repo.expanded = !repo.expanded;
                }
            }
            SelectedItem::Branch => {
                if let Some(branch) = self.current_branch_mut() {
                    branch.expanded = !branch.expanded;
                }
            }
            _ => {}
        }
    }

    fn confirm_selection(&mut self) -> Action {
        match self.selected_item {
            SelectedItem::Repo => {
                self.toggle_expand();
                Action::Continue
            }
            SelectedItem::Branch | SelectedItem::Session(_) => {
                // Extract needed state before modifying self
                let (has_wip, is_dirty, worktree_name, branch_name, checked_out_branch) = {
                    let repo = match self.current_repo() {
                        Some(r) => r,
                        None => return Action::Continue,
                    };

                    if repo.data.worktrees.is_empty() {
                        return Action::Continue;
                    }

                    let branch_node = match self.current_branch() {
                        Some(b) => b,
                        None => return Action::Continue,
                    };

                    let wt_idx = branch_node.selected_worktree_idx;
                    let state = &repo.worktree_states[wt_idx];
                    let worktree = &repo.data.worktrees[wt_idx];
                    let branch_data = &repo.data.branches[self.selected_branch_idx];

                    (
                        state.has_wip,
                        state.is_dirty,
                        worktree.name.clone(),
                        branch_data.branch.clone(),
                        worktree.checked_out_branch.clone(),
                    )
                };

                // Reset pending commands
                self.pending_launch = PendingLaunch::default();

                // If has WIP commit, add undo command
                if has_wip {
                    self.pending_launch
                        .pre_commands
                        .push("git reset --soft HEAD~1".to_string());
                }

                // If branch differs from what's checked out, add checkout command
                if checked_out_branch.as_ref() != Some(&branch_name) {
                    self.pending_launch
                        .pre_commands
                        .push(format!("git checkout {}", branch_name));
                }

                // If dirty, show confirmation dialog
                if is_dirty {
                    self.confirm_dialog = Some(ConfirmDialog {
                        message: format!(
                            "Worktree '{}' has uncommitted changes.\nCreate WIP commit?",
                            worktree_name
                        ),
                    });
                    return Action::Continue;
                }

                self.do_launch()
            }
        }
    }

    fn do_launch(&self) -> Action {
        Action::Launch
    }

    pub fn launch_selection(&self) -> Result<(), Box<dyn Error>> {
        let repo = match self.current_repo() {
            Some(r) => r,
            None => return Ok(()),
        };

        if repo.data.worktrees.is_empty() {
            return Ok(());
        }

        let branch = match self.current_branch() {
            Some(b) => b,
            None => return Ok(()),
        };

        let worktree = &repo.data.worktrees[branch.selected_worktree_idx];

        // Generate and launch editor config with any pending git commands
        let editor_config = actions::generate_editor_config(
            &worktree.path,
            &self.config.editor,
            &self.pending_launch.pre_commands,
        )?;
        actions::open_config(&editor_config)?;

        // Generate and launch session configs
        let branch_data = match self.current_branch_data() {
            Some(bd) => bd,
            None => return Ok(()),
        };

        for uuid in &branch.selected_sessions {
            if let Some(session) = branch_data.sessions.iter().find(|s| &s.uuid == uuid) {
                // Migrate session to target worktree if needed
                let source_path = PathBuf::from(&session.project_path);
                if source_path != worktree.path {
                    let _ = migrate::migrate_session(&session.uuid, &source_path, &worktree.path);
                }

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
