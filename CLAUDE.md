# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
rtk cargo build

# Run
rtk cargo run

# Build release binary (named `orb-tui`)
rtk cargo build --release

# Check (fast, no codegen)
rtk cargo check

# Lint
rtk cargo clippy

# Format
rtk cargo fmt
```

## Architecture

This is a Rust terminal UI app built with `ratatui` + `crossterm`. It wraps the `docker` CLI to manage OrbStack containers.

### Module structure

- **`main.rs`** — event loop, threading orchestration, timer logic
- **`app.rs`** — `App` state struct; all mutable state lives here
- **`handler.rs`** — maps `Action` values to state mutations and spawns background threads
- **`input/`** — `Action` enum (`action.rs`) and key-to-action mapping (`keybindings.rs`)
- **`docker/`** — `client.rs` calls the `docker` CLI via `std::process::Command`; `types.rs` holds DTOs and domain structs
- **`ui/`** — pure rendering functions using ratatui widgets; no state mutation

### Threading model

Three background channels run concurrently:

1. **Refresh thread** (`spawn_refresh`) — fetches container list + stats every 5 seconds via `docker ps` and `docker stats`. Skipped while user is actively typing or in search mode.
2. **Detail worker** (`spawn_detail_worker`) — persistent thread receiving `DetailRequest` over a channel. Drains stale requests and processes only the latest. Handles `ContainerInspect`, `ContainerLogs`, and `GroupLogs`.
3. **Action threads** — fire-and-forget threads for start/stop/restart/remove. Results arrive on `action_tx` and trigger a refresh.

### Detail loading / caching

- `App.detail_generation` (u64) increments on every selection or tab change. Workers tag results with the generation they were spawned for; stale results are discarded.
- `App.info_cache` (HashMap keyed by container ID) caches inspect results so switching back to the Info tab doesn't re-fetch.
- Logs are never cached — always re-fetched.
- `App.detail_dirty` + `DETAIL_DEBOUNCE` (200ms after last keypress) gate when a detail request is sent.

### List model

Containers are grouped by `compose_project` label. `App.list_items` is a flat `Vec<ListItem>` that interleaves `GroupHeader` and `Container` entries. `rebuild_list_items()` regenerates this vec from `App.containers` whenever containers or filters change. Groups without a compose project render containers directly (no header).

### Key data flow

```
KeyEvent -> map_key_to_action() -> Action -> handler::handle_action() -> App mutation
                                                                       -> spawn thread -> channel -> App update on next tick
```

### Docker interaction

All Docker calls in `docker/client.rs` run `docker` CLI subprocesses. There is no Docker API/SDK dependency. Group actions (`run_action_parallel`, `get_group_logs`) parallelize across containers using `std::thread`.
