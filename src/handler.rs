use std::sync::mpsc;
use std::thread;

use crate::app::{App, ConfirmAction, DetailTab, FocusedPanel, InputMode, ListItem};
use crate::docker::client;
use crate::input::Action;

pub struct ActionResult {
    pub message: String,
}

pub fn handle_action(app: &mut App, action: Action, action_tx: &mpsc::Sender<ActionResult>) {
    match app.input_mode {
        InputMode::Confirm => handle_confirm(app, action, action_tx),
        InputMode::Normal => handle_normal(app, action, action_tx),
        InputMode::Search => handle_search(app, action),
    }
}

fn handle_normal(app: &mut App, action: Action, action_tx: &mpsc::Sender<ActionResult>) {
    match action {
        Action::Quit => app.should_quit = true,

        Action::MoveUp => {
            if app.focused_panel == FocusedPanel::Detail {
                scroll_detail(app, -1);
            } else {
                app.move_selection_up();
            }
        }
        Action::MoveDown => {
            if app.focused_panel == FocusedPanel::Detail {
                scroll_detail(app, 1);
            } else {
                app.move_selection_down();
            }
        }

        Action::Select => {
            if let Some(ListItem::GroupHeader { .. }) = app.list_items.get(app.selected_index) {
                app.toggle_group();
            } else if app.focused_panel == FocusedPanel::ContainerList {
                app.focused_panel = FocusedPanel::Detail;
            }
        }

        Action::Back => {
            if app.fullscreen_detail {
                app.fullscreen_detail = false;
            } else if app.focused_panel == FocusedPanel::Detail {
                app.focused_panel = FocusedPanel::ContainerList;
            }
        }

        Action::NextTab => switch_tab(app, app.detail_tab.next()),
        Action::PrevTab => switch_tab(app, app.detail_tab.prev()),
        Action::TabInfo => switch_tab(app, DetailTab::Info),
        Action::TabLogs => switch_tab(app, DetailTab::Logs),
        Action::TabStats => switch_tab(app, DetailTab::Stats),

        Action::StartContainer => spawn_action(app, action_tx, "start", "started"),
        Action::StopContainer => spawn_action(app, action_tx, "stop", "stopped"),
        Action::RestartContainer => spawn_action(app, action_tx, "restart", "restarted"),
        Action::RemoveContainer => {
            if let Some(project) = app.selected_group_project().map(String::from) {
                let ids: Vec<String> = app
                    .group_containers(&project)
                    .iter()
                    .map(|c| c.id.clone())
                    .collect();
                if !ids.is_empty() {
                    let count = ids.len();
                    app.confirm_message =
                        Some(format!("Remove {count} containers in '{project}'? (y/n)"));
                    app.confirm_action = Some(ConfirmAction::RemoveGroup(ids));
                    app.input_mode = InputMode::Confirm;
                }
            } else if let Some(container) = app.selected_container() {
                let name = container.display_name().to_string();
                let id = container.id.clone();
                app.confirm_message = Some(format!("Remove container '{name}'? (y/n)"));
                app.confirm_action = Some(ConfirmAction::RemoveContainer(id));
                app.input_mode = InputMode::Confirm;
            }
        }

        Action::ToggleLogFollow => {
            app.log_follow = !app.log_follow;
            let status = if app.log_follow { "on" } else { "off" };
            app.set_message(format!("Log follow: {status}"));
        }

        Action::ToggleFullScreen => {
            app.fullscreen_detail = !app.fullscreen_detail;
            if app.fullscreen_detail {
                app.focused_panel = FocusedPanel::Detail;
            }
        }

        Action::ScrollHalfPageUp => scroll_detail(app, -15),
        Action::ScrollHalfPageDown => scroll_detail(app, 15),
        Action::ScrollToTop => {
            if app.detail_tab == DetailTab::Logs {
                app.log_follow = false;
                app.log_scroll_offset = 0;
            }
            app.detail_scroll_offset = 0;
        }
        Action::ScrollToBottom => {
            if app.detail_tab == DetailTab::Logs {
                app.log_follow = true;
            }
            app.detail_scroll_offset = usize::MAX / 2;
        }

        Action::EnterSearch => {
            app.search_query.clear();
            app.input_mode = InputMode::Search;
            app.focused_panel = FocusedPanel::ContainerList;
        }

        Action::ClearFilter => {
            if app.fullscreen_detail {
                app.fullscreen_detail = false;
            } else if !app.search_filter.is_empty() {
                app.search_filter.clear();
                app.rebuild_list_items();
                app.selected_index = 0;
                app.on_selection_changed();
            } else if app.focused_panel == FocusedPanel::Detail {
                app.focused_panel = FocusedPanel::ContainerList;
            }
        }

        Action::OpenOrbStack if !app.docker_available && !app.orbstack_starting => {
            let _ = client::start_orbstack();
            app.orbstack_starting = true;
        }

        _ => {}
    }
}

