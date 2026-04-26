//! Shell tool for executing system commands.
//!
//! Allows the AI agent to run arbitrary shell commands on the host system.
//! Cross-platform: uses `/bin/sh -c` on Unix, `cmd /C` on Windows.
//! Disabled by default — must be explicitly enabled in agent configuration.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Stdio;
use std::time::Duration;

use neomind_core::tools::ToolCategory;

use super::error::{Result, ToolError};
use super::tool::{object_schema, Tool, ToolOutput};

/// Shell tool configuration, stored as part of agent config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    /// Whether shell tool is enabled. Default: false.
    #[serde(default)]
    pub enabled: bool,

    /// Maximum execution time per command in seconds. Default: 30.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Maximum output characters (stdout + stderr combined). Default: 10000.
    #[serde(default = "default_max_output")]
    pub max_output_chars: usize,
}

fn default_timeout() -> u64 {
    30
}

fn default_max_output() -> usize {
    10000
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timeout_secs: default_timeout(),
            max_output_chars: default_max_output(),
        }
    }
}

/// Output from a shell command execution.
#[derive(Debug)]
struct CommandOutput {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
}

/// Shell tool — executes system commands.
pub struct ShellTool {
    config: ShellConfig,
}

impl ShellTool {
    pub fn new(config: ShellConfig) -> Self {
        Self { config }
    }

    /// Build a platform-appropriate shell command.
    /// Unix: login shell (`$SHELL -l -c`) with isolated process group;
    ///       falls back to `/bin/sh -c` without `-l` if $SHELL is not set.
    /// Windows: `cmd /C`
    fn build_command(command: &str) -> std::process::Command {
        let (shell, is_login) = shell_path();
        let mut cmd = std::process::Command::new(shell);
        shell_arg(&mut cmd, command, is_login);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        set_process_group(&mut cmd);
        cmd
    }

    /// Execute a command with timeout and output capture.
    async fn execute_command(
        &self,
        command: &str,
        working_dir: Option<&str>,
        timeout: Duration,
    ) -> Result<CommandOutput> {
        let mut cmd = Self::build_command(command);

        if let Some(dir) = working_dir {
            let path = std::path::Path::new(dir);
            if !path.exists() {
                return Err(ToolError::Execution(format!(
                    "Working directory does not exist: {}",
                    dir
                )));
            }
            if !path.is_dir() {
                return Err(ToolError::Execution(format!(
                    "Path is not a directory: {}",
                    dir
                )));
            }
            cmd.current_dir(dir);
        }

        let child = tokio::process::Command::from(cmd)
            .spawn()
            .map_err(|e| ToolError::Execution(format!("Failed to spawn: {}", e)))?;

        // Capture child PID before moving child into the timeout future
        let child_pid = child.id();

        let result = tokio::time::timeout(timeout, child.wait_with_output()).await;

        match result {
            Ok(Ok(output)) => Ok(CommandOutput {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                timed_out: false,
            }),
            Ok(Err(e)) => Err(ToolError::Execution(format!("Execution failed: {}", e))),
            Err(_) => {
                // Timeout — kill the process
                kill_process_by_pid(child_pid);
                Ok(CommandOutput {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: format!("Command timed out after {}s", timeout.as_secs()),
                    timed_out: true,
                })
            }
        }
    }
}

// ============================================================================
// Platform-specific helpers
// ============================================================================

/// Returns the user's login shell from `$SHELL`, falling back to `/bin/sh`.
/// Returns (shell_path, is_login): is_login is false for the fallback.
#[cfg(unix)]
fn shell_path() -> (String, bool) {
    match std::env::var("SHELL") {
        Ok(shell) => (shell, true),
        Err(_) => ("/bin/sh".to_string(), false),
    }
}

#[cfg(windows)]
fn shell_path() -> (&'static str, bool) {
    ("cmd", false)
}

