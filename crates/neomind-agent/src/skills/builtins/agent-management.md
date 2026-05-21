---
id: agent-management
name: Agent Management CLI Commands
category: agent
origin: builtin
priority: 80
token_budget: 10000
triggers:
  keywords: [agent, 代理, AI代理, agent list, list agent, agent create, 创建代理, agent invoke, 调用代理, agent memory, 代理记忆, agent execution, 代理执行, schedule, cron, interval, monitor, 监控]
  tool_target:
    - tool: agent
      actions: [list, get, create, update, delete, control, invoke, memory, executions, latest-execution, conversation, send-message]
anti_triggers:
  keywords: [device, 设备, rule, 规则, dashboard, 仪表盘]
---

# Agent Management CLI Commands

Use `neomind` CLI commands via the `shell` tool to manage AI agents. All commands begin with `neomind agent`.

## ⚠️ CRITICAL: Common Operations Checklist

When user asks to **start/stop/delete** an agent, you MUST:
1. `neomind agent list` → find the agent ID
2. Then execute the actual action command:

| User Request | Command |
|---|---|
| 启动/Start agent | `neomind agent control <ID> --status active` |
| 停止/Stop agent | `neomind agent control <ID> --status paused` |
| 删除/Delete agent | `neomind agent delete <ID>` |
| 创建后启动 | `neomind agent create ...` then `neomind agent control <ID> --status active` |
| 发消息/Send message | `neomind agent send-message <ID> --message '...'` |
| 查看执行结果 | `neomind agent latest-execution <ID>` |

> **DO NOT stop after `agent list`!** Listing is only step 1. You MUST proceed to the action command.

## Command Reference

### List All Agents

```bash
neomind agent list
```

Returns all agents with their IDs, names, status, and schedule type.

### Get Agent Details

```bash
neomind agent get <ID>
```

Returns full agent configuration including prompt, model, schedule, and status.

### Create Agent

```bash
neomind agent create --name '<name>' --prompt '<task_prompt>'
```

**Required flags**: `--name`, `--prompt`

**Optional flags**:
- `--description` — human-readable description of the agent's purpose
- `--schedule-type` — one of: `interval`, `cron`, `event` (default: `event`)
- `--schedule-config` — depends on schedule type (see below)
- `--model` — LLM model to use (e.g., `deepseek-v4`, `qwen3.5:4b`)
- `--system-prompt` — system-level instructions for the agent

**Schedule configuration**:
- `event` (default): agent runs when triggered manually or by external events. No `--schedule-config` needed.
- `interval`: `--schedule-config` is the interval in seconds (e.g., `"300"` for every 5 minutes, `"3600"` for hourly)
- `cron`: `--schedule-config` is a cron expression (e.g., `"0 9 * * *"` for daily at 9 AM)

**Important**: CLI-created agents always use `execution_mode: "free"` (no bound resources).

### Update Agent

```bash
neomind agent update <ID> --name '<new_name>'
neomind agent update <ID> --prompt '<new_prompt>'
neomind agent update <ID> --model '<model_name>'
neomind agent update <ID> --system-prompt '<new_system_prompt>'
neomind agent update <ID> --description '<new_description>'
```

Any combination of flags can be used in a single update command. Only specified fields are changed.

### Control Agent Status

```bash
neomind agent control <ID> --action active
neomind agent control <ID> --action paused
neomind agent control <ID> --status active
neomind agent control <ID> --status paused
```

Both `--action` and `--status` are accepted and behave identically. Values are `active` or `paused`. Agents must be `active` to run on schedule or receive invocations.

### Invoke Agent (One-shot Execution)

```bash
neomind agent invoke <ID> --input '<message>'
```

Triggers a single execution of the agent with the provided input. The agent runs immediately regardless of its schedule.

### Get Agent Memory

```bash
neomind agent memory <ID>
```

Returns knowledge extracted from past agent conversations. Memory is accumulated over time and helps the agent maintain context.

