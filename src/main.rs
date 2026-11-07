mod app;
mod config;
mod docker;
mod error;
mod handler;
mod input;
mod ui;

use std::collections::HashMap;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use app::{App, DetailTab, InputMode};
use docker::types::{Container, ContainerDetail, DockerStatsEntry};
use input::map_key_to_action;

const TICK_RATE: Duration = Duration::from_millis(50);
const DETAIL_DEBOUNCE: Duration = Duration::from_millis(200);
const IDLE_THRESHOLD: Duration = Duration::from_secs(1);

// --- Background refresh (container list + stats) ---

struct RefreshResult {
    containers: Option<Vec<Container>>,
    stats: Option<HashMap<String, DockerStatsEntry>>,
}

fn spawn_refresh(tx: &mpsc::Sender<RefreshResult>) {
    let tx = tx.clone();
    thread::spawn(move || {
        let containers = docker::client::list_containers().ok();
        let stats = docker::client::get_container_stats().ok();
        let _ = tx.send(RefreshResult { containers, stats });
    });
}

// --- Detail worker (inspect, logs) ---

enum DetailRequest {
    ContainerInspect {
        generation: u64,
        container_id: String,
    },
    ContainerLogs {
        generation: u64,
        container_id: String,
        tail: usize,
    },
    GroupLogs {
        generation: u64,
        containers: Vec<(String, String)>,
        tail: usize,
    },
}

enum DetailResult {
    Inspect {
        generation: u64,
        container_id: String,
        detail: Option<Box<ContainerDetail>>,
        error: Option<String>,
    },
    Logs {
        generation: u64,
        logs: Option<String>,
        error: Option<String>,
    },
}

fn spawn_detail_worker() -> (mpsc::Sender<DetailRequest>, mpsc::Receiver<DetailResult>) {
    let (req_tx, req_rx) = mpsc::channel::<DetailRequest>();
    let (res_tx, res_rx) = mpsc::channel::<DetailResult>();

    thread::spawn(move || {
        while let Ok(request) = req_rx.recv() {
            // Drain any newer requests, only process the latest
            let mut latest = request;
            while let Ok(newer) = req_rx.try_recv() {
                latest = newer;
            }

            match latest {
                DetailRequest::ContainerInspect {
                    generation,
                    container_id,
                } => {
                    let (detail, error) = match docker::client::inspect_container(&container_id) {
                        Ok(d) => (Some(Box::new(d)), None),
                        Err(e) => (None, Some(format!("{e}"))),
                    };
                    let _ = res_tx.send(DetailResult::Inspect {
                        generation,
                        container_id,
                        detail,
                        error,
                    });
                }
                DetailRequest::ContainerLogs {
                    generation,
                    container_id,
                    tail,
                } => {
                    let (logs, error) =
                        match docker::client::get_container_logs(&container_id, tail) {
                            Ok(l) => (Some(l), None),
                            Err(e) => (None, Some(format!("{e}"))),
                        };
                    let _ = res_tx.send(DetailResult::Logs {
                        generation,
                        logs,
                        error,
                    });
                }
                DetailRequest::GroupLogs {
                    generation,
                    containers,
                    tail,
                } => {
                    let (logs, error) = match docker::client::get_group_logs(&containers, tail) {
                        Ok(l) => (Some(l), None),
                        Err(e) => (None, Some(format!("{e}"))),
                    };
                    let _ = res_tx.send(DetailResult::Logs {
                        generation,
                        logs,
                        error,
                    });
                }
            }
        }
    });

    (req_tx, res_rx)
}

/// Build a detail request for the current app state. Returns None if nothing needs loading.
fn build_detail_request(app: &App) -> Option<DetailRequest> {
    let generation = app.detail_generation;

    if let Some(project) = app.selected_group_project() {
        if app.detail_tab == DetailTab::Logs {
            let containers: Vec<(String, String)> = app
                .group_containers(project)
                .iter()
                .filter(|c| c.state.is_running())
                .map(|c| (c.id.clone(), c.display_name().to_string()))
                .collect();
            return Some(DetailRequest::GroupLogs {
                generation,
                containers,
                tail: 50,
            });
        }
        return None;
    }

    let id = app.selected_container_id()?;

    match app.detail_tab {
        DetailTab::Info | DetailTab::Stats => {
            if app.info_cache.contains_key(&id) {
                return None;
            }
            Some(DetailRequest::ContainerInspect {
                generation,
                container_id: id,
            })
        }
        DetailTab::Logs => Some(DetailRequest::ContainerLogs {
            generation,
            container_id: id,
            tail: 500,
        }),
    }
}

