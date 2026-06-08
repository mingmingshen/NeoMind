---
id: agent-management
name: AI Agent Management
category: agent
origin: builtin
priority: 85
token_budget: 10000
triggers:
  keywords: [agent, 代理, AI代理, agent create, 创建代理, agent control, schedule, cron, interval, 监控, agent invoke, 调用代理, agent memory, 代理记忆, agent execution, 代理执行, 定时任务, scheduled task, agent schedule, agent update]
  tool_target:
    - tool: agent
      actions: [list, get, create, update, delete, control, invoke, memory, executions, latest-execution, conversation, send-message]
anti_triggers:
  keywords: [device, 设备, rule, 规则, dashboard, 仪表盘, extension develop]
---

# AI Agent Management

Agents are LLM-powered automated tasks. They can be scheduled (interval/cron) or event-driven, and have access to the shell tool to execute CLI commands.

## CRITICAL: Create → Active Pattern

New agents are created in **active** state and start executing immediately (if scheduled). Use this pattern:

```bash
neomind agent create --name 'Monitor' --prompt 'Check battery levels'
# → Returns agent ID (e.g., agent-abc123), already active and running

# To pause later:
neomind agent control agent-abc123 --status paused

# To resume:
neomind agent control agent-abc123 --status active
```

## Schedule Types

| Type | `--schedule-type` | Shortcut / `--schedule-config` | Example |
|------|-------------------|-------------------------------|---------|
| Event | `event` (default) | Not needed | Manual trigger via `invoke` |
| Interval | `interval` | `--every 5m` (shortcut) or `--schedule-config '300'` | `--every 5m` = every 5 min |
| Cron | `cron` | `--schedule-config` | `--schedule-config '0 9 * * *'` = daily 9 AM |

**`--every` shortcut**: `--every 30s`, `--every 5m`, `--every 1h`, `--every 2d` — replaces `--schedule-type interval --schedule-config <seconds>`.

## Execution Modes

| Mode | Description | When to Use |
|------|-------------|-------------|
| `free` (default) | No bound resources, agent has full platform access | General monitoring, analysis tasks |
| `focused` | Bound to specific devices/rules | Requires `--device-ids` or `--resources` |

## Choosing the Right LLM Backend

Before creating an agent, check available backends and their capabilities:

```bash
neomind llm list
```

The response includes a `capabilities` object per backend. Match it to your task:

| Task Type | Required Capability | How to Check |
|-----------|-------------------|--------------|
| Text-only (monitoring, alerts, reports) | None (any backend works) | — |
| Vision / Image analysis | `multimodal: true` or `supports_images: true` | Look for vision models (e.g., `qwen3-vl`, `gpt-4o`) |
| Tool calling / function calling | `function_calling: true` | Most modern models support this |
| Long context tasks | `max_context` value | Compare context window sizes |

> If `--llm-backend` is not specified, the **default active backend** is used.

**Example — creating a vision-capable agent:**
```bash
# Step 1: Find a backend with multimodal support
neomind llm list
# Look for backends where capabilities.multimodal or capabilities.supports_images is true

# Step 2: Create agent with that backend
neomind agent create \
  --name 'Image Analyzer' \
  --prompt 'Analyze device camera images and detect anomalies' \
  --llm-backend <multimodal_backend_id>
```

## Command Reference

### Create Agent

```bash
neomind agent create \
  --name '<name>' \
  --prompt '<task_description>' \
  [--schedule-type <event|interval|cron>] \
  [--schedule-config '<config>'] \
  [--description '<desc>'] \
  [--llm-backend '<llm_backend_id>'] \
  [--system-prompt '<instructions>'] \
  [--execution-mode <free|focused>] \
  [--device-ids 'id1,id2']
```

**Required**: `--name`, `--prompt`
**Important**: `--llm-backend` selects which LLM powers the agent. Run `neomind llm list` first and match backend capabilities to your task (see "Choosing the Right LLM Backend" above).

### Control Agent

```bash
neomind agent control <ID> active    # Start
neomind agent control <ID> paused    # Stop
```