### Get Execution History

```bash
neomind agent executions <ID>
neomind agent executions <ID> --limit 10 --offset 0
```

Returns a paginated list of past executions with status, timestamps, and results. Default limit is applied if not specified.

### Get Latest Execution

```bash
neomind agent latest-execution <ID>
```

Returns only the most recent execution record. Useful for quickly checking if the last run succeeded.

### Get Agent Conversation

```bash
neomind agent conversation <ID>
neomind agent conversation <ID> --limit 20
```

Returns the full message history between the user and the agent, including tool calls and responses.

### Send Message to Agent

```bash
neomind agent send-message <ID> --message '<message>'
neomind agent send-message <ID> --message '<message>' --type instruction
```

Sends a message to the agent's conversation. The `--type` flag can specify the message type (e.g., `instruction` for directives that guide agent behavior).

### Delete Agent

```bash
neomind agent delete <ID>
```

Permanently removes the agent and its data. This action cannot be undone.

## Workflows

### Create an Interval-Based Monitoring Agent

Creates an agent that runs on a fixed interval (e.g., every 5 minutes).

```bash
# Step 1: Create the agent with interval schedule
neomind agent create \
  --name 'Battery Monitor' \
  --prompt 'Check all devices battery levels. List any devices with battery below 20%. If any are found, send a warning message.' \
  --description 'Monitors device battery levels every 5 minutes' \
  --schedule-type interval \
  --schedule-config '300' \
  --model deepseek-v4 \
  --system-prompt 'You are an IoT monitoring assistant. Be concise and actionable.'

# Step 2: Activate the agent (replace AGENT_ID with actual ID from create output)
neomind agent control <AGENT_ID> --action active

# Step 3: Verify it is running
neomind agent get <AGENT_ID>

# Step 4: After some time, check the latest execution
neomind agent latest-execution <AGENT_ID>

# Step 5: Review accumulated knowledge
neomind agent memory <AGENT_ID>
```

### Create a Cron-Based Agent

Creates an agent that runs on a cron schedule (e.g., daily at 9:00 AM).

```bash
# Create a daily morning report agent
neomind agent create \
  --name 'Morning Report' \
  --prompt 'Generate a summary of all device statuses. Count online/offline devices. Report any anomalies from the last 24 hours.' \
  --description 'Daily 9 AM device status report' \
  --schedule-type cron \
  --schedule-config '0 9 * * *' \
  --model deepseek-v4

# Activate it
neomind agent control <AGENT_ID> --action active
```

### Create an Event-Driven Agent (Default)

Creates an agent that only runs when manually invoked or triggered. This is the default schedule type.

```bash
neomind agent create \
  --name 'On-Demand Analyzer' \
  --prompt 'Analyze the provided input and generate a detailed report.' \
  --description 'On-demand analysis agent'

# Run it whenever needed
neomind agent invoke <AGENT_ID> --input 'Analyze the current state of all temperature sensors'
```

### Quick One-Shot Task via Invoke

For agents that already exist, use `invoke` for immediate one-off tasks.

```bash
# Invoke with a specific query
neomind agent invoke <AGENT_ID> --input 'List all devices with battery below 30%'

# Invoke with a complex multi-step instruction
neomind agent invoke <AGENT_ID> --input 'Check device sensor-001 telemetry data, compare it with yesterday, and report any significant deviations'
```

### Debug Agent Behavior

When an agent is not behaving as expected, use these commands to diagnose issues.

```bash
# 1. Check current configuration and status
neomind agent get <ID>

# 2. Review recent execution history for errors
neomind agent executions <ID> --limit 5

# 3. Check the very latest execution for immediate status
neomind agent latest-execution <ID>

# 4. Read the full conversation to understand what the agent did
neomind agent conversation <ID> --limit 20

# 5. Check extracted memory for stale or incorrect knowledge
neomind agent memory <ID>
```

