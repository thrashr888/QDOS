//! Docker CLI operations

use super::state::{ContainerEntry, ContainerStatus, ImageEntry, NetworkEntry, VolumeEntry};
use std::process::Command;

/// Run a command and capture output
fn run_command(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", cmd, e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if stderr.is_empty() {
            Err(format!("{} failed with no output", cmd))
        } else {
            Err(stderr)
        }
    }
}

/// Check if docker is available
pub fn check_docker() -> bool {
    Command::new("docker")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// List containers
pub fn list_containers(all: bool) -> Result<Vec<ContainerEntry>, String> {
    let format = "{{.ID}}|{{.Names}}|{{.Image}}|{{.Status}}|{{.Ports}}|{{.CreatedAt}}";
    let mut args = vec!["ps", "--format", format];
    if all {
        args.push("-a");
    }

    let output = run_command("docker", &args)?;
    let mut containers = Vec::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(6, '|').collect();
        if parts.len() >= 4 {
            containers.push(ContainerEntry {
                id: parts[0].to_string(),
                name: parts.get(1).unwrap_or(&"").to_string(),
                image: parts.get(2).unwrap_or(&"").to_string(),
                status_text: parts.get(3).unwrap_or(&"").to_string(),
                status: ContainerStatus::from_str(parts.get(3).unwrap_or(&"")),
                ports: parts.get(4).unwrap_or(&"").to_string(),
                created: parts.get(5).unwrap_or(&"").to_string(),
            });
        }
    }

    Ok(containers)
}

/// Start container
pub fn start_container(id: &str) -> Result<String, String> {
    run_command("docker", &["start", id])
}

/// Stop container
pub fn stop_container(id: &str) -> Result<String, String> {
    run_command("docker", &["stop", id])
}

/// Restart container
pub fn restart_container(id: &str) -> Result<String, String> {
    run_command("docker", &["restart", id])
}

/// Remove container
pub fn remove_container(id: &str, force: bool) -> Result<String, String> {
    if force {
        run_command("docker", &["rm", "-f", id])
    } else {
        run_command("docker", &["rm", id])
    }
}

/// Get container logs
pub fn get_logs(id: &str, tail: usize) -> Result<Vec<String>, String> {
    let tail_str = tail.to_string();
    let output = run_command("docker", &["logs", "--tail", &tail_str, id])?;
    Ok(output.lines().map(String::from).collect())
}

/// Exec command in container (returns output)
pub fn exec_command(id: &str, cmd: &str) -> Result<String, String> {
    // Split command by spaces (simple parsing)
    let mut args = vec!["exec", id];
    args.extend(cmd.split_whitespace());
    run_command("docker", &args)
}

/// Inspect container/image
pub fn inspect(id: &str) -> Result<String, String> {
    run_command("docker", &["inspect", id])
}

/// List images
pub fn list_images() -> Result<Vec<ImageEntry>, String> {
    let format = "{{.ID}}|{{.Repository}}|{{.Tag}}|{{.Size}}|{{.CreatedAt}}";
    let output = run_command("docker", &["images", "--format", format])?;
    let mut images = Vec::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(5, '|').collect();
        if parts.len() >= 4 {
            images.push(ImageEntry {
                id: parts[0].to_string(),
                repository: parts.get(1).unwrap_or(&"").to_string(),
                tag: parts.get(2).unwrap_or(&"").to_string(),
                size: parts.get(3).unwrap_or(&"").to_string(),
                created: parts.get(4).unwrap_or(&"").to_string(),
            });
        }
    }

    Ok(images)
}

/// Pull image
pub fn pull_image(name: &str) -> Result<String, String> {
    run_command("docker", &["pull", name])
}

/// Remove image
pub fn remove_image(id: &str) -> Result<String, String> {
    run_command("docker", &["rmi", id])
}

