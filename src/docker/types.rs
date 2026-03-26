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
