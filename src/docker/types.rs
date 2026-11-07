use serde::Deserialize;
use std::collections::HashMap;

/// DTO for `docker ps --format json`. Extra fields kept for serde compatibility.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DockerPsEntry {
    #[serde(rename = "ID")]
    pub id: String,
    pub names: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub ports: String,
    pub labels: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DockerStatsEntry {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "CPUPerc")]
    pub cpu_perc: String,
    pub mem_usage: String,
    pub mem_perc: String,
    #[serde(rename = "NetIO")]
    pub net_io: String,
    #[serde(rename = "BlockIO")]
    pub block_io: String,
    #[serde(rename = "PIDs")]
    pub pids: String,
}

#[derive(Debug, Clone)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: ContainerState,
    pub compose_project: Option<String>,
    pub compose_service: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerState {
    Running,
    Exited,
    Paused,
    Restarting,
    Dead,
    Created,
    Removing,
}

impl ContainerState {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "running" => Self::Running,
            "exited" => Self::Exited,
            "paused" => Self::Paused,
            "restarting" => Self::Restarting,
            "dead" => Self::Dead,
            "created" => Self::Created,
            "removing" => Self::Removing,
            _ => Self::Exited,
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Running => "●",
            Self::Exited => "■",
            Self::Paused => "⏸",
            Self::Restarting => "↻",
            Self::Dead => "✕",
            Self::Created => "○",
            Self::Removing => "…",
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }
}

#[derive(Debug, Clone)]
pub struct PortForward {
    pub host_port: String,
    pub container_port: String,
    pub protocol: String,
}

#[derive(Debug, Clone)]
pub struct ContainerDetail {
    pub name: String,
    pub id: String,
    pub image: String,
    pub status: String,
    pub domain: Option<String>,
    pub ip_address: Option<String>,
    pub port_forwards: Vec<PortForward>,
    pub labels: HashMap<String, String>,
    pub restart_policy: String,
    pub network_mode: String,
    pub cpu_limit: Option<f64>,
    pub memory_limit: Option<u64>,
}

impl Container {
    pub fn from_ps_entry(entry: DockerPsEntry) -> Self {
        let labels = parse_labels(&entry.labels);
        let compose_project = labels.get("com.docker.compose.project").cloned();
        let compose_service = labels.get("com.docker.compose.service").cloned();
        let name = entry
            .names
            .split(',')
            .next()
            .unwrap_or(&entry.names)
            .to_string();

        Container {
            id: entry.id,
            name,
            image: entry.image,
            state: ContainerState::from_str(&entry.state),
            compose_project,
            compose_service,
        }
    }

    pub fn display_name(&self) -> &str {
        self.compose_service.as_deref().unwrap_or(&self.name)
    }
}

