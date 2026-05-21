---
id: message-management
name: Message Management CLI Commands
category: message
origin: builtin
priority: 80
token_budget: 10000
triggers:
  keywords: [message, 消息, 通知, notification, message list, list message, message send, 发送消息, message read, 已读, unread, 未读, alert, 警报, acknowledge, ack, channel, 通道, 通知通道, webhook, dingtalk, 钉钉, email, 邮件]
  tool_target:
    - tool: message
      actions: [list, get, send, read, ack, channel-list, channel-get, channel-create, channel-update, channel-delete, channel-types, channel-test]
anti_triggers:
  keywords: [device, 设备, rule, 规则, agent, 代理, dashboard, 仪表盘]
---

# Message Management CLI Commands

Use `neomind` CLI commands via the `shell` tool to manage messages and notifications.

## Commands Overview

| Command | Description |
|---------|-------------|
| `neomind message list` | List messages with optional filters |
| `neomind message get <ID>` | Get details of a specific message |
| `neomind message send` | Send a new message |
| `neomind message read <ID>` | Mark a message as read |
| `neomind message ack <ID>` | Mark a message as read (alias) |

---

## List Messages

Returns a paginated list of messages. Supports filtering by severity and read status.

```bash
neomind message list [--limit N] [--offset N] [--severity LEVEL] [--status STATUS]
```

**Flags:**

| Flag | Required | Default | Description |
|------|----------|---------|-------------|
| `--limit` | No | — | Maximum number of messages to return |
| `--offset` | No | — | Number of messages to skip (for pagination) |
| `--severity` | No | — | Filter by severity: `info`, `warning`, `error`, `critical` |
| `--status` | No | — | Filter by status: `read`, `unread` |

**Examples:**

```bash
# List all messages (first page)
neomind message list

# Paginate through results
neomind message list --limit 10 --offset 20

# Show only unread messages
neomind message list --status unread

# Show critical messages only
neomind message list --severity critical

# Combine filters: unread warnings
neomind message list --severity warning --status unread
```

**API mapping:** `GET /messages?limit=N&offset=N&severity=X&status=X`

---

## Get Message Details

Retrieve full details of a specific message by its ID.

```bash
neomind message get <ID>
```

**Examples:**

```bash
neomind message get msg-123
neomind message get 42
```

**API mapping:** `GET /messages/{id}`

---

## Send Message

Create and send a new message or notification.

```bash
neomind message send --title '<title>' --message '<content>' [--severity LEVEL] [--source SOURCE]
```

**Flags:**

| Flag | Required | Default | Description |
|------|----------|---------|-------------|
| `--title` | Yes | — | Message title (short summary) |
| `--message` | Yes | — | Message body (full content) |
| `--severity` | No | `info` | Severity level: `info`, `warning`, `error`, `critical` |
| `--source` | No | — | Source identifier (e.g., "agent", "system", extension name) |

**Severity levels:**
- `info` — General information, no action needed
- `warning` — Attention recommended, potential issue
- `error` — Problem detected, action likely needed
- `critical` — Urgent issue, immediate action required

**Examples:**

```bash
# Simple info notification
neomind message send --title 'Task Complete' --message 'Dashboard created successfully' --severity info

# Warning about a device condition
neomind message send --title 'Low Battery' --message 'Device sensor-001 battery is at 15%' --severity warning

# Critical alert for offline device
neomind message send --title 'Device Offline' --message 'Sensor-003 has been offline for 30 minutes' --severity critical

# With source attribution
neomind message send --title 'Rule Triggered' --message 'Temperature threshold exceeded on floor-3' --severity warning --source agent
```

**API mapping:** `POST /messages` with body `{title, message, severity, source}`

---

## Read/Acknowledge Message

Mark a message as read. Both `read` and `ack` are equivalent.

```bash
neomind message read <ID>
neomind message ack <ID>
```

**Examples:**

```bash
neomind message read msg-123
neomind message ack 42
```

**API mapping:** `POST /messages/{id}/acknowledge`

---

## Workflows

### Check and clear unread messages

Review and acknowledge unread notifications.

```bash
# Step 1: List all unread messages
neomind message list --status unread

# Step 2: Read details of a specific message
neomind message get <MESSAGE_ID>

# Step 3: Mark it as read
neomind message read <MESSAGE_ID>

# Step 4: Confirm it's cleared
neomind message list --status unread
```

