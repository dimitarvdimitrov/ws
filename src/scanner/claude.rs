use super::SessionProvider;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
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
    pub provider: SessionProvider,
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

    // Glob ~/.claude/projects/*/*.jsonl
    let pattern = claude_dir.join("*").join("*.jsonl");
    let pattern_str = pattern.to_string_lossy();

    for entry in glob::glob(&pattern_str)? {
        if let Ok(path) = entry {
            match parse_jsonl_session(&path) {
                Ok(session) => sessions.push(session),
                Err(e) => {
                    eprintln!("Warning: failed to parse {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(sessions)
}

/// Parse a single JSONL session file into a Session.
///
/// Extracts metadata by reading lines one at a time:
/// - `cwd` and `gitBranch` from the first line that has them.
/// - `first_prompt` from the first `type: "user"` line with a string `message.content`.
/// - `summary` from a `type: "summary"` line (if present).
/// - `message_count` as the count of `type: "user"` lines.
/// - `modified` from file mtime (reliable proxy since Claude writes as the session progresses).
fn parse_jsonl_session(path: &PathBuf) -> Result<Session, Box<dyn Error>> {
    let uuid = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?
        .to_string();

    let modified = fs::metadata(path)?
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as i64;

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut cwd: Option<String> = None;
    let mut git_branch: Option<String> = None;
    let mut first_prompt: Option<String> = None;
    let mut summary: Option<String> = None;
    let mut message_count: i64 = 0;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let value: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Extract cwd and gitBranch from the first line that has them.
        if cwd.is_none() {
            if let Some(c) = value.get("cwd").and_then(|v| v.as_str()) {
                cwd = Some(c.to_string());
            }
        }
        if git_branch.is_none() {
            if let Some(b) = value.get("gitBranch").and_then(|v| v.as_str()) {
                git_branch = Some(b.to_string());
            }
        }

        let line_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match line_type {
            "user" => {
                message_count += 1;

                // Extract first prompt from first user message with string content.
                if first_prompt.is_none() {
                    if let Some(content) = value
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str())
                    {
                        first_prompt = Some(content.to_string());
                    }
                }
            }
            "summary" => {
                if let Some(s) = value.get("summary").and_then(|v| v.as_str()) {
                    summary = Some(s.to_string());
                }
            }
            _ => {}
        }
    }

    Ok(Session {
        uuid,
        project_path: cwd.unwrap_or_default(),
        git_branch,
        summary,
        first_prompt,
        modified,
        message_count: Some(message_count),
        provider: SessionProvider::Claude,
    })
}