/// List volumes
pub fn list_volumes() -> Result<Vec<VolumeEntry>, String> {
    let format = "{{.Name}}|{{.Driver}}|{{.Mountpoint}}";
    let output = run_command("docker", &["volume", "ls", "--format", format])?;
    let mut volumes = Vec::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, '|').collect();
        if parts.len() >= 2 {
            volumes.push(VolumeEntry {
                name: parts[0].to_string(),
                driver: parts.get(1).unwrap_or(&"").to_string(),
                mountpoint: parts.get(2).unwrap_or(&"").to_string(),
            });
        }
    }

    Ok(volumes)
}

/// Remove volume
pub fn remove_volume(name: &str) -> Result<String, String> {
    run_command("docker", &["volume", "rm", name])
}

/// List networks
pub fn list_networks() -> Result<Vec<NetworkEntry>, String> {
    let format = "{{.ID}}|{{.Name}}|{{.Driver}}|{{.Scope}}";
    let output = run_command("docker", &["network", "ls", "--format", format])?;
    let mut networks = Vec::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() >= 3 {
            networks.push(NetworkEntry {
                id: parts[0].to_string(),
                name: parts.get(1).unwrap_or(&"").to_string(),
                driver: parts.get(2).unwrap_or(&"").to_string(),
                scope: parts.get(3).unwrap_or(&"").to_string(),
            });
        }
    }

    Ok(networks)
}

/// Remove network
pub fn remove_network(name: &str) -> Result<String, String> {
    run_command("docker", &["network", "rm", name])
}

/// Prune stopped containers
pub fn prune_containers() -> Result<String, String> {
    run_command("docker", &["container", "prune", "-f"])
}

/// Prune unused images
pub fn prune_images() -> Result<String, String> {
    run_command("docker", &["image", "prune", "-f"])
}

// === Build operations ===

use std::io::{BufRead, BufReader};
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// A background build process
pub struct BuildProcess {
    /// Shared output buffer
    pub output: Arc<Mutex<Vec<String>>>,
    /// Build result: None while running, Some(true) on success, Some(false) on failure
    pub result: Arc<Mutex<Option<bool>>>,
    /// Thread handle
    _handle: JoinHandle<()>,
}

impl BuildProcess {
    /// Check if build is still running
    pub fn is_running(&self) -> bool {
        self.result.lock().unwrap().is_none()
    }

    /// Check if build succeeded (None if still running)
    pub fn succeeded(&self) -> Option<bool> {
        *self.result.lock().unwrap()
    }

    /// Get current output lines
    pub fn get_output(&self) -> Vec<String> {
        self.output.lock().unwrap().clone()
    }
}

/// Start a build process in the background
pub fn start_build(dockerfile_dir: &std::path::Path, tag: &str) -> Result<BuildProcess, String> {
    let output = Arc::new(Mutex::new(Vec::new()));
    let result = Arc::new(Mutex::new(None));

    let output_clone = Arc::clone(&output);
    let result_clone = Arc::clone(&result);
    let dir = dockerfile_dir.to_path_buf();
    let tag = tag.to_string();

    let handle = thread::spawn(move || {
        // Spawn docker build with piped output
        let child_result = std::process::Command::new("docker")
            .current_dir(&dir)
            .args(["build", "-t", &tag, "."])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match child_result {
            Ok(mut child) => {
                read_build_output(&mut child, &output_clone);

                // Wait for process to complete
                let success = child.wait().map(|s| s.success()).unwrap_or(false);
                *result_clone.lock().unwrap() = Some(success);
            }
            Err(e) => {
                output_clone
                    .lock()
                    .unwrap()
                    .push(format!("Failed to start build: {}", e));
                *result_clone.lock().unwrap() = Some(false);
            }
        }
    });

    Ok(BuildProcess {
        output,
        result,
        _handle: handle,
    })
}

