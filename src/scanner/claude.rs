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
    pub message_count: Option<i64>,
}

#[derive(Deserialize)]
struct SessionIndex {
    entries: Vec<SessionIndexEntry>,
}

#[derive(Deserialize)]
struct SessionIndexEntry {
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(rename = "projectPath")]
    project_path: String,
    #[serde(rename = "gitBranch")]
    git_branch: Option<String>,
    summary: Option<String>,
    #[serde(rename = "firstPrompt")]
    first_prompt: Option<String>,
    modified: String, // ISO 8601 date string
    #[serde(rename = "messageCount")]
    message_count: Option<i64>,
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
    let index: SessionIndex = serde_json::from_str(&contents)?;

    let sessions = index
        .entries
        .into_iter()
        .map(|e| {
            // Parse ISO 8601 date to unix timestamp (ms)
            let modified = chrono::DateTime::parse_from_rfc3339(&e.modified)
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0);

            Session {
                uuid: e.session_id,
                project_path: e.project_path,
                git_branch: e.git_branch,
                summary: e.summary,
                first_prompt: e.first_prompt,
                modified,
                message_count: e.message_count,
            }
        })
        .collect();

    Ok(sessions)
}
