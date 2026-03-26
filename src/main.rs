mod app;
mod docker;
mod error;
mod handler;
mod input;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::{App, InputMode};
use input::map_key_to_action;

const TICK_RATE: Duration = Duration::from_millis(50);
const DETAIL_DEBOUNCE: Duration = Duration::from_millis(200);
const REFRESH_INTERVAL: Duration = Duration::from_secs(5);
const IDLE_THRESHOLD: Duration = Duration::from_secs(1);

fn main() -> anyhow::Result<()> {
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
    let mut last_refresh = Instant::now();
    let mut last_keypress = Instant::now();
    let mut needs_initial_load = true;
    let mut needs_initial_stats = true;

    loop {
        terminal.draw(|frame| {
            ui::render(frame, &mut app);
        })?;

        // Drain all pending key events before any blocking work
        if event::poll(TICK_RATE)? {
            loop {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        last_keypress = Instant::now();
                        let action = map_key_to_action(key, app.input_mode);
                        handler::handle_action(&mut app, action);
                    }
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }

        if app.should_quit {
            break;
        }

        // Load container list on first tick (after the empty frame is already painted)
        if needs_initial_load {
            handler::refresh_containers(&mut app);
            if let Some(i) = app
                .list_items
                .iter()
                .position(|item| matches!(item, app::ListItem::Container { .. }))
            {
                app.selected_index = i;
            }
            needs_initial_load = false;
            // detail_dirty is already true from App::new(), debounce handles the rest
            continue;
        }

        let idle_time = last_keypress.elapsed();

        // Load detail data only after navigation settles
        if app.needs_detail_load() && idle_time >= DETAIL_DEBOUNCE {
            handler::load_detail_now(&mut app);
        }

        // Load stats once after startup
        if needs_initial_stats && idle_time >= DETAIL_DEBOUNCE {
            if let Ok(stats) = docker::client::get_container_stats() {
                app.container_stats = stats;
            }
            needs_initial_stats = false;
        }

        // Skip all heavy work while user is active or in search mode
        if app.input_mode == InputMode::Search || idle_time < IDLE_THRESHOLD {
            continue;
        }

        // Periodic background refresh (only when user is idle)
        if last_refresh.elapsed() >= REFRESH_INTERVAL {
            last_refresh = Instant::now();
            let prev_selection = app.selected_container_id();
            handler::refresh_containers(&mut app);

            if let Some(prev_id) = prev_selection {
                if let Some(i) = app.list_items.iter().position(|item| {
                    matches!(item, app::ListItem::Container { index } if app.containers[*index].id == prev_id)
                }) {
                    app.selected_index = i;
                }
            }

            if let Ok(stats) = docker::client::get_container_stats() {
                app.container_stats = stats;
            }

            if app.detail_tab == app::DetailTab::Logs {
                if let Some(id) = app.selected_container_id() {
                    if let Ok(logs) = docker::client::get_container_logs(&id, 500) {
                        app.container_logs = Some(logs);
                    }
                }
            }

            app.message = None;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
