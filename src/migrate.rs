use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// Entry in sessions-index.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIndexEntry {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "fullPath")]
    pub full_path: String,
    #[serde(rename = "projectPath")]
    pub project_path: String,
    #[serde(rename = "gitBranch")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(rename = "firstPrompt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_prompt: Option<String>,
    pub modified: String,
    #[serde(rename = "messageCount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionIndex {
    entries: Vec<SessionIndexEntry>,
}

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

/// Read sessions-index.json from a Claude project directory
fn read_sessions_index(project_dir: &Path) -> Result<SessionIndex, Box<dyn Error>> {
    let index_path = project_dir.join("sessions-index.json");
    if !index_path.exists() {
        return Ok(SessionIndex {
            entries: Vec::new(),
        });
    }
    let contents = fs::read_to_string(&index_path)?;
    let index: SessionIndex = serde_json::from_str(&contents)?;
    Ok(index)
}

/// Write sessions-index.json to a Claude project directory
fn write_sessions_index(project_dir: &Path, index: &SessionIndex) -> Result<(), Box<dyn Error>> {
    // Ensure directory exists
    fs::create_dir_all(project_dir)?;

    let index_path = project_dir.join("sessions-index.json");
    let contents = serde_json::to_string_pretty(index)?;
    fs::write(index_path, contents)?;
    Ok(())
}

/// Migrate a Claude session from one worktree to another.
///
/// This involves:
/// 1. Moving the session JSONL file from source to target project directory
/// 2. Removing the entry from source's sessions-index.json
/// 3. Adding the entry to target's sessions-index.json with updated paths
pub fn migrate_session(
    session_uuid: &str,
    source_project_path: &Path,
    target_project_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let projects_dir = claude_projects_dir()?;

    // Convert paths to Claude project directory names
    let source_dir_name = path_to_project_dir(source_project_path);
    let target_dir_name = path_to_project_dir(target_project_path);

    let source_project_dir = projects_dir.join(&source_dir_name);
    let target_project_dir = projects_dir.join(&target_dir_name);

    // Locate source JSONL file
    let source_jsonl = source_project_dir.join(format!("{}.jsonl", session_uuid));
    let target_jsonl = target_project_dir.join(format!("{}.jsonl", session_uuid));

    if !source_jsonl.exists() {
        return Err(format!("Session file not found: {:?}", source_jsonl).into());
    }

    // Read source index and find the entry
    let mut source_index = read_sessions_index(&source_project_dir)?;
    let entry_pos = source_index
        .entries
        .iter()
        .position(|e| e.session_id == session_uuid)
        .ok_or_else(|| format!("Session {} not found in source index", session_uuid))?;

    let mut entry = source_index.entries.remove(entry_pos);

    // Update entry paths for the target project
    entry.project_path = target_project_path.to_string_lossy().to_string();
    entry.full_path = target_jsonl.to_string_lossy().to_string();

    // Ensure target directory exists and move the file
    fs::create_dir_all(&target_project_dir)?;
    fs::rename(&source_jsonl, &target_jsonl)?;

    // Write updated source index
    write_sessions_index(&source_project_dir, &source_index)?;

    // Read target index and add the entry
    let mut target_index = read_sessions_index(&target_project_dir)?;
    target_index.entries.push(entry);
    write_sessions_index(&target_project_dir, &target_index)?;

    Ok(())
}
