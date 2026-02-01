mod actions;
mod config;
mod db;
mod migrate;
mod scanner;
mod tui;

use clap::Parser;
use std::error::Error;

#[derive(Parser)]
#[command(name = "ws", about = "Git worktree & Claude session manager")]
struct Cli {
    /// Run background scan to update database
    #[arg(long)]
    scan: bool,

    /// Filter strings (all args become the initial filter)
    #[arg(trailing_var_arg = true)]
    filter: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if cli.scan {
        run_scan()?;
    } else {
        let filter = cli.filter.join(" ");
        run_tui(filter)?;
    }

    Ok(())
}

fn run_scan() -> Result<(), Box<dyn Error>> {
    let config = config::Config::load()?;
    run_scan_with_config(&config)
}

fn run_scan_with_config(config: &config::Config) -> Result<(), Box<dyn Error>> {
    let mut db = db::Database::open()?;

    // Scan git repos and worktrees
    let repos = scanner::git::scan_repos(&config.scan_dirs)?;
    for repo in &repos {
        db.upsert_repo(repo)?;
        for worktree in &repo.worktrees {
            db.upsert_worktree(&repo.path, worktree)?;
        }
    }

    // Scan Claude sessions
    let sessions = scanner::claude::scan_sessions()?;
    for session in &sessions {
        db.upsert_session(session)?;
    }

    // Cleanup stale entries
    db.delete_stale_repos(&repos)?;
    db.delete_stale_sessions(&sessions)?;

    Ok(())
}

fn run_tui(filter: String) -> Result<(), Box<dyn Error>> {
    // Cleanup old launch configs from previous runs
    actions::cleanup_old_configs()?;

    let config = config::Config::load()?;

    if config.scan_on_open {
        run_scan_with_config(&config)?;
    }

    let db = db::Database::open()?;

    tui::run(db, config, filter)?;

    Ok(())
}
