# Data Push Module

> **Since**: v0.8.0
> **Module**: `neomind-data-push`
> **Storage**: `data/data-push.redb`

## Overview

The Data Push module forwards device telemetry and extension output to external systems in real time or on a schedule. It supports two push target types -- **Webhook** (HTTP POST/PUT) and **MQTT** (publish to an external broker) -- and provides configurable data filtering, payload templating, retry logic, and batch aggregation.

Key capabilities:

- **Event-driven delivery** -- subscribe to the internal EventBus and push immediately when matching data arrives.
- **Interval polling** -- periodically pull data from the time-series store.
- **Data source filtering** -- prefix-based pattern matching with optional change detection.
- **Handlebars templates** -- transform payloads before delivery using `{{source_id}}`, `{{value}}`, `{{timestamp}}`, etc.
- **Retry with exponential backoff** -- configurable `max_retries`, `backoff_secs`, and `max_backoff_secs`.
- **Batch aggregation** -- group multiple events into a single payload before sending.
- **Delivery logging** -- every delivery attempt is recorded with status, payload, response, and error details.
- **Test endpoint** -- send a sample payload to verify target connectivity without waiting for real data.

---

## Module Structure

```
crates/neomind-data-push/
  Cargo.toml
  src/
    lib.rs              -- Module root, public exports
    types.rs            -- Core types (PushTarget, DeliveryLog, PushSchedule, etc.)
    store.rs            -- Redb persistence for targets and delivery logs
    manager.rs          -- PushManager orchestrator, CRUD + lifecycle operations
    scheduler.rs        -- PushScheduler, event-driven and interval task management
    filter.rs           -- DataSourceMatcher with prefix matching and change detection
    template.rs         -- Handlebars template renderer with built-in helpers
    targets/
      mod.rs            -- PushDestination trait and factory function
      webhook.rs        -- Webhook target (HTTP POST/PUT via reqwest)
      mqtt.rs           -- MQTT target (publish via rumqttc with lazy connection)
```

---

## Core Types

### PushTarget

A configured push target that defines where and how data is forwarded.

| Field | Type | Description |
|---|---|---|
| `id` | `String` (UUID) | Auto-generated unique identifier |
| `name` | `String` | Human-readable name (required) |
| `enabled` | `bool` | Whether the target is active (default: `true`) |
| `target_type` | `PushTargetType` | `"webhook"` or `"mqtt"` |
| `config` | `serde_json::Value` | Target-specific configuration (URL, broker, etc.) |
| `schedule` | `PushSchedule` | Event-driven or interval schedule |
| `data_filter` | `DataSourceFilter` | Source patterns and change detection |
| `template` | `Option<String>` | Handlebars template for payload transformation |
| `retry_config` | `RetryConfig` | Retry policy (defaults: 3 retries, 5s backoff) |
| `batch_config` | `BatchConfig` | Batch aggregation settings (defaults: batch_size=1, 2s interval) |
| `created_at` | `i64` | Unix timestamp of creation |
| `updated_at` | `i64` | Unix timestamp of last update |

### PushSchedule (tagged enum)

```json
// Event-driven: push immediately when matching data arrives
{
  "type": "event_driven",
  "event_types": ["device_metric", "extension_output"]
}

// Interval: poll every N seconds
{
  "type": "interval",
  "interval_secs": 60
}
```

Supported `event_types`:
- `"device_metric"` -- device telemetry data
- `"extension_output"` -- extension module output
- `"alert_created"` -- alert events

When `event_types` is an empty array, all event types are matched.

### DataSourceFilter

```json
{
  "source_patterns": ["device:sensor-001:", "extension:weather:"],
  "only_changes": false
}
```

- **`source_patterns`**: Prefix patterns matched against DataSourceId. An empty array matches all sources.
- **`only_changes`**: When `true`, only pushes when the value has changed since the last delivery for that source.

DataSourceId format: `{type}:{id}:{field}` (e.g., `device:sensor-001:temperature`).

Matching logic: a source matches if its ID starts with any pattern OR equals any pattern exactly.

### RetryConfig

```json
{
  "max_retries": 3,
  "backoff_secs": 5,
  "max_backoff_secs": 300
}
```

| Field | Default | Description |
|---|---|---|
| `max_retries` | `3` | Maximum number of retry attempts |
| `backoff_secs` | `5` | Initial backoff in seconds (doubles each retry) |
| `max_backoff_secs` | `300` | Cap for exponential backoff |

Backoff sequence: 5s, 10s, 20s, 40s, ... (capped at `max_backoff_secs`).

