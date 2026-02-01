use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Session {
    pub uuid: String,
    pub project_path: String,
    pub git_branch: Option<String>,
    pub summary: Option<String>,
    pub first_prompt: Option<String>,
    pub modified: i64,
}

#[derive(Deserialize)]
struct SessionIndexEntry {
    #[serde(rename = "sessionUuid")]
    session_uuid: String,
    #[serde(rename = "projectPath")]
    project_path: String,
    #[serde(rename = "gitBranch")]
    git_branch: Option<String>,
    summary: Option<String>,
    #[serde(rename = "firstPrompt")]
    first_prompt: Option<String>,
    modified: i64,
}

pub fn scan_sessions() -> Result<Vec<Session>, Box<dyn Error>> {
    let claude_dir = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join(".claude")
        .join("projects");

    if !claude_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    // Glob ~/.claude/projects/*/sessions-index.json
    let pattern = claude_dir.join("*").join("sessions-index.json");
    let pattern_str = pattern.to_string_lossy();

    for entry in glob::glob(&pattern_str)? {
        if let Ok(path) = entry {
            if let Ok(parsed) = parse_sessions_index(&path) {
                sessions.extend(parsed);
            }
        }
    }

    Ok(sessions)
}

fn parse_sessions_index(path: &PathBuf) -> Result<Vec<Session>, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let entries: Vec<SessionIndexEntry> = serde_json::from_str(&contents)?;

    let sessions = entries
        .into_iter()
        .map(|e| Session {
            uuid: e.session_uuid,
            project_path: e.project_path,
            git_branch: e.git_branch,
            summary: e.summary,
            first_prompt: e.first_prompt,
            modified: e.modified,
        })
        .collect();

    Ok(sessions)
}