fn handle_search(app: &mut App, action: Action) {
    match action {
        Action::SearchType(c) => {
            app.search_query.push(c);
            app.rebuild_list_items();
            app.selected_index = 0;
        }
        Action::SearchBackspace => {
            app.search_query.pop();
            app.rebuild_list_items();
            app.selected_index = 0;
        }
        Action::SearchConfirm => {
            app.search_filter = app.search_query.clone();
            app.search_query.clear();
            app.input_mode = InputMode::Normal;
            app.rebuild_list_items();
            if app.selected_index >= app.list_items.len() {
                app.selected_index = 0;
            }
            app.on_selection_changed();
        }
        Action::SearchCancel => {
            app.search_query.clear();
            app.input_mode = InputMode::Normal;
            app.rebuild_list_items();
            if app.selected_index >= app.list_items.len() {
                app.selected_index = 0;
            }
        }
        _ => {}
    }
}

fn switch_tab(app: &mut App, tab: DetailTab) {
    app.detail_tab = tab;
    app.detail_scroll_offset = 0;
    app.log_scroll_offset = 0;

    if app.selected_group_project().is_some() {
        if tab == DetailTab::Logs {
            app.container_logs = None;
            app.mark_dirty();
        } else {
            let key = app.selected_key();
            app.detail_loaded_for = key;
            app.detail_dirty = false;
        }
        return;
    }

    if let Some(id) = app.selected_container_id()
        && tab != DetailTab::Logs
        && app.info_cache.contains_key(&id)
    {
        app.detail_loaded_for = Some(id);
        app.detail_dirty = false;
        return;
    }

    app.mark_dirty();
}

fn scroll_detail(app: &mut App, amount: isize) {
    let (offset, follow) = if app.detail_tab == DetailTab::Logs {
        (&mut app.log_scroll_offset, Some(&mut app.log_follow))
    } else {
        (&mut app.detail_scroll_offset, None)
    };

    if amount < 0 {
        if let Some(follow) = follow {
            *follow = false;
        }
        *offset = offset.saturating_sub(amount.unsigned_abs());
    } else {
        *offset = offset.saturating_add(amount as usize);
    }
}

/// Spawn a container action (start/stop/restart) on a background thread.
fn spawn_action(
    app: &mut App,
    action_tx: &mpsc::Sender<ActionResult>,
    action: &str,
    past_tense: &str,
) {
    let ids: Vec<String> = if let Some(project) = app.selected_group_project().map(String::from) {
        let ids: Vec<String> = app
            .group_containers(&project)
            .iter()
            .map(|c| c.id.clone())
            .collect();
        if ids.is_empty() {
            return;
        }
        let count = ids.len();
        app.set_message(format!("{} {count} containers...", capitalize(action)));
        ids
    } else if let Some(id) = app.selected_container_id() {
        app.set_message(format!("{}...", capitalize(action)));
        vec![id]
    } else {
        return;
    };

    let action = action.to_string();
    let past_tense = past_tense.to_string();
    let tx = action_tx.clone();
    app.busy_count += 1;

    thread::spawn(move || {
        let (successes, errors) = client::run_action_parallel(&action, &ids);
        let message = if ids.len() == 1 {
            if errors.is_empty() {
                format!("Container {past_tense}")
            } else {
                format!("Error: {}", errors.join(", "))
            }
        } else if errors.is_empty() {
            format!("{successes} containers {past_tense}")
        } else {
            format!("{successes} {past_tense}, {} failed", errors.len())
        };
        let _ = tx.send(ActionResult { message });
    });
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

fn handle_confirm(app: &mut App, action: Action, action_tx: &mpsc::Sender<ActionResult>) {
    match action {
        Action::ConfirmYes => {
            if let Some(confirm_action) = app.confirm_action.take() {
                match confirm_action {
                    ConfirmAction::RemoveContainer(id) => {
                        app.info_cache.remove(&id);
                        app.set_message("Removing...");
                        app.busy_count += 1;
                        let tx = action_tx.clone();
                        thread::spawn(move || {
                            let message = match client::remove_container(&id) {
                                Ok(()) => "Container removed".to_string(),
                                Err(e) => format!("Error: {e}"),
                            };
                            let _ = tx.send(ActionResult { message });
                        });
                    }
                    ConfirmAction::RemoveGroup(ids) => {
                        for id in &ids {
                            app.info_cache.remove(id);
                        }
                        let count = ids.len();
                        app.set_message(format!("Removing {count} containers..."));
                        app.busy_count += 1;
                        let tx = action_tx.clone();
                        thread::spawn(move || {
                            let (successes, errors) = client::run_action_parallel("rm", &ids);
                            let message = if errors.is_empty() {
                                format!("{successes} containers removed")
                            } else {
                                format!("{successes} removed, {} failed", errors.len())
                            };
                            let _ = tx.send(ActionResult { message });
                        });
                    }
                }
            }
            app.confirm_message = None;
            app.input_mode = InputMode::Normal;
        }
        Action::ConfirmNo | Action::Quit => {
            app.confirm_message = None;
            app.confirm_action = None;
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
}

pub fn refresh_containers(app: &mut App) {
    match client::list_containers() {
        Ok(containers) => {
            app.containers = containers;
            app.update_auto_collapse();
            app.rebuild_list_items();
        }
        Err(e) => {
            app.set_message(format!("Error loading containers: {e}"));
        }
    }
}
