use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate Warp launch config for editor window
pub fn generate_editor_config(
    worktree_path: &PathBuf,
    editor: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let config_name = format!("ws-main-{}", timestamp);
    let config_path = warp_config_dir()?.join(format!("{}.yaml", config_name));

    let worktree_name = worktree_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "worktree".to_string());

    let yaml = format!(
        r#"---
name: {}
windows:
  - tabs:
      - title: {}
        layout:
          cwd: {}
          commands:
            - exec: {} .
"#,
        config_name,
        worktree_name,
        worktree_path.display(),
        editor
    );

    fs::write(&config_path, yaml)?;
    Ok(config_path)
}

/// Generate Warp launch config for a Claude session
pub fn generate_session_config(
    session_uuid: &str,
    worktree_path: &PathBuf,
    title: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let config_name = format!("ws-session-{}", &session_uuid[..8.min(session_uuid.len())]);
    let config_path = warp_config_dir()?.join(format!("{}.yaml", config_name));

    let yaml = format!(
        r#"---
name: {}
windows:
  - tabs:
      - title: {}
        layout:
          cwd: {}
          commands:
            - exec: claude --resume {}
"#,
        config_name,
        title.replace('"', "'"), // Escape quotes in title
        worktree_path.display(),
        session_uuid
    );

    fs::write(&config_path, yaml)?;
    Ok(config_path)
}

/// Open a Warp launch config
pub fn open_config(config_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    Command::new("open").arg(config_path).spawn()?;
    Ok(())
}

/// Cleanup old ws-* launch configs from previous runs
pub fn cleanup_old_configs() -> Result<(), Box<dyn Error>> {
    let config_dir = warp_config_dir()?;
    if !config_dir.exists() {
        return Ok(());
    }

    let pattern = config_dir.join("ws-*.yaml");
    let pattern_str = pattern.to_string_lossy();

    for entry in glob::glob(&pattern_str)? {
        if let Ok(path) = entry {
            let _ = fs::remove_file(path);
        }
    }

    Ok(())
}

fn warp_config_dir() -> Result<PathBuf, Box<dyn Error>> {
    let path = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join(".warp")
        .join("launch_configurations");

    // Ensure directory exists
    fs::create_dir_all(&path)?;

    Ok(path)
}
