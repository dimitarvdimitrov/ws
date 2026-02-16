use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// Convert a filesystem path to Claude's project directory name
/// e.g., /Users/dimitar/Documents/worktree -> -Users-dimitar-Documents-worktree
pub fn path_to_project_dir(path: &Path) -> String {
    path.to_string_lossy().replace('/', "-")
}

/// Get the Claude projects directory (~/.claude/projects)
fn claude_projects_dir() -> Result<PathBuf, Box<dyn Error>> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    Ok(home.join(".claude").join("projects"))
}

/// Migrate a Claude session from one worktree to another.
///
/// Copies the session JSONL file from the source project directory to the
/// target project directory. Claude Code resolves sessions directly from
/// JSONL files under ~/.claude/projects/<encoded-cwd>/, so having the file
/// present in the target directory is sufficient.
pub fn migrate_session(
    session_uuid: &str,
    source_project_path: &Path,
    target_project_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let projects_dir = claude_projects_dir()?;

    let source_dir_name = path_to_project_dir(source_project_path);
    let target_dir_name = path_to_project_dir(target_project_path);

    let source_project_dir = projects_dir.join(&source_dir_name);
    let target_project_dir = projects_dir.join(&target_dir_name);

    let source_jsonl = source_project_dir.join(format!("{}.jsonl", session_uuid));
    let target_jsonl = target_project_dir.join(format!("{}.jsonl", session_uuid));

    if !source_jsonl.exists() {
        return Err(format!("Session file not found: {:?}", source_jsonl).into());
    }

    if target_jsonl.exists() {
        // Already migrated
        return Ok(());
    }

    fs::create_dir_all(&target_project_dir)?;
    fs::copy(&source_jsonl, &target_jsonl)?;

    Ok(())
}