/// Read output from build process
fn read_build_output(child: &mut Child, output: &Arc<Mutex<Vec<String>>>) {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let output_stdout = Arc::clone(output);
    let stdout_handle = stdout.map(|stdout| {
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                output_stdout.lock().unwrap().push(line);
            }
        })
    });

    let output_stderr = Arc::clone(output);
    let stderr_handle = stderr.map(|stderr| {
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                output_stderr.lock().unwrap().push(line);
            }
        })
    });

    // Wait for both readers to finish
    if let Some(h) = stdout_handle {
        let _ = h.join();
    }
    if let Some(h) = stderr_handle {
        let _ = h.join();
    }
}

/// Build image from Dockerfile (blocking version for compatibility)
pub fn build_image(dockerfile_dir: &std::path::Path, tag: &str) -> Result<Vec<String>, String> {
    let output = std::process::Command::new("docker")
        .current_dir(dockerfile_dir)
        .args(["build", "-t", tag, "."])
        .output()
        .map_err(|e| format!("Failed to run docker build: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Docker build outputs to stderr for progress
    let mut lines: Vec<String> = stdout.lines().map(String::from).collect();
    lines.extend(stderr.lines().map(String::from));

    if output.status.success() {
        Ok(lines)
    } else {
        Err(lines.join("\n"))
    }
}

// === Compose operations ===

/// Check if docker-compose or docker compose is available
pub fn check_compose() -> bool {
    // Try docker compose (v2)
    if std::process::Command::new("docker")
        .args(["compose", "version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return true;
    }
    // Try docker-compose (v1)
    std::process::Command::new("docker-compose")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run compose command (handles v1 vs v2)
fn run_compose_command(compose_file: &std::path::Path, args: &[&str]) -> Result<String, String> {
    let dir = compose_file.parent().unwrap_or(std::path::Path::new("."));
    let file_name = compose_file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("docker-compose.yml");

    // Try docker compose (v2) first
    let output = std::process::Command::new("docker")
        .current_dir(dir)
        .args(["compose", "-f", file_name])
        .args(args)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            Ok(format!("{}{}", stdout, stderr))
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            Err(stderr)
        }
        Err(_) => {
            // Fall back to docker-compose (v1)
            let mut cmd_args = vec!["-f", file_name];
            cmd_args.extend(args);

            let output = std::process::Command::new("docker-compose")
                .current_dir(dir)
                .args(&cmd_args)
                .output()
                .map_err(|e| format!("Failed to run docker-compose: {}", e))?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                Ok(format!("{}{}", stdout, stderr))
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
    }
}

/// List compose services
pub fn compose_ps(
    compose_file: &std::path::Path,
) -> Result<Vec<super::state::ComposeService>, String> {
    let output = run_compose_command(compose_file, &["ps", "--format", "table"])?;
    let mut services = Vec::new();

    for line in output.lines().skip(1) {
        // Skip header
        if line.is_empty() {
            continue;
        }
        // Parse table format: NAME  SERVICE  STATUS  PORTS
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            services.push(super::state::ComposeService {
                name: parts[0].to_string(),
                status: parts.get(2).unwrap_or(&"").to_string(),
                ports: parts.get(3..).map(|p| p.join(" ")).unwrap_or_default(),
            });
        }
    }

    Ok(services)
}

/// Start compose services
pub fn compose_up(compose_file: &std::path::Path) -> Result<String, String> {
    run_compose_command(compose_file, &["up", "-d"])
}

/// Stop compose services
pub fn compose_down(compose_file: &std::path::Path) -> Result<String, String> {
    run_compose_command(compose_file, &["down"])
}

/// Restart a compose service
pub fn compose_restart(compose_file: &std::path::Path, service: &str) -> Result<String, String> {
    run_compose_command(compose_file, &["restart", service])
}

/// Get compose service logs
pub fn compose_logs(
    compose_file: &std::path::Path,
    service: &str,
    tail: usize,
) -> Result<Vec<String>, String> {
    let tail_str = tail.to_string();
    let output = run_compose_command(compose_file, &["logs", "--tail", &tail_str, service])?;
    Ok(output.lines().map(String::from).collect())
}
