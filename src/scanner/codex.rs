use super::{Session, SessionProvider};
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Deserialize)]
struct SessionMeta {
    #[serde(rename = "type")]
    entry_type: String,
    payload: SessionPayload,
}

#[derive(Deserialize)]
struct SessionPayload {
    id: String,
    cwd: Option<String>,
    git: Option<GitInfo>,
}

#[derive(Deserialize)]
struct GitInfo {
    branch: Option<String>,
}

#[derive(Deserialize)]
struct HistoryEntry {
    session_id: String,
    text: String,
}

pub fn scan_sessions() -> Result<Vec<Session>, Box<dyn Error>> {
    let codex_dir = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join(".codex")
        .join("sessions");

    if !codex_dir.exists() {
        return Ok(Vec::new());
    }

    // Load history for first prompts
    let first_prompts = load_history()?;

    let mut sessions = Vec::new();

    // Glob ~/.codex/sessions/YYYY/MM/DD/*.jsonl
    let pattern = codex_dir.join("*/*/*/*.jsonl");
    let pattern_str = pattern.to_string_lossy();

    for entry in glob::glob(&pattern_str)? {
        if let Ok(path) = entry {
            match parse_session_file(&path, &first_prompts) {
                Ok(session) => sessions.push(session),
                Err(e) => {
                    // Log but continue - handle missing fields gracefully
                    eprintln!("Warning: failed to parse {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(sessions)
}

fn load_history() -> Result<HashMap<String, String>, Box<dyn Error>> {
    let history_path = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join(".codex")
        .join("history.jsonl");

    let mut prompts = HashMap::new();

    if !history_path.exists() {
        return Ok(prompts);
    }

    let file = File::open(&history_path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        if let Ok(line) = line {
            if let Ok(entry) = serde_json::from_str::<HistoryEntry>(&line) {
                // Only keep first prompt per session
                prompts.entry(entry.session_id).or_insert(entry.text);
            }
        }
    }

    Ok(prompts)
}

fn parse_session_file(
    path: &PathBuf,
    first_prompts: &HashMap<String, String>,
) -> Result<Session, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read first line only
    let first_line = reader.lines().next().ok_or("Empty session file")??;

    let meta: SessionMeta = serde_json::from_str(&first_line)?;

    if meta.entry_type != "session_meta" {
        return Err("First line is not session_meta".into());
    }

    // Get modified time from file metadata
    let modified = fs::metadata(path)?
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as i64;

    let git_branch = meta.payload.git.and_then(|g| g.branch);
    let first_prompt = first_prompts.get(&meta.payload.id).cloned();

    Ok(Session {
        uuid: meta.payload.id.clone(),
        project_path: meta.payload.cwd.unwrap_or_default(),
        git_branch,
        summary: None, // Codex doesn't have summaries
        first_prompt,
        modified,
        message_count: None, // Could count lines, but expensive
        provider: SessionProvider::Codex,
    })
}
