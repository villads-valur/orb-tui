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
    /// Groups the user has manually toggled. Their collapse state won't be
    /// overridden by auto-collapse until the group's running state changes.
    pub manually_toggled: HashSet<String>,

    /// When true the detail panel takes the full width (hides container list).
    pub fullscreen_detail: bool,

    pub confirm_message: Option<String>,
    pub confirm_action: Option<ConfirmAction>,
    pub message: Option<String>,
    /// Shown in the footer until the next keypress. Unlike `message` it isn't cleared on refresh.
    pub config_error: Option<String>,

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
    RemoveGroup(Vec<String>),
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
            manually_toggled: HashSet::new(),

            fullscreen_detail: false,

            confirm_message: None,
            confirm_action: None,
            message: None,
            config_error: None,

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

        if let Some(ref id) = new_key
            && self.detail_tab != DetailTab::Logs
            && self.info_cache.contains_key(id)
        {
            self.detail_loaded_for = new_key;
            self.detail_dirty = false;
            return;
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
                self.collapsed_groups.insert(project.clone());
            }
            self.manually_toggled.insert(project);
            self.rebuild_list_items();
        }
    }

    /// Auto-collapse groups with no running containers and expand groups that
    /// have running containers. Manual overrides are respected until the
    /// group's running state changes (stopped <-> running).
    pub fn update_auto_collapse(&mut self) {
        let mut groups_with_running: HashSet<String> = HashSet::new();
        let mut all_projects: HashSet<String> = HashSet::new();

        for container in &self.containers {
            if let Some(ref project) = container.compose_project {
                all_projects.insert(project.clone());
                if container.state == ContainerState::Running {
                    groups_with_running.insert(project.clone());
                }
            }
        }

        for project in &all_projects {
            let has_running = groups_with_running.contains(project);
            let is_collapsed = self.collapsed_groups.contains(project);

            if self.manually_toggled.contains(project) {
                // Clear override once the user's choice matches what
                // auto-collapse would do anyway.
                let auto_would_collapse = !has_running;
                if is_collapsed == auto_would_collapse {
                    self.manually_toggled.remove(project);
                }
                continue;
            }

            if has_running {
                self.collapsed_groups.remove(project);
            } else {
                self.collapsed_groups.insert(project.clone());
            }
        }
    }

    pub fn running_count(&self) -> usize {
        self.containers
            .iter()
            .filter(|c| c.state.is_running())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn container(
        id: &str,
        state: ContainerState,
        project: Option<&str>,
        service: Option<&str>,
    ) -> Container {
        Container {
            id: id.to_string(),
            name: format!("{id}-name"),
            image: format!("{id}-image"),
            state,
            compose_project: project.map(String::from),
            compose_service: service.map(String::from),
        }
    }

    fn app_with(containers: Vec<Container>) -> App {
        let mut app = App::new();
        app.containers = containers;
        app.rebuild_list_items();
        app
    }

    fn shape(app: &App) -> Vec<String> {
        app.list_items
            .iter()
            .map(|item| match item {
                ListItem::GroupHeader {
                    project,
                    expanded,
                    running_count,
                    total_count,
                } => {
                    let marker = if *expanded { "" } else { "-" };
                    format!("H{marker}:{project}({running_count}/{total_count})")
                }
                ListItem::Container { index } => format!("C:{index}"),
            })
            .collect()
    }

    use ContainerState::{Exited, Running};

    #[test]
    fn empty_containers_gives_empty_list() {
        let app = app_with(vec![]);
        assert!(app.list_items.is_empty());
    }

    #[test]
    fn ungrouped_containers_have_no_header() {
        let app = app_with(vec![
            container("a", Running, None, None),
            container("b", Exited, None, None),
        ]);
        assert_eq!(shape(&app), ["C:0", "C:1"]);
    }

    #[test]
    fn groups_get_header_followed_by_their_containers() {
        let app = app_with(vec![
            container("a", Running, Some("api"), Some("web")),
            container("b", Running, Some("api"), Some("db")),
        ]);
        assert_eq!(shape(&app), ["H:api(2/2)", "C:0", "C:1"]);
    }

    #[test]
    fn groups_sorted_alphabetically_and_ungrouped_last() {
        let app = app_with(vec![
            container("b1", Running, Some("beta"), None),
            container("solo", Running, None, None),
            container("a1", Running, Some("alpha"), None),
        ]);
        assert_eq!(
            shape(&app),
            ["H:alpha(1/1)", "C:2", "H:beta(1/1)", "C:0", "C:1"]
        );
    }

    #[test]
    fn container_order_within_group_preserved() {
        let app = app_with(vec![
            container("c", Running, Some("p"), None),
            container("other", Running, Some("q"), None),
            container("a", Running, Some("p"), None),
            container("b", Running, Some("p"), None),
        ]);
        assert_eq!(
            shape(&app),
            ["H:p(3/3)", "C:0", "C:2", "C:3", "H:q(1/1)", "C:1"]
        );
    }

    #[test]
    fn header_counts_running_and_total() {
        let app = app_with(vec![
            container("a", Running, Some("p"), None),
            container("b", Exited, Some("p"), None),
            container("c", ContainerState::Paused, Some("p"), None),
        ]);
        assert_eq!(shape(&app)[0], "H:p(1/3)");
    }

    #[test]
    fn collapsed_group_shows_header_only() {
        let mut app = app_with(vec![
            container("a", Running, Some("p"), None),
            container("b", Exited, Some("p"), None),
            container("c", Running, Some("q"), None),
        ]);
        app.collapsed_groups.insert("p".to_string());
        app.rebuild_list_items();
        assert_eq!(shape(&app), ["H-:p(1/2)", "H:q(1/1)", "C:2"]);
    }

    #[test]
    fn collapse_does_not_affect_ungrouped() {
        let mut app = app_with(vec![
            container("a", Running, Some("p"), None),
            container("solo", Running, None, None),
        ]);
        app.collapsed_groups.insert("p".to_string());
        app.rebuild_list_items();
        assert_eq!(shape(&app), ["H-:p(1/1)", "C:1"]);
    }

    #[test]
    fn filter_matches_name_image_service_and_project() {
        let containers = vec![
            container("byname", Running, None, None),
            container("byimage", Running, None, None),
            container("byservice", Running, Some("x"), Some("svc-match")),
            container("byproject", Running, Some("proj-match"), None),
        ];
        let cases = [
            ("byname-name", vec!["C:0"]),
            ("byimage-image", vec!["C:1"]),
            ("svc-match", vec!["H:x(1/1)", "C:2"]),
            ("proj-match", vec!["H:proj-match(1/1)", "C:3"]),
        ];
        for (filter, expected) in cases {
            let mut app = app_with(containers.clone());
            app.search_filter = filter.to_string();
            app.rebuild_list_items();
            assert_eq!(shape(&app), expected, "filter: {filter}");
        }
    }

    #[test]
    fn filter_is_case_insensitive() {
        let mut app = app_with(vec![container("Web", Running, None, None)]);
        app.search_filter = "wEB-NAME".to_string();
        app.rebuild_list_items();
        assert_eq!(shape(&app), ["C:0"]);
    }

    #[test]
    fn filter_excludes_group_header_when_no_members_match() {
        let mut app = app_with(vec![
            container("a", Running, Some("p"), None),
            container("match", Running, None, None),
        ]);
        app.search_filter = "match".to_string();
        app.rebuild_list_items();
        assert_eq!(shape(&app), ["C:1"]);
    }

    #[test]
    fn header_counts_reflect_filtered_members_only() {
        let mut app = app_with(vec![
            container("web", Running, Some("p"), None),
            container("db", Exited, Some("p"), None),
            container("worker", Running, Some("p"), None),
        ]);
        app.search_filter = "web".to_string();
        app.rebuild_list_items();
        assert_eq!(shape(&app), ["H:p(1/1)", "C:0"]);
    }

    #[test]
    fn search_mode_uses_search_query_not_search_filter() {
        let mut app = app_with(vec![
            container("a", Running, None, None),
            container("b", Running, None, None),
        ]);
        app.search_filter = "a-name".to_string();
        app.search_query = "b-name".to_string();

        app.input_mode = InputMode::Search;
        app.rebuild_list_items();
        assert_eq!(shape(&app), ["C:1"]);

        app.input_mode = InputMode::Normal;
        app.rebuild_list_items();
        assert_eq!(shape(&app), ["C:0"]);
    }

    #[test]
    fn selected_index_clamped_when_list_shrinks() {
        let mut app = app_with(vec![
            container("a", Running, None, None),
            container("b", Running, None, None),
            container("c", Running, None, None),
        ]);
        app.selected_index = 2;
        app.search_filter = "a-name".to_string();
        app.rebuild_list_items();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn selected_index_untouched_when_list_becomes_empty() {
        let mut app = app_with(vec![
            container("a", Running, None, None),
            container("b", Running, None, None),
        ]);
        app.selected_index = 1;
        app.search_filter = "nothing-matches".to_string();
        app.rebuild_list_items();
        assert!(app.list_items.is_empty());
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn toggle_group_collapses_expands_and_marks_manual() {
        let mut app = app_with(vec![container("a", Running, Some("p"), None)]);
        app.selected_index = 0;

        app.toggle_group();
        assert_eq!(shape(&app), ["H-:p(1/1)"]);
        assert!(app.manually_toggled.contains("p"));

        app.toggle_group();
        assert_eq!(shape(&app), ["H:p(1/1)", "C:0"]);
    }

    #[test]
    fn toggle_group_on_container_row_does_nothing() {
        let mut app = app_with(vec![container("a", Running, Some("p"), None)]);
        app.selected_index = 1;
        app.toggle_group();
        assert!(app.collapsed_groups.is_empty());
        assert!(app.manually_toggled.is_empty());
    }

    #[test]
    fn auto_collapse_collapses_stopped_and_expands_running_groups() {
        let mut app = app_with(vec![
            container("a", Running, Some("live"), None),
            container("b", Exited, Some("live"), None),
            container("c", Exited, Some("dead"), None),
            container("solo", Exited, None, None),
        ]);
        app.collapsed_groups.insert("live".to_string());

        app.update_auto_collapse();

        assert!(!app.collapsed_groups.contains("live"));
        assert!(app.collapsed_groups.contains("dead"));
        assert_eq!(app.collapsed_groups.len(), 1);
    }

    #[test]
    fn auto_collapse_respects_manual_toggle_until_state_matches() {
        let mut app = app_with(vec![container("a", Running, Some("p"), None)]);

        // User collapses a running group: auto-collapse leaves it alone.
        app.collapsed_groups.insert("p".to_string());
        app.manually_toggled.insert("p".to_string());
        app.update_auto_collapse();
        assert!(app.collapsed_groups.contains("p"));
        assert!(app.manually_toggled.contains("p"));

        // Group stops: collapsed now matches auto behaviour, so override clears.
        app.containers[0].state = Exited;
        app.update_auto_collapse();
        assert!(app.collapsed_groups.contains("p"));
        assert!(!app.manually_toggled.contains("p"));

        // Group starts again: auto-collapse takes over and expands it.
        app.containers[0].state = Running;
        app.update_auto_collapse();
        assert!(!app.collapsed_groups.contains("p"));
    }
}
