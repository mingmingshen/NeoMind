---
id: extension-management
name: Extension Management CLI Commands
category: extension
origin: builtin
priority: 70
token_budget: 10000
triggers:
  keywords: [extension, 扩展, plugin, 插件, extension list, list extension, extension install, 安装扩展, extension status, extension logs, marketplace, 市场扩展, extension info, extension get, nep, extension metrics, extension uninstall, extension health]
  tool_target:
    - tool: extension
      actions: [list, get, info, status, logs, install, uninstall, market-list, market-install]
anti_triggers:
  keywords: [device, 设备, rule, 规则, agent, 代理, dashboard, 仪表盘]
---

# Extension Management CLI Commands

Use `neomind` CLI commands via the `shell` tool to manage extensions. All commands start with `neomind extension`.

---

## Command Reference

### `neomind extension list`

Lists all installed extensions with their IDs, names, versions, and status.

```bash
neomind extension list
```

Returns an array of extensions. Use the extension ID from the output for all other commands.

---

### `neomind extension get <ID>` / `neomind extension info <ID>`

Returns full extension metadata. **`get` and `info` are aliases** — both route to the same function and produce identical output.

```bash
neomind extension get <extension-id>
neomind extension info <extension-id>
```

**This is the most important command for dashboard building.** The response includes:

- Extension metadata (name, version, description, type)
- **Available metrics** — the data fields the extension exposes (e.g., temperature, humidity, power). Each metric has a name, type, and unit.
- **Available commands** — actions the extension can perform.

Use the metrics list to construct `dataSourceId` values for dashboard components. The format is `extension:<extension-id>:<metric-field>`. For example, if extension `weather-forecast` exposes a metric called `temp`, the dataSourceId is `extension:weather-forecast:temp`.

Always run this command before creating or updating dashboards that use extension data.

---

### `neomind extension status <ID>`

Performs a health check on the extension. Returns running status, uptime, and error information.

```bash
neomind extension status <extension-id>
```

Returns: `running`, `stopped`, `error`, or `crashed`. If the extension is in an error state, check the logs for details.

---

### `neomind extension logs <ID>`

Retrieves extension log output for debugging.

```bash
neomind extension logs <extension-id>
neomind extension logs <extension-id> --limit 50
```

**Flags:**
- `--limit <N>` — limit the number of log lines returned (e.g., `--limit 20`). Default returns all available logs.

**IMPORTANT:** The flag is `--limit`, NOT `--lines`. Always use `--limit` when working through the shell tool.

---

### `neomind extension install <path>`

Installs an extension from a local `.nep` file (NeoMind Extension Package).

```bash
neomind extension install /path/to/extension.nep
```

The path must point to a valid `.nep` file. After installation, the command returns the extension ID. Always run `neomind extension status <ID>` afterward to verify the extension started successfully.

---

### `neomind extension uninstall <ID>`

Removes an extension. The extension is stopped and all its files are cleaned up.

```bash
neomind extension uninstall <extension-id>
```

---

### `neomind extension market-list`

Browse available extensions from the marketplace.

```bash
neomind extension market-list
```

Returns a list of extensions with IDs, names, descriptions, and available versions.

---

### `neomind extension market-install <ID>`

Installs an extension from the marketplace by its marketplace ID.

```bash
neomind extension market-install <extension-id>
neomind extension market-install <extension-id> --version 1.2.0
```

**Flags:**
- `--version <VERSION>` — install a specific version. If omitted, installs the latest version.

After installation, verify with `neomind extension status <ID>`.

---

## Commands NOT Available via Shell Tool

The following commands exist in the `neomind` CLI but are **NOT routed through shell.rs internal execution**. They require process spawning and will fail if the agent tries to use them through the internal CLI path:

- `neomind extension validate <path>` — validate a `.nep` file. Flags: `--verbose`
- `neomind extension create <name>` — scaffold a new extension project. Flags: `--extension-type`, `--output`
- `neomind extension build <path>` — build an extension from source

These are developer tools typically run directly in a terminal, not through the AI agent.

---

## Workflows

### Workflow 1: Install an Extension from the Marketplace

```bash
# Step 1: Browse the marketplace
neomind extension market-list

# Step 2: Install the desired extension
neomind extension market-install weather-forecast

# Step 3: Verify it is running
neomind extension status weather-forecast

# Step 4: If status shows an error, check logs
neomind extension logs weather-forecast --limit 30

# Step 5: Discover available metrics and commands
neomind extension info weather-forecast
```

