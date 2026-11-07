# AGENTS.md

Rust TUI (`ratatui` + `crossterm`) for managing OrbStack/Docker containers by shelling out to the `docker` CLI.

## Commands

Run the same checks as CI (`.github/workflows/ci.yml`) before pushing:

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run          # run the app
```

## Constraints

- MSRV 1.88, edition 2024 (let-chains are fine). CI runs 1.88 and stable on Linux and macOS.
- Talk to Docker only through `docker` CLI subprocesses in `docker/client.rs`. No Docker SDK.
- Ask before adding dependencies.
- Tests live inline in `#[cfg(test)]` modules next to the code. Keep logic testable without Docker.

## Modules

- `main.rs`: event loop, background threads, timers (`TICK_RATE`, `IDLE_THRESHOLD`, `DETAIL_DEBOUNCE`)
- `app.rs`: `App`, which holds all mutable state, plus the list model
- `handler.rs`: `handle_action()` applies an `Action` to `App` and spawns action threads
- `config.rs`: loads optional `$XDG_CONFIG_HOME/orb-tui/config.toml` (refresh interval, keymap). Missing file means defaults. Invalid file means defaults plus `app.config_error`
- `input/`: `Action` enum, default keymap and `map_key_to_action()`
- `docker/`: `client.rs` runs the CLI, `types.rs` has DTOs and domain structs
- `ui/`: pure rendering, never mutates state

## Data flow

```
KeyEvent -> map_key_to_action(key, mode, &keymap) -> Action -> handle_action() -> App
                                                                 -> thread -> channel -> App on next tick
```

## Background work

- Refresh thread (`spawn_refresh`): `docker ps` + `docker stats` every `config.refresh_interval` (default 5s). Skipped in search mode or while the user is typing.
- Detail worker (`spawn_detail_worker`): one long-lived thread. It handles only the latest `DetailRequest` (inspect, logs, group logs).
- Action threads: fire-and-forget start/stop/restart/remove. Results arrive on `action_tx` and trigger a refresh.
- `App.detail_generation` increments on every selection or tab change, and results from an older generation are dropped. Inspect results are cached in `App.info_cache`. Logs are always re-fetched.

## List model

`App.list_items` is a flat `Vec<ListItem>` of `GroupHeader` and `Container` entries, grouped by the `compose_project` label. Call `rebuild_list_items()` whenever containers or filters change. Containers without a project render without a header.

## Adding a keybinding or action

1. Add the variant to `Action` (`input/action.rs`).
2. Bind it in `default_bindings()`, and add it to `REMAPPABLE_ACTIONS` if users should be able to remap it (`input/keybindings.rs`).
3. Handle it in `handler.rs`.
4. Update the README keybinding table, and the config section if it's remappable.