### BatchConfig

```json
{
  "batch_size": 10,
  "batch_interval_ms": 2000
}
```

| Field | Default | Description |
|---|---|---|
| `batch_size` | `1` | Max events before flushing. `1` = no batching (immediate send) |
| `batch_interval_ms` | `2000` | Max time to wait before flushing a partial batch |

When `batch_size > 1`, events are buffered until either the batch is full or the interval timer fires. The batch payload structure:

```json
{
  "batch": true,
  "count": 5,
  "items": [
    { "source_id": "device:s1:temp", "value": 25.5, "timestamp": 1700000000, "metadata": null },
    ...
  ]
}
```

### DeliveryLog

Every delivery attempt creates a log entry:

| Field | Type | Description |
|---|---|---|
| `id` | `String` (UUID) | Log entry ID |
| `target_id` | `String` | Which push target this belongs to |
| `status` | `DeliveryStatus` | `pending`, `success`, `failed`, `retrying` |
| `data_source_id` | `String` | Source that triggered this delivery |
| `payload_sent` | `String` | The actual payload sent to the target |
| `response` | `Option<String>` | Response body from the target (if applicable) |
| `attempts` | `u32` | Number of attempts made |
| `created_at` | `i64` | Unix timestamp of first attempt |
| `completed_at` | `Option<i64>` | Unix timestamp of final outcome |
| `error` | `Option<String>` | Error message if failed |

### PushStats

Aggregated statistics across all targets:

```json
{
  "total_targets": 5,
  "active_targets": 3,
  "total_deliveries": 0,
  "successful_deliveries": 0,
  "failed_deliveries": 0
}
```

### TemplateContext

Variables available in Handlebars templates:

| Variable | Type | Description |
|---|---|---|
| `{{source_id}}` | `String` | Full DataSourceId (e.g., `device:s1:temp`) |
| `{{value}}` | `Value` | The data value (number, string, boolean, JSON) |
| `{{timestamp}}` | `i64` | Unix timestamp of the data point |
| `{{metadata}}` | `Value` | Optional metadata attached to the event |

Built-in Handlebars helpers:
- `{{json value}}` -- serialize a value to a JSON string
- `{{timestamp_format timestamp}}` -- format a Unix timestamp to ISO 8601 / RFC 3339

---

## Push Target Types

### Webhook

Sends an HTTP POST (or PUT) request with a JSON body to the specified URL.

**Configuration:**

```json
{
  "url": "https://example.com/api/webhook",
  "method": "POST",
  "headers": {
    "X-Custom-Header": "value"
  },
  "auth_token": "Bearer token here",
  "timeout_secs": 30
}
```

| Field | Default | Description |
|---|---|---|
| `url` | (required) | Target URL |
| `method` | `"POST"` | HTTP method (`"POST"` or `"PUT"`) |
| `headers` | `{}` | Custom HTTP headers |
| `auth_token` | `null` | Bearer token for Authorization header |
| `auth_basic` | `null` | Basic auth (`{ "username": "...", "password": "..." }`) |
| `timeout_secs` | `30` | Request timeout in seconds |

`auth_token` and `auth_basic` are mutually exclusive. If both are provided, `auth_token` takes precedence.

### MQTT

Publishes to an external MQTT broker using the rumqttc async client.

**Configuration:**

```json
{
  "broker": "broker.hivemq.com",
  "port": 1883,
  "topic": "neomind/data/sensor-001",
  "username": "user",
  "password": "pass",
  "qos": 1,
  "client_id": "neomind-push"
}
```

| Field | Default | Description |
|---|---|---|
| `broker` | (required) | MQTT broker hostname |
| `port` | `1883` | Broker port |
| `topic` | (required) | Topic to publish to |
| `username` | `null` | Authentication username |
| `password` | `null` | Authentication password |
| `qos` | `1` | QoS level: `0` (at most once), `1` (at least once), `2` (exactly once) |
| `client_id` | `"neomind-push"` | Client ID prefix (random suffix appended for uniqueness) |

The MQTT connection is established lazily on the first publish and maintained for subsequent deliveries.

---

## Schedule Types

### Event-Driven

Subscribes to the NeoMind EventBus and pushes data immediately when matching events arrive.

```json
{
  "type": "event_driven",
  "event_types": ["device_metric", "extension_output"]
}
```

Behavior:
1. Subscribes to the EventBus broadcast channel.
2. Filters incoming events by `event_types`.
3. Extracts source_id, value, and timestamp from matching events.
4. Applies `DataSourceFilter` patterns and change detection.
5. Renders the payload through the Handlebars template (or default JSON).
6. Delivers with retry logic.

