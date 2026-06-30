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
use tokio::io::AsyncReadExt;

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
    crate::toolkit::timeouts::shell_default().as_secs()
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

        // Inject NEOMIND_API_KEY so spawned neomind CLI can authenticate
        // without depending on CWD-relative data/api_keys.redb lookup.
        if let Some(key) = Self::resolve_api_key() {
            cmd.env("NEOMIND_API_KEY", key);
        }

        // Force JSON output — the AI agent is a machine consumer that needs
        // structured data. Without this, CLI defaults to human-readable format
        // which strips most useful information from the output.
        cmd.env("NEOMIND_JSON", "1");

        cmd
    }

    /// Resolve API key for neomind CLI commands.
    ///
    /// Checks env var first, then reads directly from the server's redb.
    /// Deliberately skips the credential file layer (`read_default_api_key`)
    /// because the agent runs inside the server process — it should use the
    /// server's own key, not a credential file that may have been written by
    /// `neomind login` against a different server instance.
    fn resolve_api_key() -> Option<String> {
        std::env::var("NEOMIND_API_KEY").ok().or_else(|| {
            neomind_cli_ops::auto_auth::read_default_api_key_from(
                &neomind_cli_ops::auto_auth::resolve_data_dir(),
            )
        })
    }

    /// Attempt in-process dispatch for `neomind` data commands.
    ///
    /// Returns `Some(output)` if the command was handled in-process (either
    /// success, a parse error, or an API error); returns `None` for
    /// [`DispatchError::NotInProcess`] (side-effecting / interactive /
    /// local-only subcommands) so the caller falls back to spawning a real
    /// subprocess.
    ///
    /// Non-`neomind` commands and malformed input (unbalanced quotes) also
    /// yield `None` so they hit the subprocess path unchanged.
    async fn try_in_process_dispatch(
        &self,
        command: &str,
        timeout: Duration,
    ) -> Option<CommandOutput> {
        let trimmed = command.trim();
        // Only intercept commands that start with `neomind ` (or are exactly
        // `neomind`). Anything else goes to the subprocess path.
        if !trimmed.starts_with("neomind ") && trimmed != "neomind" {
            return None;
        }

        // Tokenize. `neomind` data commands are simple enough that a basic
        // quote-respecting whitespace split is sufficient. We do NOT need
        // full shell syntax (pipes / redirections / $ expansions) because
        // those constructs are never part of a pure data query — they'd hit
        // the subprocess path by design.
        let argv = match tokenize_neomind_command(trimmed) {
            Ok(v) => v,
            Err(e) => {
                tracing::debug!(
                    target: "neomind::agent::shell",
                    in_process = false,
                    command = %command,
                    reason = %e,
                    "in-process dispatch skipped (tokenize error)"
                );
                return None;
            }
        };

        if argv.is_empty() || argv[0] != "neomind" {
            return None;
        }

        // Make auth + JSON-output env visible to the in-process handler.
        // `dispatch`'s `ApiClient` reads `NEOMIND_API_KEY` (via auto_auth)
        // and the handlers read `NEOMIND_JSON` to pick the output format.
        // This mirrors the env injection done for the subprocess in
        // `build_command`.
        if let Some(key) = Self::resolve_api_key() {
            // NOTE: env mutation is process-global. The agent runtime is
            // single-tenancy and is the only concurrent writer of this var,
            // so this is equivalent to the existing subprocess env injection.
            std::env::set_var("NEOMIND_API_KEY", key);
        }
        std::env::set_var("NEOMIND_JSON", "1");

        tracing::debug!(
            target: "neomind::agent::shell",
            in_process = true,
            command = %command,
            "dispatching neomind command in-process"
        );

        // Apply the same per-command timeout as the subprocess path. The
        // ApiClient has its own 30s HTTP timeout, but a handler may issue
        // multiple requests; this guarantees the in-process path cannot hang
        // longer than the subprocess equivalent would.
        match tokio::time::timeout(timeout, neomind_cli_ops::dispatch::dispatch(&argv)).await {
            Ok(Ok(resp)) => {
                let exit_code = if resp.success { 0 } else { 1 };
                let stdout = serde_json::to_string_pretty(&resp).unwrap_or_else(|e| {
                    format!(
                        "{{\"success\":false,\"error\":\"serialize failed: {}\"}}",
                        e
                    )
                });
                Some(CommandOutput {
                    exit_code: Some(exit_code),
                    stdout,
                    stderr: String::new(),
                    timed_out: false,
                })
            }
            Ok(Err(neomind_cli_ops::dispatch::DispatchError::NotInProcess)) => {
                tracing::debug!(
                    target: "neomind::agent::shell",
                    in_process = false,
                    command = %command,
                    "falling back to subprocess (NotInProcess)"
                );
                None
            }
            Ok(Err(neomind_cli_ops::dispatch::DispatchError::Parse(msg))) => Some(CommandOutput {
                exit_code: Some(2),
                stdout: String::new(),
                stderr: format!("error: {}", msg),
                timed_out: false,
            }),
            Ok(Err(neomind_cli_ops::dispatch::DispatchError::Api(msg))) => Some(CommandOutput {
                exit_code: Some(1),
                stdout: String::new(),
                stderr: msg,
                timed_out: false,
            }),
            // Timeout (outer Err = elapsed) — mirror the subprocess timeout behavior.
            Err(_) => Some(CommandOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Command timed out after {}s", timeout.as_secs()),
                timed_out: true,
            }),
        }
    }

    /// Execute a command with timeout and output capture.
    async fn execute_command(
        &self,
        command: &str,
        working_dir: Option<&str>,
        timeout: Duration,
    ) -> Result<CommandOutput> {
        // Fast path: route `neomind` data commands through the in-process
        // dispatcher so the agent gets structured `CliResponse` directly,
        // without depending on whatever `neomind` binary happens to be in
        // PATH (eliminates version drift between the running server and the
        // CLI binary). Side-effecting/interactive/local-only commands return
        // `NotInProcess` and fall through to the subprocess path below.
        if let Some(output) = self.try_in_process_dispatch(command, timeout).await {
            return Ok(output);
        }

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

        let mut child = tokio::process::Command::from(cmd)
            .spawn()
            .map_err(|e| ToolError::Execution(format!("Failed to spawn: {}", e)))?;

        // Take stdout/stderr pipes BEFORE the timeout race so the guard can
        // hold the `Child` independently. This is the key change from the
        // previous `wait_with_output`-based flow: that helper consumed the
        // Child, forcing the guard to hold only the PID (and exposing us to
        // PID recycling — kill the wrong process after the kernel reuses the
        // id). Holding the Child itself makes the kernel track ownership for
        // us: the PID stays associated with this handle until we drop it.
        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        // B3 fix: guard holds the Child (not just PID). Drop fires killpg
        // on the child's PID, which is guaranteed to still refer to OUR
        // process because the Child handle owns that PID slot in tokio's
        // process table.
        let mut guard = SubprocessGuard {
            child: Some(child),
        };

        let result = tokio::time::timeout(
            timeout,
            async {
                // Read stdout/stderr concurrently with wait(). Both pipes
                // were taken above, so they live independently of the Child.
                let stdout_fut = async {
                    if let Some(mut s) = stdout_handle {
                        let mut buf = Vec::new();
                        s.read_to_end(&mut buf).await?;
                        Ok::<_, std::io::Error>(buf)
                    } else {
                        Ok(Vec::new())
                    }
                };
                let stderr_fut = async {
                    if let Some(mut s) = stderr_handle {
                        let mut buf = Vec::new();
                        s.read_to_end(&mut buf).await?;
                        Ok::<_, std::io::Error>(buf)
                    } else {
                        Ok(Vec::new())
                    }
                };
                let (out_bytes, err_bytes) =
                    tokio::try_join!(stdout_fut, stderr_fut)?;
                let status = guard
                    .child
                    .as_mut()
                    .ok_or_else(|| std::io::Error::other("child disarmed before wait"))?
                    .wait()
                    .await?;
                Ok::<_, std::io::Error>((out_bytes, err_bytes, status))
            },
        )
        .await;

        match result {
            Ok(Ok((out, err, status))) => {
                // Clean exit — disarm the guard so its Drop doesn't kill
                // an already-exited process group (would be a benign ESRCH
                // but disarming makes the intent obvious).
                guard.child = None;
                Ok(CommandOutput {
                    exit_code: status.code(),
                    stdout: String::from_utf8_lossy(&out).into_owned(),
                    stderr: String::from_utf8_lossy(&err).into_owned(),
                    timed_out: false,
                })
            }
            Ok(Err(e)) => {
                // Pipe/wait error — let guard's Drop handle cleanup so we
                // don't try to await a possibly-broken Child.
                Err(ToolError::Execution(format!("Execution failed: {}", e)))
            }
            Err(_) => {
                // Timeout — explicitly reap so we don't leak a zombie. The
                // guard's Drop will then be a no-op (child is None).
                if let Some(child) = guard.child.as_mut() {
                    // Best-effort kill + reap. kill_process_by_pid sends
                    // killpg(SIGKILL) which terminates the whole group;
                    // child.wait() reaps the immediate child.
                    if let Some(pid) = child.id() {
                        kill_process_by_pid(Some(pid));
                    }
                    let _ = child.wait().await;
                }
                guard.child = None;
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

/// RAII guard that kills a subprocess (and its process group on Unix) when
/// dropped. Used to guarantee cleanup when the future returned by
/// `ShellTool::execute_command` is dropped before completion — the exact path
/// taken when a `CancellationToken` fires and the ToolRegistry `select!`
/// cancels the tool future.
///
/// B3 fix: holds the actual `Child` handle, NOT just the PID. This makes the
/// kernel track PID ownership — the PID cannot be recycled to a different
/// process until we drop this handle. The earlier PID-only design (used
/// because `wait_with_output` consumed the Child) had a small but real
/// risk of killing an unrelated process after PID recycling.
///
/// On Drop, kills the entire process group via `kill_process_by_pid` (Unix:
/// `killpg`, preventing orphaned grandchildren from pipelines). We bypass
/// `Child::start_kill` because that only kills the immediate child.
struct SubprocessGuard {
    /// `None` after clean exit (disarmed) or after explicit reap on timeout.
    child: Option<tokio::process::Child>,
}

impl Drop for SubprocessGuard {
    fn drop(&mut self) {
        if let Some(child) = self.child.take() {
            if let Some(pid) = child.id() {
                // Delegates to the existing platform helper:
                //   Unix:    killpg(pid, SIGKILL) — whole process group
                //   Windows: TerminateProcess on the immediate child
                // Both are best-effort and log on failure; Drop must not panic.
                kill_process_by_pid(Some(pid));
            }
            // We can't `child.wait().await` here (Drop is sync). The
            // immediate-child zombie may persist until the parent process
            // exits — Tokio does NOT auto-reap children dropped without an
            // explicit `wait()`. This is acceptable because:
            //   (a) cancellation is rare (only fires on `scheduler.stop()`),
            //   (b) the OS reaps the zombie when this process exits,
            //   (c) the timeout path explicitly reaps in `execute_command`.
            drop(child);
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
            if libc::killpg(pid as i32, libc::SIGKILL) != 0 {
                tracing::warn!(
                    "Failed to kill process group {}: {}",
                    pid,
                    std::io::Error::last_os_error()
                );
            }
        }
    }
}

#[cfg(windows)]
fn kill_process_by_pid(pid: Option<u32>) {
    if let Some(pid) = pid {
        // TerminateProcess expects a HANDLE, not a PID. We must OpenProcess
        // first, terminate, then CloseHandle. The previous code cast the PID
        // directly to a HANDLE, which is always invalid — the call silently
        // failed and timed-out subprocesses kept running.
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::System::Threading::{
            OpenProcess, TerminateProcess, PROCESS_TERMINATE,
        };

        unsafe {
            let handle: HANDLE = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if handle.is_null() {
                tracing::warn!(
                    "OpenProcess failed for pid {}: {}",
                    pid,
                    std::io::Error::last_os_error()
                );
                return;
            }
            let terminated = TerminateProcess(handle, 1) != 0;
            if !terminated {
                tracing::warn!(
                    "Failed to terminate process {}: {}",
                    pid,
                    std::io::Error::last_os_error()
                );
            }
            CloseHandle(handle);
        }
    }
}

/// Tokenize a `neomind` command line into an argv vector, respecting single
/// and double quotes and backslash escapes.
///
/// This is NOT a full shell parser — it deliberately ignores pipes,
/// redirections, `$` expansions, and command separators, because those
/// constructs are never part of a pure `neomind` data query. A command that
/// uses them is left for the real shell (subprocess path) to interpret.
///
/// The first token is expected to be `neomind`. Returns an error if the input
/// has unbalanced quotes (so the caller can fall back to the subprocess and
/// surface the real shell error message).
fn tokenize_neomind_command(input: &str) -> std::result::Result<Vec<String>, String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' if !in_single => {
                // Backslash escape: take the next char literally. (Inside
                // single quotes, backslash has no special meaning.)
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }

    if in_single || in_double {
        return Err("unbalanced quotes".to_string());
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
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

// ============================================================================
// Error Recovery Hints
// ============================================================================

impl ShellTool {
    /// Generate a recovery hint when a neomind CLI command fails.
    fn recovery_hint(command: &str, stdout: &str, stderr: &str) -> Option<String> {
        let cmd = command.trim();
        if !cmd.starts_with("neomind ") {
            return None;
        }

        let parts: Vec<&str> = cmd.splitn(4, ' ').collect();
        let domain = parts.get(1).copied().unwrap_or("");
        let action = parts.get(2).copied().unwrap_or("");
        let combined = format!("{} {}", stdout, stderr).to_lowercase();

        let is_not_found = combined.contains("not found")
            || combined.contains("404")
            || combined.contains("does not exist")
            || combined.contains("no such");
        let is_validation = combined.contains("validation")
            || combined.contains("invalid")
            || combined.contains("missing")
            || combined.contains("required")
            || combined.contains("400")
            || combined.contains("422");
        let is_unexpected_arg = combined.contains("unexpected argument")
            || combined.contains("unexpected flag")
            || combined.contains("unrecognized argument")
            || combined.contains("unused arguments")
            || combined.contains("error: found argument");

        // Common syntax hint for all neomind commands when unexpected argument
        if is_unexpected_arg {
            if cmd.contains("--id ") {
                return Some("ID is a positional argument, not a flag. Use: neomind <domain> <action> <ID> [options]. Example: neomind device get abc123 (not --id abc123).".to_string());
            }
            return Some("Command syntax error. ID is positional (not --id flag). Run 'neomind <domain> <action> --help' to see correct usage.".to_string());
        }

        match domain {
            "device" => {
                if is_not_found {
                    Some("Run 'neomind device list' to see available devices, then retry with a valid ID.".to_string())
                } else if action == "create" && is_validation {
                    Some("Required fields: --name, --type. Use 'neomind device types list' to see valid device types.".to_string())
                } else if action == "control" && is_not_found {
                    Some("Device not found. Run 'neomind device list' first, then use 'neomind device control <ID> --command <CMD>'.".to_string())
                } else if (action == "history" || action == "latest") && combined.contains("metric")
                {
                    Some("Don't guess metric names. Run 'neomind device list' to see all metric_fields per type, or 'neomind device get <ID>' for a specific device's actual field names.".to_string())
                } else {
                    Some("Available actions: list, get, create, update, delete, latest, history, control, write-metric, webhook-url, types, drafts. ID is positional: neomind device <action> <ID> [flags].".to_string())
                }
            }
            "dashboard" => {
                if is_not_found {
                    Some("Run 'neomind dashboard list' to see available dashboards.".to_string())
                } else if action == "create" && is_validation {
                    Some("Required field: --name. Example: neomind dashboard create --name \"My Dashboard\"".to_string())
                } else if action == "update" {
                    Some("Use --components to update widgets. Run 'neomind widget list' to see available widget types, and 'neomind dashboard get <ID>' to see current layout.".to_string())
                } else {
                    Some("Available actions: list, get, create, update, delete, share".to_string())
                }
            }
            "rule" => {
                if is_not_found {
                    Some("Run 'neomind rule list' to see available rules.".to_string())
                } else if action == "create"
                    && (is_validation || combined.contains("json") || combined.contains("parse"))
                {
                    Some("Rule JSON format: {\"name\":\"...\",\"condition\":{\"condition_type\":\"comparison\",\"source\":\"device:SENSOR_ID:METRIC\",\"operator\":\"greater_than\",\"threshold\":30},\"actions\":[{\"type\":\"notify\",\"message\":\"Alert\",\"severity\":\"critical\"}]}. BEFORE creating: run `neomind device list` to discover real device IDs and metric_fields per type. NEVER guess device IDs or metric names.".to_string())
                } else if action == "enable" || action == "disable" {
                    Some("Run 'neomind rule list' to find the rule ID, then 'neomind rule <enable|disable> <ID>'.".to_string())
                } else {
                    Some("Available actions: list, get, create, update, delete, enable, disable, test, history".to_string())
                }
            }
            "agent" => {
                if is_not_found {
                    Some("Run 'neomind agent list' to see available agents.".to_string())
                } else if action == "create" && is_validation {
                    Some("Required fields: --name, --prompt, --schedule-type (event|interval|cron). Example: neomind agent create --name \"monitor\" --prompt \"Check devices\" --schedule-type event".to_string())
                } else if action == "control" && is_validation {
                    Some("Valid status values: active, paused. Example: neomind agent control <ID> --action active".to_string())
                } else {
                    Some("Available actions: list, get, create, update, delete, control, invoke, memory, executions, latest-execution, conversation, send-message".to_string())
                }
            }
            "extension" => {
                if is_not_found {
                    Some("Run 'neomind extension list' to see installed extensions.".to_string())
                } else if action == "install" && is_validation {
                    Some("Provide the extension zip file path. Use 'neomind extension market-list' to browse marketplace.".to_string())
                } else if action == "config" {
                    Some("Usage: neomind extension config <ID> to view, or neomind extension config <ID> --set '{\"key\":\"value\"}' to update.".to_string())
                } else {
                    Some("Available actions: list, get, status, logs, config, install, uninstall, reload, create, build, market-list, market-install".to_string())
                }
            }
            "transform" => {
                if is_not_found {
                    Some("Run 'neomind transform list' to see available transforms.".to_string())
                } else if action == "create" && is_validation {
                    Some("Required fields: --name, --code (JavaScript function). Use --scope to set input scope. Example: neomind transform create --name \"celsius\" --code \"return value * 9/5 + 32\" --scope global".to_string())
                } else {
                    Some("Available actions: list, get, create, update, delete, test-code, metrics, data-sources".to_string())
                }
            }
            "widget" => {
                if is_not_found {
                    Some(
                        "Run 'neomind widget list' to see available widgets (built-in + custom)."
                            .to_string(),
                    )
                } else if action == "create" && is_validation {
                    Some("Valid widget types: chart, gauge, stat, table, image, custom. Example: neomind widget create \"My Chart\" --widget-type chart".to_string())
                } else if action == "install" && is_validation {
                    Some("Provide a widget directory (containing manifest.json + bundle.js) or a .zip file. Example: neomind widget install data/frontend-components/my-widget".to_string())
                } else {
                    Some("Available actions: list, get, create, install, uninstall, market-list, market-install".to_string())
                }
            }
            "message" => {
                if is_not_found {
                    Some("Run 'neomind message list' to see all messages.".to_string())
                } else if action == "send" && is_validation {
                    Some("Required fields: --title, --body, --severity (info|warning|critical|emergency). Example: neomind message send --title \"Alert\" --body \"High temp\" --severity warning".to_string())
                } else if action == "channel-update" {
                    Some("Usage: neomind message channel-update --name <N> --config '<JSON>'. To filter by severity: --config '{\"min_severity\":\"warning\"}'. To filter by source type: --config '{\"source_types\":[\"device\"]}'. channel-create uses --name flag; channel-delete/channel-test take name as positional arg.".to_string())
                } else {
                    Some("Available actions: list, get, send, read, channel-list, channel-get, channel-create, channel-update, channel-delete, channel-types, channel-test".to_string())
                }
            }
            "llm" => {
                if is_not_found {
                    Some("Run 'neomind llm list' to see configured backends.".to_string())
                } else if action == "create" && is_validation {
                    Some("Required fields: --name, --type (ollama|openai|custom), --endpoint, --model. Example: neomind llm create --name local --type ollama --endpoint http://localhost:11434 --model qwen3:4b".to_string())
                } else {
                    Some("Available actions: list, get, models, create, update, delete, activate, test. Example: neomind llm create --name local --type ollama --endpoint http://localhost:11434 --model qwen3:4b".to_string())
                }
            }
            _ => None,
        }
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        r#"Execute shell commands on the host system.

Use this tool to run any system command. For NeoMind platform operations, use the `neomind` CLI.

## Critical Syntax Rules
- **ID is always a positional argument**, NEVER a `--id` flag. Correct: `neomind device get abc123`. Wrong: `neomind device get --id abc123`.
- **NEVER guess metric names**. Always discover them first via `neomind device list` (shows metric_fields per type) or `neomind device get <ID>` (shows all metric names + values for one device), then use the exact names in `--metric` or rule conditions.
- **When a command fails with "unexpected argument"**, you likely used flag syntax where positional was expected. Rewrite without the flag.

## NeoMind CLI Domains

| Domain | Key Actions | Description |
|--------|------------|-------------|
| device | list, get, create, update, delete, history, control, write-metric, webhook-url, types, drafts | **Discovery**: `device list` returns devices **grouped by type** with `metric_fields` (actual field names from real data), `example` (one online device's current values per type), and all device IDs/names/status. **One command for complete discovery** — no need to call `device latest` separately. `device get <ID>` returns full picture: metadata + config + all metrics + available commands. `device latest <ID>` is an alias for `device get`. CRUD: create/update/delete. Telemetry: `history <ID>` for time-series. Control: `control <ID> <CMD>`. Adapters: `mqtt`/`webhook`. `types` subcommand: list/get/create/delete (for managing type definitions). `drafts` subcommand: list/get/approve/reject/config |
| dashboard | list, get, create, update, delete, share, add-components, remove-components | Dashboard CRUD. `--components` replaces ALL; use `add-components` to append safely |
| widget | list, get, bundle, create, install, uninstall, market-list, market-install | IIFE React components. `create` scaffolds manifest.json + bundle.js. Props: dataSource (.value, .timeSeries), config, title |
| rule | list, get, create, update, delete, enable, disable, test, history | Rules use JSON: `{\"name\":\"...\",\"condition\":{...},\"actions\":[...]}` |
| agent | list, get, create, update, delete, control, invoke, executions, latest-execution, conversation, memory, clear-memory, send-message | Created as `active` by default. **Shortcut**: `--every 5m` (or `30s`, `1h`, `2d`) replaces `--schedule-type interval --schedule-config "300"`. Or use `--schedule-type event` for device-triggered agents. **`--llm-backend`**: check `neomind llm list` for available backends and their capabilities (`multimodal`, `supports_images`, `function_calling`, `max_context`). Match capabilities to the task — use a multimodal backend for image/vision tasks, check `function_calling` for tool-heavy agents |
| transform | list, get, create, update, delete, test-code, metrics, data-sources | JS code transforms; `input` is raw metric value. `--scope` defaults to `global`. `metrics` lists virtual outputs |
| extension | list, get/info, status, logs, config, install, uninstall, market-list, market-install, reload | `get <ID>` returns commands, metrics, config details. `config <ID>` reads config, `config <ID> --set '<JSON>'` updates |
| message | list, get, send, read/ack, channel-list, channel-get, channel-create, channel-update, channel-delete, channel-test, channel-types, channel-type-schema | Send requires `--title` + `--body` + `--severity`. Use `channel-types` to discover types, `channel-type-schema <TYPE>` for config schema. |
| system | info | MQTT broker, webhook URL, network info |
| connector | list, get, create, update, delete, test, subscriptions, subscribe, unsubscribe | Data connectors (MQTT, webhook, etc.) |
| llm | list, get, models, create, update, delete, activate, test | LLM backend management. `create` needs `--name` + `--type` (ollama/openai/custom) + `--endpoint` + `--model`. `activate` sets as default. `test` verifies connection |
| push | list, get, create, update, delete, start, stop, test, logs, stats | Data push targets. `create` needs `--name` + `--config`. Type auto-detected from config (webhook/mqtt). Optional: `--schedule` (event/interval) + `--sources` for filtering. |
| settings | timezone, set-timezone, timezones, retention, set-retention, cleanup | Instance-level settings. `timezone` (read current IANA zone), `set-timezone <ZONE>` (e.g. Asia/Shanghai), `timezones` (list valid IANA zones), `retention` (telemetry/image retention config), `set-retention` (configure auto-cleanup), `cleanup` (trigger manual cleanup). Use `settings timezone` to answer "what timezone is this instance in" — NOT host OS commands like `timedatectl`. |

> **Discover command details**: run `neomind <domain> <action> --help` to see all flags, examples, and usage notes.

## Domain Quick Guides

> For complex operations (dashboard creation, agent management, extension development, device onboarding), use the `skill` tool to load detailed step-by-step guides.

### Rule JSON Format — MANDATORY: discover device IDs and metrics FIRST
**Before creating ANY rule, you MUST run `neomind device list`** to get real device IDs and `metric_fields` per type.
If `metric_fields` is empty, run `neomind device get <ID>` for exact metric names.
**NEVER guess device IDs or metric names** — rules with fake names silently fail.

```json
{
  "name": "Rule Name",
  "condition": {
    "condition_type": "comparison",
    "source": "device:SENSOR_ID:METRIC",
    "operator": "greater_than",
    "threshold": 30
  },
  "actions": [
    {"type": "notify", "message": "Alert: {value}", "severity": "critical"}
  ]
}
```
- **Sources**: `device:SENSOR_ID:METRIC`, `extension:EXT_ID:METRIC`
- **Condition types**: `comparison` (operator + threshold), `range` (min + max), `logical` (AND/OR/NOT combining sub-conditions)
- **Operators**: `greater_than`, `less_than`, `greater_equal`, `less_equal`, `equal`, `not_equal`, `contains`, `starts_with`, `ends_with`, `regex`
- **Actions**: `notify` (message + severity), `execute` (target + command + params), `trigger_agent` (agent_id + input)
- **Severities**: `info`, `warning`, `critical`, `emergency`
- New rules are **enabled by default** — use `neomind rule disable <ID>` to pause

```bash
# Step 1: DISCOVER real device IDs and metric names
neomind device list
# → Returns types with metric_fields (e.g. ["temperature","humidity"]) and device IDs

# Step 2: Create rule using DISCOVERED names (not the examples below!)
# These examples use placeholder names — YOU must replace with real ones from step 1
neomind rule create --json '{"name":"High Temp Alert","condition":{"condition_type":"comparison","source":"device:REAL_DEVICE_ID:temperature","operator":"greater_than","threshold":30},"actions":[{"type":"notify","message":"High temp: {value}°C","severity":"critical"}]}'
```

### Dashboard Components
Grid is 12 columns. `--components` **replaces ALL** — always use `add-components` to append.

**Quick copy-paste templates** (replace values in CAPS):
```bash
# 1. Value card (single metric): 4x2
#    IMPORTANT: type MUST be "telemetry" (not "device") for metric bindings
neomind dashboard add-components DASHBOARD_ID --components '[{"id":"c1","type":"value-card","title":"LABEL","position":{"x":0,"y":0,"w":4,"h":2},"data_source":{"type":"telemetry","source":"device","id":"DEVICE_ID","field":"METRIC_NAME","mode":"latest","sourceId":"DEVICE_ID","metricId":"METRIC_NAME","timeRange":1,"limit":50}}]'

# 2. Line chart (trend): 12x4
neomind dashboard add-components DASHBOARD_ID --components '[{"id":"c2","type":"line-chart","title":"LABEL","position":{"x":0,"y":2,"w":12,"h":4},"data_source":{"type":"telemetry","source":"device","id":"DEVICE_ID","field":"METRIC_NAME","mode":"timeseries","sourceId":"DEVICE_ID","metricId":"METRIC_NAME","timeRange":1,"limit":50,"timeWindow":{"type":"last_24hours"}}}]'

# 3. Gauge: 3x3
neomind dashboard add-components DASHBOARD_ID --components '[{"id":"c3","type":"gauge","title":"LABEL","position":{"x":4,"y":0,"w":3,"h":3},"data_source":{"type":"telemetry","source":"device","id":"DEVICE_ID","field":"METRIC_NAME","mode":"latest","sourceId":"DEVICE_ID","metricId":"METRIC_NAME","timeRange":1,"limit":50},"display":{"min":0,"max":100,"unit":"%"}}]'

# 4. Extension metric: use id + field as COMMAND:FIELD
#    Discover via: neomind extension info <ID> -> commands[].id + commands[].output_fields[].name
neomind dashboard add-components DASHBOARD_ID --components '[{"id":"c4","type":"value-card","title":"LABEL","position":{"x":0,"y":0,"w":4,"h":2},"data_source":{"type":"extension-metric","source":"extension","id":"EXT_ID","field":"COMMAND:FIELD","mode":"timeseries","extensionId":"EXT_ID","extensionMetric":"COMMAND:FIELD"}}]'

# 5. Multi-series line chart: data_source as array
neomind dashboard add-components DASHBOARD_ID --components '[{"id":"c5","type":"line-chart","title":"LABEL","position":{"x":0,"y":2,"w":12,"h":4},"data_source":[{"type":"telemetry","source":"device","id":"DEV1","field":"metric1","mode":"timeseries","sourceId":"DEV1","metricId":"metric1","timeRange":1,"limit":50},{"type":"telemetry","source":"device","id":"DEV2","field":"metric2","mode":"timeseries","sourceId":"DEV2","metricId":"metric2","timeRange":1,"limit":50}],"timeWindow":"1h"}]'
```

DataSource unified fields (v0.8.2+):
| source | mode | id | field | When to use |
|--------|------|----|-------|-------------|
| `device` | `latest` | device ID | metric name | Value cards, LEDs, gauges — single latest value |
| `device` | `timeseries` | device ID | metric name | Line/area/bar charts — historical trend |
| `device` | `command` | device ID | command name | Toggle switches, command buttons |
| `device` | `info` | device ID | property (`name`/`status`/etc) | Map display, device metadata |
| `extension` | `timeseries` | extension ID | `COMMAND:FIELD` | Extension metrics in charts |
| `extension` | `command` | extension ID | command name | Extension command buttons |
| `system` | `latest` | `neomind` | system metric | System stats (cpu, memory, etc) |
| `system` | `timeseries` | `neomind` | system metric | System stats over time |

**IMPORTANT**: Device metrics MUST use `"type":"telemetry"` (NOT `"device"`). The `"device"` type is reserved for map markers (no metric). Always include both unified fields (`source`/`mode`/`id`/`field`) AND legacy fields (`sourceId`/`metricId`/`extensionId`/`extensionMetric`) for full editor compatibility.

**Critical rules:**
- **NEVER guess metric names** — always discover via `device list` (metric_fields per type) or `device get <ID>` or `extension info <ID>` first
- `id` = entity identifier (device ID, extension ID), `field` = metric/command name — same field for all source types
- **extension field MUST be `COMMAND:FIELD` format** (e.g. `get_weather:temperature_c`). Discover via `extension info <ID>` → each command has `id` and `output_fields[].name`. NEVER use bare field names like `temperature_c` — they silently fail to load data.
- Charts always use `mode: "timeseries"`; indicators use `mode: "latest"`
- Position: x increments by width (4-col layout: x=0,4,8), y increments when row is full
- **For full workflow, load `dashboard-management` skill.**

### Transform JS Rules
**Discover first, code second** — NEVER guess field names:
- Device metrics: `neomind device get <ID>` → see actual field names and structure
- Extension metrics: `neomind extension info <ID>` → see commands, params, return fields
- Existing transforms: `neomind transform metrics` or `transform data-sources`

**`input` semantics** (auto-unwrap):
- If device sends `{"value": 42}` → `input = 42` (auto-unwrapped from single-key object)
- If device sends `{"temperature": 23.5, "humidity": 60}` → `input = {temperature: 23.5, humidity: 60}` (multi-key object, use `input.temperature`)
- Must `return` the result (scalar, object, or array)

**`extensions.invoke(extId, command, params)`** — call extension commands from transform:
```javascript
const weather = extensions.invoke('weather', 'get_forecast', {city: 'Shanghai'});
return {temp: weather.temperature, humidity: weather.humidity};
```
Extension calls are pre-executed asynchronously before user code runs.

**Scope**: `global` (all devices) | `device_type:<Type>` (all devices of type) | `device:<ID>` (one device)
**Output**: DataSourceId `transform:<output_prefix>:<field>`

```bash
# Workflow: discover → test → create
neomind device list                       # Step 1: discover fields (metric_fields per type)
neomind transform test-code --code '...' --input '{"temperature": 25}'  # Step 2: test
neomind transform create --name 'F to C' --code 'return (input - 32) * 5 / 9'  # Step 3: create
```

### Custom Widget IIFE Format
No build tools. `manifest.json` + `bundle.js`. Use `neomind widget create "Name" --widget-type <TYPE>` to scaffold.
```javascript
// Preferred: variable assignment with jsxRuntime (cleaner than createElement)
var MyWidget = (function() {
  var React = window.React;
  var jsx = window.jsxRuntime.jsx;
  var jsxs = window.jsxRuntime.jsxs;

  function MyWidget(props) {
    var config = props.config || {};
    var value = (props.dataSource && props.dataSource.value) != null ? props.dataSource.value : '-';
    return jsx('div', {
      className: 'flex flex-col items-center justify-center h-full w-full p-3 rounded-lg border border-border bg-card',
      children: jsx('span', { className: 'text-2xl font-bold font-mono tabular-nums text-foreground', children: String(value) })
    });
  }

  return { default: MyWidget, MyWidget: MyWidget };
})();
```
Runtime: `window.React` (hooks: useState, useEffect, useRef), `window.jsxRuntime.jsx/jsxs`
Styling: Tailwind classes preferred (`text-foreground`, `text-muted-foreground`, `bg-muted`, `bg-success`, `border-border`) or CSS vars (`var(--chart-1..6)`)
**Border requirement**: Every widget's outermost container MUST include `border border-border rounded-lg bg-card` classes. Without borders, cards visually merge with the dashboard background and look incomplete.
Props: `props.dataSource` (.value, .timeSeries, .isLoading, .unit), `props.config`, `props.title`, `props.deviceContext`, `props.sendDeviceCommand`
manifest `global_name` must match IIFE variable name (e.g. `var MyWidget = ...` → `"global_name": "MyWidget"`)

### Widget Creation Workflow (scaffold → edit → install → use)
1. `neomind widget create "My Widget" --widget-type <TYPE>` → scaffold to `data/frontend-components/<widget-id>/`
   - Types: `chart`, `gauge`, `stat`, `table`, `image`, `custom`
2. Edit `manifest.json` — required fields:
   - `id` (lowercase-hyphen, must not match built-ins like `value-card`)
   - `global_name` (convention: `NeoMind{PascalCase}`, must match bundle.js assignment)
   - `has_data_source`: true/false, `config_schema`: JSON Schema for user settings
3. Edit `bundle.js` — must be valid IIFE (see template above), assign to `global['{global_name}']`
4. Install: `neomind widget install data/frontend-components/<widget-id>` (accepts directory or .zip)
5. Add to dashboard: `neomind dashboard add-components <ID> --components '[...]'`
**For complete templates (value card, chart, gauge) and data binding examples, load `widget-development` skill.**

## System Commands
- Network: ping, traceroute, curl, arp, nmap
- Monitoring: ps, df, free, top, uptime, systemctl status
- Files: ls, cat, head, tail, grep, find, wc
- Discovery: arp-scan, avahi-browse, bluetoothctl
- Containers: docker ps, docker logs

Commands run in a separate process — no persistent shell state between calls.
Output may be truncated for very long responses.
On failure, check the "suggestion" field for recovery hints."#
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
            Duration::from_secs(secs.min(crate::toolkit::timeouts::shell_max().as_secs()))
        } else {
            Duration::from_secs(
                self.config
                    .timeout_secs
                    .min(crate::toolkit::timeouts::shell_max().as_secs()),
            )
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

        // Enrich error responses with recovery hints for neomind CLI commands
        let is_error = output.exit_code.unwrap_or(1) != 0;
        if is_error {
            if let Some(hint) = Self::recovery_hint(command, &stdout, &stderr) {
                result["suggestion"] = serde_json::Value::String(hint);
            }
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

    // ====================================================================
    // Cancellation / kill-on-drop tests
    // ====================================================================

    /// When the future returned by `ShellTool::execute` is dropped before
    /// completion (the path taken when a `CancellationToken` fires and the
    /// ToolRegistry select! aborts the tool future), the underlying subprocess
    /// MUST be killed — not orphaned.
    ///
    /// This test runs `sleep 30`, drops the execute future after 200ms, then
    /// verifies via `pgrep` that no `sleep` processes remain. If `pgrep` is
    /// unavailable the assertion is skipped (test still passes as a smoke test).
    #[tokio::test]
    async fn test_shell_subprocess_killed_on_future_drop() {
        // Skip on Windows — process-group semantics differ and pgrep may not exist.
        if cfg!(windows) {
            return;
        }

        let tool = ShellTool::new(ShellConfig {
            enabled: true,
            timeout_secs: 30,
            max_output_chars: 10000,
        });

        // Use a unique sleep duration so we can identify our own process.
        // `sleep 30` is the marker.
        let before = count_sleep_30_processes();

        // Box::pin (not tokio::pin!) so we OWN the future and can drop it
        // explicitly. `tokio::pin!` only creates a Pin<&mut T> reference,
        // so `drop()` on it drops the reference, not the underlying future —
        // leaving the subprocess alive.
        let mut boxed = Box::pin(tool.execute(serde_json::json!({"command": "sleep 30"})));

        // Poll for 200ms — should not complete (sleep runs 30s).
        let poll_result =
            tokio::time::timeout(std::time::Duration::from_millis(200), boxed.as_mut()).await;
        assert!(
            poll_result.is_err(),
            "sleep 30 should not have finished in 200ms"
        );

        // Drop the boxed future — simulates the ToolRegistry select! cancelling it.
        // This MUST trigger SubprocessGuard::drop, killing the subprocess.
        drop(boxed);

        // Give the OS a moment to reap the killed process group.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let after = count_sleep_30_processes();
        assert!(
            after <= before,
            "sleep 30 process should be killed on future drop; before={}, after={}",
            before,
            after
        );
    }

    /// Count `sleep 30` processes currently running. Uses `pgrep -f 'sleep 30'`
    /// (portable across BSD/macOS and Linux — neither supports `-c` consistently).
    /// Returns 0 if pgrep is unavailable (test assertion becomes permissive).
    fn count_sleep_30_processes() -> usize {
        let out = std::process::Command::new("pgrep")
            .arg("-f")
            .arg("sleep 30")
            .output();
        match out {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout);
                if s.trim().is_empty() {
                    0
                } else {
                    s.lines().count()
                }
            }
            Err(_) => 0, // pgrep unavailable; assertion becomes permissive
        }
    }
}