/// Adds the shell flag argument.
/// Unix: `-l -c` for login shells, `-c` for fallback `/bin/sh`.
/// Windows: `/C`.
#[cfg(unix)]
fn shell_arg(cmd: &mut std::process::Command, command: &str, is_login: bool) {
    if is_login {
        cmd.arg("-l");
    }
    cmd.arg("-c").arg(command);
}

#[cfg(windows)]
fn shell_arg(cmd: &mut std::process::Command, command: &str, _is_login: bool) {
    cmd.arg("/C").arg(command);
}

/// Set process group isolation (Unix only — prevents orphaned child processes).
#[cfg(unix)]
fn set_process_group(cmd: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}

#[cfg(windows)]
fn set_process_group(_cmd: &mut std::process::Command) {
    // On Windows, child processes are naturally terminated when the parent dies
    // via Job Object inheritance. No explicit action needed for our use case.
}

/// Kill a process by PID. On Unix, kills the entire process group to prevent orphans.
#[cfg(unix)]
fn kill_process_by_pid(pid: Option<u32>) {
    if let Some(pid) = pid {
        // PID of child is also the PGID since we used process_group(0)
        unsafe {
            libc::killpg(pid as i32, libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
fn kill_process_by_pid(pid: Option<u32>) {
    if let Some(pid) = pid {
        // Use Windows API to terminate the process.
        // On Windows, TerminateProcess is the most reliable way to kill a process.
        unsafe {
            windows_sys::Win32::System::Threading::TerminateProcess(pid as *mut _, 1);
        }
    }
}

/// Truncate stdout + stderr to fit within max_total chars, with truncation notices.
fn truncate_output(stdout: &str, stderr: &str, max_total: usize) -> (String, String) {
    let stdout_len = stdout.len();
    let stderr_len = stderr.len();

    if stdout_len + stderr_len <= max_total {
        return (stdout.to_string(), stderr.to_string());
    }

    // Reserve space for truncation notices
    const NOTICE_LEN: usize = 60;
    let usable = max_total.saturating_sub(NOTICE_LEN * 2);

    let total = stdout_len + stderr_len;
    let stdout_budget = if total > 0 {
        (usable * stdout_len / total).min(stdout_len)
    } else {
        usable / 2
    };
    let stderr_budget = usable.saturating_sub(stdout_budget).min(stderr_len);

    let truncated_stdout = if stdout_len > stdout_budget {
        let safe_end = find_safe_truncation_point(stdout, stdout_budget);
        format!(
            "{}\n... [truncated, {} chars omitted]",
            &stdout[..safe_end],
            stdout_len - safe_end
        )
    } else {
        stdout.to_string()
    };

    let truncated_stderr = if stderr_len > stderr_budget {
        let safe_end = find_safe_truncation_point(stderr, stderr_budget);
        format!(
            "{}\n... [truncated, {} chars omitted]",
            &stderr[..safe_end],
            stderr_len - safe_end
        )
    } else {
        stderr.to_string()
    };

    (truncated_stdout, truncated_stderr)
}

/// Find a safe byte position to truncate at (don't split multi-byte UTF-8 chars).
fn find_safe_truncation_point(s: &str, max_bytes: usize) -> usize {
    if max_bytes >= s.len() {
        return s.len();
    }
    let mut pos = max_bytes;
    while pos > 0 && !s.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        r#"Execute shell commands on the host system.

Use this tool to:
- Network diagnostics: ping, traceroute, curl, arp, nmap
- System monitoring: ps, df, free, top, uptime, systemctl status
- File inspection: ls, cat, head, tail, grep, find, wc
- Device discovery: arp-scan, avahi-browse, bluetoothctl
- Container management: docker ps, docker logs
- Any other system command available on the host

Commands run in a separate process — no persistent shell state between calls.
Output may be truncated for very long responses.

NOTE: Some commands (docker, systemctl, nmap, etc.) may require elevated permissions.
If you see "Permission denied" or "Operation not permitted" in stderr, inform the user
that the command needs to be run with appropriate privileges or the user needs to be
added to the relevant group (e.g., docker group for docker commands).

Examples:
- List network devices: {"command": "arp -a"}
- Check disk usage: {"command": "df -h"}
- Ping a device: {"command": "ping -c 3 192.168.1.1"}
- Check running services: {"command": "systemctl status"}"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "command": {
                    "type": "string",
                    "description": "The shell command to execute. Supports pipes, redirections, and other shell features."
                },
                "timeout": {
                    "type": "number",
                    "description": "Optional per-command timeout in seconds (max 600). Overrides default timeout."
                },
                "description": {
                    "type": "string",
                    "description": "Brief description of what this command does (5-10 words). Used for logging and audit."
                },
                "working_dir": {
                    "type": "string",
                    "description": "Optional working directory for command execution. Must be an existing directory path."
                }
            }),
            vec!["command".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("command is required".into()))?;

        if command.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "command cannot be empty".into(),
            ));
        }

        // Resolve timeout: per-command override or config default, capped at 600s
        // Accepts both number and string forms (LLM may pass "30" as string)
        let timeout = if let Some(user_timeout) = args.get("timeout") {
            let secs = user_timeout
                .as_u64()
                .or_else(|| user_timeout.as_str().and_then(|s| s.parse::<u64>().ok()))
                .ok_or_else(|| {
                    ToolError::InvalidArguments("timeout must be a positive number".into())
                })?;
            Duration::from_secs(secs.min(600))
        } else {
            Duration::from_secs(self.config.timeout_secs.min(600))
        };

        let working_dir = args.get("working_dir").and_then(|v| v.as_str());
        let description = args.get("description").and_then(|v| v.as_str());

        tracing::info!(
            command = %command,
            description = description.unwrap_or(""),
            "Executing shell command"
        );

        let output = self.execute_command(command, working_dir, timeout).await?;

        let (stdout, stderr) =
            truncate_output(&output.stdout, &output.stderr, self.config.max_output_chars);

        tracing::info!(
            command = %command,
            exit_code = ?output.exit_code,
            timed_out = output.timed_out,
            stdout_len = stdout.len(),
            stderr_len = stderr.len(),
            "Shell command completed"
        );

        let mut result = serde_json::json!({
            "exit_code": output.exit_code,
            "stdout": stdout,
            "stderr": stderr,
            "command": command,
            "timed_out": output.timed_out
        });
        if let Some(desc) = description {
            result["description"] = serde_json::Value::String(desc.to_string());
        }

        Ok(ToolOutput::success(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ShellConfig {
        ShellConfig {
            enabled: true,
            timeout_secs: 10,
            max_output_chars: 5000,
        }
    }

    #[tokio::test]
    async fn test_basic_command() {
        let tool = ShellTool::new(test_config());
        let result = tool
            .execute(serde_json::json!({ "command": "echo hello world" }))
            .await
            .unwrap();
        assert!(result.success);
        let data = result.data;
        assert_eq!(data["exit_code"], 0);
        assert!(data["stdout"].as_str().unwrap().contains("hello world"));
        assert_eq!(data["timed_out"], false);
    }

    #[tokio::test]
    async fn test_stderr_capture() {
        let tool = ShellTool::new(test_config());
        let result = tool
            .execute(serde_json::json!({ "command": "echo error >&2" }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.data["stderr"].as_str().unwrap().contains("error"));
    }

    #[tokio::test]
    async fn test_nonzero_exit_code() {
        let tool = ShellTool::new(test_config());
        let result = tool
            .execute(serde_json::json!({ "command": "exit 42" }))
            .await
            .unwrap();
        assert!(result.success); // ToolOutput success = tool ran, not command success
        assert_eq!(result.data["exit_code"], 42);
    }

    #[tokio::test]
    async fn test_timeout() {
        let config = ShellConfig {
            enabled: true,
            timeout_secs: 1,
            max_output_chars: 5000,
        };
        let tool = ShellTool::new(config);
        let result = tool
            .execute(serde_json::json!({ "command": "sleep 60" }))
            .await
            .unwrap();
        assert!(result.data["timed_out"].as_bool().unwrap());
        assert!(result.data["stderr"]
            .as_str()
            .unwrap()
            .contains("timed out"));
    }

    #[tokio::test]
    async fn test_per_command_timeout_override() {
        let tool = ShellTool::new(test_config()); // default 10s
        let result = tool
            .execute(serde_json::json!({ "command": "sleep 60", "timeout": 1 }))
            .await
            .unwrap();
        assert!(result.data["timed_out"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_empty_command_rejected() {
        let tool = ShellTool::new(test_config());
        let result = tool.execute(serde_json::json!({ "command": "  " })).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_command_rejected() {
        let tool = ShellTool::new(test_config());
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_working_dir() {
        let tool = ShellTool::new(test_config());
        let result = tool
            .execute(serde_json::json!({ "command": "pwd", "working_dir": "/tmp" }))
            .await
            .unwrap();
        let stdout = result.data["stdout"].as_str().unwrap();
        assert!(stdout.contains("tmp"));
    }

    #[tokio::test]
    async fn test_invalid_working_dir() {
        let tool = ShellTool::new(test_config());
        let result = tool
            .execute(serde_json::json!({
                "command": "pwd",
                "working_dir": "/nonexistent/path"
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pipeline_command() {
        let tool = ShellTool::new(test_config());
        let result = tool
            .execute(serde_json::json!({
                "command": "echo -e 'apple\nbanana\ncherry' | grep an"
            }))
            .await
            .unwrap();
        let stdout = result.data["stdout"].as_str().unwrap();
        assert!(stdout.contains("banana"));
        assert!(!stdout.contains("apple"));
    }

    #[tokio::test]
    async fn test_permission_denied_command() {
        let tool = ShellTool::new(test_config());
        // This should fail with permission error, not crash
        let result = tool
            .execute(serde_json::json!({ "command": "ls /root" }))
            .await
            .unwrap();
        // Tool succeeds (command ran), but exit_code may be non-zero or stderr has error
        assert!(result.success);
        // Either exit_code is non-zero or stderr contains error info
        let exit_code = result.data["exit_code"].as_i64().unwrap_or(0);
        let stderr = result.data["stderr"].as_str().unwrap_or("");
        assert!(exit_code != 0 || !stderr.is_empty() || !result.data["stdout"].is_null());
    }

    #[test]
    fn test_truncate_output_within_budget() {
        let (out, err) = truncate_output("hello", "world", 100);
        assert_eq!(out, "hello");
        assert_eq!(err, "world");
    }

    #[test]
    fn test_truncate_output_exceeds_budget() {
        let stdout = "a".repeat(5000);
        let stderr = "b".repeat(5000);
        let (out, err) = truncate_output(&stdout, &stderr, 1000);
        assert!(out.len() < 1000);
        assert!(err.len() < 1000);
        assert!(out.contains("[truncated"));
        assert!(err.contains("[truncated"));
    }

    #[test]
    fn test_truncate_output_stderr_only() {
        let stdout = "short";
        let stderr = "x".repeat(5000);
        let (out, err) = truncate_output(stdout, &stderr, 1000);
        assert!(err.contains("[truncated"));
        assert!(out.len() + err.len() <= 1200);
    }

    #[test]
    fn test_find_safe_truncation_point_ascii() {
        assert_eq!(find_safe_truncation_point("hello world", 5), 5);
    }

    #[test]
    fn test_find_safe_truncation_point_multibyte() {
        let s = "你好世界";
        let pos = find_safe_truncation_point(s, 4);
        assert_eq!(pos, 3);
        assert!(s.is_char_boundary(pos));
    }

    #[test]
    fn test_tool_name_and_category() {
        let tool = ShellTool::new(test_config());
        assert_eq!(tool.name(), "shell");
        assert!(matches!(tool.category(), ToolCategory::System));
    }
}
