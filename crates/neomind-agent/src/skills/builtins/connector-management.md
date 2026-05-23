---
id: connector-management
name: Connector Management
category: agent
origin: builtin
priority: 55
token_budget: 8000
triggers:
  keywords: [connector, broker, mqtt broker, external broker, mqtt connection, data source, data connector, webhook, data ingestion, subscribe, subscription]
---

# Connector Management

Connectors are NeoMind's unified interface for data ingestion from external sources. Use `neomind connector` commands to manage them.

## Supported Types

| Type | Description |
|------|-------------|
| `mqtt` | External MQTT broker (currently the only supported type) |

More types (webhook, HTTP polling, CoAP, Modbus) will be added in future releases.

## CLI Commands

All commands use the `neomind connector` domain. The older `neomind broker` alias still works but is deprecated.

### List Connectors

```bash
neomind connector list
neomind connector list --json
```

### Get Connector Details

```bash
neomind connector get <ID>
```

### Create a Connector

```bash
neomind connector create --type mqtt --name "Factory MQTT" --host 192.168.1.100 --port 1883
neomind connector create --type mqtt --name "Secure Broker" --host broker.example.com --port 8883 --tls --username admin --password secret --topics "sensor/#,device/#"
```

Flags:
- `--type` — Connector type (default: mqtt)
- `--name` — Display name (required)
- `--host` — Hostname or IP (required)
- `--port` — Port number (default: 1883)
- `--tls` — Enable TLS
- `--username` — Auth username
- `--password` — Auth password
- `--topics` — Comma-separated topic subscriptions (default: # for all)

### Update a Connector

```bash
neomind connector update <ID> --topics "sensor/+/data" --password "newpass"
neomind connector update <ID> --disable
```

### Test Connectivity

```bash
neomind connector test <ID>
```

### Delete a Connector

```bash
neomind connector delete <ID>
```

### Manage Subscriptions

```bash
neomind connector subscriptions
neomind connector subscribe --topic "factory/+/temperature" --qos 1
neomind connector unsubscribe --topic "factory/+/temperature"
```

## Common Errors & Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| "connection failed" | Wrong host/port or firewall | Verify host, port, and network access |
| "auth failed" | Wrong credentials | Check username/password |
| "already exists" | Duplicate name | Use a different name or update existing |