fn main() -> anyhow::Result<()> {
    let (config, config_error) = config::load();

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    app.config_error = config_error.clone();
    let mut last_refresh = Instant::now();
    let mut last_keypress = Instant::now();
    let mut needs_initial_load = true;

    // Background refresh thread (container list + stats)
    let (refresh_tx, refresh_rx) = mpsc::channel::<RefreshResult>();
    let mut refresh_in_flight = false;

    // Persistent detail worker thread
    let (detail_tx, detail_rx) = spawn_detail_worker();
    let mut detail_request_sent_for_gen: Option<u64> = None;

    // Action results channel (start/stop/restart/remove)
    let (action_tx, action_rx) = mpsc::channel::<handler::ActionResult>();

    loop {
        terminal.draw(|frame| {
            ui::render(frame, &mut app);
        })?;

        // Drain all pending key events
        if event::poll(TICK_RATE)? {
            loop {
                if let Event::Key(key) = event::read()?
                    && key.kind == KeyEventKind::Press
                {
                    last_keypress = Instant::now();
                    app.config_error = None;
                    let action = map_key_to_action(key, app.input_mode, &config.keymap);
                    handler::handle_action(&mut app, action, &action_tx);
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }

        if app.should_quit {
            break;
        }

        // Initial load (blocking, once)
        if needs_initial_load {
            app.docker_available = docker::client::is_docker_available();
            if app.docker_available {
                handler::refresh_containers(&mut app);
                if let Some(i) = app
                    .list_items
                    .iter()
                    .position(|item| matches!(item, app::ListItem::Container { .. }))
                {
                    app.selected_index = i;
                }
                spawn_refresh(&refresh_tx);
                refresh_in_flight = true;
                last_refresh = Instant::now();
            }
            needs_initial_load = false;
            continue;
        }

        let idle_time = last_keypress.elapsed();

        // Send detail request to worker when debounce settles
        if app.needs_detail_load() && idle_time >= DETAIL_DEBOUNCE {
            let already_sent = detail_request_sent_for_gen == Some(app.detail_generation);
            if !already_sent {
                if let Some(request) = build_detail_request(&app) {
                    detail_request_sent_for_gen = Some(app.detail_generation);
                    let _ = detail_tx.send(request);
                } else {
                    // No fetch needed (e.g., group Info/Stats), just mark as loaded
                    app.detail_loaded_for = app.selected_key();
                    app.detail_dirty = false;
                }
            }
        }

        // Receive detail results (non-blocking)
        while let Ok(result) = detail_rx.try_recv() {
            match result {
                DetailResult::Inspect {
                    generation,
                    container_id,
                    detail,
                    error,
                } => {
                    if generation != app.detail_generation {
                        continue;
                    }
                    if let Some(detail) = detail {
                        app.info_cache.insert(container_id.clone(), *detail);
                    }
                    if let Some(e) = error {
                        app.set_message(format!("Error: {e}"));
                    }
                    app.detail_loaded_for = Some(container_id);
                    app.detail_dirty = false;
                }
                DetailResult::Logs {
                    generation,
                    logs,
                    error,
                } => {
                    if generation != app.detail_generation {
                        continue;
                    }
                    if let Some(logs) = logs {
                        app.container_logs = Some(logs);
                    }
                    if let Some(e) = error {
                        app.set_message(format!("Error: {e}"));
                    }
                    app.detail_loaded_for = app.selected_key();
                    app.detail_dirty = false;
                }
            }
        }

        // Receive action results (non-blocking)
        while let Ok(result) = action_rx.try_recv() {
            app.busy_count = app.busy_count.saturating_sub(1);
            app.set_message(result.message);
            // Trigger a refresh to pick up new container states
            spawn_refresh(&refresh_tx);
            refresh_in_flight = true;
        }

        // Tick spinner when busy
        if app.busy_count > 0 {
            app.spinner_tick = app.spinner_tick.wrapping_add(1);
        }

        // Receive background refresh results (non-blocking)
        if let Ok(result) = refresh_rx.try_recv() {
            refresh_in_flight = false;

            if let Some(containers) = result.containers {
                app.docker_available = true;
                app.orbstack_starting = false;
                app.message = None;
                let prev_selection = app.selected_container_id();
                app.containers = containers;
                app.update_auto_collapse();
                app.rebuild_list_items();

                if let Some(prev_id) = prev_selection
                    && let Some(i) = app.list_items.iter().position(|item| {
                        matches!(item, app::ListItem::Container { index } if app.containers[*index].id == prev_id)
                    }) {
                        app.selected_index = i;
                    }
            } else {
                app.docker_available = false;
            }

            if let Some(stats) = result.stats {
                app.container_stats = stats;
            }

            // Refresh logs for current container via the worker (non-blocking)
            if app.detail_tab == DetailTab::Logs
                && let Some(id) = app.selected_container_id()
            {
                let _ = detail_tx.send(DetailRequest::ContainerLogs {
                    generation: app.detail_generation,
                    container_id: id,
                    tail: 500,
                });
            }
        }

        // Skip spawning new refreshes while user is active or in search mode
        if app.input_mode == InputMode::Search || idle_time < IDLE_THRESHOLD {
            continue;
        }

        // Retry faster when Docker isn't available (checking for OrbStack startup)
        let refresh_interval = if app.docker_available {
            config.refresh_interval
        } else {
            Duration::from_secs(2)
        };

        if !refresh_in_flight && last_refresh.elapsed() >= refresh_interval {
            last_refresh = Instant::now();
            spawn_refresh(&refresh_tx);
            refresh_in_flight = true;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    if let Some(error) = config_error {
        eprintln!("{error}");
    }

    Ok(())
}
