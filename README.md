# ws

Terminal UI for managing git worktrees and Claude Code sessions. Designed for developers juggling multiple branches and AI-assisted coding sessions.

## Requirements

- Rust toolchain
- [Warp terminal](https://www.warp.dev/) (for launch configurations)

## Installation

```bash
cargo install --path .
```

## Usage

```bash
ws --scan      # Update database by scanning repos and sessions
ws             # Launch interactive TUI
ws <filter>    # Launch TUI with initial filter text
```

### TUI Navigation

- **↑/↓** - Navigate tree (repos → branches → sessions)
- **←/→** - Select worktree for a branch
- **Space** - Toggle session selection
- **Enter** - Launch selected sessions in Warp

## Configuration

Config file: `~/.config/ws/config.toml`

```toml
scan_dirs = ["~/code", "~/projects"]
editor = "cursor"
```

## Architecture

See [CLAUDE.md](./CLAUDE.md) for detailed architecture documentation.

## License

MIT
