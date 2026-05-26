# Messages & Notifications

> Set up notification channels and track message delivery across 7 platforms: Webhook, Email, Telegram, WeCom, DingTalk, Slack, and Feishu.

---

## Overview

The **Messages** page (`/messages`) has two tabs:

- **Messages** -- delivery log showing all sent notifications with status, severity, and content
- **Channels** -- notification channel configuration and management

All notification channels are created, tested, and managed from the Channels tab. Channels are then referenced by automation rules (NOTIFY action) or agents to deliver alerts.

---

## Step 1: Open the Messages Page

1. Click **Messages** in the left sidebar.
2. The page opens on the **Messages** tab by default, showing the delivery log.
3. Click the **Channels** tab to manage notification channels.

![Messages Channels](../../img/messages-channels.png)

> **What you see above**: The Messages > Channels tab. Each channel row (1) shows its name, type badge, enabled/disabled status, and a **Test** button. The action menu (three dots) provides View, Edit, Configure Filter, Enable/Disable, and Delete options. Click **Add Channel** (2) to create a new notification channel.

---

## Step 2: Create a Channel

1. On the **Channels** tab, click **Add Channel** in the top-right corner.
2. A full-screen dialog opens with a **sidebar listing all channel types**. On mobile, the channel types appear as horizontal scrollable pills at the top.

### Select a Channel Type

3. Click one of the 7 supported channel types in the sidebar:

| Channel Type | Description | Required Fields |
|---|---|---|
| **Webhook** | HTTP POST to any URL | URL |
| **Email** | SMTP email delivery | SMTP server, port, username, password, from address |
| **Telegram** | Bot message to a chat | Bot token, chat ID |
| **WeCom** | Group bot webhook | Webhook key |
| **DingTalk** | Custom robot webhook | Access token, optional secret |
| **Slack** | Incoming webhook | Webhook URL |
| **Feishu** | Custom bot webhook | Hook ID, optional secret |

4. Enter a **Channel Name** (e.g., "Ops Telegram"). The name is used to reference the channel in rules and is displayed in the delivery log.

---

## Step 3: Configure the Channel

Each channel type has its own configuration form. Fill in the required fields for your chosen type.

### Webhook

| Field | Required | Description |
|---|---|---|
| URL | Yes | Endpoint URL to receive POST requests |
| Auth Type | No | None, Bearer, Basic, API Key, or Custom Headers |
| Timeout | No | Request timeout in seconds (default: 30) |

When auth type is selected, additional fields appear:
- **Bearer** -- Token field
- **Basic** -- Username and Password fields
- **API Key** -- Header name (default: `X-API-Key`) and API key value
- **Custom Headers** -- Add multiple header name-value pairs

### Email (SMTP)

| Field | Required | Description |
|---|---|---|
| SMTP Server | Yes | Server address (e.g., `smtp.gmail.com`) |
| SMTP Port | No | Port number (default: `587` for TLS, `465` for SSL) |
| Username | Yes | SMTP login (usually your email address) |
| Password | Yes | SMTP password or app-specific password |
| From Address | Yes | Sender email address |

After saving, use the **Manage Recipients** option from the action menu to add email addresses that will receive notifications.

**Gmail quick setup**: Enable 2FA on your Google account, generate an App Password at `myaccount.google.com/apppasswords`, and use that as the SMTP password.

### Telegram

| Field | Required | Description |
|---|---|---|
| Bot Token | Yes | Token from @BotFather (format: `123456789:ABCdef...`) |
| Chat ID | Yes | Target chat ID (e.g., `-100xxx` for groups) |

**Setup steps**:
1. Open Telegram, search for **@BotFather**, send `/newbot`
2. Copy the bot token
3. Send a message to your bot, then visit `https://api.telegram.org/bot<TOKEN>/getUpdates` to find the chat ID
4. Paste both into NeoMind

### WeCom (Enterprise WeChat)

| Field | Required | Description |
|---|---|---|
| Webhook Key | Yes | Key parameter from the group bot webhook URL |

**Setup**: In WeCom, open the target group > Settings > Group Bot > Add Bot > copy the webhook URL > extract the `key=` parameter value.

### DingTalk

| Field | Required | Description |
|---|---|---|
| Access Token | Yes | Token from the custom robot webhook URL |
| Secret | No | Signing secret for HMAC verification (recommended) |

**Setup**: In DingTalk, open the group > Group Settings > Smart Group Assistant > Add Robot > Custom (via Webhook) > choose Sign mode and copy both the token and secret.

### Slack

| Field | Required | Description |
|---|---|---|
| Webhook URL | Yes | Incoming Webhook URL from your Slack app |

**Setup**: Go to `api.slack.com/apps` > Create New App > Incoming Webhooks > Add New Webhook to Workspace > copy the URL.