fn parse_labels(labels_str: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if labels_str.is_empty() {
        return map;
    }
    for pair in labels_str.split(',') {
        if let Some((key, value)) = pair.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    const PS_LINE: &str = r#"{"Command":"\"docker-entrypoint.s…\"","CreatedAt":"2026-11-01 10:00:00 +0100 CET","ID":"abc123","Image":"postgres:16","Labels":"com.docker.compose.project=shop,com.docker.compose.service=db,maintainer=someone","LocalVolumes":"1","Mounts":"pgdata","Names":"shop-db-1","Networks":"shop_default","Ports":"0.0.0.0:5432->5432/tcp","RunningFor":"2 hours ago","Size":"0B","State":"running","Status":"Up 2 hours"}"#;

    fn ps_entry(names: &str, labels: &str) -> DockerPsEntry {
        DockerPsEntry {
            id: "id1".to_string(),
            names: names.to_string(),
            image: "img".to_string(),
            state: "running".to_string(),
            status: "Up".to_string(),
            ports: String::new(),
            labels: labels.to_string(),
        }
    }

    #[test]
    fn deserializes_docker_ps_json_line() {
        let entry: DockerPsEntry = serde_json::from_str(PS_LINE).unwrap();
        assert_eq!(entry.id, "abc123");
        assert_eq!(entry.names, "shop-db-1");
        assert_eq!(entry.image, "postgres:16");
        assert_eq!(entry.state, "running");
        assert_eq!(entry.status, "Up 2 hours");
        assert_eq!(entry.ports, "0.0.0.0:5432->5432/tcp");
        assert!(entry.labels.contains("com.docker.compose.project=shop"));
    }

    #[test]
    fn deserializes_docker_stats_json_line() {
        let line = r#"{"BlockIO":"1MB / 2MB","CPUPerc":"0.50%","Container":"abc123","ID":"abc123","MemPerc":"1.20%","MemUsage":"50MiB / 4GiB","Name":"shop-db-1","NetIO":"3kB / 4kB","PIDs":"7"}"#;
        let entry: DockerStatsEntry = serde_json::from_str(line).unwrap();
        assert_eq!(entry.id, "abc123");
        assert_eq!(entry.cpu_perc, "0.50%");
        assert_eq!(entry.mem_usage, "50MiB / 4GiB");
        assert_eq!(entry.mem_perc, "1.20%");
        assert_eq!(entry.net_io, "3kB / 4kB");
        assert_eq!(entry.block_io, "1MB / 2MB");
        assert_eq!(entry.pids, "7");
    }

    #[test]
    fn ps_entry_missing_field_fails() {
        let line =
            r#"{"ID":"abc","Names":"n","Image":"i","State":"running","Status":"Up","Ports":""}"#;
        assert!(serde_json::from_str::<DockerPsEntry>(line).is_err());
    }

    #[test]
    fn from_ps_entry_extracts_compose_labels() {
        let entry: DockerPsEntry = serde_json::from_str(PS_LINE).unwrap();
        let container = Container::from_ps_entry(entry);
        assert_eq!(container.id, "abc123");
        assert_eq!(container.name, "shop-db-1");
        assert_eq!(container.image, "postgres:16");
        assert_eq!(container.state, ContainerState::Running);
        assert_eq!(container.compose_project.as_deref(), Some("shop"));
        assert_eq!(container.compose_service.as_deref(), Some("db"));
    }

    #[test]
    fn from_ps_entry_without_compose_labels() {
        let container = Container::from_ps_entry(ps_entry("plain", "foo=bar"));
        assert_eq!(container.compose_project, None);
        assert_eq!(container.compose_service, None);
    }

    #[test]
    fn from_ps_entry_uses_first_name() {
        let container = Container::from_ps_entry(ps_entry("first,second", ""));
        assert_eq!(container.name, "first");
    }

    #[test]
    fn parse_labels_empty_and_whitespace_and_value_with_equals() {
        assert!(parse_labels("").is_empty());

        let labels = parse_labels(" a = 1 ,b=x=y,novalue");
        assert_eq!(labels.get("a").map(String::as_str), Some("1"));
        assert_eq!(labels.get("b").map(String::as_str), Some("x=y"));
        assert!(!labels.contains_key("novalue"));
        assert_eq!(labels.len(), 2);
    }

    #[test]
    fn parse_labels_splits_values_containing_commas() {
        // Documents current behaviour: docker joins labels with ',' so a value
        // containing a comma is truncated and the remainder dropped.
        let labels = parse_labels("desc=one,two,k=v");
        assert_eq!(labels.get("desc").map(String::as_str), Some("one"));
        assert_eq!(labels.get("k").map(String::as_str), Some("v"));
        assert_eq!(labels.len(), 2);
    }

    #[test]
    fn container_state_from_str_all_variants_case_insensitive_and_unknown_is_exited() {
        let cases = [
            ("running", ContainerState::Running),
            ("RUNNING", ContainerState::Running),
            ("exited", ContainerState::Exited),
            ("Paused", ContainerState::Paused),
            ("restarting", ContainerState::Restarting),
            ("dead", ContainerState::Dead),
            ("created", ContainerState::Created),
            ("removing", ContainerState::Removing),
            ("something-new", ContainerState::Exited),
            ("", ContainerState::Exited),
        ];
        for (input, expected) in cases {
            assert_eq!(
                ContainerState::from_str(input),
                expected,
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn display_name_prefers_compose_service() {
        let with_service =
            Container::from_ps_entry(ps_entry("shop-web-1", "com.docker.compose.service=web"));
        assert_eq!(with_service.display_name(), "web");

        let without_service = Container::from_ps_entry(ps_entry("standalone", ""));
        assert_eq!(without_service.display_name(), "standalone");
    }
}
