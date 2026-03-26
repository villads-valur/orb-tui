use crate::app::{App, ConfirmAction, DetailTab, FocusedPanel, InputMode, ListItem};
use crate::docker::client;
use crate::input::Action;

pub fn handle_action(app: &mut App, action: Action) {
    match app.input_mode {
        InputMode::Confirm => handle_confirm(app, action),
        InputMode::Normal => handle_normal(app, action),
        InputMode::Search => handle_search(app, action),
    }
}

fn handle_normal(app: &mut App, action: Action) {
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
                load_detail_now(app);
            }
        }

        Action::Back => {
            if app.focused_panel == FocusedPanel::Detail {
                app.focused_panel = FocusedPanel::ContainerList;
            }
        }

        Action::NextTab => switch_tab(app, app.detail_tab.next()),
        Action::PrevTab => switch_tab(app, app.detail_tab.prev()),
        Action::TabInfo => switch_tab(app, DetailTab::Info),
        Action::TabLogs => switch_tab(app, DetailTab::Logs),
        Action::TabStats => switch_tab(app, DetailTab::Stats),

        Action::StartContainer => run_container_action(app, client::start_container, "started"),
        Action::StopContainer => run_container_action(app, client::stop_container, "stopped"),
        Action::RestartContainer => {
            run_container_action(app, client::restart_container, "restarted")
        }
        Action::RemoveContainer => {
            if let Some(container) = app.selected_container() {
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
            if !app.search_filter.is_empty() {
                app.search_filter.clear();
                app.rebuild_list_items();
                app.selected_index = 0;
                app.on_selection_changed();
            } else if app.focused_panel == FocusedPanel::Detail {
                app.focused_panel = FocusedPanel::ContainerList;
            }
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

    // Groups: Info and Stats are always free, only Logs needs fetching
    if app.selected_group_project().is_some() {
        if tab == DetailTab::Logs {
            app.container_logs = None;
            app.detail_dirty = true;
        } else {
            let key = app.selected_key();
            app.detail_loaded_for = key;
            app.detail_dirty = false;
        }
        return;
    }

    // Containers: use cache when available
    if let Some(id) = app.selected_container_id() {
        if tab != DetailTab::Logs && app.info_cache.contains_key(&id) {
            app.detail_loaded_for = Some(id);
            app.detail_dirty = false;
            return;
        }
    }

    app.detail_dirty = true;
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

fn run_container_action(
    app: &mut App,
    action_fn: fn(&str) -> crate::error::Result<()>,
    success_msg: &str,
) {
    if let Some(id) = app.selected_container_id() {
        if let Err(e) = action_fn(&id) {
            app.set_message(format!("Error: {e}"));
        } else {
            app.set_message(format!("Container {success_msg}"));
            refresh_containers(app);
        }
    }
}

fn handle_confirm(app: &mut App, action: Action) {
    match action {
        Action::ConfirmYes => {
            if let Some(confirm_action) = app.confirm_action.take() {
                match confirm_action {
                    ConfirmAction::RemoveContainer(id) => {
                        if let Err(e) = client::remove_container(&id) {
                            app.set_message(format!("Error: {e}"));
                        } else {
                            app.set_message("Container removed");
                            app.info_cache.remove(&id);
                            refresh_containers(app);
                        }
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
            app.rebuild_list_items();
        }
        Err(e) => {
            app.set_message(format!("Error loading containers: {e}"));
        }
    }
}

pub fn load_detail_now(app: &mut App) {
    if let Some(project) = app.selected_group_project().map(String::from) {
        if app.detail_tab == DetailTab::Logs {
            let containers: Vec<(String, String)> = app
                .group_containers(&project)
                .iter()
                .filter(|c| c.state.is_running())
                .map(|c| (c.id.clone(), c.display_name().to_string()))
                .collect();

            match client::get_group_logs(&containers, 50) {
                Ok(logs) => app.container_logs = Some(logs),
                Err(e) => app.set_message(format!("Error: {e}")),
            }
        }
        app.detail_loaded_for = Some(format!("group:{project}"));
        app.detail_dirty = false;
        return;
    }

    // Handle individual container selection
    let Some(id) = app.selected_container_id() else {
        return;
    };

    match app.detail_tab {
        DetailTab::Info => {
            if !app.info_cache.contains_key(&id) {
                match client::inspect_container(&id) {
                    Ok(detail) => {
                        app.info_cache.insert(id.clone(), detail);
                    }
                    Err(e) => app.set_message(format!("Error: {e}")),
                }
            }
        }
        DetailTab::Logs => match client::get_container_logs(&id, 500) {
            Ok(logs) => app.container_logs = Some(logs),
            Err(e) => app.set_message(format!("Error: {e}")),
        },
        DetailTab::Stats => {
            if !app.info_cache.contains_key(&id) {
                match client::inspect_container(&id) {
                    Ok(detail) => {
                        app.info_cache.insert(id.clone(), detail);
                    }
                    Err(e) => app.set_message(format!("Error: {e}")),
                }
            }
        }
    }

    app.detail_loaded_for = Some(id);
    app.detail_dirty = false;
}