When batch aggregation is enabled (`batch_size > 1`), events are buffered and flushed when either the batch is full or the `batch_interval_ms` timer fires.

### Interval

Periodically queries data at a fixed interval (currently a placeholder for querying the TimeSeriesStore).

```json
{
  "type": "interval",
  "interval_secs": 60
}
```

---

## Delivery Tracking and Retry Logic

### Delivery Flow

1. Data arrives via EventBus (event-driven) or timer tick (interval).
2. Data is filtered against `DataSourceFilter` patterns.
3. If `only_changes` is enabled, the matcher checks whether the value has changed since the last delivery for this source.
4. The payload is rendered through the template engine.
5. A `DeliveryLog` entry is created with status `pending`.
6. The payload is sent to the target.
7. On success: log status is set to `success`, `completed_at` is recorded.
8. On failure: the system retries with exponential backoff.
   - Log status is updated to `retrying` between attempts.
   - After `max_retries` failures, log status is set to `failed`.

### Retry Sequence

For default `RetryConfig` (`max_retries: 3`, `backoff_secs: 5`, `max_backoff_secs: 300`):

| Attempt | Backoff | Total elapsed |
|---|---|---|
| 1 (initial) | - | 0s |
| 2 | 5s | 5s |
| 3 | 10s | 15s |
| 4 (final) | 20s | 35s |

### Log Cleanup

Use `PushManager::cleanup_logs(older_than_days)` to remove old delivery logs. This is useful for preventing unbounded database growth.

---

## API Endpoints

All endpoints are under `/api/data-push`.

### Create Push Target

```
POST /api/data-push
```

**Request body:**

```json
{
  "name": "My Webhook",
  "target_type": "webhook",
  "config": {
    "url": "https://example.com/api/webhook",
    "headers": { "Authorization": "Bearer TOKEN" }
  },
  "schedule": {
    "type": "event_driven",
    "event_types": ["device_metric"]
  },
  "data_filter": {
    "source_patterns": ["device:sensor-001:"],
    "only_changes": false
  },
  "template": "{\"device\":\"{{source_id}}\",\"value\":{{value}},\"ts\":{{timestamp}}}",
  "enabled": true,
  "retry_config": { "max_retries": 3, "backoff_secs": 5, "max_backoff_secs": 300 },
  "batch_config": { "batch_size": 1, "batch_interval_ms": 2000 }
}
```

**Response:**

```json
{
  "success": true,
  "data": {
    "id": "uuid-of-target",
    "name": "My Webhook",
    "target_type": "webhook",
    "enabled": true
  }
}
```

### List Push Targets

```
GET /api/data-push
```

**Query parameters:**
- `enabled` (optional) -- filter by enabled status

**Response:**

```json
{
  "success": true,
  "data": {
    "targets": [...],
    "total": 5
  }
}
```

### Get Push Target

```
GET /api/data-push/:id
```

**Response:** Full `PushTarget` object.

### Update Push Target

```
PUT /api/data-push/:id
```

All fields are optional (partial update). When `config` or `schedule` changes, the target is automatically stopped and restarted.

**Request body:**

```json
{
  "name": "Updated Name",
  "enabled": false,
  "config": { "url": "https://new-url.com/webhook" }
}
```

### Delete Push Target

```
DELETE /api/data-push/:id
```

Stops the target and removes it along with its delivery history.

**Response:**

```json
{
  "success": true,
  "data": { "message": "Push target deleted" }
}
```

### Test Push Target

```
POST /api/data-push/:id/test
```

Sends a sample payload (`{ "test": true, "value": 42 }`) to the target and returns the delivery result.

**Response:** Full `DeliveryLog` object for the test attempt.

### Start Push Target

```
POST /api/data-push/:id/start
```

Enables and starts the target's schedule. Sets `enabled = true` in storage.

### Stop Push Target

```
POST /api/data-push/:id/stop
```

Stops the target's schedule and sets `enabled = false` in storage. The target configuration is preserved.

### List Delivery Logs

```
GET /api/data-push/:id/logs
```

**Query parameters:**
- `limit` (optional, default: 50, max: 200) -- number of log entries
- `offset` (optional, default: 0) -- pagination offset

**Response:**

```json
{
  "success": true,
  "data": {
    "logs": [...],
    "total": 150
  }
}
```

### Get Push Statistics

```
GET /api/data-push/stats
```