### Workflow 2: Install an Extension from a Local File

```bash
# Step 1: Install from .nep file
neomind extension install /path/to/my-extension.nep

# Step 2: Check the status
neomind extension status my-extension

# Step 3: If the extension fails to start, review logs
neomind extension logs my-extension --limit 50

# Step 4: Once running, inspect its capabilities
neomind extension info my-extension
```

### Workflow 3: Check Extension Health and Troubleshoot

```bash
# Step 1: Check running status
neomind extension status <extension-id>

# Step 2: If the extension is in an error or crashed state, view recent logs
neomind extension logs <extension-id> --limit 50

# Step 3: If logs reveal a configuration issue, you may need to uninstall and reinstall
neomind extension uninstall <extension-id>
neomind extension install /path/to/extension.nep

# Step 4: Confirm it comes back healthy
neomind extension status <extension-id>
```

### Workflow 4: Discover Extension Metrics for Dashboard Binding

This is the critical workflow when a user wants to display extension data on a dashboard.

```bash
# Step 1: List extensions to find the right one
neomind extension list

# Step 2: Get full metadata including available metrics
neomind extension info <extension-id>

# Step 3: From the response, note the metric field names.
# Construct dataSourceId values using the format:
#   extension:<extension-id>:<metric-field-name>
#
# For example, if extension "weather-forecast" reports these metrics:
#   - temp (type: number, unit: celsius)
#   - humidity (type: number, unit: percent)
#   - condition (type: string)
# Then the dataSourceId values are:
#   extension:weather-forecast:temp
#   extension:weather-forecast:humidity
#   extension:weather-forecast:condition

# Step 4: Use these dataSourceId values when creating or updating dashboard widgets.
```

### Workflow 5: View Extension Logs for Debugging

```bash
# View recent logs (last 50 lines)
neomind extension logs <extension-id> --limit 50

# View all available logs
neomind extension logs <extension-id>

# Check status alongside logs for full picture
neomind extension status <extension-id>
```

---

## Quick Reference Table

| Command | Flags | Description |
|---------|-------|-------------|
| `extension list` | — | List all installed extensions |
| `extension get <ID>` | — | Get extension metadata, metrics, commands |
| `extension info <ID>` | — | Alias for `get` — identical output |
| `extension status <ID>` | — | Health check (running/stopped/error/crashed) |
| `extension logs <ID>` | `--limit <N>` | View extension logs (NOT `--lines`) |
| `extension install <path>` | — | Install from local `.nep` file |
| `extension uninstall <ID>` | — | Remove extension |
| `extension market-list` | — | Browse marketplace |
| `extension market-install <ID>` | `--version <V>` | Install from marketplace |

---

## Important Notes

- Extension IDs can be found via `neomind extension list`.
- `info` and `get` are aliases — use whichever feels natural.
- The `info`/`get` command is **essential for dashboard work** because it returns the available metrics. Always call it before binding extension data to dashboard components.
- DataSourceId format for extension metrics: `extension:<extension-id>:<metric-field>`.
- The `--limit` flag (not `--lines`) controls log line count. This is specific to the internal execution path in shell.rs.
- `.nep` is the NeoMind Extension Package format — a zip archive containing the extension binary and metadata.
- After installing any extension, always run `neomind extension status <ID>` to confirm it started successfully before relying on its data.
- Development commands (`validate`, `create`, `build`) are not available through the AI agent's shell tool. They must be run directly in a terminal.

## Common Errors & Solutions

- **"Extension not found"**: Run `neomind extension list` to find valid extension IDs. Use the exact ID from the output.
- **Install fails from local file**: The path must point to a valid `.nep` file. Verify the file exists and is not corrupted. For marketplace extensions, use `neomind market-install <ID>` instead.
- **Extension shows "error" or "crashed" status**: Run `neomind extension logs <ID> --limit 50` to view error details. Common causes: missing config, port conflict, or incompatible runtime version.
- **Extension metrics not appearing on dashboard**: Run `neomind extension info <ID>` to verify the extension exposes metrics. The metric field names in the response must be used exactly in `extensionMetric` (not `metricId` or `property`).
- **Marketplace install fails**: Run `neomind extension market-list` first to verify the extension ID exists in the marketplace. Check network connectivity if the list command itself fails.
- **Logs command returns nothing**: Use `--limit` (not `--lines`) to control output. If the extension was just installed, logs may not be available yet -- wait a moment and retry.
