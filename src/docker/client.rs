use std::collections::HashMap;
use std::process::Command;

use crate::docker::types::{
    Container, ContainerDetail, DockerPsEntry, DockerStatsEntry, PortForward,
};
use crate::error::{AppError, Result};

pub fn is_docker_available() -> bool {
    Command::new("docker")
        .args(["info"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub fn start_orbstack() -> Result<()> {
    Command::new("open")
        .args(["-gja", "OrbStack"])
        .status()
        .map_err(|e| AppError::DockerCommand(format!("Failed to open OrbStack: {e}")))?;
    Ok(())
}

pub fn list_containers() -> Result<Vec<Container>> {
    let output = Command::new("docker")
        .args(["ps", "-a", "--format", "json", "--no-trunc"])
        .output()
        .map_err(|e| AppError::DockerCommand(format!("Failed to run docker ps: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::DockerCommand(format!(
            "docker ps failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut containers = Vec::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: DockerPsEntry = serde_json::from_str(line)
            .map_err(|e| AppError::DockerParse(format!("Failed to parse container: {e}")))?;
        containers.push(Container::from_ps_entry(entry));
    }

    Ok(containers)
}

pub fn get_container_stats() -> Result<HashMap<String, DockerStatsEntry>> {
    let output = Command::new("docker")
        .args(["stats", "--no-stream", "--format", "json", "--no-trunc"])
        .output()
        .map_err(|e| AppError::DockerCommand(format!("Failed to run docker stats: {e}")))?;

    if !output.status.success() {
        return Ok(HashMap::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut stats = HashMap::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<DockerStatsEntry>(line) {
            stats.insert(entry.id.clone(), entry);
        }
    }

    Ok(stats)
}

pub fn get_container_logs(container_id: &str, tail: usize) -> Result<String> {
    let output = Command::new("docker")
        .args(["logs", "--tail", &tail.to_string(), container_id])
        .output()
        .map_err(|e| AppError::DockerCommand(format!("Failed to get logs: {e}")))?;

    // Docker logs writes to both stdout (for stdout logs) and stderr (for stderr logs)
    let mut logs = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr_logs = String::from_utf8_lossy(&output.stderr);
    if !stderr_logs.is_empty() {
        if !logs.is_empty() {
            logs.push('\n');
        }
        logs.push_str(&stderr_logs);
    }

    Ok(logs)
}

/// Fetch logs from multiple containers in parallel using threads.
/// Each line is prefixed with the service name.
pub fn get_group_logs(
    containers: &[(String, String)], // (container_id, service_name)
    tail_per_container: usize,
) -> Result<String> {
    use std::thread;

    let handles: Vec<_> = containers
        .iter()
        .map(|(id, name)| {
            let id = id.clone();
            let name = name.clone();
            let tail = tail_per_container;
            thread::spawn(move || {
                let logs = get_container_logs(&id, tail).unwrap_or_default();
                (name, logs)
            })
        })
        .collect();

    let mut all_lines: Vec<String> = Vec::new();
    for handle in handles {
        if let Ok((name, logs)) = handle.join() {
            for line in logs.lines() {
                if !line.is_empty() {
                    all_lines.push(format!("{name:<20} | {line}"));
                }
            }
        }
    }

    Ok(all_lines.join("\n"))
}

pub fn inspect_container(container_id: &str) -> Result<ContainerDetail> {
    let output = Command::new("docker")
        .args(["inspect", container_id])
        .output()
        .map_err(|e| AppError::DockerCommand(format!("Failed to inspect container: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::DockerCommand(format!(
            "docker inspect failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| AppError::DockerParse(format!("Failed to parse inspect: {e}")))?;

    let info = parsed
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| AppError::DockerParse("Empty inspect result".to_string()))?;

    let str_field = |path: &serde_json::Value| path.as_str().unwrap_or("").to_string();

    let name = info["Name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('/')
        .to_string();
    let id = info["Id"].as_str().unwrap_or("").chars().take(12).collect();
    let fallback_image = str_field(&info["Image"]);
    let image = info["Config"]["Image"]
        .as_str()
        .unwrap_or(&fallback_image)
        .to_string();
    let status = str_field(&info["State"]["Status"]);
    let restart_policy = str_field(&info["HostConfig"]["RestartPolicy"]["Name"]);
    let network_mode = str_field(&info["HostConfig"]["NetworkMode"]);
    let port_forwards = parse_port_bindings(info);
    let ip_address = extract_ip_address(info);
    let labels = extract_labels(info);
    let domain = build_orb_domain(&labels);

    let nano_cpus = info["HostConfig"]["NanoCpus"].as_u64().unwrap_or(0);
    let cpu_limit = if nano_cpus > 0 {
        Some(nano_cpus as f64 / 1_000_000_000.0)
    } else {
        None
    };

    let memory_bytes = info["HostConfig"]["Memory"].as_u64().unwrap_or(0);
    let memory_limit = if memory_bytes > 0 {
        Some(memory_bytes)
    } else {
        None
    };

    Ok(ContainerDetail {
        name,
        id,
        image,
        status,
        domain,
        ip_address,
        port_forwards,
        labels,
        restart_policy,
        network_mode,
        cpu_limit,
        memory_limit,
    })
}

/// Run a docker action on multiple containers in parallel. Returns the count of successes.
pub fn run_action_parallel(action: &str, container_ids: &[String]) -> (usize, Vec<String>) {
    use std::thread;

    let handles: Vec<_> = container_ids
        .iter()
        .map(|id| {
            let action = action.to_string();
            let id = id.clone();
            thread::spawn(move || run_docker_action(&action, &id))
        })
        .collect();

    let mut successes = 0;
    let mut errors = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok(Ok(())) => successes += 1,
            Ok(Err(e)) => errors.push(format!("{e}")),
            Err(_) => errors.push("Thread panicked".to_string()),
        }
    }
    (successes, errors)
}

pub fn remove_container(container_id: &str) -> Result<()> {
    let output = Command::new("docker")
        .args(["rm", "--force", container_id])
        .output()
        .map_err(|e| AppError::DockerCommand(format!("Failed to remove container: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::DockerCommand(format!(
            "docker rm failed: {stderr}"
        )));
    }
    Ok(())
}

fn run_docker_action(action: &str, container_id: &str) -> Result<()> {
    let output = Command::new("docker")
        .args([action, container_id])
        .output()
        .map_err(|e| AppError::DockerCommand(format!("Failed to {action} container: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::DockerCommand(format!(
            "docker {action} failed: {stderr}"
        )));
    }

    Ok(())
}

fn parse_port_bindings(info: &serde_json::Value) -> Vec<PortForward> {
    let Some(bindings) = info["HostConfig"]["PortBindings"].as_object() else {
        return Vec::new();
    };

    let mut ports = Vec::new();
    for (container_port_proto, host_bindings) in bindings {
        let (container_port, protocol) = container_port_proto
            .split_once('/')
            .unwrap_or((container_port_proto, "tcp"));

        let Some(bindings_arr) = host_bindings.as_array() else {
            continue;
        };

        for binding in bindings_arr {
            let host_port = binding["HostPort"].as_str().unwrap_or("");
            if !host_port.is_empty() {
                ports.push(PortForward {
                    host_port: host_port.to_string(),
                    container_port: container_port.to_string(),
                    protocol: protocol.to_uppercase(),
                });
            }
        }
    }
    ports
}

fn extract_ip_address(info: &serde_json::Value) -> Option<String> {
    info["NetworkSettings"]["Networks"]
        .as_object()?
        .values()
        .find_map(|network| {
            let ip = network["IPAddress"].as_str()?;
            (!ip.is_empty()).then(|| ip.to_string())
        })
}

fn extract_labels(info: &serde_json::Value) -> HashMap<String, String> {
    info["Config"]["Labels"]
        .as_object()
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn build_orb_domain(labels: &HashMap<String, String>) -> Option<String> {
    let project = labels.get("com.docker.compose.project")?;
    let service = labels.get("com.docker.compose.service")?;
    Some(format!("{service}.{project}.orb.local"))
}