**Response:**

```json
{
  "success": true,
  "data": {
    "total_targets": 5,
    "active_targets": 3,
    "total_deliveries": 0,
    "successful_deliveries": 0,
    "failed_deliveries": 0
  }
}
```

---

## CLI Commands

The `neomind push` command group provides full management of push targets.

### push list

```
neomind push list
```

Lists all push targets with their status, type, and configuration.

### push get

```
neomind push get <ID>
```

Shows detailed information about a specific push target.

### push create

```
neomind push create \
  --name "My Webhook" \
  --type webhook \
  --config '{"url":"https://example.com/webhook"}' \
  --schedule event \
  --sources "device:sensor-001:temperature,device:sensor-001:humidity"
```

**Options:**
- `--name` (required) -- target name
- `--type` / `-t` -- target type: `webhook` (default) or `mqtt`
- `--config` -- target config as JSON string
- `--schedule` -- `event` (default) or `interval`
- `--sources` -- comma-separated source patterns (e.g., `"device:s1:"`)

**Webhook config example:**

```json
{
  "url": "https://httpbin.org/post",
  "headers": { "Authorization": "Bearer my-token" }
}
```

**MQTT config example:**

```json
{
  "broker": "broker.hivemq.com",
  "port": 1883,
  "topic": "neomind/data",
  "username": "user",
  "password": "pass"
}
```

### push update

```
neomind push update <ID> --name "New Name"
neomind push update <ID> --config '{"url":"https://new-url.com"}'
neomind push update <ID> --enabled false
```

**Options:**
- `--name` -- new name
- `--config` -- new config as JSON
- `--enabled` -- enable or disable (`true`/`false`)

### push delete

```
neomind push delete <ID>
```

Removes the target and its delivery history.

### push start

```
neomind push start <ID>
```

Enables and starts the push target.

### push stop

```
neomind push stop <ID>
```

Stops the push target without deleting it.

### push test

```
neomind push test <ID>
```

Sends a test payload to verify connectivity and configuration.

### push logs

```
neomind push logs <ID> --limit 20
```

Shows delivery logs for a specific target.

**Options:**
- `--limit` (default: 20) -- max number of log entries

### push stats

```
neomind push stats
```

Displays aggregated statistics across all push targets.

---

## Configuration Examples

### Example 1: Forward Device Temperature to Webhook

Create an event-driven push target that forwards temperature readings from a specific device:

```bash
neomind push create \
  --name "Temperature Webhook" \
  --type webhook \
  --config '{"url":"https://myapp.example.com/api/temperature"}' \
  --schedule event \
  --sources "device:temp-sensor:temperature"
```

### Example 2: MQTT Push with Authentication

Forward all device data to an external MQTT broker:

```bash
neomind push create \
  --name "MQTT Forwarder" \
  --type mqtt \
  --config '{"broker":"mqtt.mycompany.com","port":1883,"topic":"iot/neomind/data","username":"neomind","password":"secret","qos":1}' \
  --schedule event \
  --sources "device:"
```

### Example 3: Interval Polling Webhook

Push data every 60 seconds via HTTP PUT:

```bash
neomind push create \
  --name "Periodic Sync" \
  --type webhook \
  --config '{"url":"https://api.myapp.com/sync","method":"PUT","auth_token":"my-bearer-token"}' \
  --schedule interval \
  --sources "device:sensor-group-1:"
```

### Example 4: Webhook with Custom Payload Template

Using the API directly with a Handlebars template:

```json
{
  "name": "Custom Payload Webhook",
  "target_type": "webhook",
  "config": {
    "url": "https://myapp.example.com/api/data",
    "auth_token": "my-token"
  },
  "schedule": {
    "type": "event_driven",
    "event_types": ["device_metric"]
  },
  "data_filter": {
    "source_patterns": ["device:"],
    "only_changes": true
  },
  "template": "{\"sensor\":\"{{source_id}}\",\"reading\":{{value}},\"captured_at\":\"{{timestamp_format timestamp}}\"}"
}
```

### Example 5: Batch Aggregation

Aggregate up to 50 events before sending as a single batch:

```json
{
  "name": "Batch Webhook",
  "target_type": "webhook",
  "config": { "url": "https://myapp.example.com/api/batch" },
  "schedule": {
    "type": "event_driven",
    "event_types": ["device_metric"]
  },
  "data_filter": {
    "source_patterns": [],
    "only_changes": false
  },
  "batch_config": {
    "batch_size": 50,
    "batch_interval_ms": 5000
  }
}
```

