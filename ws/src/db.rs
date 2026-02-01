use crate::scanner::{claude::Session, git::Repo};
use rusqlite::{Connection, params};
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct RepoData {
    pub name: String,
    pub worktrees: Vec<WorktreeInfo>,  // All worktrees in repo
    pub branches: Vec<BranchData>,
}

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub name: String,                        // folder name for display
    pub checked_out_branch: Option<String>,  // which branch is checked out
}

#[derive(Debug, Clone)]
pub struct BranchData {
    pub branch: String,
    pub sessions: Vec<SessionData>,
}

#[derive(Debug, Clone)]
pub struct SessionData {
    pub uuid: String,
    pub summary: Option<String>,
    pub first_prompt: Option<String>,
}

impl Database {
    pub fn open() -> Result<Self, Box<dyn Error>> {
        let db_path = Self::db_path()?;

        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn db_path() -> Result<PathBuf, Box<dyn Error>> {
        let config_dir = dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("ws");
        Ok(config_dir.join("ws.db"))
    }

    fn init_schema(&self) -> Result<(), Box<dyn Error>> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS repos (
                id INTEGER PRIMARY KEY,
                path TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL,
                last_scanned INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS worktrees (
                id INTEGER PRIMARY KEY,
                repo_id INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
                path TEXT UNIQUE NOT NULL,
                branch TEXT,
                UNIQUE(repo_id, path)
            );

            CREATE TABLE IF NOT EXISTS sessions (
                uuid TEXT PRIMARY KEY,
                project_path TEXT NOT NULL,
                git_branch TEXT,
                summary TEXT,
                first_prompt TEXT,
                modified INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_sessions_branch ON sessions(git_branch);
            CREATE INDEX IF NOT EXISTS idx_worktrees_branch ON worktrees(branch);
            "#,
        )?;
        Ok(())
    }

    pub fn upsert_repo(&mut self, repo: &Repo) -> Result<(), Box<dyn Error>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO repos (path, name, last_scanned)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(path) DO UPDATE SET
                name = excluded.name,
                last_scanned = excluded.last_scanned",
            params![repo.path.to_string_lossy(), repo.name, now],
        )?;
        Ok(())
    }

    pub fn upsert_worktree(
        &mut self,
        repo_path: &std::path::Path,
        worktree: &crate::scanner::git::Worktree,
    ) -> Result<(), Box<dyn Error>> {
        // Get repo_id from repo path
        let repo_id: i64 = self.conn.query_row(
            "SELECT id FROM repos WHERE path = ?1",
            params![repo_path.to_string_lossy()],
            |row| row.get(0),
        )?;

        self.conn.execute(
            "INSERT INTO worktrees (repo_id, path, branch)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(path) DO UPDATE SET
                repo_id = excluded.repo_id,
                branch = excluded.branch",
            params![repo_id, worktree.path.to_string_lossy(), worktree.branch],
        )?;
        Ok(())
    }

    pub fn upsert_session(&mut self, session: &Session) -> Result<(), Box<dyn Error>> {
        self.conn.execute(
            "INSERT INTO sessions (uuid, project_path, git_branch, summary, first_prompt, modified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(uuid) DO UPDATE SET
                project_path = excluded.project_path,
                git_branch = excluded.git_branch,
                summary = excluded.summary,
                first_prompt = excluded.first_prompt,
                modified = excluded.modified",
            params![
                session.uuid,
                session.project_path,
                session.git_branch,
                session.summary,
                session.first_prompt,
                session.modified
            ],
        )?;
        Ok(())
    }

    pub fn delete_stale_repos(&mut self, current_repos: &[Repo]) -> Result<(), Box<dyn Error>> {
        let current_paths: HashSet<_> = current_repos
            .iter()
            .map(|r| r.path.to_string_lossy().to_string())
            .collect();

        let mut stmt = self.conn.prepare("SELECT path FROM repos")?;
        let all_paths: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();

        for path in all_paths {
            if !current_paths.contains(&path) {
                self.conn
                    .execute("DELETE FROM repos WHERE path = ?1", params![path])?;
            }
        }

        Ok(())
    }

    pub fn delete_stale_sessions(
        &mut self,
        current_sessions: &[Session],
    ) -> Result<(), Box<dyn Error>> {
        let current_uuids: HashSet<_> = current_sessions.iter().map(|s| s.uuid.clone()).collect();

        let mut stmt = self.conn.prepare("SELECT uuid FROM sessions")?;
        let all_uuids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();

        for uuid in all_uuids {
            if !current_uuids.contains(&uuid) {
                self.conn
                    .execute("DELETE FROM sessions WHERE uuid = ?1", params![uuid])?;
            }
        }

        Ok(())
    }

    /// Get repos with their branches and sessions, filtered by search string
    pub fn get_repos_with_data(&self, filter: &str) -> Result<Vec<RepoData>, Box<dyn Error>> {
        let filter_pattern = format!("%{}%", filter.to_lowercase());

        // Get all repos that have worktrees matching the filter (by repo name or branch)
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT r.id, r.name
             FROM repos r
             JOIN worktrees w ON w.repo_id = r.id
             WHERE w.branch IS NOT NULL
               AND (LOWER(r.name) LIKE ?1 OR LOWER(w.branch) LIKE ?1)
             ORDER BY r.name",
        )?;

        let repos: Vec<(i64, String)> = stmt
            .query_map(params![filter_pattern], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .filter_map(Result::ok)
            .collect();

        let mut result = Vec::new();

        for (repo_id, repo_name) in repos {
            let worktrees = self.get_worktrees_for_repo(repo_id)?;
            let branches = self.get_branches_for_repo(repo_id, &filter_pattern)?;
            if !branches.is_empty() {
                result.push(RepoData {
                    name: repo_name,
                    worktrees,
                    branches,
                });
            }
        }

        Ok(result)
    }

    /// Get all worktrees for a repo (unfiltered)
    fn get_worktrees_for_repo(&self, repo_id: i64) -> Result<Vec<WorktreeInfo>, Box<dyn Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT w.path, w.branch
             FROM worktrees w
             WHERE w.repo_id = ?1
             ORDER BY w.path",
        )?;

        let worktrees = stmt
            .query_map(params![repo_id], |row| {
                let path_str: String = row.get(0)?;
                let branch: Option<String> = row.get(1)?;
                let path = PathBuf::from(&path_str);
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| path_str.clone());
                Ok(WorktreeInfo {
                    path,
                    name,
                    checked_out_branch: branch,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(worktrees)
    }

    fn get_branches_for_repo(
        &self,
        repo_id: i64,
        filter_pattern: &str,
    ) -> Result<Vec<BranchData>, Box<dyn Error>> {
        // Get distinct branches that match filter
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT w.branch
             FROM worktrees w
             JOIN repos r ON w.repo_id = r.id
             WHERE w.repo_id = ?1
               AND w.branch IS NOT NULL
               AND (LOWER(r.name) LIKE ?2 OR LOWER(w.branch) LIKE ?2)
             ORDER BY w.branch",
        )?;

        let branches: Vec<String> = stmt
            .query_map(params![repo_id, filter_pattern], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();

        let mut result = Vec::new();
        for branch in branches {
            let sessions = self.get_sessions_for_branch(&branch)?;
            result.push(BranchData { branch, sessions });
        }

        Ok(result)
    }

    fn get_sessions_for_branch(&self, branch: &str) -> Result<Vec<SessionData>, Box<dyn Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT uuid, summary, first_prompt
             FROM sessions
             WHERE git_branch = ?1
             ORDER BY modified DESC",
        )?;

        let sessions = stmt
            .query_map(params![branch], |row| {
                Ok(SessionData {
                    uuid: row.get(0)?,
                    summary: row.get(1)?,
                    first_prompt: row.get(2)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(sessions)
    }
}
