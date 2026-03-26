# orb-tui

A terminal UI for managing Docker containers.

> **Note:** This project is in early development (v0.1). It works, but expect rough edges.

<!-- TODO: Add a screenshot or GIF here -->

## Why

Managing containers from the command line means juggling `docker ps`, `docker logs`, `docker stats`, and `docker inspect` over and over. `orb-tui` puts all of that on one screen so you can see what's happening without switching between commands.

## Features

- Real-time container dashboard with auto-refresh
- Start, stop, restart, and remove containers
- Inspect, logs, and resource stats in a tabbed detail view
- Docker Compose project grouping with collapsible groups
- Group actions (start/stop/restart all containers in a project)
- Live search and filtering
- Vim-style navigation
- OrbStack integration (auto-detect, launch prompt, `.orb.local` domains)

## Requirements

- macOS (for now)
- `docker` CLI on your PATH
- A Docker runtime ([OrbStack](https://orbstack.dev/), Docker Desktop, colima, etc.)

## Installation

### From source

```sh
git clone https://github.com/villads-valur/orb-tui.git
cd orb-tui
cargo build --release
# Binary is at target/release/orb-tui
```

## Usage

```sh
orb-tui
```

That's it.

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `Down` | Move down |
| `k` / `Up` | Move up |
| `Enter` / `l` / `Right` | Expand/collapse group, or focus detail panel |
| `h` / `Left` | Back to container list |
| `Tab` | Next tab |
| `Shift+Tab` | Previous tab |
| `1` `2` `3` | Jump to Info / Logs / Stats tab |
| `g` | Scroll to top |
| `G` | Scroll to bottom |
| `Ctrl+u` | Scroll half page up |
| `Ctrl+d` | Scroll half page down |

### Container Actions

| Key | Action |
|-----|--------|
| `s` | Start container (or all in group) |
| `x` | Stop container (or all in group) |
| `r` | Restart container (or all in group) |
| `d` | Remove container (asks for confirmation) |
| `f` | Toggle log follow mode |
| `o` | Open OrbStack (when Docker is unavailable) |

### Search

| Key | Action |
|-----|--------|
| `/` | Start searching |
| `Enter` | Confirm filter |
| `Esc` | Cancel search |

### General

| Key | Action |
|-----|--------|
| `q` / `Ctrl+c` | Quit |
| `Esc` | Clear filter / unfocus detail panel |

## Known Limitations

- macOS only (the OrbStack launcher uses `open`, but core Docker features should work anywhere)
- No config file yet
- No custom keybinding remapping
- Early days, things will change

## Contributing

This project is very young and there's a lot of room for improvement. If something bugs you or you have an idea, open an issue. Pull requests are welcome too.

Some areas that could use help:

- Linux / Windows support
- Configuration file for custom keybindings and settings
- Better error handling and edge cases
- Tests

## Built With

- [Rust](https://www.rust-lang.org/)
- [ratatui](https://github.com/ratatui/ratatui) - TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal backend

## License

[MIT](LICENSE)
