use crate::config::Config;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct Repo {
    pub id: i64,
    pub path: PathBuf,
    pub name: String,
    pub worktrees: Vec<Worktree>,
}

#[derive(Debug, Clone)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: Option<String>,
}

impl Worktree {
    /// Check if worktree has uncommitted changes (expensive, call sparingly)
    pub fn is_dirty(&self) -> bool {
        let output = Command::new("git")
            .args([
                "-C",
                self.path.to_str().unwrap_or(""),
                "status",
                "--porcelain",
            ])
            .output();

        match output {
            Ok(out) => !out.stdout.is_empty(),
            Err(_) => false,
        }
    }

    /// Check if the most recent commit is a WIP commit
    pub fn has_wip_commit(&self) -> bool {
        let output = Command::new("git")
            .args([
                "-C",
                self.path.to_str().unwrap_or(""),
                "log",
                "-1",
                "--format=%s",
            ])
            .output();

        match output {
            Ok(out) => {
                let msg = String::from_utf8_lossy(&out.stdout);
                msg.trim() == "WIP: paused work"
            }
            Err(_) => false,
        }
    }
}

pub fn scan_repos(scan_dirs: &[String]) -> Result<Vec<Repo>, Box<dyn Error>> {
    let mut repos = Vec::new();
    let mut id_counter = 0i64;

    for dir in scan_dirs {
        let expanded = Config::expand_path(dir);
        if !expanded.exists() {
            continue;
        }

        // Walk one level deep to find git repos
        for entry in WalkDir::new(&expanded).min_depth(1).max_depth(1) {
            let entry = entry?;
            if entry.file_type().is_dir() {
                let git_dir = entry.path().join(".git");
                if git_dir.exists() {
                    if let Ok(repo) = scan_single_repo(entry.path(), &mut id_counter) {
                        repos.push(repo);
                    }
                }
            }
        }
    }

    Ok(repos)
}

fn scan_single_repo(path: &Path, id_counter: &mut i64) -> Result<Repo, Box<dyn Error>> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let worktrees = parse_worktree_list(path)?;

    *id_counter += 1;
    Ok(Repo {
        id: *id_counter,
        path: path.to_path_buf(),
        name,
        worktrees,
    })
}

fn parse_worktree_list(repo_path: &Path) -> Result<Vec<Worktree>, Box<dyn Error>> {
    let output = Command::new("git")
        .args([
            "-C",
            repo_path.to_str().unwrap_or(""),
            "worktree",
            "list",
            "--porcelain",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in stdout.lines() {
        if line.starts_with("worktree ") {
            // Save previous worktree if any
            if let Some(path) = current_path.take() {
                worktrees.push(Worktree {
                    path,
                    branch: current_branch.take(),
                });
            }
            current_path = Some(PathBuf::from(&line[9..]));
        } else if line.starts_with("branch ") {
            // Extract branch name from refs/heads/...
            let full_ref = &line[7..];
            let branch = full_ref
                .strip_prefix("refs/heads/")
                .unwrap_or(full_ref)
                .to_string();
            current_branch = Some(branch);
        } else if line.starts_with("detached") {
            current_branch = None;
        }
    }

    // Save last worktree
    if let Some(path) = current_path {
        worktrees.push(Worktree {
            path,
            branch: current_branch,
        });
    }

    Ok(worktrees)
}

/// Create WIP commit in the given worktree
pub fn create_wip_commit(worktree_path: &Path) -> Result<(), Box<dyn Error>> {
    let path_str = worktree_path.to_str().ok_or("Invalid path")?;

    // Stage all changes
    let status = Command::new("git")
        .args(["-C", path_str, "add", "-A"])
        .status()?;
    if !status.success() {
        return Err("Failed to stage changes".into());
    }

    // Commit (ignore error if nothing to commit)
    let _ = Command::new("git")
        .args(["-C", path_str, "commit", "-m", "WIP: paused work"])
        .status();

    Ok(())
}

/// Undo WIP commit (soft reset)
pub fn undo_wip_commit(worktree_path: &Path) -> Result<(), Box<dyn Error>> {
    let path_str = worktree_path.to_str().ok_or("Invalid path")?;

    let status = Command::new("git")
        .args(["-C", path_str, "reset", "--soft", "HEAD~1"])
        .status()?;

    if !status.success() {
        return Err("Failed to undo WIP commit".into());
    }

    Ok(())
}
