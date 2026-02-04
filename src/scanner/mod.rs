pub mod claude;
pub mod codex;
pub mod git;

/// Identifies which AI assistant a session belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionProvider {
    Claude,
    Codex,
}

impl SessionProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionProvider::Claude => "claude",
            SessionProvider::Codex => "codex",
        }
    }
}

// Re-export Session for convenience
pub use claude::Session;
