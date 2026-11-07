# orb-tui

A terminal UI for managing your OrbStack containers.

It's built around OrbStack, but it talks to the plain `docker` CLI, so the core features work with any Docker runtime (Docker Desktop, colima, etc.). The OrbStack extras (launch prompt, `.orb.local` domains) only kick in when OrbStack is there.

> **Note:** This project is in early development (v0.1). It works, but expect rough edges.

![orb-tui in a light and a dark terminal, showing Compose projects with the Info tab open](docs/images/hero.png)

## Why

Managing containers from the command line means juggling `docker ps`, `docker logs`, `docker stats`, and `docker inspect` over and over. `orb-tui` puts all of that on one screen so you can see what's happening without switching between commands.

## Features

- Container dashboard that refreshes every 5 seconds (configurable)
- Start, stop, restart, and remove containers
- Inspect, logs, and resource stats in a tabbed detail view
- Docker Compose project grouping with collapsible groups
- Group actions (start/stop/restart/remove all containers in a project)
- Inactive groups collapse automatically
- Fullscreen detail view
- Live search by container, image, service, or project name
- Vim-style navigation
- Optional TOML config for the refresh interval and custom keybindings
- OrbStack integration (auto-detect, launch prompt, `.orb.local` domains)

## Screenshots

| Dark | Light |
|------|-------|
| ![orb-tui in a dark terminal](docs/images/dark.png) | ![orb-tui in a light terminal](docs/images/light.png) |

## Requirements

- macOS (tested). Linux should work since everything goes through the `docker` CLI, but it hasn't been tested yet
- `docker` CLI on your PATH
- Rust 1.88 or newer to build it
- A Docker runtime ([OrbStack](https://orbstack.dev/), Docker Desktop, colima, etc.)

## Installation

```sh
cargo install --git https://github.com/villads-valur/orb-tui
```

This puts `orb-tui` in `~/.cargo/bin`, which should already be on your PATH if you installed Rust with rustup.

### From a local clone

```sh
git clone https://github.com/villads-valur/orb-tui.git
cd orb-tui
cargo install --path .
```

## Usage

```sh
orb-tui
```

That's it.

## Keybindings

These are the defaults. You can remap the normal-mode keys in the [config file](#configuration).

### Navigation

| Key | Action |
|-----|--------|
| `j` / `Down` | Move down (scrolls when the detail panel is focused) |
| `k` / `Up` | Move up (scrolls when the detail panel is focused) |
| `Enter` / `l` / `Right` | Expand/collapse group, or focus detail panel |
| `h` / `Left` | Back to container list, or leave fullscreen |
| `Tab` | Next tab |
| `Shift+Tab` | Previous tab |
| `1` `2` `3` | Jump to Info / Logs / Stats tab |
| `g` | Scroll detail panel to top |
| `G` | Scroll detail panel to bottom (turns log follow back on) |
| `Ctrl+u` | Scroll detail panel half a page up |
| `Ctrl+d` | Scroll detail panel half a page down |

### Container Actions

| Key | Action |
|-----|--------|
| `s` | Start container (or all in group) |
| `x` | Stop container (or all in group) |
| `r` | Restart container (or all in group) |
| `d` | Remove container (or all in group, asks for confirmation) |
| `f` | Toggle log follow mode |
| `z` | Toggle fullscreen detail view |
| `o` | Open OrbStack (when Docker is unavailable) |

### Search

| Key | Action |
|-----|--------|
| `/` | Start searching |
| `Enter` | Confirm filter |
| `Esc` | Cancel search |

The list filters as you type.

### Confirm Prompt

| Key | Action |
|-----|--------|
| `y` / `Enter` | Confirm |
| `n` / `Esc` | Cancel |

### General

| Key | Action |
|-----|--------|
| `q` / `Ctrl+c` | Quit |
| `Esc` | Leave fullscreen, clear filter, or unfocus detail panel |

## Configuration

orb-tui reads an optional config file from `$XDG_CONFIG_HOME/orb-tui/config.toml`. If `XDG_CONFIG_HOME` isn't set it uses `~/.config/orb-tui/config.toml` (on macOS too). Every setting is optional.

```toml
# How often the container list and stats refresh, in seconds (default 5, minimum 1)
refresh_interval_secs = 5

[keybindings]
# A single key or a list of keys per action
quit = ["q", "Q"]
move_down = ["j", "down"]
move_up = ["k", "up"]
remove_container = "ctrl+x"
```

### Keybindings

- Listing an action **replaces** its default keys. Actions you don't list keep their defaults
- If you bind a key that another action uses by default, your binding wins and the other action loses that key
- Binding the same key to two actions is an error
- `Ctrl+c` always quits, whatever is in the config
- Only normal mode can be remapped. Search and the confirm prompt keep their fixed keys

**Key syntax:** a single character (`j`, `G`, `/`), or a named key: `enter`, `esc`, `tab`, `shift+tab`, `up`, `down`, `left`, `right`, `home`, `end`, `pageup`, `pagedown`, `backspace`, `delete`, `space`. Add `ctrl+` and/or `alt+` in front for modifiers (`ctrl+u`). For uppercase letters, use the letter itself (`G`), not `shift+g`.

**Actions:** `quit`, `move_up`, `move_down`, `select`, `back`, `next_tab`, `prev_tab`, `tab_info`, `tab_logs`, `tab_stats`, `start_container`, `stop_container`, `restart_container`, `remove_container`, `toggle_log_follow`, `toggle_full_screen`, `scroll_half_page_up`, `scroll_half_page_down`, `scroll_to_top`, `scroll_to_bottom`, `enter_search`, `clear_filter`, `open_orbstack`.

### Errors

If there's no config file, orb-tui uses the defaults. If the file can't be read or has a problem (invalid TOML, an unknown setting, action or key), orb-tui ignores the whole file and starts with the defaults. The error shows in the footer until you press a key, and is printed again when you quit.

## Known Limitations

- Only tested on macOS. The OrbStack launcher (`o`) uses the macOS `open` command
- Footer hints always show the default keys, even after you remap them
- Early days, things will change

## Contributing

This project is very young and there's a lot of room for improvement. If something bugs you or you have an idea, open an issue. Pull requests are welcome too.

Some areas that could use help:

- Testing on Linux and Windows
- Better error handling and edge cases
- Tests

## Built With

- [Rust](https://www.rust-lang.org/)
- [ratatui](https://github.com/ratatui/ratatui) - TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal backend

## License

[MIT](LICENSE)