---

## Frontend Management

The Data Push module is managed through the **Data Explorer** page in the NeoMind web UI. Push targets appear as a tab alongside data sources.

### Components

| Component | Location | Description |
|---|---|---|
| `PushTargetsTab` | `web/src/components/datapush/PushTargetsTab.tsx` | Main table view listing all push targets |
| `PushTargetDialog` | `web/src/components/datapush/PushTargetDialog.tsx` | Full-screen dialog for creating/editing targets |
| `DeliveryHistoryPanel` | `web/src/components/datapush/DeliveryHistoryPanel.tsx` | Full-screen dialog showing delivery logs with pagination |

### Push Targets Table

The targets table displays:
- **Name** -- with icon (Globe for webhook, Radio for MQTT) and truncated ID
- **Type** -- badge showing `webhook` or `mqtt`
- **Status** -- green dot for running, gray for stopped
- **Schedule** -- "Event-driven" or "Every Ns"
- **Sources** -- comma-separated source patterns, or "All sources"
- **Updated** -- last modification date
- **Actions** -- Toggle, Test, Logs, Edit, Delete

### Push Target Editor

The editor dialog (`PushTargetDialog`) provides a single-page layout (no wizard steps) with:
- Name field
- Target type selector (Webhook / MQTT)
- Target-specific configuration fields (URL, headers, auth for Webhook; broker, topic, credentials for MQTT)
- Schedule type selector (Event-driven / Interval)
- Data source picker with search, grouping, and multi-select
- Template editor (optional)
- Retry configuration
- Batch aggregation settings

### Delivery History

The delivery history panel shows paginated log entries (10 per page) with:
- **Status** -- color-coded badge (success, failed, pending, retrying)
- **Source** -- the data source that triggered this delivery
- **Payload** -- truncated preview of the sent payload
- **Attempts** -- number of delivery attempts
- **Time** -- timestamp of the delivery

### Store Slice

State management uses the `DataPushSlice` (`web/src/store/slices/dataPushSlice.ts`) in the Zustand store:

| State | Type | Description |
|---|---|---|
| `pushTargets` | `PushTarget[]` | List of all targets |
| `pushTargetsLoading` | `boolean` | Loading state |
| `pushStats` | `PushStats \| null` | Aggregated statistics |
| `pushTargetDialogOpen` | `boolean` | Dialog visibility |
| `editingPushTarget` | `PushTarget \| null` | Target being edited |
| `deliveryLogs` | `DeliveryLog[]` | Logs for the selected target |
| `deliveryLogsTotal` | `number` | Total log count for pagination |

### API Client

All API calls are in `web/src/lib/api.ts` under the Data Push section:

| Method | API Function | Endpoint |
|---|---|---|
| `GET` | `listPushTargets()` | `/data-push` |
| `GET` | `getPushTarget(id)` | `/data-push/:id` |
| `POST` | `createPushTarget(data)` | `/data-push` |
| `PUT` | `updatePushTarget(id, data)` | `/data-push/:id` |
| `DELETE` | `deletePushTarget(id)` | `/data-push/:id` |
| `POST` | `testPushTarget(id)` | `/data-push/:id/test` |
| `POST` | `startPushTarget(id)` | `/data-push/:id/start` |
| `POST` | `stopPushTarget(id)` | `/data-push/:id/stop` |
| `GET` | `listPushDeliveryLogs(id, limit?, offset?)` | `/data-push/:id/logs` |
| `GET` | `getPushStats()` | `/data-push/stats` |

---

## Architecture

```
  EventBus
    |
    v
  PushScheduler
    |-- event-driven task (tokio::spawn)
    |     |-- subscribe to EventBus
    |     |-- DataSourceMatcher (filter + change detection)
    |     |-- TemplateRenderer (Handlebars)
    |     |-- PushDestination (Webhook or MQTT)
    |     |-- retry with exponential backoff
    |     |-- batch aggregation (optional)
    |
    |-- interval task (tokio::spawn)
          |-- periodic timer tick
          |-- query TimeSeriesStore (planned)

  PushManager
    |-- CRUD operations (create, read, update, delete)
    |-- lifecycle (start, stop, test)
    |-- delegates to PushScheduler for running targets
    |-- delegates to DataPushStore for persistence

  DataPushStore (redb)
    |-- push_targets table (key: target ID)
    |-- delivery_logs table (key: log ID)
```

The `PushManager` is initialized during server startup. It opens the `data/data-push.redb` database, loads all persisted targets, and starts any that were previously enabled.