### Send alerts from automation

Use in agent workflows or rules to notify users of important events.

```bash
# Device went offline — critical alert
neomind message send --title 'Device Offline' --message 'Production sensor array disconnected unexpectedly' --severity critical --source device-monitor

# Threshold exceeded — warning
neomind message send --title 'High Temperature' --message 'Server room temperature reached 35C, threshold is 30C' --severity warning

# Extension error — error notification
neomind message send --title 'Extension Error' --message 'Weather extension failed to fetch data: connection timeout' --severity error --source system
```

### Filter messages by severity

Audit messages at a specific severity level.

```bash
# Check for critical issues
neomind message list --severity critical

# Review all errors
neomind message list --severity error

# Paginate through warnings
neomind message list --severity warning --limit 10 --offset 0
```

### Send info notification after completing a task

Confirm task completion to the user via the messaging system.

```bash
# After creating a dashboard
neomind message send --title 'Dashboard Ready' --message 'Battery Monitor dashboard has been created with 4 widgets' --severity info --source agent

# After applying a configuration change
neomind message send --title 'Config Updated' --message 'Retention policy changed to 30 days' --severity info --source agent

# After bulk operation
neomind message send --title 'Devices Registered' --message '5 new devices added to the system' --severity info --source agent
```

---

## Notes

- Message IDs are returned by `neomind message list` and can be used with `get`, `read`, and `ack`
- `read` and `ack` are interchangeable — both call the same acknowledge endpoint
- Use `--status unread` to find messages requiring attention
- The `--source` flag on send is informational only — it helps identify where the message originated
- Pagination uses `--limit` (page size) and `--offset` (skip count); typical page size is 10-20
- All severity levels are lowercase: `info`, `warning`, `error`, `critical`

---

## Notification Channels

Messages can be routed through notification channels (webhook, email, etc.).

### List Channels

```bash
neomind message channel-list
```

### Get Channel Details

```bash
neomind message channel-get <NAME>
```

### List Available Channel Types

```bash
neomind message channel-types
```

Returns all supported channel types (e.g., `webhook`, `email`) with their configuration schemas.

### Create Channel

```bash
neomind message channel-create --name '<name>' --type <TYPE> --config '<json>'
```

**Required flags**: `--name`, `--type`, `--config`

**Examples:**

```bash
# Webhook channel
neomind message channel-create --name 'my-webhook' --type webhook --config '{"url": "https://example.com/webhook"}'

# DingTalk (钉钉) webhook
neomind message channel-create --name 'dingtalk' --type webhook --config '{"url": "https://oapi.dingtalk.com/robot/send?access_token=YOUR_TOKEN"}'

# Email channel
neomind message channel-create --name 'alerts' --type email --config '{"smtp_host": "smtp.example.com", "smtp_port": 587, "from": "alert@example.com", "to": ["admin@example.com"]}'
```

Use `neomind message channel-types` to see all available types and their config schemas.

### Update Channel

```bash
neomind message channel-update <NAME> --config '<new_json>'
```

**Example:**

```bash
neomind message channel-update my-webhook --config '{"url": "https://new-url.example.com/hook"}'
```

### Delete Channel

```bash
neomind message channel-delete <NAME>
```

### Test Channel Connectivity

```bash
neomind message channel-test <NAME>
```

Sends a test message through the channel to verify connectivity and configuration.

## Common Errors & Solutions

- **"Message not found"**: Run `neomind message list` to find valid message IDs. Use the exact ID from the output.
- **Send fails with missing fields**: Both `--title` and `--message` are required flags. Omitting either will cause an error.
- **Invalid severity level**: Valid values are `info`, `warning`, `error`, `critical` (all lowercase). Other values like `normal`, `high`, `low` are not accepted.
- **Message still showing as unread after ack**: Ensure you are using the correct message ID from `neomind message list`. Both `read` and `ack` subcommands work identically.
- **Pagination returns no results**: The offset is zero-based. If a list returns fewer results than expected, try reducing the offset or removing filters to confirm messages exist.
- **Channel creation fails**: Use `neomind message channel-types` to check supported channel types and their required config fields. Each channel type has a specific config schema.
- **Webhook test fails**: Verify the URL is accessible. Use `neomind message channel-test <NAME>` to diagnose connectivity.
- **"Channel not found"**: Run `neomind message channel-list` to see all channels. Use the exact channel name (not ID).
