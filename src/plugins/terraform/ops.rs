//! Terraform CLI operations

use super::state::{StateResource, WorkspaceEntry};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// A background terraform process
pub struct TerraformProcess {
    /// Shared output buffer
    pub output: Arc<Mutex<Vec<String>>>,
    /// Process result: None while running, Some(true) on success, Some(false) on failure
    pub result: Arc<Mutex<Option<bool>>>,
    /// Thread handle
    _handle: JoinHandle<()>,
}

impl TerraformProcess {
    /// Check if process is still running
    pub fn is_running(&self) -> bool {
        self.result.lock().unwrap().is_none()
    }

    /// Check if process succeeded (None if still running)
    pub fn succeeded(&self) -> Option<bool> {
        *self.result.lock().unwrap()
    }

    /// Get current output lines
    pub fn get_output(&self) -> Vec<String> {
        self.output.lock().unwrap().clone()
    }
}

/// Start a terraform command in the background
pub fn start_command(cwd: &Path, args: &[&str]) -> Result<TerraformProcess, String> {
    let output = Arc::new(Mutex::new(Vec::new()));
    let result = Arc::new(Mutex::new(None));

    let output_clone = Arc::clone(&output);
    let result_clone = Arc::clone(&result);
    let dir = cwd.to_path_buf();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    let handle = thread::spawn(move || {
        run_streaming_command(&dir, &args, &output_clone, &result_clone);
    });

    Ok(TerraformProcess {
        output,
        result,
        _handle: handle,
    })
}

/// Run a streaming command
fn run_streaming_command(
    cwd: &PathBuf,
    args: &[String],
    output: &Arc<Mutex<Vec<String>>>,
    result: &Arc<Mutex<Option<bool>>>,
) {
    let child_result = Command::new("terraform")
        .current_dir(cwd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    match child_result {
        Ok(mut child) => {
            read_command_output(&mut child, output);
            let success = child.wait().map(|s| s.success()).unwrap_or(false);
            *result.lock().unwrap() = Some(success);
        }
        Err(e) => {
            output
                .lock()
                .unwrap()
                .push(format!("Failed to start terraform: {}", e));
            *result.lock().unwrap() = Some(false);
        }
    }
}

/// Read output from a child process
fn read_command_output(child: &mut Child, output: &Arc<Mutex<Vec<String>>>) {
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

    if let Some(h) = stdout_handle {
        let _ = h.join();
    }
    if let Some(h) = stderr_handle {
        let _ = h.join();
    }
}

/// Run a command and capture output
fn run_command(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("terraform")
        .current_dir(cwd)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run terraform: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if stderr.is_empty() {
            if stdout.is_empty() {
                Err("Terraform failed with no output".to_string())
            } else {
                Err(stdout)
            }
        } else {
            Err(stderr)
        }
    }
}

/// Check if terraform is available
pub fn check_terraform() -> bool {
    Command::new("terraform")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if current directory is initialized
pub fn is_initialized(cwd: &Path) -> bool {
    cwd.join(".terraform").exists()
}

/// Initialize terraform
pub fn init(cwd: &Path) -> Result<Vec<String>, String> {
    let output = run_command(cwd, &["init", "-input=false", "-no-color"])?;
    Ok(output.lines().map(String::from).collect())
}

/// Run terraform plan
pub fn plan(cwd: &Path) -> Result<Vec<String>, String> {
    let output = run_command(cwd, &["plan", "-no-color", "-input=false"])?;
    Ok(output.lines().map(String::from).collect())
}

/// Run terraform apply (for TF Cloud, this triggers remote apply)
pub fn apply(cwd: &Path) -> Result<Vec<String>, String> {
    // Note: For TF Cloud users, this triggers a remote run
    let output = run_command(
        cwd,
        &["apply", "-auto-approve", "-no-color", "-input=false"],
    )?;
    Ok(output.lines().map(String::from).collect())
}

/// Run terraform destroy
pub fn destroy(cwd: &Path) -> Result<Vec<String>, String> {
    let output = run_command(
        cwd,
        &["destroy", "-auto-approve", "-no-color", "-input=false"],
    )?;
    Ok(output.lines().map(String::from).collect())
}

/// Run terraform refresh
pub fn refresh(cwd: &Path) -> Result<Vec<String>, String> {
    let output = run_command(cwd, &["refresh", "-no-color", "-input=false"])?;
    Ok(output.lines().map(String::from).collect())
}

/// Run terraform validate
pub fn validate(cwd: &Path) -> Result<Vec<String>, String> {
    let output = run_command(cwd, &["validate", "-no-color"])?;
    Ok(output.lines().map(String::from).collect())
}

/// List workspaces
pub fn list_workspaces(cwd: &Path) -> Result<Vec<WorkspaceEntry>, String> {
    let output = run_command(cwd, &["workspace", "list"])?;
    let mut workspaces = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let (is_current, name) = if let Some(name) = trimmed.strip_prefix("* ") {
            (true, name.to_string())
        } else {
            (false, trimmed.to_string())
        };

        workspaces.push(WorkspaceEntry { name, is_current });
    }

    Ok(workspaces)
}

/// Select workspace
pub fn select_workspace(cwd: &Path, name: &str) -> Result<String, String> {
    run_command(cwd, &["workspace", "select", name])
}

/// Create workspace
pub fn create_workspace(cwd: &Path, name: &str) -> Result<String, String> {
    run_command(cwd, &["workspace", "new", name])
}

/// Delete workspace
pub fn delete_workspace(cwd: &Path, name: &str) -> Result<String, String> {
    run_command(cwd, &["workspace", "delete", name])
}

/// List state resources
pub fn list_state(cwd: &Path) -> Result<Vec<StateResource>, String> {
    let output = run_command(cwd, &["state", "list"])?;
    let mut resources = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        resources.push(StateResource::from_address(trimmed));
    }

    Ok(resources)
}

/// Show state resource detail
pub fn show_state_resource(cwd: &Path, address: &str) -> Result<Vec<String>, String> {
    let output = run_command(cwd, &["state", "show", address])?;
    Ok(output.lines().map(String::from).collect())
}

/// Remove resource from state
pub fn state_remove(cwd: &Path, address: &str) -> Result<String, String> {
    run_command(cwd, &["state", "rm", address])
}

/// Get terraform version
pub fn version() -> Result<String, String> {
    let output = Command::new("terraform")
        .arg("version")
        .output()
        .map_err(|e| format!("Failed to get version: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("Unknown")
            .to_string())
    } else {
        Err("Failed to get version".to_string())
    }
}