### Feishu (Lark)

| Field | Required | Description |
|---|---|---|
| Hook ID | Yes | Hook ID from the custom bot (last path segment of webhook URL) |
| Secret | No | Signing secret for HMAC verification |

**Setup**: In Feishu, open the group > Settings > Group Bots > Add Bot > Custom Bot > copy the webhook URL and extract the hook ID.

---

## Step 4: Test the Channel

5. Click **Save** to create the channel.
6. In the channel list, click the **Test** button next to the channel name. A sample message is sent through the channel.
7. The test result appears inline below the channel name: a green checkmark for success or a red cross with an error message for failure.

Always test before relying on a channel. The test verifies network connectivity, credentials, and message formatting. If it fails, check the error message for specific guidance.

---

## Step 5: Manage Channels

From the **Channels** tab, use the action menu (three dots) on each channel row:

| Action | Description |
|---|---|
| **View** | Open channel details (name, type, config, status) |
| **Edit** | Reopen the channel editor to modify configuration |
| **Configure Filter** | Set which message source types, categories, and minimum severity this channel accepts |
| **Manage Recipients** | (Email only) Add or remove recipient email addresses |
| **Enable / Disable** | Toggle the channel without deleting it. Disabled channels do not receive messages |
| **Delete** | Permanently remove the channel |

**Channel filter** controls which messages are forwarded to a specific channel. For example, you can configure a critical-only channel that only receives `critical` and `emergency` severity messages from device sources.

---

## Step 6: View the Delivery Log

Switch to the **Messages** tab to see all notification messages.

### Messages Table

Each message row shows:

| Column | Description |
|---|---|
| Type | Notification type badge |
| Severity | Severity icon and label: Info, Warning, Critical, or Emergency |
| Title | Message title |
| Content | Message body (truncated) with optional tags |
| Category | Message category (alert, system, business, notification) |
| Status | Active, Acknowledged, Resolved, or Archived |
| Timestamp | When the message was created |

### Filtering Messages

Click the **Filter** button in the tab bar to open the filter panel:
- **Severity** -- filter by Info, Warning, Critical, Emergency
- **Status** -- filter by Active, Acknowledged, Resolved, Archived
- **Category** -- filter by message categories discovered from data

Active filters appear as removable chips above the table.

### Message Actions

Click the action menu (three dots) on any message or open its detail dialog:

| Action | Description |
|---|---|
| **View Details** | Open the message detail dialog showing full content, source, timestamp, tags, and metadata |
| **Acknowledge** | Mark an active message as seen (status changes to Acknowledged) |
| **Resolve** | Mark a message as handled (status changes to Resolved) |
| **Delete** | Permanently remove the message |

### Delivery Retry

The system automatically retries failed deliveries:
- **Maximum retries**: 3 attempts
- **Retry interval**: 2 minutes with progressive backoff
- **Deduplication**: identical messages (same title + source + severity) are skipped within a 60-second window

Messages that fail all retries are marked as permanently failed. Check the delivery log for error details.

---

## Step 7: Use Channels with Automation Rules

Once channels are configured, reference them in automation rules to send alerts when conditions are met.

### Rule NOTIFY Action

1. Navigate to **Automation** in the sidebar.
2. Create or edit a rule (see [Automation & Data](05-automation.md) for the full rule builder flow).
3. In **Step 3 -- Action Configuration**, select **NOTIFY** as the action type.
4. Choose a **Channel** from the dropdown (shows all enabled notification channels).
5. Write the **Message Template** using variables:

| Variable | Description | Example |
|---|---|---|
| `{device_id}` | Triggering device ID | `sensor-01` |
| `{metric}` | Metric name | `temperature` |
| `{value}` | Current value | `37.5` |
| `{threshold}` | Rule threshold | `35` |
| `{rule_name}` | Rule name | `high_temp_alert` |
| `{timestamp}` | Trigger time | `2026-05-26T14:30:00Z` |

6. Click **Save** to activate the rule. When the rule fires, the notification is queued and delivered through the selected channel.

---

## Tips

- **Test before saving** -- always click Test after creating or editing a channel
- **Start with Webhook** -- the most flexible channel type, works with any HTTP endpoint
- **Configure multiple channels** -- set up redundant channels for critical alerts (e.g., Telegram + Email)
- **Use filters** -- configure channel filters to route only relevant messages and avoid notification fatigue
- **Set duration on rules** -- require conditions to persist before triggering to reduce noise
- **Check the delivery log** -- the Messages tab shows delivery status and error details for troubleshooting

---

[Previous: Dashboards](07-dashboard.md) | [Index](README.md) | [Next: Extensions](09-extensions.md)
