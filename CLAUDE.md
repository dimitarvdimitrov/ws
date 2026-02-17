# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo run                # Launch interactive TUI
cargo run -- --scan      # Run background scan to update database
cargo run -- <filter>    # Launch TUI with initial filter text
```

After making changes, install the updated binary with `cargo install --path .` from the repo root.

No unit tests exist—this is a binary crate tested manually via the TUI.

## Architecture

**ws** is a terminal UI for managing git worktrees and Claude sessions. It uses Warp terminal's launch configuration system to open editors and resume sessions.

### Two Modes

1. **Scan mode** (`ws --scan`): Updates SQLite database by scanning:
   - Git repos from configured `scan_dirs` via `git worktree list --porcelain`
   - Claude sessions from `~/.claude/projects/*/*.jsonl`

2. **TUI mode** (`ws [filter]`): Interactive tree browser
   - Hierarchy: Repo → Branch → Session
   - Worktree selection via left/right arrows
   - Session multi-select with space, launch with Enter

### Key Data Flow

```
scanner/git.rs    → finds repos & worktrees
scanner/claude.rs → finds Claude sessions
db.rs             → SQLite persistence (~/.config/ws/ws.db)
tui/app.rs        → state management & navigation
tui/tree.rs       → rendering
actions.rs        → generates Warp launch configs, opens via warp://launch/
```

### Configuration

- Config file: `~/.config/ws/config.toml`
- Fields: `scan_dirs` (array of paths), `editor` (command to run)

### Worktree State Detection

- Dirty detection: `git status --porcelain`
- WIP commits: checks if HEAD commit message starts with "WIP: paused work"
- On launch with dirty worktree, prompts to create WIP commit
- On launch with WIP commit, auto-runs `git reset --soft HEAD~1`
