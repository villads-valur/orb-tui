use std::collections::{HashMap, HashSet};

use crate::docker::types::{Container, ContainerDetail, ContainerState, DockerStatsEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Confirm,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    ContainerList,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Info,
    Logs,
    Stats,
}

impl DetailTab {
    pub fn next(self) -> Self {
        match self {
            Self::Info => Self::Logs,
            Self::Logs => Self::Stats,
            Self::Stats => Self::Info,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Info => Self::Stats,
            Self::Logs => Self::Info,
            Self::Stats => Self::Logs,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ListItem {
    GroupHeader {
        project: String,
        expanded: bool,
        running_count: usize,
        total_count: usize,
    },
    Container {
        index: usize,
    },
}

pub struct App {
    pub should_quit: bool,
    pub input_mode: InputMode,
    pub focused_panel: FocusedPanel,
    pub detail_tab: DetailTab,

    pub containers: Vec<Container>,
    pub list_items: Vec<ListItem>,
    pub selected_index: usize,
    pub container_stats: HashMap<String, DockerStatsEntry>,

    /// Per-container cache for Info tab (keyed by container ID).
    pub info_cache: HashMap<String, ContainerDetail>,
    /// Logs are not cached (always dynamic).
    pub container_logs: Option<String>,

    /// Which container the detail panel currently has data for.
    pub detail_loaded_for: Option<String>,
    pub detail_dirty: bool,
    /// Incremented on every selection/tab change. Used to discard stale worker results.
    pub detail_generation: u64,

    pub log_scroll_offset: usize,
    pub log_follow: bool,
    pub detail_scroll_offset: usize,

    pub collapsed_groups: HashSet<String>,

    pub confirm_message: Option<String>,
    pub confirm_action: Option<ConfirmAction>,
    pub message: Option<String>,

    pub search_query: String,
    pub search_filter: String,

    pub docker_available: bool,
    pub orbstack_starting: bool,

    pub busy_count: usize,
    pub spinner_tick: usize,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    RemoveContainer(String),
}

impl App {
    pub fn new() -> Self {
        App {
            should_quit: false,
            input_mode: InputMode::Normal,
            focused_panel: FocusedPanel::ContainerList,
            detail_tab: DetailTab::Info,

            containers: Vec::new(),
            list_items: Vec::new(),
            selected_index: 0,
            container_stats: HashMap::new(),

            info_cache: HashMap::new(),
            container_logs: None,

            detail_loaded_for: None,
            detail_dirty: true,
            detail_generation: 0,

            log_scroll_offset: 0,
            log_follow: true,
            detail_scroll_offset: 0,

            collapsed_groups: HashSet::new(),

            confirm_message: None,
            confirm_action: None,
            message: None,

            search_query: String::new(),
            search_filter: String::new(),

            docker_available: true,
            orbstack_starting: false,
            busy_count: 0,
            spinner_tick: 0,
        }
    }

    pub fn spinner_frame(&self) -> &'static str {
        const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        FRAMES[self.spinner_tick % FRAMES.len()]
    }

    pub fn set_message(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
    }

    /// Get the cached detail for the currently selected container, if available.
    pub fn selected_detail(&self) -> Option<&ContainerDetail> {
        let id = self.selected_container_id()?;
        self.info_cache.get(&id)
    }

    pub fn active_filter(&self) -> &str {
        if self.input_mode == InputMode::Search {
            &self.search_query
        } else {
            &self.search_filter
        }
    }

    fn matches_filter(&self, container: &Container) -> bool {
        let filter = self.active_filter();
        if filter.is_empty() {
            return true;
        }
        let query = filter.to_lowercase();
        container.name.to_lowercase().contains(&query)
            || container.image.to_lowercase().contains(&query)
            || container
                .compose_service
                .as_deref()
                .is_some_and(|s| s.to_lowercase().contains(&query))
            || container
                .compose_project
                .as_deref()
                .is_some_and(|s| s.to_lowercase().contains(&query))
    }

    pub fn rebuild_list_items(&mut self) {
        self.list_items.clear();

        let mut groups: Vec<(Option<String>, Vec<usize>)> = Vec::new();
        let mut group_map: HashMap<Option<String>, usize> = HashMap::new();

        for (idx, container) in self.containers.iter().enumerate() {
            if !self.matches_filter(container) {
                continue;
            }
            let key = container.compose_project.clone();
            if let Some(&group_idx) = group_map.get(&key) {
                groups[group_idx].1.push(idx);
            } else {
                group_map.insert(key.clone(), groups.len());
                groups.push((key, vec![idx]));
            }
        }

        groups.sort_by(|a, b| match (&a.0, &b.0) {
            (Some(a_name), Some(b_name)) => a_name.cmp(b_name),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        for (project, container_indices) in &groups {
            if let Some(project_name) = project {
                let running_count = container_indices
                    .iter()
                    .filter(|&&idx| self.containers[idx].state == ContainerState::Running)
                    .count();

                self.list_items.push(ListItem::GroupHeader {
                    project: project_name.clone(),
                    expanded: !self.collapsed_groups.contains(project_name),
                    running_count,
                    total_count: container_indices.len(),
                });

                if !self.collapsed_groups.contains(project_name) {
                    for &idx in container_indices {
                        self.list_items.push(ListItem::Container { index: idx });
                    }
                }
            } else {
                for &idx in container_indices {
                    self.list_items.push(ListItem::Container { index: idx });
                }
            }
        }

        if !self.list_items.is_empty() && self.selected_index >= self.list_items.len() {
            self.selected_index = self.list_items.len() - 1;
        }
    }

    pub fn selected_container(&self) -> Option<&Container> {
        self.list_items.get(self.selected_index).and_then(|item| {
            if let ListItem::Container { index } = item {
                self.containers.get(*index)
            } else {
                None
            }
        })
    }

    pub fn selected_container_id(&self) -> Option<String> {
        self.selected_container().map(|c| c.id.clone())
    }

    pub fn selected_group_project(&self) -> Option<&str> {
        if let Some(ListItem::GroupHeader { project, .. }) =
            self.list_items.get(self.selected_index)
        {
            Some(project.as_str())
        } else {
            None
        }
    }

    /// A unique key for what's currently selected (container ID or "group:<project>")
    pub fn selected_key(&self) -> Option<String> {
        if let Some(id) = self.selected_container_id() {
            Some(id)
        } else {
            self.selected_group_project().map(|p| format!("group:{p}"))
        }
    }

    pub fn group_containers(&self, project: &str) -> Vec<&Container> {
        self.containers
            .iter()
            .filter(|c| c.compose_project.as_deref() == Some(project))
            .collect()
    }

    pub fn move_selection_down(&mut self) {
        if !self.list_items.is_empty() && self.selected_index < self.list_items.len() - 1 {
            self.selected_index += 1;
            self.on_selection_changed();
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.on_selection_changed();
        }
    }

    pub fn on_selection_changed(&mut self) {
        let new_key = self.selected_key();
        if new_key == self.detail_loaded_for {
            return;
        }

        self.container_logs = None;
        self.detail_scroll_offset = 0;
        self.log_scroll_offset = 0;

        if self.selected_group_project().is_some() {
            if self.detail_tab == DetailTab::Logs {
                self.mark_dirty();
            } else {
                self.detail_loaded_for = new_key;
                self.detail_dirty = false;
            }
            return;
        }

        if let Some(ref id) = new_key {
            if self.detail_tab != DetailTab::Logs && self.info_cache.contains_key(id) {
                self.detail_loaded_for = new_key;
                self.detail_dirty = false;
                return;
            }
        }

        self.mark_dirty();
    }

    pub fn mark_dirty(&mut self) {
        self.detail_dirty = true;
        self.detail_generation += 1;
    }

    pub fn needs_detail_load(&self) -> bool {
        if !self.detail_dirty {
            return false;
        }
        let Some(key) = self.selected_key() else {
            return false;
        };

        // Group: only Logs tab needs loading
        if self.selected_group_project().is_some() {
            return self.detail_tab == DetailTab::Logs;
        }

        // Container: Info tab only needs load if not cached
        if self.detail_tab == DetailTab::Info && self.info_cache.contains_key(&key) {
            return false;
        }
        true
    }

    pub fn toggle_group(&mut self) {
        if let Some(ListItem::GroupHeader { project, .. }) =
            self.list_items.get(self.selected_index)
        {
            let project = project.clone();
            if self.collapsed_groups.contains(&project) {
                self.collapsed_groups.remove(&project);
            } else {
                self.collapsed_groups.insert(project);
            }
            self.rebuild_list_items();
        }
    }

    pub fn running_count(&self) -> usize {
        self.containers
            .iter()
            .filter(|c| c.state.is_running())
            .count()
    }
}
