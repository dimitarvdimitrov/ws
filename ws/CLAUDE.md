# ws - Git Worktree & Claude Session Manager

A Rust CLI/TUI tool for managing git worktrees and resuming Claude sessions.

## Goal

Quickly switch between git worktrees while preserving and resuming associated Claude sessions. The tool:
1. Scans git repos + worktrees and Claude session history
2. Presents a filterable tree view: Branch → Worktrees → Sessions
3. Opens Warp terminal windows with the editor and selected Claude sessions

## Usage

```bash
# Scan repos and sessions (run via cron every minute)
ws --scan

# Launch TUI
ws

# Launch TUI with initial filter
ws mitko ingester
```

## TUI Controls

| Key | Action |
|-----|--------|
| ↑/↓ | Navigate branches/sessions |
| ←/→ | Cycle worktree selection for current branch |
| Space | Toggle session checkbox (for resuming) |
| Enter | Confirm - opens Warp windows |
| / | Focus filter input |
| Esc | Clear filter / exit |
| q | Quit |

## WIP Commit Handling

- **Dirty worktree selected**: Prompts to create "WIP: paused work" commit
- **WIP commit detected**: Automatically runs `git reset --soft HEAD~1` to restore staged changes

## Architecture

```
src/
├── main.rs              # CLI entry: ws --scan | ws [filter...]
├── scanner/
│   ├── git.rs           # Scan repos, worktrees, dirty state
│   └── claude.rs        # Parse ~/.claude/projects/*/sessions-index.json
├── db.rs                # SQLite schema + queries
├── tui/
│   ├── app.rs           # State machine, event handling
│   ├── tree.rs          # Tree rendering
│   └── confirmation.rs  # Dirty worktree dialog
├── actions.rs           # Warp launch config generation
└── config.rs            # Config loading
```

## Configuration

Location: `~/Library/Application Support/ws/config.toml` (macOS) or `~/.config/ws/config.toml` (Linux)

```toml
scan_dirs = ["~/Documents"]
editor = "code"  # defaults to $EDITOR
```

## Database

SQLite at `~/Library/Application Support/ws/ws.db` with tables:
- `repos` - git repositories
- `worktrees` - worktrees per repo with branch info
- `sessions` - Claude sessions with git branch association
