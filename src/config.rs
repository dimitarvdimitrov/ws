use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_scan_dirs")]
    pub scan_dirs: Vec<String>,

    #[serde(default = "default_editor")]
    pub editor: String,

    #[serde(default = "default_scan_on_open")]
    pub scan_on_open: bool,
}

fn default_scan_on_open() -> bool {
    true
}

fn default_scan_dirs() -> Vec<String> {
    vec!["~/Documents".to_string()]
}

fn default_editor() -> String {
    std::env::var("EDITOR").unwrap_or_else(|_| "code".to_string())
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan_dirs: default_scan_dirs(),
            editor: default_editor(),
            scan_on_open: default_scan_on_open(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&contents)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    fn config_path() -> Result<PathBuf, Box<dyn Error>> {
        // Check ~/.config/ws first (XDG style), then platform default
        if let Some(home) = dirs::home_dir() {
            let xdg_path = home.join(".config/ws/config.toml");
            if xdg_path.exists() {
                return Ok(xdg_path);
            }
        }

        let config_dir = dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("ws");
        Ok(config_dir.join("config.toml"))
    }

    pub fn expand_path(path: &str) -> PathBuf {
        if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(&path[2..]);
            }
        }
        PathBuf::from(path)
    }
}