### Send Instruction to a Running Agent

Send a directive to guide an active agent's behavior without triggering a new execution.

```bash
# Send a behavioral instruction
neomind agent send-message <AGENT_ID> --message 'Focus on temperature sensors in building A only' --type instruction

# Send a regular message
neomind agent send-message <AGENT_ID> --message 'What was the result of the last check?'
```

### Pause and Reactivate an Agent

Temporarily stop a scheduled agent, then resume it later.

```bash
# Pause the agent — stops all scheduled executions
neomind agent control <AGENT_ID> --action paused

# Verify it is paused
neomind agent get <AGENT_ID>

# ... later, reactivate it
neomind agent control <AGENT_ID> --action active
```

### Modify an Existing Agent

Update an agent's configuration without recreating it.

```bash
# Change the prompt
neomind agent update <AGENT_ID> --prompt 'Check battery levels and also report devices that have been offline for more than 1 hour'

# Switch to a different model
neomind agent update <AGENT_ID> --model qwen3.5:4b

# Update multiple fields at once
neomind agent update <AGENT_ID> --name 'Enhanced Battery Monitor' --description 'Extended monitoring with offline detection' --system-prompt 'You are a thorough IoT monitoring assistant. Always include device IDs in reports.'
```

### Full Lifecycle: Create, Run, Monitor, Cleanup

End-to-end example of creating an agent, running it, checking results, and cleaning up.

```bash
# 1. Create the agent
neomind agent create \
  --name 'Health Check' \
  --prompt 'Check all devices, report online/offline counts, list any devices with anomalies' \
  --schedule-type interval \
  --schedule-config '600'

# 2. Activate it
neomind agent control <AGENT_ID> --action active

# 3. Wait for execution, then check results
neomind agent latest-execution <AGENT_ID>

# 4. Review conversation
neomind agent conversation <AGENT_ID> --limit 10

# 5. Check memory
neomind agent memory <AGENT_ID>

# 6. When done, pause or delete
neomind agent control <AGENT_ID> --action paused
# OR permanently remove:
neomind agent delete <AGENT_ID>
```

## Notes

- Agent IDs are returned by `neomind agent create` and visible in `neomind agent list`
- Both `--model` and `--llm-backend` are accepted when creating or updating agents
- `control` accepts both `--action` and `--status` flags interchangeably
- `invoke` runs the agent immediately regardless of schedule; for recurring tasks use `create` with `--schedule-type`
- `memory` contains knowledge automatically extracted from past conversations
- `conversation` shows the full message log including tool calls — useful for debugging
- Schedule types: `event` (default, manual trigger), `interval` (every N seconds), `cron` (cron expression)
- `execution_mode` is always set to `"free"` for CLI-created agents (no bound device/rule resources)
- Use `latest-execution` (with hyphen) for the most recent execution record
- Use `send-message` (with hyphen) to send messages to an agent's conversation

## Common Errors & Solutions

- **"Agent not found"**: Run `neomind agent list` to find valid agent IDs. Use the exact ID from the output.
- **Create fails**: The required flags are `--name` and `--prompt`. If the schedule is not `event` (default), also provide `--schedule-type` and `--schedule-config`.
- **Control command fails**: Valid status values are `active` and `paused` only. Both `--action` and `--status` flags are accepted. Do not use `start`, `stop`, `running`, or other values.
- **Agent not executing on schedule**: Run `neomind agent get <ID>` to check if the status is `active`. If it is `paused`, run `neomind agent control <ID> --action active`. Also check `neomind agent latest-execution <ID>` for errors.
- **`execution_mode: "free"` required**: CLI-created agents automatically use `execution_mode: "free"` (no bound resources). If an API call fails with a resource binding error, ensure `execution_mode` is set to `"free"`.
- **Agent list only shows IDs/names**: After listing, use `neomind agent get <ID>` for full details including status, schedule, and prompt.