### Invoke (One-shot Execution)

```bash
neomind agent invoke <ID> --input 'Analyze current temperature sensors'
```

### Get Details & Status

```bash
neomind agent get <ID>          # Full config + status
neomind agent list              # All agents
```

### Update Agent

```bash
neomind agent update <ID> --prompt 'New task description'
neomind agent update <ID> --llm-backend qwen3.5:4b
neomind agent update <ID> --name 'Better Name' --description 'Updated'
```

### Monitor Executions

```bash
neomind agent executions <ID> --limit 10      # Execution history
neomind agent latest-execution <ID>           # Most recent execution
neomind agent conversation <ID> --limit 20    # Full message log
neomind agent memory <ID>                     # Execution journal + knowledge files
```

### Send Message

```bash
neomind agent send-message <ID> --body 'Focus on building A sensors'
neomind agent send-message <ID> --body 'Directive' --type instruction
```

## Workflows

### Interval-Based Monitoring Agent

```bash
# 1. Create agent that runs every 5 minutes (active immediately)
neomind agent create \
  --name 'Battery Monitor' \
  --prompt 'Check all devices battery levels. List devices below 20%. Send warning if any found.' \
  --every 5m

# 2. Check results after a few minutes
neomind agent latest-execution <AGENT_ID>
```

### Cron-Based Daily Report

```bash
# Daily at 9:00 AM (active immediately)
neomind agent create \
  --name 'Morning Report' \
  --prompt 'Summarize all device statuses. Count online/offline. Report anomalies from last 24 hours.' \
  --schedule-type cron \
  --schedule-config '0 9 * * *'
```

### On-Demand Analysis Agent

```bash
# No schedule — runs when invoked
neomind agent create \
  --name 'Device Analyzer' \
  --prompt 'Analyze the provided input and generate a detailed report.'

# Run whenever needed
neomind agent invoke <AGENT_ID> --input 'Analyze temperature trends for sensor-001'
```

### Focused Mode Agent (Bound to Specific Devices)

```bash
# Create agent that only has access to specific devices
neomind agent create \
  --name 'Sensor Monitor' \
  --prompt 'Monitor temperature and humidity sensors. Alert if any reading is abnormal.' \
  --every 5m \
  --execution-mode focused \
  --device-ids 'sensor-001,sensor-002,sensor-003'
```

### Debug Agent Issues

```bash
# 1. Check status and config
neomind agent get <ID>

# 2. See recent execution results (check status, duration, error)
neomind agent latest-execution <ID>

# 3. If latest execution failed, check full conversation to see what happened
neomind agent conversation <ID> --limit 20

# 4. Check if LLM backend is available
neomind llm list
# If the configured model is not available, update:
neomind agent update <ID> --llm-backend <available_backend>

# 5. Check agent memory (execution journal + knowledge files)
neomind agent memory <ID>

# 6. If agent is stuck in a loop, pause and review
neomind agent control <ID> --status paused
neomind agent conversation <ID> --limit 50

# 7. After fixing, re-activate
neomind agent control <ID> --status active
```

### Full Lifecycle

```bash
neomind agent create --name 'Health Check' --prompt 'Check all devices' --every 10m
# ... agent starts immediately, check results ...
neomind agent latest-execution <ID>
neomind agent control <ID> --status paused    # Stop when done
neomind agent delete <ID>                      # Remove when no longer needed
```

## Common Errors & Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| "Agent not found" | Wrong ID | Run `neomind agent list` for valid IDs |
| Create fails | Missing `--name` or `--prompt` | Both are required flags |
| Agent not running on schedule | Status is `paused` or wrong schedule config | Run `agent get <ID>` to check status and schedule, then `agent control <ID> --status active` if paused |
| Control fails | Invalid status value | Only `active` and `paused` are valid |
| Focused mode error | No resources bound | Add `--device-ids` or `--resources` |
| Execution shows error | LLM or tool failure | Check `agent conversation <ID>` for details |
| Bad LLM responses | Wrong model/backend | Run `neomind llm list` for available backends, update with `agent update <ID> --llm-backend <backend>` |
