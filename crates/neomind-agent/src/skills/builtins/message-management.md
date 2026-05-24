---
id: message-management
name: Message Management & Channel Configuration
category: message
origin: builtin
priority: 80
token_budget: 8000
triggers:
  keywords: [message, 消息, 通知, notification, channel, 通道, alert, 告警, 报警, send message, 发送消息, webhook channel, email channel, 邮件通道, 消息管理, severity, 严重程度, acknowledge, 确认, message channel]
  tool_target:
    - tool: message
      actions: [list, get, send, read, ack, channel-list, channel-get, channel-create, channel-update, channel-delete, channel-test, channel-types]
anti_triggers:
  keywords: [dashboard, 仪表盘, agent, 代理, extension develop, 扩展开发, rule, 规则, device connect, 设备连接]
---

# Message Management & Channel Configuration

Messages are platform notifications that can be sent programmatically and delivered through configurable channels (webhook, email, etc.).

## CRITICAL Rules

1. **`--title` and `--message` are required** for sending — not `--body` or `--channel`
2. **Channels must be created before rules can use them** for notifications
3. **Always test a channel** after creation with `message channel-test <NAME>`

## Command Reference

### Send Message

```bash
neomind message send --title '<TITLE>' --message '<TEXT>' [--severity <LEVEL>] [--source <SRC>]
```

| Flag | Required | Default | Description |
|------|----------|---------|-------------|
| `--title` | Yes | — | Message title |
| `--message` | Yes | — | Message body text |
| `--severity` | No | info | Severity: `info`, `warning`, `error`, `critical` |
| `--source` | No | — | Source identifier |

### List & Read Messages

```bash
neomind message list                              # List all messages
neomind message list --severity warning --limit 20 # Filtered list
neomind message get <ID>                          # Get message details
neomind message read <ID>                         # Mark as read (alias: ack)
```

### Channel Management

```bash
neomind message channel-list                      # List all channels
neomind message channel-types                     # List available channel types
neomind message channel-get <NAME>                # Get channel details
neomind message channel-create --name <N> --type <T> --config '<JSON>'  # Create channel
neomind message channel-update <NAME> --config '<JSON>'                 # Update channel config
neomind message channel-delete <NAME>             # Delete channel
neomind message channel-test <NAME>               # Test channel delivery
```

## Channel Types

### Webhook Channel

Sends HTTP POST with message payload to a configured URL.

```bash
neomind message channel-create --name alerts --type webhook \
  --config '{"url": "https://hooks.example.com/notify", "headers": {"Authorization": "Bearer token123"}}'
```

**Config fields:**
- `url` (required): Webhook endpoint URL
- `headers` (optional): Custom HTTP headers

### Email Channel

Sends messages via SMTP.

```bash
neomind message channel-create --name email-alerts --type email \
  --config '{"smtp_server": "smtp.example.com", "smtp_port": 587, "username": "user@example.com", "password": "pass", "from_address": "neo@example.com", "use_tls": true}'
```

**Config fields:**
- `smtp_server` (required): SMTP server hostname
- `smtp_port` (optional, default 587): SMTP port
- `username` (required): SMTP auth username
- `password` (required): SMTP auth password
- `from_address` (required): Sender email
- `use_tls` (optional, default true): Enable TLS

**To discover available channel types and their schemas:**
```bash
neomind message channel-types
```

## Workflow Examples

### Set Up Webhook Alert Channel

```bash
# Step 1: Create the channel
neomind message channel-create --name alerts --type webhook \
  --config '{"url": "https://hooks.slack.com/services/T00/B00/xxx"}'

# Step 2: Test the channel
neomind message channel-test alerts

# Step 3: Send a test message
neomind message send --title 'Test' --message 'Channel setup complete' --severity info
```

### Send Warning Alert

```bash
neomind message send --title 'Low Battery' \
  --message 'Sensor-001 battery at 15%' \
  --severity warning \
  --source sensor-001
```

### Send Critical Alert

```bash
neomind message send --title 'System Overheating' \
  --message 'Temperature sensor reads 95°C — immediate action required' \
  --severity critical
```

### Review & Acknowledge Messages

```bash
# List unread warnings and errors
neomind message list --severity warning
neomind message list --severity error

# Acknowledge a message
neomind message read <ID>
```

### Update Channel Config

```bash
neomind message channel-update alerts \
  --config '{"url": "https://new-hook.example.com/alert", "headers": {"Authorization": "Bearer new-token"}}'
```

### Delete a Channel

```bash
neomind message channel-delete alerts
```

## Using Messages with Rules

Messages are typically sent by rules. When a rule triggers, it can send messages through all configured channels:

```bash
# 1. Set up a channel first
neomind message channel-create --name alerts --type webhook \
  --config '{"url": "https://hooks.example.com/notify"}'

# 2. Create a rule that sends notifications
neomind rule create --name 'High Temp Alert' --dsl 'RULE high_temp
  WHEN sensor-001.temperature > 35
  DO
    NOTIFY "Temperature {value}°C on {{device.name}}" [alerts]
  END'

# 3. Enable the rule
neomind rule enable <RULE_ID>
```

## Common Errors & Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| "Missing required field: title" | Using wrong flag names | Use `--title` and `--message` (not `--body` or `--channel`) |
| "Channel not found" | Wrong channel name | Run `message channel-list` for valid names |
| Webhook not delivering | Wrong URL or network issue | Test with `message channel-test <NAME>` |
| "Invalid channel type" | Unsupported type | Run `message channel-types` for available types |
| "SMTP auth failed" | Wrong email credentials | Verify SMTP settings and update with `channel-update` |
| "Message not found" | Wrong message ID | Run `message list` for valid IDs |
